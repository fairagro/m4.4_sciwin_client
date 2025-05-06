mod common;
use common::os_path;
use cwl::{
    clt::{Argument, CommandLineTool},
    load_tool,
    requirements::{DockerRequirement, Requirement},
    types::{CWLType, Entry},
};
use fstest::fstest;
use git2::Repository;
use s4n::{
    commands::tool::{create_tool, handle_tool_commands, CreateToolArgs, ToolCommands},
    repo::{get_modified_files, stage_all},
};
use std::{
    env,
    fs::{self, read_to_string},
    path::Path,
};

#[fstest(repo = true, files = ["tests/test_data/input.txt", "tests/test_data/echo.py"])]
pub fn tool_create_test() {
    let tool_create_args = CreateToolArgs {
        command: vec!["python".to_string(), "echo.py".to_string(), "--test".to_string(), "input.txt".to_string()],
        ..Default::default()
    };
    let cmd = ToolCommands::Create(tool_create_args);
    assert!(handle_tool_commands(&cmd).is_ok());

    //check for files being present
    let output_paths = vec![Path::new("results.txt"), Path::new("workflows/echo/echo.cwl")];
    for output_path in output_paths {
        assert!(output_path.exists());
    }

    //no uncommitted left?
    let repo = Repository::open(".").unwrap();
    assert!(get_modified_files(&repo).is_empty());
}

