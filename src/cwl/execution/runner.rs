use crate::cwl::clt::Argument;
use crate::cwl::clt::Command;
use crate::cwl::clt::CommandLineBinding;
use crate::cwl::execution::staging::unstage_files;
use crate::cwl::execution::util::evaluate_input_as_string;
use crate::cwl::execution::util::evaluate_outputs;
use crate::cwl::execution::validate::rewire_paths;
use crate::error::CommandError;
use crate::util::format_command;
use crate::{
    cwl::{
        clt::{CommandLineTool, DefaultValue},
        execution::{
            environment::{set_tool_environment_vars, unset_environment_vars},
            staging::stage_required_files,
            validate::set_placeholder_values,
        },
    },
    io::{create_and_write_file, get_shell_command},
    util::{get_available_ram, get_processor_count},
};
use std::{
    collections::HashMap,
    env,
    error::Error,
    path::{Path, PathBuf},
    process::Command as SystemCommand,
};
use tempfile::tempdir;

pub fn run_commandlinetool(
    tool: &mut CommandLineTool,
    input_values: Option<HashMap<String, DefaultValue>>,
    cwl_path: Option<&str>,
    out_dir: Option<String>,
) -> Result<(), Box<dyn Error>> {
    //create staging directory
    let dir = tempdir()?;
    eprintln!("📁 Created staging directory: {:?}", dir.path());

    //save reference to current working directory
    let current = env::current_dir()?;
    let output_directory = if let Some(out) = out_dir { &PathBuf::from(out) } else { &current };

    //build runtime object
    let runtime = HashMap::from([
        ("outdir".to_string(), output_directory.to_string_lossy().into_owned()),
        ("tmpdir".to_string(), dir.path().to_string_lossy().into_owned()),
        ("cores".to_string(), get_processor_count().to_string()),
        ("ram".to_string(), get_available_ram().to_string()),
    ]);

    //change working directory to the cwl file's path as all paths are given relative to the tool
    let tool_path = if let Some(file) = cwl_path { Path::new(file).parent().unwrap() } else { Path::new(".") };
    env::set_current_dir(current.join(tool_path))?;

    //replace inputs and runtime placeholders in tool with the actual values
    set_placeholder_values(tool, input_values.as_ref(), &runtime);

    //stage files listed in input default values, input values or initial work dir requirements
    let staged_files = stage_required_files(tool, &input_values, dir.path().to_path_buf())?;

    //change working directory to tmp folder, we will execute tool from root here
    env::set_current_dir(dir.path())?;

    //set environment variables
    let environment_variables = set_tool_environment_vars(tool);

    //rewire files in tool to staged ones
    let mut input_values = input_values;
    rewire_paths(tool, &mut input_values, &staged_files);

    //set required environment variables
    let home_directory = env::var("HOME").unwrap_or_default();
    let tmp_directory = env::var("TMPDIR").unwrap_or_default();
    env::set_var("HOME", &runtime["outdir"]);
    env::set_var("TMPDIR", &runtime["tmpdir"]);

    //run the tool command)
    run_command(tool, input_values).map_err(|e| CommandError {
        message: format!("❌ Error in Tool execution: {}", e),
        exit_code: tool.get_error_code(),
    })?;
    //reset required environment variables
    env::set_var("HOME", home_directory);
    env::set_var("TMPDIR", tmp_directory);

    //remove staged files
    unstage_files(&staged_files, dir.path(), &tool.outputs)?;

    //evaluate output files
    evaluate_outputs(&tool.outputs, output_directory)?;

    //unset environment variables
    unset_environment_vars(environment_variables);

    //come back to original directory
    env::set_current_dir(current)?;

    eprintln!("✔️  Command Executed with status: success!");
    Ok(())
}

pub fn run_command(tool: &CommandLineTool, input_values: Option<HashMap<String, DefaultValue>>) -> Result<(), Box<dyn Error>> {
    let mut args = vec![];

    //get executable
    let cmd = match &tool.base_command {
        Command::Single(cmd) => cmd,
        Command::Multiple(vec) => &vec[0],
    };

    args.push(cmd);
    //append rest of base command as args
    if let Command::Multiple(ref vec) = &tool.base_command {
        args.extend(&vec[1..]);
    }

    let mut bindings: Vec<(isize, usize, CommandLineBinding)> = vec![];

    //handle arguments field...
    if let Some(arguments) = &tool.arguments {
        for (i, arg) in arguments.iter().enumerate() {
            match arg {
                Argument::String(str) => {
                    let binding = CommandLineBinding {
                        value_from: Some(str.clone()),
                        ..Default::default()
                    };
                    bindings.push((0, i, binding));
                }
                Argument::Binding(binding) => {
                    let position = binding.position.unwrap_or_default();
                    bindings.push((position, i, binding.clone()));
                }
            }
        }
    }
    let index = bindings.len() + 1;

    //handle inputs
    for (i, input) in tool.inputs.iter().enumerate() {
        if let Some(ref binding) = &input.input_binding {
            let mut binding = binding.clone();
            let position = binding.position.unwrap_or_default();
            binding.value_from = Some(evaluate_input_as_string(input, &input_values)?);
            bindings.push((position, i + index, binding))
        }
    }

    //do sorting
    bindings.sort_by(|a, b| {
        let cmp = a.0.cmp(&b.0);
        if cmp == std::cmp::Ordering::Equal {
            a.1.cmp(&b.1)
        } else {
            cmp
        }
    });

    //add bindings
    let inputs: Vec<CommandLineBinding> = bindings.iter().map(|(_, _, binding)| binding.clone()).collect();
    for input in &inputs {
        if let Some(prefix) = &input.prefix {
            args.push(prefix);
        }
        if let Some(value) = &input.value_from {
            args.push(value)
        }
    }

    //remove empty args
    args.retain(|s| !s.is_empty());

    let mut command = if tool.has_shell_command_requirement() {
        let joined_args = args.iter().map(|s| s.as_str()).collect::<Vec<&str>>().join(" ");
        let mut cmd = get_shell_command();
        cmd.arg(joined_args);
        cmd
    } else {
        let mut cmd = SystemCommand::new(args[0]);
        for arg in &args[1..] {
            cmd.arg(arg);
        }
        cmd
    };

    //build inputs from either fn-args or default values.

    //command.args(inputs);

    //append stdin i guess?
    if let Some(stdin) = &tool.stdin {
        command.arg(stdin);
    }

    //run
    eprintln!("⏳ Executing Command: `{}`", format_command(&command));
    let output = command.output()?;

    //handle redirection of stdout
    if !output.stdout.is_empty() {
        if let Some(stdout) = &tool.stdout {
            let out = &String::from_utf8_lossy(&output.stdout);
            create_and_write_file(stdout, out)?;
        } else {
            eprintln!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }

    //handle redirection of stderr
    if !output.stderr.is_empty() {
        if let Some(stderr) = &tool.stderr {
            let out = &String::from_utf8_lossy(&output.stderr);
            create_and_write_file(stderr, out)?;
        } else {
            eprintln!("❌ {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    match output.status.success() {
        true => Ok(()),
        false => Err(format!("command returned with code {:?}", output.status.code().unwrap_or(1)).into()),
    }
}
