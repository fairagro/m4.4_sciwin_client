use s4n::cwl::{
    clt::{
        Command, CommandInputParameter, CommandLineBinding, CommandLineTool, DefaultValue,
        InitialWorkDirRequirement, Requirement,
    },
    parser::parse_command_line,
    types::CWLType,
};
use serde_yml::Value;

pub fn test_cases() -> Vec<(String, CommandLineTool)> {
    vec![
        (
            "python script.py".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec![
                    "python".to_string(),
                    "script.py".to_string(),
                ]))
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(
                    InitialWorkDirRequirement::from_file("script.py"),
                )]),
        ),
        (
            "Rscript script.R".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec![
                    "Rscript".to_string(),
                    "script.R".to_string(),
                ]))
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(
                    InitialWorkDirRequirement::from_file("script.R"),
                )]),
        ),
        (
            "python script.py --option1 value1".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec![
                    "python".to_string(),
                    "script.py".to_string(),
                ]))
                .with_inputs(vec![CommandInputParameter::default()
                    .with_id("option1")
                    .with_type(CWLType::String)
                    .with_binding(
                        CommandLineBinding::default().with_prefix(&"--option1".to_string()),
                    )
                    .with_default_value(DefaultValue::Any(Value::String(
                        "value1".to_string(),
                    )))])
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(
                    InitialWorkDirRequirement::from_file("script.py"),
                )]),
        ),
        (
            "python script.py --option1 \"value with spaces\"".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec![
                    "python".to_string(),
                    "script.py".to_string(),
                ]))
                .with_inputs(vec![CommandInputParameter::default()
                    .with_id("option1")
                    .with_type(CWLType::String)
                    .with_binding(
                        CommandLineBinding::default().with_prefix(&"--option1".to_string()),
                    )
                    .with_default_value(DefaultValue::Any(Value::String(
                        "value with spaces".to_string(),
                    )))])
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(
                    InitialWorkDirRequirement::from_file("script.py"),
                )]),
        ),
        (
            "python script.py positional1 --option1 value1".to_string(),
            CommandLineTool::default()
                .with_base_command(Command::Multiple(vec![
                    "python".to_string(),
                    "script.py".to_string(),
                ]))
                .with_inputs(vec![
                    CommandInputParameter::default()
                        .with_id("positional1")
                        .with_default_value(DefaultValue::Any(Value::String(
                            "positional1".to_string(),
                        )))
                        .with_type(CWLType::String)
                        .with_binding(CommandLineBinding::default().with_position(0)),
                    CommandInputParameter::default()
                        .with_id("option1")
                        .with_type(CWLType::String)
                        .with_binding(
                            CommandLineBinding::default().with_prefix(&"--option1".to_string()),
                        )
                        .with_default_value(DefaultValue::Any(Value::String("value1".to_string()))),
                ])
                .with_requirements(vec![Requirement::InitialWorkDirRequirement(
                    InitialWorkDirRequirement::from_file("script.py"),
                )]),
        ),
    ]
}

#[test]
pub fn test_command_line_parser() {
    for (input, expected) in test_cases() {
        let args = shlex::split(input.as_str()).expect("Parsing test case failed");
        let result = parse_command_line(args.iter().map(|x| x.as_ref()).collect());
        assert_eq!(result, expected);
        println!("{:?}", result);
    }
}

#[test]
pub fn test_execution() {
    let command = "ls -la";
    let args = shlex::split(command).expect("parsing failed");
    let result = parse_command_line(args.iter().map(|x| x.as_ref()).collect());
    let status = result.execute();
    assert!(status.success())
}
