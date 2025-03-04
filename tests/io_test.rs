mod common;
use common::os_path;
use cwl::{clt::Command, types::CWLType};
use cwl_execution::io::get_filename_without_extension;
use s4n::{
    parser::guess_type,
    io::{get_qualified_filename, get_workflows_folder, resolve_path},
};
use std::vec;

#[test]
pub fn test_get_filename_without_extension() {
    let inputs = &["results.csv", "/some/relative/path.txt", "some/archive.tar.gz"];
    let outputs = &["results", "path", "archive"];

    for i in 0..inputs.len() {
        let result = get_filename_without_extension(inputs[i]).expect("operation failed");
        assert_eq!(result, outputs[i]);
    }
}

#[test]
pub fn test_guess_type() {
    let inputs = &[
        ("./README.md", CWLType::File),
        ("/some/path/that/does/not/exist.txt", CWLType::String),
        ("src/", CWLType::Directory),
        ("--option", CWLType::String),
        ("2", CWLType::Int),
        ("1.5", CWLType::Float),
    ];

    for input in inputs {
        let t = guess_type(input.0);
        println!("{:?}=>{:?}", input.0, input.1);
        assert_eq!(t, input.1);
    }
}

#[test]
pub fn test_get_workflows_folder() {
    //could be variable in future
    assert_eq!(get_workflows_folder(), "workflows/");
}

#[test]
fn test_resolve_path() {
    let test_cases = &[
        ("tests/testdata/input.txt", "workflows/echo/echo.cwl", "../../tests/testdata/input.txt"),
        ("tests/testdata/input.txt", "workflows/echo/", "../../tests/testdata/input.txt"),
        ("workflows/echo/echo.py", "workflows/echo/echo.cwl", "echo.py"),
        ("workflows/lol/echo.py", "workflows/echo/echo.cwl", "../lol/echo.py"),
        ("/home/user/workflows/echo/echo.py", "/home/user/workflows/echo/echo.cwl", "echo.py"),
    ];
    for (path, relative_to, expected) in test_cases {
        let actual = resolve_path(path, relative_to);
        assert_eq!(actual, os_path(expected));
    }
}

#[test]
fn test_get_qualified_filename() {
    let command_multiple = Command::Multiple(vec!["python".to_string(), "test/data/script.py".to_string()]);
    let command_single = Command::Single("echo".to_string());
    let name = "hello";

    let result_name = get_qualified_filename(&command_single, Some(name.to_string()));
    let result_single = get_qualified_filename(&command_single, None);
    let result_multiple = get_qualified_filename(&command_multiple, None);

    assert_eq!(result_name, "workflows/hello/hello.cwl");
    assert_eq!(result_single, "workflows/echo/echo.cwl");
    assert_eq!(result_multiple, "workflows/script/script.cwl");
}
