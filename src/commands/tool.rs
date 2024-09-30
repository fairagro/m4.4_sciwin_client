use crate::{
    repo::{get_modified_files, open_repo},
    tool::parser,
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
    #[command(about = "Runs commandline string and creates a tool")]
    Create(CreateToolArgs),
}

#[derive(Args, Debug)]
pub struct CreateToolArgs {
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

    let mut result = parser::parse_command_line(args.command.iter().map(|x| x.as_str()).collect());

    //execute command
    let status = result.execute();

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
    for file in files {
        result.outputs.push(file);
    }

    //convert to CWL
    let cwl = result.to_cwl();    

    //generate yaml
    let yaml = serde_yml::to_string(&cwl);

    //print / save CWL
    println!("{:?}", yaml);
}
