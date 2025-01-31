use clap::{CommandFactory, Parser};
use log::{error, LevelFilter};
use s4n::{
    cli::{generate_completions, Cli, Commands},
    commands::{
        annotate::handle_annotation_command, execute::handle_execute_commands, init::handle_init_command, sync::handle_sync, tool::{create_tool, handle_tool_commands}, workflow::handle_workflow_commands
    },
    error::{CommandError, ExitCode},
    log::LOGGER,
};
use std::{error::Error, process::exit};

fn main() {
    log::set_logger(&LOGGER).map(|_| log::set_max_level(LevelFilter::Info)).unwrap();

    if let Err(e) = run() {
        error!("{e}");
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
        Commands::Init(args) => handle_init_command(args),
        Commands::Tool { command } => handle_tool_commands(command),
        Commands::Run(args) => create_tool(args),
        Commands::Workflow { command } => handle_workflow_commands(command),
        Commands::Annotate { command, tool_name } => handle_annotation_command(command, tool_name),
        Commands::Execute { command } => handle_execute_commands(command),
        Commands::Sync => handle_sync(),
        Commands::Completions { shell } => generate_completions(*shell, &mut Cli::command()),
    }
}
