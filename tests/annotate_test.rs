use tokio;
use serial_test::serial;
use std::env;
use std::fs;
use tempfile::tempdir;
use s4n::commands::annotate::{handle_annotate_commands, AnnotateCommands, AuthorArgs, PerformerArgs, AnnotateProcessArgs, OntologyMapper, annotate_process_step};

const CWL_CONTENT: &str = r#"
    class: CommandLineTool
    baseCommand: echo
    hints:
      DockerRequirement:
        dockerPull: node:slim
    inputs: []
    outputs: []
    "#;

#[tokio::test]
#[serial]
async fn test_annotate_container() {
    // Create a temporary directory
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    // Switch to the temporary directory
    env::set_current_dir(dir.path()).unwrap();

    // Create a temporary CWL file within the temporary directory
    let temp_file_name = "test.cwl";
    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    // Prepare the command for the test
    let command = AnnotateCommands::Container {
        cwl_name: temp_file_name.to_string(),
        container: "docker://my-container:latest".to_string(),
    };

    // Call the function
    let result = handle_annotate_commands(&command).await;

    // Assert the result is okay
    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    // Verify the CWL file is updated as expected
    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("docker://my-container:latest"),
        "Expected container annotation to be added, but got: {}",
        updated_content
    );

    // Restore the original directory
    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_schema() {
    // Create a temporary directory
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    // Switch to the temporary directory
    env::set_current_dir(dir.path()).unwrap();

    // Create a temporary CWL file within the temporary directory
    let temp_file_name = "test.cwl";

    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    // Prepare the command
    let command = AnnotateCommands::Schema {
        cwl_name: temp_file_name.to_string(),
        schema: "schema_definition".to_string(),
    };

    // Call the function
    let result = handle_annotate_commands(&command).await;

    // Assert the result is okay
    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    // Verify the CWL file is updated
    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("schema_definition"),
        "Expected schema annotation to be added, but got: {}",
        updated_content
    );

    // Restore the original directory
    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_namespace() {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";

    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let command = AnnotateCommands::Namespace {
        cwl_name: temp_file_name.to_string(),
        namespace: "namespace_uri".to_string(),
        short: Some("ns".to_string()),
    };

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("namespace_uri") && updated_content.contains("ns"),
        "Expected namespace annotation to be added, but got: {}",
        updated_content
    );

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_name() {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";

    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let command = AnnotateCommands::Name {
        cwl_name: temp_file_name.to_string(),
        name: "MyWorkflow".to_string(),
    };

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("MyWorkflow"),
        "Expected name annotation to be added, but got: {}",
        updated_content
    );

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_description() {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";

    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let command = AnnotateCommands::Description {
        cwl_name: temp_file_name.to_string(),
        description: "MyWorkflow description".to_string(),
    };

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("MyWorkflow description"),
        "Expected description annotation to be added, but got: {}",
        updated_content
    );

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_license() {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";

    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let command = AnnotateCommands::License {
        cwl_name: temp_file_name.to_string(),
        license: "MIT".to_string(),
    };

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("MIT"),
        "Expected license annotation to be added, but got: {}",
        updated_content
    );

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_performer() {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";

    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let command = AnnotateCommands::Performer(PerformerArgs {
        cwl_name: temp_file_name.to_string(),
        first_name: "J".to_string(),
        last_name: "Doe".to_string(), 
        mail: Some("doe@mail.com".to_string()), 
        affiliation: Some("institute1".to_string()),
    });

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("arc:first name: J") &&
        updated_content.contains("arc:last name: Doe") &&
        updated_content.contains("arc:email: doe@mail.com") &&
        updated_content.contains("arc:affiliation: institute1"),
        "Expected performer annotation to be added, but got: {}",
        updated_content
    );

    env::set_current_dir(current).unwrap();
}


#[tokio::test]
#[serial]
async fn test_annotate_author() {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";

    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let command = AnnotateCommands::Author(AuthorArgs {
        cwl_name: temp_file_name.to_string(),
        name: "J Doe".to_string(),
        mail: Some("doe@mail.com".to_string()), 
        id: Some("http://orcid.org/0000-0000-0000-0000".to_string()),
    });

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("class: s:Person") &&
        updated_content.contains("s:identifier: http://orcid.org/0000-0000-0000-0000") &&
        updated_content.contains("s:email: mailto:doe@mail.com") &&
        updated_content.contains("s:name: J Doe"),
        "Expected performer annotation to be added, but got: {}",
        updated_content
    );

    env::set_current_dir(current).unwrap();
}


#[tokio::test]
#[serial]
async fn test_annotate_process_step_with_input_output() {
    use tempfile::tempdir;
    use std::env;
    use std::fs;

    // Create a temporary directory
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    // Create a CWL file
    let cwl_file_name = "test_process.cwl";

    fs::write(cwl_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    // Define annotation arguments
    let args = AnnotateProcessArgs {
        cwl_name: cwl_file_name.to_string(),
        name: "sequence1".to_string(),
        input: Some("input_data".to_string()),
        output: Some("output_data".to_string()),
        parameter: None,
        value: None,
        key: None, 
        mapper: OntologyMapper::default(),
    };

    // Call the annotation function
    let result = annotate_process_step(&args).await;

    // Assert that the operation was successful
    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    // Verify that the CWL file is updated
    let updated_content = fs::read_to_string(cwl_file_name).expect("Failed to read updated CWL file");
    assert!(updated_content.contains("arc:has process sequence"), "Process sequence not added");
    assert!(updated_content.contains("arc:name: sequence1"), "Name not added");
    assert!(updated_content.contains("arc:has input"), "has input not added");
    assert!(updated_content.contains("input_data"), "Input not added");
    assert!(updated_content.contains("output_data"), "Output not added");

    // Restore original directory
    env::set_current_dir(current).unwrap();
}
