use crate::{
    cwl::{
        clt::{DockerRequirement, Requirement},
        format::format_cwl,
        parser,
    },
    io::{create_and_write_file, get_qualified_filename},
    repo::{commit, get_modified_files, open_repo, stage_file},
    util::{error, highlight_cwl, print_list, warn},
};
use clap::{Args, Subcommand};
use colored::Colorize;
use std::{env, error::Error, fs::remove_file, path::Path};

pub fn handle_tool_commands(subcommand: &ToolCommands) -> Result<(), Box<dyn Error>> {
    match subcommand {
        ToolCommands::Create(args) => create_tool(args)?,
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum ToolCommands {
    #[command(about = "Runs commandline string and creates a tool (\x1b[1msynonym\x1b[0m: s4n run)")]
    Create(CreateToolArgs),
}

#[derive(Args, Debug)]
pub struct CreateToolArgs {
    #[arg(short = 'n', long = "name", help = "A name to be used for this tool")]
    pub name: Option<String>,
    #[arg(short = 'c', long = "container-image", help = "An image to pull from e.g. docker hub or path to a Dockerfile")]
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

    #[arg(long = "inputs", help = "List of inputs for the tool", value_delimiter = ' ')]
    pub inputs: Option<Vec<String>>,
    //#[arg(long = "outputs", help = "List of outputs for the tool", value_delimiter = ',', num_args = 1..)] ',' delimiter would work but could not use spaces
    #[arg(long = "outputs", help = "List of outputs for the tool", value_delimiter = ' ')]
    pub outputs: Option<Vec<String>>,

    #[arg(trailing_var_arg = true, help = "Command line call e.g. python script.py [ARGUMENTS]")]
    pub command: Vec<String>,
}

// parsing doesn't work correctly because there is no argument for actual command, would work without this if command is seperated by -- but that might be inconvient for user
fn separate_elements(inputs: Option<Vec<String>>, outputs: Option<Vec<String>>, commands: Vec<String>) -> (Vec<String>, Vec<String>, Vec<String>) {
    // Unwrap inputs and outputs or use empty vectors if None
    let mut inputs_vec = inputs.unwrap_or_default();
    let mut outputs_vec = outputs.unwrap_or_default();
    let mut remaining_commands: Vec<String> = Vec::new();

    let mut after_inputs_flag = false;
    let mut after_outputs_flag = false;

    for cmd in commands {
        if cmd == "--outputs" {
            after_inputs_flag = true;
            continue; // Skip "--outputs" flag itself
        }

        if cmd.contains('.') && !after_outputs_flag {
            if after_inputs_flag {
                outputs_vec.push(cmd);
            } else {
                inputs_vec.push(cmd);
            }
        } else {
            remaining_commands.push(cmd);
            after_outputs_flag = true;
        }
    }

    // Return all three vectors
    (inputs_vec, outputs_vec, remaining_commands)
}

// problem: flag is only in actual command call not in defined inputs, match to command and add flag
fn add_flags_to_inputs_outputs(command: Vec<String>, inputs: Vec<String>, outputs: Vec<String>) -> Vec<String> {
    let mut updated_inputs = Vec::new();
    for input in &inputs {
        if let Some(index) = command.iter().position(|arg| arg == input) {
            if index > 0 && command[index - 1].starts_with('-') {
                updated_inputs.push(command[index - 1].to_string());
            }
            updated_inputs.push(input.to_string());
        }
    }
    for output in &outputs {
        if let Some(index) = command.iter().position(|arg| arg == output) {
            if index > 0 && command[index - 1].starts_with('-') {
                updated_inputs.push(command[index - 1].to_string());
            }
            updated_inputs.push(output.to_string());
        }
    }

    updated_inputs
}

/// Creates a Common Workflow Language (CWL) CommandLineTool from a command line string like `python script.py --argument`
pub fn create_tool(args: &CreateToolArgs) -> Result<(), Box<dyn Error>> {
    let cwd = env::current_dir().expect("directory to be accessible");
    let repo = open_repo(&cwd);
    if !args.is_raw {
        println!("📂 The current working directory is {}", cwd.to_str().unwrap().green().bold());
    }
    let mut cwl;
    let (inputs, outputs, commands) = separate_elements(args.inputs.clone(), args.outputs.clone(), args.command.clone());

    if !inputs.is_empty() || !outputs.is_empty() {
        let updated_inputs = add_flags_to_inputs_outputs(commands.clone(), inputs.clone(), outputs.clone());
        cwl = parser::parse_command_line_inputs(commands.iter().map(|s| s.as_str()).collect(), updated_inputs.iter().map(|s| s.as_str()).collect());
    } 
    else {
        let modified = get_modified_files(&repo);
        if !modified.is_empty() {
            println!("Uncommitted changes detected:");
            print_list(&modified);
            error("Uncommitted changes detected");
        }
        //parse input string
        if args.command.is_empty() {
            error("No commandline string given!");
        }

        cwl = parser::parse_command_line(args.command.iter().map(|x| x.as_str()).collect());
    }

    //only run if not prohibited
    if !args.no_run {
        //execute command
        if cwl.execute().is_err() {
            error(format!("Could not execute command: `{}`!", args.command.join(" ")).as_str());
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
        if outputs.is_empty() {
            println!("outputs is empty");
            //could check here if an output file matches an input string
            cwl = cwl.with_outputs(parser::get_outputs(files));
            //let stdout_file = parser::get_stdout_file();
           // if !stdout_file.is_empty() {
            //    cwl = cwl.with_outputs(parser::get_outputs_stdout(vec![stdout_file.clone()])).with_stdout(&stdout_file);
            //}
        } else {
            //let out: Vec<String> = outputs.iter().map(|s| s.to_string()).collect();
            //cwl = cwl.with_outputs(parser::get_outputs(out));
            cwl = cwl.with_outputs(parser::get_outputs(outputs));
        }
    } else {
        warn("User requested no run, could not determine outputs!")
    }

    //check container usage
    if let Some(container) = &args.container_image {
        let requirement = if container.contains("Dockerfile") {
            let image_id = if let Some(tag) = &args.container_tag { tag } else { &"sciwin-container".to_string() };
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

    //generate yaml
    if !args.is_raw {
        let path = get_qualified_filename(&cwl.base_command, args.name.clone());
        let mut yaml = cwl.save(&path);

        //format
        yaml = format_cwl(&yaml)?;

        match create_and_write_file(path.as_str(), yaml.as_str()) {
            Ok(_) => {
                println!("\n📄 Created CWL file {}", path.green().bold());
                if !args.no_commit {
                    stage_file(&repo, path.as_str()).unwrap();
                    //commit(&repo, format!("Execution of `{}`", &cmd_str).as_str());
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
