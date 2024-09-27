mod cli;
use cli::{Cli, Commands, CreateDummyArgs, DummyCommands};

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
        Commands::Tool => todo!(),
        Commands::Workflow => todo!(),
        Commands::Annotate => todo!(),
        Commands::Execute => todo!(),
        Commands::Sync => todo!(),
    }
}

fn create_dummy(args: &CreateDummyArgs) {
    println!("Dummy creation called with {:?}", args);
}
