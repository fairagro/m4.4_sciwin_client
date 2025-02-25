use cwl::{
    clt::CommandLineTool,
    types::{CWLType, DefaultValue},
};
use serde_yaml::Value;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(crate) struct RuntimeEnvironment {
    pub inputs: HashMap<String, DefaultValue>,
    pub runtime: HashMap<String, String>,
    pub environment: HashMap<String, String>,
}