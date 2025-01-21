use tokio;
use serial_test::serial;
use std::env;
use std::fs;
use tempfile::tempdir;
use s4n::commands::annotate::{handle_annotate_commands, AnnotateCommands, AuthorArgs,
     PerformerArgs, AnnotateProcessArgs, OntologyMapper, annotate_process_step,
     annotate_performer,  
     get_filename, contains_docker_requirement, annotate_field, annotate_ontology, 
     parse_cwl, annotate_default, get_json_biotools, select_annotation, bioportal_recommendations, zooma_recommendations};
use serde_yml::Value;
use std::error::Error;
use httpmock::MockServer;
use reqwest::Client; 
use httpmock::Method::GET; 
use std::collections::HashSet;
use serde_json::json;


const CWL_CONTENT: &str = r#"
    class: CommandLineTool
    baseCommand: echo
    hints:
      DockerRequirement:
        dockerPull: node:slim
    inputs: []
    outputs: []
    "#;

const CWL_CONTENT_ANNOTATED: &str = r#"
    class: CommandLineTool
    baseCommand: echo
    hints:
        DockerRequirement:
        dockerPull: node:slim
    inputs: []
    outputs: []
    s:author:
    - class: s:Person
      s:identifier: https://orcid.org/0000-0002-6130-1021
      s:email: mailto:dyuen@oicr.on.ca
      s:name: Denis Yuen
    arc:performer:
    - class: arc:Person
      arc:first name: "Example"
      arc:last name: "Person"
      arc:email: "example.person@email.de "
      arc:affiliation: "Institution"
      arc:has role:
      - class: arc:role
        arc:term accession: "https://credit.niso.org/contributor-roles/formal-analysis/"
        arc:annotation value: "Formal analysis"
    s:citation: https://dx.doi.org/10.6084/m9.figshare.3115156.v2
    s:codeRepository: https://github.com/common-workflow-language/common-workflow-language
    s:dateCreated: "2016-12-13"
    s:license: https://spdx.org/licenses/Apache-2.0
    $namespaces:
      s: https://schema.org/
      arc: https://github.com/nfdi4plants/ARC_ontology
    $schemas:
      - https://schema.org/version/latest/schemaorg-current-https.rdf
      - https://raw.githubusercontent.com/nfdi4plants/ARC_ontology/main/ARC_v2.0.owl
    "#;

// A mock of the CWL_WITH_DOCKER_CONTENT that simulates a CWL file with Docker requirement
const CWL_WITH_DOCKER_CONTENT: &str = r#"
    cwlVersion: v1.0
    class: CommandLineTool
    baseCommand: [echo]
    requirements:
        - DockerRequirement:
            dockerPull: 'busybox'
"#;

// Mocking external constants
//const SCHEMAORG_NAMESPACE: &str = "https://schema.org/version/latest/schemaorg-current-https.rdf";
const SCHEMAORG_SCHEMA: &str = "https://schema.org/";
const ARC_NAMESPACE: &str = "https://github.com/nfdi4plants/ARC_ontology";
//const ARC_SCHEMA: &str = " https://raw.githubusercontent.com/nfdi4plants/ARC_ontology/main/ARC_v2.0.owl";

