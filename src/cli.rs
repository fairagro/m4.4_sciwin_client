use crate::init::init_s4n;
use clap::{Args, Parser, Subcommand};

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
    Tool,
    Workflow,
    Annotate,
    Execute,
    Sync,
    Init(InitArgs),
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

#[derive(Args, Debug)]
pub struct InitArgs {
    #[arg(short = 'p', long = "project", help = "Name of the project")]
    project: Option<String>,
    #[arg(
        short = 'a',
        long = "arc",
        help = "Option to create basic arc folder structure"
    )]
    arc: bool,
}

pub fn init(args: &InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    init_s4n(args.project.clone(), Some(args.arc))?;
    Ok(())
}
