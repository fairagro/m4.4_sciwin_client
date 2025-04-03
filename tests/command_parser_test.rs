mod common;
use common::with_temp_repository;

use cwl_execution::runner::run_command;
use s4n::parser::parse_command_line;
use serial_test::serial;
use std::{path::Path, vec};


#[test]
#[serial]
pub fn test_cwl_execute_command_multiple() {
    with_temp_repository(|dir| {
        let command = "python scripts/echo.py --test data/input.txt";
        let args = shlex::split(command).expect("parsing failed");
        let cwl = parse_command_line(args.iter().map(AsRef::as_ref).collect());
        assert!(run_command(&cwl, &Default::default()).is_ok());

        let output_path = dir.path().join(Path::new("results.txt"));
        assert!(output_path.exists());
    });
}
