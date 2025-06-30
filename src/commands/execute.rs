use crate::config;
use clap::{Args, Subcommand};
use cwl::{
    types::{CWLType, DefaultValue, Directory, File},
    CWLDocument,
};
use cwl_execution::{execute_cwlfile, set_container_engine, ContainerEngine};
use keyring::Entry;
use remote_execution::{
    api::{
        create_workflow, download_files, get_workflow_logs, get_workflow_status, get_workflow_workspace, ping_reana, start_workflow, upload_files,
    },
    parser::generate_workflow_json_from_cwl,
    rocrate::create_ro_crate,
};
use serde_yaml::{Number, Value};
use std::io::Write;
use std::{collections::HashMap, error::Error, fs, path::PathBuf, thread, time::Duration};

pub fn handle_execute_commands(subcommand: &ExecuteCommands) -> Result<(), Box<dyn Error>> {
    match subcommand {
        ExecuteCommands::Local(args) => execute_local(args),
        ExecuteCommands::Remote(args) => execute_remote(args),
        ExecuteCommands::MakeTemplate(args) => make_template(&args.cwl),
    }
}

#[derive(Debug, Subcommand)]
pub enum ExecuteCommands {
    #[command(about = "Runs CWL files locally", visible_alias = "l")]
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
    #[arg(help = "CWL File to execute")]
    pub file: PathBuf,
    #[arg(short = 'i', long = "input", help = "Input yaml file")]
    pub input_file: Option<String>,
    #[arg(long = "rocrate", help = "Create Provenance Run Crate")]
    pub rocrate: bool,
    #[arg(long = "logout", help = "Delete reana information from credential storage (a.k.a logout)")]
    pub logout: bool,
}

pub fn execute_local(args: &LocalExecuteArgs) -> Result<(), Box<dyn Error>> {
    if args.is_quiet {
        log::set_max_level(log::LevelFilter::Error);
    }
    if args.podman {
        set_container_engine(ContainerEngine::Podman);
    } else {
        set_container_engine(ContainerEngine::Docker);
    }

    execute_cwlfile(&args.file, &args.args, args.out_dir.clone())
}

