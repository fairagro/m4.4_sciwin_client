use super::input::Input;
use std::process::{Command, ExitStatus};

#[derive(Debug, PartialEq)]
pub struct Tool {
    pub base_command: Vec<String>,
    pub inputs: Vec<Input>,
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

        let debug = format!(
            "{} {}",
            self.base_command[0],
            command
                .get_args()
                .map(|arg| arg.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        );
        println!("{}", debug);

        let output = command.output().expect("Running failed");

        println!("{}", String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }

        output.status
    }
}
