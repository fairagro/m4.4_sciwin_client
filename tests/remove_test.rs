#![allow(clippy::disallowed_macros)]
use assert_cmd::Command;
use cwl_execution::io::create_and_write_file;
use git2::Repository;
use predicates::prelude::*;
use s4n::commands::*;
use s4n::util::repo::get_modified_files;
use serial_test::serial;
use std::env;
use std::path::Path;
use std::{fs, vec};
use tempfile::tempdir;
use test_utils::with_temp_repository;

#[test]
#[serial]
fn test_remove_non_existing_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let workflows_path = temp_dir.path().join("workflows");
    let original_dir = env::current_dir()?;
    fs::create_dir(&workflows_path)?;
    //doesn't exist
    let args = RemoveCWLArgs {
        file: "non_existing_tool".to_string(),
    };

    let result = handle_remove_command(&args);

    assert!(result.is_ok(), "Function should handle non-existing tool");
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

        let args = RemoveCWLArgs { file: "echo".to_string() };
        let cmd_remove = handle_remove_command(&args);
        assert!(cmd_remove.is_ok(), "Removing tool should succeed");

        assert!(!dir.path().join(Path::new("workflows/echo/echo.cwl")).exists());
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
        let tool_remove_args = RemoveCWLArgs {
            file: "echo.cwl".to_string(),
        };
        assert!(handle_remove_command(&tool_remove_args).is_ok());

        // check if the tool was removed
        assert!(!dir.path().join(Path::new("workflows/echo/echo.cwl")).exists());
        assert!(!dir.path().join(Path::new("workflows/echo")).exists());

        // check if there are no uncommitted changes after removal
        let repo = Repository::open(dir.path()).unwrap();
        assert!(get_modified_files(&repo).is_empty());
    });
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