#[fstest(repo = true, files = ["tests/test_data/input.txt", "tests/test_data/echo_inline.py"])]
pub fn tool_create_test_inputs_outputs() {
    fs::create_dir_all("data").unwrap();
    fs::copy("input.txt", "data/input.txt").unwrap(); //copy to data folder
    fs::remove_file("input.txt").unwrap(); //remove original file

    let repo = Repository::open(".").unwrap();
    stage_all(&repo).unwrap();

    let script = "echo_inline.py".to_string();
    let input = "../../data/input.txt".to_string();

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
    let output_paths = vec![Path::new("results.txt"), tool_path];

    for output_path in output_paths {
        assert!(output_path.exists());
    }

    //check tool props
    let tool = load_tool(tool_path).unwrap();

    assert_eq!(tool.inputs.len(), 1);
    assert_eq!(tool.outputs.len(), 1);

    if let Some(req) = &tool.requirements {
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
    assert!(get_modified_files(&repo).is_empty());
}

#[fstest(repo = true, files = ["tests/test_data/input.txt", "tests/test_data/echo.py"])]
pub fn tool_create_test_is_raw() {
    let tool_create_args = CreateToolArgs {
        is_raw: true,
        command: vec!["python".to_string(), "echo.py".to_string(), "--test".to_string(), "input.txt".to_string()],
        ..Default::default()
    };
    let cmd = ToolCommands::Create(tool_create_args);
    assert!(handle_tool_commands(&cmd).is_ok());
    assert!(!Path::new("workflows/echo/echo.cwl").exists()); //no cwl file as it is outputted to stdout
    assert!(Path::new("results.txt").exists());

    //no uncommitted left?
    let repo = Repository::open(".").unwrap();
    assert!(get_modified_files(&repo).is_empty());
}

#[fstest(repo = true, files = ["tests/test_data/input.txt", "tests/test_data/echo.py"])]
pub fn tool_create_test_no_commit() {
    let tool_create_args = CreateToolArgs {
        no_commit: true, //look!
        command: vec!["python".to_string(), "echo.py".to_string(), "--test".to_string(), "input.txt".to_string()],
        ..Default::default()
    };
    let cmd = ToolCommands::Create(tool_create_args);
    assert!(handle_tool_commands(&cmd).is_ok());

    //check for files being present
    let output_paths = vec![Path::new("results.txt"), Path::new("workflows/echo/echo.cwl")];
    for output_path in output_paths {
        assert!(output_path.exists());
    }
    //as we did not commit there must be files (exactly 2, the cwl file and the results.txt)
    let repo = Repository::open(".").unwrap();
    assert_eq!(get_modified_files(&repo).len(), 2);
}

#[fstest(repo = true, files = ["tests/test_data/input.txt", "tests/test_data/echo.py"])]
pub fn tool_create_test_no_run() {
    let tool_create_args = CreateToolArgs {
        no_run: true,
        command: vec!["python".to_string(), "echo.py".to_string(), "--test".to_string(), "input.txt".to_string()],
        ..Default::default()
    };
    let cmd = ToolCommands::Create(tool_create_args);
    assert!(handle_tool_commands(&cmd).is_ok());
    assert!(Path::new("workflows/echo/echo.cwl").exists());

    //no uncommitted left?
    let repo = Repository::open(".").unwrap();
    assert!(get_modified_files(&repo).is_empty());
}

#[fstest(repo = true, files = ["tests/test_data/input.txt", "tests/test_data/echo.py"])]
pub fn tool_create_test_is_clean() {
    let tool_create_args = CreateToolArgs {
        is_clean: true,
        command: vec!["python".to_string(), "echo.py".to_string(), "--test".to_string(), "input.txt".to_string()],
        ..Default::default()
    };
    let cmd = ToolCommands::Create(tool_create_args);
    assert!(handle_tool_commands(&cmd).is_ok());
    assert!(Path::new("workflows/echo/echo.cwl").exists());
    assert!(!Path::new("results.txt").exists()); //no result is left as it is cleaned

    //no uncommitted left?
    let repo = Repository::open(".").unwrap();
    assert!(get_modified_files(&repo).is_empty());
}

#[fstest(repo = true, files = ["tests/test_data/input.txt", "tests/test_data/echo.py"])]
pub fn tool_create_test_container_image() {
    let tool_create_args = CreateToolArgs {
        container_image: Some("python".to_string()),
        command: vec!["python".to_string(), "echo.py".to_string(), "--test".to_string(), "input.txt".to_string()],
        ..Default::default()
    };
    let cmd = ToolCommands::Create(tool_create_args);
    assert!(handle_tool_commands(&cmd).is_ok());

    //read file
    let cwl_file = Path::new("workflows/echo/echo.cwl");
    let cwl_contents = read_to_string(cwl_file).expect("Could not read CWL File");
    let cwl: CommandLineTool = serde_yaml::from_str(&cwl_contents).expect("Could not convert CWL");

    let requirements = cwl.requirements.clone().expect("No requirements found!");
    assert_eq!(requirements.len(), 2);

    if let Requirement::DockerRequirement(DockerRequirement::DockerPull(image)) = &requirements[1] {
        assert_eq!(image, "python");
    } else {
        panic!("Requirement is not a Docker pull");
    }

    //no uncommitted left?
    let repo = Repository::open(".").unwrap();
    assert!(get_modified_files(&repo).is_empty());
}

#[fstest(repo = true, files = ["tests/test_data/Dockerfile", "tests/test_data/input.txt", "tests/test_data/echo.py"])]
pub fn tool_create_test_dockerfile() {
    let tool_create_args = CreateToolArgs {
        container_image: Some("Dockerfile".to_string()),
        container_tag: Some("sciwin-client".to_string()),
        command: vec!["python".to_string(), "echo.py".to_string(), "--test".to_string(), "input.txt".to_string()],
        ..Default::default()
    };
    let cmd = ToolCommands::Create(tool_create_args);
    assert!(handle_tool_commands(&cmd).is_ok());

    //read file
    let cwl_file = Path::new("workflows/echo/echo.cwl");
    let cwl_contents = read_to_string(cwl_file).expect("Could not read CWL File");
    let cwl: CommandLineTool = serde_yaml::from_str(&cwl_contents).expect("Could not convert CWL");

    let requirements = cwl.requirements.clone().expect("No requirements found!");
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
    let repo = Repository::open(".").unwrap();
    assert!(get_modified_files(&repo).is_empty());
}

#[fstest(repo = true)]
pub fn test_tool_magic_outputs() {
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
}

#[fstest(repo = true, files = ["tests/test_data/input.txt"])]
pub fn test_tool_magic_stdout() {
    let str = "wc input.txt \\> input.txt";
    let args = CreateToolArgs {
        no_commit: true,
        is_clean: true,
        command: shlex::split(str).unwrap(),
        ..Default::default()
    };

    assert!(create_tool(&args).is_ok());

    let tool = load_tool("workflows/wc/wc.cwl").unwrap();
    assert!(tool.stdout.unwrap() == *"$(inputs.input_txt.path)");
}

#[fstest(repo = true, files = ["tests/test_data/input.txt"])]
pub fn test_tool_magic_arguments(_dir: &Path) {
    let str = "cat input.txt | grep -f input.txt";
    let args = CreateToolArgs {
        no_commit: true,
        is_clean: true,
        command: shlex::split(str).unwrap(),
        ..Default::default()
    };

    assert!(create_tool(&args).is_ok());

    let tool = load_tool("workflows/cat/cat.cwl").unwrap();
    if let Argument::Binding(binding) = &tool.arguments.unwrap()[3] {
        assert!(binding.value_from == Some("$(inputs.input_txt.path)".to_string()));
    } else {
        panic!()
    }
}

#[fstest(repo = true, files = ["tests/test_data/create_dir.py"])]
pub fn test_tool_output_is_dir() {
    let name = "create_dir";
    let command = &["python", "create_dir.py"];
    let args = CreateToolArgs {
        command: command.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
        ..Default::default()
    };

    assert!(create_tool(&args).is_ok());

    let tool = load_tool(format!("workflows/{name}/{name}.cwl")).unwrap();
    assert_eq!(tool.inputs.len(), 0);
    assert_eq!(tool.outputs.len(), 1); //only folder
    assert_eq!(tool.outputs[0].id, "my_directory".to_string());
    assert_eq!(tool.outputs[0].type_, CWLType::Directory);
}

#[fstest(repo = true, files = ["tests/test_data/create_dir.py"])]
pub fn test_tool_output_complete_dir() {
    let name = "create_dir";
    let command = &["python", "create_dir.py"];
    let args = CreateToolArgs {
        outputs: Some(vec![".".into()]), //
        command: command.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
        ..Default::default()
    };

    assert!(create_tool(&args).is_ok());

    let tool = load_tool(format!("workflows/{name}/{name}.cwl")).unwrap();
    assert_eq!(tool.inputs.len(), 0);
    assert_eq!(tool.outputs.len(), 1); //only root folder
    if let Some(binding) = &tool.outputs[0].output_binding {
        assert_eq!(binding.glob, "$(runtime.outdir)".to_string());
    } else {
        panic!("No Binding")
    }

    println!("{:#?}", tool.outputs);
}

#[fstest(repo= true, files=["tests/test_data/script.sh"])]
#[cfg(target_os = "linux")]
pub fn test_shell_script() {
    use s4n::repo::stage_all;

    std::fs::set_permissions("script.sh", <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755)).unwrap();
    let repo = Repository::open(".").unwrap();
    stage_all(&repo).unwrap();

    let name = "script";
    let command = &["./script.sh"];
    let args = CreateToolArgs {
        command: command.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
        ..Default::default()
    };

    let result = create_tool(&args);
    println!("{result:#?}");
    assert!(result.is_ok());

    let tool = load_tool(format!("workflows/{name}/{name}.cwl")).unwrap();
    assert_eq!(tool.inputs.len(), 0);
    assert_eq!(tool.outputs.len(), 0);

    if let Some(req) = &tool.requirements {
        assert_eq!(req.len(), 1);
        if let Requirement::InitialWorkDirRequirement(iwdr) = &req[0] {
            assert_eq!(iwdr.listing[0].entryname, "./script.sh");
        } else {
            panic!("Not an InitialWorkDirRequirement")
        }
    } else {
        panic!("No requirements found")
    }
}

