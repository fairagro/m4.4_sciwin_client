use git2::{Commit, Error, IndexAddOption, Repository, Status, StatusOptions};
use std::path::Path;

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
                if status == Status::WT_MODIFIED || status == Status::WT_NEW {
                    files.push(path);
                }
            }
        }
        Err(e) => panic!("❌ Failed to get repository status: {}", e),
    }
    files
}

pub fn stage_file(repo: &Repository, path: &str) -> Result<(), Error> {
    let mut index = repo.index()?;
    index.add_path(Path::new(path))?;
    index.write()?;
    Ok(())
}

pub fn stage_all(repo: &Repository) -> Result<(), Error> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

pub fn commit(repo: &Repository, message: &str) -> Result<(), Error> {
    let head = repo.head()?;
    let parent = repo.find_commit(head.target().unwrap())?;
    _commit(repo, message, &[&parent])?;
    Ok(())
}

pub fn initial_commit(repo: &Repository) -> Result<(), Error> {
    _commit(repo, "Initial Commit", &[])?;
    Ok(())
}

fn _commit(repo: &Repository, message: &str, parents: &[&Commit<'_>]) -> Result<(), Error> {
    let mut index = repo.index()?;
    let new_oid = index.write_tree()?;
    let new_tree = repo.find_tree(new_oid)?;
    let author = repo.signature()?;
    repo.commit(Some("HEAD"), &author, &author, message, &new_tree, parents)?;
    Ok(())
}