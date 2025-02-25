pub mod environment;
pub mod expression;
pub mod util;

use cwl::{clt::CommandLineTool, inputs, types::DefaultValue, CWLDocument};
use environment::{collect_env_vars, collect_inputs, RuntimeEnvironment};
use expression::{eval, prepare_engine, reset_engine};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};
use tempfile::tempdir;
use util::preprocess_imports;

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

    let runtime = RuntimeEnvironment {
        runtime: HashMap::from([
            ("tooldir".to_string(), tool_dir.to_string_lossy().into_owned()),
            ("outdir".to_string(), runtime_dir.to_string_lossy().into_owned()),
            ("tmpdir".to_string(), runtime_dir.to_string_lossy().into_owned()),
            ("cores".to_string(), 0.to_string()),
            ("ram".to_string(), 0.to_string()),
        ]),
        inputs: collect_inputs(tool, &inputs)?,
        environment: collect_env_vars(tool)
    };

    prepare_engine(&runtime)?;
    
    eval("console.log(inputs);")?;

    env::set_current_dir(current_dir)?;
    reset_engine()?;
    Ok(())
}
