use super::{clt::CommandLineTool, wf::Workflow};
use crate::io::get_workflows_folder;
use std::{error::Error, fs, path::Path};

/// Locates CWL File by name
pub fn resolve_filename(cwl_filename: &str) -> String {
    format!("{}{}/{}.cwl", get_workflows_folder(), cwl_filename, cwl_filename)
}

/// Loads a CWL CommandLineTool from disk and parses given YAML
pub fn load_tool(filename: &str) -> Result<CommandLineTool, Box<dyn Error>> {
    let path = Path::new(&filename);
    if !path.exists() {
        return Err(format!("❌ Tool {} does not exist.", filename).into());
    }
    let contents = fs::read_to_string(path)?;
    let tool: CommandLineTool = serde_yml::from_str(&contents).map_err(|e| format!("❌ Could not read CommandLineTool {}: {}", filename, e))?;

    Ok(tool)
}

/// Loads a CWL Workflow from disk and parses given YAML
pub fn load_workflow(filename: &str) -> Result<Workflow, Box<dyn Error>> {
    let path = Path::new(&filename);
    if !path.exists() {
        return Err(format!("❌ Workflow {} does not exist, yet!", filename).into());
    }
    let contents = fs::read_to_string(path)?;
    let workflow: Workflow = serde_yml::from_str(&contents)?;
    Ok(workflow)
}
