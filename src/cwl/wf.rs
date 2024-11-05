use std::collections::HashMap;

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

impl Workflow {
    pub fn has_step(self: &Self, id: &str) -> bool {
        self.steps.iter().map(|s| s.id.clone()).collect::<Vec<_>>().contains(&id.to_string())
    }

    pub fn has_input(self: &Self, id: &str) -> bool {
        self.inputs.iter().map(|s| s.id.clone()).collect::<Vec<_>>().contains(&id.to_string())
    }

    pub fn get_step(self: &Self, id: &str) -> Option<&WorkflowStep> {
        self.steps.iter().find(|s| s.id == id.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStep {
    pub id: String,
    pub run: String,
    pub in_: HashMap<String, String>,
    pub out: Vec<String>,
}
