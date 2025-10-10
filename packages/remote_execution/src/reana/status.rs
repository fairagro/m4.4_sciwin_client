use crate::reana::{
    auth::get_or_prompt_credential,
    workflow::{analyze_workflow_logs, get_saved_workflows},
};
use reana::{api::get_workflow_status, reana::Reana};
use std::{error::Error, path::PathBuf};
pub(super) fn status_file_path() -> PathBuf {
    std::env::temp_dir().join("workflow_status_list.json")
}

pub fn check_remote_status(workflow_name: &Option<String>) -> Result<(), Box<dyn Error>> {
    let reana_instance = get_or_prompt_credential("reana", "instance", "Enter REANA instance URL: ")?;
    let reana_token = get_or_prompt_credential("reana", "token", "Enter REANA access token: ")?;
    let reana = Reana::new(&reana_instance, &reana_token);

    if let Some(name) = workflow_name {
        evaluate_workflow_status(&reana, name, true)?;
    } else {
        let workflows = get_saved_workflows(&reana_instance);
        if workflows.is_empty() {
            return Err(format!("No workflows saved for REANA instance '{reana_instance}'").into());
        }
        for name in workflows {
            evaluate_workflow_status(&reana, &name, false)?;
        }
    }
    Ok(())
}

fn evaluate_workflow_status(reana: &Reana, name: &str, analyze_logs: bool) -> Result<(), Box<dyn Error>> {
    let status_response = get_workflow_status(reana, name).map_err(|e| format!("Failed to fetch workflow status: {e}"))?;
    let status = status_response["status"].as_str().unwrap_or("unknown");
    let created = status_response["created"].as_str().unwrap_or("unknown");
    let icon = if status == "finished" {
        "✅"
    } else if status == "failed" {
        "❌"
    } else {
        "⌛"
    };
    eprintln!("{icon} {name} {status} created at {created}");
    //if single workflow failed, get step name and logs
    if status == "failed"
        && analyze_logs
        && let Some(logs_str) = status_response["logs"].as_str()
    {
        analyze_workflow_logs(logs_str);
    }
    Ok(())
}
