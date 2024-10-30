use super::{
    execution::runner::run_command,
    types::{CWLType, Directory, EnvironmentDef, File},
};
use crate::io::resolve_path;
use core::fmt;
use serde::{Deserialize, Deserializer, Serialize};
use serde_yml::{Mapping, Value};
use std::{collections::HashMap, error::Error, fmt::Display, path::Path, vec};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandLineTool {
    pub class: String,
    pub cwl_version: String,
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
}

impl Default for CommandLineTool {
    fn default() -> Self {
        Self {
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

    pub fn has_shell_command_requirement(&self) -> bool {
        if let Some(requirements) = &self.requirements {
            requirements.iter().any(|req| matches!(req, Requirement::ShellCommandRequirement))
        } else {
            false
        }
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

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandInputParameter {
    #[serde(default)]
    pub id: String,
    pub type_: CWLType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<DefaultValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_binding: Option<CommandLineBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
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

#[derive(Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum DefaultValue {
    File(File),
    Directory(Directory),
    Any(serde_yml::Value),
}

impl DefaultValue {
    pub fn as_value_string(&self) -> String {
        match self {
            DefaultValue::File(item) => item.location.clone(),
            DefaultValue::Directory(item) => item.location.clone(),
            DefaultValue::Any(value) => match value {
                serde_yml::Value::Bool(_) => String::from(""), // do not remove!
                _ => serde_yml::to_string(value).unwrap().trim_end().to_string(),
            },
        }
    }

    pub fn has_matching_type(&self, cwl_type: &CWLType) -> bool {
        matches!(
            (self, cwl_type),
            (DefaultValue::File(_), CWLType::File) | (DefaultValue::Directory(_), CWLType::Directory) | (DefaultValue::Any(_), _)
        )
    }
}

impl<'de> Deserialize<'de> for DefaultValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;

        let location = value.get("location").or_else(|| value.get("path")).and_then(Value::as_str);

        if let Some(location_str) = location {
            let secondary_files = value.get("secondaryFiles")
                .map(|v| serde_yml::from_value(v.clone()))
                .transpose()
                .map_err(serde::de::Error::custom)?;
        
            let basename = value.get("basename")
                .map(|v| serde_yml::from_value(v.clone()))
                .transpose()
                .map_err(serde::de::Error::custom)?;
        
            match value.get("class").and_then(Value::as_str) {
                Some("File") => {
                    let mut item = File::from_location(&location_str.to_string());
                    item.secondary_files = secondary_files;
                    item.basename = basename;
                    Ok(DefaultValue::File(item))
                }
                Some("Directory") => {
                    let mut item = Directory::from_location(&location_str.to_string());
                    item.secondary_files = secondary_files;
                    item.basename = basename;
                    Ok(DefaultValue::Directory(item))
                }
                _ => Ok(DefaultValue::Any(value)),
            }
        } else {
            Ok(DefaultValue::Any(value))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommandLineBinding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_from: Option<String>,
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
    #[serde(default)]
    pub id: String,
    pub type_: CWLType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_binding: Option<CommandOutputBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
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

fn deserialize_inputs<'de, D>(deserializer: D) -> Result<Vec<CommandInputParameter>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    let parameters = match value {
        Value::Sequence(seq) => seq
            .into_iter()
            .map(|item| {
                let param: CommandInputParameter = serde_yml::from_value(item).map_err(serde::de::Error::custom)?;
                Ok(param)
            })
            .collect::<Result<Vec<_>, _>>()?,
        Value::Mapping(map) => map
            .into_iter()
            .map(|(key, value)| {
                let id = key.as_str().ok_or_else(|| serde::de::Error::custom("Expected string key"))?;
                let param = match value {
                    Value::String(type_str) => {
                        let type_ = serde_yml::from_value::<CWLType>(Value::String(type_str)).map_err(serde::de::Error::custom)?;
                        CommandInputParameter::default().with_id(id).with_type(type_)
                    }
                    _ => {
                        let mut param: CommandInputParameter = serde_yml::from_value(value).map_err(serde::de::Error::custom)?;
                        param.id = id.to_string();
                        param
                    }
                };

                Ok(param)
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(serde::de::Error::custom("Expected sequence or mapping for inputs")),
    };

    Ok(parameters)
}

fn deserialize_outputs<'de, D>(deserializer: D) -> Result<Vec<CommandOutputParameter>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    let parameters = match value {
        Value::Sequence(seq) => seq
            .into_iter()
            .map(|item| {
                let param: CommandOutputParameter = serde_yml::from_value(item).map_err(serde::de::Error::custom)?;
                Ok(param)
            })
            .collect::<Result<Vec<_>, _>>()?,
        Value::Mapping(map) => map
            .into_iter()
            .map(|(key, value)| {
                let id = key.as_str().ok_or_else(|| serde::de::Error::custom("Expected string key"))?;
                let mut param: CommandOutputParameter = serde_yml::from_value(value).map_err(serde::de::Error::custom)?;
                param.id = id.to_string();
                Ok(param)
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(serde::de::Error::custom("Expected sequence or mapping for outputs")),
    };

    Ok(parameters)
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
    DockerRequirement(DockerRequirement),
    ResourceRequirement(ResourceRequirement),
    EnvVarRequirement(EnvVarRequirement),
    ShellCommandRequirement,
}

fn deserialize_requirements<'de, D>(deserializer: D) -> Result<Option<Vec<Requirement>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<Value> = Deserialize::deserialize(deserializer)?;
    if value.is_none() {
        return Ok(None);
    }

    let value = value.unwrap();

    let parameters = match value {
        Value::Sequence(seq) => seq
            .into_iter()
            .map(|item| {
                let param: Requirement = serde_yml::from_value(item).map_err(serde::de::Error::custom)?;
                Ok(param)
            })
            .collect::<Result<Vec<_>, _>>()?,
        Value::Mapping(map) => map
            .into_iter()
            .map(|(key, value)| {
                let class = key.as_str().ok_or_else(|| serde::de::Error::custom("Expected string key"))?;
                let mut modified_value = value;
                let new_map = match modified_value {
                    Value::Mapping(ref mut inner_map) => {
                        inner_map.insert(Value::String("class".to_string()), Value::String(class.to_string()));
                        inner_map.clone()
                    }
                    _ => {
                        let mut map = Mapping::new();
                        map.insert(Value::String("class".to_string()), Value::String(class.to_string()));
                        map
                    }
                };
                let param: Requirement = serde_yml::from_value(Value::Mapping(new_map)).map_err(serde::de::Error::custom)?;
                Ok(param)
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(serde::de::Error::custom("Expected sequence or mapping for outputs")),
    };

    Ok(Some(parameters))
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
                entry: Entry::from_file(filename),
            }],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DockerRequirement {
    DockerPull(String),
    #[serde(untagged)]
    DockerFile {
        #[serde(rename = "dockerFile")]
        docker_file: Entry,
        #[serde(rename = "dockerImageId")]
        docker_image_id: String,
    },
}

impl DockerRequirement {
    pub fn from_file(filename: &str, tag: &str) -> Self {
        DockerRequirement::DockerFile {
            docker_file: Entry::from_file(filename),
            docker_image_id: tag.to_string(),
        }
    }
    pub fn from_pull(image_id: &str) -> Self {
        DockerRequirement::DockerPull(image_id.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirement {
    pub cores_min: Option<i32>,
    pub cores_max: Option<i32>,
    pub ram_min: Option<i32>,
    pub ram_max: Option<i32>,
    pub tmpdir_min: Option<i32>,
    pub tmpdir_max: Option<i32>,
    pub outdir_min: Option<i32>,
    pub outdir_max: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarRequirement {
    pub env_def: EnviromentDefs,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum EnviromentDefs {
    Vec(Vec<EnvironmentDef>),
    Map(HashMap<String, String>),
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

impl Entry {
    pub fn from_file(path: &str) -> Entry {
        Entry::Include(Include { include: path.to_string() })
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Include {
    #[serde(rename = "$include")]
    pub include: String,
}
