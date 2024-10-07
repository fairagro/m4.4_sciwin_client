use crate::{
    cwl::{
        clt::{Command, DockerRequirement, Requirement},
        parser,
    },
    repo::{commit, get_modified_files, open_repo, stage_file},
    util::{create_and_write_file, get_filename_without_extension, get_workflows_folder, print_error_and_exit, print_files, warn},
};
use clap::{Args, Subcommand};
use colored::Colorize;
use std::env;

pub fn handle_tool_commands(subcommand: &ToolCommands) {
    match subcommand {
        ToolCommands::Create(args) => create_tool(args),
    }
}

#[derive(Debug, Subcommand)]
pub enum ToolCommands {
    #[command(about = "Runs commandline string and creates a tool (\x1b[1msynonym\x1b[0m: s4n run)")]
    Create(CreateToolArgs),
}

#[derive(Args, Debug)]
pub struct CreateToolArgs {
    #[arg(short = 'n', long = "name", help = "A name to be used for this tool")]
    name: Option<String>,
    #[arg(short = 'd', long = "no-run", help = "Do not run given command")]
    is_dry: bool,
    #[arg(short = 'c', long = "container-image", help = "An image to pull from e.g. docker hub or path to a Dockerfile")]
    container: Option<String>,
    #[arg(short = 't', long = "container-tag", help = "The tag for the container when using a Dockerfile")]
    image_id: Option<String>,
    #[arg(long = "no-commit", help = "Do not commit at the end of tool creation")]
    no_commit: bool,
    #[arg(trailing_var_arg = true, help = "Command line call e.g. python script.py [ARGUMENTS]")]
    command: Vec<String>,
}

pub fn create_tool(args: &CreateToolArgs) {
    //check if git status is clean
    let cwd = env::current_dir().expect("directory to be accessible");
    println!("📂 The current working directory is {}", cwd.to_str().unwrap().green().bold());

    let repo = open_repo(cwd);
    if !get_modified_files(&repo).is_empty() {
        print_error_and_exit("Uncommitted changes detected!", 0);
    }

    //parse input string
    if args.command.is_empty() {
        print_error_and_exit("No commandline string given!", 1);
    }

    let mut cwl = parser::parse_command_line(args.command.iter().map(|x| x.as_str()).collect());

    //only run if not prohibited
    if !args.is_dry {
        //execute command
        match cwl.execute() {
            Ok(_) => {}
            Err(e) => {
                print_error_and_exit(format!("Could not execute command: `{}` - {}!", args.command.join(" "), e).as_str(), 1);
            }
        }

        //check files that changed
        let files = get_modified_files(&repo);
        if files.is_empty() {
            warn("No output produced!")
        } else {
            println!("📜 Found changes:");
            print_files(&files);
        }

        if !args.no_commit {
            for file in &files {
                stage_file(&repo, file.as_str()).unwrap();
            }
        }
        //could check here if an output file matches an input string
        cwl = cwl.with_outputs(parser::get_outputs(files));
    } else {
        warn("User requested no run, could not determine outputs!")
    }

    //check container usage
    if let Some(container) = &args.container {
        let requirement = if container.contains("Dockerfile") {
            let image_id = if let Some(tag) = &args.image_id { tag } else { &"sciwin-container".to_string() };
            Requirement::DockerRequirement(DockerRequirement::from_file(container, image_id.as_str()))
        } else {
            Requirement::DockerRequirement(DockerRequirement::from_pull(container))
        };

        //push to existing requirements or create new
        if let Some(ref mut vec) = cwl.requirements {
            vec.push(requirement);
        } else {
            cwl = cwl.with_requirements(vec![requirement])
        }
    }

    //generate yaml
    let yaml = cwl.to_string();
    //decide over filename
    let mut filename = match cwl.base_command {
        Command::Multiple(cmd) => get_filename_without_extension(cmd[1].as_str()).unwrap_or(cmd[1].clone()),
        Command::Single(cmd) => cmd,
    };

    if let Some(name) = &args.name {
        filename = name.clone();
        if filename.ends_with(".cwl") {
            filename = filename.replace(".cwl", "");
        }
    }

    //save CWL
    let foldername = filename.clone();
    filename.push_str(".cwl");

    let path = get_workflows_folder() + &foldername + "/" + &filename;
    match create_and_write_file(path.as_str(), yaml.as_str()) {
        Ok(_) => {
            println!("\n📄 Created CWL file {}", path.green().bold());
            if !args.no_commit {
                stage_file(&repo, path.as_str()).unwrap();
                commit(&repo, format!("Execution of `{}`", args.command.join(" ").as_str()).as_str()).unwrap()
            }
        }
        Err(e) => print_error_and_exit(format!("Could not create file {} - {}", path.bold(), e).as_str(), 1),
    }
}
