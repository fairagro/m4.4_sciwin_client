///This file contains all examples described here: https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/
mod common;
use common::{check_git_user, setup_python};
use cwl::{clt::Command, load_tool, requirements::Requirement, types::Entry};
use s4n::{
    commands::{
        init::init_s4n,
        tool::{create_tool, CreateToolArgs},
    },
    io::copy_dir,
};
use serial_test::serial;
use std::{env, fs, path::PathBuf, vec};
use tempfile::{tempdir, TempDir};

fn setup() -> (PathBuf, TempDir) {
    let dir = tempdir().unwrap();

    //copy docs dit to tmp
    let test_folder = "tests/test_data/docs";
    copy_dir(test_folder, dir.path()).unwrap();

    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    //init
    check_git_user().unwrap();
    init_s4n(None, false).expect("Could not init s4n");

    (current, dir)
}

fn cleanup(current: PathBuf, dir: TempDir) {
    env::set_current_dir(current).unwrap();
    dir.close().unwrap()
}

#[test]
#[serial]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#wrapping-echo
pub fn test_wrapping_echo() {
    let (current, dir) = setup();

    let command = &["echo", "\"Hello World\""];

    let args = &CreateToolArgs {
        name: None,
        container_image: None,
        container_tag: None,
        is_raw: false,
        no_commit: false,
        no_run: false,
        is_clean: false,
        inputs: None,
        outputs: None,
        command: command.iter().map(|&s| s.to_string()).collect(),
    };
    assert!(create_tool(args).is_ok());

    let tool_path = dir.path().join("workflows/echo/echo.cwl");
    assert!(fs::exists(&tool_path).unwrap());

    let tool = load_tool(&tool_path).unwrap();
    assert_eq!(tool.base_command, Command::Single("echo".to_string()));
    assert_eq!(tool.inputs.len(), 1);

    cleanup(current, dir);
}

#[test]
#[serial]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#wrapping-echo
pub fn test_wrapping_echo_2() {
    let (current, dir) = setup();

    let command = &["echo", "\"Hello World\"", ">", "hello.yaml"];

    let name = "echo2";
    let args = &CreateToolArgs {
        name: Some(name.to_string()),
        container_image: None,
        container_tag: None,
        is_raw: false,
        no_commit: false,
        no_run: false,
        is_clean: false,
        inputs: None,
        outputs: None,
        command: command.iter().map(|&s| s.to_string()).collect(),
    };
    assert!(create_tool(args).is_ok());

    let tool_path = dir.path().join(format!("workflows/{name}/{name}.cwl"));
    assert!(fs::exists(&tool_path).unwrap());

    let tool = load_tool(&tool_path).unwrap();
    assert_eq!(tool.base_command, Command::Single("echo".to_string()));
    assert_eq!(tool.inputs.len(), 1);
    assert_eq!(tool.outputs.len(), 1);
    assert_eq!(tool.stdout, Some("hello.yaml".to_string()));

    cleanup(current, dir);
}

#[test]
#[serial]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#wrapping-a-python-script
pub fn test_wrapping_python_script() {
    let (current, dir) = setup();

    let command = &["python", "echo.py", "--message", "SciWIn rocks!", "--output-file", "out.txt"];

    let name = "echo_python";
    let args = &CreateToolArgs {
        name: Some(name.to_string()),
        container_image: None,
        container_tag: None,
        is_raw: false,
        no_commit: false,
        no_run: false,
        is_clean: false,
        inputs: None,
        outputs: None,
        command: command.iter().map(|&s| s.to_string()).collect(),
    };
    assert!(create_tool(args).is_ok());

    let tool_path = dir.path().join(format!("workflows/{name}/{name}.cwl"));
    assert!(fs::exists(&tool_path).unwrap());

    let tool = load_tool(&tool_path).unwrap();
    assert_eq!(tool.base_command, Command::Multiple(vec!["python".to_string(), "echo.py".to_string()]));
    assert_eq!(tool.inputs.len(), 2);
    assert_eq!(tool.outputs.len(), 1);

    cleanup(current, dir);
}

