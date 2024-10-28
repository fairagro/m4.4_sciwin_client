use clap::{Args, Subcommand};
use std::{collections::HashMap, error::Error, fs};

use crate::cwl::{clt::DefaultValue, runner::run_commandlinetool};

pub fn handle_execute_commands(subcommand: &ExecuteCommands) -> Result<(), Box<dyn Error>> {
    match subcommand {
        ExecuteCommands::Local(args) => execute_local(args)?,
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum ExecuteCommands {
    #[command(about = "Runs CWL locally using a custom runner")]
    Local(LocalExecuteArgs),
}

#[derive(Args, Debug)]
pub struct LocalExecuteArgs {
    #[arg(help = "CWL File to execute")]
    pub file: String,
    #[arg(help = "Optional Input file providing inputs for cwl file")]
    pub inputs: Option<String>,

    #[arg(trailing_var_arg = true, help = "other arguments provided to cwl file")]
    pub args: Vec<String>,
}

pub fn execute_local(args: &LocalExecuteArgs) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(&args.file).map_err(|e| format!("Could not load File {}: {}", args.file, e))?;
    let tool = serde_yml::from_str(&contents).map_err(|e| format!("Could not load CommandLineTool: {}", e))?;

    let mut inputs: Option<HashMap<String, DefaultValue>> = None;

    //TODO: handle args.args

    //check for yaml input
    if let Some(input) = &args.inputs {
        let yaml = fs::read_to_string(input).map_err(|e| format!("Could not load File {}: {}", input, e))?;
        inputs = Some(serde_yml::from_str(&yaml).map_err(|e| format!("Could not read input file: {}", e))?);
    }

    run_commandlinetool(&tool, inputs)?;

    Ok(())
}
