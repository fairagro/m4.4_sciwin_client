#![allow(clippy::disallowed_macros)]
use git2::Repository;
use s4n::commands::*;
use s4n::{cli::Commands, repo::get_modified_files};
use serial_test::serial;
use std::path::Path;
use test_utils::with_temp_repository;

#[test]
#[serial]
fn test_remove_non_existing_tool() {
    let args = RemoveCWLArgs {
        file: "non_existing_tool".to_string(),
    };

    let result = handle_remove_command(&args);
    assert!(result.is_err());
}

#[test]
#[serial]
pub fn tool_remove_test() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateArgs {
            name: Some("echo".to_string()),
            command: vec![
                "python".to_string(),
                "scripts/echo.py".to_string(),
                "--test".to_string(),
                "data/input.txt".to_string(),
            ],
            ..Default::default()
        };
        let cmd_create = Commands::Create(tool_create_args);
        if let Commands::Create(ref args) = cmd_create {
            assert!(handle_create_command(args).is_ok());
        }
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
        let tool_create_args = CreateArgs {
            name: Some("echo".to_string()),
            command: vec![
                "python".to_string(),
                "scripts/echo.py".to_string(),
                "--test".to_string(),
                "data/input.txt".to_string(),
            ],
            ..Default::default()
        };
        let cmd_create = Commands::Create(tool_create_args);
        if let Commands::Create(ref args) = cmd_create {
            assert!(handle_create_command(args).is_ok());
        }
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
