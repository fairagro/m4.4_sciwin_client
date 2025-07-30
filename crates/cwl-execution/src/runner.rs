use crate::{
    CommandError, InputObject, container_engine,
    environment::{RuntimeEnvironment, collect_environment},
    execute,
    expression::{
        eval, eval_tool, evaluate_condition, load_lib, parse_expressions, prepare_expression_engine, process_expressions, replace_expressions,
        reset_expression_engine, set_self, unset_self,
    },
    format_command,
    io::{copy_dir, copy_file, create_and_write_file_forced, get_random_filename, get_shell_command, print_output, set_print_output},
    scatter::{self},
    staging::stage_required_files,
    util::{
        copy_output_dir, evaluate_command_outputs, evaluate_expression_outputs, evaluate_input, evaluate_input_as_string, get_file_metadata,
        is_docker_installed,
    },
    validate::set_placeholder_values,
};
use commonwl::{
    Argument, CWLDocument, CWLType, Command, CommandLineTool, DefaultValue, Directory, Entry, File, PathItem, ScatterMethod, SingularPlural,
    StringOrDocument, StringOrNumber, Workflow,
    inputs::{CommandLineBinding, LinkMerge},
    requirements::{DockerRequirement, InlineJavascriptRequirement, Requirement, StringOrInclude},
};
use log::{info, warn};
use rand::{Rng, distr::Alphanumeric};
use serde_yaml::Value;
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::{self},
    path::{MAIN_SEPARATOR_STR, Path, PathBuf},
    process::Command as SystemCommand,
    time::{Duration, Instant},
};
use tempfile::tempdir;
use wait_timeout::ChildExt;

