use clap::Parser;
use s4n::{
    cli::{Cli, Commands, CreateDummyArgs, DummyCommands},
    commands::{
        init::handle_init_command,
        tool::{create_tool, handle_tool_commands},
    },
};
use std::{error::Error, process::exit};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    match &args.command {
        Commands::Dummy { command } => match command {
            DummyCommands::Create(args) => create_dummy(args)?,
            DummyCommands::Read => todo!(),
            DummyCommands::Update => todo!(),
            DummyCommands::Delete => todo!(),
        },
        Commands::Tool { command } => handle_tool_commands(command)?,
        Commands::Run(args) => create_tool(args)?,
        Commands::Workflow => todo!(),
        Commands::Annotate => todo!(),
        Commands::Execute => todo!(),
        Commands::Sync => todo!(),
        Commands::Init(args) => handle_init_command(args)?,
        /*
        Commands::Init(init_args) => {
            let project = init_args.project.clone();
            let arc = init_args.arc;
            init_s4n(project, arc)?;
        }
        */
    }
    Ok(())
}

fn create_dummy(args: &CreateDummyArgs) -> Result<(), Box<dyn Error>> {
    println!("Dummy creation called with {:?}", args);
    Ok(())
}
