mod common;
use common::with_temp_repository;
use cwl::{clt::CommandLineTool, types::DefaultValue};
use s4n::execution::runner::{run_command, run_commandlinetool};
use serial_test::serial;
use std::{collections::HashMap, fs};

#[test]
#[serial]
pub fn test_run_command_simple() {
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
        let tool: CommandLineTool = serde_yaml::from_str(cwl).expect("Tool parsing failed");
        assert!(run_command(&tool, None).is_ok());

        let output = dir.path().join("output.txt");
        assert!(output.exists());
        let contents = fs::read_to_string(output).expect("Could not read output");
        assert_eq!(contents.trim(), "Hello CWL");
    });
}

#[test]
#[serial]
pub fn test_run_command_simple_with_args() {
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

        let inputs: HashMap<String, DefaultValue> = serde_yaml::from_str(yml).expect("Input parsing failed");

        let tool: CommandLineTool = serde_yaml::from_str(cwl).expect("Tool parsing failed");
        assert!(run_command(&tool, Some(inputs)).is_ok());

        let output = dir.path().join("output.txt");
        assert!(output.exists());
        let contents = fs::read_to_string(output).expect("Could not read output");
        assert_eq!(contents.trim(), "Hello World");
    });
}

#[test]
#[serial]
pub fn test_run_command_mismatching_args() {
    with_temp_repository(|_| {
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

        let yml = r"
message:
  class: File
  location: whale.txt
  ";

        let inputs: HashMap<String, DefaultValue> = serde_yaml::from_str(yml).expect("Input parsing failed");

        let tool: CommandLineTool = serde_yaml::from_str(cwl).expect("Tool parsing failed");

        let result = run_command(&tool, Some(inputs));
        assert!(result.is_err());
    });
}

#[test]
#[serial]
pub fn test_run_commandlinetool() {
    let cwl = r"
#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

requirements:
- class: InitialWorkDirRequirement
  listing:
  - entryname: tests/test_data/echo.py
    entry:
      $include: tests/test_data/echo.py

inputs:
- id: test
  type: File
  default:
    class: File
    location: tests/test_data/input.txt
  inputBinding:
    prefix: '--test'

outputs:
- id: results
  type: File
  outputBinding:
    glob: results.txt

baseCommand:
- python
- tests/test_data/echo.py
";

    let mut tool: CommandLineTool = serde_yaml::from_str(cwl).expect("Tool parsing failed");
    let result = run_commandlinetool(&mut tool, None, None, None);
    assert!(result.is_ok());
    //delete results.txt
    let _ = fs::remove_file("results.txt");
    match result {
        Ok(_) => println!("success!"),
        Err(e) => eprintln!("{e:?}"),
    }
}
