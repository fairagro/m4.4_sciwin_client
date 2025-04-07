///This file contains all examples described here: https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/
mod common;
use common::{check_git_user, setup_python};
use cwl::{clt::Command, load_tool, load_workflow, requirements::Requirement, types::Entry};
use cwl_execution::io::copy_dir;
use s4n::commands::{
    execute::{execute_local, LocalExecuteArgs, Runner},
    init::init_s4n,
    tool::{create_tool, list_tools, CreateToolArgs},
    workflow::{connect_workflow_nodes, create_workflow, get_workflow_status, save_workflow, ConnectWorkflowArgs, CreateWorkflowArgs},
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
#[cfg_attr(target_os = "windows", ignore)]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#wrapping-echo
pub fn test_wrapping_echo() {
    let (current, dir) = setup();

    let command = &["echo", "\"Hello World\""];

    let args = &CreateToolArgs {
        command: command.iter().map(|&s| s.to_string()).collect(),
        ..Default::default()
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
#[cfg_attr(target_os = "windows", ignore)]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#wrapping-echo
pub fn test_wrapping_echo_2() {
    let (current, dir) = setup();

    let command = &["echo", "\"Hello World\"", ">", "hello.yaml"];

    let name = "echo2";
    let args = &CreateToolArgs {
        name: Some(name.to_string()),
        command: command.iter().map(|&s| s.to_string()).collect(),
        ..Default::default()
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
        command: command.iter().map(|&s| s.to_string()).collect(),
        ..Default::default()
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
        no_run: true,
        command: command.iter().map(|&s| s.to_string()).collect(),
        ..Default::default()
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
        no_run: true,
        outputs: Some(vec!["sleep.txt".to_string()]),
        command: command.iter().map(|&s| s.to_string()).collect(),
        ..Default::default()
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
        inputs: Some(vec!["file.txt".to_string()]),
        outputs: Some(vec!["out.txt".to_string()]),
        command: command.iter().map(|&s| s.to_string()).collect(),
        ..Default::default()
    };
    assert!(create_tool(args).is_ok());

    let tool_path = dir.path().join(format!("workflows/{name}/{name}.cwl"));
    assert!(fs::exists(&tool_path).unwrap());

    let tool = load_tool(&tool_path).unwrap();
    assert_eq!(tool.base_command, Command::Multiple(vec!["python".to_string(), "load.py".to_string()]));
    assert_eq!(tool.inputs.len(), 1);
    assert_eq!(tool.outputs.len(), 1);

    assert!(tool.requirements.is_some());
    let requirements = tool.requirements.clone().unwrap();
    assert_eq!(requirements.len(), 1);

    if let Requirement::InitialWorkDirRequirement(initial) = &requirements[0] {
        assert_eq!(initial.listing.len(), 2);
        assert_eq!(initial.listing[0].entryname, "load.py");
        assert_eq!(initial.listing[1].entryname, "file.txt");
        assert_eq!(initial.listing[1].entry, Entry::Source("$(inputs.file_txt)".into()));
    } else {
        panic!("InitialWorkDirRequirement not found!");
    }

    cleanup(current, dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore)]
///see https://fairagro.github.io/m4.4_sciwin_client/examples/tool-creation/#piping
pub fn test_piping() {
    let (current, dir) = setup();

    let command = &["cat", "speakers.csv", "|", "head", "-n", "5", ">", "speakers_5.csv"];

    let name = "cat";
    let args = &CreateToolArgs {
        command: command.iter().map(|&s| s.to_string()).collect(),
        ..Default::default()
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
        container_image: Some("pandas/pandas:pip-all".to_string()),
        command: command.iter().map(|&s| s.to_string()).collect(),
        ..Default::default()
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
        container_image: Some("Dockerfile".to_string()),
        container_tag: Some("my-docker".to_string()),
        command: command.iter().map(|&s| s.to_string()).collect(),
        ..Default::default()
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
///see https://fairagro.github.io/m4.4_sciwin_client/getting-started/example/
//docker not working on MacOS Github Actions
#[cfg_attr(target_os = "macos", ignore)]
pub fn test_example_project() {
    //set up environment
    let dir = tempdir().unwrap();
    let dir_str = &dir.path().to_string_lossy();
    let test_folder = "tests/test_data/hello_world";
    copy_dir(test_folder, dir.path()).unwrap();

    //delete all cwl files as we want to generate
    fs::remove_dir_all(dir.path().join("workflows/main")).unwrap();
    fs::remove_file(dir.path().join("workflows/plot/plot.cwl")).unwrap();
    fs::remove_file(dir.path().join("workflows/calculation/calculation.cwl")).unwrap();

    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();
    let (newpath, restore) = setup_python(dir_str);

    //modify path variable
    env::set_var("PATH", newpath);

    check_git_user().unwrap();

    //init project
    init_s4n(None, false).expect("Could not init s4n");

    //create calculation tool
    create_tool(&CreateToolArgs {
        command: [
            "python".to_string(),
            "workflows/calculation/calculation.py".to_string(),
            "--speakers".to_string(),
            "data/speakers_revised.csv".to_string(),
            "--population".to_string(),
            "data/population.csv".to_string(),
        ]
        .to_vec(),
        container_image: Some("pandas/pandas:pip-all".to_string()),
        ..Default::default()
    })
    .expect("Could not create calculation tool");
    assert!(fs::exists("workflows/calculation/calculation.cwl").unwrap());

    //create calculation tool
    create_tool(&CreateToolArgs {
        command: [
            "python".to_string(),
            "workflows/plot/plot.py".to_string(),
            "--results".to_string(),
            "results.csv".to_string(),
        ]
        .to_vec(),
        container_image: Some("workflows/plot/Dockerfile".to_string()),
        container_tag: Some("matplotlib".to_string()),
        ..Default::default()
    })
    .expect("Could not create plot tool");
    assert!(fs::exists("workflows/plot/plot.cwl").unwrap());
    //list tools
    list_tools(&Default::default()).unwrap();

    //create workflow
    let name = "test_workflow".to_string();
    let create_args = CreateWorkflowArgs {
        name: name.clone(),
        force: false,
    };
    create_workflow(&create_args).expect("Could not create workflow");

    //add connections to inputs
    connect_workflow_nodes(&ConnectWorkflowArgs {
        name: name.clone(),
        from: "@inputs/population".to_string(),
        to: "calculation/population".to_string(),
    })
    .expect("Could not add input to calculation/population");

    connect_workflow_nodes(&ConnectWorkflowArgs {
        name: name.clone(),
        from: "@inputs/speakers".to_string(),
        to: "calculation/speakers".to_string(),
    })
    .expect("Could not add input to calculation/speakers");

    //connect second step
    connect_workflow_nodes(&ConnectWorkflowArgs {
        name: name.clone(),
        from: "calculation/results".to_string(),
        to: "plot/results".to_string(),
    })
    .expect("Could not add input to plot/results");

    //connect output
    connect_workflow_nodes(&ConnectWorkflowArgs {
        name,
        from: "plot/results".to_string(),
        to: "@outputs/out".to_string(),
    })
    .expect("Could not add input to output/out");

    //save workflow
    save_workflow(&create_args).expect("Could not save workflow");
    let wf_path = PathBuf::from("workflows/test_workflow/test_workflow.cwl");
    assert!(fs::exists(&wf_path).unwrap());

    let workflow = load_workflow(&wf_path).unwrap();
    assert!(workflow.has_input("speakers"));
    assert!(workflow.has_input("population"));
    assert!(workflow.has_output("out"));
    assert!(workflow.has_step("calculation"));
    assert!(workflow.has_step("plot"));
    assert!(workflow.has_step_input("speakers"));
    assert!(workflow.has_step_input("population"));
    assert!(workflow.has_step_input("calculation/results"));
    assert!(workflow.has_step_output("calculation/results"));
    assert!(workflow.has_step_output("plot/results"));

    //workflow status
    get_workflow_status(&create_args).expect("Could not print status");

    //remove outputs
    fs::remove_file("results.csv").unwrap();
    fs::remove_file("results.svg").unwrap();

    assert!(!fs::exists("results.csv").unwrap());
    assert!(!fs::exists("results.svg").unwrap());

    //execute workflow
    execute_local(&LocalExecuteArgs {
        runner: Runner::Custom,
        is_quiet: false,
        file: wf_path,
        args: vec!["inputs.yml".to_string()],
        ..Default::default()
    })
    .expect("Could not execute Workflow");

    //check that only svg file is there now!
    assert!(!fs::exists("results.csv").unwrap());
    assert!(fs::exists("results.svg").unwrap());

    env::set_var("PATH", restore);
    env::set_current_dir(current).unwrap();
}
