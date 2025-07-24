use crate::util::repo::add_submodule;
use clap::Args;
use reqwest::Url;
use std::{error::Error, path::Path};

#[derive(Args, Debug)]
pub struct InstallPackageArgs {
    #[arg(value_name = "PACKAGE_IDENTIFIER", required = false)]
    pub identifier: String,
    #[arg(short = 'b', long = "branch", help = "Specify branch or commit")]
    pub branch: Option<String>,
}

#[derive(Args, Debug)]
pub struct PackageArgs {
    #[arg(value_name = "PACKAGE_IDENTIFIER", required = false)]
    pub identifier: String,
}

pub fn install_package(url: &str, branch: &Option<String>) -> Result<(), Box<dyn Error>> {
    let url = if url.starts_with("http") {
        url
    } else {
        &format!("https://github.com/{url}")
    };
    let url = url.strip_suffix(".git").unwrap_or(url);

    let url_obj = Url::parse(url)?;

    let package_dir = Path::new("packages");
    let repo_name = url_obj.path().strip_prefix("/").unwrap();
    add_submodule(url, branch, &package_dir.join(repo_name))?;

    Ok(())
}

pub fn remove_package(_package_id: &str) -> Result<(), Box<dyn Error>> {
    Ok(())
}
