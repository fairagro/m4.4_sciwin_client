use super::{
    deserialize::{deserialize_list, Identifiable},
    inputs::{deserialize_inputs, CommandInputParameter, WorkflowStepInput},
    outputs::WorkflowOutputParameter,
    requirements::{deserialize_requirements, Requirement},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Represents a CWL Workflow, a  process characterized by multiple subprocess steps,
/// where step outputs are connected to the inputs of downstream steps to form a
/// directed acyclic graph, and independent steps may run concurrently.
///
/// Reference: [CWL Workflow Specification](https://www.commonwl.org/v1.2/Workflow.html)
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub cwl_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_requirements")]
    #[serde(default)]
    pub requirements: Option<Vec<Requirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_requirements")]
    pub hints: Option<Vec<Requirement>>,
    #[serde(deserialize_with = "deserialize_inputs")]
    pub inputs: Vec<CommandInputParameter>,
    #[serde(deserialize_with = "deserialize_list")]
    pub outputs: Vec<WorkflowOutputParameter>,
    #[serde(deserialize_with = "deserialize_list")]
    pub steps: Vec<WorkflowStep>,
}

impl Default for Workflow {
    fn default() -> Self {
        Self {
            id: None,
            label: None,
            doc: None,
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
        self.steps.iter().map(|s| s.id.clone()).any(|x| x == *id)
    }

    pub fn has_input(&self, id: &str) -> bool {
        self.inputs.iter().map(|s| s.id.clone()).any(|x| x == *id)
    }

    pub fn has_output(&self, id: &str) -> bool {
        self.outputs.iter().map(|s| s.id.clone()).any(|x| x == *id)
    }

    pub fn has_step_input(&self, id: &str) -> bool {
        self.steps.iter().any(|step| {
            step.in_.clone().into_values().any(|val| {
                let src = match val {
                    WorkflowStepInput::String(str) => str,
                    WorkflowStepInput::Parameter(par) => par.source.unwrap_or_default(),
                };
                src == id
            })
        })
    }

    pub fn has_step_output(&self, output_source: &str) -> bool {
        let parts = output_source.split('/').collect::<Vec<_>>();
        if parts.len() != 2 {
            return false;
        }
        let step = self.get_step(parts[0]);
        if step.is_none() {
            return false;
        }

        step.unwrap().out.iter().any(|output| output == parts[1])
    }

    pub fn get_step(&self, id: &str) -> Option<&WorkflowStep> {
        self.steps.iter().find(|s| s.id == *id)
    }

    pub fn sort_steps(&self) -> Result<Vec<String>, String> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        for step in &self.steps {
            in_degree.entry(step.id.clone()).or_insert(0);

            for input in step.in_.values() {
                let parts: Vec<&str> = match input {
                    WorkflowStepInput::String(string) => string.split('/').collect(),
                    WorkflowStepInput::Parameter(parameter) => {
                        if let Some(source) = &parameter.source {
                            source.split('/').collect()
                        } else {
                            vec![]
                        }
                    }
                };

                if parts.len() == 2 {
                    let dependency = parts[0];
                    graph.entry(dependency.to_string()).or_default().push(step.id.clone());
                    *in_degree.entry(step.id.clone()).or_insert(0) += 1;
                }
            }
        }
        let mut queue: VecDeque<String> = in_degree.iter().filter(|&(_, &degree)| degree == 0).map(|(id, _)| id.clone()).collect();

        let mut sorted_steps = Vec::new();
        while let Some(step) = queue.pop_front() {
            sorted_steps.push(step.clone());

            if let Some(dependents) = graph.get(&step) {
                for dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        if sorted_steps.len() != self.steps.len() {
            return Err("❗ Cycle detected in the workflow".into());
        }

        Ok(sorted_steps)
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStep {
    #[serde(default)]
    pub id: String,
    pub run: String,
    pub in_: HashMap<String, WorkflowStepInput>,
    pub out: Vec<String>,
}
impl Identifiable for WorkflowStep {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_identifyable() {
        let mut input = WorkflowStep::default();
        assert_eq!(input.id(), "");
        input.set_id("test".to_string());
        assert_eq!(input.id(), "test");
    }

    #[test]
    fn test_workflow_steps() {
        let contents = fs::read_to_string("../../tests/test_data/hello_world/workflows/main/main.cwl").unwrap();
        let workflow: Workflow = serde_yaml::from_str(&contents).unwrap();

        assert!(workflow.has_step("calculation"));
        assert!(workflow.has_step("plot"));
        assert!(!workflow.has_step("bogus"));

        assert!(workflow.has_input("population"));
        assert!(workflow.has_input("speakers"));
        assert!(!workflow.has_input("bogus"));

        assert!(workflow.has_output("out"));
        assert!(!workflow.has_output("bogus"));

        assert!(workflow.has_step_input("calculation/results"));
        assert!(!workflow.has_step_input("plot/results"));
        assert!(!workflow.has_step_input("bogus"));

        assert!(workflow.has_step_output("calculation/results"));
        assert!(!workflow.has_step_output("calculation/bogus"));
        assert!(!workflow.has_step_output("bogus"));
    }
}
