mod common;
use common::with_temp_repository;
use s4n::cwl::{
    clt::{CommandLineTool, DefaultValue},
    runner::run_command_line_tool,
};
use serial_test::serial;
use std::{collections::HashMap, fs};

#[test]
#[serial]
pub fn test_run_command_line_tool_simple() {
    with_temp_repository(|dir| {
        let cwl = r#"
#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

inputs:
- id: message
  type: string
  default: "Hello CWL"
  inputBinding:
    position: 0

baseCommand: echo

stdout: output.txt

outputs: 
- id: output
  type: File
  glob: output.txt

"#;
        let tool: CommandLineTool = serde_yml::from_str(cwl).expect("Tool parsing failed");
        assert!(run_command_line_tool(tool, None).is_ok());

        let output = dir.path().join("output.txt");
        assert!(output.exists());
        let contents = fs::read_to_string(output).expect("Could not read output");
        assert_eq!(contents.trim(), "Hello CWL")
    });
}

#[test]
#[serial]
pub fn test_run_command_line_tool_simple_with_args() {
    with_temp_repository(|dir| {
        let cwl = r#"
#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

inputs:
- id: message
  type: string
  default: "Hello CWL"
  inputBinding:
    position: 0

baseCommand: echo

stdout: output.txt

outputs: 
- id: output
  type: File
  glob: output.txt

"#;

        let yml = "message: \"Hello World\"";

        let inputs: HashMap<String, DefaultValue> = serde_yml::from_str(yml).expect("Input parsing failed");

        let tool: CommandLineTool = serde_yml::from_str(cwl).expect("Tool parsing failed");
        assert!(run_command_line_tool(tool, Some(inputs)).is_ok());

        let output = dir.path().join("output.txt");
        assert!(output.exists());
        let contents = fs::read_to_string(output).expect("Could not read output");
        assert_eq!(contents.trim(), "Hello World")
    });
}
