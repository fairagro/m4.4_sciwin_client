use super::{clt::CommandLineTool, wf::Workflow};
use crate::io::get_workflows_folder;
use std::{error::Error, fmt::Debug, fs, path::Path};

/// Locates CWL File by name
pub fn resolve_filename(cwl_filename: &str) -> String {
    format!("{}{}/{}.cwl", get_workflows_folder(), cwl_filename, cwl_filename)
}

/// Loads a CWL CommandLineTool from disk and parses given YAML
pub fn load_tool<P: AsRef<Path> + Debug>(filename: P) -> Result<CommandLineTool, Box<dyn Error>> {
    let path = filename.as_ref();
    if !path.exists() {
        return Err(format!("❌ Tool {:?} does not exist.", filename).into());
    }
    let contents = fs::read_to_string(path)?;
    let tool: CommandLineTool = serde_yml::from_str(&contents).map_err(|e| format!("❌ Could not read CommandLineTool {:?}: {}", filename, e))?;

    Ok(tool)
}

/// Loads a CWL Workflow from disk and parses given YAML
pub fn load_workflow<P: AsRef<Path> + Debug>(filename: P) -> Result<Workflow, Box<dyn Error>> {
    let path = filename.as_ref();
    if !path.exists() {
        return Err(format!("❌ Workflow {:?} does not exist, yet!", filename).into());
    }
    let contents = fs::read_to_string(path)?;
    let workflow: Workflow = serde_yml::from_str(&contents).map_err(|e| format!("❌ Could not read Workflow {:?}: {}", filename, e))?;
    Ok(workflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_filename() {
        let name = "my-tool";
        let filename = resolve_filename(name);
        assert_eq!(filename, "workflows/my-tool/my-tool.cwl".to_string());
    }

    #[test]
    fn test_load_tool() {
        let path = "tests/test_data/echo.cwl";

        let tool_result = load_tool(path);
        assert!(tool_result.is_ok());
    }

    #[test]
    fn test_load_workflow() {
        let path = "tests/test_data/test-wf.cwl";

        let wf_result = load_workflow(path);
        assert!(wf_result.is_ok());
    }
}
