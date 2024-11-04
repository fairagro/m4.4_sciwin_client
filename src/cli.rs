use crate::commands::tool::{CreateToolArgs, ToolCommands};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name="s4n", about="Client tool for Scientific Workflow Infrastructure (SciWIn)", long_about=None, version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Provides commands to create and work with CWL CommandLineTools")]
    Tool {
        #[command(subcommand)]
        command: ToolCommands,
    },
    #[command(hide = true)]
    Run(CreateToolArgs),
    Workflow,
    Annotate,
    Execute,
    Sync,
}