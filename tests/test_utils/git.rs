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
    let mut config = Config::open_default()?;
    if config.get_string("user.name").is_err() {
        config.remove_multivar("user.name", ".*").ok();
        config.set_str("user.name", &whoami::username()).expect("Could not set name");
    }

    if config.get_string("user.email").is_err() {
        config
            .set_str("user.email", &format!("{}@example.com", whoami::username()))
            .expect("Could not set email");
    }

    Ok(())
}
