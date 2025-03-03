mod common;
use common::with_temp_repository;
use cwl::{
    clt::{Argument, Command, CommandLineTool},
    inputs::{CommandInputParameter, CommandLineBinding},
    outputs::{CommandOutputBinding, CommandOutputParameter},
    requirements::{InitialWorkDirRequirement, Requirement},
    types::{CWLType, DefaultValue, File},
};
use cwl_execution::runner::run_command;
use s4n::parser::{get_outputs, parse_command_line};
use serde_yaml::Value;
use serial_test::serial;
use std::{path::Path, vec};

pub fn test_cases() -> Vec<(String, CommandLineTool)> {
    vec![
        (
            "python script.py".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec!["python".to_string(), "script.py".to_string()]))
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(
                    "script.py",
                ))]),
        ),
        (
            "Rscript script.R".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec!["Rscript".to_string(), "script.R".to_string()]))
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(
                    "script.R",
                ))]),
        ),
        (
            "python script.py --option1 value1".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec!["python".to_string(), "script.py".to_string()]))
                .with_inputs(vec![CommandInputParameter::default()
                    .with_id("option1")
                    .with_type(CWLType::String)
                    .with_binding(CommandLineBinding::default().with_prefix(&"--option1".to_string()))
                    .with_default_value(DefaultValue::Any(Value::String("value1".to_string())))])
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(
                    "script.py",
                ))]),
        ),
        (
            "python script.py --option1 \"value with spaces\"".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec!["python".to_string(), "script.py".to_string()]))
                .with_inputs(vec![CommandInputParameter::default()
                    .with_id("option1")
                    .with_type(CWLType::String)
                    .with_binding(CommandLineBinding::default().with_prefix(&"--option1".to_string()))
                    .with_default_value(DefaultValue::Any(Value::String("value with spaces".to_string())))])
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(
                    "script.py",
                ))]),
        ),
        (
            "python script.py positional1 --option1 value1".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec!["python".to_string(), "script.py".to_string()]))
                .with_inputs(vec![
                    CommandInputParameter::default()
                        .with_id("positional1")
                        .with_default_value(DefaultValue::Any(Value::String("positional1".to_string())))
                        .with_type(CWLType::String)
                        .with_binding(CommandLineBinding::default().with_position(0)),
                    CommandInputParameter::default()
                        .with_id("option1")
                        .with_type(CWLType::String)
                        .with_binding(CommandLineBinding::default().with_prefix(&"--option1".to_string()))
                        .with_default_value(DefaultValue::Any(Value::String("value1".to_string()))),
                ])
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(
                    "script.py",
                ))]),
        ),
    ]
}

#[test]
pub fn test_parse_command_line() {
    for (input, expected) in test_cases() {
        let args = shlex::split(input.as_str()).expect("Parsing test case failed");
        let result = parse_command_line(args.iter().map(AsRef::as_ref).collect());
        assert_eq!(result, expected);
        println!("{result:?}");
    }
}

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
    assert!(run_command(&cwl, None).is_ok());
}

#[test]
#[serial]
pub fn test_cwl_execute_command_multiple() {
    with_temp_repository(|dir| {
        let command = "python scripts/echo.py --test data/input.txt";
        let args = shlex::split(command).expect("parsing failed");
        let cwl = parse_command_line(args.iter().map(AsRef::as_ref).collect());
        assert!(run_command(&cwl, None).is_ok());

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
                glob: Some("my-file.txt".to_string()),
                ..Default::default()
            }),
        CommandOutputParameter::default()
            .with_type(CWLType::File)
            .with_id("archive")
            .with_binding(CommandOutputBinding {
                glob: Some("archive.tar.gz".to_string()),
                ..Default::default()
            }),
    ];

    let outputs = get_outputs(files);
    assert_eq!(outputs, expected);
}

#[test]
pub fn test_parse_redirect() {
    let command = "cat tests/test_data/input.txt \\> output.txt";
    let split_params = shlex::split(command).unwrap();
    let tool = parse_command_line(split_params.iter().map(AsRef::as_ref).collect());

    assert!(tool.stdout == Some("output.txt".to_string()));
}

#[test]
pub fn test_parse_redirect_stderr() {
    let command = "cat tests/test_data/inputtxt 2\\> err.txt";
    let split_params = shlex::split(command).unwrap();
    let tool = parse_command_line(split_params.iter().map(AsRef::as_ref).collect());

    assert!(tool.stderr == Some("err.txt".to_string()));
}

#[test]
pub fn test_parse_pipe_op() {
    let command = "df \\| grep --line-buffered tmpfs \\> df.log";
    let split_params = shlex::split(command).unwrap();
    let tool = parse_command_line(split_params.iter().map(AsRef::as_ref).collect());

    assert!(tool.arguments.is_some());
    assert!(tool.has_shell_command_requirement());

    if let Some(args) = tool.arguments {
        if let Argument::Binding(pipe) = &args[0] {
            assert!(pipe.value_from == Some("|".to_string()));
        } else {
            panic!();
        }
    }

    assert!(tool.stdout.is_none()); //as it is in args!
}
