use cwl::{
    clt::CommandLineTool,
    requirements::Requirement,
    types::{CWLType, DefaultValue, EnviromentDefs},
};
use serde_yaml::Value;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(crate) struct RuntimeEnvironment {
    pub inputs: HashMap<String, DefaultValue>,
    pub runtime: HashMap<String, String>,
    pub environment: HashMap<String, String>,
}

pub(crate) fn collect_inputs(
    tool: &CommandLineTool,
    inputs: &HashMap<String, DefaultValue>,
) -> Result<HashMap<String, DefaultValue>, Box<dyn std::error::Error>> {
    tool.inputs
        .iter()
        .map(|i| {
            if let Some(value) = inputs.get(&i.id) {
                if value.has_matching_type(&i.type_) {
                    return Ok((i.id.clone(), value.load()));
                } else {
                    Err(format!("CWLType {:?} is not matching input value: \n{:#?}", i.type_, value))?
                }
            } else if let Some(default) = &i.default {
                return Ok((i.id.clone(), default.load()));
            }

            if let CWLType::Optional(_) = i.type_ {
                return Ok((i.id.clone(), DefaultValue::Any(Value::Null)));
            }
            Err(format!("No Input provided for {:?}", i.id))?
        })
        .collect::<Result<HashMap<_, _>, Box<dyn std::error::Error>>>()
}

pub(crate) fn collect_env_vars(tool: &CommandLineTool) -> HashMap<String, String> {
    tool.requirements
        .iter()
        .chain(tool.hints.iter())
        .flatten()
        .filter_map(|r| {
            if let Requirement::EnvVarRequirement(evr) = r {
                match &evr.env_def {
                    EnviromentDefs::Vec(vec) => Some(vec.iter().map(|d| (d.env_name.clone(), d.env_value.clone())).collect::<HashMap<_, _>>()),
                    EnviromentDefs::Map(map) => Some(map.clone()),
                }
            } else {
                None
            }
        })
        .flatten()
        .collect::<HashMap<_, _>>()
}