#[fstest(repo = true)]
/// see Issue [#89](https://github.com/fairagro/m4.4_sciwin_client/issues/89)
pub fn test_tool_uncommitted_no_run() {
    let root = env!("CARGO_MANIFEST_DIR");
    fs::copy(format!("{root}/tests/test_data/input.txt"), "input.txt").unwrap(); //repo is not in a clean state now!
    let args = CreateToolArgs {
        command: ["echo".to_string(), "Hello World".to_string()].to_vec(),
        no_run: true,
        ..Default::default()
    };
    //should be ok to not commit changes, as tool does not run
    assert!(create_tool(&args).is_ok());
}

#[fstest(repo = true, files = ["tests/test_data/subfolders.py"])]
/// see Issue [#88](https://github.com/fairagro/m4.4_sciwin_client/issues/88)
pub fn test_tool_output_subfolders() {
    let args = CreateToolArgs {
        command: ["python".to_string(), "subfolders.py".to_string()].to_vec(),
        ..Default::default()
    };
    //should be ok to not commit changes, as tool does not run
    assert!(create_tool(&args).is_ok());
}

#[fstest(repo = true)]
#[cfg(target_os = "linux")]
pub fn tool_create_remote_file() {
    let tool_create_args = CreateToolArgs {
        command: vec![
            "wget".to_string(),
            "https://raw.githubusercontent.com/fairagro/m4.4_sciwin_client/refs/heads/main/README.md".to_string(),
        ],
        ..Default::default()
    };
    let cmd = ToolCommands::Create(tool_create_args);
    assert!(handle_tool_commands(&cmd).is_ok());

    //check file
    assert!(Path::new("README.md").exists());

    //check input
    let tool_path = Path::new("workflows/wget/wget.cwl");
    let tool = load_tool(tool_path).unwrap();
    assert_eq!(tool.inputs.len(), 1);
    assert_eq!(tool.inputs[0].type_, CWLType::File);
}
