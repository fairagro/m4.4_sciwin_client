use super::deserialize::{deserialize_list, Identifiable};
use super::{
    clt::CommandLineTool,
    inputs::CommandInputParameter,
    loader::{load_tool, resolve_filename},
    outputs::WorkflowOutputParameter,
    requirements::{deserialize_requirements, Requirement},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub class: String,
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
    #[serde(deserialize_with = "deserialize_list")]
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

    /// Adds a connection between an input and a CommandLineTool. The tool will be registered as step if it is not already and an Workflow input will be added.
    pub fn add_input_connection(&mut self, from_input: &str, to: &String) -> Result<(), Box<dyn Error>> {
        let to_parts = to.split('/').collect::<Vec<_>>();

        let to_filename = resolve_filename(to_parts[0]);
        let to_tool: CommandLineTool = load_tool(&to_filename)?;
        let to_slot = to_tool.inputs.iter().find(|i| i.id == to_parts[1]).expect("No slot");

        //register input
        if !self.has_input(from_input) {
            self.inputs.push(CommandInputParameter::default().with_id(from_input).with_type(to_slot.type_.clone()));
        }

        self.add_new_step_if_not_exists(to_parts[0], &to_tool);
        //add input in step
        self.steps
            .iter_mut()
            .find(|step| step.id == to_parts[0])
            .unwrap()
            .in_
            .insert(to_parts[1].to_string(), from_input.to_owned());

        println!("➕ Added or updated connection from inputs.{} to {} in workflow", from_input, to);

        Ok(())
    }

    /// Adds a connection between an output and a CommandLineTool. The tool will be registered as step if it is not already and an Workflow output will be added.
    pub fn add_output_connection(&mut self, from: &String, to_output: &str) -> Result<(), Box<dyn Error>> {
        let from_parts = from.split('/').collect::<Vec<_>>();

        let from_filename = resolve_filename(from_parts[0]);
        let from_tool: CommandLineTool = load_tool(&from_filename)?;
        let from_slot = from_tool.outputs.iter().find(|i| i.id == from_parts[1]).expect("No slot");

        if !self.has_output(to_output) {
            self.outputs.push(WorkflowOutputParameter::default().with_id(to_output).clone());
        }

        let output = self.outputs.iter_mut().find(|o| o.id == to_output).unwrap();
        output.type_ = from_slot.type_.clone();
        output.output_source = from.clone();

        println!("➕ Added or updated connection from {} to outputs.{} in workflow!", from, to_output);

        Ok(())
    }

    /// Adds a connection between two a CommandLineToos. The tools will be registered as step if registered not already.
    pub fn add_step_connection(&mut self, from: &str, to: &str) -> Result<(), Box<dyn Error>> {
        //handle from
        let from_parts = from.split('/').collect::<Vec<_>>();
        //check if step already exists and create if not
        if !self.has_step(from_parts[0]) {
            let from_filename = resolve_filename(from_parts[0]);
            let from_tool: CommandLineTool = load_tool(&from_filename)?;
            let from_outputs = from_tool.get_output_ids();
            if !from_outputs.contains(&from_parts[1].to_string()) {
                return Err(format!(
                    "❌ Tool {} does not have output `{}`. Cannot not create node from {} in Workflow!",
                    from_parts[0], from_parts[1], from_filename
                )
                .into());
            }

            //create step
            self.add_new_step_if_not_exists(from_parts[0], &from_tool);
        } else {
            println!("🔗 Found step {} in workflow. Not changing that!", from_parts[0]);
        }

        //handle to
        let to_parts = to.split('/').collect::<Vec<_>>();
        //check if step exists
        if !self.has_step(to_parts[0]) {
            let to_filename = resolve_filename(to_parts[0]);
            let to_tool: CommandLineTool = load_tool(&to_filename)?;

            self.add_new_step_if_not_exists(to_parts[0], &to_tool);
        }

        let step = self.steps.iter_mut().find(|s| s.id == to_parts[0]).unwrap(); //safe here!
        step.in_.insert(to_parts[1].to_string(), from.to_string());

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStep {
    #[serde(default)]
    pub id: String,
    pub run: String,
    pub in_: HashMap<String, String>,
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
