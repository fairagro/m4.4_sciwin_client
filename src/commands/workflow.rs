use crate::{
    cwl::{
        format::format_cwl,
        inputs::WorkflowStepInput,
        loader::{load_tool, load_workflow},
        wf::Workflow,
    },
    io::{create_and_write_file, get_workflows_folder},
    repo::{commit, stage_file},
};
use clap::{Args, Subcommand};
use colored::Colorize;
use git2::Repository;
use prettytable::{row, Table};
use std::{error::Error, fs, io::Write, path::Path, vec};

pub fn handle_workflow_commands(command: &WorkflowCommands) -> Result<(), Box<dyn Error>> {
    match command {
        WorkflowCommands::Create(args) => create_workflow(args)?,
        WorkflowCommands::Connect(args) => connect_workflow_nodes(args)?,
        WorkflowCommands::Disconnect(args) => disconnect_workflow_nodes(args)?,
        WorkflowCommands::Save(args) => save_workflow(args)?,
        WorkflowCommands::Status(args) => get_workflow_status(args)?,
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum WorkflowCommands {
    #[command(about = "Creates a blank workflow")]
    Create(CreateWorkflowArgs),
    #[command(about = "Connects a workflow node")]
    Connect(ConnectWorkflowArgs),
    #[command(about = "Disconnects a workflow node")]
    Disconnect(ConnectWorkflowArgs),
    #[command(about = "Saves a workflow")]
    Save(CreateWorkflowArgs),
    #[command(about = "Shows socket status of workflow")]
    Status(CreateWorkflowArgs),
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

pub fn disconnect_workflow_nodes(args: &ConnectWorkflowArgs) -> Result<(), Box<dyn Error>> {
    // Get the workflow
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);
    let mut workflow = load_workflow(&filename)?;

    let from_parts = args.from.split('/').collect::<Vec<_>>();
    let to_parts = args.to.split('/').collect::<Vec<_>>();

    if from_parts[0] == "@inputs" {
        workflow.remove_input_connection(from_parts[1], &args.to)?;
    } else if to_parts[0] == "@outputs" {
        workflow.remove_output_connection(&args.from, to_parts[1])?;
    } else {
        workflow.remove_step_connection(&args.from, &args.to)?;
    }

    // save workflow
    let mut yaml = serde_yml::to_string(&workflow)?;
    yaml = format_cwl(&yaml)?;
    let mut file = fs::File::create(&filename)?;
    file.write_all(yaml.as_bytes())?;
    println!("✔️  Updated Workflow {}!", filename);

    Ok(())
}

pub fn save_workflow(args: &CreateWorkflowArgs) -> Result<(), Box<dyn Error>> {
    //get workflow
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);
    let repo = Repository::open(".")?;
    stage_file(&repo, &filename)?;
    let msg = &format!("✅ Saved workflow {}", args.name);
    println!("{}", msg);
    commit(&repo, msg)?;
    Ok(())
}

pub fn get_workflow_status(args: &CreateWorkflowArgs) -> Result<(), Box<dyn Error>> {
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);
    let path = Path::new(&filename).parent().unwrap_or(Path::new("."));
    let workflow = load_workflow(&filename)?;

    println!("Status report for Workflow {}", filename.green().bold());

    let mut table = Table::new();
    table.set_titles(row![bFg => "Tool", "Inputs", "Outputs"]);

    //check if workflow inputs are all connected
    let input_status = workflow
        .inputs
        .iter()
        .map(|input| {
            if workflow.has_step_input(&input.id) {
                format!("✅    {}", input.id)
            } else if input.default.is_some() {
                format!("🔘    {}", input.id)
            } else {
                format!("❌    {}", input.id)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    //check if workflow outputs are all connected
    let output_status = workflow
        .outputs
        .iter()
        .map(|output| {
            if workflow.has_step_output(&output.output_source) {
                format!("✅    {}", output.id)
            } else {
                format!("❌    {}", output.id)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    table.add_row(row![b -> "<Workflow>", input_status, output_status]);
    table.add_row(row![b -> "Steps:"]);

    for step in &workflow.steps {
        let tool = load_tool(&path.join(&step.run).to_string_lossy())?;

        let input_status = tool
            .inputs
            .iter()
            .map(|input| {
                if step.in_.contains_key(&input.id) {
                    format!("✅    {}", input.id)
                } else if input.default.is_some() {
                    format!("🔘    {}", input.id)
                } else {
                    format!("❌    {}", input.id)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let output_status = tool
            .outputs
            .iter()
            .map(|output| {
                if workflow.steps.iter().any(|s| {
                    s.in_.clone().into_values().any(|v| {
                        let src = match v {
                            WorkflowStepInput::String(str) => str,
                            WorkflowStepInput::Parameter(par) => par.source.unwrap_or_default(),
                        };
                        src == format!("{}/{}", step.id, output.id)
                    })
                }) || workflow.outputs.iter().any(|o| o.output_source == format!("{}/{}", step.id, output.id))
                {
                    format!("✅    {}", output.id)
                } else {
                    format!("❌    {}", output.id)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        table.add_row(row![b -> &step.run, &input_status, &output_status]);
    }

    table.printstd();

    println!("✅ : connected - 🔘 : tool default - ❌ : no connection");

    Ok(())
}
