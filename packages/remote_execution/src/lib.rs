use std::path::PathBuf;
mod reana;

pub fn schedule_run(file: &PathBuf, input_file: &Option<PathBuf>) -> Result<String, Box<dyn std::error::Error>> {
    reana::execute_remote_start(file, input_file)
}

pub fn check_status(workflow_name: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    reana::check_remote_status(workflow_name)
}

pub fn download_results(workflow_name: &str, output_dir: Option<&String>) -> Result<(), Box<dyn std::error::Error>> {
    reana::download_remote_results(workflow_name, output_dir)
}

pub fn export_rocrate(workflow_name: &str, output_dir: Option<&String>) -> Result<(), Box<dyn std::error::Error>> {
    reana::export_rocrate(workflow_name, output_dir)
}

pub fn logout() -> Result<(), Box<dyn std::error::Error>> {
    reana::logout_reana()
}

pub fn watch(workflow_name: &str, rocrate: bool) -> Result<(), Box<dyn std::error::Error>> {
    reana::watch(workflow_name, rocrate)
}
