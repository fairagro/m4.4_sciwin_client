use crate::cwl::execution::runner::run_commandlinetool;
use crate::{
    cwl::{
        format::format_cwl,
        parser::{self, post_process_cwl},
        requirements::{DockerRequirement, Requirement},
    },
    io::{create_and_write_file, get_qualified_filename},
    repo::{commit, get_modified_files, stage_file},
    util::{error, highlight_cwl, print_list, warn},
};
use clap::{Args, Subcommand};
use colored::Colorize;
use git2::Repository;
use prettytable::{Cell, Row, Table};
use serde_yml::Value;
use std::{env, error::Error, fs, fs::remove_file, path::Path, path::PathBuf};
use walkdir::WalkDir;

pub fn handle_tool_commands(subcommand: &ToolCommands) -> Result<(), Box<dyn Error>> {
    match subcommand {
        ToolCommands::Create(args) => create_tool(args)?,
        ToolCommands::List(args) => list_tools(args)?,
        ToolCommands::Remove(args) => remove_tool(args)?,
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum ToolCommands {
    #[command(about = "Runs commandline string and creates a tool (\x1b[1msynonym\x1b[0m: s4n run)")]
    Create(CreateToolArgs),
    #[command(about = "Lists all tools", visible_alias = "ls")]
    List(ListToolArgs),
    #[command(about = "Remove a tool, e.g. s4n tool rm toolname", visible_alias = "rm")]
    Remove(RemoveToolArgs),
}

#[derive(Args, Debug)]
pub struct CreateToolArgs {
    #[arg(short = 'n', long = "name", help = "A name to be used for this tool")]
    pub name: Option<String>,
    #[arg(
        short = 'c',
        long = "container-image",
        help = "An image to pull from e.g. docker hub or path to a Dockerfile"
    )]
    pub container_image: Option<String>,
    #[arg(short = 't', long = "container-tag", help = "The tag for the container when using a Dockerfile")]
    pub container_tag: Option<String>,

    #[arg(short = 'r', long = "raw", help = "Outputs the raw CWL contents to terminal")]
    pub is_raw: bool,
    #[arg(long = "no-commit", help = "Do not commit at the end of tool creation")]
    pub no_commit: bool,
    #[arg(long = "no-run", help = "Do not run given command")]
    pub no_run: bool,
    #[arg(long = "clean", help = "Deletes created outputs after usage")]
    pub is_clean: bool,
    #[arg(short = 'i', long = "inputs", help = "Force values to be considered as an input.", value_delimiter = ' ')]
    pub inputs: Option<Vec<String>>,
    #[arg(short = 'o', long = "outputs", help = "Force values to be considered as an output.", value_delimiter = ' ')]
    pub outputs: Option<Vec<String>>,
    #[arg(trailing_var_arg = true, help = "Command line call e.g. python script.py [ARGUMENTS]")]
    pub command: Vec<String>,
}

