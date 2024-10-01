mod cli;
use clap::Parser;
use cli::{Cli, Commands, CreateDummyArgs, DummyCommands};
use commands::tool::handle_tool_commands;
use s4n::commands::{self, tool::create_tool};

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
        Commands::Run(args) => create_tool(args),
        Commands::Workflow => todo!(),
        Commands::Annotate => todo!(),
        Commands::Execute => todo!(),
        Commands::Sync => todo!(),
    }
}

fn create_dummy(args: &CreateDummyArgs) {
    println!("Dummy creation called with {:?}", args);
}
