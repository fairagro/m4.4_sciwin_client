use super::{
    deserialize::{deserialize_list, Identifiable},
    inputs::WorkflowStepInputParameter,
    outputs::WorkflowOutputParameter,
    requirements::{deserialize_hints, deserialize_requirements, Requirement},
    CWLDocument, DocumentBase,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_yaml::Value;
use std::{
    collections::{HashMap, VecDeque},
    ops::{Deref, DerefMut},
};

/// Represents a CWL Workflow, a  process characterized by multiple subprocess steps,
/// where step outputs are connected to the inputs of downstream steps to form a
/// directed acyclic graph, and independent steps may run concurrently.
///
/// Reference: <https://www.commonwl.org/v1.2/Workflow.html>
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    #[serde(flatten)]
    pub base: DocumentBase,
    #[serde(deserialize_with = "deserialize_list")]
    pub outputs: Vec<WorkflowOutputParameter>,
    #[serde(deserialize_with = "deserialize_list")]
    pub steps: Vec<WorkflowStep>,
}

impl Default for Workflow {
    fn default() -> Self {
        Self {
            base: DocumentBase {
                id: None,
                label: None,
                doc: None,
                class: String::from("Workflow"),
                cwl_version: Some(String::from("v1.2")),
                requirements: Default::default(),
                hints: Default::default(),
                inputs: Default::default(),
                intent: Option::default(),
            },
            outputs: Default::default(),
            steps: Default::default(),
        }
    }
}

impl Deref for Workflow {
    type Target = DocumentBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Workflow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Workflow {
    /// Checks whether the `Workflow` has a `WorkflowStep` of id `id`
    pub fn has_step(&self, id: &str) -> bool {
        self.steps.iter().map(|s| s.id.clone()).any(|x| x == *id)
    }

    /// Checks whether the `Workflow` has an input of id `id`
    pub fn has_input(&self, id: &str) -> bool {
        self.inputs.iter().map(|s| s.id.clone()).any(|x| x == *id)
    }

    /// Checks whether the `Workflow` has an ouput of id `id`
    pub fn has_output(&self, id: &str) -> bool {
        self.outputs.iter().map(|s| s.id.clone()).any(|x| x == *id)
    }

    /// Checks whether the `Workflow` has a `WorkflowStep` with an input of id `id`
    pub fn has_step_input(&self, id: &str) -> bool {
        self.steps
            .iter()
            .any(|step| step.in_.iter().any(|val| val.source == Some(id.to_string())))
    }

    /// Checks whether the `Workflow` has a `WorkflowStep` with an ouput of id `id`
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

    /// Returns the `Workflow`'s `WorkflowStep` with id `id`
    pub fn get_step(&self, id: &str) -> Option<&WorkflowStep> {
        self.steps.iter().find(|s| s.id == *id)
    }

    /// Sorts `WorkflowStep`s to get the sequence of execution
    pub fn sort_steps(&self) -> Result<Vec<String>, String> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        for step in &self.steps {
            in_degree.entry(step.id.clone()).or_insert(0);

            for input in &step.in_ {
                let parts: Vec<&str> = if let Some(source) = &input.source {
                    source.split('/').collect()
                } else {
                    vec![]
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
    pub run: StringOrDocument,    
    #[serde(deserialize_with = "deserialize_workflow_inputs")]
    pub in_: Vec<WorkflowStepInputParameter>,
    pub out: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(deserialize_with = "deserialize_requirements")]
    pub requirements: Vec<Requirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(deserialize_with = "deserialize_hints")]
    pub hints: Vec<Requirement>,
}

impl Identifiable for WorkflowStep {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

pub fn deserialize_workflow_inputs<'de, D>(deserializer: D) -> Result<Vec<WorkflowStepInputParameter>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    let parameters = match value {
        Value::Sequence(seq) => seq
            .into_iter()
            .map(|item| {
                let param: WorkflowStepInputParameter = serde_yaml::from_value(item).map_err(serde::de::Error::custom)?;
                Ok(param)
            })
            .collect::<Result<Vec<_>, _>>()?,
        Value::Mapping(map) => map
            .into_iter()
            .map(|(key, value)| {
                let id = key.as_str().ok_or_else(|| serde::de::Error::custom("Expected string key"))?;
                let param = if let Value::String(source_str) = value {
                    WorkflowStepInputParameter::default().with_id(id).with_source(source_str)
                } else {
                    let mut param: WorkflowStepInputParameter = serde_yaml::from_value(value).map_err(serde::de::Error::custom)?;
                    param.id = id.to_string();
                    param
                };

                Ok(param)
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(serde::de::Error::custom("Expected sequence or mapping for inputs")),
    };

    Ok(parameters)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum StringOrDocument {
    String(String),
    Document(Box<CWLDocument>),
}

impl Default for StringOrDocument {
    fn default() -> Self {
        Self::String(String::default())
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
