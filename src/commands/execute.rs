use clap::{Args, Subcommand, ValueEnum};
use std::{collections::HashMap, error::Error, fs, process::Command};

use crate::cwl::{
    clt::{CommandLineTool, DefaultValue},
    execution::runner::run_commandlinetool,
};

pub fn handle_execute_commands(subcommand: &ExecuteCommands) -> Result<(), Box<dyn Error>> {
    match subcommand {
        ExecuteCommands::Local(args) => execute_local(args)?,
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum ExecuteCommands {
    #[command(about = "Runs CWL files locally using a custom runner or cwltool")]
    Local(LocalExecuteArgs),
}

#[derive(Args, Debug)]
pub struct LocalExecuteArgs {
    #[arg(value_enum, default_value_t = Runner::Custom, short = 'r', long = "runner", help="Choose your cwl runner implementation")]
    pub runner: Runner,
    #[arg(long = "outdir", help = "A path to output resulting files to")]
    pub out_dir: Option<String>,
    #[arg(long = "quiet", help = "Runner does not print to stdout")]
    pub is_quiet: bool,
    #[arg(help = "CWL File to execute")]
    pub file: String,
    #[arg(trailing_var_arg = true, help = "Other arguments provided to cwl file", allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum Runner {
    #[clap(name = "cwltool")]
    CWLTool,
    Custom,
}

pub fn execute_local(args: &LocalExecuteArgs) -> Result<(), Box<dyn Error>> {
    match args.runner {
        Runner::CWLTool => {
            if !args.is_quiet {
                eprintln!("💻 Executing {} using cwltool.", &args.file);
            }
            let mut cmd = Command::new("cwltool");

            //handle args
            if args.is_quiet {
                cmd.arg("--quiet");
            }
            if let Some(outdir) = &args.out_dir {
                cmd.arg("--outdir").arg(outdir);
            }

            cmd.arg(&args.file).args(&args.args);
            let output = &cmd.output()?;
            if !output.stdout.is_empty() {
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
            Ok(())
        }
        Runner::Custom => {
            if !args.is_quiet {
                eprintln!(
                    "💻 Executing {} using SciWIn's custom runner. Use `--runner cwltool` to use reference runner (if installed). SciWIn's runner currently only supports 'CommandLineTools'!",
                    &args.file
                );
            }

            let contents = fs::read_to_string(&args.file).map_err(|e| format!("Could not load File {}: {}", args.file, e))?;
            let mut tool: CommandLineTool = serde_yml::from_str(&contents).map_err(|e| format!("Could not load CommandLineTool: {}", e))?;

            let mut inputs: Option<HashMap<String, DefaultValue>> = None;

            //check for yaml input
            match args.args.len() {
                // is input.yml file
                1 => {
                    let input = &args.args[0];
                    if !input.starts_with("-") {
                        let yaml = fs::read_to_string(input).map_err(|e| format!("Could not load File {}: {}", input, e))?;
                        inputs = Some(serde_yml::from_str(&yaml).map_err(|e| format!("Could not read input file: {}", e))?);
                    }
                }
                //arguments given as commandline inputs
                n if n > 1 => {
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
                //ignore and use without args
                _ => {}
            }

            run_commandlinetool(&mut tool, inputs, Some(args.file.as_str()), args.out_dir.clone())?;

            Ok(())
        }
    }
}
