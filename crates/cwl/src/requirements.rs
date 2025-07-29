use super::types::{Dirent, Entry, EnviromentDefs};
use crate::{
    types::{DefaultValue, Include},
    StringOrNumber,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_yaml::{Mapping, Value};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "class")]
pub enum Requirement {
    InitialWorkDirRequirement(InitialWorkDirRequirement),
    DockerRequirement(DockerRequirement),
    ResourceRequirement(ResourceRequirement),
    EnvVarRequirement(EnvVarRequirement),
    ShellCommandRequirement,
    ToolTimeLimit(ToolTimeLimit),
    NetworkAccess(NetworkAccess),
    InlineJavascriptRequirement(InlineJavascriptRequirement),
    SubworkflowFeatureRequirement,
    //as dummys, not used at this point
    SoftwareRequirement,
    SchemaDefRequirement,
    ScatterFeatureRequirement,
    MultipleInputFeatureRequirement,
    StepInputExpressionRequirement,
    LoadListingRequirement,
    InplaceUpdateRequirement,
    WorkReuse,
}

pub trait FromRequirement<T> {
    fn get(req: &Requirement) -> Option<&T>;
    fn get_mut(req: &mut Requirement) -> Option<&mut T>;
}

macro_rules! impl_from_requirement {
    ($i:ident, $t:ty) => {
        impl FromRequirement<$t> for Requirement {
            fn get(req: &Requirement) -> Option<&$t> {
                if let Requirement::$i(v) = req {
                    Some(v)
                } else {
                    None
                }
            }

            fn get_mut(req: &mut Requirement) -> Option<&mut $t> {
                if let Requirement::$i(v) = req {
                    Some(v)
                } else {
                    None
                }
            }
        }
    };
}

impl_from_requirement!(ToolTimeLimit, ToolTimeLimit);
impl_from_requirement!(NetworkAccess, NetworkAccess);
impl_from_requirement!(DockerRequirement, DockerRequirement);
impl_from_requirement!(EnvVarRequirement, EnvVarRequirement);
impl_from_requirement!(ResourceRequirement, ResourceRequirement);
impl_from_requirement!(InitialWorkDirRequirement, InitialWorkDirRequirement);
impl_from_requirement!(InlineJavascriptRequirement, InlineJavascriptRequirement);

pub fn deserialize_requirements<'de, D>(deserializer: D) -> Result<Vec<Requirement>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_requirements_or_hints(deserializer, true)
}

pub fn deserialize_hints<'de, D>(deserializer: D) -> Result<Vec<Requirement>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_requirements_or_hints(deserializer, false)
}

fn deserialize_requirements_or_hints<'de, D>(deserializer: D, strict: bool) -> Result<Vec<Requirement>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<Value> = Deserialize::deserialize(deserializer)?;
    if value.is_none() {
        return Ok(vec![]);
    }

    let value = value.unwrap();

    let mut requirements = vec![];

    let decide_about_item = |item: Value| -> Result<Option<Requirement>, D::Error> {
        match serde_yaml::from_value(item) {
            Ok(req) => Ok(Some(req)),
            Err(e) if strict => Err(serde::de::Error::custom(e)),
            Err(_) => Ok(None),
        }
    };

    match value {
        Value::Sequence(seq) => {
            for item in seq {
                if let Some(req) = decide_about_item(item)? {
                    requirements.push(req);
                }
            }
        }
        Value::Mapping(map) => {
            for (key, value) in map {
                let class = key.as_str().ok_or_else(|| serde::de::Error::custom("Expected string key"))?;
                let mut modified_value = value;
                let new_map = if let Value::Mapping(ref mut inner_map) = modified_value {
                    inner_map.insert(Value::String("class".to_string()), Value::String(class.to_string()));
                    inner_map.clone()
                } else {
                    let mut map = Mapping::new();
                    map.insert(Value::String("class".to_string()), Value::String(class.to_string()));
                    map
                };
                if let Some(req) = decide_about_item(Value::Mapping(new_map))? {
                    requirements.push(req);
                }
            }
        }
        _ => return Err(serde::de::Error::custom("Expected sequence or mapping for requirements")),
    };

    Ok(requirements)
}

