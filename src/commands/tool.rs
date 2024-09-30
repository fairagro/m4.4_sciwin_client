use crate::tool::parser;
use clap::{Args, Subcommand};

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

pub(crate) fn create_tool(args: &CreateToolArgs) {
    let result = parser::parse_command_line(args.command.iter().map(|x| x.as_str()).collect());
    let _status = result.execute();
    println!("{:?}", result);
}
