use clap::{CommandFactory, Parser};
use cwl_execution::{CommandError, ExitCode};
use log::{LevelFilter, error};
use s4n::{
    cli::{generate_completions, Cli, Commands},
    commands::{
        check_git_config, create_tool, handle_annotation_command, handle_execute_commands, handle_init_command, handle_list_command, handle_remove_command, handle_tool_commands, handle_workflow_commands, install_package, remove_package
    },
    util::LOGGER,
};
use std::{error::Error, process::exit};

fn main() {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info)).unwrap();

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

    check_git_config()?;
    match &args.command {
        Commands::Init(args) => Ok(handle_init_command(args)?),
        Commands::Tool { command } => Ok(handle_tool_commands(command)?),
        Commands::Run(args) => Ok(create_tool(args)?),
        Commands::Workflow { command } => Ok(handle_workflow_commands(command)?),
        Commands::Execute { command } => handle_execute_commands(command),
        Commands::Install(args) => Ok(install_package(&args.identifier, &args.branch)?),
        Commands::Uninstall(args) => Ok(remove_package(&args.identifier)?),
        Commands::Annotate { command, tool_name } => handle_annotation_command(command, tool_name),
        Commands::Completions { shell } => Ok(generate_completions(*shell, &mut Cli::command())?),
        Commands::List(args) => Ok(handle_list_command(args)?),
        Commands::Remove(args) => Ok(handle_remove_command(args)?)
    }
}
