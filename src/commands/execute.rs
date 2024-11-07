use crate::{
    cwl::{
        clt::{CommandLineTool, DefaultValue}, execution::runner::run_commandlinetool, parser::guess_type, types::{CWLType, Directory, File, PathItem}
    },
    io::join_path_string,
};
use clap::{Args, Subcommand, ValueEnum};
use std::{collections::HashMap, error::Error, fs, path::Path, process::Command};

pub fn handle_execute_commands(subcommand: &ExecuteCommands) -> Result<(), Box<dyn Error>> {
    match subcommand {
        ExecuteCommands::Local(args) | ExecuteCommands::L(args) => execute_local(args)?,
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum ExecuteCommands {
    #[command(about = "Runs CWL files locally using a custom runner or cwltool (\x1b[1msynonym\x1b[0m: s4n ex l)")]
    Local(LocalExecuteArgs),
    #[command(hide = true)]
    L(LocalExecuteArgs),
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

            let is_file_input = args.args.len() == 1 && !&args.args[0].starts_with("-");

            //check for yaml input
            match args.args.len() {
                // is input.yml file
                1 => {
                    let input = &args.args[0];
                    if is_file_input {
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
                            let raw_value = &args.args[i + 1];
                            let value = match guess_type(raw_value) {
                                CWLType::File => DefaultValue::File(File::from_location(raw_value)),
                                CWLType::Directory => DefaultValue::Directory(Directory::from_location(raw_value)),
                                _ => serde_yml::from_str(&args.args[i + 1])?
                            };                            
                            map.insert(key, value);
                            i += 1;
                        }
                        i += 1;
                    }
                }
                //ignore and use without args
                _ => {}
            }

            fn correct_path<T: PathItem>(item: &mut T, path_prefix: &Path) {
                let location = item.location().clone();
                item.set_location(join_path_string(path_prefix, &location));
                if let Some(secondary_files) = item.secondary_files_mut() {
                    for sec_file in secondary_files {
                        match sec_file {
                            DefaultValue::File(file) => {
                                file.set_location(join_path_string(path_prefix, &file.location));
                            }
                            DefaultValue::Directory(directory) => directory.set_location(join_path_string(path_prefix, &directory.location)),
                            DefaultValue::Any(_) => (),
                        }
                    }
                }
            }

            //make paths relative to calling object
            if let Some(inputs) = &mut inputs {
                let path_prefix = if is_file_input { Path::new(&args.args[0]).parent().unwrap() } else { Path::new(".") };
                for value in inputs.values_mut() {
                    match value {
                        DefaultValue::File(file) => correct_path(file, path_prefix),
                        DefaultValue::Directory(directory) => correct_path(directory, path_prefix),
                        DefaultValue::Any(_) => (),
                    }
                }
            }

            run_commandlinetool(&mut tool, inputs, Some(args.file.as_str()), args.out_dir.clone())?;

            Ok(())
        }
    }
}
