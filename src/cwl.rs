use crate::io::{get_workflows_folder, resolve_path};
use cwl::{
    clt::CommandLineTool,
    inputs::{CommandInputParameter, WorkflowStepInput},
    load_tool,
    outputs::WorkflowOutputParameter,
    requirements::{DockerRequirement, Requirement},
    types::{DefaultValue, Entry, PathItem},
    wf::{Workflow, WorkflowStep},
};
use log::{info, warn};
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet, util::{as_24_bit_terminal_escaped, LinesWithEndings}};
use std::{collections::HashMap, error::Error};

pub trait Connectable {
    fn remove_output_connection(&mut self, from: &str, to_output: &str) -> Result<(), Box<dyn Error>>;
    fn remove_input_connection(&mut self, from_input: &str, to: &str) -> Result<(), Box<dyn Error>>;
    fn add_step_connection(&mut self, from: &str, to: &str) -> Result<(), Box<dyn Error>>;
    fn add_output_connection(&mut self, from: &str, to_output: &str) -> Result<(), Box<dyn Error>>;
    fn add_input_connection(&mut self, from_input: &str, to: &str) -> Result<(), Box<dyn Error>>;
    fn add_new_step_if_not_exists(&mut self, name: &str, tool: &CommandLineTool);
    fn remove_step_connection(&mut self, from: &str, to: &str) -> Result<(), Box<dyn Error>>;
}

pub trait Saveable {
    fn save(&mut self, path: &str) -> String;
}

impl Saveable for CommandLineTool {
    fn save(&mut self, path: &str) -> String {
        //rewire paths to new location
        for input in &mut self.inputs {
            if let Some(DefaultValue::File(value)) = &mut input.default {
                value.location = Some(resolve_path(value.get_location(), path));
            }
            if let Some(DefaultValue::Directory(value)) = &mut input.default {
                value.location = Some(resolve_path(value.get_location(), path));
            }
        }

        if let Some(requirements) = &mut self.requirements {
            for requirement in requirements {
                if let Requirement::DockerRequirement(docker) = requirement {
                    if let DockerRequirement::DockerFile {
                        docker_file: Entry::Include(include),
                        docker_image_id: _,
                    } = docker
                    {
                        include.include = resolve_path(&include.include, path)
                    }
                } else if let Requirement::InitialWorkDirRequirement(iwdr) = requirement {
                    for listing in &mut iwdr.listing {
                        if let Entry::Include(include) = &mut listing.entry {
                            include.include = resolve_path(&include.include, path)
                        }
                    }
                }
            }
        }
        self.to_string()
    }
}

impl Connectable for Workflow {
    fn add_new_step_if_not_exists(&mut self, name: &str, tool: &CommandLineTool) {
        if !self.has_step(name) {
            let workflow_step = WorkflowStep {
                id: name.to_string(),
                run: format!("../{name}/{name}.cwl"),
                in_: HashMap::new(),
                out: tool.get_output_ids(),
            };
            self.steps.push(workflow_step);

            info!("➕ Added step {name} to workflow");
        }
    }

    /// Adds a connection between an input and a CommandLineTool. The tool will be registered as step if it is not already and an Workflow input will be added.
    fn add_input_connection(&mut self, from_input: &str, to: &str) -> Result<(), Box<dyn Error>> {
        let to_parts = to.split('/').collect::<Vec<_>>();

        let to_filename = resolve_filename(to_parts[0]);
        let to_tool: CommandLineTool = load_tool(&to_filename)?;
        let to_slot = to_tool.inputs.iter().find(|i| i.id == to_parts[1]).expect("No slot");

        //register input
        if !self.has_input(from_input) {
            self.inputs
                .push(CommandInputParameter::default().with_id(from_input).with_type(to_slot.type_.clone()));
        }

        self.add_new_step_if_not_exists(to_parts[0], &to_tool);
        //add input in step
        self.steps
            .iter_mut()
            .find(|step| step.id == to_parts[0])
            .unwrap()
            .in_
            .insert(to_parts[1].to_string(), WorkflowStepInput::String(from_input.to_owned()));

        info!("➕ Added or updated connection from inputs.{from_input} to {to} in workflow");

        Ok(())
    }

