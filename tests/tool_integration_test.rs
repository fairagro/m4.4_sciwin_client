mod common;
use common::{check_git_user, os_path, with_temp_repository};
use cwl::{
    clt::{Argument, CommandLineTool},
    load_tool,
    requirements::{DockerRequirement, Requirement},
    types::{CWLType, Entry},
};
use cwl_execution::io::copy_file;
use git2::Repository;
use s4n::{
    commands::{
        init::init_s4n,
        tool::{create_tool, handle_tool_commands, CreateToolArgs, ToolCommands},
    },
    repo::get_modified_files,
};
use serial_test::serial;
use std::{env, fs::read_to_string, path::Path};
use tempfile::tempdir;

#[test]
#[serial]
pub fn tool_create_test() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            command: vec![
                "python".to_string(),
                "scripts/echo.py".to_string(),
                "--test".to_string(),
                "data/input.txt".to_string(),
            ],
            ..Default::default()
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());

        //check for files being present
        let output_paths = vec![
            dir.path().join(Path::new("results.txt")),
            dir.path().join(Path::new("workflows/echo/echo.cwl")),
        ];
        for output_path in output_paths {
            assert!(output_path.exists());
        }

        //no uncommitted left?
        let repo = Repository::open(dir.path()).unwrap();
        assert!(get_modified_files(&repo).is_empty());
    });
}

#[test]
#[serial]
pub fn tool_create_test_inputs_outputs() {
    with_temp_repository(|dir| {
        let script = "scripts/echo_inline.py".to_string();
        let input = "../data/input.txt".to_string();

        let tool_create_args = CreateToolArgs {
            inputs: Some(vec![input.clone()]),
            outputs: Some(vec!["results.txt".to_string()]),
            command: vec!["python".to_string(), script.clone()],
            ..Default::default()
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());

        let tool_path = Path::new("workflows/echo_inline/echo_inline.cwl");

        //check for files being present
        let output_paths = vec![dir.path().join(Path::new("results.txt")), dir.path().join(tool_path)];

        for output_path in output_paths {
            assert!(output_path.exists());
        }

        //check tool props
        let tool = load_tool(tool_path).unwrap();

        assert_eq!(tool.inputs.len(), 1);
        assert_eq!(tool.outputs.len(), 1);

        if let Some(req) = tool.requirements {
            if let Requirement::InitialWorkDirRequirement(iwdr) = &req[0] {
                assert_eq!(iwdr.listing.len(), 2);
                assert_eq!(iwdr.listing[0].entryname, script);
                assert_eq!(iwdr.listing[1].entryname, input);
            } else {
                panic!("Not an InitialWorkDirRequirement")
            }
        } else {
            panic!("No Requirements set")
        }

        //no uncommitted left?
        let repo = Repository::open(dir.path()).unwrap();
        assert!(get_modified_files(&repo).is_empty());
    });
}

#[test]
#[serial]
pub fn tool_create_test_is_raw() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            is_raw: true,
            command: vec![
                "python".to_string(),
                "scripts/echo.py".to_string(),
                "--test".to_string(),
                "data/input.txt".to_string(),
            ],
            ..Default::default()
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());
        assert!(!dir.path().join(Path::new("workflows/echo/echo.cwl")).exists()); //no cwl file as it is outputted to stdout
        assert!(dir.path().join(Path::new("results.txt")).exists());

        //no uncommitted left?
        let repo = Repository::open(dir.path()).unwrap();
        assert!(get_modified_files(&repo).is_empty());
    });
}

#[test]
#[serial]
pub fn tool_create_test_no_commit() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            no_commit: true, //look!
            command: vec![
                "python".to_string(),
                "scripts/echo.py".to_string(),
                "--test".to_string(),
                "data/input.txt".to_string(),
            ],
            ..Default::default()
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());

        //check for files being present
        let output_paths = vec![
            dir.path().join(Path::new("results.txt")),
            dir.path().join(Path::new("workflows/echo/echo.cwl")),
        ];
        for output_path in output_paths {
            assert!(output_path.exists());
        }
        //as we did not commit there must be files (exactly 2, the cwl file and the results.txt)
        let repo = Repository::open(dir.path()).unwrap();
        assert_eq!(get_modified_files(&repo).len(), 2);
    });
}

