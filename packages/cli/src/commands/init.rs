use crate::{
    config::Config,
    repo::{commit, get_modified_files, initial_commit, stage_all},
};
use anyhow::anyhow;
use clap::Args;
use git2::Repository;
use log::{error, info, warn};
use rust_xlsxwriter::Workbook;
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Args, Debug, Default)]
pub struct InitArgs {
    #[arg(short = 'p', long = "project", help = "Name of the project")]
    project: Option<String>,
    #[arg(short = 'a', long = "arc", help = "Option to create basic arc folder structure")]
    arc: bool,
}

const GITIGNORE_CONTENT: &str = include_str!("../../resources/default.gitignore");

pub fn handle_init_command(args: &InitArgs) -> anyhow::Result<()> {
    if let Err(e) = initialize_project(&args.project, args.arc) {
        git_cleanup(args.project.clone());
        return Err(anyhow!("Could not initialize Project: {e}"));
    }
    Ok(())
}

pub fn initialize_project(folder_name: &Option<String>, arc: bool) -> Result<(), Box<dyn std::error::Error>> {
    let folder = folder_name.as_deref();
    let repo = if is_git_repo(folder) {
        Repository::open(folder.unwrap_or("."))?
    } else {
        init_git_repo(folder)?
    };
    if arc {
        create_arc_folder_structure(folder)?;
    } else {
        create_minimal_folder_structure(folder, false)?;
    }

    write_config(folder)?;

    let files = get_modified_files(&repo);
    if files.is_empty() {
        error!("Nothing to commit");
    } else {
        stage_all(&repo)?;
        if repo.head().is_ok() {
            commit(&repo, "🚀 Initialized Project")?;
        } else {
            initial_commit(&repo)?;
        }
    }

    Ok(())
}

fn write_config(folder: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    // create workflow toml
    let mut cfg = Config::default();
    let dir = if let Some(folder) = folder {
        PathBuf::from(folder)
    } else {
        env::current_dir().unwrap_or_default()
    };
    cfg.workflow.name = dir.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    fs::write(dir.join("workflow.toml"), toml::to_string_pretty(&cfg)?)?;

    Ok(())
}

fn is_git_repo(path: Option<&str>) -> bool {
    // Determine the base directory from the provided path or use the current directory
    let base_dir = match path {
        Some(folder) => Path::new(folder).to_path_buf(),
        None => {
            // Get the current working directory
            env::current_dir().expect("Failed to get current directory")
        }
    };

    Repository::open(&base_dir).is_ok()
}

pub fn git_cleanup(folder_name: Option<String>) {
    // init project in folder name failed, delete it
    if let Some(folder) = folder_name {
        if std::fs::remove_dir_all(&folder).is_ok() {
            info!("Cleaned up failed init in folder: {folder}");
        } else {
            warn!("Failed to clean up folder: {folder}");
        }
    }
    // init project in current folder failed, only delete .git folder
    else {
        let git_folder = ".git";
        if std::fs::remove_dir_all(git_folder).is_ok() {
            info!("Cleaned up .git folder in current directory");
        } else {
            warn!("Failed to clean up .git folder in current directory");
        }
    }
}

pub fn init_git_repo(base_folder: Option<&str>) -> Result<Repository, Box<dyn std::error::Error>> {
    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => env::current_dir()?,
    };

    fs::create_dir_all(&base_dir)?;
    let repo = Repository::init(&base_dir)?;

    let gitignore_path = base_dir.join(PathBuf::from(".gitignore"));
    if !gitignore_path.exists() {
        fs::write(&gitignore_path, GITIGNORE_CONTENT)?;
    }

    //append .s4n folder to .gitignore, whatever it may contains
    let mut gitignore = fs::OpenOptions::new().append(true).open(gitignore_path)?;
    writeln!(gitignore, "\n.s4n")?;

    Ok(repo)
}

pub fn create_minimal_folder_structure(base_folder: Option<&str>, silent: bool) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => env::current_dir()?,
    };

    // Create the base directory
    if !base_dir.exists() {
        fs::create_dir_all(&base_dir)?;
    }

    // Check and create subdirectories
    let workflows_dir = base_dir.join("workflows");
    if !workflows_dir.exists() {
        fs::create_dir_all(&workflows_dir)?;
    }
    File::create(workflows_dir.join(".gitkeep"))?;

    if !silent {
        info!("📂 Project Initialization successful:");
        info!("{} (Base)", base_dir.display());
        info!("  ├── workflows");
    }

    Ok(())
}

