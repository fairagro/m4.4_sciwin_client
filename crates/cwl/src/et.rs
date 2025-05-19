use super::outputs::{deserialize_outputs, CommandOutputParameter};
use crate::DocumentBase;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut, Range};

/// An `ExpressionTool` is a type of Process object that can be run by itself or as a Workflow step. 
/// It executes a pure Javascript expression that has access to the same input parameters as a workflow. 
/// It is meant to be used sparingly as a way to isolate complex Javascript expressions that need to operate on input data and produce some result; 
/// perhaps just a rearrangement of the inputs. No Docker software container is required or allowed.
/// Reference: <https://www.commonwl.org/v1.2/Workflow.html#ExpressionTool>
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionTool {
    #[serde(flatten)]
    pub base: DocumentBase,
    #[serde(deserialize_with = "deserialize_outputs")]
    pub outputs: Vec<CommandOutputParameter>,
    pub expression: String,
}

impl Default for ExpressionTool {
    fn default() -> Self {
        Self {
            base: DocumentBase {
                cwl_version: Some(String::from("v1.2")),
                class: String::from("ExpressionTool"),
                id: Option::default(),
                label: Option::default(),
                doc: Option::default(),
                requirements: Vec::default(),
                hints: Vec::default(),
                inputs: Vec::default(),
                intent: Option::default(),
            },
            outputs: Vec::default(),
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
