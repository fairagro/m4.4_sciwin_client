use crate::{
    cwl::{
        clt::{DockerRequirement, Requirement},
        format::format_cwl,
        parser,
    },
    io::{create_and_write_file, get_qualified_filename},
    repo::{commit, get_modified_files, stage_file},
    util::{error, highlight_cwl, print_list, warn},
};
use clap::{Args, Subcommand};
use colored::Colorize;
use git2::Repository;
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

    #[arg(short = 'i', long = "inputs", help = "Inputs for the tool", value_delimiter = ' ')]
    pub inputs: Option<Vec<String>>,
    #[arg(short = 'o', long = "outputs", help = "Outputs for the tool", value_delimiter = ' ')]
    pub outputs: Option<Vec<String>>,

    #[arg(trailing_var_arg = true, help = "Command line call e.g. python script.py [ARGUMENTS]")]
    pub command: Vec<String>,
}

// problem: flag is only in actual command call not in defined inputs, match to command and add flag
fn add_flags_to_inputs_outputs(command: Vec<String>, inputs: Vec<String>, outputs: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut updated_inputs = Vec::new();
    let mut num_inputs = 0;
    for input in &inputs {
        if let Some(index) = command.iter().position(|arg| arg == input) {
            if index > 0 && command[index - 1].starts_with('-') {
                updated_inputs.push(command[index - 1].to_string());
            }
            updated_inputs.push(input.to_string());
            num_inputs += 1;
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
    let updated_outputs: Vec<String> = outputs.clone().into_iter().filter(|output| !updated_inputs.contains(output)).collect();
    if !inputs.iter().all(|input| updated_inputs.contains(input)) && inputs.len() > num_inputs {
        let mut combined_inputs = updated_inputs.clone();
        for input in &inputs {
            if !combined_inputs.contains(input) {
                combined_inputs.push(input.clone());
            }
        }
        return (combined_inputs, updated_outputs);
    }
    if updated_inputs.len() >= inputs.len() {
        (updated_inputs, updated_outputs)
    } else {
        (inputs, updated_outputs)
    }
}

fn validate_parameters(command: &[String], inputs: &[String], outputs: &[String]) {
    // Check if any parameters in inputs are not found in outputs
    let missing_inputs: Vec<&String> = inputs.iter().filter(|input| !command.contains(*input)).collect();
    if !missing_inputs.is_empty() {
        println!(" \x1b[33m The following inputs are not found in command: {:?} \x1b[0m", missing_inputs);
    }
    // Check if any command parameters after "-" are not in inputs or outputs
    let mut warnings = Vec::new();
    for (i, arg) in command.iter().enumerate() {
        if arg.starts_with('-') && i + 1 < command.len() {
            let next_param = &command[i + 1];
            if !inputs.contains(next_param) && !outputs.contains(next_param) {
                warnings.push(next_param.clone());
            }
        }
    }
    if !warnings.is_empty() {
        println!("⚠️ \x1b[33m May have detected additional input or outputs: {:?} \x1b[0m", warnings);
    }
}

/// Creates a Common Workflow Language (CWL) CommandLineTool from a command line string like `python script.py --argument`
pub fn create_tool(args: &CreateToolArgs) -> Result<(), Box<dyn Error>> {
    //check if git status is clean
    let cwd = env::current_dir().expect("directory to be accessible");
    let repo = Repository::open(&cwd).map_err(|e| format!("Could not find git repository at {:?}: {}", cwd, e))?;
    if !args.is_raw {
        println!("📂 The current working directory is {}", cwd.to_str().unwrap().green().bold());
    }

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

    let mut cwl;
    let updated_inputs;
    let mut updated_outputs = args.outputs.clone().unwrap_or_default();
    let inputs = args.inputs.clone().unwrap_or_default();
    let outputs = args.outputs.clone().unwrap_or_default();

    if !inputs.is_empty() || !outputs.is_empty() {
        validate_parameters(&args.command, &inputs, &outputs);
        (updated_inputs, updated_outputs) = add_flags_to_inputs_outputs(args.command.clone(), inputs.clone(), outputs.clone());
        if updated_inputs.len() >= inputs.len() {
            cwl = parser::parse_command_line_inputs(args.command.iter().map(|s| s.as_str()).collect(), updated_inputs.iter().map(|s| s.as_str()).collect());
        } else {
            cwl = parser::parse_command_line(args.command.iter().map(|x| x.as_str()).collect());
            println!("Else case: old command");
        }
    } else {
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
            if !outputs.is_empty() && files != outputs {
                println!("\n⚠️ The list of outputs: {:?} is differs from the list of changed files: {:?} ", &outputs, &files);
            }
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
        //if no --outputs provided infer output from git even if --inputs provided
        if outputs.is_empty() {
            //could check here if an output file matches an input string
            cwl = cwl.with_outputs(parser::get_outputs(files));
        } else {
            cwl = cwl.with_outputs(parser::get_outputs(updated_outputs));
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
