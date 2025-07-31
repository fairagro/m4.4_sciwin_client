use crate::config;
use clap::{Args, Subcommand};
use commonwl::{CWLDocument, CWLType, DefaultValue, Directory, File};
use cwl_execution::{ContainerEngine, execute_cwlfile, set_container_engine};
use keyring::Entry;
use remote_execution::{
    api::{
        create_workflow, download_files, get_workflow_logs, get_workflow_specification, get_workflow_status, get_workflow_workspace, ping_reana,
        start_workflow, upload_files,
    },
    parser::generate_workflow_json_from_cwl,
    rocrate::create_ro_crate,
    arc_rocrate::workflow_json_to_arc_rocrate,
};
use serde_yaml::{Number, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::{collections::HashMap, error::Error, fs, path::PathBuf, thread, time::Duration};

pub fn handle_execute_commands(subcommand: &ExecuteCommands) -> Result<(), Box<dyn Error>> {
    match subcommand {
        ExecuteCommands::Local(args) => execute_local(args),
        ExecuteCommands::Remote(remote_args) => match &remote_args.command {
            RemoteSubcommands::Start {
                file,
                input_file,
                rocrate,
                watch,
                logout,
            } => execute_remote_start(file, input_file, *rocrate, *watch, *logout),
            RemoteSubcommands::Status { workflow_name } => check_remote_status(workflow_name),
            RemoteSubcommands::Download { workflow_name, output_dir } => download_remote_results(workflow_name, output_dir),
            RemoteSubcommands::Rocrate { workflow_name, output_dir, arc } => export_rocrate(workflow_name, output_dir, *arc),
            RemoteSubcommands::Logout => logout_reana(),
        },
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

#[derive(Debug, Args)]
pub struct RemoteExecuteArgs {
    #[command(subcommand)]
    pub command: RemoteSubcommands,
}

#[derive(Debug, Subcommand)]
pub enum RemoteSubcommands {
    #[command(about = "Schedules Execution on REANA")]
    Start {
        #[arg(help = "CWL File to execute")]
        file: PathBuf,
        #[arg(short = 'i', long = "input", help = "Input YAML file")]
        input_file: Option<String>,
        #[arg(long = "rocrate", help = "Create Provenance Run Crate")]
        rocrate: bool,
        #[arg(long = "logout", help = "Delete reana information from credential storage (a.k.a logout)")]
        logout: bool,
        #[arg(long = "watch", help = "Wait for workflow execution to finish and download result")]
        watch: bool,
    },
    #[command(about = "Get the status of Execution on REANA")]
    Status {
        #[arg(help = "Workflow name to check (if omitted, checks all)")]
        workflow_name: Option<String>,
    },
    #[command(about = "Downloads finished Workflow from REANA")]
    Download {
        #[arg(help = "Workflow name to download results for")]
        workflow_name: String,
        #[arg(short = 'd', long = "output_dir", help = "Optional output directory to save downloaded files")]
        output_dir: Option<String>,
    },
    #[command(about = "Downloads finished Workflow Run RO-Crate from REANA")]
    Rocrate {
        #[arg(help = "Workflow name to create a Provenance Run Crate for")]
        workflow_name: String,
        #[arg(
            short = 'd',
            long = "rocrate_dir",
            default_value = "rocrate",
            help = "Optional directory to save RO-Crate to, default rocrate"
        )]
        output_dir: Option<String>,
        #[arg(short = 'a', long = "arc", help = "Export RO-Crate in ARC format")]
        arc: bool,
    },
    #[command(about = "Delete reana information from credential storage (a.k.a logout)")]
    Logout,
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

/// Check status for either single workflow or all of "unwatched" remote workflows
pub fn check_remote_status(workflow_name: &Option<String>) -> Result<(), Box<dyn Error>> {
    let reana_instance = get_or_prompt_credential("reana", "instance", "Enter REANA instance URL: ")?;
    let reana_token = get_or_prompt_credential("reana", "token", "Enter REANA access token: ")?;
    if let Some(name) = workflow_name {
        let status_response =
            get_workflow_status(&reana_instance, &reana_token, name).map_err(|e| format!("Failed to fetch workflow status: {e}"))?;
        let status = status_response["status"].as_str().unwrap_or("unknown");
        let created = status_response["created"].as_str().unwrap_or("unknown");
        println!("{name} {status} created at {created}");
        //if single workflow failed, get step name and logs
        if status == "failed" {
            if let Some(logs_str) = status_response["logs"].as_str() {
                analyze_workflow_logs(logs_str);
            }
        }
    } else {
        let file_path = status_file_path();
        if !file_path.exists() {
            return Err(format!("Workflow status file not found at path: {file_path:?}").into());
        }
        let file = fs::File::open(&file_path)?;
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let status_response =
                    get_workflow_status(&reana_instance, &reana_token, trimmed).map_err(|e| format!("Failed to fetch workflow status: {e}"))?;
                let status = status_response["status"].as_str().unwrap_or("unknown");
                let created = status_response["created"].as_str().unwrap_or("unknown");
                println!("{trimmed} {status} created at {created}");
            }
        }
    }
    Ok(())
}

