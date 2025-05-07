use clap::{Args, Subcommand, ValueEnum};
use cwl::{
    types::{CWLType, DefaultValue, Directory, File},
    CWLDocument,
};
use cwl_execution::{execute_cwlfile, set_container_engine, ContainerEngine};
use log::info;
use serde_yaml::{Number, Value};
use std::{collections::HashMap, error::Error, fs, path::PathBuf, process::Command};
use remote_execution::api::{ping_reana, upload_files, start_workflow, download_files, create_workflow, get_workflow_status};
use remote_execution::parser::generate_workflow_json_from_cwl;
use std::{thread, time::Duration};

pub fn handle_execute_commands(subcommand: &ExecuteCommands) -> Result<(), Box<dyn Error>> {
    match subcommand {
        ExecuteCommands::Local(args) => execute_local(args),
        ExecuteCommands::Remote(args) => execute_remote(args),
        ExecuteCommands::MakeTemplate(args) => make_template(&args.cwl),
    }
}

#[derive(Debug, Subcommand)]
pub enum ExecuteCommands {
    #[command(about = "Runs CWL files locally using a custom runner or cwltool", visible_alias = "l")]
    Local(LocalExecuteArgs),
    #[command(about = "Runs CWL files remotely using reana", visible_alias = "r")]
    Remote(RemoteExecuteArgs),
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
    #[arg(long = "podman", help = "Use podman instead of docker")]
    pub podman: bool,
    #[arg(help = "CWL File to execute")]
    pub file: PathBuf,
    #[arg(trailing_var_arg = true, help = "Other arguments provided to cwl file", allow_hyphen_values = true)]
    pub args: Vec<String>,
}


#[derive(Args, Debug, Default)]
pub struct RemoteExecuteArgs {
    #[arg(short = 'r', long = "instance", help="Reana instance")]
    pub instance: String,
    #[arg(short = 't', long = "token", help="Your reana token")]
    pub token: String,
    #[arg(short = 'c', long = "cookie", help="Your session cookie")]
    pub cookie_value: String,
    #[arg(help = "CWL File to execute")]
    pub file: PathBuf,
    #[arg(short = 'i', long = "input", help="Input yaml file")]
    pub input_file: Option<String>,
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
            if args.podman {
                cmd.arg("--podman");
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
                    "💻 Executing {:?} using SciWIn's custom runner. Use `--runner cwltool` to use reference runner (if installed).",
                    &args.file
                );
            }

            if args.podman {
                set_container_engine(ContainerEngine::Podman);
            } else {
                set_container_engine(ContainerEngine::Docker);
            }

            execute_cwlfile(&args.file, &args.args, args.out_dir.clone())
        }
    }
}


pub fn execute_remote(args: &RemoteExecuteArgs) -> Result<(), Box<dyn Error>> {
    const POLL_INTERVAL_SECS: u64 = 5;
    const TERMINAL_STATUSES: [&str; 3] = ["finished", "failed", "deleted"];
    let reana_instance = &args.instance;
    let reana_token = &args.token;
    let cookie_value = &args.cookie_value;
    let file = &args.file;
    let input_file = &args.input_file;

    let workflow_json = generate_workflow_json_from_cwl(file, input_file)?;

    let ping_status = ping_reana(reana_instance)?;
    if ping_status.get("status").and_then(|s| s.as_str()) != Some("200") {
        eprintln!("Unexpected response from Reana server: {ping_status:?}");
        return Ok(());
    }

    let create_response = create_workflow(reana_instance, reana_token, cookie_value, &workflow_json)?;
    if let Some(workflow_name) = create_response["workflow_name"].as_str() {

        upload_files(reana_instance, reana_token, cookie_value, input_file, file, workflow_name, &workflow_json)?;

        let converted_yaml: serde_yaml::Value = serde_json::from_value(workflow_json.clone())?;
        start_workflow(reana_instance, reana_token, cookie_value, workflow_name, None, false, converted_yaml)?;

        loop {
            let status_response = get_workflow_status(reana_instance, reana_token, cookie_value, workflow_name)?;
            let workflow_status = status_response["status"]
                .as_str()
                .unwrap_or("unknown");


            if TERMINAL_STATUSES.contains(&workflow_status) {
                match workflow_status {
                    "finished" => {
                        println!("✅ Workflow finished successfully.");
                        download_files(reana_instance, reana_token, cookie_value, workflow_name, &workflow_json)?;
                    }
                    "failed" => {
                        eprintln!("❌ Workflow execution failed.");
                    }
                    "deleted" => {
                        eprintln!("⚠️ Workflow was deleted before completion.");
                    }
                    _ => {}
                }
                break;
            }

            thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
        }
    } else {
        eprintln!("Workflow creation failed {create_response:?}");
    }

    Ok(())
}


pub fn make_template(filename: &PathBuf) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(filename)?;
    let cwl: CWLDocument = serde_yaml::from_str(&contents)?;

    let template = &cwl
        .inputs
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
        CWLType::Int | CWLType::Long => Value::Number(Number::from(42)),
        CWLType::Float | CWLType::Double => Value::Number(Number::from(69.42)),
        CWLType::String => Value::String("Hello World".into()),
        CWLType::Any => Value::String("Any Value".into()),
        _ => Value::Null,
    }
}
