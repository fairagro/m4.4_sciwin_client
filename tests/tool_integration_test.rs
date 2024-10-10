mod common;
use common::with_temp_repository;
use s4n::commands::tool::{handle_tool_commands, CreateToolArgs, ToolCommands};
use serial_test::serial;
use std::path::Path;

#[test]
#[serial]
pub fn tool_create_test() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            name: None,
            container_image: None,
            container_tag: None,
            is_raw: false,
            no_commit: false,
            no_run: false,
            is_clean: false,
            command: vec!["python".to_string(), "scripts/echo.py".to_string(), "--test".to_string(), "data/input.txt".to_string()],
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());

        //check for files being present
        let output_paths = vec![dir.path().join(Path::new("results.txt")), dir.path().join(Path::new("workflows/echo/echo.cwl"))];
        for output_path in output_paths {
            assert!(output_path.exists());
        }
    });
}
