use cwl_execution::{environment::RuntimeEnvironment, runner::run_command};
use s4n::parser::parse_command_line;
use serial_test::serial;
use std::path::Path;
use test_utils::with_temp_repository;

#[test]
#[serial]
pub fn test_cwl_execute_command_multiple() {
    with_temp_repository(|dir| {
        let command = "python scripts/echo.py --test data/input.txt";
        let args = shlex::split(command).expect("parsing failed");
        let cwl = parse_command_line(&args.iter().map(AsRef::as_ref).collect::<Vec<_>>());
        assert!(run_command(&cwl, &mut RuntimeEnvironment::default()).is_ok());

        let output_path = dir.path().join(Path::new("results.txt"));
        assert!(output_path.exists());
    });
}
