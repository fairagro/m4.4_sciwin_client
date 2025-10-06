//! Contains the structure of CWL `CommandLineTool` definitions together with helpful functions for convenience

use super::{
    inputs::{CommandInputParameter, CommandLineBinding},
    outputs::{CommandOutputParameter, deserialize_outputs},
    requirements::Requirement,
    types::CWLType,
};
use crate::{
    DocumentBase, Operation,
    requirements::{DockerRequirement, FromRequirement},
};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

/// A Command Line Tool is a non-interactive executable program that reads some input, performs a computation,
/// and terminates after producing some output. Command line programs are a flexible unit of code sharing and reuse,
/// unfortunately the syntax and input/output semantics among command line programs is extremely heterogeneous.
/// A common layer for describing the syntax and semantics of programs can reduce this incidental
/// complexity by providing a consistent way to connect programs together. This specification defines the
/// Common Workflow Language (CWL) Command Line Tool Description, a vendor-neutral standard for describing the syntax and
/// input/output semantics of command line programs.
///
/// Reference: <https://www.commonwl.org/v1.2/CommandLineTool.html>
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommandLineTool {
    #[serde(flatten)]
    pub base: DocumentBase,
    #[serde(default)]
    pub base_command: Command,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(deserialize_with = "deserialize_outputs")]
    pub outputs: Vec<CommandOutputParameter>,
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
            base: DocumentBase {
                id: Option::default(),
                label: Option::default(),
                doc: Option::default(),
                class: String::from("CommandLineTool"),
                cwl_version: Some(String::from("v1.2")),
                inputs: Vec::default(),
                requirements: Vec::default(),
                hints: Vec::default(),
                intent: Option::default(),
            },
            base_command: Default::default(),
            stdin: Option::default(),
            stdout: Option::default(),
            stderr: Option::default(),
            outputs: Vec::default(),
            arguments: Option::default(),
            success_codes: Option::default(),
            permanent_fail_codes: Option::default(),
            temporary_fail_codes: Option::default(),
        }
    }
}

impl Deref for CommandLineTool {
    type Target = DocumentBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for CommandLineTool {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Operation for CommandLineTool {
    fn outputs_mut(&mut self) -> &mut Vec<CommandOutputParameter> {
        &mut self.outputs
    }

    fn outputs(&self) -> &Vec<CommandOutputParameter> {
        &self.outputs
    }
}

impl Display for CommandLineTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_yaml::to_string(self) {
            Ok(yaml) => write!(f, "{yaml}"),
            Err(_) => Err(fmt::Error),
        }
    }
}

impl CommandLineTool {
    /// Adds a base command to this `CommandLineTool` and returns the updated tool
    pub fn with_base_command(mut self, command: Command) -> Self {
        self.base_command = command;
        self
    }

    /// Adds arguments to this `CommandLineTool` and returns the updated tool
    pub fn with_arguments(mut self, args: Option<Vec<Argument>>) -> Self {
        self.arguments = args;
        self
    }

    /// Adds inputs to this `CommandLineTool` and returns the updated tool
    pub fn with_inputs(mut self, inputs: Vec<CommandInputParameter>) -> Self {
        self.inputs = inputs;
        self
    }

    /// Adds outputs to this `CommandLineTool` and returns the updated tool
    pub fn with_outputs(mut self, outputs: Vec<CommandOutputParameter>) -> Self {
        self.outputs = outputs;
        self
    }

    /// Adds requirements to this `CommandLineTool` and returns the updated tool
    pub fn with_requirements(mut self, requirements: Vec<Requirement>) -> Self {
        self.requirements = requirements;
        self
    }

    pub fn append_requirement(mut self, requirement: Requirement) -> Self {
        self.requirements.push(requirement);
        self
    }

    /// Adds hints to this `CommandLineTool` and returns the updated tool
    pub fn with_hints(mut self, requirements: Vec<Requirement>) -> Self {
        self.hints = requirements;
        self
    }

    /// Adds stdout to this `CommandLineTool` and returns the updated tool
    pub fn with_stdout(mut self, stdout: Option<String>) -> Self {
        self.stdout = stdout;
        self
    }

    /// Adds stderr to this `CommandLineTool` and returns the updated tool
    pub fn with_stderr(mut self, stderr: Option<String>) -> Self {
        self.stderr = stderr;
        self
    }

