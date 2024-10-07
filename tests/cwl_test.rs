use s4n::cwl::{
    clt::{Command, CommandInputParameter, CommandLineBinding, CommandLineTool, DefaultValue, DockerRequirement, InitialWorkDirRequirement, Requirement},
    types::{CWLType, File},
};
use serde_yml::Value;

#[test]
pub fn test_cwl_save() {
    let inputs = vec![
        CommandInputParameter::default()
            .with_id("positional1")
            .with_default_value(DefaultValue::File(File::from_location(&"test_data/input.txt".to_string())))
            .with_type(CWLType::String)
            .with_binding(CommandLineBinding::default().with_position(0)),
        CommandInputParameter::default()
            .with_id("option1")
            .with_type(CWLType::String)
            .with_binding(CommandLineBinding::default().with_prefix(&"--option1".to_string()))
            .with_default_value(DefaultValue::Any(Value::String("value1".to_string()))),
    ];
    let mut clt = CommandLineTool::default()
        .with_base_command(Command::Multiple(vec!["python".to_string(), "test/script.py".to_string()]))
        .with_inputs(inputs)
        .with_requirements(vec![
            Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("test/script.py")),
            Requirement::DockerRequirement(DockerRequirement::from_file("test/data/Dockerfile", "test")),
        ]);

    clt.save(&"workflows/tool/tool.cwl".to_string());
    println!("{:?}", clt);
    assert!(false)
}
