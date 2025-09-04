use crate::{
    util::{
        get_workflows_folder,
        repo::{commit, stage_file},
    },
};
use anyhow::anyhow;
use git2::Repository;
use log::info;
use crate::commands::CreateArgs;

pub fn save_workflow(args: &CreateArgs) -> anyhow::Result<()> {
     let name = args.name.as_deref().ok_or_else(|| anyhow!("❌ Workflow name is required"))?;
    //get workflow
    let filename = format!("{}{}/{}.cwl", get_workflows_folder(), name, name);
    let repo = Repository::open(".")?;
    stage_file(&repo, &filename)?;
    let msg = &format!("✅ Saved workflow {name}");
    info!("{msg}");
    commit(&repo, msg)?;
    Ok(())
}