pub mod environment;
pub mod io;
pub mod runner;
pub mod staging;
pub mod util;
pub mod validate;

use std::{error::Error, fmt::Display};
use std::{num::NonZero, process::Command, thread};
use sysinfo::System;

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

pub(crate) fn get_processor_count() -> usize {
    thread::available_parallelism().map(NonZero::get).unwrap_or(1)
}

pub(crate) fn get_available_ram() -> u64 {
    let mut system = System::new_all();
    system.refresh_all();
    system.free_memory() / 1024
}

pub fn format_command(command: &Command) -> String {
    let program = command.get_program().to_string_lossy();

    let args: Vec<String> = command
        .get_args()
        .map(|arg| {
            let arg_str = arg.to_string_lossy();
            arg_str.to_string()
        })
        .collect();

    format!("{} {}", program, args.join(" "))
}