#[test]
#[serial]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#wrapping-a-long-running-script
pub fn test_wrapping_a_long_running_script() {
    let (current, dir) = setup();

    let command = &["python", "sleep.py"];

    let name = "sleep";
    let args = &CreateToolArgs {
        name: None,
        container_image: None,
        container_tag: None,
        is_raw: false,
        no_commit: false,
        no_run: true, //
        is_clean: false,
        inputs: None,
        outputs: None,
        command: command.iter().map(|&s| s.to_string()).collect(),
    };
    assert!(create_tool(args).is_ok());

    let tool_path = dir.path().join(format!("workflows/{name}/{name}.cwl"));
    assert!(fs::exists(&tool_path).unwrap());

    let tool = load_tool(&tool_path).unwrap();
    assert_eq!(tool.base_command, Command::Multiple(vec!["python".to_string(), "sleep.py".to_string()]));
    assert_eq!(tool.inputs.len(), 0);
    assert_eq!(tool.outputs.len(), 0);

    cleanup(current, dir);
}

#[test]
#[serial]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#wrapping-a-long-running-script
pub fn test_wrapping_a_long_running_script2() {
    let (current, dir) = setup();

    let command = &["python", "sleep.py"];

    let name = "sleep2";
    let args = &CreateToolArgs {
        name: Some(name.to_string()),
        container_image: None,
        container_tag: None,
        is_raw: false,
        no_commit: false,
        no_run: true, //
        is_clean: false,
        inputs: None,
        outputs: Some(vec!["sleep.txt".to_string()]),
        command: command.iter().map(|&s| s.to_string()).collect(),
    };
    assert!(create_tool(args).is_ok());

    let tool_path = dir.path().join(format!("workflows/{name}/{name}.cwl"));
    assert!(fs::exists(&tool_path).unwrap());

    let tool = load_tool(&tool_path).unwrap();
    assert_eq!(tool.base_command, Command::Multiple(vec!["python".to_string(), "sleep.py".to_string()]));
    assert_eq!(tool.inputs.len(), 0);
    assert_eq!(tool.outputs.len(), 1);

    cleanup(current, dir);
}

#[test]
#[serial]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#implicit-inputs-hardcoded-files
pub fn test_implicit_inputs_hardcoded_files() {
    let (current, dir) = setup();

    let command = &["python", "load.py"];

    let name = "load";
    let args = &CreateToolArgs {
        name: None,
        container_image: None,
        container_tag: None,
        is_raw: false,
        no_commit: false,
        no_run: false,
        is_clean: false,
        inputs: Some(vec!["file.txt".to_string()]),
        outputs: Some(vec!["out.txt".to_string()]),
        command: command.iter().map(|&s| s.to_string()).collect(),
    };
    assert!(create_tool(args).is_ok());

    let tool_path = dir.path().join(format!("workflows/{name}/{name}.cwl"));
    assert!(fs::exists(&tool_path).unwrap());

    let tool = load_tool(&tool_path).unwrap();
    assert_eq!(tool.base_command, Command::Multiple(vec!["python".to_string(), "load.py".to_string()]));
    assert_eq!(tool.inputs.len(), 1);
    assert_eq!(tool.outputs.len(), 1);

    assert!(tool.requirements.is_some());
    let requirements = tool.requirements.unwrap();
    assert_eq!(requirements.len(), 1);

    if let Requirement::InitialWorkDirRequirement(initial) = &requirements[0] {
        assert_eq!(initial.listing.len(), 2);
        assert_eq!(initial.listing[0].entryname, "file.txt");
        assert_eq!(initial.listing[0].entry, Entry::Source("$(inputs.file_txt)".into()));
        
        assert_eq!(initial.listing[1].entryname, "load.py");
    } else {
        panic!("InitialWorkDirRequirement not found!");
    }

    cleanup(current, dir);
}

