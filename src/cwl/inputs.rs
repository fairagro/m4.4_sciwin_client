use super::{
    deserialize::Identifiable,
    types::{CWLType, DefaultValue},
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_yml::Value;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
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

impl Identifiable for CommandInputParameter {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_id(&mut self, id: String) {
        self.id = id
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommandLineBinding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<isize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_quote: Option<bool>,
}

impl CommandLineBinding {
    pub fn with_prefix(mut self, prefix: &String) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    pub fn with_position(mut self, position: isize) -> Self {
        self.position = Some(position);
        self
    }
}

pub fn deserialize_inputs<'de, D>(deserializer: D) -> Result<Vec<CommandInputParameter>, D::Error>
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum WorkflowStepInput {
    String(String),
    Parameter(WorkflowStepInutParameter),
}

impl Default for WorkflowStepInput {
    fn default() -> Self {
        WorkflowStepInput::String(String::default())
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStepInutParameter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<DefaultValue>,
}
