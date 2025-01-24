use clap::Parser;
use s4n::{
    cli::{Cli, Commands},
    commands::{
        execute::handle_execute_commands,
        init::handle_init_command,
        tool::{create_tool, handle_tool_commands},
        workflow::handle_workflow_commands, 
        annotate::{handle_annotate_commands, annotate_default}, 
    },
    error::{CommandError, ExitCode},
};
use std::{error::Error, process::exit};
use tokio::runtime::Builder;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
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
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()?;


    match &args.command {
        Commands::Init(args) => handle_init_command(args)?,
        Commands::Tool { command } => handle_tool_commands(command)?,
        Commands::Run(args) => create_tool(args)?,
        Commands::Workflow { command } => handle_workflow_commands(command)?,
        Commands::Annotate { command, tool_name } => {
            if let Some(subcommand) = command {
                runtime.block_on(handle_annotate_commands(subcommand))?;
            } else if let Some(name) = tool_name {
                annotate_default(name)?;
            } else {
                eprintln!("Error: No subcommand or tool name provided for annotate.");
            }
        }
        Commands::Execute { command } => handle_execute_commands(command)?,
        Commands::Sync => todo!(),
    }
    Ok(())
}
