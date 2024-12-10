use s4n::{
  commands::workflow::{connect_workflow_nodes, disconnect_workflow_nodes, create_workflow, ConnectWorkflowArgs, CreateWorkflowArgs},
  cwl::loader::load_workflow,
  io::create_and_write_file,
};
use serial_test::serial;
use std::{env, path::Path};
use tempfile::tempdir;
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
#[serial]
pub fn test_create_workflow() {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();
    let args = CreateWorkflowArgs {
        name: "test".to_string(),
        force: false,
    };
    let result = create_workflow(&args);
    assert!(result.is_ok());

    let path = "workflows/test/test.cwl";
    assert!(Path::new(path).exists());

    env::set_current_dir(current).unwrap();
}


#[test]
#[serial]
pub fn test_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir().unwrap();
    let current = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    create_and_write_file("workflows/calculation/calculation.cwl", CALCULATION_FILE).unwrap();
    create_and_write_file("workflows/plot/plot.cwl", PLOT_FILE).unwrap();

    let args = CreateWorkflowArgs {
        name: "test".to_string(),
        force: false,
    };
    let result = create_workflow(&args);
    assert!(result.is_ok());

    let connect_args = vec![
        ConnectWorkflowArgs {
            name: "test".to_string(),
            from: "@inputs/speakers".to_string(),
            to: "calculation/speakers".to_string(),
        },
        ConnectWorkflowArgs {
            name: "test".to_string(),
            from: "@inputs/pop".to_string(),
            to: "calculation/population".to_string(),
        },
        ConnectWorkflowArgs {
            name: "test".to_string(),
            from: "calculation/results".to_string(),
            to: "plot/results".to_string(),
        },
        ConnectWorkflowArgs {
            name: "test".to_string(),
            from: "plot/results".to_string(),
            to: "@outputs/out".to_string(),
        },
    ];
    for c in &connect_args {
        let result = connect_workflow_nodes(c);
        assert!(result.is_ok())
    }

    let workflow = load_workflow("workflows/test/test.cwl").unwrap();

    let mut cmd = Command::cargo_bin("s4n")?;
    let mut _output = cmd
        .arg("workflow")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("test"))
        .stdout(predicate::str::contains("calculation").not())
        .get_output()
        .clone();

        let mut _output2 = Command::cargo_bin("s4n")?
        .arg("workflow")
        .arg("list")
        .arg("-a")
        .assert()
        .success()
        .stdout(predicate::str::contains("test"))
        .stdout(predicate::str::contains("pop"))
        .stdout(predicate::str::contains("speakers"))
        .stdout(predicate::str::contains("out"))
        .stdout(predicate::str::contains("calculation"))
        .stdout(predicate::str::contains("plot"))
        .get_output()
        .clone();

    assert!(workflow.has_input("speakers"));
    assert!(workflow.has_input("pop"));
    assert!(workflow.has_output("out"));

    assert!(workflow.has_step("calculation"));
    assert!(workflow.has_step("plot"));

    assert!(workflow.has_step_input("speakers"));
    assert!(workflow.has_step_input("pop"));
    assert!(workflow.has_step_input("calculation/results"));
    assert!(workflow.has_step_output("plot/results"));

    for c in connect_args {
        let result = disconnect_workflow_nodes(&c);
        assert!(result.is_ok());
    }

    // Reload the workflow and validate disconnections
    let workflow = load_workflow("workflows/test/test.cwl").unwrap();

    assert!(!workflow.has_input("speakers"));
    assert!(!workflow.has_input("pop"));
    assert!(!workflow.has_output("out"));

    assert!(workflow.has_step("calculation"));
    assert!(workflow.has_step("plot"));

    assert!(!workflow.has_step_input("speakers"));
    assert!(!workflow.has_step_input("pop"));
    assert!(!workflow.has_step_input("calculation/results"));
    assert!(!workflow.has_step_output("plot/results"));

    env::set_current_dir(current).unwrap();
    
    Ok(())
}


const CALCULATION_FILE: &str = r"#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

requirements:
- class: InitialWorkDirRequirement
  listing:
  - entryname: calculation.py
    entry:
      $include: ../../calculation.py

inputs:
- id: population
  type: File
  default:
    class: File
    location: ../../population.csv
  inputBinding:
    prefix: --population
- id: speakers
  type: File
  default:
    class: File
    location: ../../speakers_revised.csv
  inputBinding:
    prefix: --speakers

outputs:
- id: results
  type: File
  outputBinding:
    glob: results.csv

baseCommand:
- python
- calculation.py
";

const PLOT_FILE: &str = r"#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

requirements:
- class: InitialWorkDirRequirement
  listing:
  - entryname: plot.py
    entry:
      $include: ../../plot.py

inputs:
- id: results
  type: File
  default:
    class: File
    location: ../../results.csv
  inputBinding:
    prefix: --results

outputs:
- id: results
  type: File
  outputBinding:
    glob: results.svg

baseCommand:
- python
- plot.py
";
