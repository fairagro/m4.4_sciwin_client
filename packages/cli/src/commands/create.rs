use crate::{cwl::highlight_cwl, print_diff, print_list};
use anyhow::anyhow;
use clap::Args;
use colored::Colorize;
use git2::Repository;
use log::{info, warn};
use s4n_core::io::{get_qualified_filename, get_workflows_folder};
use std::{env, path::PathBuf};

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
    let cwl = s4n_core::tool::create_tool(
        &args.command,
        &args.outputs.clone().unwrap_or_default(),
        &args.inputs.clone().unwrap_or_default(),
        args.no_run,
        args.is_clean,
        !args.no_commit,
        args.no_defaults,
    )?;
    if args.no_run {
        warn!("User requested no execution, could not determine outputs!");
    }

    info!("Found outputs:");
    let string_outputs = cwl
        .outputs
        .iter()
        .map(|o| o.output_binding.clone().unwrap_or_default().glob.unwrap_or_default())
        .collect::<Vec<_>>();
    print_list(&string_outputs);

    // Handle container requirements
    let container = if let Some(container_image) = &args.container_image {
        let container = s4n_core::tool::ContainerInfo {
            image: container_image.clone(),
            tag: args.container_tag.clone(),
        };
        Some(container)
    } else {
        None
    };
    let cwl = s4n_core::tool::add_tool_requirements(cwl, container.as_ref(), args.enable_network, &args.mount.clone().unwrap_or_default());

    //save tool
    let path = get_qualified_filename(&cwl.base_command, args.name.clone());
    let yaml = s4n_core::tool::finalize_tool(cwl, &path)?;
    if args.is_raw {
        highlight_cwl(&yaml);
    } else {
        let cwd = env::current_dir()?;
        let repo = Repository::open(&cwd).map_err(|e| anyhow!("Could not find git repository at {cwd:?}: {e}"))?;
        s4n_core::tool::save_tool_to_disk(&yaml, &path, &repo, !args.no_commit)?;
        info!("\n📄 Created CWL file {}", path.green().bold());
    }
    Ok(())
}
