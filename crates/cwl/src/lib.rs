use clt::CommandLineTool;
use et::ExpressionTool;
use inputs::deserialize_inputs;
use inputs::CommandInputParameter;
use requirements::deserialize_requirements;
use requirements::Requirement;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::ops::Deref;
use std::ops::DerefMut;
use std::{error::Error, fmt::Debug, fs, path::Path};
use wf::Workflow;

pub mod clt;
pub mod deserialize;
pub mod et;
pub mod format;
pub mod inputs;
pub mod outputs;
pub mod requirements;
pub mod types;
pub mod wf;

#[derive(Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum CWLDocument {
    CommandLineTool(CommandLineTool),
    Workflow(Workflow),
    ExpressionTool(ExpressionTool),
}

impl Deref for CWLDocument {
    type Target = DocumentBase;

    fn deref(&self) -> &Self::Target {
        match self{
            CWLDocument::CommandLineTool(clt) => &clt.base,
            CWLDocument::Workflow(wf) => &wf.base,
            CWLDocument::ExpressionTool(et) => &et.base,
        }
    }
}

impl DerefMut for CWLDocument {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self{
            CWLDocument::CommandLineTool(clt) => &mut clt.base,
            CWLDocument::Workflow(wf) => &mut wf.base,
            CWLDocument::ExpressionTool(et) => &mut et.base,
        }
    }
}

impl<'de> Deserialize<'de> for CWLDocument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        let class = value
            .get("class")
            .ok_or_else(|| serde::de::Error::missing_field("class"))?
            .as_str()
            .ok_or_else(|| serde::de::Error::missing_field("class must be of type string"))?;

        match class {
            "CommandLineTool" => serde_yaml::from_value(value)
                .map(CWLDocument::CommandLineTool)
                .map_err(serde::de::Error::custom),
            "ExpressionTool" => serde_yaml::from_value(value)
                .map(CWLDocument::ExpressionTool)
                .map_err(serde::de::Error::custom),
            "Workflow" => serde_yaml::from_value(value).map(CWLDocument::Workflow).map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom(format!("Unknown variant of CWL file: {class}"))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentBase {
    pub class: String,
    pub cwl_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(deserialize_with = "deserialize_inputs")]
    pub inputs: Vec<CommandInputParameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_requirements")]
    #[serde(default)]
    pub requirements: Option<Vec<Requirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_requirements")]
    #[serde(default)]
    pub hints: Option<Vec<Requirement>>,
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

/// Loads a CWL ExpresionTool from disk and parses given YAML
pub fn load_expression_tool<P: AsRef<Path> + Debug>(filename: P) -> Result<ExpressionTool, Box<dyn Error>> {
    let path = filename.as_ref();
    if !path.exists() {
        return Err(format!("❌ ExpressionTool {:?} does not exist.", filename).into());
    }
    let contents = fs::read_to_string(path)?;
    let tool: ExpressionTool = serde_yaml::from_str(&contents).map_err(|e| format!("❌ Could not read ExpressionTool {:?}: {}", filename, e))?;

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
