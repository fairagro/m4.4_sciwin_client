use super::types::{CWLType, File};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommandLineTool {
    pub class: String,
    pub cwl_version: String,
    pub base_command: Option<Command>,
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
            base_command: Default::default(),
            inputs: Default::default(),
            outputs: Default::default(),
            requirements: Default::default(),
        }
    }
}

impl CommandLineTool {
    pub fn base_command(mut self, command: Command) -> Self {
        self.base_command = Some(command);
        self
    }
    pub fn inputs(mut self, inputs: Vec<CommandInputParameter>) -> Self {
        self.inputs = inputs;
        self
    }
    pub fn outputs(mut self, outputs: Vec<CommandOutputParameter>) -> Self {
        self.outputs = outputs;
        self
    }
    pub fn requirements(mut self, requirements: Vec<Requirement>) -> Self {
        self.requirements = Some(requirements);
        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Command {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommandInputParameter {
    pub id: String,
    pub type_: CWLType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_binding: Option<CommandLineBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<File>, //refactor to enum of file and dir
}

impl Default for CommandInputParameter {
    fn default() -> Self {
        Self {
            id: Default::default(),
            type_: CWLType::Null,
            input_binding: Default::default(),
            default: Default::default(),
        }
    }
}

impl CommandInputParameter {
    pub fn new(id: &str) -> Self {
        CommandInputParameter {
            id: id.to_string(),
            ..Default::default()
        }
    }

    pub fn set_type(mut self, t: CWLType) -> Self {
        self.type_ = t;
        self
    }

    pub fn set_default(mut self, f: File) -> Self {
        self.default = Some(f);
        self
    }

    pub fn set_binding(mut self, binding: CommandLineBinding) -> Self {
        self.input_binding = Some(binding);
        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommandLineBinding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
}

impl CommandLineBinding {
    pub fn with_prefix(prefix: &String) -> Self {
        CommandLineBinding {
            prefix: Some(prefix.to_string()),
            position: None,
        }
    }

    pub fn with_position(position: usize) -> Self {
        CommandLineBinding {
            prefix: None,
            position: Some(position),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutputParameter {
    pub id: String,
    pub type_: CWLType,
    pub output_binding: Option<CommandOutputBinding>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutputBinding {
    pub glob: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "class")]
pub enum Requirement {
    InitialWorkDirRequirement(InitialWorkDirRequirement),
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub entryname: String,
    pub entry: Entry,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Entry {
    Source(String),
    Include(Include),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Include {
    #[serde(rename = "$include")]
    pub include: String,
}
