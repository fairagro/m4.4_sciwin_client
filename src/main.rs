mod cli;
use s4n::commands;
use cli::{Cli, Commands, CreateDummyArgs, DummyCommands};
use commands::tool::handle_tool_commands;
use clap::Parser;

fn main() {
    let args = Cli::parse();

    match &args.command {
        Commands::Dummy { command } => match command {
            DummyCommands::Create(args) => create_dummy(args),
            DummyCommands::Read => todo!(),
            DummyCommands::Update => todo!(),
            DummyCommands::Delete => todo!(),
        },
        Commands::Tool { command } => handle_tool_commands(command),
        Commands::Workflow => todo!(),
        Commands::Annotate => todo!(),
        Commands::Execute => todo!(),
        Commands::Sync => todo!(),
    }
}

fn create_dummy(args: &CreateDummyArgs) {
    println!("Dummy creation called with {:?}", args);
}
