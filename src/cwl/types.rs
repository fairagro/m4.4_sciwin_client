use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_yml::Value;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CWLType {
    #[default]
    Null,
    Boolean,
    Int,
    Long,
    Float,
    Double,
    String,
    #[serde(rename = "File")]
    File,
    #[serde(rename = "Directory")]
    Directory,
    #[serde(rename = "Any")]
    Any,
    Stdout,
    Stderr,
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
            let secondary_files = value
                .get("secondaryFiles")
                .map(|v| serde_yml::from_value(v.clone()))
                .transpose()
                .map_err(serde::de::Error::custom)?;

            let basename = value
                .get("basename")
                .map(|v| serde_yml::from_value(v.clone()))
                .transpose()
                .map_err(serde::de::Error::custom)?;

            match value.get("class").and_then(Value::as_str) {
                Some("File") => {
                    let format = value
                        .get("format")
                        .map(|v| serde_yml::from_value(v.clone()))
                        .transpose()
                        .map_err(serde::de::Error::custom)?;
                    let mut item = File::from_location(&location_str.to_string());
                    item.secondary_files = secondary_files;
                    item.basename = basename;
                    item.format = format;
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

pub trait PathItem {
    fn location(&self) -> &String;
    fn set_location(&mut self, new_location: String);
    fn secondary_files_mut(&mut self) -> Option<&mut Vec<DefaultValue>>;
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub class: String,
    #[serde(alias = "path")]
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_files: Option<Vec<DefaultValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

impl File {
    pub fn from_location(location: &String) -> Self {
        File {
            class: String::from("File"),
            location: location.to_string(),
            secondary_files: None,
            basename: None,
            format: None,
        }
    }
}

impl PathItem for File {
    fn set_location(&mut self, new_location: String) {
        self.location = new_location;
    }

    fn secondary_files_mut(&mut self) -> Option<&mut Vec<DefaultValue>> {
        self.secondary_files.as_mut()
    }

    fn location(&self) -> &String {
        &self.location
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub class: String,
    #[serde(alias = "path")]
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_files: Option<Vec<DefaultValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basename: Option<String>,
}

impl Directory {
    pub fn from_location(location: &String) -> Self {
        Directory {
            class: String::from("Directory"),
            location: location.to_string(),
            secondary_files: None,
            basename: None,
        }
    }
}

impl PathItem for Directory {
    fn set_location(&mut self, new_location: String) {
        self.location = new_location;
    }

    fn secondary_files_mut(&mut self) -> Option<&mut Vec<DefaultValue>> {
        self.secondary_files.as_mut()
    }

    fn location(&self) -> &String {
        &self.location
    }
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentDef {
    pub env_name: String,
    pub env_value: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct OutputFile {
    pub location: String,
    pub basename: String,
    pub class: String,
    pub checksum: String,
    pub size: u64,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct OutputDirectory {
    pub location: String,
    pub basename: String,
    pub class: String,
    pub listing: Vec<OutputItem>,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum OutputItem {
    OutputFile(OutputFile),
    OutputDirectory(OutputDirectory),
}

impl OutputItem {
    pub fn to_default_value(&self) -> DefaultValue {
        match self {
            OutputItem::OutputFile(output_file) => DefaultValue::File(File::from_location(&output_file.path)),
            OutputItem::OutputDirectory(output_directory) => DefaultValue::Directory(Directory::from_location(&output_directory.path)),
        }
    }
}
