use clap::{Args, Parser, Subcommand};

use crate::commands::tool::ToolCommands;

#[derive(Parser, Debug)]
#[command(name="s4n", about="Client tool for Scientific Workflow Infrastructure (SciWIn)", long_about=None, version)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    //temporary dummy command to showcase clap usage
    Dummy {
        #[command(subcommand)]
        command: DummyCommands,
    },
    Tool {
        #[command(subcommand)]
        command: ToolCommands,
    },
    Workflow,
    Annotate,
    Execute,
    Sync,
}

#[derive(Debug, Subcommand)]
pub(crate) enum DummyCommands {
    #[command(about = "Creates a dummy")]
    Create(CreateDummyArgs),
    Read,
    Update,
    Delete,
}

#[derive(Args, Debug)]
pub(crate) struct CreateDummyArgs {
    name: String,
    #[arg(short = 'o', long = "option")]
    option: Option<String>,
}
