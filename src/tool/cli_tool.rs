use super::input::Input;
use crate::cwl::{
    clt::{
        Command as CWLCommand, CommandInputParameter, CommandLineBinding, CommandLineTool,
        CommandOutputBinding, CommandOutputParameter, InitialWorkDirRequirement, Requirement,
    },
    types::{CWLType, File},
};
use crate::util::get_filename_without_extension;
use std::process::{Command, ExitStatus};

#[derive(Debug, PartialEq)]
pub struct Tool {
    pub base_command: Vec<String>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<String>,
}

impl Tool {
    pub fn execute(&self) -> ExitStatus {
        let mut command = Command::new(&self.base_command[0]);
        if self.base_command.len() > 1 {
            command.arg(&self.base_command[1]);
        }
        for input in &self.inputs {
            if let Some(prefix) = &input.prefix {
                command.arg(prefix);
            }
            if let Some(value) = &input.value {
                command.arg(value);
            }
        }

        //debug print command
        if cfg!(debug_assertions) {
            let cmd = format!(
                "{} {}",
                self.base_command[0],
                command
                    .get_args()
                    .map(|arg| arg.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            println!("❕ Executing command: {}", cmd);
        }

        let output = command.output().expect("Running failed");

        println!("{}", String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            eprintln!("❌ {}", String::from_utf8_lossy(&output.stderr));
        }

        output.status
    }
}

impl Tool {
    pub fn to_cwl(&self) -> CommandLineTool {
        let mut tool = CommandLineTool::default()
            .base_command(CWLCommand::Multiple(self.base_command.to_owned()))
            .inputs(
                self.inputs
                    .iter()
                    .map(|i| {
                        let mut input = CommandInputParameter::new(&i.id).set_type(CWLType::File); //build checks for that!
                        if let Some(value) = &i.value {
                            input = input.set_default(File::new(value));
                        }
                        if let Some(prefix) = &i.prefix {
                            input = input.set_binding(CommandLineBinding::with_prefix(prefix))
                        }
                        if let Some(position) = i.index {
                            input = input.set_binding(CommandLineBinding::with_position(position))
                        }
                        input
                    })
                    .collect(),
            )
            .outputs(
                self.outputs
                    .iter()
                    .map(|o| CommandOutputParameter {
                        id: get_filename_without_extension(o).unwrap_or(o.to_string()),
                        type_: CWLType::File,
                        output_binding: Some(CommandOutputBinding {
                            glob: o.to_string(),
                        }),
                    })
                    .collect(),
            );
        if self.base_command.len() > 1 {
            tool = tool.requirements(vec![Requirement::InitialWorkDirRequirement(
                InitialWorkDirRequirement::from_file(self.base_command[1].as_str()),
            )])
        }
        tool
    }
}
