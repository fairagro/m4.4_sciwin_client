use crate::{
    environment::{collect_environment, collect_inputs, RuntimeEnvironment},
    execute,
    expression::{eval_tool, parse_expressions, prepare_expression_engine, reset_expression_engine},
    format_command, get_available_ram, get_processor_count,
    io::{copy_dir, copy_file, create_and_write_file_forced, get_random_filename, get_shell_command, print_output, set_print_output},
    staging::{stage_required_files, unstage_files},
    util::{copy_output_dir, evaluate_command_outputs, evaluate_expression_outputs, evaluate_input, evaluate_input_as_string, get_file_metadata},
    validate::set_placeholder_values,
    CommandError,
};
use cwl::{
    clt::{Argument, Command, CommandLineTool},
    inputs::{CommandLineBinding, WorkflowStepInput},
    requirements::check_timelimit,
    types::{CWLType, DefaultValue, PathItem},
    wf::Workflow,
    CWLDocument,
};
use log::info;
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::{self},
    path::{Path, PathBuf},
    process::Command as SystemCommand,
    time::{Duration, Instant},
};
use tempfile::tempdir;
use wait_timeout::ChildExt;

pub fn run_workflow(
    workflow: &mut Workflow,
    input_values: HashMap<String, DefaultValue>,
    cwl_path: Option<&PathBuf>,
    out_dir: Option<String>,
) -> Result<HashMap<String, DefaultValue>, Box<dyn Error>> {
    let clock = Instant::now();

    let sorted_step_ids = workflow.sort_steps()?;

    let dir = tempdir()?;
    let tmp_path = dir.path().to_string_lossy().into_owned();
    let current = env::current_dir()?;
    let output_directory = if let Some(out) = out_dir {
        out
    } else {
        current.to_string_lossy().into_owned()
    };

    let workflow_folder = cwl_path.unwrap().parent().unwrap_or(Path::new("."));

    //prevent tool from outputting
    set_print_output(false);

    let mut outputs: HashMap<String, DefaultValue> = HashMap::new();
    for step_id in sorted_step_ids {
        if let Some(step) = workflow.get_step(&step_id) {
            let path = workflow_folder.join(step.run.clone());

            //map inputs to correct fields
            let mut step_inputs = HashMap::new();

            for (key, input) in &step.in_ {
                match input {
                    WorkflowStepInput::String(in_string) => {
                        let parts: Vec<&str> = in_string.split('/').collect();
                        if parts.len() == 2 {
                            step_inputs.insert(key.to_string(), outputs.get(in_string).unwrap().to_default_value());
                        } else if let Some(input) = workflow.inputs.iter().find(|i| i.id == *in_string) {
                            let value = evaluate_input(input, &input_values.clone())?;
                            step_inputs.insert(key.to_string(), value.to_owned());
                        }
                    }
                    WorkflowStepInput::Parameter(parameter) => {
                        let source = parameter.source.clone().unwrap_or_default();
                        let source_parts: Vec<&str> = source.split('/').collect();
                        if source_parts.len() == 2 {
                            //handle default
                            if let Some(out_value) = outputs.get(&source) {
                                step_inputs.insert(key.to_string(), out_value.to_default_value());
                            } else if let Some(default) = &parameter.default {
                                step_inputs.insert(key.to_string(), default.to_owned());
                            }
                        } else if let Some(default) = &parameter.default {
                            step_inputs.insert(key.to_string(), default.to_owned());
                        }
                        if let Some(input) = workflow.inputs.iter().find(|i| i.id == *source) {
                            let value = evaluate_input(input, &input_values.clone())?;
                            if step_inputs.contains_key(key) {
                                if let DefaultValue::Any(val) = &value {
                                    if val.is_null() {
                                        continue; //do not overwrite existing value with null
                                    }
                                }
                            }
                            step_inputs.insert(key.to_string(), value.to_owned());
                        }
                    }
                }
            }

            let step_outputs = execute(&path, step_inputs, Some(tmp_path.clone()))?;
            for (key, value) in step_outputs {
                outputs.insert(format!("{}/{}", step.id, key), value);
            }
        } else {
            return Err(format!("Could not find step {}", step_id).into());
        }
    }

    set_print_output(true);

    let mut output_values = HashMap::new();
    for output in &workflow.outputs {
        let source = &output.output_source;
        if let Some(value) = &outputs.get(source) {
            let value = match value {
                DefaultValue::File(file) => {
                    let path = file.path.as_ref().map_or_else(String::new, |p| p.clone());
                    let new_loc = Path::new(&path).to_string_lossy().replace(&tmp_path, &output_directory);
                    copy_file(&path, &new_loc)?;
                    let mut file = file.clone();
                    file.path = Some(new_loc.to_string());
                    file.location = Some(format!("file://{}", new_loc));
                    DefaultValue::File(file)
                }
                DefaultValue::Directory(dir) => {
                    let path = dir.path.as_ref().map_or_else(String::new, |p| p.clone());
                    let new_loc = Path::new(&path).to_string_lossy().replace(&tmp_path, &output_directory);
                    copy_dir(&path, &new_loc)?;
                    let mut dir = dir.clone();
                    dir.path = Some(new_loc.to_string());
                    dir.location = Some(format!("file://{}", new_loc));
                    DefaultValue::Directory(dir)
                }
                DefaultValue::Any(str) => DefaultValue::Any(str.clone()),
            };
            output_values.insert(&output.id, value.clone());
        } else if let Some(input) = workflow.inputs.iter().find(|i| i.id == *source) {
            let result = evaluate_input(input, &input_values)?;
            let value = match &result {
                DefaultValue::File(file) => {
                    let dest = format!("{}/{}", output_directory, file.get_location());
                    fs::copy(workflow_folder.join(file.get_location()), &dest).map_err(|e| format!("Could not copy file to {}: {}", dest, e))?;
                    DefaultValue::File(get_file_metadata(Path::new(&dest).to_path_buf(), file.format.clone()))
                }
                DefaultValue::Directory(directory) => DefaultValue::Directory(
                    copy_output_dir(
                        workflow_folder.join(directory.get_location()),
                        format!("{}/{}", &output_directory, &directory.get_location()),
                    )
                    .map_err(|e| format!("Could not provide output directory: {}", e))?,
                ),
                DefaultValue::Any(inner) => DefaultValue::Any(inner.clone()),
            };
            output_values.insert(&output.id, value);
        }
    }

    info!(
        "✔️  Workflow {:?} executed successfully in {:.0?}!",
        &cwl_path.unwrap_or(&PathBuf::default()),
        clock.elapsed()
    );
    Ok(output_values.into_iter().map(|(k, v)| (k.clone(), v)).collect())
}

