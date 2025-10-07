use s4n_core::repo::{commit, stage_file};
use clap::Args;
use git2::Repository;
use log::info;
use s4n_core::io::get_workflows_folder;

#[derive(Args, Debug)]
pub struct SaveArgs {
    #[arg(help = "Name of the workflow to be saved", value_name = "WORKFLOW_NAME")]
    pub name: String,
}

pub fn save_workflow(args: &SaveArgs) -> anyhow::Result<()> {
    //get workflow
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), args.name, args.name);
    let repo = Repository::open(".")?;
    stage_file(&repo, &filename)?;
    let msg = &format!("✅ Saved workflow {}", args.name);
    info!("{msg}");
    commit(&repo, msg)?;
    Ok(())
}
