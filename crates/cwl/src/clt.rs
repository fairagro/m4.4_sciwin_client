use super::{
    inputs::{deserialize_inputs, CommandInputParameter, CommandLineBinding},
    outputs::{deserialize_outputs, CommandOutputParameter},
    requirements::{deserialize_requirements, Requirement},
    types::CWLType,
};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandLineTool {
    pub class: String,
    pub cwl_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(default)]
    pub base_command: Command,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(deserialize_with = "deserialize_inputs")]
    pub inputs: Vec<CommandInputParameter>,
    #[serde(deserialize_with = "deserialize_outputs")]
    pub outputs: Vec<CommandOutputParameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_requirements")]
    #[serde(default)]
    pub requirements: Option<Vec<Requirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_requirements")]
    #[serde(default)]
    pub hints: Option<Vec<Requirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<Argument>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_codes: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permanent_fail_codes: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary_fail_codes: Option<Vec<i32>>,
}

impl Default for CommandLineTool {
    fn default() -> Self {
        Self {
            id: None,
            label: None,
            doc: None,
            class: String::from("CommandLineTool"),
            cwl_version: String::from("v1.2"),
            base_command: Default::default(),
            stdin: Default::default(),
            stdout: Default::default(),
            stderr: Default::default(),
            inputs: Default::default(),
            outputs: Default::default(),
            requirements: Default::default(),
            hints: Default::default(),
            arguments: Default::default(),
            success_codes: None,
            permanent_fail_codes: None,
            temporary_fail_codes: None,
        }
    }
}

impl CommandLineTool {
    pub fn with_base_command(mut self, command: Command) -> Self {
        self.base_command = command;
        self
    }
    pub fn with_inputs(mut self, inputs: Vec<CommandInputParameter>) -> Self {
        self.inputs = inputs;
        self
    }
    pub fn with_outputs(mut self, outputs: Vec<CommandOutputParameter>) -> Self {
        self.outputs = outputs;
        self
    }
    pub fn with_requirements(mut self, requirements: Vec<Requirement>) -> Self {
        self.requirements = Some(requirements);
        self
    }
    pub fn with_stdout(mut self, stdout: Option<String>) -> Self {
        self.stdout = stdout;
        self
    }
    pub fn with_stderr(mut self, stderr: Option<String>) -> Self {
        self.stderr = stderr;
        self
    }
    pub fn with_arguments(mut self, args: Option<Vec<Argument>>) -> Self {
        self.arguments = args;
        self
    }
}

impl Display for CommandLineTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_yml::to_string(self) {
            Ok(yaml) => write!(f, "{}", yaml),
            Err(_) => Err(fmt::Error),
        }
    }
}

impl CommandLineTool {
    pub fn get_output_ids(&self) -> Vec<String> {
        self.outputs.iter().map(|o| o.id.clone()).collect::<Vec<_>>()
    }

    pub fn has_shell_command_requirement(&self) -> bool {
        if let Some(requirements) = &self.requirements {
            requirements.iter().any(|req| matches!(req, Requirement::ShellCommandRequirement))
        } else {
            false
        }
    }

    pub fn get_error_code(&self) -> i32 {
        if let Some(code) = &self.permanent_fail_codes {
            code[0]
        } else {
            1
        }
    }

    pub fn has_stdout_output(&self) -> bool {
        self.outputs.iter().any(|o| matches!(o.type_, CWLType::Stdout))
    }

    pub fn has_stderr_output(&self) -> bool {
        self.outputs.iter().any(|o| matches!(o.type_, CWLType::Stderr))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Argument {
    String(String),
    Binding(CommandLineBinding),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Command {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for Command {
    fn default() -> Self {
        Command::Single(String::new())
    }
}