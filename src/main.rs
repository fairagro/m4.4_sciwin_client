use clap::Parser;
use s4n::{
    cli::{Cli, Commands},
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
        Commands::Init(args) => handle_init_command(args)?,
        Commands::Tool { command } => handle_tool_commands(command)?,
        Commands::Run(args) => create_tool(args)?,
        Commands::Workflow => todo!(),
        Commands::Annotate => todo!(),
        Commands::Execute => todo!(),
        Commands::Sync => todo!(),
    }
    Ok(())
}