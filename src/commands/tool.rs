use crate::{
    cwl::{clt::Command, parser},
    repo::{get_modified_files, open_repo},
    util::{create_and_write_file, get_filename_without_extension},
};
use clap::{Args, Subcommand};
use std::env;

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
    #[arg(short = 'n', long = "name")]
    name: Option<String>,
    #[arg(trailing_var_arg = true)]
    command: Vec<String>,
}

pub fn create_tool(args: &CreateToolArgs) {
    //check if git status is clean
    let cwd = env::current_dir().expect("current directory does not exist or is not accessible");
    println!("The current working directory is {:?}", cwd);
    let repo = open_repo(cwd);
    if !get_modified_files(&repo).is_empty() {
        panic!("❌ Uncommitted changes detected, aborting ...")
    }

    //parse input string
    if args.command.is_empty() {
        panic!("❌ No commandline string given!")
    }

    let mut cwl = parser::parse_command_line(args.command.iter().map(|x| x.as_str()).collect());

    //execute command
    let status = cwl.execute();

    if !status.success() {
        panic!(
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

    //generate yaml
    let yaml = cwl.to_string();
    //decide over filename
    let filename = match cwl.base_command {
        Command::Multiple(cmd) => {
            get_filename_without_extension(cmd[1].as_str()).unwrap_or(cmd[1].clone())
        }
        Command::Single(cmd) => cmd,
    };

    //save CWL
    if let Err(e) = create_and_write_file(format!("{filename}.cwl").as_str(), yaml.as_str()) {
        panic!("❌ Could not create file {}.cwl because {}", filename, e);
    }
}