#[tokio::test]
#[serial]
async fn test_annotate_container() {

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";
    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let command = AnnotateCommands::Container {
        cwl_name: temp_file_name.to_string(),
        container: "docker://my-container:latest".to_string(),
    };

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("docker://my-container:latest"),
        "Expected container annotation to be added, but got: {}",
        updated_content
    );

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_new_container() {

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";
    fs::write(temp_file_name, CWL_CONTENT_ANNOTATED).expect("Failed to write CWL file");

    let command = AnnotateCommands::Container {
        cwl_name: temp_file_name.to_string(),
        container: "docker://my-container:latest".to_string(),
    };

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("docker://my-container:latest"),
        "Expected container annotation to be added, but got: {}",
        updated_content
    );

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_same_container() {

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";
    fs::write(temp_file_name, CWL_CONTENT_ANNOTATED).expect("Failed to write CWL file");

    let command = AnnotateCommands::Container {
        cwl_name: temp_file_name.to_string(),
        container: "node:slim".to_string(),
    };

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("node:slim"),
        "Expected container annotation to be added, but got: {}",
        updated_content
    );

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_schema() {

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";

    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let command = AnnotateCommands::Schema {
        cwl_name: temp_file_name.to_string(),
        schema: "schema_definition".to_string(),
    };

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("schema_definition"),
        "Expected schema annotation to be added, but got: {}",
        updated_content
    );

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
async fn test_annotate_author_exists() {
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
async fn test_annotate_same_author() {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";

    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let command = AnnotateCommands::Author(AuthorArgs {
        cwl_name: temp_file_name.to_string(),
        name: "Denis Yuen".to_string(),
        mail: Some("dyuen@oicr.on.ca".to_string()), 
        id: Some("https://orcid.org/0000-0002-6130-1021".to_string()),
    });

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("class: s:Person") &&
        updated_content.contains("s:identifier: https://orcid.org/0000-0002-6130-1021") &&
        updated_content.contains("s:email: mailto:dyuen@oicr.on.ca") &&
        updated_content.contains("s:name: Denis Yuen"),
        "Expected performer annotation to be added, but got: {}",
        updated_content
    );

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_process_step_with_input_output() {

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let cwl_file_name = "test_process.cwl";

    fs::write(cwl_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let args = AnnotateCommands::Process(AnnotateProcessArgs {
        cwl_name: cwl_file_name.to_string(),
        name: "sequence1".to_string(),
        input: Some("input_data".to_string()),
        output: Some("output_data".to_string()),
        parameter: None,
        value: None,
        key: None, 
        mapper: OntologyMapper::default(),
    });


    let result = handle_annotate_commands(&args).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(cwl_file_name).expect("Failed to read updated CWL file");
    println!("updated_content {:?}", updated_content);
    assert!(updated_content.contains("arc:has process sequence"), "Process sequence not added");
    assert!(updated_content.contains("arc:name: sequence1"), "Name not added");
    assert!(updated_content.contains("arc:has input"), "has input not added");
    assert!(updated_content.contains("arc:has output"), "has output not added");
    assert!(updated_content.contains("input_data"), "Input not added");
    assert!(updated_content.contains("output_data"), "Output not added");

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_custom() {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";

    fs::write(temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let command = AnnotateCommands::Custom {
        cwl_name: temp_file_name.to_string(),
        field: "test_field".to_string(),
        value: "test_value".to_string(),
    };

    let result = handle_annotate_commands(&command).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("test_field"),
        "Expected test_field annotation to be added, but got: {}",
        updated_content
    );
    assert!(
        updated_content.contains("test_value"),
        "Expected test_value annotation to be added, but got: {}",
        updated_content
    );
    env::set_current_dir(current).unwrap();
}



#[tokio::test]
#[serial]
async fn test_annotate_process() {

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let cwl_file_name = "test_process.cwl";

    fs::write(cwl_file_name, CWL_CONTENT).expect("Failed to write CWL file");

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

    let result = annotate_process_step(&args).await;

    assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

    let updated_content = fs::read_to_string(cwl_file_name).expect("Failed to read updated CWL file");
    println!("updated_content {:?}", updated_content);
    assert!(updated_content.contains("arc:has process sequence"), "Process sequence not added");
    assert!(updated_content.contains("arc:name: sequence1"), "Name not added");
    assert!(updated_content.contains("arc:has input"), "has input not added");
    assert!(updated_content.contains("arc:has output"), "has output not added");
    assert!(updated_content.contains("input_data"), "Input not added");
    assert!(updated_content.contains("output_data"), "Output not added");

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_get_filename() {
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let base_name = "example";
    let cwl_name = format!("{}.cwl", base_name);
    let workflows_dir = dir.path().join(format!("workflows/{}", base_name));
    fs::create_dir_all(&workflows_dir).unwrap();
    let file_in_current_dir = dir.path().join(&cwl_name);
    let file_in_workflows_dir = workflows_dir.join(&cwl_name);

    fs::write(&file_in_current_dir, "").unwrap();
    let result = get_filename(&base_name);
    assert!(
        result.is_ok(),
        "Expected Ok(file path), got Err: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        file_in_current_dir.display().to_string(),
        "File not correctly located in the current directory"
    );

    fs::remove_file(&file_in_current_dir).unwrap();

    fs::write(&file_in_workflows_dir, "").unwrap();
    let result = get_filename(&base_name);
    assert!(
        result.is_ok(),
        "Expected Ok(file path), got Err: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        file_in_workflows_dir.display().to_string(),
        "File not correctly located in the workflows directory"
    );

    fs::remove_file(&file_in_workflows_dir).unwrap();

    let result = get_filename(&base_name);
    assert!(
        result.is_err(),
        "Expected Err(file not found), got Ok: {:?}",
        result
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("CWL file 'example.cwl' not found"),
        "Expected error message about missing file, but got different error"
    );

    env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_annotate_performer_add_to_existing_list() {

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let cwl_filename = "test_process.cwl";

    fs::write(cwl_filename, CWL_CONTENT_ANNOTATED).expect("Failed to write CWL file");

    let args = PerformerArgs {
        cwl_name: cwl_filename.to_string(),
        first_name: "Jane".to_string(),
        last_name: "Smith".to_string(),
        mail: Some("jane.smith@example.com".to_string()),
        affiliation: Some("Example Organization".to_string()),
    };

    let result = annotate_performer(&args);

    assert!(result.is_ok(), "Expected Ok, got {:?}", result);

    let updated_yaml: Value = serde_yml::from_str(&std::fs::read_to_string(cwl_filename).unwrap()).unwrap();

    if let Value::Sequence(performers) = &updated_yaml["arc:performer"] {
        assert_eq!(performers.len(), 2, "Expected 2 performers, found {}", performers.len());
        let new_performer = &performers[1];
        assert_eq!(new_performer["arc:first name"], "Jane");
        assert_eq!(new_performer["arc:last name"], "Smith");
        assert_eq!(new_performer["arc:email"], "jane.smith@example.com");
        assert_eq!(new_performer["arc:affiliation"], "Example Organization");
    } else {
        panic!("Expected 'arc:performer' to be a sequence.");
    }

    env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_annotate_performer_avoid_duplicate() {

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let cwl_content = r#"
    arc:performer:
      - class: arc:Person
        arc:first name: "Charlie"
        arc:last name: "Davis"
        arc:email: "charlie.davis@example.com"
    "#;

    let cwl_filename = "test.cwl";

    std::fs::write(&cwl_filename, cwl_content).unwrap();

    let args = PerformerArgs {
        cwl_name: cwl_filename.to_string(),
        first_name: "Charlie".to_string(),
        last_name: "Davis".to_string(),
        mail: Some("charlie.davis@example.com".to_string()),
        affiliation: None,
    };

    let result = annotate_performer(&args);

    assert!(result.is_ok(), "Expected Ok, got {:?}", result);

    let updated_yaml: Value = serde_yml::from_str(&std::fs::read_to_string(cwl_filename).unwrap()).unwrap();

    if let Value::Sequence(performers) = &updated_yaml["arc:performer"] {
        assert_eq!(performers.len(), 1, "Expected 1 performer, found {}", performers.len());
    } else {
        panic!("Expected 'arc:performer' to be a sequence.");
    }

    env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_annotate_performer_invalid_root() {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let cwl_content = r#"
    - not_a_mapping
    "#;

    let cwl_filename = "test_invalid_root.cwl";

    std::fs::write(&cwl_filename, cwl_content).unwrap();

    let args = PerformerArgs {
        cwl_name: cwl_filename.to_string(),
        first_name: "David".to_string(),
        last_name: "Evans".to_string(),
        mail: None,
        affiliation: None,
    };

    let result = annotate_performer(&args);

    assert!(result.is_err(), "Expected Err, got {:?}", result);

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_contains_docker_requirement() {
    use tempfile::tempdir;
    use std::fs;

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let file_with_docker = dir.path().join("with_docker.cwl");
    let file_without_docker = dir.path().join("without_docker.cwl");
    let empty_file = dir.path().join("empty.cwl");

    let content_with_docker = r#"
class: CommandLineTool
requirements:
  DockerRequirement:
    dockerPull: "python:3.9"
    "#;
    fs::write(&file_with_docker, content_with_docker).unwrap();
    let result = contains_docker_requirement(file_with_docker.to_str().unwrap());
    assert!(
        result.is_ok(),
        "Expected Ok(true), but got Err: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        true,
        "Expected true for file containing 'DockerRequirement'"
    );


    let content_without_docker = r#"
class: CommandLineTool
inputs: []
outputs: []
    "#;
    fs::write(&file_without_docker, content_without_docker).unwrap();
    let result = contains_docker_requirement(file_without_docker.to_str().unwrap());
    assert!(
        result.is_ok(),
        "Expected Ok(false), but got Err: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        false,
        "Expected false for file not containing 'DockerRequirement'"
    );

    fs::write(&empty_file, "").unwrap();
    let result = contains_docker_requirement(empty_file.to_str().unwrap());
    assert!(
        result.is_ok(),
        "Expected Ok(false) for empty file, but got Err: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        false,
        "Expected false for empty file"
    );

    env::set_current_dir(current).unwrap();
}

#[tokio::test]
#[serial]
async fn test_annotate_field() {
    use tempfile::tempdir;
    use std::fs;

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let temp_file_name = "test.cwl";

    let existing_field_content = r#"
class: CommandLineTool
s:license: "MIT"
    "#;
    fs::write(temp_file_name, existing_field_content).unwrap();

    let result = annotate_field(temp_file_name, "s:license", "MIT");
    assert!(
        result.is_ok(),
        "Expected Ok(()), but got Err: {:?}",
        result
    );

    let updated_content = fs::read_to_string(temp_file_name).unwrap();
    assert!(
        updated_content.contains("s:license: MIT"),
        "Expected 's:license' field to remain unchanged, but got: {}",
        updated_content
    );

    let different_value_content = r#"
class: CommandLineTool
s:license: "GPL"
    "#;
    fs::write(temp_file_name, different_value_content).unwrap();

    let result = annotate_field(temp_file_name, "s:license", "MIT");
    assert!(
        result.is_ok(),
        "Expected Ok(()), but got Err: {:?}",
        result
    );

    let updated_content = fs::read_to_string(temp_file_name).unwrap();
    assert!(
        updated_content.contains("s:license: MIT"),
        "Expected 's:license' field to be updated to 'MIT', but got: {}",
        updated_content
    );

    let no_field_content = r#"
class: CommandLineTool
    "#;
    fs::write(temp_file_name, no_field_content).unwrap();

    let result = annotate_field(temp_file_name, "s:license", "MIT");
    assert!(
        result.is_ok(),
        "Expected Ok(()), but got Err: {:?}",
        result
    );

    let updated_content = fs::read_to_string(temp_file_name).unwrap();
    assert!(
        updated_content.contains("s:license: MIT"),
        "Expected 's:license' field to be added, but got: {}",
        updated_content
    );

    // Case 4: Invalid YAML file
    let invalid_yaml_content = r#"
class: CommandLineTool
    invalid_yaml: {::}
    "#;
    fs::write(temp_file_name, invalid_yaml_content).unwrap();

    let result = annotate_field(temp_file_name, "s:license", "MIT");
    assert!(
        result.is_err(),
        "Expected Err for invalid YAML, but got Ok(()): {:?}",
        result
    );

    env::set_current_dir(current).unwrap();
}


#[test]
fn test_annotate_ontology() {

    let term_accession = "http://purl.obolibrary.org/obo/NCIT_C43582";
    let annotation_value = "Data Transformation";
    let result = annotate_ontology(term_accession, None, annotation_value);
    assert!(result.is_ok(), "Expected Ok(()), but got Err: {:?}", result);

    let result_str = result.unwrap();
    assert!(result_str.contains("arc:term accession: http://purl.obolibrary.org/obo/NCIT_C43582"));
    assert!(result_str.contains("arc:annotation value: Data Transformation"));
    assert!(!result_str.contains("arc:term source REF"));

    let source_ref = "NCIT";
    let result = annotate_ontology(term_accession, Some(source_ref), annotation_value);
    assert!(result.is_ok(), "Expected Ok(()), but got Err: {:?}", result);

    let result_str = result.unwrap();
    assert!(result_str.contains("arc:term source REF: NCIT"));
}

#[test]
fn test_annotate_ontology_empty_annotation_value() {

    let term_accession = "http://purl.obolibrary.org/obo/NCIT_C43582";
    let annotation_value = "";
    let result = annotate_ontology(term_accession, None, annotation_value);
    assert!(result.is_ok(), "Expected Ok(()), but got Err: {:?}", result);

    let result_str = result.unwrap();
    println!("result_str {:?}", result_str);
    assert!(result_str.contains("arc:term accession: http://purl.obolibrary.org/obo/NCIT_C43582"));
    assert!(result_str.contains("arc:annotation value: "));
    assert!(!result_str.contains("arc:term source REF"));
}


#[test]
#[serial]
fn test_annotate_default() {

    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let tool_name = "test_tool";
    let temp_file_name = format!("{}.cwl", tool_name);

    fs::write(&temp_file_name, CWL_CONTENT).expect("Failed to write CWL file");

    let result = annotate_default(tool_name);
    assert!(result.is_ok(), "Expected Ok(()), got: {:?}", result);

    // Read the updated file and check if annotations were added
    let updated_content = fs::read_to_string(&temp_file_name).expect("Failed to read updated CWL file");
    assert!(
        updated_content.contains("$namespaces:") &&
        updated_content.contains("s:") &&
        updated_content.contains("$schemas:") &&
        updated_content.contains(SCHEMAORG_SCHEMA),
        "Expected annotations for schemaorg to be added, but got: {}",
        updated_content
    );
    assert!(
        updated_content.contains("arc:") &&
        updated_content.contains(ARC_NAMESPACE),
        "Expected annotations for arc to be added, but got: {}",
        updated_content
    );

    
    let docker_tool_name = "docker_tool";
    let docker_temp_file_name = format!("{}.cwl", docker_tool_name);
    fs::write(&docker_temp_file_name, CWL_WITH_DOCKER_CONTENT).expect("Failed to write CWL file with Docker");

    let result = annotate_default(docker_tool_name);
    assert!(result.is_ok(), "Expected Ok(()), got: {:?}", result);

    let updated_docker_content = fs::read_to_string(&docker_temp_file_name).expect("Failed to read updated CWL file with Docker");
    assert!(
        updated_docker_content.contains("Docker Container"),
        "Expected Docker Container annotation to be added, but got: {}",
        updated_docker_content
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
    let yaml_content = r#"
        name: "example_tool"
        version: "1.0"
    "#;
    fs::write(&cwl_path, yaml_content).unwrap();

    let result = parse_cwl(cwl_path.to_str().unwrap());
    assert!(result.is_ok(), "Expected Ok(Value), got: {:?}", result);

    if let Value::Mapping(mapping) = result.unwrap() {
        assert_eq!(
            mapping.get(&Value::String("name".to_string())),
            Some(&Value::String("example_tool".to_string())),
            "Expected 'name' key to be parsed correctly"
        );
        assert_eq!(
            mapping.get(&Value::String("version".to_string())),
            Some(&Value::String("1.0".to_string())),
            "Expected 'version' key to be parsed correctly"
        );
    } else {
        panic!("Parsed YAML is not a Mapping");
    }

    std::env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
fn test_parse_cwl_valid_relative_path() {
    let dir = tempdir().unwrap();
    let current = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let file_name = "valid_tool.cwl";
    let yaml_content = r#"
        name: "example_tool"
        version: "1.0"
    "#;
    fs::write(file_name, yaml_content).unwrap();

    let result = parse_cwl(file_name);
    assert!(result.is_ok(), "Expected Ok(Value), got: {:?}", result);

    if let Value::Mapping(mapping) = result.unwrap() {
        assert_eq!(
            mapping.get(&Value::String("name".to_string())),
            Some(&Value::String("example_tool".to_string())),
            "Expected 'name' key to be parsed correctly"
        );
        assert_eq!(
            mapping.get(&Value::String("version".to_string())),
            Some(&Value::String("1.0".to_string())),
            "Expected 'version' key to be parsed correctly"
        );
    } else {
        panic!("Parsed YAML is not a Mapping");
    }

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
    assert!(
        result.is_err(),
        "Expected Err for non-existent file, got: {:?}",
        result
    );

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
    "#; // Missing closing quote on name value
    fs::write(file_name, yaml_content).unwrap();

    let result = parse_cwl(file_name);
    assert!(
        result.is_err(),
        "Expected Err for invalid YAML, got: {:?}",
        result
    );

    std::env::set_current_dir(current).unwrap();
}

#[tokio::test]
async fn test_get_json_biotools_success() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tool")
            .header("Authorization", "apikey token=test_api_key");
        then.status(200)
            .json_body(serde_json::json!({
                "id": "example_tool",
                "name": "Example Tool",
                "version": "1.0"
            }));
    });

    let client = Client::new();
    let url = &format!("{}/tool", &server.base_url());
    let result = get_json_biotools(url, &client, "test_api_key").await;

    assert!(result.is_ok(), "Expected Ok(jsonValue), got Err: {:?}", result);
    let json = result.unwrap();
    assert_eq!(json["id"], "example_tool");
    assert_eq!(json["name"], "Example Tool");
    assert_eq!(json["version"], "1.0");

    mock.assert();
}

#[tokio::test]
async fn test_get_json_biotools_unauthorized() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tool")
            .header("Authorization", "apikey token=invalid_key");
        then.status(401)
            .json_body(serde_json::json!({
                "error": "Unauthorized"
            }));
    });

    let client = Client::new();
    let url = &format!("{}/tool", &server.base_url());
    let result = get_json_biotools(url, &client, "invalid_key").await;

    assert!(result.is_err(), "Expected Err, got Ok: {:?}", result);

    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("401"),
        "Expected error message to indicate unauthorized, got: {}",
        error_message
    );

    mock.assert();
}

#[tokio::test]
async fn test_get_json_biotools_not_found() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tool")
            .header("Authorization", "apikey token=test_api_key");
        then.status(404)
            .json_body(serde_json::json!({
                "error": "Not Found"
            }));
    });

    let client = Client::new();
    let url = &format!("{}/tool", &server.base_url());
    let result = get_json_biotools(url, &client, "test_api_key").await;

    assert!(result.is_err(), "Expected Err, got Ok: {:?}", result);

    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("404"),
        "Expected error message to indicate not found, got: {}",
        error_message
    );

    mock.assert();
}
    