    /// Returns the List of CommandOutputParameter.id of the `CommandLineTool`
    pub fn get_output_ids(&self) -> Vec<String> {
        self.outputs.iter().map(|o| o.id.clone()).collect::<Vec<_>>()
    }

    /// Checks whether the `CommandLineTool` has a `ShellCommandRequirement` in requirements
    pub fn has_shell_command_requirement(&self) -> bool {
        self.requirements.iter().any(|req| matches!(req, Requirement::ShellCommandRequirement))
    }

    /// Checks whether the `CommandLineTool` has a `DockerRequirement` in requirements and returns an Option to it
    /// do not change calls to the generic get_requirement
    pub fn get_docker_requirement(&self) -> Option<&DockerRequirement> {
        self.requirements.iter().find_map(|req| Requirement::get(req))
    }

    /// Gets the permanent fail code of the `CommandLineTool`
    pub fn get_error_code(&self) -> i32 {
        if let Some(code) = &self.permanent_fail_codes { code[0] } else { 1 }
    }

    /// Gets the success code of the `CommandLineTool`
    pub fn get_sucess_code(&self) -> i32 {
        if let Some(code) = &self.success_codes { code[0] } else { 0 }
    }

    /// Checks whether the `CommandLineTool` has an Output of `CWLType` Stdout
    pub fn has_stdout_output(&self) -> bool {
        self.outputs.iter().any(|o| matches!(o.type_, CWLType::Stdout))
    }

    /// Checks whether the `CommandLineTool` has an Output of `CWLType` Stderr
    pub fn has_stderr_output(&self) -> bool {
        self.outputs.iter().any(|o| matches!(o.type_, CWLType::Stderr))
    }
}

/// Command line bindings which are not directly associated with input parameters.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Argument {
    String(String),
    Binding(CommandLineBinding),
}

/// Command line base command which can be a single string or a list of strings.
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

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Single(cmd) => write!(f, "{}", cmd),
            Command::Multiple(cmds) => write!(f, "{}", cmds.join(" ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_simple_cwl() {
        let cwl = r"#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

requirements:
- class: InitialWorkDirRequirement
  listing:
  - entryname: calculation.py
    entry:
      $include: calculation.py

inputs:
- id: population
  type: File
  default:
    class: File
    location: ../../assays/population/dataset/population.csv
  inputBinding:
    prefix: -p
- id: speakers
  type: File
  default:
    class: File
    location: ../../assays/speakers/dataset/speakers_revised.csv
  inputBinding:
    prefix: -s

outputs:
- id: output
  type: File
  outputBinding:
    glob: results.csv

baseCommand:
- python
- calculation.py
";
        let clt: Result<CommandLineTool, serde_yaml::Error> = serde_yaml::from_str(cwl);
        assert!(clt.is_ok());
    }

    #[test]
    pub fn create_simple_cwl() {
        let tool = CommandLineTool::default()
            .with_base_command(Command::Single("ls".to_string()))
            .with_inputs(vec![CommandInputParameter {
                id: "la".to_string(),
                type_: CWLType::Boolean,
                input_binding: Some(CommandLineBinding {
                    prefix: Some("-la".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }])
            .with_outputs(vec![]);
        assert!(serde_yaml::to_string(&tool).is_ok());
    }

    #[test]
    pub fn test_get_error_code() {
        let mut tool = CommandLineTool::default();
        assert_eq!(tool.get_error_code(), 1);
        tool.permanent_fail_codes = Some(vec![42]);
        assert_eq!(tool.get_error_code(), 42);
    }

    #[test]
    pub fn test_has_stdout() {
        let tool = CommandLineTool::default().with_outputs(vec![CommandOutputParameter {
            id: "stdout".to_string(),
            type_: CWLType::Stdout,
            ..Default::default()
        }]);
        assert!(tool.has_stdout_output());
    }

    #[test]
    pub fn test_has_stderr() {
        let tool = CommandLineTool::default().with_outputs(vec![CommandOutputParameter {
            id: "stderr".to_string(),
            type_: CWLType::Stderr,
            ..Default::default()
        }]);
        assert!(tool.has_stderr_output());
    }
}
