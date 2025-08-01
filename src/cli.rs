use crate::commands::{AnnotateCommands, CreateToolArgs, ExecuteCommands, InitArgs, InstallPackageArgs, PackageArgs, ToolCommands, WorkflowCommands};
use clap::{Command, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use std::io;

#[derive(Parser, Debug)]
#[command(name="s4n", about=format!(r#"
 _____        _  _    _  _____         _____  _  _               _   
/  ___|      (_)| |  | ||_   _|       /  __ \| |(_)             | |   
\ `--.   ___  _ | |  | |  | |  _ __   | /  \/| | _   ___  _ __  | |_  
 `--. \ / __|| || |/\| |  | | | '_ \  | |    | || | / _ \| '_ \ | __|
/\__/ /| (__ | |\  /\  / _| |_| | | | | \__/\| || ||  __/| | | || |_  
\____/  \___||_| \/  \/  \___/|_| |_|  \____/|_||_| \___||_| |_| \__|

Client tool for Scientific Workflow Infrastructure (SciWIn)
Documentation: https://fairagro.github.io/m4.4_sciwin_client/

Version: {}"#, env!("CARGO_PKG_VERSION"))
, long_about=None, version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Initializes project folder structure and repository")]
    Init(InitArgs),
    #[command(about = "Provides commands to create and work with CWL CommandLineTools")]
    Tool {
        #[command(subcommand)]
        command: ToolCommands,
    },
    #[command(hide = true)]
    Run(CreateToolArgs),
    #[command(about = "Provides commands to create and work with CWL Workflows")]
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommands,
    },
    #[command(about = "Installs a workflow as submodule", visible_alias = "i")]
    Install(InstallPackageArgs),
    #[command(about = "Removes an installed workflow")]
    Uninstall(PackageArgs),
    #[command(about = "Execution of CWL Files locally or on remote servers", visible_alias = "ex")]
    Execute {
        #[command(subcommand)]
        command: ExecuteCommands,
    },
    #[command(about = "Annotate CWL files")]
    Annotate {
        #[command(subcommand)]
        command: Option<AnnotateCommands>,
        /// Name of the tool or workflow to annotate
        #[arg(value_name = "TOOL_NAME", required = false)]
        tool_name: Option<String>,
    },
    #[command(about = "Generate shell completions")]
    Completions {
        #[arg()]
        shell: Shell,
    },
}

pub fn generate_completions<G: Generator>(generator: G, cmd: &mut Command) -> anyhow::Result<()> {
    generate(generator, cmd, cmd.get_name().to_string(), &mut io::stdout());
    Ok(())
}
