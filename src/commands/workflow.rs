use clap::{Args, Subcommand};
use std::{error::Error, fs, io::Write, path::Path, vec};

use crate::{
    cwl::{
        clt::CommandLineTool,
        format::format_cwl,
        wf::{Workflow, WorkflowStep},
    },
    io::{create_and_write_file, get_workflows_folder},
};

pub fn handle_workflow_commands(command: &WorkflowCommands) -> Result<(), Box<dyn Error>> {
    match command {
        WorkflowCommands::Create(args) => create_workflow(args)?,
        WorkflowCommands::Connect(args) => connect_workflow_nodes(args)?,
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum WorkflowCommands {
    #[command(about = "Creates a blank workflow")]
    Create(CreateWorkflowArgs),
    #[command(about = "Connects a workflow node")]
    Connect(ConnectWorkflowArgs),
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
    let path = Path::new(&filename);
    if !path.exists() {
        return Err(format!("❌ Workflow {} does not exist, yet!", filename).into());
    }
    let workflow_contents = fs::read_to_string(path)?;
    let mut workflow: Workflow = serde_yml::from_str(&workflow_contents)?;

    //handle from
    let from_parts = args.from.split('/').collect::<Vec<_>>();
    let from_filename = format!("{}{}/{}.cwl", get_workflows_folder(), from_parts[0], from_parts[0]);
    let from_path = Path::new(&from_filename);
    if !from_path.exists() {
        return Err(format!("❌ Tool {} does not exist. Cannot not create node from {} in Workflow {}!", from_filename, from_filename, filename).into());
    }
    let from_contents = fs::read_to_string(from_path)?;
    let from_tool: CommandLineTool = serde_yml::from_str(&from_contents).map_err(|e| format!("❌ Could not read CommandLineTool {}: {}", from_filename, e))?;
    if from_tool.inputs.iter().find(|i| i.id == from_parts[1]).is_none() {
        return Err(format!(
            "❌ Tool {} does not have input {}. Cannot not create node from {} in Workflow {}!",
            from_filename, from_parts[1], from_filename, filename
        )
        .into());
    }
    let from_outputs = from_tool.outputs.iter().map(|o| o.id.clone()).collect::<Vec<_>>();

    //create step
    let workflow_step = WorkflowStep {
        id: from_parts[0].to_string(),
        run: format!("../{}/{}.cwl", from_parts[0], from_parts[0]),
        in_: vec![],
        out: from_outputs,
    };

    workflow.steps.push(workflow_step);


    //save workflow
    let mut yaml = serde_yml::to_string(&workflow)?;
    yaml = format_cwl(&yaml)?;
    let mut file = fs::File::create(filename)?;
    file.write_all(yaml.as_bytes())?;

    Ok(())
}
