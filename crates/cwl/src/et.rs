use super::{
    inputs::{deserialize_inputs, CommandInputParameter},
    outputs::{deserialize_outputs, CommandOutputParameter},
    requirements::{deserialize_requirements, Requirement},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionTool {
    pub cwl_version: String,
    pub class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(deserialize_with = "deserialize_inputs")]
    pub inputs: Vec<CommandInputParameter>,
    #[serde(deserialize_with = "deserialize_outputs")]
    pub outputs: Vec<CommandOutputParameter>,
    pub expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_requirements")]
    #[serde(default)]
    pub requirements: Option<Vec<Requirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_requirements")]
    #[serde(default)]
    pub hints: Option<Vec<Requirement>>,
}

impl Default for ExpressionTool {
    fn default() -> Self {
        Self {
            cwl_version: Default::default(),
            class: String::from("ExpressionTool"),
            id: Default::default(),
            label: Default::default(),
            doc: Default::default(),
            inputs: Default::default(),
            outputs: Default::default(),
            expression: Default::default(),
            requirements: Default::default(),
            hints: Default::default(),
        }
    }
}