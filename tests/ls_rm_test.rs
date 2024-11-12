use s4n::commands::tool::{remove_tool, ToolArgs};
use serial_test::serial;
use std::env;
use std::fs::create_dir_all;
use std::io;
use std::{fs, vec};
use tempfile::tempdir;

#[test]
fn test_remove_non_existing_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let workflows_path = temp_dir.path().join("workflows");
    fs::create_dir(&workflows_path)?;
    //doesn't exist
    let args = ToolArgs {
        tool: vec!["non_existing_tool".to_string()],
    };

    // Call remove_tool and verify no directory was removed
    let result = remove_tool(&args);

    // Check that the function executed without error, even though the tool doesn't exist
    assert!(result.is_ok(), "Function should handle non-existing tool gracefully");

    Ok(())
}

#[test]
fn test_empty_tool_list() -> Result<(), Box<dyn std::error::Error>> {
    let args = ToolArgs { tool: vec![] };

    let output = std::panic::catch_unwind(|| {
        remove_tool(&args).unwrap();
    });
    // Assert that the function ran successfully
    assert!(output.is_ok(), "Function should handle empty tool list gracefully");

    Ok(())
}

#[test]
#[serial]
fn test_remove_existing_tool_directory() -> io::Result<()> {
    let temp_dir = env::temp_dir().join("rm_existing");
    let workflows_path = temp_dir.as_path().join("workflows");
    let tool_name = "example_tool";
    let tool_path = workflows_path.join(tool_name);
    create_dir_all(&tool_path)?;
    fs::File::create(tool_path.join("example_tool.cwl"))?;

    env::set_current_dir(temp_dir.clone()).unwrap();
    let args = ToolArgs { tool: vec![tool_name.to_string()] };
    let result = remove_tool(&args);

    assert!(result.is_ok());
    //assert that it was deleted
    assert!(!tool_path.exists());

    Ok(())
}

#[test]
#[serial]
fn test_remove_tool_with_extension() -> io::Result<()> {
    let temp_dir = env::temp_dir().join("rm_extension");
    println!("Temporary directory: {}", temp_dir.display());

    let workflows_path = temp_dir.as_path().join("workflows");
    let tool_name = "tool_with_ext.cwl";
    let tool_path = workflows_path.join("tool_with_ext");

    create_dir_all(&tool_path)?;
    fs::File::create(tool_path.join("tool_with_ext.cwl"))?;
    env::set_current_dir(temp_dir.clone()).unwrap();

    let args = ToolArgs { tool: vec![tool_name.to_string()] };

    let result = remove_tool(&args);

    assert!(result.is_ok());
    assert!(!tool_path.exists());

    Ok(())
}
