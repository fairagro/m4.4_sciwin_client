use crate::commands::{
    execute::ExecuteCommands,
    tool::{CreateToolArgs, ToolCommands},
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
    #[command(about = "Provides commands to create and work with CWL CommandLineTools")]
    Tool {
        #[command(subcommand)]
        command: ToolCommands,
    },
    #[command(hide = true)]
    Run(CreateToolArgs),
    Workflow,
    Annotate,
    #[command(about = "Execution of CWL Files locally or on remote servers (\x1b[1msynonym\x1b[0m: s4n ex)")]
    Execute {
        #[command(subcommand)]
        command: ExecuteCommands,
    },    
    #[command(hide = true)]
    Ex {
        #[command(subcommand)]
        command: ExecuteCommands,
    }, 
    Sync,
}
