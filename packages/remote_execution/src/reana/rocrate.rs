use crate::reana::{auth::get_or_prompt_credential, workflow::analyze_workflow_logs};
use reana::{
    api::{get_workflow_logs, get_workflow_specification, get_workflow_status, get_workflow_workspace},
    reana::Reana,
    rocrate::create_ro_crate,
};
use std::{error::Error, fs, path::PathBuf};

pub fn export_rocrate(workflow_name: &str, ro_crate_dir: Option<&String>) -> Result<(), Box<dyn Error>> {
    let reana_instance = get_or_prompt_credential("reana", "instance", "Enter REANA instance URL: ")?;
    let reana_token = get_or_prompt_credential("reana", "token", "Enter REANA access token: ")?;
    let reana = Reana::new(&reana_instance, &reana_token);

    // Get workflow status, only export if finished?
    let status_response = get_workflow_logs(&reana, workflow_name).map_err(|e| format!("Failed to fetch workflow status: {e}"))?;
    let workflow_status = status_response["status"].as_str().unwrap_or("unknown");
    match workflow_status {
        "finished" => {
            let workflow_json = get_workflow_specification(&reana, workflow_name)?;
            let config_path = PathBuf::from("workflow.toml");
            let config_str = fs::read_to_string(&config_path)?;
            let specification = workflow_json
                .get("specification")
                .ok_or("❌ 'specification' field missing in workflow JSON")?;
            let logs = get_workflow_logs(&reana, workflow_name)?;
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
            create_ro_crate(
                specification,
                &logs_str,
                &conforms_to,
                ro_crate_dir.cloned(),
                &workspace_files,
                workflow_name,
                &config_str,
            )?;
        }
        "failed" => {
            let logs = get_workflow_status(&reana, workflow_name)?;
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