    /// Adds a connection between an output and a CommandLineTool. The tool will be registered as step if it is not already and an Workflow output will be added.
    fn add_output_connection(&mut self, from: &str, to_output: &str) -> Result<(), Box<dyn Error>> {
        let from_parts = from.split('/').collect::<Vec<_>>();

        let from_filename = resolve_filename(from_parts[0]);
        let from_tool: CommandLineTool = load_tool(&from_filename)?;
        let from_slot = from_tool.outputs.iter().find(|i| i.id == from_parts[1]).expect("No slot");

        if !self.has_output(to_output) {
            self.outputs.push(WorkflowOutputParameter::default().with_id(to_output).clone());
        }

        let output = self.outputs.iter_mut().find(|o| o.id == to_output).unwrap();
        output.type_.clone_from(&from_slot.type_);
        output.output_source.clone_from(&from.to_string());

        info!("➕ Added or updated connection from {from} to outputs.{to_output} in workflow!");

        Ok(())
    }

    /// Adds a connection between two a CommandLineToos. The tools will be registered as step if registered not already.
    fn add_step_connection(&mut self, from: &str, to: &str) -> Result<(), Box<dyn Error>> {
        //handle from
        let from_parts = from.split('/').collect::<Vec<_>>();
        //check if step already exists and create if not
        if !self.has_step(from_parts[0]) {
            let from_filename = resolve_filename(from_parts[0]);
            let from_tool: CommandLineTool = load_tool(&from_filename)?;
            let from_outputs = from_tool.get_output_ids();
            if !from_outputs.contains(&from_parts[1].to_string()) {
                return Err(format!(
                    "Tool {} does not have output `{}`. Cannot not create node from {} in Workflow!",
                    from_parts[0], from_parts[1], from_filename
                )
                .into());
            }

            //create step
            self.add_new_step_if_not_exists(from_parts[0], &from_tool);
        } else {
            info!("🔗 Found step {} in workflow. Not changing that!", from_parts[0]);
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
    fn remove_step_connection(&mut self, from: &str, to: &str) -> Result<(), Box<dyn Error>> {
        let from_parts = from.split('/').collect::<Vec<_>>();
        let to_parts = to.split('/').collect::<Vec<_>>();
        if from_parts.len() != 2 {
            return Err(format!("Invalid '--from' format: {from}. Please use tool/parameter or @inputs/parameter.").into());
        }
        if to_parts.len() != 2 {
            return Err(format!("Invalid '--to' format: {to}. Please use tool/parameter or @outputs/parameter.").into());
        }
        if !self.has_step(to_parts[0]) {
            return Err(format!("Step {} not found!", to_parts[0]).into());
        }
        let step = self.steps.iter_mut().find(|s| s.id == to_parts[0]);
        // If the step is found, try to remove the connection by removing input from tool_y that uses output of tool_x
        //Input is empty, change that?
        if let Some(step) = step {
            if step.in_.remove(to_parts[1]).is_some() {
                info!("🔗 Successfully disconnected {from} from {to}");
            } else {
                warn!("No connection found between {from} and {to}. Nothing to disconnect.");
            }
            Ok(())
        } else {
            Err(format!("Failed to find step {} in workflow!", to_parts[0]).into())
        }
    }

    /// Removes an input from inputs and removes it from CommandLineTool input.
    fn remove_input_connection(&mut self, from_input: &str, to: &str) -> Result<(), Box<dyn Error>> {
        let to_parts = to.split('/').collect::<Vec<_>>();
        if to_parts.len() != 2 {
            return Err(format!("Invalid 'to' format for input connection: {from_input} to:{to}").into());
        }
        if let Some(index) = self.inputs.iter().position(|s| s.id == *from_input.to_string()) {
            self.inputs.remove(index);
        }
        if let Some(step) = self.steps.iter_mut().find(|s| s.id == to_parts[0]) {
            if step.in_.remove(to_parts[1]).is_some() {
                info!("➖ Successfully disconnected input {from_input} from {to}");
            } else {
                warn!("No input connection found for {from_input} to disconnect.");
            }
        } else {
            return Err(format!("Step {} not found in workflow!", to_parts[0]).into());
        }

        Ok(())
    }

    /// Removes a connection between an output and a `CommandLineTool`.
    fn remove_output_connection(&mut self, from: &str, to_output: &str) -> Result<(), Box<dyn Error>> {
        let from_parts = from.split('/').collect::<Vec<_>>();
        let mut removed_from_outputs = false;
        if let Some(index) = self.outputs.iter().position(|o| o.id == to_output) {
            // Remove the output connection
            self.outputs.remove(index);
            removed_from_outputs = true;
            info!("➖ Removed connection to outputs.{to_output} from workflow!");
        }
        // Check if this output is part of any step output and remove it, do we want that?
        let mut removed_from_step = false;
        if let Some(step) = self.steps.iter_mut().find(|s| s.id == from_parts[0]) {
            if let Some(output_index) = step.out.iter().position(|out| out == from_parts[1]) {
                step.out.remove(output_index);
                removed_from_step = true;
                info!("➖ Removed output {to_output} from step {} in workflow!", step.id);
            }
        }
        if !removed_from_outputs {
            warn!("No matching output found for '{to_output}' in workflow outputs.");
        }
        if !removed_from_step {
            warn!("No matching step output found for '{to_output}'.");
        }

        Ok(())
    }
}

/// Locates CWL File by name
pub fn resolve_filename(cwl_filename: &str) -> String {
    format!("{}{}/{}.cwl", get_workflows_folder(), cwl_filename, cwl_filename)
}

pub fn highlight_cwl(yaml: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension("yaml").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["InspiredGitHub"]);

    for line in LinesWithEndings::from(yaml) {
        let ranges = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("{escaped}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cwl::{
        clt::Command,
        inputs::CommandLineBinding,
        requirements::InitialWorkDirRequirement,
        types::{CWLType, File, Listing},
    };
    use serde_yaml::Value;
    use std::path::Path;

    pub fn os_path(path: &str) -> String {
        if cfg!(target_os = "windows") {
            Path::new(path).to_string_lossy().replace('/', "\\")
        } else {
            path.to_string()
        }
    }

    #[test]
    fn test_resolve_filename() {
        let name = "my-tool";
        let filename = resolve_filename(name);
        assert_eq!(filename, "workflows/my-tool/my-tool.cwl".to_string());
    }

    #[test]
    pub fn test_cwl_save() {
        let inputs = vec![
            CommandInputParameter::default()
                .with_id("positional1")
                .with_default_value(DefaultValue::File(File::from_location(&"test_data/input.txt".to_string())))
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_position(0)),
            CommandInputParameter::default()
                .with_id("option1")
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_prefix(&"--option1".to_string()))
                .with_default_value(DefaultValue::Any(Value::String("value1".to_string()))),
        ];
        let mut clt = CommandLineTool::default()
            .with_base_command(Command::Multiple(vec!["python".to_string(), "test/script.py".to_string()]))
            .with_inputs(inputs)
            .with_requirements(vec![
                Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("test/script.py")),
                Requirement::DockerRequirement(DockerRequirement::from_file("test/data/Dockerfile", "test")),
            ]);

        clt.save("workflows/tool/tool.cwl");

        //check if paths are rewritten upon tool saving

        assert_eq!(
            clt.inputs[0].default,
            Some(DefaultValue::File(File::from_location(&os_path("../../test_data/input.txt"))))
        );
        let requirements = &clt.requirements.as_ref().unwrap();
        let req_0 = &requirements[0];
        let req_1 = &requirements[1];
        assert_eq!(
            *req_0,
            Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement {
                listing: vec![Listing {
                    entry: Entry::from_file(&os_path("../../test/script.py")),
                    entryname: "test/script.py".to_string()
                }]
            })
        );
        assert_eq!(
            *req_1,
            Requirement::DockerRequirement(DockerRequirement::DockerFile {
                docker_file: Entry::from_file(&os_path("../../test/data/Dockerfile")),
                docker_image_id: "test".to_string()
            })
        );
    }
}
