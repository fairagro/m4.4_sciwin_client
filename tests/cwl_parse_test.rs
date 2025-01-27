use cwl::{
    clt::{Command, CommandLineTool},
    inputs::{CommandInputParameter, CommandLineBinding},
    types::CWLType,
};

#[test]
pub fn test_simple_cwl() {
    let cwl = r"#!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

requirements:
- class: InitialWorkDirRequirement
  listing:
  - entryname: calculation.py
    entry:
      $include: calculation.py

inputs:
- id: population
  type: File
  default:
    class: File
    location: ../../assays/population/dataset/population.csv
  inputBinding:
    prefix: -p
- id: speakers
  type: File
  default:
    class: File
    location: ../../assays/speakers/dataset/speakers_revised.csv
  inputBinding:
    prefix: -s

outputs:
- id: output
  type: File
  outputBinding:
    glob: results.csv

baseCommand:
- python
- calculation.py
";
    let clt: Result<CommandLineTool, serde_yml::Error> = serde_yml::from_str(cwl);
    println!("{clt:?}");
    assert!(clt.is_ok());
}

#[test]
pub fn create_simple_cwl() {
    let tool = CommandLineTool::default()
        .with_base_command(Command::Single("ls".to_string()))
        .with_inputs(vec![CommandInputParameter {
            id: "la".to_string(),
            type_: CWLType::Boolean,
            input_binding: Some(CommandLineBinding {
                prefix: Some("-la".to_string()),
                position: None,
                value_from: None,
                shell_quote: None,
            }),
            default: None,
            format: None,
        }])
        .with_outputs(vec![]);
    let result = serde_yml::to_string(&tool);
    println!("{result:?}");
    assert!(result.is_ok());
}