pub fn run_workflow(
    workflow: &mut Workflow,
    input_values: &InputObject,
    cwl_path: &PathBuf,
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

    let workflow_folder = if cwl_path.is_file() {
        cwl_path.parent().unwrap_or(Path::new("."))
    } else {
        cwl_path
    };

    let input_values = input_values.handle_requirements(&workflow.requirements, &workflow.hints);

    //prevent tool from outputting
    set_print_output(false);

    let mut outputs: HashMap<String, DefaultValue> = HashMap::new();
    for step_id in sorted_step_ids {
        if let Some(step) = workflow.get_step(&step_id) {
            let path = if let StringOrDocument::String(run) = &step.run {
                Some(workflow_folder.join(run))
            } else {
                None
            };

            //map inputs to correct fields
            let mut step_inputs = HashMap::new();
            for parameter in &step.in_ {
                let source = parameter.source.as_deref().unwrap_or_default();
                let source_parts: Vec<&str> = source.split('/').collect();
                //try output
                if source_parts.len() == 2
                    && let Some(out_value) = outputs.get(source)
                {
                    step_inputs.insert(parameter.id.to_string(), out_value.to_default_value());
                    continue;
                }

                //try default
                if let Some(default) = &parameter.default {
                    step_inputs.entry(parameter.id.to_string()).or_insert(default.to_owned());
                }

                //try input
                if let Some(input) = workflow.inputs.iter().find(|i| i.id == *source) {
                    let value = evaluate_input(input, &input_values.inputs)?;
                    match value {
                        DefaultValue::Any(val) if val.is_null() => continue,
                        _ => {
                            step_inputs.insert(parameter.id.to_string(), value.clone());
                        }
                    }
                }
                if source.starts_with("[") {
                    //source can be array of input IDs if this requirement is set!
                    let array: Vec<String> = serde_yaml::from_str(source)?;
                    if workflow.has_requirement(Requirement::MultipleInputFeatureRequirement) {
                        let mut data = vec![];
                        for item in array {
                            if let Some(input) = workflow.inputs.iter().find(|i| i.id == item) {
                                let value = evaluate_input(input, &input_values.inputs)?;
                                match parameter.link_merge {
                                    None | Some(LinkMerge::MergeNested) => data.push(value),
                                    Some(LinkMerge::MergeFlattened) => {
                                        if let DefaultValue::Array(vec) = value {
                                            data.extend(vec);
                                        } else {
                                            return Err("Expected array for MergeFlattened".into());
                                        }
                                    }
                                }
                            } else {
                                return Err(format!("Could not find input: {item}").into());
                            }
                        }
                        step_inputs.insert(parameter.id.to_string(), DefaultValue::Array(data));
                    } else if array.len() == 1
                        && let Some(input) = workflow.inputs.iter().find(|i| i.id == array[0])
                    {
                        //if requirement is not set, but array is of length 1 we use first value or wrap into array if linkmerge tells to do
                        let value = evaluate_input(input, &input_values.inputs)?;
                        match parameter.link_merge {
                            Some(LinkMerge::MergeFlattened) | None => step_inputs.insert(parameter.id.to_string(), value),
                            Some(LinkMerge::MergeNested) => step_inputs.insert(parameter.id.to_string(), DefaultValue::Array(vec![value])),
                        };
                    }
                }
            }
            let mut input_values = input_values.handle_requirements(&step.requirements, &step.hints);
            input_values.inputs = step_inputs;

            //check conditional execution
            if let Some(condition) = &step.when {
                if workflow.cwl_version == Some("v1.0".to_string()) || workflow.cwl_version == Some("v1.1".to_string()) {
                    return Err(format!("Conditional execution is not supported in CWL {:?}", workflow.cwl_version).into());
                }
                if !evaluate_condition(condition, &input_values.inputs)? {
                    continue;
                }
            }

            //decide if we are going to use scatter or normal execution
            let step_outputs = if let Some(scatter) = &step.scatter
                && workflow.has_requirement(Requirement::ScatterFeatureRequirement)
            {
                //get input
                let scatter_keys = match scatter {
                    SingularPlural::Singular(item) => vec![item.clone()],
                    SingularPlural::Plural(items) => items.clone(),
                };

                let method = step.scatter_method.as_ref().unwrap_or(&ScatterMethod::DotProduct);

                let scatter_inputs = scatter::gather_inputs(&scatter_keys, &input_values)?;
                let jobs = scatter::gather_jobs(&scatter_inputs, &scatter_keys, method)?;

                let mut step_outputs: HashMap<String, Vec<DefaultValue>> = HashMap::new();
                for job in jobs {
                    let mut sub_inputs = input_values.clone();
                    for (k, v) in job {
                        sub_inputs.inputs.insert(k, v);
                    }

                    let singular_outputs = execute_step(step, sub_inputs, &path, workflow_folder, &tmp_path)?;

                    for (key, value) in singular_outputs {
                        step_outputs.entry(key).or_default().push(value);
                    }
                }
                step_outputs
                    .into_iter()
                    .map(|(k, v)| (k, DefaultValue::Array(v)))
                    .collect::<HashMap<_, _>>()
            } else {
                execute_step(step, input_values, &path, workflow_folder, &tmp_path)?
            };

            for (key, value) in step_outputs {
                outputs.insert(format!("{}/{}", step.id, key), value);
            }
        } else {
            return Err(format!("Could not find step {step_id}").into());
        }
    }

    set_print_output(true);

    fn output_file(file: &File, tmp_path: &str, output_directory: &str) -> Result<File, Box<dyn Error>> {
        let path = file.path.as_ref().map_or_else(String::new, |p| p.clone());
        let new_loc = Path::new(&path).to_string_lossy().replace(tmp_path, output_directory);
        copy_file(&path, &new_loc)?;
        let mut file = file.clone();
        file.path = Some(new_loc.to_string());
        file.location = Some(format!("file://{new_loc}"));
        Ok(file)
    }

    fn output_dir(dir: &Directory, tmp_path: &str, output_directory: &str) -> Result<Directory, Box<dyn Error>> {
        let path = dir.path.as_ref().map_or_else(String::new, |p| p.clone());
        let new_loc = Path::new(&path).to_string_lossy().replace(tmp_path, output_directory);
        copy_dir(&path, &new_loc)?;
        let mut dir = dir.clone();
        dir.path = Some(new_loc.to_string());
        dir.location = Some(format!("file://{new_loc}"));
        Ok(dir)
    }

    let mut output_values = HashMap::new();
    for output in &workflow.outputs {
        let source = &output.output_source;
        if let Some(value) = &outputs.get(source) {
            let value = match value {
                DefaultValue::File(file) => DefaultValue::File(output_file(file, &tmp_path, &output_directory)?),
                DefaultValue::Directory(dir) => DefaultValue::Directory(output_dir(dir, &tmp_path, &output_directory)?),
                DefaultValue::Any(value) => DefaultValue::Any(value.clone()),
                DefaultValue::Array(array) => DefaultValue::Array(
                    array
                        .iter()
                        .map(|item| {
                            Ok(match item {
                                DefaultValue::File(file) => DefaultValue::File(output_file(file, &tmp_path, &output_directory)?),
                                DefaultValue::Directory(dir) => DefaultValue::Directory(output_dir(dir, &tmp_path, &output_directory)?),
                                DefaultValue::Any(value) => DefaultValue::Any(value.clone()),
                                _ => item.clone(),
                            })
                        })
                        .collect::<Result<Vec<_>, Box<dyn Error>>>()?,
                ),
            };
            output_values.insert(&output.id, value.clone());
        } else if let Some(input) = workflow.inputs.iter().find(|i| i.id == *source) {
            let result = evaluate_input(input, &input_values.inputs)?;
            let value = match &result {
                DefaultValue::File(file) => {
                    let dest = format!("{}/{}", output_directory, file.get_location());
                    fs::copy(workflow_folder.join(file.get_location()), &dest).map_err(|e| format!("Could not copy file to {dest}: {e}"))?;
                    DefaultValue::File(get_file_metadata(Path::new(&dest).to_path_buf(), file.format.clone()))
                }
                DefaultValue::Directory(directory) => DefaultValue::Directory(
                    copy_output_dir(
                        workflow_folder.join(directory.get_location()),
                        format!("{}/{}", &output_directory, &directory.get_location()),
                    )
                    .map_err(|e| format!("Could not provide output directory: {e}"))?,
                ),
                DefaultValue::Any(inner) => DefaultValue::Any(inner.clone()),
                DefaultValue::Array(inner) => DefaultValue::Array(inner.clone()),
            };
            output_values.insert(&output.id, value);
        }
    }

    info!("✔️  Workflow {:?} executed successfully in {:.0?}!", &cwl_path, clock.elapsed());
    Ok(output_values.into_iter().map(|(k, v)| (k.clone(), v)).collect())
}