#[tokio::test]
async fn test_get_json_biotools_invalid_json() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tool")
            .header("Authorization", "apikey token=test_api_key");
        then.status(200)
            .body("invalid_json");
    });

    let client = Client::new();
    let url = &format!("{}/tool", &server.base_url());
    let result = get_json_biotools(url, &client, "test_api_key").await;

    assert!(result.is_err(), "Expected Err, got Ok: {:?}", result);

    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("Failed to parse JSON response"),
        "Expected error message to indicate invalid JSON, got: {}",
        error_message
    );

    mock.assert();
}


#[test]
fn test_select_annotation_do_not_use_ontology() {
    let recommendations = HashSet::new();
    let term = "example_term".to_string();

    let input_fn = || Ok("0".to_string()); 
    let result = select_annotation(&recommendations, term.clone(), input_fn);

    assert!(result.is_ok());
    let annotation = result.unwrap();
    assert_eq!(annotation.0, term);
    assert_eq!(annotation.1, "N/A");
    assert_eq!(annotation.2, "N/A");
}

#[test]
fn test_select_annotation_valid_choice() {
    let mut recommendations = HashSet::new();
    recommendations.insert((
        "Example Label".to_string(),
        "Example Ontology".to_string(),
        "Example ID".to_string(),
    ));
    let term = "example_term".to_string();

    let input_fn = || Ok("1".to_string()); 
    let result = select_annotation(&recommendations, term, input_fn);

    assert!(result.is_ok());
    let annotation = result.unwrap();
    assert_eq!(annotation.0, "Example Label");
    assert_eq!(annotation.1, "Example Ontology");
    assert_eq!(annotation.2, "Example ID");
}

