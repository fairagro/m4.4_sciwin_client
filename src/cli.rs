use crate::commands::{
    execute::ExecuteCommands,
    init::InitArgs,
    tool::{CreateToolArgs, ToolCommands},
    workflow::WorkflowCommands
};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name="s4n", about="Client tool for Scientific Workflow Infrastructure (SciWIn)", long_about=None, version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Initializes project folder structure and repository")]
    Init(InitArgs),
    #[command(about = "Provides commands to create and work with CWL CommandLineTools")]
    Tool {
        #[command(subcommand)]
        command: ToolCommands,
    },
    #[command(hide = true)]
    Run(CreateToolArgs),
    #[command(about = "Provides commands to create and work with CWL Workflows")]
    Workflow{
        #[command(subcommand)]
        command: WorkflowCommands
    },
    Annotate,
    #[command(about = "Execution of CWL Files locally or on remote servers", visible_alias = "ex")]
    Execute {
        #[command(subcommand)]
        command: ExecuteCommands,
    },
    Sync,
}
