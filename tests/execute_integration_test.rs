use cwl_execution::io::copy_dir;
use s4n::commands::{LocalExecuteArgs, execute_local};
use serial_test::serial;
use std::{
    env,
    fs::{self},
    iter,
    path::{Path, PathBuf},
};
use tempfile::tempdir;

#[test]
#[serial]
pub fn test_execute_local() {
    let args = LocalExecuteArgs {
        file: PathBuf::from("tests/test_data/echo.cwl"),
        ..Default::default()
    };

    execute_local(&args).expect("Could not execute CommandLineTool");

    let file = Path::new("results.txt");
    assert!(file.exists());

    //check file validity
    let contents = fs::read_to_string(file).unwrap();
    let expected = include_str!("test_data/input.txt");

    assert_eq!(contents, expected);

    fs::remove_file(file).unwrap();
}

#[test]
#[serial]
pub fn test_execute_local_with_args() {
    let args = LocalExecuteArgs {
        file: PathBuf::from("tests/test_data/echo.cwl"),
        args: ["--test", "tests/test_data/input_alt.txt"]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ..Default::default()
    };

    execute_local(&args).expect("Could not execute CommandLineTool");

    let file = Path::new("results.txt");
    assert!(file.exists());

    //check file validity
    let contents = fs::read_to_string(file).unwrap();
    let expected = include_str!("test_data/input_alt.txt");

    assert_eq!(contents, expected);

    fs::remove_file(file).unwrap();
}

#[test]
#[serial]
pub fn test_execute_local_with_file() {
    let args = LocalExecuteArgs {
        file: PathBuf::from("tests/test_data/echo.cwl"),
        args: iter::once(&"tests/test_data/echo-job.yml").map(ToString::to_string).collect::<Vec<_>>(),
        ..Default::default()
    };

    execute_local(&args).expect("Could not execute CommandLineTool");

    let file = Path::new("results.txt");
    assert!(file.exists());

    //check file validity
    let contents = fs::read_to_string(file).unwrap();
    let expected = include_str!("test_data/input_alt.txt");

    assert_eq!(contents, expected);

    fs::remove_file(file).unwrap();
}

#[test]
#[serial]
pub fn test_execute_local_outdir() {
    let dir = tempdir().unwrap();
    let args = LocalExecuteArgs {
        out_dir: Some(dir.path().to_string_lossy().into_owned()),
        file: PathBuf::from("tests/test_data/echo.cwl"),
        ..Default::default()
    };

    execute_local(&args).expect("Could not execute CommandLineTool");

    let file = dir.path().join("results.txt");
    assert!(file.exists());
    fs::remove_file(file).unwrap();
}

#[test]
#[serial]
pub fn test_execute_local_is_quiet() {
    //does not really test if it is quiet but rather that the process works
    let args = LocalExecuteArgs {
        is_quiet: true,
        file: PathBuf::from("tests/test_data/echo.cwl"),
        ..Default::default()
    };

    execute_local(&args).expect("Could not execute CommandLineTool");

    let file = Path::new("results.txt");
    assert!(file.exists());
    fs::remove_file(file).unwrap();
}

#[test]
#[serial]
//docker not working on MacOS Github Actions
#[cfg_attr(target_os = "macos", ignore)]
pub fn test_execute_local_workflow() {
    let folder = "./tests/test_data/hello_world";

    let dir = tempdir().unwrap();
    let dir_str = &dir.path().to_string_lossy();
    copy_dir(folder, dir.path()).unwrap();

    let current_dir = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();
    //execute workflow
    let args = LocalExecuteArgs {
        file: PathBuf::from(format!("{dir_str}/workflows/main/main.cwl")),
        args: vec!["inputs.yml".to_string()],
        ..Default::default()
    };
    let result = execute_local(&args);
    println!("{result:#?}");
    assert!(result.is_ok());

    //check if file is written which means wf ran completely
    let results_url = format!("{dir_str}/results.svg");
    let path = Path::new(&results_url);
    assert!(path.exists());

    env::set_current_dir(current_dir).unwrap();
}

