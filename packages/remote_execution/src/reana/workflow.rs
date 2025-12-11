use crate::reana::{auth::login_reana, compatibility::compatibility_adjustments, status::status_file_path};
use reana::{
    api::{create_workflow, ping_reana},
    parser::generate_workflow_json_from_input,
    reana::Reana,
};
use s4n_core::config;
use log::info;
use std::{collections::HashMap, error::Error, fs, path::{PathBuf, Path}};

pub fn execute_remote_start(file: &Path, input_file: &Option<PathBuf>) -> Result<String, Box<dyn Error>> {
    let current_dir = std::env::current_dir()?;
    let canonical_file = if file.exists() {
        file.canonicalize().unwrap_or_else(|_| file.to_path_buf())
    } else {
        current_dir.join(file)
    };
    let config: Option<config::Config> = if Path::new("workflow.toml").exists() {
        let toml_str = fs::read_to_string("workflow.toml")?;
        Some(toml::from_str(&toml_str)?)
    } else {
        None
    };
    let workflow_name = derive_workflow_name(&canonical_file, config.as_ref());
    let (mut workflow_json, mut cwl_file, temp_dir, input_yaml_file) = generate_workflow_json_from_input(&canonical_file, input_file.as_deref())?;
    let exec_root = if file.is_dir() {
        file.to_path_buf()
    } else if let Some(ref tmp) = temp_dir {
        tmp.path().to_path_buf()
    } else {
        canonical_file.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
    };
    let base_path = determine_common_base_path(file, input_file.as_ref(), &cwl_file, input_yaml_file.as_ref(), &exec_root);
    let base_dir_option = if file.is_dir() || temp_dir.is_some() {
        Some(base_path.as_path())
    } else {
        None
    };
    compatibility_adjustments(&mut workflow_json, base_dir_option)?;
    let (reana_instance, reana_token) = login_reana()?;
    let reana = Reana::new(&reana_instance, &reana_token);
    let ping_status = ping_reana(&reana)?;
    if ping_status.get("status").and_then(|s| s.as_str()) != Some("200") {
        return Err(Box::from(format!("⚠️ Unexpected response from REANA server: {ping_status:?}")));
    }
    let workflow_json_value = serde_json::to_value(&workflow_json)?;
    let converted_yaml: serde_yaml::Value = serde_json::from_value(workflow_json_value.clone())?;
    if !cwl_file.exists() {
        let combined = current_dir.join(&cwl_file);
        if combined.exists() {
            cwl_file = combined;
        } else {
            eprintln!("⚠️ CWL file not found: {cwl_file:?}");
        }
    }
    let resolved_input_yaml = input_yaml_file.or_else(|| input_file.clone()).map(|p| {
        let combined = current_dir.join(&p);
        if combined.exists() {
            combined
        } else {
            eprintln!("⚠️ Input YAML not found: {p:?}");
            p
        }
    });
    let create_response = create_workflow(&reana, &workflow_json_value, Some(&workflow_name))?;
    let workflow_name = create_response["workflow_name"]
        .as_str()
        .ok_or("Missing workflow_name in REANA response")?;
    reana::api::upload_files(&reana, &resolved_input_yaml, &cwl_file, workflow_name, &workflow_json_value, &base_path)?;
    reana::api::start_workflow(&reana, workflow_name, None, None, false, &converted_yaml)?;
    info!("✅ Workflow execution started successfully on REANA");
    save_workflow_name(&reana_instance, workflow_name)?;
    Ok(workflow_name.to_owned())
}

fn determine_common_base_path(
    file: &Path,
    input_file: Option<&PathBuf>,
    cwl_file: &Path,
    input_yaml_file: Option<&PathBuf>,
    fallback: &Path,
) -> PathBuf {
    if let Some(input) = input_file {
        if file.exists() && input.exists() {
            if let Some(common) = longest_common_dir(file, input) {
                return common;
            }
        }
    }
    if let Some(input_yaml) = input_yaml_file {
        if input_yaml.exists() && cwl_file.exists() {
            if let Some(common) = longest_common_dir(cwl_file, input_yaml) {
                return common;
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| fallback.to_path_buf())
}

fn longest_common_dir(a: &Path, b: &Path) -> Option<PathBuf> {
    let mut common = PathBuf::new();
    for (comp_a, comp_b) in a.components().zip(b.components()) {
        if comp_a == comp_b {
            common.push(comp_a.as_os_str());
        } else {
            break;
        }
    }
    if common.as_os_str().is_empty() {
        None
    } else {
        Some(common)
    }
}


pub fn analyze_workflow_logs(logs_str: &str) {
    let logs: serde_json::Value = serde_json::from_str(logs_str).expect("Invalid logs JSON");
    let mut found_failure = false;
    for (_job_id, job_info) in logs.as_object().unwrap() {
        let status = job_info["status"].as_str().unwrap_or("unknown");
        let job_name = job_info["job_name"].as_str().unwrap_or("unknown");
        let logs_text = job_info["logs"].as_str().unwrap_or("");
        if status == "failed" {
            eprintln!("❌ Workflow execution failed at step {job_name}:");
            eprintln!("Logs:\n{logs_text}\n");
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
                eprintln!("❌ Workflow execution failed. Workflow step {job_name} may have encountered an error:");
                eprintln!("Logs:\n{logs_text}\n");
            }
        }
    }
}

fn save_workflow_name(instance_url: &str, name: &str) -> std::io::Result<()> {
    let file_path = status_file_path();
    let mut workflows: HashMap<String, Vec<String>> = if file_path.exists() {
        let content = fs::read_to_string(&file_path)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };
    let entry = workflows.entry(instance_url.to_string()).or_default();
    if !entry.contains(&name.to_string()) {
        entry.push(name.to_string());
    }
    fs::write(&file_path, serde_json::to_string_pretty(&workflows)?)?;
    Ok(())
}

pub(super) fn get_saved_workflows(instance_url: &str) -> Vec<String> {
    let file_path = status_file_path();
    if !file_path.exists() {
        return vec![];
    }
    let content = fs::read_to_string(&file_path).unwrap_or_default();
    let workflows: HashMap<String, Vec<String>> = serde_json::from_str(&content).unwrap_or_default();
    workflows.get(instance_url).cloned().unwrap_or_default()
}

fn derive_workflow_name(file: &std::path::Path, config: Option<&config::Config>) -> String {
    let file_stem = file.file_stem().unwrap_or_default().to_string_lossy();
    config
        .as_ref()
        .map_or_else(|| file_stem.to_string(), |c| format!("{} - {}", c.workflow.name, file_stem))
}
