use clap::{Args, Subcommand};
use std::error::Error;

use crate::{
    cwl::{format::format_cwl, wf::Workflow},
    io::{create_and_write_file, get_workflows_folder},
};

pub fn handle_workflow_commands(command: &WorkflowCommands) -> Result<(), Box<dyn Error>> {
    match command {
        WorkflowCommands::Create(args) => create_workflow(args)?,
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum WorkflowCommands {
    #[command(about = "Creates a blank workflow")]
    Create(CreateWorkflowArgs),
}

#[derive(Args, Debug)]
pub struct CreateWorkflowArgs {
    #[arg(help = "A name to be used for this tool")]
    pub name: String,
}

pub fn create_workflow(args: &CreateWorkflowArgs) -> Result<(), Box<dyn Error>> {
    let wf = Workflow::default();

    let mut yaml = serde_yml::to_string(&wf)?;
    yaml = format_cwl(&yaml)?;

    let filename = format!("{}/{}/{}.cwl", get_workflows_folder(), args.name, args.name);

    create_and_write_file(&filename, &yaml).map_err(|e| format!("Could not create workflow {}: {}", args.name, e))?;

    Ok(())
}
