use crate::{
    commands::list::{list_multiple, list_single_cwl},
    cwl::{Saveable, highlight_cwl},
    parser::{self, post_process_cwl},
    print_list,
    util::{
        get_qualified_filename,
        repo::{commit, get_modified_files, stage_file},
    },
};
use anyhow::anyhow;
use clap::{Args, Subcommand};
use colored::Colorize;
use commonwl::{
    DefaultValue, Directory,
    format::format_cwl,
    requirements::{DockerRequirement, InitialWorkDirRequirement, NetworkAccess, Requirement, WorkDirItem},
};
use cwl_execution::{environment::RuntimeEnvironment, io::create_and_write_file, runner::command::run_command};
use git2::Repository;
use log::{error, info, warn};
use std::{
    env,
    fs::{self, remove_file},
    path::{Path, PathBuf},
};

pub fn handle_tool_commands(subcommand: &ToolCommands) -> anyhow::Result<()> {
    match subcommand {
        ToolCommands::Create(args) => create_tool(args),
        ToolCommands::List(args) => list_tools(args),
        ToolCommands::Remove(args) => remove_tool(args),
    }
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

#[derive(Args, Debug, Default)]
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
    #[arg(long = "no-defaults", help = "Removes default values from inputs")]
    pub no_defaults: bool,
    #[arg(long = "net", alias = "enable-network", help = "Enables network in container")]
    pub enable_network: bool,
    #[arg(short = 'i', long = "inputs", help = "Force values to be considered as an input.", value_delimiter = ' ')]
    pub inputs: Option<Vec<String>>,
    #[arg(
        short = 'o',
        long = "outputs",
        help = "Force values to be considered as an output.",
        value_delimiter = ' '
    )]
    pub outputs: Option<Vec<String>>,
    #[arg(
        short = 'm',
        long = "mount",
        help = "Mounts a directory into the working directory",
        value_delimiter = ' '
    )]
    pub mount: Option<Vec<PathBuf>>,
    #[arg(trailing_var_arg = true, help = "Command line call e.g. python script.py [ARGUMENTS]")]
    pub command: Vec<String>,
}

#[derive(Args, Debug)]
pub struct RemoveToolArgs {
    #[arg(trailing_var_arg = true, help = "Remove a tool")]
    pub tool_names: Vec<String>,
}

#[derive(Args, Debug, Default)]
pub struct ListToolArgs {
    pub name: Option<PathBuf>,
    #[arg(short = 'a', long = "all", help = "Outputs the tools with inputs and outputs")]
    pub list_all: bool,
}

