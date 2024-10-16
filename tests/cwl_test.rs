use std::path::Path;

use s4n::cwl::{
    clt::{Command, CommandInputParameter, CommandLineBinding, CommandLineTool, DefaultValue, DockerRequirement, Entry, InitialWorkDirRequirement, Listing, Requirement},
    types::{CWLType, File},
};
use serde_yml::Value;

fn os_path(path: &str) -> String {
    if cfg!(target_os = "windows") {
        Path::new(path).to_string_lossy().replace("/", "\\")
    } else {
        path.to_string()
    }
}

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

    clt.save("workflows/tool/tool.cwl");

    //check if paths are rewritten upon tool saving

    assert_eq!(clt.inputs[0].default, Some(DefaultValue::File(File::from_location(&os_path("../../test_data/input.txt")))));
    let requirements = &clt.requirements.unwrap();
    let req_0 = &requirements[0];
    let req_1 = &requirements[1];
    assert_eq!(
        *req_0,
        Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement {
            listing: vec![Listing {
                entry: Entry::from_file(&os_path("../../test/script.py")),
                entryname: "test/script.py".to_string()
            }]
        })
    );
    assert_eq!(
        *req_1,
        Requirement::DockerRequirement(DockerRequirement::DockerFile {
            docker_file: Entry::from_file(&os_path("../../test/data/Dockerfile")),
            docker_image_id: "test".to_string()
        })
    );
}