pub fn download_remote_results(workflow_name: &str, output_dir: &Option<String>) -> Result<(), Box<dyn Error>> {
    let reana_instance = get_or_prompt_credential("reana", "instance", "Enter REANA instance URL: ")?;
    let reana_token = get_or_prompt_credential("reana", "token", "Enter REANA access token: ")?;
    let status_response =
        get_workflow_status(&reana_instance, &reana_token, workflow_name).map_err(|e| format!("Failed to fetch workflow status: {e}"))?;
    let workflow_status = status_response["status"].as_str().unwrap_or("unknown");
    // Get workflow status, only download if finished?
    match workflow_status {
        "finished" => {
            let workflow_json = get_workflow_specification(&reana_instance, &reana_token, workflow_name)?;
            let output_files = workflow_json
                .get("specification")
                .and_then(|spec| spec.get("outputs"))
                .and_then(|outputs| outputs.get("files"))
                .and_then(|files| files.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|filename| format!("outputs/{filename}"))
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();
            download_files(&reana_instance, &reana_token, workflow_name, &output_files, output_dir.as_deref())?;
        }
        "failed" => {
            if let Some(logs_str) = status_response["logs"].as_str() {
                analyze_workflow_logs(logs_str);
            }
            return Err("❌ Workflow '{workflow_name}' failed.".into());
        }
        "created" | "pending" | "running" | "stopped" => {
            return Err(format!("⚠️ Workflow '{workflow_name}' is in '{workflow_status}' state. Cannot export RO-Crate.").into());
        }
        unknown => {
            return Err(format!("❌ Unrecognized workflow status: {unknown}").into());
        }
    }
    Ok(())
}

pub fn export_rocrate(workflow_name: &str, ro_crate_dir: &Option<String>, arc: bool) -> Result<(), Box<dyn Error>> {
    let reana_instance = get_or_prompt_credential("reana", "instance", "Enter REANA instance URL: ")?;
    let reana_token = get_or_prompt_credential("reana", "token", "Enter REANA access token: ")?;
    // Get workflow status, only export if finished?
    let status_response =
        get_workflow_status(&reana_instance, &reana_token, workflow_name).map_err(|e| format!("Failed to fetch workflow status: {e}"))?;
    let workflow_status = status_response["status"].as_str().unwrap_or("unknown");
    match workflow_status {
        "finished" => {
            let workflow_json = get_workflow_specification(&reana_instance, &reana_token, workflow_name)?;
            let config_path = PathBuf::from("workflow.toml");
            let config_str = fs::read_to_string(&config_path)?;
            let specification = workflow_json
                .get("specification")
                .ok_or("❌ 'specification' field missing in workflow JSON")?;
            let logs = get_workflow_logs(&reana_instance, &reana_token, workflow_name)?;
            let logs_str = serde_json::to_string_pretty(&logs).expect("Failed to serialize REANA JSON logs");
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
            if arc {
                workflow_json_to_arc_rocrate(specification,ro_crate_dir.as_deref().unwrap_or("run"));
            } else {
                create_ro_crate(
                    specification,
                    &logs_str,
                    &conforms_to,
                    ro_crate_dir.clone(),
                    &workspace_files,
                    workflow_name,
                    &config_str,
                )?;
            }
        }
        "failed" => {
            let logs = get_workflow_status(&reana_instance, &reana_token, workflow_name)?;
            let logs_str = serde_json::to_string_pretty(&logs).expect("Failed to serialize REANA JSON logs");
            analyze_workflow_logs(&logs_str);
            return Err("❌ Workflow failed. Logs analyzed.".into());
        }
        "created" | "pending" | "running" | "stopped" => {
            return Err(format!("⚠️ Workflow '{workflow_name}' is in '{workflow_status}' state. Cannot export RO-Crate.").into());
        }
        unknown => {
            return Err(format!("❌ Unrecognized workflow status: {unknown}").into());
        }
    }

    Ok(())
}

fn status_file_path() -> PathBuf {
    std::env::temp_dir().join("workflow_status_list.txt")
}

fn save_workflow_name(name: &str) -> std::io::Result<()> {
    let file_path = status_file_path();
    let mut file = OpenOptions::new().create(true).append(true).open(&file_path)?;
    writeln!(file, "{name}")?;
    Ok(())
}

fn get_or_prompt_credential(service: &str, key: &str, prompt: &str) -> Result<String, Box<dyn Error>> {
    let entry = Entry::new(service, key)?;
    match entry.get_password() {
        Ok(val) => Ok(val),
        Err(keyring::Error::NoEntry) => {
            print!("{prompt}");
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let value = input.trim().to_string();
            entry.set_password(&value)?;
            Ok(value)
        }
        Err(e) => Err(Box::new(e)),
    }
}

