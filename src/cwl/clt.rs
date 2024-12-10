use super::{
    execution::runner::run_command,
    inputs::{deserialize_inputs, CommandInputParameter, CommandLineBinding},
    outputs::{deserialize_outputs, CommandOutputParameter},
    requirements::{deserialize_requirements, DockerRequirement, Requirement},
    types::{CWLType, DefaultValue, Entry},
};
use crate::io::resolve_path;
use core::fmt;
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt::Display};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandLineTool {
    pub class: String,
    pub cwl_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(default)]
    pub base_command: Command,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(deserialize_with = "deserialize_inputs")]
    pub inputs: Vec<CommandInputParameter>,
    #[serde(deserialize_with = "deserialize_outputs")]
    pub outputs: Vec<CommandOutputParameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_requirements")]
    #[serde(default)]
    pub requirements: Option<Vec<Requirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_requirements")]
    #[serde(default)]
    pub hints: Option<Vec<Requirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<Argument>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_codes: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permanent_fail_codes: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary_fail_codes: Option<Vec<i32>>,
}

impl Default for CommandLineTool {
    fn default() -> Self {
        Self {
            id: None,
            label: None,
            doc: None,
            class: String::from("CommandLineTool"),
            cwl_version: String::from("v1.2"),
            base_command: Default::default(),
            stdin: Default::default(),
            stdout: Default::default(),
            stderr: Default::default(),
            inputs: Default::default(),
            outputs: Default::default(),
            requirements: Default::default(),
            hints: Default::default(),
            arguments: Default::default(),
            success_codes: None,
            permanent_fail_codes: None,
            temporary_fail_codes: None,
        }
    }
}

impl CommandLineTool {
    pub fn with_base_command(mut self, command: Command) -> Self {
        self.base_command = command;
        self
    }
    pub fn with_inputs(mut self, inputs: Vec<CommandInputParameter>) -> Self {
        self.inputs = inputs;
        self
    }
    pub fn with_outputs(mut self, outputs: Vec<CommandOutputParameter>) -> Self {
        self.outputs = outputs;
        self
    }
    pub fn with_requirements(mut self, requirements: Vec<Requirement>) -> Self {
        self.requirements = Some(requirements);
        self
    }
}

impl Display for CommandLineTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_yml::to_string(self) {
            Ok(yaml) => write!(f, "{}", yaml),
            Err(_) => Err(fmt::Error),
        }
    }
}

impl CommandLineTool {
    pub fn execute(&self) -> Result<(), Box<dyn Error>> {
        run_command(self, None)
    }

    pub fn save(&mut self, path: &str) -> String {
        //rewire paths to new location
        for input in &mut self.inputs {
            if let Some(DefaultValue::File(value)) = &mut input.default {
                value.location = resolve_path(&value.location, path);
            }
            if let Some(DefaultValue::Directory(value)) = &mut input.default {
                value.location = resolve_path(&value.location, path);
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

    pub fn get_output_ids(&self) -> Vec<String> {
        self.outputs.iter().map(|o| o.id.clone()).collect::<Vec<_>>()
    }

    pub fn has_shell_command_requirement(&self) -> bool {
        if let Some(requirements) = &self.requirements {
            requirements.iter().any(|req| matches!(req, Requirement::ShellCommandRequirement))
        } else {
            false
        }
    }

    pub fn get_error_code(&self) -> i32 {
        if let Some(code) = &self.permanent_fail_codes {
            code[0]
        } else {
            1
        }
    }

    pub fn has_stdout_output(&self) -> bool {
        self.outputs.iter().any(|o| matches!(o.type_, CWLType::Stdout))
    }

    pub fn has_stderr_output(&self) -> bool {
        self.outputs.iter().any(|o| matches!(o.type_, CWLType::Stderr))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Argument {
    String(String),
    Binding(CommandLineBinding),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Command {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for Command {
    fn default() -> Self {
        Command::Single(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cwl::{
        requirements::InitialWorkDirRequirement,
        types::{CWLType, File, Listing},
    };
    use serde_yml::Value;
    use std::path::Path;

    pub fn os_path(path: &str) -> String {
        if cfg!(target_os = "windows") {
            Path::new(path).to_string_lossy().replace('/', "\\")
        } else {
            path.to_string()
        }
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

        assert_eq!(clt.inputs[0].default, Some(DefaultValue::File(File::from_location(&os_path("../../test_data/input.txt")))));
        let requirements = &clt.requirements.unwrap();
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