fn execute_step(
    step: &commonwl::WorkflowStep,
    input_values: InputObject,
    path: &Option<PathBuf>,
    workflow_folder: &Path,
    tmp_path: &str,
) -> Result<HashMap<String, DefaultValue>, Box<dyn Error>> {
    let step_outputs = if let Some(path) = path {
        execute(path, &input_values, Some(tmp_path), None)?
    } else if let StringOrDocument::Document(doc) = &step.run {
        execute(workflow_folder, &input_values, Some(tmp_path), Some(doc))?
    } else {
        unreachable!()
    };
    Ok(step_outputs)
}

pub fn run_tool(
    tool: &mut CWLDocument,
    input_values: &InputObject,
    cwl_path: &PathBuf,
    out_dir: Option<String>,
) -> Result<HashMap<String, DefaultValue>, Box<dyn Error>> {
    //measure performance
    let clock = Instant::now();
    if !print_output() {
        info!("🚲 Executing Tool {cwl_path:?} ...");
    }
    //create staging directory
    let dir = tempdir()?;
    info!("📁 Created staging directory: {:?}", dir.path());

    //save reference to current working directory
    let current = env::current_dir()?;
    let output_directory = if let Some(out) = out_dir { &PathBuf::from(out) } else { &current };

    //set tool path. all paths are given relative to the tool
    let tool_path = cwl_path.parent().unwrap_or(Path::new("."));

    //create runtime tmpdir
    let tmp_dir = tempdir()?;

    let mut input_values = input_values.handle_requirements(&tool.requirements, &tool.hints);
    input_values.lock();

    //build runtime object
    let mut runtime = RuntimeEnvironment::initialize(tool, &input_values, dir.path(), tool_path, tmp_dir.path())?;

    //replace inputs and runtime placeholders in tool with the actual values
    set_placeholder_values(tool, &runtime, &mut input_values);
    runtime.environment = collect_environment(&input_values);

    // run expression engine
    prepare_expression_engine(&runtime)?;
    if let Some(ijr) = input_values.get_requirement::<InlineJavascriptRequirement>() {
        if let Some(expression_lib) = &ijr.expression_lib {
            for lib in expression_lib {
                if let StringOrInclude::Include(lib_include) = lib {
                    load_lib(tool_path.join(&lib_include.include))?;
                } else if let StringOrInclude::String(lib_string) = lib {
                    eval(lib_string)?;
                }
            }
        }
        process_expressions(tool, &mut input_values)?;
    }

    //stage files listed in input default values, input values or initial work dir requirements
    stage_required_files(tool, &input_values, &mut runtime, tool_path, dir.path(), output_directory)?;

    //change working directory to tmp folder, we will execute tool from root here
    env::set_current_dir(dir.path())?;

    //run the tool
    let mut result_value: Option<serde_yaml::Value> = None;
    if let CWLDocument::CommandLineTool(clt) = tool {
        run_command(clt, &mut runtime).map_err(|e| CommandError {
            message: format!("Error in Tool execution: {e}"),
            exit_code: clt.get_error_code(),
        })?;
    } else if let CWLDocument::ExpressionTool(et) = tool {
        prepare_expression_engine(&runtime)?;
        let expressions = parse_expressions(&et.expression);
        result_value = Some(eval_tool::<serde_yaml::Value>(&expressions[0].expression())?);
        reset_expression_engine()?;
    }

    //evaluate output files
    prepare_expression_engine(&runtime)?;
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
    reset_expression_engine()?;

    //come back to original directory
    env::set_current_dir(current)?;

    info!("✔️  Tool {:?} executed successfully in {:.0?}!", &cwl_path, clock.elapsed());
    Ok(outputs)
}