#[test]
#[serial]
pub fn tool_create_test_no_run() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            no_run: true,
            command: vec![
                "python".to_string(),
                "scripts/echo.py".to_string(),
                "--test".to_string(),
                "data/input.txt".to_string(),
            ],
            ..Default::default()
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());
        assert!(dir.path().join(Path::new("workflows/echo/echo.cwl")).exists());

        //no uncommitted left?
        let repo = Repository::open(dir.path()).unwrap();
        assert!(get_modified_files(&repo).is_empty());
    });
}

#[test]
#[serial]
pub fn tool_create_test_is_clean() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            is_clean: true,
            command: vec![
                "python".to_string(),
                "scripts/echo.py".to_string(),
                "--test".to_string(),
                "data/input.txt".to_string(),
            ],
            ..Default::default()
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());
        assert!(dir.path().join(Path::new("workflows/echo/echo.cwl")).exists());
        assert!(!dir.path().join(Path::new("results.txt")).exists()); //no result is left as it is cleaned

        //no uncommitted left?
        let repo = Repository::open(dir.path()).unwrap();
        assert!(get_modified_files(&repo).is_empty());
    });
}

#[test]
#[serial]
pub fn tool_create_test_container_image() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            container_image: Some("python".to_string()),
            command: vec![
                "python".to_string(),
                "scripts/echo.py".to_string(),
                "--test".to_string(),
                "data/input.txt".to_string(),
            ],
            ..Default::default()
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());

        //read file
        let cwl_file = dir.path().join(Path::new("workflows/echo/echo.cwl"));
        let cwl_contents = read_to_string(cwl_file).expect("Could not read CWL File");
        let cwl: CommandLineTool = serde_yaml::from_str(&cwl_contents).expect("Could not convert CWL");

        let requirements = cwl.requirements.expect("No requirements found!");
        assert_eq!(requirements.len(), 2);

        if let Requirement::DockerRequirement(DockerRequirement::DockerPull(image)) = &requirements[1] {
            assert_eq!(image, "python");
        } else {
            panic!("Requirement is not a Docker pull");
        }

        //no uncommitted left?
        let repo = Repository::open(dir.path()).unwrap();
        assert!(get_modified_files(&repo).is_empty());
    });
}

#[test]
#[serial]
pub fn tool_create_test_dockerfile() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            container_image: Some("Dockerfile".to_string()),
            container_tag: Some("sciwin-client".to_string()),
            command: vec![
                "python".to_string(),
                "scripts/echo.py".to_string(),
                "--test".to_string(),
                "data/input.txt".to_string(),
            ],
            ..Default::default()
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());

        //read file
        let cwl_file = dir.path().join(Path::new("workflows/echo/echo.cwl"));
        let cwl_contents = read_to_string(cwl_file).expect("Could not read CWL File");
        let cwl: CommandLineTool = serde_yaml::from_str(&cwl_contents).expect("Could not convert CWL");

        let requirements = cwl.requirements.expect("No requirements found!");
        assert_eq!(requirements.len(), 2);

        if let Requirement::DockerRequirement(DockerRequirement::DockerFile {
            docker_file,
            docker_image_id,
        }) = &requirements[1]
        {
            assert_eq!(*docker_file, Entry::from_file(&os_path("../../Dockerfile"))); //as file is in root and cwl in workflows/echo
            assert_eq!(*docker_image_id, "sciwin-client".to_string());
        } else {
            panic!("Requirement is not a Dockerfile");
        }

        //no uncommitted left?
        let repo = Repository::open(dir.path()).unwrap();
        assert!(get_modified_files(&repo).is_empty());
    });
}

#[test]
#[serial]
pub fn test_tool_magic_outputs() {
    with_temp_repository(|_| {
        let str = "touch output.txt";
        let args = CreateToolArgs {
            no_commit: true,
            is_clean: true,
            command: shlex::split(str).unwrap(),
            ..Default::default()
        };

        assert!(create_tool(&args).is_ok());

        let tool = load_tool("workflows/touch/touch.cwl").unwrap();

        assert!(tool.outputs[0].output_binding.as_ref().unwrap().glob == *"$(inputs.output_txt)");
    });
}

