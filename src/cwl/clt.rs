use super::types::{CWLType, File};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, process::Command as SystemCommand, process::ExitStatus};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandLineTool {
    pub class: String,
    pub cwl_version: String,
    pub base_command: Command,
    pub inputs: Vec<CommandInputParameter>,
    pub outputs: Vec<CommandOutputParameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirements: Option<Vec<Requirement>>,
}

impl Default for CommandLineTool {
    fn default() -> Self {
        Self {
            class: String::from("CommandLineTool"),
            cwl_version: String::from("v1.2"),
            base_command: Command::Single("echo".to_string()),
            inputs: Default::default(),
            outputs: Default::default(),
            requirements: Default::default(),
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
    pub fn execute(&self) -> ExitStatus {
        let cmd = match &self.base_command {
            Command::Single(cmd) => cmd,
            Command::Multiple(vec) => &vec[0],
        };

        let mut command = SystemCommand::new(cmd);
        if let Command::Multiple(ref vec) = &self.base_command {
            command.arg(&vec[1]);
        }
        for input in &self.inputs {
            if let Some(binding) = &input.input_binding {
                if let Some(prefix) = &binding.prefix {
                    command.arg(prefix);
                }
            }
            if let Some(default_) = &input.default {
                let value = match &default_ {
                    DefaultValue::File(file) => &file.location,
                    DefaultValue::Any(value) => &serde_yml::to_string(value).unwrap(),
                };
                command.arg(value);
            }
        }

        //debug print command
        if cfg!(debug_assertions) {
            let cmd = format!(
                "{} {}",
                command.get_program().to_str().unwrap(),
                command
                    .get_args()
                    .map(|arg| arg.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            println!("❕ Executing command: {}", cmd);
        }

        let output = command.output().expect("Could not execute command!");

        //report from stdout/stderr
        println!("{}", String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            eprintln!("❌ {}", String::from_utf8_lossy(&output.stderr));
        }

        output.status
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Command {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandInputParameter {
    pub id: String,
    pub type_: CWLType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_binding: Option<CommandLineBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<DefaultValue>, //refactor to enum of file and dir
}

impl CommandInputParameter {
    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_type(mut self, t: CWLType) -> Self {
        self.type_ = t;
        self
    }

    pub fn with_default_value(mut self, f: DefaultValue) -> Self {
        self.default = Some(f);
        self
    }

    pub fn with_binding(mut self, binding: CommandLineBinding) -> Self {
        self.input_binding = Some(binding);
        self
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum DefaultValue {
    File(File),
    Any(serde_yml::Value),
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandLineBinding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
}

impl CommandLineBinding {
    pub fn with_prefix(mut self, prefix: &String) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    pub fn with_position(mut self, position: usize) -> Self {
        self.position = Some(position);
        self
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutputParameter {
    pub id: String,
    pub type_: CWLType,
    pub output_binding: Option<CommandOutputBinding>,
}

impl CommandOutputParameter {
    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }
    pub fn with_type(mut self, type_: CWLType) -> Self {
        self.type_ = type_;
        self
    }
    pub fn with_binding(mut self, binding: CommandOutputBinding) -> Self {
        self.output_binding = Some(binding);
        self
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutputBinding {
    pub glob: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "class")]
pub enum Requirement {
    InitialWorkDirRequirement(InitialWorkDirRequirement),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InitialWorkDirRequirement {
    pub listing: Vec<Listing>,
}

impl InitialWorkDirRequirement {
    pub fn from_file(filename: &str) -> Self {
        InitialWorkDirRequirement {
            listing: vec![Listing {
                entryname: filename.to_string(),
                entry: Entry::Include(Include {
                    include: filename.to_string(),
                }),
            }],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub entryname: String,
    pub entry: Entry,
}
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Entry {
    Source(String),
    Include(Include),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Include {
    #[serde(rename = "$include")]
    pub include: String,
}