pub fn create_arc_folder_structure(base_folder: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => env::current_dir()?,
    };

    // Create the base directory
    if !base_dir.exists() {
        fs::create_dir_all(&base_dir)?;
    }

    create_investigation_excel_file(base_dir.to_str().unwrap_or(""))?;
    // Check and create subdirectories
    let dirs = vec!["studies", "assays", "runs"];
    for dir_name in dirs {
        let dir = base_dir.join(dir_name);
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        File::create(dir.join(".gitkeep"))?;
    }
    //create workflows folder
    create_minimal_folder_structure(base_folder, true)?;

    info!("📂 Project Initialization successful:");
    info!("{} (Base)", base_dir.display());
    info!("  ├── assays");
    info!("  ├── studies");
    info!("  ├── workflows");
    info!("  └── runs");

    Ok(())
}

pub fn create_investigation_excel_file(directory: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Construct the full path for the Excel file
    let excel_path = PathBuf::from(directory).join("isa_investigation.xlsx");

    // Create the directory if it doesn't exist
    fs::create_dir_all(excel_path.parent().unwrap())?;
    // Create a new workbook
    let mut workbook = Workbook::new();

    // Add a worksheet
    let mut worksheet = workbook.add_worksheet();
    worksheet = worksheet.set_name("isa_investigation")?;

    // Define column names
    let columns = vec![
        "ONTOLOGY SOURCE REFERENCE",
        "Term Source Name",
        "Term Source File",
        "Term Source Version",
        "Term Source Description",
        "INVESTIGATION",
        "Investigation Identifier",
        "Investigation Title",
        "Investigation Description",
        "Investigation Submission Date",
        "Investigation Public Release Date",
        "INVESTIGATION PUBLICATIONS",
        "Investigation Publication PubMed ID",
        "Investigation Publication DOI",
        "Investigation Publication Author List",
        "Investigation Publication Title",
        "Investigation Publication Status",
        "Investigation Publication Status Term Accession Number",
        "Investigation Publication Status Term Source REF",
        "INVESTIGATION CONTACTS",
        "Investigation Person Last Name",
        "Investigation Person First Name",
        "Investigation Person Mid Initials",
        "Investigation Person Email",
        "Investigation Person Phone",
        "Investigation Person Fax",
        "Investigation Person Address",
        "Investation Person Affiliation",
        "Investigation Person Roles",
        "Investigation Person Roles Term Accession Number",
        "Investigation Person Roles Term Source REF",
        "Comment[ORCID]",
    ];

    // Calculate column width based on maximum length
    let max_length = columns.iter().map(|s| s.len()).max().unwrap_or(0);

    // Convert usize to f64
    let width_f64: f64 = max_length as f64;

    worksheet.set_column_width(0, width_f64)?;

    // Write column names
    for (i, &col) in columns.iter().enumerate() {
        worksheet.write_string(u32::try_from(i)?, 0, col)?;
    }

    // Save the workbook to the specified file path
    workbook.save(excel_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn test_init_git_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_folder = temp_dir.path().join("my_repo");

        let result = init_git_repo(Some(base_folder.to_str().unwrap()));
        assert!(result.is_ok(), "Expected successful initialization");

        // Verify that the .git directory was created
        let git_dir = base_folder.join(".git");
        assert!(git_dir.exists(), "Expected .git directory to be created");
    }

    #[test]
    #[serial]
    fn test_is_git_repo() {
        let repo_dir = tempdir().unwrap();
        let repo_dir_str = repo_dir.path().to_str().unwrap();
        let repo_dir_string = String::from(repo_dir_str);

        let _ = init_git_repo(Some(&repo_dir_string));
        let result = is_git_repo(Some(&repo_dir_string));
        // Assert that directory is a git repo
        assert!(result, "Expected directory to be a git repo true, got false");
    }

    #[test]
    #[serial]
    fn test_is_not_git_repo() {
        //create directory that is not a git repo
        let no_repo = tempdir().unwrap();

        let no_repo_str = no_repo.path().to_str().unwrap();
        let no_repo_string = String::from(no_repo_str);

        // call is_git repo_function
        let result = is_git_repo(Some(&no_repo_string));

        // assert that it is not a git repo
        assert!(!result, "Expected directory to not be a git repo");
    }
}
