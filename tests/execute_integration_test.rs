use s4n::commands::execute::{execute_local, LocalExecuteArgs, Runner};
use serial_test::serial;
use std::{fs, path::Path};

#[test]
#[serial]
pub fn test_execute_local() {
    let args = LocalExecuteArgs {
        runner: Runner::Custom,
        out_dir: None,
        is_quiet: false,
        file: "tests/test_data/echo.cwl".to_string(),
        args: vec![],
    };

    execute_local(&args).expect("Could not execute CommandLineTool");

    let file = Path::new("results.txt");
    assert!(file.exists());

    //check file validity
    let contents = fs::read_to_string(file).unwrap();
    let expected = fs::read_to_string("tests/test_data/input.txt").unwrap();

    assert_eq!(contents, expected);

    fs::remove_file(file).unwrap();
}

#[test]
#[serial]
pub fn test_execute_local_with_args() {
    let args = LocalExecuteArgs {
        runner: Runner::Custom,
        out_dir: None,
        is_quiet: false,
        file: "tests/test_data/echo.cwl".to_string(),
        args: ["--test", "tests/test_data/input_alt.txt"].iter().map(|a| a.to_string()).collect::<Vec<_>>(),
    };

    execute_local(&args).expect("Could not execute CommandLineTool");

    let file = Path::new("results.txt");
    assert!(file.exists());

    //check file validity
    let contents = fs::read_to_string(file).unwrap();
    let expected = fs::read_to_string("tests/test_data/input_alt.txt").unwrap();

    assert_eq!(contents, expected);

    fs::remove_file(file).unwrap();
}

#[test]
#[serial]
pub fn test_execute_local_with_file() {
    let args = LocalExecuteArgs {
        runner: Runner::Custom,
        out_dir: None,
        is_quiet: false,
        file: "tests/test_data/echo.cwl".to_string(),
        args: ["tests/test_data/echo-job.yml"].iter().map(|a| a.to_string()).collect::<Vec<_>>(),
    };

    execute_local(&args).expect("Could not execute CommandLineTool");

    let file = Path::new("results.txt");
    assert!(file.exists());

    //check file validity
    let contents = fs::read_to_string(file).unwrap();
    let expected = fs::read_to_string("tests/test_data/input_alt.txt").unwrap();

    assert_eq!(contents, expected);
    
    fs::remove_file(file).unwrap();
}