pub fn run_command(tool: &CommandLineTool, runtime: &mut RuntimeEnvironment) -> Result<(), Box<dyn Error>> {
    let mut command = build_command(tool, runtime)?;

    if let Some(docker) = tool.get_docker_requirement() {
        if is_docker_installed() {
            command = build_docker_command(&mut command, docker, runtime);
        } else {
            eprintln!("Docker is not installed, can not use containerization on this system!");
            warn!("Docker is not installed, can not use");
        }
    }

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
    {
        let out = &String::from_utf8_lossy(&output.stdout);
        if let Some(stdout) = &tool.stdout {
            create_and_write_file_forced(stdout, out)?;
        } else if tool.has_stdout_output() {
            let output = tool.outputs.iter().filter(|o| matches!(o.type_, CWLType::Stdout)).collect::<Vec<_>>()[0];
            let filename = output
                .output_binding
                .as_ref()
                .and_then(|binding| binding.glob.clone())
                .unwrap_or_else(|| get_random_filename(&format!("{}_stdout", output.id), "out"));
            create_and_write_file_forced(filename, out)?;
        } else if !output.stdout.is_empty() {
            eprintln!("{out}");
        }
    }
    //handle redirection of stderr
    {
        let out = &String::from_utf8_lossy(&output.stderr);
        if let Some(stderr) = &tool.stderr {
            create_and_write_file_forced(stderr, out)?;
        } else if tool.has_stderr_output() {
            let output = tool.outputs.iter().filter(|o| matches!(o.type_, CWLType::Stderr)).collect::<Vec<_>>()[0];
            let filename = output
                .output_binding
                .as_ref()
                .and_then(|binding| binding.glob.clone())
                .unwrap_or_else(|| get_random_filename(&format!("{}_stderr", output.id), "out"));
            create_and_write_file_forced(filename, out)?;
        } else if !output.stderr.is_empty() {
            eprintln!("❌ {out}");
        }
    }

    let status_code = output.status.code().unwrap_or(1);
    runtime
        .runtime
        .insert("exitCode".to_string(), StringOrNumber::Integer(status_code as u64));

    if output.status.success() || tool.get_sucess_code() == status_code {
        Ok(()) //fails expectedly
    } else {
        Err(format!("command returned with code {status_code:?}").into())
    }
}

