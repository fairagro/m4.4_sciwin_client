use assert_cmd::Command;
use git2::Repository;
use predicates::prelude::*;
use s4n::commands::tool::{remove_tool, ToolArgs};
use serial_test::serial;
use std::env;
use std::fs::{create_dir_all, File};
use std::{fs, vec};
use tempfile::tempdir;

#[test]
fn test_remove_non_existing_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let workflows_path = temp_dir.path().join("workflows");
    let original_dir = env::current_dir()?;
    fs::create_dir(&workflows_path)?;
    //doesn't exist
    let args = ToolArgs {
        tool: vec!["non_existing_tool".to_string()],
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
    let args = ToolArgs { tool: vec![] };
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
fn test_remove_existing_tool_directory() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = env::temp_dir().join("rm_existing");
    let workflows_path = temp_dir.as_path().join("workflows");
    let tool_name = "example_tool";
    let tool_path = workflows_path.join(tool_name);
    let original_dir = env::current_dir()?;

    // Initialize a repository
    create_dir_all(&temp_dir)?;

    let _repo = match Repository::init(&temp_dir) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to initialize repository: {}", e),
    };

    create_dir_all(&tool_path)?;
    fs::File::create(tool_path.join("example_tool.cwl"))?;
    env::set_current_dir(temp_dir.clone()).unwrap();

    let args = ToolArgs { tool: vec![tool_name.to_string()] };
    let result = remove_tool(&args);

    assert!(result.is_ok());
    assert!(!tool_path.exists());
    env::set_current_dir(&original_dir)?;
    Ok(())
}

#[test]
#[serial]
fn test_remove_tool_with_extension() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = env::temp_dir().join("rm_extension");
    println!("Temporary directory: {}", temp_dir.display());
    let workflows_path = temp_dir.as_path().join("workflows");
    let tool_name = "tool_with_ext.cwl";
    let tool_path = workflows_path.join("tool_with_ext");
    let original_dir = env::current_dir()?;

    // Initialize a repository
    create_dir_all(&temp_dir)?;
    let _repo = match Repository::init(&temp_dir) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to initialize repository: {}", e),
    };

    create_dir_all(&tool_path)?;
    fs::File::create(tool_path.join("tool_with_ext.cwl"))?;
    env::set_current_dir(temp_dir.clone()).unwrap();

    let args = ToolArgs { tool: vec![tool_name.to_string()] };

    let result = remove_tool(&args);

    assert!(result.is_ok());
    assert!(!tool_path.exists());
    env::set_current_dir(&original_dir)?;
    Ok(())
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
