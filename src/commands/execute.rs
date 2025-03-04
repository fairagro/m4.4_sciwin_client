use crate::{
    error::CommandError,
    execution::{
        runner::{run_commandlinetool, run_workflow},
        util::preprocess_cwl,
    },
    io::join_path_string,
    parser::guess_type,
};
use clap::{Args, Subcommand, ValueEnum};
use cwl::{
    clt::CommandLineTool,
    types::{CWLType, DefaultValue, Directory, File, PathItem},
    wf::Workflow,
    CWLDocument,
};
use log::info;
use serde_yaml::{Number, Value};
use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn handle_execute_commands(subcommand: &ExecuteCommands) -> Result<(), Box<dyn Error>> {
    match subcommand {
        ExecuteCommands::Local(args) => execute_local(args),
        ExecuteCommands::MakeTemplate(args) => make_template(&args.cwl),
    }
}

#[derive(Debug, Subcommand)]
pub enum ExecuteCommands {
    #[command(about = "Runs CWL files locally using a custom runner or cwltool", visible_alias = "l")]
    Local(LocalExecuteArgs),
    #[command(about = "Creates job file template for execution (e.g. inputs.yaml)")]
    MakeTemplate(MakeTemplateArgs),
}

#[derive(Args, Debug)]
pub struct MakeTemplateArgs {
    #[arg(help = "CWL File to create input template for")]
    pub cwl: PathBuf,
}

#[derive(Args, Debug, Default)]
pub struct LocalExecuteArgs {
    #[arg(value_enum, default_value_t = Runner::Custom, short = 'r', long = "runner", help="Choose your cwl runner implementation")]
    pub runner: Runner,
    #[arg(long = "outdir", help = "A path to output resulting files to")]
    pub out_dir: Option<String>,
    #[arg(long = "quiet", help = "Runner does not print to stdout")]
    pub is_quiet: bool,
    #[arg(help = "CWL File to execute")]
    pub file: PathBuf,
    #[arg(trailing_var_arg = true, help = "Other arguments provided to cwl file", allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(ValueEnum, Debug, Clone, Default)]
pub enum Runner {
    #[clap(name = "cwltool")]
    CWLTool,
    #[default]
    Custom,
}

pub fn execute_local(args: &LocalExecuteArgs) -> Result<(), Box<dyn Error>> {
    match args.runner {
        Runner::CWLTool => {
            if !args.is_quiet {
                eprintln!("💻 Executing {:?} using cwltool.", &args.file);
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
                info!(
                    "💻 Executing {:?} using SciWIn's custom runner. Use `--runner cwltool` to use reference runner (if installed). 
⚠️  The internal runner currently is for testing purposes only and does not support containerization, yet!",
                    &args.file
                );
            }

            //gather inputs
            let contents = fs::read_to_string(&args.file).map_err(|e| format!("Could not load File {:?}: {}", args.file, e))?;
            let mut inputs: Option<HashMap<String, DefaultValue>> = None;
            let is_file_input = args.args.len() == 1 && !&args.args[0].starts_with("-");

            //check for yaml input
            match args.args.len() {
                // is input.yml file
                1 => {
                    let input = &args.args[0];
                    if is_file_input {
                        let yaml = fs::read_to_string(input).map_err(|e| format!("Could not load File {}: {}", input, e))?;
                        inputs = Some(serde_yaml::from_str(&yaml).map_err(|e| format!("Could not read input file: {}", e))?);
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
                                _ => serde_yaml::from_str(&args.args[i + 1])?,
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
                let location = item.get_location().clone();
                item.set_location(join_path_string(path_prefix, &location));
                if let Some(secondary_files) = item.secondary_files_mut() {
                    for sec_file in secondary_files {
                        match sec_file {
                            DefaultValue::File(file) => {
                                file.set_location(join_path_string(path_prefix, &file.get_location()));
                            }
                            DefaultValue::Directory(directory) => directory.set_location(join_path_string(path_prefix, &directory.get_location())),
                            DefaultValue::Any(_) => (),
                        }
                    }
                }
            }

            //make paths relative to calling object
            if let Some(inputs) = &mut inputs {
                let path_prefix = if is_file_input {
                    Path::new(&args.args[0]).parent().unwrap()
                } else {
                    Path::new(".")
                };
                for value in inputs.values_mut() {
                    match value {
                        DefaultValue::File(file) => correct_path(file, path_prefix),
                        DefaultValue::Directory(directory) => correct_path(directory, path_prefix),
                        DefaultValue::Any(_) => (),
                    }
                }
            }

            //preprocess cwl import statements
            let preprocessed_contents = preprocess_cwl(&contents, &args.file);

            let cwl_yaml: Value = serde_yaml::from_str(&preprocessed_contents).map_err(|e| format!("Could not load YAML: {}", e))?;
            let class = cwl_yaml.get("class").expect("Could not get class").as_str().unwrap();
            let is_workflow = class == "Workflow";
            let is_tool = class == "CommandLineTool";
            if is_tool {
                let mut tool: CommandLineTool = serde_yaml::from_value(cwl_yaml).map_err(|e| format!("Could not load CommandLineTool: {}", e))?;
                run_commandlinetool(&mut tool, inputs, Some(&args.file), args.out_dir.clone())?;
            } else if is_workflow {
                let mut workflow: Workflow = serde_yaml::from_value(cwl_yaml).map_err(|e| format!("Could not load Workflow: {}", e))?;
                run_workflow(&mut workflow, inputs, Some(&args.file), args.out_dir.clone())?;
            } else {
                Err(CommandError {
                    exit_code: 33,
                    message: format!("CWL Document of class {class:?} is not supported"),
                })?
            }

            Ok(())
        }
    }
}

pub fn make_template(filename: &PathBuf) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(filename)?;
    let cwl: CWLDocument = serde_yaml::from_str(&contents)?;

    let inputs = match cwl {
        CWLDocument::CommandLineTool(tool) => tool.inputs,
        CWLDocument::Workflow(workflow) => workflow.inputs,
        CWLDocument::ExpressionTool(expression_tool) => expression_tool.inputs,
    };

    let template = inputs
        .iter()
        .map(|i| {
            let id = &i.id;
            let dummy_value = match &i.type_ {
                CWLType::Optional(cwltype) => default_values(cwltype),
                CWLType::Array(cwltype) => DefaultValue::Any(Value::Sequence(vec![defaults(cwltype), defaults(cwltype)])),
                cwltype => default_values(cwltype),
            };
            (id, dummy_value)
        })
        .collect::<HashMap<_, _>>();
    let yaml = serde_yaml::to_string(&template)?;
    println!("{yaml}");
    Ok(())
}

fn default_values(cwltype: &CWLType) -> DefaultValue {
    match cwltype {
        CWLType::File => DefaultValue::File(File::from_location(&"./path/to/file.txt".into())),
        CWLType::Directory => DefaultValue::Directory(Directory::from_location(&"./path/to/dir".into())),
        _ => DefaultValue::Any(defaults(cwltype)),
    }
}

fn defaults(cwltype: &CWLType) -> Value {
    match cwltype {
        CWLType::Boolean => Value::Bool(true),
        CWLType::Int => Value::Number(Number::from(42)),
        CWLType::Long => Value::Number(Number::from(42)),
        CWLType::Float => Value::Number(Number::from(69.42)),
        CWLType::Double => Value::Number(Number::from(69.42)),
        CWLType::String => Value::String("Hello World".into()),
        CWLType::Any => Value::String("Any Value".into()),
        _ => Value::Null,
    }
}
