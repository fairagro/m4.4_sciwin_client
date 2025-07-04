use crate::{
    cwl::Connectable, io::get_workflows_folder, print_diff, repo::{commit, stage_file}
};
use clap::{Args, Subcommand, ValueEnum};
use colored::Colorize;
use cwl::{
    format::format_cwl,
    load_tool, load_workflow,
    wf::{StringOrDocument, Workflow, WorkflowStep},
    CWLDocument,
};
use cwl_execution::io::create_and_write_file;
use git2::Repository;
use log::{error, info};
use prettytable::{row, Cell, Row, Table};
use serde_yaml::Value;
use std::path::PathBuf;
use std::{env, error::Error, fs, io::Write, path::Path, vec};
use walkdir::WalkDir;

pub fn handle_workflow_commands(command: &WorkflowCommands) -> Result<(), Box<dyn Error>> {
    match command {
        WorkflowCommands::Create(args) => create_workflow(args),
        WorkflowCommands::Connect(args) => connect_workflow_nodes(args),
        WorkflowCommands::Disconnect(args) => disconnect_workflow_nodes(args),
        WorkflowCommands::Save(args) => save_workflow(args),
        WorkflowCommands::Status(args) => get_workflow_status(args),
        WorkflowCommands::List(args) => list_workflows(args),
        WorkflowCommands::Remove(args) => remove_workflow(args),
        WorkflowCommands::Visualize(args) => visualize(&args.filename, &args.renderer, args.no_defaults),
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
    #[command(about = "Shows socket status of workflow")]
    Status(CreateWorkflowArgs),
    #[command(about = "List all workflows", visible_alias = "ls")]
    List(ListWorkflowArgs),
    #[command(about = "Remove a workflow", visible_alias = "rm")]
    Remove(RemoveWorkflowArgs),
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

#[derive(Args, Debug, Default)]
pub struct ListWorkflowArgs {
    #[arg(short = 'a', long = "all", help = "Outputs the tools with inputs and outputs")]
    pub list_all: bool,
}

#[derive(Args, Debug)]
pub struct RemoveWorkflowArgs {
    #[arg(trailing_var_arg = true, help = "Remove a workflow")]
    pub rm_workflow: Vec<String>,
}

pub fn create_workflow(args: &CreateWorkflowArgs) -> Result<(), Box<dyn Error>> {
    let wf = Workflow::default();

    let mut yaml = serde_yaml::to_string(&wf)?;
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
    let mut yaml = serde_yaml::to_string(&workflow)?;
    yaml = format_cwl(&yaml)?;
    let old = fs::read_to_string(&filename)?;
    let mut file = fs::File::create(&filename)?;
    file.write_all(yaml.as_bytes())?;
    info!("✔️  Updated Workflow {filename}!");
    print_diff(&old,  &yaml);

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
    let mut yaml = serde_yaml::to_string(&workflow)?;
    yaml = format_cwl(&yaml)?;
    let old = fs::read_to_string(&filename)?;
    let mut file = fs::File::create(&filename)?;
    file.write_all(yaml.as_bytes())?;
    info!("✔️  Updated Workflow {filename}!");
    print_diff(&old,  &yaml);

    Ok(())
}

pub fn save_workflow(args: &CreateWorkflowArgs) -> Result<(), Box<dyn Error>> {
    //get workflow
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);
    let repo = Repository::open(".")?;
    stage_file(&repo, &filename)?;
    let msg = &format!("✅ Saved workflow {}", args.name);
    info!("{msg}");
    commit(&repo, msg)?;
    Ok(())
}

pub fn get_workflow_status(args: &CreateWorkflowArgs) -> Result<(), Box<dyn Error>> {
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);
    let path = Path::new(&filename).parent().unwrap_or(Path::new("."));
    let workflow = load_workflow(&filename)?;

    info!("Status report for Workflow {}", filename.green().bold());

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
        let tool = match &step.run {
            StringOrDocument::String(run) => load_tool(path.join(run))?,
            StringOrDocument::Document(boxed_doc) => match &**boxed_doc {
                CWLDocument::CommandLineTool(doc) => doc.clone(),
                _ => unreachable!(), //see #95
            },
        };
        let input_status = tool
            .inputs
            .iter()
            .map(|input| {
                if step.in_.iter().any(|i| i.id == input.id) {
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
                if workflow
                    .steps
                    .iter()
                    .any(|s| s.in_.clone().iter().any(|v| v.source == Some(format!("{}/{}", step.id, output.id))))
                    || workflow.outputs.iter().any(|o| o.output_source == format!("{}/{}", step.id, output.id))
                {
                    format!("✅    {}", output.id)
                } else {
                    format!("❌    {}", output.id)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let run = if let StringOrDocument::String(run) = &step.run {
            run
        } else {
            &String::from("Inline Document")
        };
        table.add_row(row![b -> run, &input_status, &output_status]);
    }

    table.printstd();

    info!("✅ : connected - 🔘 : tool default - ❌ : no connection");

    Ok(())
}

pub fn list_workflows(args: &ListWorkflowArgs) -> Result<(), Box<dyn Error>> {
    // Print the current working directory
    let cwd = env::current_dir()?;
    info!("📂 Scanning for workflows in: {}", cwd.to_str().unwrap_or("Invalid UTF-8").blue().bold());

    // Build the path to the "workflows" folder
    let folder_path = cwd.join("workflows");

    let mut table = Table::new();

    if args.list_all {
        // Add table headers only if listing all details
        table.add_row(Row::new(vec![
            Cell::new("Workflow").style_spec("bFg"),
            Cell::new("Inputs").style_spec("bFg"),
            Cell::new("Outputs").style_spec("bFg"),
            Cell::new("Steps").style_spec("bFg"),
        ]));
    }

    for entry in WalkDir::new(&folder_path).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let file_name = entry.file_name().to_string_lossy();

            // Only process .cwl files
            if let Some(workflow_name) = file_name.strip_suffix(".cwl") {
                let file_path = entry.path();

                // Read the contents of the file for detailed information
                if let Ok(content) = fs::read_to_string(file_path) {
                    if let Ok(parsed_yaml) = serde_yaml::from_str::<Value>(&content) {
                        if parsed_yaml.get("class").and_then(|v| v.as_str()) == Some("Workflow") {
                            // Extract inputs, outputs, and steps
                            let inputs_list = extract_workflow_list(parsed_yaml.get("inputs"));
                            let outputs_list = extract_workflow_list(parsed_yaml.get("outputs"));
                            let steps_list = extract_step_ids(parsed_yaml.get("steps"));

                            // Format with line breaks
                            let inputs_str = format_with_line_breaks(&inputs_list, 3);
                            let outputs_str = format_with_line_breaks(&outputs_list, 3);
                            let steps_str = format_with_line_breaks(&steps_list, 3);

                            if args.list_all {
                                // Add row to the table
                                table.add_row(Row::new(vec![
                                    Cell::new(workflow_name).style_spec("bFg"),
                                    Cell::new(&inputs_str),
                                    Cell::new(&outputs_str),
                                    Cell::new(&steps_str),
                                ]));
                            } else {
                                // Print only the workflow name if not all details
                                println!("📄 {}", workflow_name.green().bold());
                            }
                        }
                    }
                }
            }
        }
    }

    // Print the table if listing all details
    if args.list_all {
        table.printstd();
    }

    Ok(())
}

/// Helper function to extract IDs of items in a CWL workflow
fn extract_workflow_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Mapping(mapping)) => mapping.keys().filter_map(|key| key.as_str().map(String::from)).collect(),
        Some(Value::Sequence(sequence)) => sequence
            .iter()
            .filter_map(|item| item.get("id").and_then(|id| id.as_str()).map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

/// Extract step IDs from a CWL workflow
fn extract_step_ids(value: Option<&Value>) -> Vec<String> {
    let mut step_ids = Vec::new();

    match value {
        // If steps are in a mapping format (YAML dictionary)
        Some(Value::Mapping(mapping)) => {
            step_ids.extend(mapping.keys().filter_map(|key| key.as_str().map(String::from)));
        }
        // If steps are in an array format (YAML list)
        Some(Value::Sequence(sequence)) => {
            step_ids.extend(
                sequence
                    .iter()
                    .filter_map(|step| step.get("id").and_then(|id| id.as_str()).map(String::from)),
            );
        }
        _ => {}
    }

    step_ids
}

/// Helper function to format a list of strings with line breaks every `max_per_line` items
fn format_with_line_breaks(items: &[String], max_per_line: usize) -> String {
    items
        .chunks(max_per_line)
        .map(|chunk| chunk.join(", "))
        .collect::<Vec<String>>()
        .join("\n")
}

/// Remove a workflow
pub fn remove_workflow(args: &RemoveWorkflowArgs) -> Result<(), Box<dyn Error>> {
    let cwd = env::current_dir()?;
    let repo = Repository::open(cwd)?;
    let workflows_path = PathBuf::from("workflows");
    for wf in &args.rm_workflow {
        let mut wf_path = workflows_path.join(wf);
        let file_path = PathBuf::from(wf);
        // Check if the path has an extension
        if file_path.extension().is_some() {
            // If it has an extension, remove it
            let file_stem = file_path.file_stem().unwrap_or_default();
            wf_path = workflows_path.join(file_stem);
        }
        // Check if the directory exists
        if wf_path.exists() && wf_path.is_dir() {
            // Attempt to remove the directory
            fs::remove_dir_all(&wf_path)?;
            info!("{} {}", "Removed workflow:".green(), wf_path.display().to_string().green());
            commit(&repo, format!("Deletion of `{}`", wf.as_str()).as_str()).unwrap();
        } else {
            error!("Workflow '{}' does not exist.", wf_path.display().to_string().red());
        }
    }
    //we could also remove all tools if no wf is specified but maybe too dangerous
    if args.rm_workflow.is_empty() {
        error!("Please enter a tool or a list of workflows");
    }
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

pub fn visualize(filename: &PathBuf, renderer: &Renderer, no_defaults: bool) -> Result<(), Box<dyn Error>> {
    let cwl = load_workflow(filename)?;

    let code = match renderer {
        Renderer::Dot => dot(&cwl, filename, no_defaults)?,
        Renderer::Mermaid => mermaid(&cwl, filename, no_defaults)?,
    };
    println!("{code}");
    Ok(())
}

static BROWN_LIGHT: &str = "#F8CBAD";
static BROWN_DARK: &str = "#823909";
static GRAY_LIGHT: &str = "#EEEEEE";
static GRAY_DARK: &str = "#818281";
static GREEN_LIGHT: &str = "#C5E0B4";
static GREEN_DARK: &str = "#385723";
static BLUE_LIGHT: &str = "#6FC1B5";
static BLUE_DARK: &str = "#0f9884";
static BLUE_LIGHTER: &str = "#9FD6CE";
static BLUE_LIGHTEST: &str = "#cfeae6";

fn mermaid(cwl: &Workflow, filename: &Path, no_defaults: bool) -> Result<String, Box<dyn Error>> {
    let cfg = format!(
        r#"---
config:
  theme: base
  look: neo
  themeVariables:
    primaryColor: '{GREEN_LIGHT}'
    primaryTextColor: '#231f20'
    secondaryColor: '{GRAY_LIGHT}'
    lineColor: '{GREEN_DARK}'    
    fontSize: 12px
    tertiaryTextColor: '#231f20'
    fontFamily: 'Fira Sans, trebuchet ms, verdana, arial'
---"#
    );

    let mut nodes = vec![cfg.to_string()];
    nodes.push("flowchart TB".to_string());
    nodes.push(format!("  linkStyle default stroke:{GREEN_DARK},stroke-width: 2px;"));
    nodes.push("  subgraph inputs[Workflow Inputs]".to_string());
    nodes.push("    direction TB".to_string());
    for input in &cwl.inputs {
        nodes.push(format!("        {}({})", input.id, input.id));
    }
    nodes.push("  end".to_string());

    nodes.push("  subgraph outputs[Workflow Outputs]".to_string());
    nodes.push("    direction TB".to_string());
    for output in &cwl.outputs {
        nodes.push(format!("        {}({})", output.id, output.id));
    }
    nodes.push("    end".to_string());

    nodes.push(String::new());
    for step in &cwl.steps {
        nodes.push(format!("  {}({})", step.id, step.id));
        nodes.push(format!("  style {} stroke:{GREEN_DARK},stroke-width:2px;", step.id));

        for input in &step.in_ {
            if let Some(src) = &input.source {
                let src_id = src.split("/").next().unwrap();
                nodes.push(format!("  {} --> |{}|{}", src_id, input.id, step.id));
            }
        }

        if !no_defaults {
            if let Some(doc) = load_step(step, filename) {
                for input in &doc.inputs {
                    if !step.in_.iter().any(|i| i.id == input.id) && input.default.is_some() {
                        nodes.push(format!(
                            "  {}_{}({})",
                            step.id,
                            input.id,
                            input.default.as_ref().unwrap().as_value_string()
                        ));
                        nodes.push(format!(
                            "  style {}_{} font-size:9px,fill:{BLUE_LIGHTEST}, stroke:{BLUE_LIGHTER},stroke-width:2px;",
                            step.id, input.id
                        ));
                        nodes.push(format!("  {}_{} -->|{}| {}", step.id, input.id, input.id, step.id));
                    }
                }
            }
        }
    }

    for output in &cwl.outputs {
        let src = output.output_source.split("/").next().unwrap();
        nodes.push(format!("  {} --> |{}|{}", src, output.id, output.id));
    }

    nodes.push(String::new());
    nodes.push(format!("  style inputs fill:{GRAY_LIGHT},stroke-width:2px;"));
    nodes.push(format!("  style outputs fill:{GRAY_LIGHT},stroke-width:2px;"));

    for input in &cwl.inputs {
        nodes.push(format!("  style {} stroke:{BLUE_DARK},fill:{BLUE_LIGHT},stroke-width:2px;", input.id));
    }

    for output in &cwl.outputs {
        nodes.push(format!("  style {} stroke:{BROWN_DARK},fill:{BROWN_LIGHT},stroke-width:2px;", output.id))
    }

    Ok(nodes.join("\n"))
}

fn dot(cwl: &Workflow, filename: &Path, no_defaults: bool) -> Result<String, Box<dyn Error>> {
    let mut nodes = vec![
        "digraph workflow {".to_string(),
        "  rankdir=TB;".to_string(),
        "  bgcolor=\"transparent\";".to_string(),
        "  node [fontname=\"Fira Sans\", style=filled, shape=record, penwidth=2];".to_string(),
        format!("  edge [fontname=\"Fira Sans\", fontsize=\"9\",fontcolor=\"{GRAY_DARK}\",penwidth=2, color=\"{GREEN_DARK}\"]"),
    ];

    nodes.push("  subgraph cluster_inputs {".to_string());
    nodes.push("    label=\"Workflow Inputs\";".to_string());
    nodes.push("    fontname=\"Fira Sans\";".to_string());
    nodes.push("    labelloc=t;".to_string());
    nodes.push("    penwidth=2;".to_string());
    nodes.push("    style=\"filled\";".to_string());
    nodes.push(format!("    color=\"{GRAY_DARK}\";"));
    nodes.push(format!("    fillcolor=\"{GRAY_LIGHT}\";"));
    for input in &cwl.inputs {
        nodes.push(format!(
            "    {} [label=\"{}\", fillcolor=\"{BLUE_LIGHT}\", color=\"{BLUE_DARK}\"];",
            input.id, input.id
        ));
    }
    nodes.push("  }".to_string());

    nodes.push("  subgraph cluster_outputs {".to_string());
    nodes.push("    label=\"Workflow Outputs\";".to_string());
    nodes.push("    fontname=\"Fira Sans\";".to_string());
    nodes.push("    labelloc=b;".to_string());
    nodes.push("    penwidth=2;".to_string());
    nodes.push("    style=filled;".to_string());
    nodes.push(format!("    color=\"{GRAY_DARK}\";"));
    nodes.push(format!("    fillcolor=\"{GRAY_LIGHT}\";"));
    for output in &cwl.outputs {
        nodes.push(format!(
            "    {} [label=\"{}\", fillcolor=\"{BROWN_LIGHT}\", color=\"{BROWN_DARK}\"];",
            output.id, output.id
        ));
    }
    nodes.push("  }".to_string());
    for step in &cwl.steps {
        nodes.push(format!(
            "  {} [label=\"{}\", fillcolor=\"{GREEN_LIGHT}\", color=\"{GREEN_DARK}\"];",
            step.id, step.id
        ));

        for input in &step.in_ {
            if let Some(src) = &input.source {
                let src_id = src.split("/").next().unwrap();
                nodes.push(format!("  {} -> {}[label=\"{}\"];", src_id, step.id, input.id));
            }
        }
        if !no_defaults {
            if let Some(doc) = load_step(step, filename) {
                for input in &doc.inputs {
                    if !step.in_.iter().any(|i| i.id == input.id) && input.default.is_some() {
                        nodes.push(format!(
                            "  {}_{} [label=\"{}\", height=0.25, fontsize=10, color=\"{BLUE_LIGHTER}\", fillcolor=\"{BLUE_LIGHTEST}\"];",
                            step.id,
                            input.id,
                            input.default.as_ref().unwrap().as_value_string()
                        ));
                        nodes.push(format!(
                            "  {}_{} -> {}[label=\"{}\", style=dashed];",
                            step.id, input.id, step.id, input.id
                        ));
                    }
                }
            }
        }
    }

    for output in &cwl.outputs {
        let src = output.output_source.split("/").next().unwrap();
        nodes.push(format!("  {} -> {}[label=\"{}\"];", src, output.id, output.id));
    }

    nodes.push("}".to_string());

    Ok(nodes.join("\n"))
}

fn load_step(step: &WorkflowStep, filename: &Path) -> Option<CWLDocument> {
    match &step.run {
        StringOrDocument::String(f) => {
            let step_path = filename.parent().unwrap_or(Path::new("")).join(f);
            if step_path.exists() {
                Some(serde_yaml::from_str(&fs::read_to_string(step_path).ok()?).ok()?)
            } else {
                None
            }
        }
        StringOrDocument::Document(doc) => Some((**doc).clone()),
    }
}
