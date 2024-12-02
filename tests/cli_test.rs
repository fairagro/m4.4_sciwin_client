mod common;
use common::{check_git_user, setup_python};
use s4n::{
    commands::{
        execute::{execute_local, LocalExecuteArgs, Runner},
        init::init_s4n,
        tool::{create_tool, list_tools, CreateToolArgs, LsArgs},
        workflow::{connect_workflow_nodes, create_workflow, get_workflow_status, save_workflow, ConnectWorkflowArgs, CreateWorkflowArgs},
    },
    cwl::loader::load_workflow,
    io::copy_dir,
};
use serial_test::serial;
use std::{
    env,
    fs::{self, remove_dir_all, remove_file},
    vec,
};
use tempfile::tempdir;

#[test]
#[serial]
///Tests a whole s4n workflow
pub fn test_cli_s4n_workflow() {
    //set up environment
    let dir = tempdir().unwrap();
    let dir_str = &dir.path().to_string_lossy();
    let test_folder = "tests/test_data/hello_world";
    copy_dir(test_folder, dir_str).unwrap();

    //delete all cwl files as we want to generate them
    remove_dir_all(dir.path().join("workflows/main")).unwrap();
    remove_file(dir.path().join("workflows/calculation/calculation.cwl")).unwrap();
    remove_file(dir.path().join("workflows/plot/plot.cwl")).unwrap();

    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();
    let restore = setup_python(&dir_str);

    check_git_user().unwrap();

    //init project
    init_s4n(None, false).expect("Could not init s4n");

    //create calculation tool
    create_tool(&CreateToolArgs {
        name: None,
        container_image: None,
        container_tag: None,
        is_raw: false,
        no_commit: false,
        no_run: false,
        is_clean: false,
        command: [
            "python".to_string(),
            "workflows/calculation/calculation.py".to_string(),
            "--speakers".to_string(),
            "data/speakers_revised.csv".to_string(),
            "--population".to_string(),
            "data/population.csv".to_string(),
        ]
        .to_vec(),
    })
    .expect("Could not create calculation tool");
    assert!(fs::exists("workflows/calculation/calculation.cwl").unwrap());

    //create calculation tool
    create_tool(&CreateToolArgs {
        name: None,
        container_image: None,
        container_tag: None,
        is_raw: false,
        no_commit: false,
        no_run: false,
        is_clean: false,
        command: [
            "python".to_string(),
            "workflows/plot/plot.py".to_string(),
            "--results".to_string(),
            "results.csv".to_string(),
        ]
        .to_vec(),
    })
    .expect("Could not create plot tool");
    assert!(fs::exists("workflows/plot/plot.cwl").unwrap());

    //list tools
    list_tools(&LsArgs { list_all: true }).unwrap();

    //create workflow
    let name = "test_workflow".to_string();
    let create_args = CreateWorkflowArgs { name: name.clone(), force: false };
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
        name: name.clone(),
        from: "plot/results".to_string(),
        to: "@outputs/out".to_string(),
    })
    .expect("Could not add input to output/out");

    //save workflow
    save_workflow(&create_args).expect("Could not save workflow");
    let wf_path = "workflows/test_workflow/test_workflow.cwl";
    assert!(fs::exists(wf_path).unwrap());

    let workflow = load_workflow(wf_path).unwrap();
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
    remove_file("results.csv").unwrap();
    remove_file("results.svg").unwrap();

    assert!(!fs::exists("results.csv").unwrap());
    assert!(!fs::exists("results.svg").unwrap());

    //execute workflow
    execute_local(&LocalExecuteArgs {
        runner: Runner::Custom,
        out_dir: None,
        is_quiet: false,
        file: wf_path.to_string(),
        args: vec!["inputs.yml".to_string()],
    })
    .expect("Could not execute Workflow");

    //check that only svg file is there now!
    assert!(!fs::exists("results.csv").unwrap());
    assert!(fs::exists("results.svg").unwrap());

    env::set_var("PATH", restore);
    env::set_current_dir(current).unwrap();
}
