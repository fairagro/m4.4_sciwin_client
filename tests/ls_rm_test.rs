use assert_cmd::Command;
use predicates::prelude::*;
use s4n::commands::tool::{remove_tool, RmArgs, CreateToolArgs, handle_tool_commands, ToolCommands};
use serial_test::serial;
use std::env;
use std::fs::File;
use std::{fs, vec};
use tempfile::tempdir;
use s4n::repo::{get_modified_files, open_repo};
mod common;
use common::with_temp_repository;
use std::io::Write;

#[test]
fn test_remove_non_existing_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let workflows_path = temp_dir.path().join("workflows");
    let original_dir = env::current_dir()?;
    fs::create_dir(&workflows_path)?;
    //doesn't exist
    let args = RmArgs {
        rm_tool: vec!["non_existing_tool".to_string()],
    };

    // Call remove_tool and verify no directory was removed
    let result = remove_tool(&args);

    // Check that the function executed without error, even though the tool doesn't exist
    assert!(result.is_ok(), "Function should handle non-existing tool gracefully");
    env::set_current_dir(&original_dir)?;
    Ok(())
}

#[test]
fn test_remove_empty_tool_list() -> Result<(), Box<dyn std::error::Error>> {
    let args = RmArgs { rm_tool: vec![] };
    let original_dir = env::current_dir()?;
    let output = std::panic::catch_unwind(|| {
        remove_tool(&args).unwrap();
    });
    // Assert that the function ran successfully
    assert!(output.is_ok(), "Function should handle empty tool list gracefully");
    env::set_current_dir(&original_dir)?;
    Ok(())
}

#[test]
#[serial]
pub fn tool_remove_test() {
    with_temp_repository(|dir| {
        // Create a tool to be removed later
        let tool_create_args = CreateToolArgs {
            name: Some("echo".to_string()), // tool name
            container_image: None,
            container_tag: None,
            is_raw: false,
            no_commit: false,
            no_run: false,
            is_clean: false,
            command: vec!["python".to_string(), "scripts/echo.py".to_string(), "--test".to_string(), "data/input.txt".to_string()],
        };
        let cmd_create = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd_create).is_ok());

        // Check if the tool was created
        assert!(dir.path().join("workflows/echo").exists()); // tool folder created

        // Now remove the tool
        let tool_remove_args = RmArgs {
            rm_tool: vec!["echo".to_string()], // tool to remove
        };
        let cmd_remove = ToolCommands::Rm(tool_remove_args);
        assert!(handle_tool_commands(&cmd_remove).is_ok());

        // Check if the tool was removed
        assert!(!dir.path().join("workflows/echo").exists()); // tool folder should be removed

        // Check if there are no uncommitted changes after removal
        let repo = open_repo(dir.path());
        assert!(get_modified_files(&repo).is_empty());
    });
}

#[test]
#[serial]
pub fn tool_remove_test_extension() {
    with_temp_repository(|dir| {
        // Create a tool to be removed later
        let tool_create_args = CreateToolArgs {
            name: Some("echo".to_string()), // tool name
            container_image: None,
            container_tag: None,
            is_raw: false,
            no_commit: false,
            no_run: false,
            is_clean: false,
            command: vec!["python".to_string(), "scripts/echo.py".to_string(), "--test".to_string(), "data/input.txt".to_string()],
        };
        let cmd_create = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd_create).is_ok());

        // Check if the tool was created
        assert!(dir.path().join("workflows/echo").exists()); // tool folder created

        // Now remove the tool
        let tool_remove_args = RmArgs {
            rm_tool: vec!["echo.cwl".to_string()], // tool to remove
        };
        let cmd_remove = ToolCommands::Rm(tool_remove_args);
        assert!(handle_tool_commands(&cmd_remove).is_ok());

        // Check if the tool was removed
        assert!(!dir.path().join("workflows/echo").exists()); // tool folder should be removed

        // Check if there are no uncommitted changes after removal
        let repo = open_repo(dir.path());
        assert!(get_modified_files(&repo).is_empty());
    });
}


#[test]
#[serial]
fn test_list_tools() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    println!("Temporary directory created at: {:?}", dir.path());

    fs::create_dir_all(dir.path().join("workflows"))?;
    fs::create_dir_all(dir.path().join("workflows").join("test1"))?;
    fs::create_dir_all(dir.path().join("workflows").join("test2"))?;
    fs::create_dir_all(dir.path().join("workflows").join("test3"))?;

    File::create(dir.path().join("workflows").join("test1/test1.cwl"))?;
    File::create(dir.path().join("workflows").join("test2/test2.cwl"))?;
    File::create(dir.path().join("workflows").join("test3/other_file.txt"))?;

    assert!(dir.path().join("workflows").join("test1/test1.cwl").exists(), "test1.cwl was not created!");
    assert!(dir.path().join("workflows").join("test2/test2.cwl").exists(), "test2.cwl was not created!");
    assert!(dir.path().join("workflows").join("test3/other_file.txt").exists(), "other_file.txt was not created!");

    let original_dir = env::current_dir()?;
    env::set_current_dir(dir.path())?;

    let mut cmd = Command::cargo_bin("s4n")?;
    let output = cmd
        .arg("tool")
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("test1.cwl"))
        .stdout(predicate::str::contains("test2.cwl"))
        .stdout(predicate::str::contains("other_file.txt").not())
        .get_output()
        .clone();

    println!("Command Output: {}", String::from_utf8_lossy(&output.stdout));

    env::set_current_dir(original_dir)?;

    Ok(())
}


#[test]
#[serial]
fn test_list_tools_with_list_all() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory for the test
    let temp_dir = tempdir()?;
    let workflows_dir = temp_dir.path().join("workflows");
    fs::create_dir(&workflows_dir)?;

    // Create dummy CWL files
    let cwl_content_1 = r#"
    inputs:
      - id: speakers
        type: string
      - id: population
        type: int
    outputs:
      - id: results
        type: File
    "#;
    let cwl_content_2 = r#"
    inputs:
      - id: data
        type: File
    outputs:
      - id: chart
        type: File
    "#;

    let cwl_file_1 = workflows_dir.join("calculation.cwl");
    let cwl_file_2 = workflows_dir.join("plot.cwl");

    {
        let mut file = File::create(&cwl_file_1)?;
        file.write_all(cwl_content_1.as_bytes())?;
    }
    {
        let mut file = File::create(&cwl_file_2)?;
        file.write_all(cwl_content_2.as_bytes())?;
    }

    // Set the current directory to the temporary directory
    let original_cwd = env::current_dir()?;
    env::set_current_dir(&temp_dir)?;

    let mut cmd = Command::cargo_bin("s4n")?;
    let _output = cmd
        .arg("tool")
        .arg("ls")
        .arg("-a")
        .assert()
        .success()
        .stdout(predicate::str::contains("calculation.cwl"))
        .stdout(predicate::str::contains("speakers"))
        .stdout(predicate::str::contains("population"))
        .stdout(predicate::str::contains("plot.cwl"))
        .stdout(predicate::str::contains("results"))
        .get_output()
        .clone();

    // Restore the original working directory
    env::set_current_dir(original_cwd)?;

  

    Ok(())
}