#[test]
fn test_select_annotation_invalid_choice() {
    let mut recommendations = HashSet::new();
    recommendations.insert((
        "Example Label".to_string(),
        "Example Ontology".to_string(),
        "Example ID".to_string(),
    ));
    let term = "example_term".to_string();

    let input_fn = || Ok("invalid".to_string()); 
    let result = select_annotation(&recommendations, term, input_fn);

    assert!(result.is_err());
}

#[test]
fn test_select_annotation_out_of_range_choice() {
    let mut recommendations = HashSet::new();
    recommendations.insert((
        "Example Label".to_string(),
        "Example Ontology".to_string(),
        "Example ID".to_string(),
    ));
    let term = "example_term".to_string();

    let input_fn = || Ok("2".to_string()); 
    let result = select_annotation(&recommendations, term, input_fn);

    assert!(result.is_err());
}

#[tokio::test]
async fn test_bioportal_recommendations_empty_response() -> Result<(), Box<dyn Error>> {
    let server = MockServer::start();
    let biotools_key = "fake_biotools_key";
    let search_term = "example_term";
    let max_recommendations = 5;

    let _mock = server.mock(|when, then| {
        when.method(GET)
            .path("/annotator")
            .query_param("text", search_term);
        then.status(200).json_body(json!([]));
    });

    let rest_url_bioportal = server.url("/annotator");
    let result = bioportal_recommendations(&rest_url_bioportal, biotools_key, max_recommendations).await;

    assert!(result.is_err());
    Ok(())
}

#[tokio::test]
async fn test_bioportal_recommendations_unauthorized() -> Result<(), Box<dyn Error>> {
    let server = MockServer::start();
    let biotools_key = "invalid_key";
    let search_term = "example_term";
    let max_recommendations = 5;

    let _mock = server.mock(|when, then| {
        when.method(GET)
            .path("/annotator")
            .query_param("text", search_term);
        then.status(401)
            .json_body(json!({ "error": "Unauthorized" }));
    });

    let rest_url_bioportal = server.url("/annotator");
    let result = bioportal_recommendations(&rest_url_bioportal, biotools_key, max_recommendations).await;

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Unauthorized"));
    }

    Ok(())
}

#[tokio::test]
async fn test_zooma_recommendations_empty_response() -> Result<(), Box<dyn Error>> {
    let server = MockServer::start();
    let max_recommendations = 5;

    let _mock = server.mock(|when, then| {
        when.method(GET)
            .path("/zooma/annotate")
            .query_param("q", "example+term");
        then.status(200).json_body(json!([]));
    });

    let rest_url_zooma = server.url("/zooma/annotate?q=");
    let result = zooma_recommendations(&rest_url_zooma, max_recommendations).await;

    assert!(result.is_err());
    Ok(())
}