pub fn execute_remote(args: &RemoteExecuteArgs) -> Result<(), Box<dyn Error>> {
    const POLL_INTERVAL_SECS: u64 = 5;
    const TERMINAL_STATUSES: [&str; 3] = ["finished", "failed", "deleted"];
    let ro_crate_dir = "rocrate";
    let file = &args.file;
    let input_file = &args.input_file;

    //try getting the workflow name from the toml
    let config_path = PathBuf::from("workflow.toml");

    let config: Option<config::Config> = if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        Some(toml::from_str(&content).map_err(|e| format!("❌ Failed to parse workflow.toml: {e}"))?)
    } else {
        None
    };
    // Get workflow name
    let file_str = file.file_stem().unwrap_or_default().to_string_lossy();
    let workflow_name = Some(
        config
            .as_ref()
            .map(|c| c.workflow.name.as_str())
            .map(|name| format!("{name} - {file_str}"))
            .unwrap_or_else(|| file_str.to_string()),
    );
    if args.logout {
        // Delete reana credentials
        let entry = Entry::new("reana", "instance")?;
        entry.delete_credential()?;
        let entry = Entry::new("reana", "token")?;
        entry.delete_credential()?;
        println!("✅ Successfully logged out from previous REANA instances.");
    }

    // Get or prompt REANA instance
    let entry = Entry::new("reana", "instance")?;
    let reana_instance = match entry.get_password() {
        Ok(url) => url,
        Err(keyring::Error::NoEntry) => {
            print!("Enter REANA instance URL: ");
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let value = input.trim().to_string();
            entry.set_password(&value)?;
            value
        }
        _ => entry.get_password().map_err(|e| format!("❌ Failed to get REANA instance URL: {e}"))?,
    };
    // Get or prompt REANA token
    let entry = Entry::new("reana", "token")?;
    let reana_token = match entry.get_password() {
        Ok(token) => token,
        Err(keyring::Error::NoEntry) => {
            print!("Enter REANA access token: ");
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let value = input.trim().to_string();
            entry.set_password(&value)?;
            value
        }
        _ => entry.get_password().map_err(|e| format!("❌ Failed to get REANA token: {e}"))?,
    };

    let config_str = fs::read_to_string(&config_path).map_err(|e| format!("❌ Failed to read workflow.toml: {e}"))?;

    let workflow_json = generate_workflow_json_from_cwl(file, input_file).map_err(|e| format!("Failed to generate workflow JSON from CWL: {e}"))?;

    let converted_yaml: serde_yaml::Value =
        serde_json::from_value(workflow_json.clone()).map_err(|e| format!("Failed to convert workflow JSON to YAML: {e}"))?;

    println!("✅ Created workflow JSON");

    let ping_status = ping_reana(&reana_instance).map_err(|e| format!("Failed to ping Reana server: {e}"))?;

    if ping_status.get("status").and_then(|s| s.as_str()) != Some("200") {
        eprintln!("⚠️ Unexpected response from Reana server: {ping_status:?}");
        return Ok(());
    }

    let create_response = create_workflow(&reana_instance, &reana_token, &workflow_json, workflow_name.as_deref())
        .map_err(|e| format!("Failed to create workflow on Reana: {e}"))?;

    let Some(workflow_name) = create_response["workflow_name"].as_str() else {
        return Err("Missing 'workflow_name' in workflow creation response".into());
    };

    upload_files(&reana_instance, &reana_token, input_file, file, workflow_name, &workflow_json)
        .map_err(|e| format!("Failed to upload files to Reana: {e}"))?;

    start_workflow(&reana_instance, &reana_token, workflow_name, None, None, false, converted_yaml)
        .map_err(|e| format!("Failed to start workflow: {e}"))?;

    loop {
        let status_response =
            get_workflow_status(&reana_instance, &reana_token, workflow_name).map_err(|e| format!("Failed to fetch workflow status: {e}"))?;

        let workflow_status = status_response["status"].as_str().unwrap_or("unknown");

        if TERMINAL_STATUSES.contains(&workflow_status) {
            match workflow_status {
                "finished" => {
                    println!("✅ Workflow finished successfully.");
                    let output_files = workflow_json
                        .get("outputs")
                        .and_then(|outputs| outputs.get("files"))
                        .and_then(|files| files.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|filename| format!("outputs/{filename}"))
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default();
                    download_files(&reana_instance, &reana_token, workflow_name, &output_files, Some(ro_crate_dir))?;
                    if args.rocrate {
                        let logs = get_workflow_logs(&reana_instance, &reana_token, workflow_name)?;
                        let logs_str = serde_json::to_string_pretty(&logs).expect("Failed to serialize reana JSON logs");
                        let conforms_to = [
                            "https://w3id.org/ro/wfrun/process/0.5",
                            "https://w3id.org/ro/wfrun/workflow/0.5",
                            "https://w3id.org/ro/wfrun/provenance/0.5",
                            "https://w3id.org/workflowhub/workflow-ro-crate/1.0",
                        ];
                        let workspace_response = get_workflow_workspace(&reana_instance, &reana_token, workflow_name)?;
                        let workspace_files: Vec<String> = workspace_response
                            .get("items")
                            .and_then(|items| items.as_array())
                            .map(|array| array.iter().filter_map(|item| item.get("name")?.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        create_ro_crate(
                            &workflow_json,
                            &logs_str,
                            &conforms_to,
                            Some(ro_crate_dir.to_string()),
                            &workspace_files,
                            workflow_name,
                            &config_str,
                        )?;
                    }
                }
                "failed" => {
                    if let Some(logs_str) = status_response["logs"].as_str() {
                        let logs: serde_json::Value = serde_json::from_str(logs_str).expect("Invalid logs JSON");
                        let mut found_failure = false;
                        for (_job_id, job_info) in logs.as_object().unwrap() {
                            let status = job_info["status"].as_str().unwrap_or("unknown");
                            let job_name = job_info["job_name"].as_str().unwrap_or("unknown");
                            let logs_text = job_info["logs"].as_str().unwrap_or("");
                            if status == "failed" {
                                println!("❌ Workflow execution failed at step {job_name}:");
                                println!("Logs:\n{logs_text}\n");
                                found_failure = true;
                            }
                        }
                        // sometimes a workflow step fails but it is marked as finished, search for errors and suggest as failed step
                        if !found_failure {
                            for (_job_id, job_info) in logs.as_object().unwrap() {
                                let job_name = job_info["job_name"].as_str().unwrap_or("unknown");
                                let logs_text = job_info["logs"].as_str().unwrap_or("");
                                //search for error etc in logs of steps
                                if logs_text.contains("Error") || logs_text.contains("Exception")
                                    || logs_text.contains("Traceback") || logs_text.to_lowercase().contains("failed")
                                {
                                    println!("❌ Workflow execution failed. Workflow step {job_name} may have encountered an error:");
                                    println!("Logs:\n{logs_text}\n");
                                }
                            }
                        }
                    }
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
        CWLType::File => DefaultValue::File(File::from_location("./path/to/file.txt")),
        CWLType::Directory => DefaultValue::Directory(Directory::from_location("./path/to/dir")),
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
