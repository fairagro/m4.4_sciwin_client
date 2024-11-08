use std::{error::Error, fmt::Display};

pub trait ExitCode {
    fn exit_code(&self) -> i32;
}

#[derive(Debug)]
pub struct CommandError {
    pub message: String,
    pub exit_code: i32,
}

impl Error for CommandError {}

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, code: {}", self.message, self.exit_code)
    }
}

impl ExitCode for CommandError {
    fn exit_code(&self) -> i32 {
        self.exit_code
    }
}