pub fn create_tool(args: &CreateToolArgs) -> anyhow::Result<()> {
    // Parse input string
    if args.command.is_empty() {
        return Err(anyhow!("No commandline string given!"));
    }
    let command = args.command.iter().map(String::as_str).collect::<Vec<_>>();

    // Check if git status is clean
    let cwd = env::current_dir()?;
    if !args.is_raw {
        info!("📂 The current working directory is {}", cwd.to_string_lossy().green().bold());
    }

    let repo = Repository::open(&cwd).map_err(|e| anyhow!("Could not find git repository at {cwd:?}: {e}"))?;
    let modified = get_modified_files(&repo);

    //check for uncommited changes if a run will be made
    if !args.no_run && !modified.is_empty() {
        error!("Uncommitted changes detected:");
        print_list(&modified);
        return Err(anyhow!("Uncommitted changes detected"));
    }

    let mut cwl = parser::parse_command_line(&command);

    // Handle outputs
    let outputs = args.outputs.as_deref().unwrap_or(&[]);
    if !outputs.is_empty() {
        cwl = cwl.with_outputs(parser::get_outputs(outputs));
    }

    // Only run if not prohibited
    if args.no_run {
        warn!("User requested no execution, could not determine outputs!");
    } else {
        // Execute command
        run_command(&cwl, &mut RuntimeEnvironment::default()).map_err(|e| anyhow!("Could not execute command: `{}`: {}!", command.join(" "), e))?;

        // Check files that changed
        let mut files = get_modified_files(&repo);
        files.retain(|f| !modified.contains(f)); //remove files that were changed before run
        if files.is_empty() && outputs.is_empty() {
            warn!("No output produced!");
        } else if !args.is_raw {
            info!("📜 Found changes:");
            print_list(&files);
        }

        if args.is_clean {
            for file in &files {
                remove_file(file)?;
            }
        }

        if !args.no_commit {
            for file in &files {
                let path = Path::new(file);
                if path.exists() {
                    //in case new dir was created
                    if path.is_dir() {
                        let paths = std::fs::read_dir(path)?;
                        for entry in paths {
                            let entry = entry?;
                            let file_path = entry.path();
                            if file_path.is_file()
                                && let Err(e) = stage_file(&repo, file_path.to_str().unwrap())
                            {
                                eprintln!("Error staging file '{}': {}", file_path.display(), e);
                            }
                        }
                    } else {
                        stage_file(&repo, file.as_str())?;
                    }
                }
            }
        }
        // Add outputs if not specified
        if outputs.is_empty() {
            cwl = cwl.with_outputs(parser::get_outputs(&files));
        }
    }

    //add fixed inputs
    if let Some(fixed_inputs) = &args.inputs {
        parser::add_fixed_inputs(&mut cwl, &fixed_inputs.iter().map(String::as_str).collect::<Vec<_>>())
            .map_err(|e| anyhow!("Could not gather fixed inputs: {e}"))?;
    }

    // Handle container requirements
    if let Some(container) = &args.container_image {
        let requirement = if container.contains("Dockerfile") {
            let image_id = args.container_tag.as_deref().unwrap_or("sciwin-container");
            Requirement::DockerRequirement(DockerRequirement::from_file(container, image_id))
        } else {
            Requirement::DockerRequirement(DockerRequirement::from_pull(container))
        };

        cwl = cwl.append_requirement(requirement);
    }

    if args.enable_network {
        cwl = cwl.append_requirement(Requirement::NetworkAccess(NetworkAccess { network_access: true }));
    }

    if let Some(mounts) = &args.mount {
        let entries = mounts.iter().filter_map(|m| {
            if m.is_dir() {
                Some(WorkDirItem::FileOrDirectory(Box::new(DefaultValue::Directory(Directory::from_path(m)))))
            } else {
                eprintln!("{} is not a directory and has been skipped!", m.display());
                None
            }
        });

        if let Some(iwdr) = cwl.get_requirement_mut::<InitialWorkDirRequirement>() {
            iwdr.listing.extend(entries);
        } else {
            let iwdr = InitialWorkDirRequirement { listing: entries.collect() };
            if !iwdr.listing.is_empty() {
                cwl = cwl.append_requirement(Requirement::InitialWorkDirRequirement(iwdr));
            }
        }
    }

    if args.no_defaults {
        for input in &mut cwl.inputs {
            if input.default.is_some() {
                input.default = None;
                info!("Removed default value from input: {}", input.id);
            }
        }
    }

    post_process_cwl(&mut cwl);

    let path = get_qualified_filename(&cwl.base_command, args.name.clone());
    let mut yaml = cwl.prepare_save(&path);
    yaml = format_cwl(&yaml).map_err(|e| anyhow!("Failed to format CWL: {e}"))?;
    if args.is_raw {
        highlight_cwl(&yaml);
    } else {
        match create_and_write_file(&path, &yaml) {
            Ok(()) => {
                info!("\n📄 Created CWL file {}", path.green().bold());
                if !args.no_commit {
                    stage_file(&repo, &path)?;
                    commit(&repo, &format!("🪄 Creation of `{path}`"))?;
                }
            }
            Err(e) => return Err(anyhow!("Creation of File {path} failed: {e}")),
        }
    }
    Ok(())
}

#[allow(clippy::disallowed_macros)]
pub fn list_tools(args: &ListToolArgs) -> anyhow::Result<()> {
    if let Some(name) = &args.name {
        if name.exists() && name.is_file() {
            list_single_cwl(name)?;
        } else if name.is_dir() {
            list_multiple(name, args.list_all)?;
        }
    } else {
        list_multiple(env::current_dir()?, args.list_all)?;
    }
    Ok(())
}

pub fn remove_tool(args: &RemoveToolArgs) -> anyhow::Result<()> {
    if args.tool_names.is_empty() {
        error!("No tool provided!");
        return Err(anyhow!("No Tool provided!"));
    }

    let cwd = env::current_dir()?;
    let repo = Repository::open(cwd)?;
    let workflows_path = PathBuf::from("workflows");
    for tool in &args.tool_names {
        let mut tool_path = workflows_path.join(tool);
        let file_path = PathBuf::from(tool);

        if file_path.extension().is_some() {
            // If it has an extension, remove it
            let file_stem = file_path.file_stem().unwrap_or_default();
            tool_path = workflows_path.join(file_stem);
        }

        if tool_path.exists() && tool_path.is_dir() {
            fs::remove_dir_all(&tool_path)?;
            info!("{} {}", "🗑️ Removed ".green(), tool_path.to_string_lossy().green());
            commit(&repo, format!("🗑️ Removed `{tool}`").as_str()).unwrap();
        } else {
            error!("Tool '{}' does not exist.", tool_path.to_string_lossy().red());
        }
    }

    Ok(())
}