#[test]
#[serial]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#piping
pub fn test_piping() {
    let (current, dir) = setup();

    let command = &["cat", "speakers.csv", "|", "head", "-n", "5", ">", "speakers_5.csv"];

    let name = "cat";
    let args = &CreateToolArgs {
        name: None,
        container_image: None,
        container_tag: None,
        is_raw: false,
        no_commit: false,
        no_run: false,
        is_clean: false,
        inputs: None,
        outputs: None,
        command: command.iter().map(|&s| s.to_string()).collect(),
    };
    assert!(create_tool(args).is_ok());

    let tool_path = dir.path().join(format!("workflows/{name}/{name}.cwl"));
    assert!(fs::exists(&tool_path).unwrap());

    let tool = load_tool(&tool_path).unwrap();
    assert_eq!(tool.base_command, Command::Single("cat".to_string()));
    assert_eq!(tool.inputs.len(), 1);
    assert_eq!(tool.outputs.len(), 1);
    assert!(tool.arguments.is_some());
    assert_eq!(tool.arguments.unwrap().len(), 6);

    cleanup(current, dir);
}

#[test]
#[serial]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#pulling-containers
pub fn test_pulling_containers() {
    let (current, dir) = setup();

    let command = &[
        "python",
        "calculation.py",
        "--population",
        "population.csv",
        "--speakers",
        "speakers_revised.csv",
    ];

    let name = "calculation";
    let args = &CreateToolArgs {
        name: None,
        container_image: Some("pandas/pandas:pip-all".to_string()),
        container_tag: None,
        is_raw: false,
        no_commit: false,
        no_run: false,
        is_clean: false,
        inputs: None,
        outputs: None,
        command: command.iter().map(|&s| s.to_string()).collect(),
    };

    //setup python env
    let (newpath, restore) = setup_python(dir.path().to_str().unwrap());
    env::set_var("PATH", newpath);

    assert!(create_tool(args).is_ok());

    //restore path
    env::set_var("PATH", restore);

    let tool_path = dir.path().join(format!("workflows/{name}/{name}.cwl"));
    assert!(fs::exists(&tool_path).unwrap());

    let tool = load_tool(&tool_path).unwrap();
    assert_eq!(
        tool.base_command,
        Command::Multiple(vec!["python".to_string(), "calculation.py".to_string()])
    );
    assert_eq!(tool.inputs.len(), 2);
    assert_eq!(tool.outputs.len(), 1);

    cleanup(current, dir);
}

#[test]
#[serial]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#building-custom-containers
pub fn test_building_custom_containers() {
    let (current, dir) = setup();

    let command = &[
        "python",
        "calculation.py",
        "--population",
        "population.csv",
        "--speakers",
        "speakers_revised.csv",
    ];

    let name = "calculation";
    let args = &CreateToolArgs {
        name: None,
        container_image: Some("Dockerfile".to_string()),
        container_tag: Some("my-docker".to_string()),
        is_raw: false,
        no_commit: false,
        no_run: false,
        is_clean: false,
        inputs: None,
        outputs: None,
        command: command.iter().map(|&s| s.to_string()).collect(),
    };

    //setup python env
    let (newpath, restore) = setup_python(dir.path().to_str().unwrap());
    env::set_var("PATH", newpath);

    assert!(create_tool(args).is_ok());

    //restore path
    env::set_var("PATH", restore);

    let tool_path = dir.path().join(format!("workflows/{name}/{name}.cwl"));
    assert!(fs::exists(&tool_path).unwrap());

    let tool = load_tool(&tool_path).unwrap();
    assert_eq!(
        tool.base_command,
        Command::Multiple(vec!["python".to_string(), "calculation.py".to_string()])
    );
    assert_eq!(tool.inputs.len(), 2);
    assert_eq!(tool.outputs.len(), 1);

    cleanup(current, dir);
}
