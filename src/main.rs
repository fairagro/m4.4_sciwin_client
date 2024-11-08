use clap::Parser;
use s4n::cli::{Cli, Commands};
use s4n::commands::execute::handle_execute_commands;
use s4n::error::{CommandError, ExitCode};
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
        if let Some(cmd_err) = e.downcast_ref::<CommandError>() {
            exit(cmd_err.exit_code());
        } else {
            exit(1);
        }
    }
    exit(0);
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    match &args.command {
        Commands::Init(args) => handle_init_command(args)?,
        Commands::Tool { command } => handle_tool_commands(command)?,
        Commands::Run(args) => create_tool(args)?,
        Commands::Workflow => todo!(),
        Commands::Annotate => todo!(),
        Commands::Execute { command } | Commands::Ex { command } => handle_execute_commands(command)?,
        Commands::Sync => todo!(),
    }
    Ok(())
}
