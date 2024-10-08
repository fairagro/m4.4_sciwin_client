mod cli;
mod init;
use cli::{init, Cli, Commands, CreateDummyArgs, DummyCommands};

use clap::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match &args.command {
        Commands::Dummy { command } => match command {
            DummyCommands::Create(args) => create_dummy(args)?,
            DummyCommands::Read => todo!(),
            DummyCommands::Update => todo!(),
            DummyCommands::Delete => todo!(),
        },
        Commands::Tool => todo!(),
        Commands::Workflow => todo!(),
        Commands::Annotate => todo!(),
        Commands::Execute => todo!(),
        Commands::Sync => todo!(),
        Commands::Init(args) => init(args)?,
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

fn create_dummy(args: &CreateDummyArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("Dummy creation called with {:?}", args);
    Ok(())
}
