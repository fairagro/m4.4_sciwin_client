pub mod environment;
pub mod expression;
pub mod preprocess;
pub mod runner;
pub(crate) mod staging;
pub mod util;

use cwl::{
    clt::CommandLineTool,
    inputs::WorkflowStepInput,
    requirements::check_timelimit,
    types::{DefaultValue, Directory, File, OutputItem},
    wf::Workflow,
    CWLDocument,
};
use environment::{collect_env_vars, collect_inputs, collect_outputs, evaluate_input, RuntimeEnvironment};
use expression::{prepare_expression_engine, replace_expressions, reset_expression_engine};
use pathdiff::diff_paths;
use preprocess::{preprocess_imports, process_expressions};
use runner::run_command;
use staging::{stage_input_files, stage_required_files, unstage_required_files};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};
use std::{error::Error, fmt::Display};
use tempfile::tempdir;
use util::{copy_dir, copy_file};

pub fn execute(
    path: impl AsRef<Path>,
    inputs: HashMap<String, DefaultValue>,
    outdir: Option<impl AsRef<Path>>,
) -> Result<HashMap<String, OutputItem>, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(&path)?;

    //preprocess import statements
    let contents = preprocess_imports(&contents, &path);

    //parse
    let doc: CWLDocument = serde_yaml::from_str(&contents).map_err(|e| format!("Could not parse CWL File: {e}"))?;

    let outputs = match doc {
        CWLDocument::CommandLineTool(tool) => run_commandlinetool(&tool, inputs, path, outdir)?,
        CWLDocument::Workflow(workflow) => run_workflow(&workflow, inputs, path, outdir)?,
    };
    Ok(outputs)
}