fn logout_reana() -> Result<(), Box<dyn Error>> {
    Entry::new("reana", "instance")?.delete_credential()?;
    Entry::new("reana", "token")?.delete_credential()?;
    println!("✅ Successfully logged out from previous REANA instances.");
    Ok(())
}

fn derive_workflow_name(file: &std::path::Path, config: Option<&config::Config>) -> String {
    let file_stem = file.file_stem().unwrap_or_default().to_string_lossy();
    config
        .as_ref()
        .map_or_else(|| file_stem.to_string(), |c| format!("{} - {}", c.workflow.name, file_stem))
}

pub fn analyze_workflow_logs(logs_str: &str) {
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
            if logs_text.contains("Error")
                || logs_text.contains("Exception")
                || logs_text.contains("Traceback")
                || logs_text.to_lowercase().contains("failed")
            {
                println!("❌ Workflow execution failed. Workflow step {job_name} may have encountered an error:");
                println!("Logs:\n{logs_text}\n");
            }
        }
    }
}

pub fn execute_remote_start(file: &PathBuf, input_file: &Option<String>, rocrate: bool, watch: bool, logout: bool) -> Result<(), Box<dyn Error>> {
    const POLL_INTERVAL_SECS: u64 = 5;
    const TERMINAL_STATUSES: [&str; 3] = ["finished", "failed", "deleted"];
    let config_path = PathBuf::from("workflow.toml");
    let config: Option<config::Config> = if config_path.exists() {
        Some(toml::from_str(&fs::read_to_string(&config_path)?)?)
    } else {
        None
    };
    let workflow_name = derive_workflow_name(file, config.as_ref());
    // Get credentials
    let reana_instance = get_or_prompt_credential("reana", "instance", "Enter REANA instance URL: ")?;
    let reana_token = get_or_prompt_credential("reana", "token", "Enter REANA access token: ")?;
    // Ping
    let ping_status = ping_reana(&reana_instance)?;
    if ping_status.get("status").and_then(|s| s.as_str()) != Some("200") {
        eprintln!("⚠️ Unexpected response from Reana server: {ping_status:?}");
        return Ok(());
    }
    // Generate worfklow.json
    let workflow_json = generate_workflow_json_from_cwl(file, input_file)?;
    let converted_yaml: serde_yaml::Value = serde_json::from_value(workflow_json.clone())?;
    // Create workflow
    let create_response = create_workflow(&reana_instance, &reana_token, &workflow_json, Some(&workflow_name))?;
    let Some(workflow_name) = create_response["workflow_name"].as_str() else {
        return Err("Missing workflow_name in response".into());
    };
    upload_files(&reana_instance, &reana_token, input_file, file, workflow_name, &workflow_json)?;
    start_workflow(&reana_instance, &reana_token, workflow_name, None, None, false, converted_yaml)?;
    println!("✅ Started workflow execution");
    if watch {
        loop {
            let status_response =
                get_workflow_status(&reana_instance, &reana_token, workflow_name).map_err(|e| format!("Failed to fetch workflow status: {e}"))?;
            let workflow_status = status_response["status"].as_str().unwrap_or("unknown");
            if TERMINAL_STATUSES.contains(&workflow_status) {
                match workflow_status {
                    "finished" => {
                        println!("✅ Workflow finished successfully.");
                        if let Err(e) = download_remote_results(workflow_name, &None) {
                            eprintln!("Error downloading remote results: {e}");
                        }
                        if rocrate {
                            if let Err(e) = export_rocrate(workflow_name, &Some("rocrate".to_string()), false) {
                                eprintln!("Error trying to create a Provenance RO-Crate: {e}");
                            }
                        }
                    }
                    "failed" => {
                        if let Some(logs_str) = status_response["logs"].as_str() {
                            analyze_workflow_logs(logs_str);
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
    } else {
        save_workflow_name(workflow_name)?;
    }
    if logout {
        if let Err(e) = logout_reana() {
            eprintln!("Error logging out of reana instance: {e}");
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_type() {
        assert_eq!(defaults(&CWLType::Int), Value::Number(Number::from(42)));
        assert_eq!(defaults(&CWLType::Boolean), Value::Bool(true));
        assert_eq!(defaults(&CWLType::Long), Value::Number(Number::from(42)));
        assert_eq!(defaults(&CWLType::Float), Value::Number(Number::from(69.42)));
        assert_eq!(defaults(&CWLType::String), Value::String("Hello World".into()));
        assert_eq!(defaults(&CWLType::Any), Value::String("Any Value".into()));
    }

    #[test]
    fn test_default_values() {
        assert_eq!(
            default_values(&CWLType::File),
            DefaultValue::File(File::from_location("./path/to/file.txt"))
        );
        assert_eq!(
            default_values(&CWLType::Directory),
            DefaultValue::Directory(Directory::from_location("./path/to/dir"))
        );
        assert_eq!(default_values(&CWLType::String), DefaultValue::Any(Value::String("Hello World".into())));
    }
}
