#![allow(clippy::disallowed_macros)]
use serde_yaml::Value;
use serial_test::serial;
use std::env;
use std::fs;
use tempfile::tempdir;
use cwl_annotation::{
    common::{annotate_license, parse_cwl, get_filename, annotate_default, annotate},
};

const CWL_CONTENT: &str = r"
    class: CommandLineTool
    baseCommand: echo
    hints:
      DockerRequirement:
        dockerPull: node:slim
    inputs: []
    outputs: []
    ";

const SCHEMAORG_NAMESPACE: &str = "https://schema.org/";
const SCHEMAORG_SCHEMA: &str = "https://schema.org/version/latest/schemaorg-current-https.rdf";
const ARC_NAMESPACE: &str = "https://github.com/nfdi4plants/ARC_ontology";
const ARC_SCHEMA: &str = "https://raw.githubusercontent.com/nfdi4plants/ARC_ontology/main/ARC_v2.0.owl";

#[tokio::test]
#[serial]
async fn test_annotate_license() {

    let dir = tempdir().unwrap();
    let current = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let cwl_name = "test_license.cwl";
    fs::write(cwl_name, "class: CommandLineTool\n").unwrap();

    let license = Some("MIT".to_string());
    let result = annotate_license(cwl_name, &license).await;
    assert!(result.is_ok());

    let yaml = parse_cwl(cwl_name).unwrap();
    if let Value::Mapping(ref mapping) = yaml {
        assert!(mapping.contains_key(Value::String("s:license".to_string())));
        assert_eq!(
            mapping.get(Value::String("s:license".to_string())),
            Some(Value::Sequence(vec![Value::String("MIT".to_string())])).as_ref()
        );
        assert!(mapping.contains_key(Value::String("$namespaces".to_string())));
        assert!(mapping.contains_key(Value::String("$schemas".to_string())));
    } else {
        panic!("YAML root is not a mapping");
    }

    std::env::set_current_dir(current).unwrap();
}


#[test]
#[serial]
fn test_get_filename() {
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let base_name = "example";
    let cwl_name = format!("{base_name}.cwl");
    let workflows_dir = dir.path().join(format!("workflows/{base_name}"));
    fs::create_dir_all(&workflows_dir).unwrap();
    let file_in_current_dir = dir.path().join(cwl_name.clone());
    let file_in_workflows_dir = workflows_dir.join(cwl_name);

    // Create file in the current directory
    fs::write(&file_in_current_dir, "").unwrap();

    // Get the canonical paths
    let file_in_current_dir_canonical = fs::canonicalize(&file_in_current_dir).unwrap();
    let result = get_filename(base_name);

    assert!(result.is_ok(), "Expected Ok(file path), got Err: {result:?}");
    assert_eq!(
        fs::canonicalize(result.unwrap()).unwrap(),
        file_in_current_dir_canonical,
        "File not correctly located in the current directory"
    );

    fs::remove_file(&file_in_current_dir).unwrap();

    // Create file in the workflows directory
    fs::write(&file_in_workflows_dir, "").unwrap();
    let file_in_workflows_dir_canonical = fs::canonicalize(&file_in_workflows_dir).unwrap();
    let result = get_filename(base_name);

    assert!(result.is_ok(), "Expected Ok(file path), got Err: {result:?}");
    assert_eq!(
        fs::canonicalize(result.unwrap()).unwrap(),
        file_in_workflows_dir_canonical,
        "File not correctly located in the workflows directory"
    );

    fs::remove_file(&file_in_workflows_dir).unwrap();

    // Test case where file is not found
    let result = get_filename(base_name);
    assert!(result.is_err(), "Expected Err(file not found), got Ok: {result:?}");
    assert!(
        result.unwrap_err().to_string().contains("CWL file 'example.cwl' not found"),
        "Expected error message about missing file, but got different error"
    );

    env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_annotate_default() {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let tool_name = "test_tool";
    let temp_file_name = format!("{tool_name}.cwl");

    fs::write(&temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let result = annotate_default(tool_name);
    assert!(result.is_ok(), "Expected Ok(()), got: {result:?}");

    // Read the updated file and check if annotations were added
    let updated_content = fs::read_to_string(&temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("$namespaces:")
            && updated_content.contains("s:")
            && updated_content.contains("$schemas:")
            && updated_content.contains(SCHEMAORG_SCHEMA)
            && updated_content.contains(SCHEMAORG_NAMESPACE),
        "Expected annotations for schemaorg to be added, but got: {updated_content}"
    );
    assert!(
        updated_content.contains("arc:") && updated_content.contains(ARC_SCHEMA) && updated_content.contains(ARC_NAMESPACE),
        "Expected annotations for arc to be added, but got: {updated_content}"
    );

    env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_parse_cwl_valid_absolute_path() {
    let dir = tempdir().unwrap();
    let current = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let file_name = "valid_tool.cwl";
    let cwl_path = dir.path().join(file_name);

    fs::write(cwl_path, CWL_CONTENT).unwrap();

    let result = parse_cwl(file_name);
    assert!(result.is_ok(), "Expected Ok(Value), got Err: {result:?}");

    let yaml = result.unwrap();
    assert_eq!(yaml["class"], "CommandLineTool");
    assert_eq!(yaml["baseCommand"], "echo");

    std::env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_parse_cwl_valid_relative_path() {
    let dir = tempdir().unwrap();
    let current = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let file_name = "valid_tool.cwl";

    fs::write(file_name, CWL_CONTENT).unwrap();

    let result = parse_cwl(file_name);
    assert!(result.is_ok(), "Expected Ok(Value), got Err: {result:?}");

    let yaml = result.unwrap();
    assert_eq!(yaml["class"], "CommandLineTool");
    assert_eq!(yaml["baseCommand"], "echo");

    std::env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_parse_cwl_file_not_found() {
    let dir = tempdir().unwrap();
    let current = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let file_name = "non_existent_tool.cwl";

    let result = parse_cwl(file_name);
    assert!(result.is_err(), "Expected Err for non-existent file, got: {result:?}");

    std::env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_parse_cwl_invalid_yaml() {
    let dir = tempdir().unwrap();
    let current = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let file_name = "invalid_tool.cwl";
    let yaml_content = r#"
        name: "example_tool
        version: "1.0"
    "#;
    fs::write(file_name, yaml_content).unwrap();

    let result = parse_cwl(file_name);
    assert!(result.is_err(), "Expected Err for invalid YAML, got: {result:?}");

    std::env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_namespace_key_as_sequence() {
    let dir = tempdir().unwrap();
    let current = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let file_name = "valid_tool.cwl";
    fs::write(file_name, CWL_CONTENT).unwrap();
    let result = annotate(file_name, "namespace", Some("key"), None);
    assert!(result.is_ok());
    std::env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_namespace_key_as_mapping() {
    let dir = tempdir().unwrap();
    let current = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let file_name = "valid_tool.cwl";
    fs::write(file_name, CWL_CONTENT).unwrap();
    let result = annotate(file_name, "namespace", Some("key"), Some("value"));
    assert!(result.is_ok());
    std::env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_add_to_sequence() {
    let dir = tempdir().unwrap();
    let current = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let file_name = "valid_tool.cwl";

    fs::write(file_name, CWL_CONTENT).unwrap();
    let result = annotate(file_name, "namespace", Some("new_key"), None);
    assert!(result.is_ok());
    std::env::set_current_dir(current).unwrap();
}
