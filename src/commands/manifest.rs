use crate::config::{Config, Dependency};
use clap::Args;
use cwl_execution::io::copy_dir;
use git2::{build::RepoBuilder, FetchOptions};
use reqwest::Url;
use std::{error::Error, fs, path::Path};
use tempfile::tempdir;

#[derive(Args, Debug, Default)]
pub struct ManifestArgs {
    #[arg(value_name = "CRATE ID OR URL", help = "The Workflow Crate to install")]
    pub id: Option<String>,
    #[arg(long = "git", help = "install from git repo")]
    pub git: bool,
}

pub fn add(args: &ManifestArgs) -> Result<(), Box<dyn Error>> {
    if !args.git {
        todo!()
    } else if let Some(url) = &args.id {
        let uri = Url::parse(url)?;
        let mut segments = uri.path_segments().unwrap();
        let name = segments.next_back().unwrap();

        download_repo(name, url)?;

        let mut manifest: Config = toml::from_str(&fs::read_to_string("workflow.toml")?)?;
        let mut deps = manifest.dependencies.unwrap_or_default();
        deps.insert(
            name.to_string(),
            Dependency {
                git: Some(url.to_string()),
                ..Default::default()
            },
        );
        manifest.dependencies = Some(deps);
        fs::write("workflow.toml", manifest.to_toml()?)?;
    }
    Ok(())
}

fn download_repo(name: &str, url: &str) -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;

    let mut fo: FetchOptions = FetchOptions::new();
    fo.depth(1);
    let repo = RepoBuilder::new().fetch_options(fo).clone(url, dir.path())?;

    if !dir.path().join("workflow.toml").exists() {
        return Err("Repository is not a SciWIn Workflow!".into());
    }

    let head = repo.head()?;
    let branch = head.name().ok_or_else(|| git2::Error::from_str("HEAD is not symbolic ref"))?;
    let branch = branch.strip_prefix("refs/heads/").unwrap_or(branch);

    fs::remove_dir_all(dir.path().join(".git"))?;

    let dest = Path::new(".s4n").join(name).join(branch);

    copy_dir(dir.path(), &dest)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::commands::init::initialize_project;

    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_add_git_repo() {
        let url = "https://github.com/JensKrumsieck/hello_s4n";
        let args = ManifestArgs {
            id: Some(url.to_string()),
            git: true,
        };

        let dir = tempdir().unwrap();
        let current = env::current_dir().unwrap();
        env::set_current_dir(dir.path()).unwrap();
        initialize_project(None, false).unwrap();

        assert!(add(&args).is_ok());
        println!("{}", fs::read_to_string("workflow.toml").unwrap());

        //workflow folder exists
        assert!(Path::new(".s4n").join("hello_s4n").join("master").join("workflows").exists());

        env::set_current_dir(current).unwrap();
    }
}
