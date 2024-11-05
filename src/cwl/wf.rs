use serde::{Deserialize, Serialize};

use super::clt::{CommandInputParameter, CommandOutputParameter, Requirement};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub class: String,
    pub cwl_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirements: Option<Vec<Requirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<Requirement>>,
    pub inputs: Vec<CommandInputParameter>,
    pub outputs: Vec<CommandOutputParameter>,
    pub steps: Vec<WorkflowStep>,
}

impl Default for Workflow {
    fn default() -> Self {
        Self {
            class: String::from("Workflow"),
            cwl_version: String::from("v1.2"),
            requirements: Default::default(),
            hints: Default::default(),
            inputs: Default::default(),
            outputs: Default::default(),
            steps: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStep {
    pub id: String,
    pub run: String,
    pub in_: Vec<String>,
    pub out: Vec<String>,
}
