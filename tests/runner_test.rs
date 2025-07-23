mod common;
use common::with_temp_repository;
use commonwl::{load_tool, CWLDocument, CommandLineTool, DefaultValue};
use cwl_execution::{
    environment::RuntimeEnvironment,
    runner::{run_command, run_tool},
};
use s4n::parser::parse_command_line;
use serial_test::serial;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tempfile::tempdir;

#[test]
#[serial]
pub fn test_cwl_execute_command_multiple() {
    with_temp_repository(|dir| {
        let command = "python scripts/echo.py --test data/input.txt";
        let args = shlex::split(command).expect("parsing failed");
        let cwl = parse_command_line(&args.iter().map(AsRef::as_ref).collect::<Vec<_>>());
        assert!(run_command(&cwl, &mut RuntimeEnvironment::default()).is_ok());

        let output_path = dir.path().join(Path::new("results.txt"));
        assert!(output_path.exists());
    });
}

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
        assert!(run_command(&tool, &mut RuntimeEnvironment::default()).is_ok());

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

        let inputs = serde_yaml::from_str(yml).expect("Input parsing failed");
        let mut runtime = RuntimeEnvironment {
            inputs,
            ..Default::default()
        };
        let tool: CommandLineTool = serde_yaml::from_str(cwl).expect("Tool parsing failed");
        assert!(run_command(&tool, &mut runtime).is_ok());

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
        let mut runtime = RuntimeEnvironment {
            inputs,
            ..Default::default()
        };
        let tool: CommandLineTool = serde_yaml::from_str(cwl).expect("Tool parsing failed");

        let result = run_command(&tool, &mut runtime);
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

    let mut tool: CWLDocument = serde_yaml::from_str(cwl).expect("Tool parsing failed");
    let result = run_tool(&mut tool, &Default::default(), &PathBuf::default(), None);
    assert!(result.is_ok());
    //delete results.txt
    let _ = fs::remove_file("results.txt");
    match result {
        Ok(_) => println!("success!"),
        Err(e) => eprintln!("{e:?}"),
    }
}

#[test]
#[serial]
pub fn test_run_commandlinetool_array_glob() {
    let dir = tempdir().unwrap();
    let mut tool = CWLDocument::CommandLineTool(load_tool("tests/test_data/array_test.cwl").expect("Tool parsing failed"));
    let result = run_tool(
        &mut tool,
        &Default::default(),
        &PathBuf::default(),
        Some(dir.path().to_string_lossy().into_owned()),
    );
    assert!(result.is_ok(), "{result:?}");
}
