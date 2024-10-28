use clap::{Args, Subcommand};
use std::{collections::HashMap, error::Error, fs};

use crate::cwl::{
    clt::{CommandLineTool, DefaultValue},
    runner::run_commandlinetool,
};

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
    #[arg(trailing_var_arg = true, help = "other arguments provided to cwl file", allow_hyphen_values = true)]
    pub args: Vec<String>,
}

pub fn execute_local(args: &LocalExecuteArgs) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(&args.file).map_err(|e| format!("Could not load File {}: {}", args.file, e))?;
    let tool: CommandLineTool = serde_yml::from_str(&contents).map_err(|e| format!("Could not load CommandLineTool: {}", e))?;

    let mut inputs: Option<HashMap<String, DefaultValue>> = None;

    //check for yaml input
    if args.args.len() == 1 {
        let input = &args.args[0];
        if !input.starts_with("-") {
            let yaml = fs::read_to_string(input).map_err(|e| format!("Could not load File {}: {}", input, e))?;
            inputs = Some(serde_yml::from_str(&yaml).map_err(|e| format!("Could not read input file: {}", e))?);
        }
    } else if args.args.len() > 1 {
        inputs = Some(HashMap::new());
        let map = inputs.as_mut().unwrap();
        let mut i = 0;
        while i < args.args.len() {
            if args.args[i].starts_with("-") {
                let key = args.args[i].trim_start_matches("--").to_string();
                let value: DefaultValue = serde_yml::from_str(&args.args[i + 1])?;
                map.insert(key, value);
                i += 1;
            }
            i += 1;
        }
    }

    run_commandlinetool(&tool, inputs, Some(args.file.as_str()))?;

    Ok(())
}
