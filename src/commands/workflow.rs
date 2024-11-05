use clap::{Args, Subcommand};
use std::{collections::HashMap, error::Error, fs, hash::Hash, io::Write, path::Path};

use crate::{
    cwl::{
        clt::{CommandInputParameter, CommandLineTool},
        format::format_cwl,
        loader::{load_tool, load_workflow},
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
    let mut workflow = load_workflow(&filename)?;

    let from_parts = args.from.split('/').collect::<Vec<_>>();
    let to_parts = args.to.split('/').collect::<Vec<_>>();
    if from_parts[0] == "@inputs".to_string() {
        input_connection(from_parts[1], &args.to, &mut workflow, &filename)?;
    } else if to_parts[0] == "$outputs".to_string() {
    } else {
        step_connection(&args.from, &args.to, &mut workflow, &filename)?;
    }

    //save workflow
    let mut yaml = serde_yml::to_string(&workflow)?;
    yaml = format_cwl(&yaml)?;
    let mut file = fs::File::create(&filename)?;
    file.write_all(yaml.as_bytes())?;
    println!("✔️  Updated Workflow {}!", filename);

    Ok(())
}

pub fn input_connection(from_input: &str, to: &String, workflow: &mut Workflow, filename: &str) -> Result<(), Box<dyn Error>> {
    let to_parts = to.split('/').collect::<Vec<_>>();

    let to_filename = format!("{}{}/{}.cwl", get_workflows_folder(), to_parts[0], to_parts[0]);
    let to_tool = load_tool(&to_filename)?;
    let to_slot = to_tool.inputs.iter().find(|i| i.id == to_parts[1]).expect("No slut");

    //register intput
    if !workflow.has_input(from_input) {
        workflow.inputs.push(CommandInputParameter::default().with_id(from_input).with_type(to_slot.type_.clone()));
    }

    if !workflow.has_step(to_parts[0]) {
        //create step
        let workflow_step = WorkflowStep {
            id: to_parts[0].to_string(),
            run: format!("../{}/{}.cwl", to_parts[0], to_parts[0]),
            in_: HashMap::new(),
            out: vec![],
        };
        workflow.steps.push(workflow_step.clone());
        println!("➕ Added step {} to workflow {}", to_parts[0], filename);
    }
    //add input in step
    workflow
        .steps
        .iter_mut()
        .find(|step| step.id == to_parts[0])
        .unwrap()
        .in_
        .insert(to_parts[1].to_string(), from_input.to_string());

    println!("➕ Added connection from inputs.{} to {} in workflow {}", from_input, to, filename);

    Ok(())
}

pub fn step_connection(from: &String, to: &String, workflow: &mut Workflow, filename: &str) -> Result<(), Box<dyn Error>> {
    //handle from
    let from_parts = from.split('/').collect::<Vec<_>>();
    //check if step already exists and create if not
    if !workflow.has_step(from_parts[0]) {
        let from_filename = format!("{}{}/{}.cwl", get_workflows_folder(), from_parts[0], from_parts[0]);
        let from_tool: CommandLineTool = load_tool(&from_filename)?;
        let from_outputs = from_tool.outputs.iter().map(|o| o.id.clone()).collect::<Vec<_>>();
        if !from_outputs.contains(&from_parts[1].to_string()) {
            return Err(format!(
                "❌ Tool {} does not have output `{}`. Cannot not create node from {} in Workflow {}!",
                from_parts[0], from_parts[1], from_filename, filename
            )
            .into());
        }

        //create step
        let workflow_step = WorkflowStep {
            id: from_parts[0].to_string(),
            run: format!("../{}/{}.cwl", from_parts[0], from_parts[0]),
            in_: HashMap::new(),
            out: from_outputs,
        };
        workflow.steps.push(workflow_step);
        println!("➕ Added step {} to workflow {}", from_parts[0], filename);
    } else {
        println!("🔗 Found step {} in workflow {}. Not changing that!", from_parts[0], filename);
    }

    //handle to
    let to_parts = to.split('/').collect::<Vec<_>>();
    //check if step exists
    let to_step = workflow.get_step(to_parts[0]).expect(
        format!(
            "❌ Step {} does not exists. Cannot not create node to {} in Workflow {}! Try adding {} first!",
            to_parts[0], to_parts[0], filename, to_parts[0]
        )
        .as_str(),
    );
    Ok(())
}
