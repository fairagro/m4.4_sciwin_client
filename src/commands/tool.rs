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
use std::{env, error::Error, fs::remove_file, path::Path, path::PathBuf, io::Write, fs, io};
use walkdir::WalkDir;

pub fn handle_tool_commands(subcommand: &ToolCommands) -> Result<(), Box<dyn Error>> {
    match subcommand {
        ToolCommands::Create(args) => create_tool(args)?,
        ToolCommands::Ls => list_tools()?,
        ToolCommands::Rm(args) => remove_tool(args)?,
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum ToolCommands {
    #[command(about = "Runs commandline string and creates a tool (\x1b[1msynonym\x1b[0m: s4n run)")]
    Create(CreateToolArgs),
    #[command(about = "Lists all tools")]
    Ls,
    #[command(about = "Remove a tool, e.g. s4n tool rm toolname")]
    Rm(ToolArgs),
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

    #[arg(trailing_var_arg = true, help = "Command line call e.g. python script.py [ARGUMENTS]")]
    pub command: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ToolArgs {
    #[arg(trailing_var_arg = true, help = "Remove a tool")]
    pub tool: Vec<String>,
}

/// Creates a Common Workflow Language (CWL) CommandLineTool from a command line string like `python script.py --argument`
pub fn create_tool(args: &CreateToolArgs) -> Result<(), Box<dyn Error>> {
    //check if git status is clean
    let cwd = env::current_dir().expect("directory to be accessible");
    if !args.is_raw {
        println!("📂 The current working directory is {}", cwd.to_str().unwrap().green().bold());
    }

    let repo = open_repo(cwd);
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

    let mut cwl = parser::parse_command_line(args.command.iter().map(|x| x.as_str()).collect());

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
        //could check here if an output file matches an input string
        cwl = cwl.with_outputs(parser::get_outputs(files));
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


pub fn list_tools() -> Result<(), Box<dyn std::error::Error>> {
    // Print the current working directory
    let cwd = env::current_dir()?;
    println!("📂 The following tools were found in {}", cwd.to_str().unwrap().blue().bold());

    // Build the path to the "workflows" folder
    let folder_path = cwd.join("workflows");

    // Walk recursively through all directories and subdirectories
    for entry in WalkDir::new(&folder_path).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            // Print the file names, could also print directories or something else
            let file_name = entry.file_name().to_string_lossy();
            println!("📄 {}", file_name.green().bold());
        }
    }
    Ok(())
}



pub fn remove_tool(args: &ToolArgs) -> Result<(), Box<dyn std::error::Error>> {
    let workflows_path = PathBuf::from("workflows");
    for tool in &args.tool {
        let mut tool_path = workflows_path.join(tool);
        let file_path = PathBuf::from(tool);
       // Check if the path has an extension
        if file_path.extension().is_some() {
            // If it has an extension, remove it
            let file_stem = file_path.file_stem().unwrap_or_default(); 
            tool_path= workflows_path.join(file_stem);
        } 
        
        // Check if the directory exists
        if tool_path.exists() && tool_path.is_dir() {
            // Ask for user confirmation
            print!("Are you sure you want to remove the tool '{}'? (y/n): ", tool.red());
            io::stdout().flush()?; 

            // Read user input
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();

            if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                // Attempt to remove the directory
                fs::remove_dir_all(&tool_path)?;
                println!("{} {}", "Removed tool:".green(), tool_path.display().to_string().green());
            } else {
                println!("{}", "Operation canceled. Tool not removed.".green());
            }
        } else {
            println!("Tool '{}' does not exist.", tool_path.display().to_string().red());
        }
    }

    if args.tool.is_empty(){
        println!("Please enter a tool or a list of tools. Alternatively remove everything? Do we want that? Ask");
    } 
    Ok(())
}