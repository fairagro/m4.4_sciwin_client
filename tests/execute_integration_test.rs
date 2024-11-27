use s4n::{
    commands::execute::{execute_local, LocalExecuteArgs, Runner},
    io::copy_dir,
};
use serial_test::serial;
use std::{
    env,
    fs::{self},
    path::Path,
    process::Command,
};
use tempfile::tempdir;

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

#[test]
#[serial]
pub fn test_execute_local_outdir() {
    let dir = tempdir().unwrap();
    let args = LocalExecuteArgs {
        runner: Runner::Custom,
        out_dir: Some(dir.path().to_string_lossy().into_owned()),
        is_quiet: false,
        file: "tests/test_data/echo.cwl".to_string(),
        args: vec![],
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
        runner: Runner::Custom,
        out_dir: None,
        is_quiet: true,
        file: "tests/test_data/echo.cwl".to_string(),
        args: vec![],
    };

    execute_local(&args).expect("Could not execute CommandLineTool");

    let file = Path::new("results.txt");
    assert!(file.exists());
    fs::remove_file(file).unwrap();
}

#[test]
#[serial]
pub fn test_execute_local_cwltool() {
    //as cwltool does not support windows, we can not test that
    if !cfg!(target_os = "windows") {
        let args = LocalExecuteArgs {
            runner: Runner::CWLTool,
            out_dir: None,
            is_quiet: false,
            file: "tests/test_data/echo.cwl".to_string(),
            args: vec![],
        };

        execute_local(&args).expect("Could not execute CommandLineTool");

        let file = Path::new("results.txt");
        assert!(file.exists());
        fs::remove_file(file).unwrap();
    }
}

#[test]
#[serial]
pub fn test_execute_local_workflow() {
    let folder = "./tests/test_data/hello_world";

    let dir = tempdir().unwrap();
    let dir_str = &dir.path().to_string_lossy();
    copy_dir(folder, dir_str).unwrap();

    let current_dir = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    //windows stuff
    let ext = if cfg!(target_os = "windows") { ".exe" } else { "" };
    let path_sep = if cfg!(target_os = "windows") { ";" } else { ":" };
    let venv_scripts = if cfg!(target_os = "windows") { "Scripts" } else { "bin" };

    //set up python venv
    let _ = Command::new("python").arg("-m").arg("venv").arg(".venv").output().unwrap();
    let old_path = env::var("PATH").unwrap();
    let python_path = format!("{}/.venv/{}", dir_str, venv_scripts);
    let new_path = format!("{}{}{}", python_path, path_sep, old_path);
    //modify path variable
    env::set_var("PATH", new_path);

    //install packages
    let req_path = format!("{}/requirements.txt", dir_str);
    let _ = Command::new(python_path + "/pip" + ext).arg("install").arg("-r").arg(req_path).output().unwrap();

    //execute workflow
    let args = LocalExecuteArgs {
        runner: Runner::Custom,
        out_dir: None,
        is_quiet: false,
        file: format!("{}/workflows/main/main.cwl", dir_str),
        args: vec!["inputs.yml".to_string()],
    };
    assert!(execute_local(&args).is_ok());

    //check if file is written which means wf ran completely
    let results_url = format!("{}/results.svg", dir_str);
    let path = Path::new(&results_url);
    assert!(path.exists());

    //reset
    env::set_var("PATH", old_path);
    env::set_current_dir(current_dir).unwrap();
}
