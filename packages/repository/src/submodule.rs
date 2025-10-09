use git2::{Error, Repository, build::RepoBuilder};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::{commit, ini, stage_all};

/// Returns a list of paths of all submodules in the repository.
pub fn get_submodule_paths(repo: &Repository) -> Result<Vec<PathBuf>, Error> {
    let submodules = repo.submodules()?;
    let paths = submodules.iter().map(|s| s.path().to_path_buf()).collect();
    Ok(paths)
}

/// Adds a submodule to the current repository, stages the changes, and commits them.
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

/// Removes a submodule from the current repository, stages the changes, and commits them.
pub fn remove_submodule(name: &str) -> Result<(), Error> {
    let current_dir = env::current_dir().unwrap_or(PathBuf::from("."));
    let repo = Repository::open(&current_dir)?;

    let module = repo.find_submodule(name)?;
    let path = module.path();

    fs::remove_dir_all(path).ok();

    //remove ksubmodule config
    let prefix = format!("submodule \"{name}\"");
    ini::remove_section(current_dir.join(".git/config"), &prefix).map_err(|_| git2::Error::from_str("Could not delete config entry"))?;
    ini::remove_section(current_dir.join(".gitmodules"), &prefix).map_err(|_| git2::Error::from_str("Could not delete .gitmodulesg entry"))?;

    //stage and commit
    stage_all(&repo)?;
    commit(&repo, &format!("📦 Removed Package {}", name.strip_prefix("packages/").unwrap_or(name)))?;
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use fstest::fstest;

    #[fstest(repo = true)]
    fn test_add_remove_submodule() {
        let current_dir = env::current_dir().unwrap_or(PathBuf::from("."));
        Repository::init(&current_dir).unwrap();

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
