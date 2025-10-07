use std::time::Duration;

use git2::{Config, IndexAddOption, Repository};

pub(super) fn initial_commit(repo: &Repository) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    let new_oid = index.write_tree()?;
    let new_tree = repo.find_tree(new_oid)?;
    let author = repo.signature()?;
    repo.commit(Some("HEAD"), &author, &author, "Initial commit", &new_tree, &[])?;
    Ok(())
}

pub(super) fn stage_all(repo: &Repository) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    index.add_all(std::iter::once(&"*"), IndexAddOption::DEFAULT, None)?;
    index.write()
}

pub fn check_git_user() -> Result<(), git2::Error> {
    for i in 0..5 {
        match write_config() {
            Ok(_) => return Ok(()),
            Err(_) => {
                eprintln!("git config is currently being accessed. Attempt #{i}");
                std::thread::sleep(Duration::from_millis(100))
            }
        }
    }

    Ok(())
}

fn write_config() -> Result<(), git2::Error> {
    let mut config = Config::open_default()?;
    if config.get_string("user.name").is_err() {
        config.remove_multivar("user.name", ".*").ok();
        config.set_str("user.name", &whoami::username())?;
    }

    if config.get_string("user.email").is_err() {
        config.set_str("user.email", &format!("{}@example.com", whoami::username()))?;
    }
    Ok(())
}
