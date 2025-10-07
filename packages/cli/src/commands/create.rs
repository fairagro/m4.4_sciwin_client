use crate::{
    cwl::{Saveable, highlight_cwl},
    print_diff, print_list,
};
use anyhow::anyhow;
use clap::Args;
use colored::Colorize;
use commonwl::execution::{environment::RuntimeEnvironment, io::create_and_write_file, runner::command::run_command};
use commonwl::{
    DefaultValue, Directory,
    format::format_cwl,
    requirements::{DockerRequirement, InitialWorkDirRequirement, NetworkAccess, Requirement, WorkDirItem},
};
use git2::Repository;
use log::{error, info, warn};
use s4n_core::parser::{self, post_process_cwl};
use s4n_core::repo::{get_modified_files, stage_file};
use s4n_core::{
    io::{get_qualified_filename, get_workflows_folder},
    repo::commit,
};
use std::{
    env,
    fs::remove_file,
    path::{Path, PathBuf},
};

pub fn handle_create_command(args: &CreateArgs) -> anyhow::Result<()> {
    if args.command.is_empty() && args.name.is_some() {
        info!("ℹ️  Workflow creation is optional. Creation will be triggered by adding the first connection, too!");
        create_workflow(args)
    } else {
        create_tool(args)
    }
}

#[derive(Args, Debug, Default)]
pub struct CreateArgs {
    #[arg(short = 'n', long = "name", help = "A name to be used for this workflow or tool")]
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
    #[arg(short = 'f', long = "force", help = "Overwrites existing workflow")]
    pub force: bool,
    #[arg(trailing_var_arg = true, help = "Command line call e.g. python script.py [ARGUMENTS]")]
    pub command: Vec<String>,
}

pub fn create_workflow(args: &CreateArgs) -> anyhow::Result<()> {
    let Some(name) = &args.name else {
        return Err(anyhow!("❌ Workflow name is required"));
    };
    //check if workflow already exists
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), name, name);
    let yaml = s4n_core::workflow::create_workflow(&filename, args.force)?;
    info!("📄 Created new Workflow file: {filename}");
    print_diff("", &yaml);

    Ok(())
}

pub fn create_tool(args: &CreateArgs) -> anyhow::Result<()> {
    // Parse input string
    if args.command.is_empty() {
        return Err(anyhow!(
            "No commandline string given! Please provide a name for the workflow or a command to run."
        ));
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
