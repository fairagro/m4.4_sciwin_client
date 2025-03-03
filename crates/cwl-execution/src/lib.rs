pub mod environment;
pub mod expression;
pub mod preprocess;
pub mod runner;
pub(crate) mod staging;
pub mod util;

use cwl::{clt::CommandLineTool, requirements::check_timelimit, types::DefaultValue, CWLDocument};
use environment::{collect_env_vars, collect_inputs, collect_outputs, RuntimeEnvironment};
use expression::{prepare_expression_engine, replace_expressions, reset_expression_engine};
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

pub fn execute(
    path: impl AsRef<Path>,
    inputs: HashMap<String, DefaultValue>,
    outdir: Option<impl AsRef<Path>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(&path)?;

    //preprocess import statements
    let contents = preprocess_imports(&contents, &path);

    //parse
    let doc: CWLDocument = serde_yaml::from_str(&contents).map_err(|e| format!("Could not parse CWL File: {e}"))?;

    match doc {
        CWLDocument::CommandLineTool(tool) => run_commandlinetool(&tool, inputs, path, outdir),
        _ => todo!(),
    }
}

fn run_commandlinetool(
    tool: &CommandLineTool,
    inputs: HashMap<String, DefaultValue>,
    tool_path: impl AsRef<Path>,
    outdir: Option<impl AsRef<Path>>,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let json = serde_json::to_string_pretty(&outputs)?;
    println!("{json}");

    env::set_current_dir(current_dir)?;
    reset_expression_engine()?;
    Ok(())
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
