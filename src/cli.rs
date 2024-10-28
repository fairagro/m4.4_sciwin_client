use crate::commands::{
    execute::ExecuteCommands,
    tool::{CreateToolArgs, ToolCommands},
};
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name="s4n", about="Client tool for Scientific Workflow Infrastructure (SciWIn)", long_about=None, version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    //temporary dummy command to showcase clap usage
    Dummy {
        #[command(subcommand)]
        command: DummyCommands,
    },
    #[command(about = "Provides commands to create and work with CWL CommandLineTools")]
    Tool {
        #[command(subcommand)]
        command: ToolCommands,
    },
    #[command(hide = true)]
    Run(CreateToolArgs),
    Workflow,
    Annotate,
    Execute {
        #[command(subcommand)]
        command: ExecuteCommands,
    },
    Sync,
}

//temporary demo how to use clap, move to commands folder for real commands
#[derive(Debug, Subcommand)]
pub enum DummyCommands {
    #[command(about = "Creates a dummy")]
    Create(CreateDummyArgs),
    Read,
    Update,
    Delete,
}

#[derive(Args, Debug)]
pub struct CreateDummyArgs {
    name: String,
    #[arg(short = 'o', long = "option")]
    option: Option<String>,
}
