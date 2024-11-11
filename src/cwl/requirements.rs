use super::types::{Entry, EnviromentDefs, Listing};
use serde::{Deserialize, Deserializer, Serialize};
use serde_yml::{Mapping, Value};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "class")]
pub enum Requirement {
    InitialWorkDirRequirement(InitialWorkDirRequirement),
    DockerRequirement(DockerRequirement),
    ResourceRequirement(ResourceRequirement),
    EnvVarRequirement(EnvVarRequirement),
    ShellCommandRequirement,
}

pub fn deserialize_requirements<'de, D>(deserializer: D) -> Result<Option<Vec<Requirement>>, D::Error>
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
    pub fn from_contents(entryname: &str, contents: &str) -> Self {
        InitialWorkDirRequirement {
            listing: vec![Listing {
                entryname: entryname.to_string(),
                entry: Entry::Source(contents.to_string()),
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
