use assert_cmd::Command;
use cwl_execution::io::create_and_write_file;
use git2::Repository;
use predicates::prelude::*;
use s4n::commands::tool::{handle_tool_commands, remove_tool, CreateToolArgs, RemoveToolArgs, ToolCommands};
use s4n::repo::get_modified_files;
use serial_test::serial;
use std::env;
use std::{fs, vec};
use tempfile::tempdir;
mod common;
use common::with_temp_repository;
use std::path::Path;

#[test]
#[serial]
fn test_remove_non_existing_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let workflows_path = temp_dir.path().join("workflows");
    let original_dir = env::current_dir()?;
    fs::create_dir(&workflows_path)?;
    //doesn't exist
    let args = RemoveToolArgs {
        tool_names: vec!["non_existing_tool".to_string()],
    };

    let result = remove_tool(&args);

    assert!(result.is_ok(), "Function should handle non-existing tool");
    env::set_current_dir(&original_dir)?;
    Ok(())
}

#[test]
#[serial]
fn test_remove_empty_tool_list() -> Result<(), Box<dyn std::error::Error>> {
    let args = RemoveToolArgs { tool_names: vec![] };
    let original_dir = env::current_dir()?;
    let output = std::panic::catch_unwind(|| {
        remove_tool(&args).unwrap();
    });
    assert!(output.is_ok(), "Function should handle empty tool list");
    env::set_current_dir(&original_dir)?;
    Ok(())
}

#[test]
#[serial]
pub fn tool_remove_test() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            name: Some("echo".to_string()),
            command: vec![
                "python".to_string(),
                "scripts/echo.py".to_string(),
                "--test".to_string(),
                "data/input.txt".to_string(),
            ],
            ..Default::default()
        };
        let cmd_create = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd_create).is_ok());
        assert!(dir.path().join(Path::new("workflows/echo")).exists());

        let tool_remove_args = RemoveToolArgs {
            tool_names: vec!["echo".to_string()],
        };
        let cmd_remove = ToolCommands::Remove(tool_remove_args);
        assert!(handle_tool_commands(&cmd_remove).is_ok());
        assert!(!dir.path().join(Path::new("workflows/echo")).exists());

        let repo = Repository::open(dir.path()).unwrap();
        assert!(get_modified_files(&repo).is_empty());
    });
}

#[test]
#[serial]
pub fn tool_remove_test_extension() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            name: Some("echo".to_string()),
            command: vec![
                "python".to_string(),
                "scripts/echo.py".to_string(),
                "--test".to_string(),
                "data/input.txt".to_string(),
            ],
            ..Default::default()
        };
        let cmd_create = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd_create).is_ok());
        assert!(dir.path().join(Path::new("workflows/echo")).exists());

        // remove the tool
        let tool_remove_args = RemoveToolArgs {
            tool_names: vec!["echo.cwl".to_string()],
        };
        let cmd_remove = ToolCommands::Remove(tool_remove_args);
        assert!(handle_tool_commands(&cmd_remove).is_ok());

        // check if the tool was removed
        assert!(!dir.path().join(Path::new("workflows/echo")).exists());

        // check if there are no uncommitted changes after removal
        let repo = Repository::open(dir.path()).unwrap();
        assert!(get_modified_files(&repo).is_empty());
    });
}

#[test]
#[serial]
fn test_list_tools() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir().unwrap();
    let original_cwd = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    create_and_write_file("workflows/calculation/calculation.cwl", CALCULATION_FILE).unwrap();
    create_and_write_file("workflows/plot/plot.cwl", PLOT_FILE).unwrap();

    let mut cmd = Command::cargo_bin("s4n")?;
    let _output_all = cmd
        .arg("tool")
        .arg("ls")
        .arg("-a")
        .assert()
        .success()
        .stdout(predicate::str::contains("calculation"))
        .stdout(predicate::str::contains("calculation/speakers"))
        .stdout(predicate::str::contains("calculation/population"))
        .stdout(predicate::str::contains("calculation/results"))
        .stdout(predicate::str::contains("plot"))
        .stdout(predicate::str::contains("plot/results"))
        .get_output()
        .clone();

    let mut cmd = Command::cargo_bin("s4n")?;
    let _output_tools = cmd
        .arg("tool")
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("calculation"))
        .stdout(predicate::str::contains("plot"))
        .stdout(predicate::str::contains("calculation/speakers").not())
        .stdout(predicate::str::contains("calculation/population").not())
        .stdout(predicate::str::contains("calculation/results").not())
        .stdout(predicate::str::contains("plot/results").not())
        .get_output()
        .clone();

    env::set_current_dir(original_cwd)?;

    Ok(())
}
const CALCULATION_FILE: &str = r#"#!/usr/bin/env cwl-runner

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
"#;

const PLOT_FILE: &str = r#"#!/usr/bin/env cwl-runner

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
"#;
