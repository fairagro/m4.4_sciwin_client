use crate::{
    cwl::Connectable,
    print_diff,
    util::{
        DotRenderer, MermaidRenderer, get_workflows_folder, render,
        repo::{commit, stage_file},
    },
};
use anyhow::anyhow;
use clap::{Args, Subcommand, ValueEnum};
use commonwl::{Workflow, format::format_cwl, load_workflow};
use cwl_execution::io::create_and_write_file;
use git2::Repository;
use log::{error, info};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn handle_workflow_commands(command: &WorkflowCommands) -> anyhow::Result<()> {
    match command {
        WorkflowCommands::Create(args) => create_workflow(args),
        WorkflowCommands::Connect(args) => connect_workflow_nodes(args),
        WorkflowCommands::Disconnect(args) => disconnect_workflow_nodes(args),
        WorkflowCommands::Save(args) => save_workflow(args),
        WorkflowCommands::Visualize(args) => visualize(&args.filename, &args.renderer, args.no_defaults),
        _ => {
            error!("This command has been removed!");
            Ok(())
        }
    }
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
    #[command(about = "REMOVED!")]
    Status(CreateWorkflowArgs),
    #[command(about = "REMOVED!", visible_alias = "ls")]
    List,
    #[command(about = "REMOVED!", visible_alias = "rm")]
    Remove,
    #[command(about = "Creates a visual representation of a workflow")]
    Visualize(VisualizeWorkflowArgs),
}

#[derive(Args, Debug)]
pub struct CreateWorkflowArgs {
    #[arg(help = "A name to be used for this tool")]
    pub name: String,
    #[arg(short = 'f', long = "force", help = "Overwrites existing workflow")]
    pub force: bool,
}

pub fn create_workflow(args: &CreateWorkflowArgs) -> anyhow::Result<()> {
    let wf = Workflow::default();

    let mut yaml = serde_yaml::to_string(&wf)?;
    yaml = format_cwl(&yaml).map_err(|e| anyhow!("Could not formal yaml: {e}"))?;

    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);

    //removes file first if exists and force is given
    if args.force {
        let path = Path::new(&filename);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }

    create_and_write_file(&filename, &yaml).map_err(|e| anyhow!("❌ Could not create workflow {} at {}: {}", args.name, filename, e))?;
    info!("📄 Created new Workflow file: {filename}");
    print_diff("", &yaml);

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

pub fn connect_workflow_nodes(args: &ConnectWorkflowArgs) -> anyhow::Result<()> {
    //get workflow
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);
    let mut workflow = load_workflow(&filename).map_err(|e| anyhow!("Could not load workflow {filename}: {e}"))?;

    let from_parts = args.from.split('/').collect::<Vec<_>>();
    let to_parts = args.to.split('/').collect::<Vec<_>>();
    if from_parts[0] == "@inputs" {
        workflow
            .add_input_connection(from_parts[1], &args.to)
            .map_err(|e| anyhow!("Could not add input connection from {} to {}: {e}", from_parts[1], args.to))?;
    } else if to_parts[0] == "@outputs" {
        workflow
            .add_output_connection(&args.from, to_parts[1])
            .map_err(|e| anyhow!("Could not add output connection from {} to {}: {e}", args.from, to_parts[1]))?;
    } else {
        workflow
            .add_step_connection(&args.from, &args.to)
            .map_err(|e| anyhow!("Could not add connection from {} to {}:: {e}", args.from, args.to))?;
    }

    //save workflow
    let mut yaml = serde_yaml::to_string(&workflow)?;
    yaml = format_cwl(&yaml).map_err(|e| anyhow!("Could not format yaml: {e}"))?;
    let old = fs::read_to_string(&filename)?;
    let mut file = fs::File::create(&filename)?;
    file.write_all(yaml.as_bytes())?;
    info!("✔️  Updated Workflow {filename}!");
    print_diff(&old, &yaml);

    Ok(())
}

pub fn disconnect_workflow_nodes(args: &ConnectWorkflowArgs) -> anyhow::Result<()> {
    // Get the workflow
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);
    let mut workflow = load_workflow(&filename).map_err(|e| anyhow!("Could not load workflow {filename}: {e}"))?;

    let from_parts = args.from.split('/').collect::<Vec<_>>();
    let to_parts = args.to.split('/').collect::<Vec<_>>();

    if from_parts[0] == "@inputs" {
        workflow
            .remove_input_connection(from_parts[1], &args.to)
            .map_err(|e| anyhow!("Could not remove input connection from {} to {}: {e}", from_parts[1], args.to))?;
    } else if to_parts[0] == "@outputs" {
        workflow
            .remove_output_connection(&args.from, to_parts[1])
            .map_err(|e| anyhow!("Could not remove output connection from {} to {}: {e}", args.from, to_parts[1]))?;
    } else {
        workflow
            .remove_step_connection(&args.from, &args.to)
            .map_err(|e| anyhow!("Could not remove connection from {} to {}:: {e}", args.from, args.to))?;
    }

    // save workflow
    let mut yaml = serde_yaml::to_string(&workflow)?;
    yaml = format_cwl(&yaml).map_err(|e| anyhow!("Could not format yaml: {e}"))?;
    let old = fs::read_to_string(&filename)?;
    let mut file = fs::File::create(&filename)?;
    file.write_all(yaml.as_bytes())?;
    info!("✔️  Updated Workflow {filename}!");
    print_diff(&old, &yaml);

    Ok(())
}

pub fn save_workflow(args: &CreateWorkflowArgs) -> anyhow::Result<()> {
    //get workflow
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);
    let repo = Repository::open(".")?;
    stage_file(&repo, &filename)?;
    let msg = &format!("✅ Saved workflow {}", args.name);
    info!("{msg}");
    commit(&repo, msg)?;
    Ok(())
}

#[derive(Args, Debug)]
pub struct VisualizeWorkflowArgs {
    #[arg(help = "Path to a workflow")]
    pub filename: PathBuf,
    #[arg(short = 'r', long = "renderer", help = "Select a flavor", value_enum, default_value_t = Renderer::Mermaid)]
    pub renderer: Renderer,
    #[arg(long = "no-defaults", help = "Do not print default values", default_value_t = false)]
    pub no_defaults: bool,
}

#[derive(Default, Debug, Clone, ValueEnum)]
pub enum Renderer {
    #[default]
    Mermaid,
    Dot,
}

#[allow(clippy::disallowed_macros)]
pub fn visualize(filename: &PathBuf, renderer: &Renderer, no_defaults: bool) -> anyhow::Result<()> {
    let cwl = load_workflow(filename).map_err(|e| anyhow!("Could mot load Workflow {filename:?}: {e}"))?;

    let code = match renderer {
        Renderer::Dot => render(&mut DotRenderer::default(), &cwl, filename, no_defaults),
        Renderer::Mermaid => render(&mut MermaidRenderer::default(), &cwl, filename, no_defaults),
    }
    .map_err(|e| anyhow!("Could not render visualization for {filename:?} using {renderer:?}: {e}"))?;

    println!("{code}");
    Ok(())
}
