use git2::{build::RepoBuilder, Commit, Error, IndexAddOption, Repository, Status, StatusOptions};
use std::{
    env, iter,
    path::{Path, PathBuf},
};

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
    let branch: &str = if let Some(branch) = branch { branch } else { "HEAD" };

    let repo = Repository::open(&current_dir)?;

    //clone and initialize submodule
    RepoBuilder::new().branch(branch).clone(url, path)?;
    let mut module = repo.submodule(url, path, false)?;

    //set correct branch to submodule
    let mut repo = Repository::open(&current_dir)?;
    repo.submodule_set_branch(module.name().unwrap(), branch)?;
    module.sync()?;

    //commit
    module.add_finalize()?;
    commit(&repo, &format!("Installed Package {}", module.name().unwrap_or("")))?;
    Ok(())
}
