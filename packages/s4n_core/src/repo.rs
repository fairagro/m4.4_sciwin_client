use git2::{Commit, Error, IndexAddOption, Repository, Status, StatusOptions, build::RepoBuilder};
use std::{
    env, fs, iter,
    path::{Path, PathBuf},
};

use crate::remove_ini_section;

pub fn get_modified_files(repo: &Repository) -> Vec<String> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);

    let mut files = vec![];

    match repo.statuses(Some(&mut opts)) {
        Ok(statuses) => {
            // Print the status of the repository
            for entry in statuses.iter() {
                let status = entry.status();
                let path = entry.path().unwrap_or("unknown").to_owned();
                if status.contains(Status::WT_MODIFIED) || status.contains(Status::WT_NEW) {
                    files.push(path);
                }
            }
        }
        Err(e) => panic!("❌ Failed to get repository status: {e}"),
    }
    files
}

pub fn stage_file(repo: &Repository, path: &str) -> Result<(), Error> {
    let mut index = repo.index()?;
    index.add_path(Path::new(path))?;
    index.write()
}

pub fn stage_all(repo: &Repository) -> Result<(), Error> {
    let mut index = repo.index()?;
    index.add_all(iter::once(&"*"), IndexAddOption::DEFAULT, None)?;
    index.write()
}

pub fn commit(repo: &Repository, message: &str) -> Result<(), Error> {
    let head = repo.head()?;
    let parent = repo.find_commit(head.target().unwrap())?;
    commit_impl(repo, message, &[&parent])
}

pub fn initial_commit(repo: &Repository) -> Result<(), Error> {
    commit_impl(repo, "Initial Commit", &[])
}

fn commit_impl(repo: &Repository, message: &str, parents: &[&Commit<'_>]) -> Result<(), Error> {
    let mut index = repo.index()?;
    let new_oid = index.write_tree()?;
    let new_tree = repo.find_tree(new_oid)?;
    let author = repo.signature()?;
    repo.commit(Some("HEAD"), &author, &author, message, &new_tree, parents)?;
    Ok(())
}

pub fn get_submodule_paths(repo: &Repository) -> Result<Vec<PathBuf>, Error> {
    let submodules = repo.submodules()?;
    let paths = submodules.iter().map(|s| s.path().to_path_buf()).collect();
    Ok(paths)
}

pub fn add_submodule(url: &str, branch: &Option<String>, path: &Path) -> Result<(), Error> {
    let current_dir = env::current_dir().unwrap_or(PathBuf::from("."));

    let repo = Repository::open(&current_dir)?;
    //clone and initialize submodule
    if let Some(branch) = branch {
        RepoBuilder::new().branch(branch).clone(url, path)?;
    } else {
        RepoBuilder::new().clone(url, path)?;
    }
    let mut module = repo.submodule(url, path, false)?;

    //set correct branch to submodule
    if let Some(branch) = branch {
        let mut repo = Repository::open(&current_dir)?;
        repo.submodule_set_branch(module.name().unwrap(), branch)?;
        module.sync()?;
    }

    //commit
    module.add_finalize()?;
    let name = module.name().unwrap_or("");
    commit(&repo, &format!("📦 Installed Package {}", name.strip_prefix("packages/").unwrap_or(name)))?;
    Ok(())
}

pub fn remove_submodule(name: &str) -> Result<(), Error> {
    let current_dir = env::current_dir().unwrap_or(PathBuf::from("."));
    let repo = Repository::open(&current_dir)?;

    let module = repo.find_submodule(name)?;
    let path = module.path();

    fs::remove_dir_all(path).ok();

    //remove ksubmodule config
    let prefix = format!("submodule \"{name}\"");
    remove_ini_section(current_dir.join(".git/config"), &prefix).map_err(|_| git2::Error::from_str("Could not delete config entry"))?;
    remove_ini_section(current_dir.join(".gitmodules"), &prefix).map_err(|_| git2::Error::from_str("Could not delete .gitmodulesg entry"))?;

    //stage and commit
    stage_all(&repo)?;
    commit(&repo, &format!("📦 Removed Package {}", name.strip_prefix("packages/").unwrap_or(name)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::initialize_project;
    use fstest::fstest;

    #[fstest(repo = true)]
    fn test_add_remove_submodule() {
        initialize_project(&PathBuf::from(".")).unwrap();

        let result = add_submodule(
            "https://github.com/JensKrumsieck/PorphyStruct",
            &Some("docs".to_string()),
            Path::new("ps"),
        );
        assert!(result.is_ok());

        //check whether a file is present
        assert!(fs::exists("ps/LICENSE").unwrap());

        let result = remove_submodule("ps");
        assert!(result.is_ok());

        //check whether a file is absent
        assert!(!fs::exists("ps/LICENSE").unwrap());
    }
}
