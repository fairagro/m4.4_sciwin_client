use crate::{cwl::resolve_filename, util::repo::commit};
use clap::Args;
use git2::Repository;
use log::{info, warn};
use std::{env, fs, path::Path};

#[derive(Args, Debug, Default)]
pub struct RemoveCWLArgs {
    pub file: String,
}

pub fn handle_remove_command(args: &RemoveCWLArgs) -> anyhow::Result<()> {
    let filename = resolve_filename(&args.file).map_err(|e| anyhow::anyhow!("Could not resolve CWL File: {}", e))?;
    remove_cwl_file(&filename)
}

fn remove_cwl_file(filename: impl AsRef<Path>) -> anyhow::Result<()> {
    let filename = filename.as_ref();
    let cwd = env::current_dir()?;
    let repo = Repository::open(cwd)?;

    if filename.exists() && filename.is_file() && filename.extension().is_some_and(|e| e == "cwl") {
        let folder = filename.parent().expect("Can not get parent dir");
        fs::remove_file(filename)?;

        let mut iter = fs::read_dir(folder)?;
        let next = iter.next();
        eprintln!("{next:?}");
        if next.is_none() {
            fs::remove_dir_all(folder)?;
        }

        let message = format!("✔️  Removed CWL file: {}", filename.display());
        info!("{}", message);
        commit(&repo, &message)?;
    } else {
        warn!("File {} does not exist or is not a CWL file.", filename.display());
    }
    Ok(())
}