#[test]
#[serial]
pub fn test_execute_local_tool_default_cwl() {
    let path = PathBuf::from("tests/test_data/default.cwl");
    let dir = tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().into_owned();
    let out_file = format!("{}/file.wtf", &out_dir);

    let args = LocalExecuteArgs {
        out_dir: Some(out_dir.clone()),
        is_quiet: true,
        file: path.clone(),
        ..Default::default()
    };
    let args_override = LocalExecuteArgs {
        out_dir: Some(out_dir),
        is_quiet: true,
        file: path,
        args: vec!["--file1".to_string(), "tests/test_data/input.txt".to_string()],
        ..Default::default()
    };

    assert!(execute_local(&args).is_ok());
    assert!(fs::exists(&out_file).unwrap());
    let contents = fs::read_to_string(&out_file).unwrap();
    assert_eq!(contents, "File".to_string());

    assert!(execute_local(&args_override).is_ok());
    assert!(fs::exists(&out_file).unwrap());
    let contents = fs::read_to_string(&out_file).unwrap();
    assert_eq!(contents, "Hello fellow CWL-enjoyers!".to_string());
}

#[test]
#[serial]
pub fn test_execute_local_workflow_no_steps() {
    //has no steps, do not complain!
    let path = PathBuf::from("tests/test_data/wf_inout.cwl");
    let dir = tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().into_owned();

    let args = LocalExecuteArgs {
        out_dir: Some(out_dir),
        is_quiet: true,
        file: path,
        ..Default::default()
    };

    assert!(execute_local(&args).is_ok());
}

#[test]
#[serial]
pub fn test_execute_local_workflow_in_param() {
    let path = PathBuf::from("tests/test_data/test-wf_features.cwl");
    let dir = tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().into_owned();
    let out_file = format!("{}/file.wtf", &out_dir);

    let args = LocalExecuteArgs {
        out_dir: Some(out_dir),
        is_quiet: true,
        file: path,
        args: vec!["--pop".to_string(), "tests/test_data/input.txt".to_string()],
        ..Default::default()
    };

    assert!(execute_local(&args).is_ok());
    assert!(fs::exists(&out_file).unwrap());
    let contents = fs::read_to_string(&out_file).unwrap();
    assert_eq!(contents, "Hello fellow CWL-enjoyers!".to_string());
}

#[test]
#[serial]
pub fn test_execute_local_workflow_dir_out() {
    //has no steps, do not complain!
    let path = PathBuf::from("tests/test_data/wf_inout_dir.cwl");
    let dir = tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().into_owned();
    let out_path = format!("{}/test_dir", &out_dir);

    let args = LocalExecuteArgs {
        out_dir: Some(out_dir),
        is_quiet: true,
        file: path,
        ..Default::default()
    };

    assert!(execute_local(&args).is_ok());
    assert!(fs::exists(format!("{out_path}/file.txt")).unwrap());
    assert!(fs::exists(format!("{out_path}/input.txt")).unwrap());
}

#[test]
#[serial]
pub fn test_execute_local_workflow_file_out() {
    //has no steps, do not complain!
    let path = PathBuf::from("tests/test_data/wf_inout_file.cwl");
    let dir = tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().into_owned();
    let out_path = format!("{out_dir}/file.txt");

    let args = LocalExecuteArgs {
        out_dir: Some(out_dir),
        is_quiet: true,
        file: path,
        ..Default::default()
    };

    assert!(execute_local(&args).is_ok());
    assert!(fs::exists(out_path).unwrap());
}

#[test]
#[serial]
pub fn test_execute_local_workflow_directory_out() {
    let path = PathBuf::from("tests/test_data/mkdir_wf.cwl");
    let dir = tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().into_owned();

    let args = LocalExecuteArgs {
        out_dir: Some(out_dir),
        is_quiet: true,
        file: path,
        args: vec!["--dirname".to_string(), "test_directory".to_string()],
        ..Default::default()
    };

    assert!(execute_local(&args).is_ok());
}

#[test]
#[serial]
pub fn test_execute_local_with_binary_input() {
    let path = PathBuf::from("tests/test_data/read_bin.cwl");
    let dir = tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().into_owned();
    let out_path = format!("{}/output.txt", &out_dir);

    let args = LocalExecuteArgs {
        out_dir: Some(out_dir),
        is_quiet: true,
        file: path,
        ..Default::default()
    };

    assert!(execute_local(&args).is_ok());
    assert!(fs::exists(&out_path).unwrap());
    let contents = fs::read_to_string(&out_path).unwrap();
    assert_eq!(contents, "69420".to_string());
}
