use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{clt::{CommandInputParameter, CommandLineTool, Requirement}, types::CWLType};

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
    pub outputs: Vec<WorkflowOutputParameter>,
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
    pub fn has_step(&self, id: &str) -> bool {
        self.steps.iter().map(|s| s.id.clone()).collect::<Vec<_>>().contains(&id.to_string())
    }

    pub fn has_input(&self, id: &str) -> bool {
        self.inputs.iter().map(|s| s.id.clone()).collect::<Vec<_>>().contains(&id.to_string())
    }

    pub fn has_output(&self, id: &str) -> bool {
        self.outputs.iter().map(|s| s.id.clone()).collect::<Vec<_>>().contains(&id.to_string())
    }

    pub fn get_step(&self, id: &str) -> Option<&WorkflowStep> {
        self.steps.iter().find(|s| s.id == *id)
    }

    pub fn add_new_step_if_not_exists(&mut self, name: &str, tool: &CommandLineTool) {
        if !self.has_step(name) {
            let workflow_step = WorkflowStep {
                id: name.to_string(),
                run: format!("../{}/{}.cwl", name, name),
                in_: HashMap::new(),
                out: tool.get_output_ids(),
            };
            self.steps.push(workflow_step);

            println!("➕ Added step {} to workflow", name);
        }
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

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowOutputParameter{
    pub id: String,
    pub type_: CWLType,
    pub output_source: String,
}

impl WorkflowOutputParameter {
    pub fn with_id(&mut self, id: &str) -> &Self {
        self.id = id.to_string();
        self
    }
}