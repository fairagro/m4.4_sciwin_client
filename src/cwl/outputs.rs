use super::{deserialize::Identifiable, types::CWLType};
use serde::{Deserialize, Serialize};

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

impl Identifiable for CommandOutputParameter {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_id(&mut self, id: String) {
        self.id = id
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutputBinding {
    pub glob: String,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowOutputParameter {
    #[serde(default)]
    pub id: String,
    pub type_: CWLType,
    pub output_source: String,
}

impl WorkflowOutputParameter {
    pub fn with_id(&mut self, id: &str) -> &Self {
        self.id = id.to_string();
        self
    }
}

impl Identifiable for WorkflowOutputParameter {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_id(&mut self, id: String) {
        self.id = id
    }
}
