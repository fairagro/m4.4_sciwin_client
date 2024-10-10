use std::{fs, path::Path, vec};

use s4n::cwl::{
    clt::{Command, CommandInputParameter, CommandLineBinding, CommandLineTool, CommandOutputBinding, CommandOutputParameter, DefaultValue, InitialWorkDirRequirement, Requirement},
    parser::{get_outputs, parse_command_line},
    types::{CWLType, File},
};
use serde_yml::Value;

pub fn test_cases() -> Vec<(String, CommandLineTool)> {
    vec![
        (
            "python script.py".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec!["python".to_string(), "script.py".to_string()]))
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("script.py"))]),
        ),
        (
            "Rscript script.R".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec!["Rscript".to_string(), "script.R".to_string()]))
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("script.R"))]),
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
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("script.py"))]),
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
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("script.py"))]),
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
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("script.py"))]),
        ),
    ]
}

#[test]
pub fn test_parse_command_line() {
    for (input, expected) in test_cases() {
        let args = shlex::split(input.as_str()).expect("Parsing test case failed");
        let result = parse_command_line(args.iter().map(|x| x.as_ref()).collect());
        assert_eq!(result, expected);
        println!("{:?}", result);
    }
}

#[test]
pub fn test_parse_command_line_testdata() {
    let command = "python tests/test_data/echo.py --test tests/test_data/input.txt";
    let args = shlex::split(command).expect("parsing failed");
    let cwl = parse_command_line(args.iter().map(|x| x.as_ref()).collect());
    let expected = CommandLineTool::default()
        .with_base_command(Command::Multiple(vec!["python".to_string(), "tests/test_data/echo.py".to_string()]))
        .with_inputs(vec![CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::File)
            .with_binding(CommandLineBinding::default().with_prefix(&"--test".to_string()))
            .with_default_value(DefaultValue::File(File::from_location(&"tests/test_data/input.txt".to_string())))])
        .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("tests/test_data/echo.py"))]);
    assert_eq!(cwl, expected)
}

#[test]
pub fn test_cwl_execute_command_single() {
    let command = "ls -la";
    let args = shlex::split(command).expect("parsing failed");
    let cwl = parse_command_line(args.iter().map(|x| x.as_ref()).collect());
    assert!(cwl.execute().is_ok())
}

#[test]
pub fn test_cwl_execute_command_multiple() {
    let command = "python ./tests/test_data/echo.py --test ./tests/test_data/input.txt";
    let args = shlex::split(command).expect("parsing failed");
    let cwl = parse_command_line(args.iter().map(|x| x.as_ref()).collect());
    assert!(cwl.execute().is_ok());

    let output_path = Path::new("results.txt");
    assert!(output_path.exists());

    assert!(fs::remove_file(output_path).is_ok());
    assert!(!output_path.exists());
}

#[test]
pub fn test_get_outputs() {
    let files = vec!["my-file.txt".to_string(), "archive.tar.gz".to_string()];
    let expected = vec![
        CommandOutputParameter::default()
            .with_type(CWLType::File)
            .with_id("my-file")
            .with_binding(CommandOutputBinding { glob: "my-file.txt".to_string() }),
        CommandOutputParameter::default()
            .with_type(CWLType::File)
            .with_id("archive")
            .with_binding(CommandOutputBinding { glob: "archive.tar.gz".to_string() }),
    ];

    let outputs = get_outputs(files);
    assert_eq!(outputs, expected);
}
