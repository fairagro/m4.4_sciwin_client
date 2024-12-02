use crate::{
    cwl::{format::format_cwl, loader::load_workflow, wf::Workflow},
    io::{create_and_write_file, get_workflows_folder}, repo::{commit, stage_file},
};
use clap::{Args, Subcommand};
use git2::Repository;
use std::{error::Error, fs, io::Write, path::Path};

pub fn handle_workflow_commands(command: &WorkflowCommands) -> Result<(), Box<dyn Error>> {
    match command {
        WorkflowCommands::Create(args) => create_workflow(args)?,
        WorkflowCommands::Connect(args) => connect_workflow_nodes(args)?,
        WorkflowCommands::Save(args) => save_workflow(args)?,
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum WorkflowCommands {
    #[command(about = "Creates a blank workflow")]
    Create(CreateWorkflowArgs),
    #[command(about = "Connects a workflow node")]
    Connect(ConnectWorkflowArgs),
    #[command(about = "Saves a workflow")]
    Save(CreateWorkflowArgs),
}

#[derive(Args, Debug)]
pub struct CreateWorkflowArgs {
    #[arg(help = "A name to be used for this tool")]
    pub name: String,
    #[arg(short = 'f', long = "force", help = "Overwrites existing workflow")]
    pub force: bool,
}

pub fn create_workflow(args: &CreateWorkflowArgs) -> Result<(), Box<dyn Error>> {
    let wf = Workflow::default();

    let mut yaml = serde_yml::to_string(&wf)?;
    yaml = format_cwl(&yaml)?;

    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);

    //removes file first if exists and force is given
    if args.force {
        let path = Path::new(&filename);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }

    create_and_write_file(&filename, &yaml).map_err(|e| format!("❌ Could not create workflow {} at {}: {}", args.name, filename, e))?;
    println!("📄 Created new Workflow file: {}", filename);

    Ok(())
}

#[derive(Args, Debug)]
pub struct ConnectWorkflowArgs {
    #[arg(help = "Name of the workflow name to be altered")]
    pub name: String,
    #[arg(short = 'f', long = "from", help = "Starting Node: [tool]/[output]")]
    pub from: String,
    #[arg(short = 't', long = "to", help = "Ending Node: [tool]/[input]")]
    pub to: String,
}

pub fn connect_workflow_nodes(args: &ConnectWorkflowArgs) -> Result<(), Box<dyn Error>> {
    //get workflow
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);
    let mut workflow = load_workflow(&filename)?;

    let from_parts = args.from.split('/').collect::<Vec<_>>();
    let to_parts = args.to.split('/').collect::<Vec<_>>();
    if from_parts[0] == "@inputs" {
        workflow.add_input_connection(from_parts[1], &args.to)?;
    } else if to_parts[0] == "@outputs" {
        workflow.add_output_connection(&args.from, to_parts[1])?;
    } else {
        workflow.add_step_connection(&args.from, &args.to)?;
    }

    //save workflow
    let mut yaml = serde_yml::to_string(&workflow)?;
    yaml = format_cwl(&yaml)?;
    let mut file = fs::File::create(&filename)?;
    file.write_all(yaml.as_bytes())?;
    println!("✔️  Updated Workflow {}!", filename);

    Ok(())
}

fn save_workflow(args: &CreateWorkflowArgs) -> Result<(), Box<dyn Error>> {
    //get workflow
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);
    let repo = Repository::open(".")?;
    stage_file(&repo, &filename)?;
    let msg = &format!("✅ Saved workflow {}", args.name);
    println!("{}", msg);
    commit(&repo, msg)?;
    Ok(())
}
