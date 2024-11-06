use super::clt::DefaultValue;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
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
}

impl File {
    pub fn from_location(location: &String) -> Self {
        File {
            class: String::from("File"),
            location: location.to_string(),
            secondary_files: None,
            basename: None,
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
    pub listing: Vec<OutputFile>,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum OutputItem {
    OutputFile(OutputFile),
    OutputDirectory(OutputDirectory),
}
