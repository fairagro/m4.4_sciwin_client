use super::{
    clt::CommandLineTool,
    deserialize::{deserialize_list, Identifiable},
    inputs::{deserialize_inputs, CommandInputParameter, WorkflowStepInput},
    loader::{load_tool, resolve_filename},
    outputs::WorkflowOutputParameter,
    requirements::{deserialize_requirements, Requirement},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    error::Error,
};

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
            .insert(to_parts[1].to_string(), WorkflowStepInput::String(from_input.to_owned()));

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
        step.in_.insert(to_parts[1].to_string(), WorkflowStepInput::String(from.to_string()));

        Ok(())
    }

    /// Removes a connection between two CommandLineTools by removing input from tool_y that is also output of tool_x.
    pub fn remove_step_connection(&mut self, from: &str, to: &str) -> Result<(), Box<dyn Error>> {
        let from_parts = from.split('/').collect::<Vec<_>>();
        let to_parts = to.split('/').collect::<Vec<_>>();
        if from_parts.len() != 2 {
            return Err(format!("❌ Invalid '--from' format: {}. Please use tool/parameter or @inputs/parameter.", from).into());
        }
        if to_parts.len() != 2 {
            return Err(format!("❌ Invalid '--to' format: {}. Please use tool/parameter or @outputs/parameter.", to).into());
        }
        if !self.has_step(to_parts[0]) {
            return Err(format!("❌ Step {} not found!", to_parts[0]).into());
        }
        let step = self.steps.iter_mut().find(|s| s.id == to_parts[0]);
        // If the step is found, try to remove the connection by removing input from tool_y that uses output of tool_x
        //Input is empty, change that?
        if let Some(step) = step {
            if step.in_.remove(to_parts[1]).is_some() {
                println!("🔗 Successfully disconnected {} from {}", from, to);
            } else {
                println!(
                    "⚠️ No connection found between {} and {}. Nothing to disconnect.", from, to);
            }
            Ok(())
        } else {
            Err(format!("❌ Failed to find step {} in workflow!", to_parts[0]).into())
        }
    }
    
    /// Removes an input from inputs and removes it from CommandLineTool input.
    pub fn remove_input_connection(&mut self, from_input: &str, to: &str) -> Result<(), Box<dyn Error>> {
        let to_parts = to.split('/').collect::<Vec<_>>();
        if to_parts.len() != 2 {
            return Err(format!("❌ Invalid 'to' format for input connection: {} to:{}", from_input, to).into());
        }
        if let Some(index) = self.inputs.iter().position(|s| s.id == *from_input.to_string())
        {
            self.inputs.remove(index);
        }
        if let Some(step) = self.steps.iter_mut().find(|s| s.id == to_parts[0]) {
            if step.in_.remove(to_parts[1]).is_some() {
                println!("➖ Successfully disconnected input {} from {}", from_input, to);
            } else {
                println!("⚠️ No input connection found for {} to disconnect.", from_input);
            }
        } else {
            return Err(format!("❌ Step {} not found in workflow!", to_parts[0]).into());
        }

        Ok(())
    }
    /// Removes a connection between an output and a CommandLineTool.
    pub fn remove_output_connection(&mut self, to_output: &str) -> Result<(), Box<dyn Error>> {
        if let Some(index) = self.outputs.iter().position(|o| o.id == to_output) {
            // Remove the output connection, only removes outputs so far better to change that to also remove output of step?
            self.outputs.remove(index);
            println!(
                "➖ Removed connection to outputs.{} from workflow!",
                to_output
            );
        } else {
            return Err(format!("No output connection found for '{}'", to_output).into());
        }

        Ok(())
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
    use crate::cwl::loader::load_workflow;

    #[test]
    fn test_workflow_has_step() {
        let workflow = load_workflow("tests/test_data/hello_world/workflows/main/main.cwl").unwrap();

        assert!(workflow.has_step("calculation"));
        assert!(workflow.has_step("plot"));
        assert!(!workflow.has_step("bogus"));
    }

    #[test]
    fn test_workflow_has_input() {
        let workflow = load_workflow("tests/test_data/hello_world/workflows/main/main.cwl").unwrap();

        assert!(workflow.has_input("population"));
        assert!(workflow.has_input("speakers"));
        assert!(!workflow.has_input("bogus"));
    }

    #[test]
    fn test_workflow_has_output() {
        let workflow = load_workflow("tests/test_data/hello_world/workflows/main/main.cwl").unwrap();

        assert!(workflow.has_output("out"));
        assert!(!workflow.has_output("bogus"));
    }

    #[test]
    fn test_workflow_has_step_input() {
        let workflow = load_workflow("tests/test_data/hello_world/workflows/main/main.cwl").unwrap();

        assert!(workflow.has_step_input("calculation/results"));
        assert!(!workflow.has_step_input("plot/results"));
    }

    #[test]
    fn test_workflow_has_step_output() {
        let workflow = load_workflow("tests/test_data/hello_world/workflows/main/main.cwl").unwrap();

        assert!(workflow.has_step_output("calculation/results"));
        assert!(!workflow.has_step_output("calculation/bogus"));
    }
}