#[derive(Args, Debug)]
pub struct RemoveToolArgs {
    #[arg(trailing_var_arg = true, help = "Remove a tool")]
    pub tool_names: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ListToolArgs {
    #[arg(short = 'a', long = "all", help = "Outputs the tools with inputs and outputs")]
    pub list_all: bool,
}

/// Creates a Common Workflow Language (CWL) CommandLineTool from a command line string like `python script.py --argument`
pub fn create_tool(args: &CreateToolArgs) -> Result<(), Box<dyn Error>> {
    //check if git status is clean
    let cwd = env::current_dir().expect("directory to be accessible");
    if !args.is_raw {
        println!("📂 The current working directory is {}", cwd.to_str().unwrap().green().bold());
    }
    let inputs = args.inputs.as_deref().unwrap_or(&[]);
    let outputs = args.outputs.as_deref().unwrap_or(&[]);

    let repo = Repository::open(&cwd).map_err(|e| format!("Could not find git repository at {:?}: {}", cwd, e))?;
    let modified = get_modified_files(&repo);
    if !modified.is_empty() {
        println!("Uncommitted changes detected:");
        print_list(&modified);
        return Err(error("Uncommitted changes detected").into());
    }

    //parse input string
    if args.command.is_empty() {
        return Err(error("No commandline string given!").into());
    }
    let mut cwl = parser::parse_command_line(args.command.iter().map(|x| x.as_str()).collect());

    if !inputs.is_empty() || !outputs.is_empty() {
        cwl = parser::parse_command_line_fixed_inputs(
            args.command.iter().map(|s| s.as_str()).collect(),
            inputs.iter().map(|s| s.as_str()).collect(),
        );
        cwl = cwl.with_outputs(parser::get_outputs(outputs.to_vec()));
    }

    if let Some(container) = &args.container_image {
        let requirement = if container.contains("Dockerfile") {
            let image_id = if let Some(tag) = &args.container_tag {
                tag
            } else {
                &"sciwin-container".to_string()
            };
            Requirement::DockerRequirement(DockerRequirement::from_file(container, image_id.as_str()))
        } else {
            Requirement::DockerRequirement(DockerRequirement::from_pull(container))
        };

        //push to existing requirements or create new
        if let Some(ref mut vec) = cwl.requirements {
            vec.push(requirement);
        } else {
            cwl = cwl.with_requirements(vec![requirement])
        }
    }
    let mut yaml = "".to_string();

    //only run if not prohibited
    if !args.no_run {
        //execute command
        if inputs.is_empty() && outputs.is_empty() {
            if cwl.execute().is_err() {
                return Err(error(format!("Could not execute command: `{}`!", args.command.join(" ")).as_str()).into());
            }
        } else {
            let path = get_qualified_filename(&cwl.base_command, args.name.clone());
            let path_buf = PathBuf::from(path.clone());
            yaml = cwl.save(&path);
            run_commandlinetool(&mut cwl, None, Some(&path_buf), None)?;
        }

        //check files that changed
        let files = get_modified_files(&repo);
        if files.is_empty() {
            warn("No output produced!")
        } else if !args.is_raw {
            println!("📜 Found changes:");
            print_list(&files);
        }

        if args.is_clean {
            for file in &files {
                remove_file(file).unwrap()
            }
        }

        if !args.no_commit {
            for file in &files {
                //could be cleaned before
                if Path::new(file).exists() {
                    stage_file(&repo, file.as_str()).unwrap();
                }
            }
        }
        //could check here if an output file matches an input string
        if outputs.is_empty() {
            cwl = cwl.with_outputs(parser::get_outputs(files));
        }
        
    } else {
        warn("User requested no run, could not determine outputs!")
    }

    post_process_cwl(&mut cwl);

    //generate yaml
    if !args.is_raw {
        let path = get_qualified_filename(&cwl.base_command, args.name.clone());

        if inputs.is_empty() && outputs.is_empty() {
            yaml = cwl.save(&path);
            
        }
        yaml = format_cwl(&yaml)?;
        match create_and_write_file(path.as_str(), yaml.as_str()) {
            Ok(_) => {
                println!("\n📄 Created CWL file {}", path.green().bold());
                if !args.no_commit {
                    stage_file(&repo, path.as_str()).unwrap();
                    commit(&repo, format!("Execution of `{}`", args.command.join(" ").as_str()).as_str()).unwrap();
                }
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    } else {
        let mut yaml = serde_yml::to_string(&cwl)?;
        yaml = format_cwl(&yaml)?;
        highlight_cwl(yaml.as_str());

        Ok(())
    }
}

pub fn list_tools(args: &ListToolArgs) -> Result<(), Box<dyn Error>> {
    // Print the current working directory
    let cwd = env::current_dir()?;
    println!("📂 Scanning for tools in: {}", cwd.to_str().unwrap_or("Invalid UTF-8").blue().bold());

    // Build the path to the "workflows" folder
    let folder_path = cwd.join("workflows");

    // Create a table
    let mut table = Table::new();

    // Add table headers
    table.add_row(Row::new(vec![
        Cell::new("Tool").style_spec("bFg"),
        Cell::new("Inputs").style_spec("bFg"),
        Cell::new("Outputs").style_spec("bFg"),
    ]));

    // Walk recursively through all directories and subdirectories
    for entry in WalkDir::new(&folder_path).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let file_name = entry.file_name().to_string_lossy();

            // Only process .cwl files
            if let Some(tool_name) = file_name.strip_suffix(".cwl") {
                let mut inputs_list = Vec::new();
                let mut outputs_list = Vec::new();

                // Read the contents of the file
                let file_path = entry.path();
                if let Ok(content) = fs::read_to_string(file_path) {
                    // Parse content
                    if let Ok(parsed_yaml) = serde_yml::from_str::<Value>(&content) {
                        if parsed_yaml.get("class").and_then(|v| v.as_str()) == Some("CommandLineTool") {
                            if args.list_all {
                                // Extract inputs
                                if let Some(inputs) = parsed_yaml.get("inputs") {
                                    for input in inputs.as_sequence().unwrap_or(&vec![]) {
                                        if let Some(id) = input.get("id").and_then(|v| v.as_str()) {
                                            inputs_list.push(format!("{}/{}", tool_name, id));
                                        }
                                    }
                                }
                                // Extract outputs
                                if let Some(outputs) = parsed_yaml.get("outputs") {
                                    for output in outputs.as_sequence().unwrap_or(&vec![]) {
                                        if let Some(id) = output.get("id").and_then(|v| v.as_str()) {
                                            outputs_list.push(format!("{}/{}", tool_name, id));
                                        }
                                    }
                                }
                                // add row to the table
                                table.add_row(Row::new(vec![
                                    Cell::new(tool_name).style_spec("bFg"),
                                    Cell::new(&inputs_list.join(", ")),
                                    Cell::new(&outputs_list.join(", ")),
                                ]));
                            } else {
                                // Print only the tool name if not all details
                                println!("📄 {}", tool_name.green().bold());
                            }
                        }
                    }
                }
            }
        }
    }
    // Print the table
    if args.list_all {
        table.printstd();
    }
    Ok(())
}

pub fn remove_tool(args: &RemoveToolArgs) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let repo = Repository::open(cwd)?;
    let workflows_path = PathBuf::from("workflows");
    for tool in &args.tool_names {
        let mut tool_path = workflows_path.join(tool);
        let file_path = PathBuf::from(tool);
        // Check if the path has an extension
        if file_path.extension().is_some() {
            // If it has an extension, remove it
            let file_stem = file_path.file_stem().unwrap_or_default();
            tool_path = workflows_path.join(file_stem);
        }
        // Check if the directory exists
        if tool_path.exists() && tool_path.is_dir() {
            // Attempt to remove the directory
            fs::remove_dir_all(&tool_path)?;
            println!("{} {}", "Removed tool:".green(), tool_path.display().to_string().green());
            commit(&repo, format!("Deletion of `{}`", tool.as_str()).as_str()).unwrap();
        } else {
            println!("Tool '{}' does not exist.", tool_path.display().to_string().red());
        }
    }
    //we could also remove all tools if no tool is specified but maybe too dangerous
    if args.tool_names.is_empty() {
        println!("Please enter a tool or a list of tools");
    }
    Ok(())
}
