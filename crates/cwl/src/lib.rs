use clt::CommandLineTool;
use serde::Deserialize;
use std::{error::Error, fmt::Debug, fs, path::Path};
use wf::Workflow;

pub mod clt;
pub mod deserialize;
pub mod format;
pub mod inputs;
pub mod outputs;
pub mod requirements;
pub mod types;
pub mod wf;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum CWLDocument {
    CommandLineTool(CommandLineTool),
    Workflow(Workflow),
}

/// Loads a CWL CommandLineTool from disk and parses given YAML
pub fn load_tool<P: AsRef<Path> + Debug>(filename: P) -> Result<CommandLineTool, Box<dyn Error>> {
    let path = filename.as_ref();
    if !path.exists() {
        return Err(format!("❌ Tool {:?} does not exist.", filename).into());
    }
    let contents = fs::read_to_string(path)?;
    let tool: CommandLineTool = serde_yaml::from_str(&contents).map_err(|e| format!("❌ Could not read CommandLineTool {:?}: {}", filename, e))?;

    Ok(tool)
}

/// Loads a CWL Workflow from disk and parses given YAML
pub fn load_workflow<P: AsRef<Path> + Debug>(filename: P) -> Result<Workflow, Box<dyn Error>> {
    let path = filename.as_ref();
    if !path.exists() {
        return Err(format!("❌ Workflow {:?} does not exist, yet!", filename).into());
    }
    let contents = fs::read_to_string(path)?;
    let workflow: Workflow = serde_yaml::from_str(&contents).map_err(|e| format!("❌ Could not read Workflow {:?}: {}", filename, e))?;
    Ok(workflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("../../tests/test_data/default.cwl")]
    #[case("../../tests/test_data/echo.cwl")]
    #[case("../../tests/test_data/mkdir.cwl")]
    #[case("../../tests/test_data/hello_world/workflows/calculation/calculation.cwl")]
    #[case("../../tests/test_data/hello_world/workflows/plot/plot.cwl")]

    fn test_load_multiple_tools(#[case] filename: &str) {
        let tool = load_tool(filename);
        assert!(tool.is_ok());
    }

    #[test]
    #[should_panic]
    fn test_load_tool_fails() {
        let _ = load_tool("this is not valid").unwrap();
    }

    #[rstest]
    #[case("../../tests/test_data/mkdir_wf.cwl")]
    #[case("../../tests/test_data/test-wf.cwl")]
    #[case("../../tests/test_data/test-wf_features.cwl")]
    #[case("../../tests/test_data/wf_inout.cwl")]
    #[case("../../tests/test_data/wf_inout_dir.cwl")]
    #[case("../../tests/test_data/wf_inout_file.cwl")]
    #[case("../../tests/test_data/hello_world/workflows/main/main.cwl")]
    fn test_load_multiple_wfs(#[case] filename: &str) {
        let workflow = load_workflow(filename);
        assert!(workflow.is_ok());
    }

    #[test]
    #[should_panic]
    fn test_load_wf_fails() {
        let _ = load_workflow("this is not valid").unwrap();
    }
}