#[derive(Debug, Clone)]
struct BoundBinding {
    sort_key: Vec<SortKey>,
    command: CommandLineBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SortKey {
    Int(i32),
    Str(String),
}

fn build_command(tool: &CommandLineTool, runtime: &RuntimeEnvironment) -> Result<SystemCommand, Box<dyn Error>> {
    let mut args: Vec<String> = vec![];

    //get executable
    let cmd = match &tool.base_command {
        Command::Single(cmd) => cmd,
        Command::Multiple(vec) => {
            if vec.is_empty() {
                &String::new()
            } else {
                &vec[0]
            }
        }
    };

    if !cmd.is_empty() {
        args.push(cmd.to_string());
        //append rest of base command as args
        if let Command::Multiple(vec) = &tool.base_command {
            args.extend(vec[1..].iter().cloned());
        }
    }

    let mut bindings: Vec<BoundBinding> = vec![];

    //handle arguments field...
    if let Some(arguments) = &tool.arguments {
        for (i, arg) in arguments.iter().enumerate() {
            let mut sort_key = vec![];
            match arg {
                Argument::String(str) => {
                    let binding = CommandLineBinding {
                        value_from: Some(str.clone()),
                        ..Default::default()
                    };
                    sort_key.push(SortKey::Int(0));
                    sort_key.push(SortKey::Int(i32::try_from(i)?));
                    bindings.push(BoundBinding { sort_key, command: binding });
                }
                Argument::Binding(binding) => {
                    let position = i32::try_from(binding.position.unwrap_or_default())?;
                    sort_key.push(SortKey::Int(position));
                    sort_key.push(SortKey::Int(i32::try_from(i)?));
                    bindings.push(BoundBinding {
                        sort_key,
                        command: binding.clone(),
                    });
                }
            }
        }
    }

    //handle inputs
    for input in &tool.inputs {
        if let Some(binding) = &input.input_binding {
            let mut binding = binding.clone();
            let position = binding.position.unwrap_or_default();
            let mut sort_key = vec![SortKey::Int(i32::try_from(position)?), SortKey::Str(input.id.clone())];

            let value = runtime.inputs.get(&input.id);
            set_self(&value)?;
            if let Some(value_from) = &binding.value_from {
                if let Some(val) = value {
                    if let DefaultValue::Any(Value::Null) = val {
                        continue;
                    } else {
                        binding.value_from = Some(replace_expressions(value_from).unwrap_or(value_from.to_string()));
                    }
                }
            } else if matches!(input.type_, CWLType::Array(_)) {
                let val = evaluate_input(input, &runtime.inputs)?;
                if let DefaultValue::Array(vec) = val {
                    if vec.is_empty() {
                        continue;
                    }
                    if let Some(sep) = &binding.item_separator {
                        binding.value_from = Some(vec.iter().map(|i| i.as_value_string()).collect::<Vec<_>>().join(sep).to_string());
                    } else {
                        for (i, item) in vec.iter().enumerate() {
                            binding.value_from = Some(item.as_value_string());
                            sort_key.push(SortKey::Int(i32::try_from(i)?));
                            bindings.push(BoundBinding {
                                sort_key: sort_key.clone(),
                                command: binding.clone(),
                            });
                        }
                        unset_self()?;
                        continue;
                    }
                }
            } else {
                let binding_str = evaluate_input_as_string(input, &runtime.inputs)?;
                if matches!(input.type_, CWLType::Optional(_)) && binding_str == "null" {
                    continue;
                }
                binding.value_from = Some(binding_str.replace("'", ""));
            }
            unset_self()?;
            bindings.push(BoundBinding {
                sort_key,
                command: binding.clone(),
            })
        }
    }

    //do sorting
    bindings.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

    //add bindings
    let inputs: Vec<CommandLineBinding> = bindings.iter().map(|b| b.command.clone()).collect();
    for input in &inputs {
        if let Some(prefix) = &input.prefix {
            args.push(prefix.to_string());
        }
        if let Some(value) = &input.value_from {
            if tool.has_shell_command_requirement() {
                if let Some(shellquote) = input.shell_quote {
                    if shellquote {
                        args.push(format!("\"{value}\""));
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
    command.env(
        "HOME",
        runtime
            .runtime
            .get("outdir")
            .unwrap_or(&StringOrNumber::String(current_dir.clone()))
            .to_string(),
    );
    command.env(
        "TMPDIR",
        runtime
            .runtime
            .get("tmpdir")
            .unwrap_or(&StringOrNumber::String(current_dir.clone()))
            .to_string(),
    );
    command.current_dir(
        runtime
            .runtime
            .get("outdir")
            .unwrap_or(&StringOrNumber::String(current_dir.clone()))
            .to_string(),
    );

    Ok(command)
}

fn build_docker_command(command: &mut SystemCommand, docker: &DockerRequirement, runtime: &RuntimeEnvironment) -> SystemCommand {
    let container_engine = container_engine().to_string();

    let docker_image = if let Some(pull) = &docker.docker_pull {
        pull
    } else if let (Some(docker_file), Some(docker_image_id)) = (&docker.docker_file, &docker.docker_image_id) {
        let path = match docker_file {
            Entry::Include(include) => include.include.clone(),
            Entry::Source(src) => {
                let path = format!("{}/Dockerfile", runtime.runtime["tmpdir"]);
                fs::write(&path, src).unwrap();
                path
            }
        };
        let path = path.trim_start_matches(&("..".to_owned() + MAIN_SEPARATOR_STR)).to_string();

        let mut build = SystemCommand::new(&container_engine);
        build.args(["build", "-f", &path, "-t", docker_image_id, "."]);
        let output = build.output().expect("Could not build container!");
        println!("{}", String::from_utf8_lossy(&output.stderr));
        docker_image_id
    } else {
        unreachable!()
    };
    let mut docker_command = SystemCommand::new(&container_engine);

    //create workdir vars
    let workdir = if let Some(docker_output_directory) = &docker.docker_output_directory {
        docker_output_directory
    } else {
        &format!("/{}", rand::rng().sample_iter(&Alphanumeric).take(5).map(char::from).collect::<String>())
    };
    let outdir = &runtime.runtime["outdir"];
    let tmpdir = &runtime.runtime["tmpdir"];

    let workdir_mount = format!("--mount=type=bind,source={outdir},target={workdir}");
    let tmpdir_mount = format!("--mount=type=bind,source={tmpdir},target=/tmp");
    let workdir_arg = format!("--workdir={}", &workdir);
    docker_command.args(["run", "-i", &workdir_mount, &tmpdir_mount, &workdir_arg, "--rm"]);
    #[cfg(unix)]
    {
        docker_command.arg(get_user_flag());
    }
    //add all environment vars
    docker_command.arg(format!("--env=HOME={}", &workdir));
    docker_command.arg("--env=TMPDIR=/tmp");
    for (key, val) in command.get_envs().skip_while(|(key, _)| *key == "HOME" || *key == "TMPDIR") {
        docker_command.arg(format!("--env={}={}", key.to_string_lossy(), val.unwrap().to_string_lossy()));
    }

    if let Some(StringOrNumber::Integer(i)) = runtime.runtime.get("network") {
        if *i != 1 {
            docker_command.arg("--net=none");
        }
        //net enabled if i == 1 = not append arg
    } else {
        docker_command.arg("--net=none");
    }

    docker_command.arg(docker_image);
    docker_command.arg(command.get_program());

    //rewrite home dir
    let args = command
        .get_args()
        .map(|arg| {
            arg.to_string_lossy()
                .into_owned()
                .replace(&runtime.runtime["outdir"].to_string(), workdir)
                .replace("\\", "/")
        })
        .collect::<Vec<_>>();
    docker_command.args(args);

    docker_command
}

#[cfg(unix)]
fn get_user_flag() -> String {
    use nix::unistd::{getgid, getuid};
    format!("--user={}:{}", getuid().as_raw(), getgid().as_raw())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set_container_engine;
    use commonwl::load_tool;

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

    #[test]
    fn test_build_command_docker() {
        set_container_engine(crate::ContainerEngine::Docker);
        //tool has docker requirement
        let tool = load_tool("../../tests/test_data/hello_world/workflows/calculation/calculation.cwl").unwrap();
        let runtime = RuntimeEnvironment {
            runtime: HashMap::from([
                ("outdir".to_string(), StringOrNumber::String("testdir".to_string())),
                ("tmpdir".to_string(), StringOrNumber::String("testdir".to_string())),
            ]),
            ..Default::default()
        };

        let mut cmd = build_command(&tool, &runtime).unwrap();
        let cmd = build_docker_command(&mut cmd, tool.get_docker_requirement().unwrap(), &runtime);
        print!("{}", format_command(&cmd));
        assert!(cmd.get_program().to_string_lossy().contains("docker"));
    }

    #[test]
    fn test_build_command_podman() {
        set_container_engine(crate::ContainerEngine::Podman);

        //tool has docker requirement
        let tool = load_tool("../../tests/test_data/hello_world/workflows/calculation/calculation.cwl").unwrap();
        let runtime = RuntimeEnvironment {
            runtime: HashMap::from([
                ("outdir".to_string(), StringOrNumber::String("testdir".to_string())),
                ("tmpdir".to_string(), StringOrNumber::String("testdir".to_string())),
            ]),
            ..Default::default()
        };

        let mut cmd = build_command(&tool, &runtime).unwrap();
        let cmd = build_docker_command(&mut cmd, tool.get_docker_requirement().unwrap(), &runtime);
        print!("{}", format_command(&cmd));
        assert!(cmd.get_program().to_string_lossy().contains("podman"));
    }
}