pub fn run_tool(
    tool: &mut CWLDocument,
    input_values: HashMap<String, DefaultValue>,
    cwl_path: Option<&PathBuf>,
    out_dir: Option<String>,
) -> Result<HashMap<String, DefaultValue>, Box<dyn Error>> {
    //measure performance
    let clock = Instant::now();
    if !print_output() {
        info!("🚲 Executing Tool {:?} ...", cwl_path.unwrap_or(&PathBuf::default()));
    }
    //create staging directory
    let dir = tempdir()?;
    info!("📁 Created staging directory: {:?}", dir.path());

    //save reference to current working directory
    let current = env::current_dir()?;
    let output_directory = if let Some(out) = out_dir { &PathBuf::from(out) } else { &current };

    //set tool path. all paths are given relative to the tool
    let tool_path = if let Some(file) = cwl_path.as_ref() {
        file.parent().unwrap()
    } else {
        Path::new(".")
    };

    //create runtime tmpdir
    let tmp_dir = tempdir()?;

    //build runtime object
    let mut runtime = RuntimeEnvironment {
        runtime: HashMap::from([
            (
                "tooldir".to_string(),
                tool_path.parent().unwrap_or(Path::new(".")).to_string_lossy().into_owned(),
            ),
            ("outdir".to_string(), dir.path().to_string_lossy().into_owned()),
            ("tmpdir".to_string(), tmp_dir.path().to_string_lossy().into_owned()),
            ("cores".to_string(), get_processor_count().to_string()),
            ("ram".to_string(), get_available_ram().to_string()),
        ]),
        time_limit: check_timelimit(tool).unwrap_or(0),
        inputs: collect_inputs(tool, input_values)?,
        ..Default::default()
    };

    //replace inputs and runtime placeholders in tool with the actual values
    set_placeholder_values(tool, &runtime);
    runtime.environment = collect_environment(tool);

    //stage files listed in input default values, input values or initial work dir requirements
    let staged_files = stage_required_files(tool, &mut runtime.inputs, tool_path, dir.path(), output_directory)?;

    //change working directory to tmp folder, we will execute tool from root here
    env::set_current_dir(dir.path())?;

    //run the tool
    let mut result_value: Option<serde_yaml::Value> = None;
    if let CWLDocument::CommandLineTool(clt) = tool {
        run_command(clt, &runtime).map_err(|e| CommandError {
            message: format!("Error in Tool execution: {}", e),
            exit_code: clt.get_error_code(),
        })?;
    } else if let CWLDocument::ExpressionTool(et) = tool {
        prepare_expression_engine(&runtime)?;
        let expressions = parse_expressions(&et.expression);
        result_value = Some(eval_tool::<serde_yaml::Value>(&expressions[0].expression())?);
        reset_expression_engine()?;
    }

    //remove staged files
    let outputs = match &tool {
        CWLDocument::CommandLineTool(clt) => &clt.outputs,
        CWLDocument::ExpressionTool(et) => &et.outputs,
        CWLDocument::Workflow(_) => unreachable!(),
    };
    unstage_files(&staged_files, dir.path(), outputs)?;

    //evaluate output files
    let outputs = if let CWLDocument::CommandLineTool(clt) = &tool {
        evaluate_command_outputs(clt, output_directory)?
    } else if let CWLDocument::ExpressionTool(et) = &tool {
        if let Some(value) = result_value {
            evaluate_expression_outputs(et, value)?
        } else {
            HashMap::new()
        }
    } else {
        unreachable!()
    };
    //come back to original directory
    env::set_current_dir(current)?;

    info!(
        "✔️  Tool {:?} executed successfully in {:.0?}!",
        &cwl_path.unwrap_or(&PathBuf::default()),
        clock.elapsed()
    );
    Ok(outputs)
}

