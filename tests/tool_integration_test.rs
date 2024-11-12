mod common;
use common::{os_path, with_temp_repository};
use git2::Repository;
use s4n::{
    commands::tool::{handle_tool_commands, CreateToolArgs, ToolCommands},
    cwl::clt::{CommandLineTool, DockerRequirement, Entry, Requirement},
    repo::get_modified_files,
};
use serial_test::serial;
use std::{fs::read_to_string, path::Path};

#[test]
#[serial]
pub fn tool_create_test() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            name: None,
            container_image: None,
            container_tag: None,
            is_raw: false,
            no_commit: false,
            no_run: false,
            is_clean: false,
            command: vec!["python".to_string(), "scripts/echo.py".to_string(), "--test".to_string(), "data/input.txt".to_string()],
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());

        //check for files being present
        let output_paths = vec![dir.path().join(Path::new("results.txt")), dir.path().join(Path::new("workflows/echo/echo.cwl"))];
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
pub fn tool_create_test_is_raw() {
    with_temp_repository(|dir| {
        let tool_create_args = CreateToolArgs {
            name: None,
            container_image: None,
            container_tag: None,
            is_raw: true, //look!
            no_commit: false,
            no_run: false,
            is_clean: false,
            command: vec!["python".to_string(), "scripts/echo.py".to_string(), "--test".to_string(), "data/input.txt".to_string()],
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
            name: None,
            container_image: None,
            container_tag: None,
            is_raw: false,
            no_commit: true, //look!
            no_run: false,
            is_clean: false,
            command: vec!["python".to_string(), "scripts/echo.py".to_string(), "--test".to_string(), "data/input.txt".to_string()],
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());

        //check for files being present
        let output_paths = vec![dir.path().join(Path::new("results.txt")), dir.path().join(Path::new("workflows/echo/echo.cwl"))];
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
            name: None,
            container_image: None,
            container_tag: None,
            is_raw: false,
            no_commit: false,
            no_run: true, //look!
            is_clean: false,
            command: vec!["python".to_string(), "scripts/echo.py".to_string(), "--test".to_string(), "data/input.txt".to_string()],
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
            name: None,
            container_image: None,
            container_tag: None,
            is_raw: false,
            no_commit: false,
            no_run: false,
            is_clean: true, //look!
            command: vec!["python".to_string(), "scripts/echo.py".to_string(), "--test".to_string(), "data/input.txt".to_string()],
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
            name: None,
            container_image: Some("python".to_string()), //look!
            container_tag: None,
            is_raw: false,
            no_commit: false,
            no_run: false,
            is_clean: false,
            command: vec!["python".to_string(), "scripts/echo.py".to_string(), "--test".to_string(), "data/input.txt".to_string()],
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());

        //read file
        let cwl_file = dir.path().join(Path::new("workflows/echo/echo.cwl"));
        let cwl_contents = read_to_string(cwl_file).expect("Could not read CWL File");
        let cwl: CommandLineTool = serde_yml::from_str(&cwl_contents).expect("Could not convert CWL");

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
            name: None,
            container_image: Some("Dockerfile".to_string()),  //look
            container_tag: Some("sciwin-client".to_string()), //look!
            is_raw: false,
            no_commit: false,
            no_run: false,
            is_clean: false,
            command: vec!["python".to_string(), "scripts/echo.py".to_string(), "--test".to_string(), "data/input.txt".to_string()],
        };
        let cmd = ToolCommands::Create(tool_create_args);
        assert!(handle_tool_commands(&cmd).is_ok());

        //read file
        let cwl_file = dir.path().join(Path::new("workflows/echo/echo.cwl"));
        let cwl_contents = read_to_string(cwl_file).expect("Could not read CWL File");
        let cwl: CommandLineTool = serde_yml::from_str(&cwl_contents).expect("Could not convert CWL");

        let requirements = cwl.requirements.expect("No requirements found!");
        assert_eq!(requirements.len(), 2);

        if let Requirement::DockerRequirement(DockerRequirement::DockerFile { docker_file, docker_image_id }) = &requirements[1] {
            assert_eq!(*docker_file, Entry::from_file(&os_path("../../Dockerfile"))); //as file is in root and cwl in workflows/echo
            assert_eq!(*docker_image_id, "sciwin-client".to_string())
        } else {
            panic!("Requirement is not a Dockerfile");
        }

        //no uncommitted left?
        let repo = Repository::open(dir.path()).unwrap();
        assert!(get_modified_files(&repo).is_empty());
    });
}