fn get_entry_name(input: &str) -> String {
    let i = input
        .trim_start_matches(|c: char| !c.is_alphabetic())
        .to_string()
        .replace(['.', '/'], "_");
    format!("$(inputs.{})", i.to_lowercase()).to_string()
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum WorkDirItem {
    Dirent(Dirent),
    Expression(String),
    FileOrDirectory(Box<DefaultValue>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InitialWorkDirRequirement {
    pub listing: Vec<WorkDirItem>,
}

impl InitialWorkDirRequirement {
    pub fn from_file(filename: &str) -> Self {
        InitialWorkDirRequirement {
            listing: vec![WorkDirItem::Dirent(Dirent {
                entryname: Some(filename.to_string()),
                entry: Entry::from_file(filename),
                ..Default::default()
            })],
        }
    }
    pub fn from_files(filenames: &[&str]) -> Self {
        InitialWorkDirRequirement {
            listing: filenames
                .iter()
                .map(|&filename| {
                    WorkDirItem::Dirent(Dirent {
                        entryname: Some(filename.to_string()),
                        entry: Entry::Source(get_entry_name(filename)),
                        ..Default::default()
                    })
                })
                .collect(),
        }
    }
    pub fn from_contents(entryname: &str, contents: &str) -> Self {
        InitialWorkDirRequirement {
            listing: vec![WorkDirItem::Dirent(Dirent {
                entryname: Some(entryname.to_string()),
                entry: Entry::Source(contents.to_string()),
                ..Default::default()
            })],
        }
    }

    pub fn add_files(&mut self, filenames: &[&str]) {
        self.listing.extend(filenames.iter().map(|&f| {
            WorkDirItem::Dirent(Dirent {
                entryname: Some(f.to_string()),
                entry: Entry::Source(get_entry_name(f)),
                ..Default::default()
            })
        }));
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DockerRequirement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_pull: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_file: Option<Entry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_image_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_output_directory: Option<String>,
}

impl DockerRequirement {
    pub fn from_file(filename: &str, tag: &str) -> Self {
        DockerRequirement {
            docker_file: Some(Entry::from_file(filename)),
            docker_image_id: Some(tag.to_string()),
            ..Default::default()
        }
    }
    pub fn from_pull(image_id: &str) -> Self {
        DockerRequirement {
            docker_pull: Some(image_id.to_string()),
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores_min: Option<StringOrNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores_max: Option<StringOrNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram_min: Option<StringOrNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram_max: Option<StringOrNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmpdir_min: Option<StringOrNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmpdir_max: Option<StringOrNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outdir_min: Option<StringOrNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outdir_max: Option<StringOrNumber>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarRequirement {
    pub env_def: EnviromentDefs,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum StringOrInclude {
    String(String),
    Include(Include),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct InlineJavascriptRequirement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression_lib: Option<Vec<StringOrInclude>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ToolTimeLimit {
    pub timelimit: u64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAccess {
    pub network_access: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_initial_workdir_requirement() {
        let req = InitialWorkDirRequirement::from_file("../../tests/test_data/echo.py");
        assert_eq!(req.listing.len(), 1);
        assert!(matches!(req.listing[0], WorkDirItem::Dirent(_)));
        if let WorkDirItem::Dirent(dirent) = &req.listing[0] {
            assert_eq!(dirent.entryname, Some("../../tests/test_data/echo.py".to_string()));
        }
    }

    #[test]
    pub fn test_initial_workdir_requirement_multiple() {
        let req = InitialWorkDirRequirement::from_files(&["../../tests/test_data/file.txt", "../../tests/test_data/input_alt.txt"]);
        assert_eq!(req.listing.len(), 2);
    }
}
