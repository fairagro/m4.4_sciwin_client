use super::{
    inputs::{CommandInputParameter, CommandLineBinding},
    outputs::{deserialize_outputs, CommandOutputParameter},
    requirements::Requirement,
    types::CWLType,
};
use crate::{requirements::DockerRequirement, DocumentBase};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

/// Represents a CWL CommandLineTool, a process characterized by the execution of a standalone,
/// non-interactive program which is invoked on some input, produces output, and then terminates.
///
/// Reference: [CWL CommandLineTool Specification](https://www.commonwl.org/v1.2/CommandLineTool.html)
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
                id: None,
                label: None,
                doc: None,
                class: String::from("CommandLineTool"),
                cwl_version: String::from("v1.2"),
                inputs: Default::default(),
                requirements: Default::default(),
                hints: Default::default(),
            },
            base_command: Default::default(),
            stdin: Default::default(),
            stdout: Default::default(),
            stderr: Default::default(),
            outputs: Default::default(),
            arguments: Default::default(),
            success_codes: None,
            permanent_fail_codes: None,
            temporary_fail_codes: None,
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
    pub fn with_hints(mut self, requirements: Vec<Requirement>) -> Self {
        self.hints = Some(requirements);
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
        match serde_yaml::to_string(self) {
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

    pub fn get_docker_requirement(&self) -> Option<DockerRequirement> {
        self.requirements.as_ref()?.iter().find_map(|req| {
            if let Requirement::DockerRequirement(dr) = req {
                Some(dr.clone()) // Return the found DockerRequirement
            } else {
                None
            }
        })
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
        println!("{clt:?}");
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
        let result = serde_yaml::to_string(&tool);
        println!("{result:?}");
        assert!(result.is_ok());
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
            output_binding: None,
            format: None,
        }]);
        assert!(tool.has_stdout_output());
    }

    #[test]
    pub fn test_has_stderr() {
        let tool = CommandLineTool::default().with_outputs(vec![CommandOutputParameter {
            id: "stderr".to_string(),
            type_: CWLType::Stderr,
            output_binding: None,
            format: None,
        }]);
        assert!(tool.has_stderr_output());
    }
}