pub fn run_command(tool: &CommandLineTool, runtime: &RuntimeEnvironment) -> Result<(), Box<dyn Error>> {
    let mut command = build_command(tool, runtime)?;

    //run
    info!("⏳ Executing Command: `{}`", format_command(&command));

    let output = if runtime.time_limit > 0 {
        let mut child = command.spawn()?;
        if child.wait_timeout(Duration::from_secs(runtime.time_limit))?.is_none() {
            child.kill()?;
            return Err("Time elapsed".into());
        }
        child.wait_with_output()?
    } else {
        command.output()?
    };

    //handle redirection of stdout
    if !output.stdout.is_empty() {
        let out = &String::from_utf8_lossy(&output.stdout);
        if let Some(stdout) = &tool.stdout {
            create_and_write_file_forced(stdout, out)?;
        } else if tool.has_stdout_output() {
            let output = tool.outputs.iter().filter(|o| matches!(o.type_, CWLType::Stdout)).collect::<Vec<_>>()[0];
            let filename = if let Some(binding) = &output.output_binding {
                &binding.glob
            } else {
                &get_random_filename(&format!("{}_stdout", output.id), "out")
            };
            create_and_write_file_forced(filename, out)?;
        } else {
            eprintln!("{}", out);
        }
    }
    //handle redirection of stderr
    if !output.stderr.is_empty() {
        let out = &String::from_utf8_lossy(&output.stderr);
        if let Some(stderr) = &tool.stderr {
            create_and_write_file_forced(stderr, out)?;
        } else if tool.has_stderr_output() {
            let output = tool.outputs.iter().filter(|o| matches!(o.type_, CWLType::Stderr)).collect::<Vec<_>>()[0];
            let filename = if let Some(binding) = &output.output_binding {
                &binding.glob
            } else {
                &get_random_filename(&format!("{}_stderr", output.id), "out")
            };
            create_and_write_file_forced(filename, out)?;
        } else {
            eprintln!("❌ {}", out);
        }
    }

    match output.status.success() {
        true => Ok(()),
        false => Err(format!("command returned with code {:?}", output.status.code().unwrap_or(1)).into()),
    }
}