fn run_workflow(
    workflow: &Workflow,
    inputs: HashMap<String, DefaultValue>,
    workflow_path: impl AsRef<Path>,
    outdir: Option<impl AsRef<Path>>,
) -> Result<HashMap<String, OutputItem>, Box<dyn std::error::Error>> {
    let sorted_step_ids = workflow.sort_steps()?;
    let current = env::current_dir()?;
    let dir = tempdir()?;
    let workflow_dir = workflow_path.as_ref().parent().unwrap_or(Path::new("."));
    let output_dir = outdir.map(|d| d.as_ref().to_path_buf()).unwrap_or(current);

    let mut outputs: HashMap<String, OutputItem> = HashMap::new();
    for step_id in sorted_step_ids {
        if let Some(step) = workflow.get_step(&step_id) {
            let path = workflow_dir.join(step.run.clone());

            //map inputs to correct fields
            let mut step_inputs = HashMap::new();
            for (key, input) in &step.in_ {
                match input {
                    WorkflowStepInput::String(in_string) => {
                        let parts: Vec<&str> = in_string.split('/').collect();
                        if parts.len() == 2 {
                            step_inputs.insert(key.to_string(), outputs.get(in_string).unwrap().to_default_value());
                        } else if let Some(input) = workflow.inputs.iter().find(|i| i.id == *in_string) {
                            let (_, value) = evaluate_input(input, &inputs)?;
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
                            let (_, value) = evaluate_input(input, &inputs)?;
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
            let step_outputs = execute(path, step_inputs, Some(dir.path()))?;
            for (key, value) in step_outputs {
                outputs.insert(format!("{}/{}", step.id, key), value);
            }
        } else {
            return Err(format!("Could not find step {}", step_id).into());
        }
    }
    let mut output_values: HashMap<String, OutputItem> = HashMap::new();
    for output in &workflow.outputs {
        let source = &output.output_source;
        if let Some(value) = outputs.get(source) {
            if let OutputItem::Value(value) = value {
                let value = match &value {
                    DefaultValue::File(file) => {
                        let relative = diff_paths(file.path.as_ref().unwrap(), dir.path()).unwrap_or(PathBuf::from(file.basename.as_ref().unwrap()));
                        let destination = output_dir.join(relative);
                        copy_file(file.path.as_ref().unwrap(), &destination)?;
                        DefaultValue::File(File::from_file(destination, file.format.clone()))
                    }
                    DefaultValue::Directory(directory) => {
                        let relative =
                            diff_paths(directory.path.as_ref().unwrap(), dir.path()).unwrap_or(PathBuf::from(directory.basename.as_ref().unwrap()));
                        let destination = output_dir.join(relative);
                        copy_dir(directory.path.as_ref().unwrap(), &destination)?;
                        DefaultValue::Directory(Directory::from_path(destination))
                    }
                    DefaultValue::Any(str) => DefaultValue::Any(str.clone()),
                };
                output_values.insert(output.id.clone(), OutputItem::Value(value));
            } //todo: arrays
        } else if let Some(input) = workflow.inputs.iter().find(|i| i.id == *source) {
            let (_, result) = evaluate_input(input, &inputs)?;
            let value = match &result {
                DefaultValue::File(file) => {
                    let relative = diff_paths(file.path.as_ref().unwrap(), dir.path()).unwrap_or(PathBuf::from(file.basename.as_ref().unwrap()));
                    let destination = output_dir.join(relative);
                    copy_file(file.path.as_ref().unwrap(), &destination)?;
                    DefaultValue::File(File::from_file(destination, file.format.clone()))
                }
                DefaultValue::Directory(directory) => {
                    let relative =
                        diff_paths(directory.path.as_ref().unwrap(), dir.path()).unwrap_or(PathBuf::from(directory.basename.as_ref().unwrap()));
                    let destination = output_dir.join(relative);
                    copy_dir(directory.path.as_ref().unwrap(), &destination)?;
                    DefaultValue::Directory(Directory::from_path(destination))
                }
                DefaultValue::Any(inner) => DefaultValue::Any(inner.clone()),
            };
            output_values.insert(output.id.clone(), OutputItem::Value(value.clone()));
        }
    }
    Ok(output_values)
}

fn run_commandlinetool(
    tool: &CommandLineTool,
    inputs: HashMap<String, DefaultValue>,
    tool_path: impl AsRef<Path>,
    outdir: Option<impl AsRef<Path>>,
) -> Result<HashMap<String, OutputItem>, Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let runtime_dir = dir.path();
    let tool_dir = tool_path.as_ref().parent().unwrap_or(Path::new("."));
    let current_dir = env::current_dir()?;
    let out_dir: &PathBuf = &outdir.map(|d| d.as_ref().to_path_buf()).unwrap_or(current_dir.clone());

    env::set_current_dir(tool_dir)?;
    let mut runtime = RuntimeEnvironment {
        runtime: HashMap::from([
            ("tooldir".to_string(), tool_dir.to_string_lossy().into_owned()),
            ("outdir".to_string(), runtime_dir.to_string_lossy().into_owned()),
            ("tmpdir".to_string(), runtime_dir.to_string_lossy().into_owned()),
            ("cores".to_string(), 0.to_string()),
            ("ram".to_string(), 0.to_string()),
        ]),
        inputs: collect_inputs(tool, &inputs)?,
        time_limit: check_timelimit(tool).unwrap_or(0),
        ..Default::default()
    };
    stage_input_files(&mut runtime, runtime_dir)?;

    prepare_expression_engine(&runtime)?;
    let mut tool = tool.clone(); //make tool mutable
    process_expressions(&mut tool);
    runtime.environment = collect_env_vars(&tool);
    stage_required_files(&tool, runtime_dir)?;

    run_command(&tool, Some(&runtime)).map_err(|e| CommandError {
        message: format!("Error in Tool execution: {}", e),
        exit_code: tool.get_error_code(),
    })?;

    unstage_required_files(&tool, runtime_dir)?;
    let outputs = collect_outputs(&tool, out_dir, &runtime)?;

    env::set_current_dir(current_dir)?;
    reset_expression_engine()?;
    Ok(outputs)
}

pub trait ExitCode {
    fn exit_code(&self) -> i32;
}

#[derive(Debug)]
pub struct CommandError {
    pub message: String,
    pub exit_code: i32,
}

impl Error for CommandError {}

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, code: {}", self.message, self.exit_code)
    }
}

impl ExitCode for CommandError {
    fn exit_code(&self) -> i32 {
        self.exit_code
    }
}
