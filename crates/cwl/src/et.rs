use super::outputs::{deserialize_outputs, CommandOutputParameter};
use crate::DocumentBase;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut, Range};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionTool {
    #[serde(flatten)]
    base: DocumentBase,
    #[serde(deserialize_with = "deserialize_outputs")]
    pub outputs: Vec<CommandOutputParameter>,
    pub expression: String,
}

impl Default for ExpressionTool {
    fn default() -> Self {
        Self {
            base: DocumentBase {
                cwl_version: Default::default(),
                class: String::from("ExpressionTool"),
                id: Default::default(),
                label: Default::default(),
                doc: Default::default(),
                requirements: Default::default(),
                hints: Default::default(),
                inputs: Default::default(),
            },
            outputs: Default::default(),
            expression: Default::default(),
        }
    }
}

impl Deref for ExpressionTool {
    type Target = DocumentBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for ExpressionTool {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[derive(Debug)]
pub enum ExpressionType {
    Paren,
    Bracket,
}

#[derive(Debug)]
pub struct Expression {
    pub type_: ExpressionType,
    pub expression: String,
    pub indices: Range<usize>,
}

impl Expression {
    pub fn expression(&self) -> String {
        match self.type_ {
            ExpressionType::Paren => self.expression.clone(),
            ExpressionType::Bracket => format!("(() => {{{}}})();", self.expression),
        }
    }
}
