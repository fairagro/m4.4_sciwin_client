use crate::{
    cwl::{
        clt::{Command, DockerRequirement, Requirement},
        parser,
    },
    repo::{get_modified_files, open_repo},
    util::{create_and_write_file, get_filename_without_extension},
};
use clap::{Args, Subcommand};
use std::{env, process::exit};

pub fn handle_tool_commands(subcommand: &ToolCommands) {
    match subcommand {
        ToolCommands::Create(args) => create_tool(args),
    }
}

#[derive(Debug, Subcommand)]
pub enum ToolCommands {
    #[command(
        about = "Runs commandline string and creates a tool (\x1b[1msynonym\x1b[0m: s4n run)"
    )]
    Create(CreateToolArgs),
}

#[derive(Args, Debug)]
pub struct CreateToolArgs {
    #[arg(short = 'n', long = "name", help = "A name to be used for this tool")]
    name: Option<String>,
    #[arg(short = 'd', long = "dry", help = "Do not run given command")]
    is_dry: bool,
    #[arg(
        short = 'c',
        long = "container",
        help = "An image to pull from e.g. docker hub or path to a Dockerfile"
    )]
    container: Option<String>,
    #[arg(
        trailing_var_arg = true,
        help = "Command line call e.g. python script.py [ARGUMENTS]"
    )]
    command: Vec<String>,
}

pub fn create_tool(args: &CreateToolArgs) {
    //check if git status is clean
    let cwd = env::current_dir().expect("directory to be accessible");
    println!("The current working directory is {:?}", cwd);
    let repo = open_repo(cwd);
    if !get_modified_files(&repo).is_empty() {
        println!("❌ Uncommitted changes detected, aborting ...");
        exit(0)
    }

    //parse input string
    if args.command.is_empty() {
        println!("❌ No commandline string given!");
        exit(0)
    }

    let mut cwl = parser::parse_command_line(args.command.iter().map(|x| x.as_str()).collect());

    //only run if not prohibited
    if !args.is_dry {
        //execute command
        let status = cwl.execute();

        if !status.success() {
            println!(
                "❌ could not execute commandline: {:?}",
                args.command.join(" ")
            )
        }

        //check files that changed
        let files = get_modified_files(&repo);
        if files.is_empty() {
            println!("⚠ No output produced!")
        }

        //could check here if an output file matches an input string
        cwl = cwl.with_outputs(parser::get_outputs(files));
    } else {
        //let them set outputs explicitly in future
        println!("⚠ User requested no run, could not determine outputs!")
    }

    //check container usage
    if let Some(container) = &args.container {
        //TODO: get id somehow
        let requirement = if container.contains("Dockerfile") {
            Requirement::DockerRequirement(DockerRequirement::from_file(container))
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
        Command::Multiple(cmd) => {
            get_filename_without_extension(cmd[1].as_str()).unwrap_or(cmd[1].clone())
        }
        Command::Single(cmd) => cmd,
    };

    if let Some(name) = &args.name {
        filename = name.clone();
        if filename.ends_with(".cwl") {
            filename = filename.replace(".cwl", "");
        }
    }

    //save CWL
    if let Err(e) = create_and_write_file(format!("{filename}.cwl").as_str(), yaml.as_str()) {
        panic!("❌ Could not create file {}.cwl because {}", filename, e);
    }
}