#[test]
#[serial]
pub fn test_tool_magic_stdout() {
    with_temp_repository(|_| {
        let str = "wc data/input.txt \\> data/input.txt";
        let args = CreateToolArgs {
            no_commit: true,
            is_clean: true,
            command: shlex::split(str).unwrap(),
            ..Default::default()
        };

        assert!(create_tool(&args).is_ok());

        let tool = load_tool("workflows/wc/wc.cwl").unwrap();
        assert!(tool.stdout.unwrap() == *"$(inputs.data_input_txt.path)");
    });
}

#[test]
#[serial]
pub fn test_tool_magic_arguments() {
    with_temp_repository(|_| {
        let str = "cat data/input.txt | grep -f data/input.txt";
        let args = CreateToolArgs {
            no_commit: true,
            is_clean: true,
            command: shlex::split(str).unwrap(),
            ..Default::default()
        };

        assert!(create_tool(&args).is_ok());

        let tool = load_tool("workflows/cat/cat.cwl").unwrap();
        if let Argument::Binding(binding) = &tool.arguments.unwrap()[3] {
            assert!(binding.value_from == Some("$(inputs.data_input_txt.path)".to_string()));
        } else {
            panic!()
        }
    });
}

#[test]
#[serial]
pub fn test_tool_output_is_dir() {
    let dir = tempdir().unwrap();

    copy_file("tests/test_data/create_dir.py", dir.path().join("create_dir.py").to_str().unwrap()).unwrap();

    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();
    check_git_user().unwrap();
    init_s4n(None, false).expect("Could not init s4n");

    let name = "create_dir";
    let command = &["python", "create_dir.py"];
    let args = CreateToolArgs {
        command: command.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        ..Default::default()
    };

    assert!(create_tool(&args).is_ok());

    let tool = load_tool(format!("workflows/{name}/{name}.cwl")).unwrap();
    assert_eq!(tool.inputs.len(), 0);
    assert_eq!(tool.outputs.len(), 1); //only folder
    assert_eq!(tool.outputs[0].id, "my_directory".to_string());
    assert_eq!(tool.outputs[0].type_, CWLType::Directory);

    env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
pub fn test_tool_output_complete_dir() {
    let dir = tempdir().unwrap();

    copy_file("tests/test_data/create_dir.py", dir.path().join("create_dir.py").to_str().unwrap()).unwrap();

    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();
    check_git_user().unwrap();
    init_s4n(None, false).expect("Could not init s4n");

    let name = "create_dir";
    let command = &["python", "create_dir.py"];
    let args = CreateToolArgs {
        outputs: Some(vec![".".into()]), //
        command: command.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        ..Default::default()
    };

    assert!(create_tool(&args).is_ok());

    let tool = load_tool(format!("workflows/{name}/{name}.cwl")).unwrap();
    assert_eq!(tool.inputs.len(), 0);
    assert_eq!(tool.outputs.len(), 1); //only root folder
    if let Some(binding) = &tool.outputs[0].output_binding {
        assert_eq!(binding.glob, "$(runtime.outdir)".to_string())
    } else {
        panic!("No Binding")
    }

    println!("{:#?}", tool.outputs);

    env::set_current_dir(current).unwrap();
}

#[test]
#[serial]
#[cfg(target_os = "linux")]
pub fn test_shell_script() {
    let dir = tempdir().unwrap();

    let script = dir.path().join("script.sh");
    copy_file("tests/test_data/script.sh", script.to_str().unwrap()).unwrap();
    std::fs::set_permissions(script, <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755)).unwrap();

    let current = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();
    check_git_user().unwrap();
    init_s4n(None, false).expect("Could not init s4n");

    let name = "script";
    let command = &["./script.sh"];
    let args = CreateToolArgs {
        command: command.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        ..Default::default()
    };

    let result = create_tool(&args);
    println!("{result:#?}");
    assert!(result.is_ok());

    let tool = load_tool(format!("workflows/{name}/{name}.cwl")).unwrap();
    assert_eq!(tool.inputs.len(), 0);
    assert_eq!(tool.outputs.len(), 0);

    if let Some(req) = tool.requirements {
        assert_eq!(req.len(), 1);
        if let Requirement::InitialWorkDirRequirement(iwdr) = &req[0] {
            assert_eq!(iwdr.listing[0].entryname, "./script.sh");
        } else {
            panic!("Not an InitialWorkDirRequirement")
        }
    } else {
        panic!("No requirements found")
    }

    env::set_current_dir(current).unwrap();
}
