use s4n::{
    cwl::{parser::guess_type, types::CWLType},
    io::{get_filename_without_extension, resolve_path},
};

#[test]
pub fn test_filename_without_extension() {
    let inputs = &["results.csv", "/some/relative/path.txt", "some/archive.tar.gz"];
    let outputs = &["results", "path", "archive"];

    for i in 0..inputs.len() {
        let result = get_filename_without_extension(inputs[i]).expect("operation failed");
        assert_eq!(result, outputs[i]);
    }
}

#[test]
pub fn test_cwl_type_inference() {
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
fn test_path_resolver() {
    let test_cases = &[
        ("tests/testdata/input.txt", "workflows/echo/echo.cwl", "../../tests/testdata/input.txt"),
        ("tests/testdata/input.txt", "workflows/echo/", "../../tests/testdata/input.txt"),
        ("workflows/echo/echo.py", "workflows/echo/echo.cwl", "echo.py"),
        ("workflows/lol/echo.py", "workflows/echo/echo.cwl", "../lol/echo.py"),
        ("/home/user/workflows/echo/echo.py", "/home/user/workflows/echo/echo.cwl", "echo.py"),
    ];
    for (path, relative_to, expected) in test_cases {
        let actual = resolve_path(path, relative_to);
        assert_eq!(actual, *expected);
    }
}