fn build_command(tool: &CommandLineTool, runtime: &RuntimeEnvironment) -> Result<SystemCommand, Box<dyn Error>> {
    let mut args: Vec<String> = vec![];

    //get executable
    let cmd = match &tool.base_command {
        Command::Single(cmd) => cmd,
        Command::Multiple(vec) => {
            if !vec.is_empty() {
                &vec[0]
            } else {
                &String::new()
            }
        }
    };

    if !cmd.is_empty() {
        args.push(cmd.to_string());
        //append rest of base command as args
        if let Command::Multiple(ref vec) = &tool.base_command {
            args.extend(vec[1..].iter().cloned());
        }
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
            binding.value_from = Some(evaluate_input_as_string(input, &runtime.inputs)?.replace("'", ""));
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
            args.push(prefix.to_string());
        }
        if let Some(value) = &input.value_from {
            if tool.has_shell_command_requirement() {
                if let Some(shellquote) = input.shell_quote {
                    if shellquote {
                        args.push(format!("\"{}\"", value));
                    } else {
                        args.push(value.to_string())
                    }
                } else {
                    args.push(value.to_string())
                }
            } else {
                args.push(value.to_string())
            }
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
        let mut cmd = SystemCommand::new(args[0].clone());
        for arg in &args[1..] {
            cmd.arg(arg);
        }
        cmd
    };

    //append stdin i guess?
    if let Some(stdin) = &tool.stdin {
        command.arg(stdin);
    }

    let current_dir = env::current_dir()?.to_string_lossy().into_owned();

    //set environment for run
    command.envs(runtime.environment.clone());
    command.env("HOME", runtime.runtime.get("outdir").unwrap_or(&current_dir));
    command.env("TMPDIR", runtime.runtime.get("tmpdir").unwrap_or(&current_dir));
    command.current_dir(runtime.runtime.get("outdir").unwrap_or(&current_dir));

    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command() {
        let yaml = r"
class: CommandLineTool
cwlVersion: v1.2
inputs:
  file1: 
    type: File
    inputBinding: {position: 0}
outputs:
  output_file:
    type: File
    outputBinding: {glob: output.txt}
baseCommand: cat
stdout: output.txt";
        let tool = &serde_yaml::from_str(yaml).unwrap();

        let inputs = r#"{
    "file1": {
        "class": "File",
        "location": "hello.txt"
    }
}"#;

        let input_values = serde_json::from_str(inputs).unwrap();
        let runtime = RuntimeEnvironment {
            inputs: input_values,
            ..Default::default()
        };
        let cmd = build_command(tool, &runtime).unwrap();

        assert_eq!(format_command(&cmd), "cat hello.txt");
    }

    #[test]
    fn test_build_command_stdin() {
        let yaml = r"
class: CommandLineTool
cwlVersion: v1.2
inputs: []
outputs: []
baseCommand: [cat]
stdin: hello.txt";
        let tool = &serde_yaml::from_str(yaml).unwrap();

        let cmd = build_command(tool, &Default::default()).unwrap();

        assert_eq!(format_command(&cmd), "cat hello.txt");
    }

    #[test]
    fn test_build_command_args() {
        let yaml = r#"class: CommandLineTool
cwlVersion: v1.2
requirements:
  - class: ShellCommandRequirement
inputs:
  indir: Directory
outputs:
  outlist:
    type: File
    outputBinding:
      glob: output.txt
arguments: ["cd", "$(inputs.indir.path)",
  {shellQuote: false, valueFrom: "&&"},
  "find", ".",
  {shellQuote: false, valueFrom: "|"},
  "sort"]
stdout: output.txt"#;
        let in_yaml = r"indir:
  class: Directory
  location: testdir";
        let tool = &serde_yaml::from_str(yaml).unwrap();
        let input_values = serde_yaml::from_str(in_yaml).unwrap();
        let runtime = RuntimeEnvironment {
            inputs: input_values,
            ..Default::default()
        };
        let cmd = build_command(tool, &runtime).unwrap();

        let shell_cmd = get_shell_command();
        let shell = shell_cmd.get_program().to_string_lossy();
        let c_arg = shell_cmd.get_args().collect::<Vec<_>>()[0].to_string_lossy();

        assert_eq!(format_command(&cmd), format!("{shell} {c_arg} cd $(inputs.indir.path) && find . | sort"));
    }
}
