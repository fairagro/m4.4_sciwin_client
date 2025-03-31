mod common;
use common::with_temp_repository;
use cwl::{
    clt::{Command, CommandLineTool},
    inputs::{CommandInputParameter, CommandLineBinding},
    outputs::{CommandOutputBinding, CommandOutputParameter},
    requirements::{InitialWorkDirRequirement, Requirement},
    types::{CWLType, DefaultValue, File},
};
use cwl_execution::runner::run_command;
use s4n::parser::{get_outputs, parse_command_line};
use serial_test::serial;
use std::{path::Path, vec};

#[test]
#[serial]
pub fn test_parse_command_line_testdata() {
    with_temp_repository(|_| {
        let command = "python scripts/echo.py --test data/input.txt";
        let args = shlex::split(command).expect("parsing failed");
        let cwl = parse_command_line(args.iter().map(AsRef::as_ref).collect());
        let expected = CommandLineTool::default()
            .with_base_command(Command::Multiple(vec!["python".to_string(), "scripts/echo.py".to_string()]))
            .with_inputs(vec![CommandInputParameter::default()
                .with_id("test")
                .with_type(CWLType::File)
                .with_binding(CommandLineBinding::default().with_prefix(&"--test".to_string()))
                .with_default_value(DefaultValue::File(File::from_location(&"data/input.txt".to_string())))])
            .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(
                "scripts/echo.py",
            ))]);
        assert_eq!(cwl, expected);
    });
}

#[test]
pub fn test_cwl_execute_command_single() {
    let command = "ls -la .";
    let args = shlex::split(command).expect("parsing failed");
    let cwl = parse_command_line(args.iter().map(AsRef::as_ref).collect());
    assert!(run_command(&cwl, &Default::default()).is_ok());
}

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

#[test]
pub fn test_get_outputs() {
    let files = vec!["my-file.txt".to_string(), "archive.tar.gz".to_string()];
    let expected = vec![
        CommandOutputParameter::default()
            .with_type(CWLType::File)
            .with_id("my-file")
            .with_binding(CommandOutputBinding {
                glob: "my-file.txt".to_string(),
            }),
        CommandOutputParameter::default()
            .with_type(CWLType::File)
            .with_id("archive")
            .with_binding(CommandOutputBinding {
                glob: "archive.tar.gz".to_string(),
            }),
    ];

    let outputs = get_outputs(files);
    assert_eq!(outputs, expected);
}
