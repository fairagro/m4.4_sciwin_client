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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub class: String,
    pub location: String,
}

impl File {
    pub fn from_location(location: &String) -> Self {
        File {
            class: String::from("File"),
            location: location.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub class: String,
    pub location: String,
}

impl Directory {
    pub fn from_location(location: &String) -> Self {
        Directory {
            class: String::from("Directory"),
            location: location.to_string(),
        }
    }
}
