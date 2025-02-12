use crate::repo::{commit, get_modified_files, initial_commit, stage_all};
use clap::Args;
use git2::Repository;
use log::{error, info};
use rust_xlsxwriter::Workbook;
use std::{
    env,
    fs::{self, File},
    path::{Path, PathBuf},
};

#[derive(Args, Debug, Default)]
pub struct InitArgs {
    #[arg(short = 'p', long = "project", help = "Name of the project")]
    project: Option<String>,
    #[arg(short = 'a', long = "arc", help = "Option to create basic arc folder structure")]
    arc: bool,
}

pub fn handle_init_command(args: &InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    init_s4n(args.project.clone(), args.arc).map_err(|e| format!("Could not init {}", e))?;
    Ok(())
}

pub fn init_s4n(folder_name: Option<String>, arc: bool) -> Result<(), Box<dyn std::error::Error>> {
    let folder = folder_name.as_deref();
    let repo = if !is_git_repo(folder) {
        init_git_repo(folder)?
    } else {
        Repository::open(folder.unwrap_or("."))?
    };
    if arc {
        create_arc_folder_structure(folder)?;
    } else {
        create_minimal_folder_structure(folder, false)?;
    }

    let files = get_modified_files(&repo);
    if !files.is_empty() {
        stage_all(&repo)?;
        if repo.head().is_ok() {
            commit(&repo, "Created Project using `s4n init`")?;
        } else {
            initial_commit(&repo)?;
        }
    } else {
        error!("Nothing to commit");
    }

    Ok(())
}

pub fn is_git_repo(path: Option<&str>) -> bool {
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

pub fn init_git_repo(base_folder: Option<&str>) -> Result<Repository, Box<dyn std::error::Error>> {
    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => env::current_dir().expect("Failed to get current directory"),
    };

    fs::create_dir_all(&base_dir)?;
    let repo = Repository::init(&base_dir).expect("Failed to execute git init command");

    let gitignore_path = base_dir.join(PathBuf::from(".gitignore"));
    if !gitignore_path.exists() {
        File::create(gitignore_path).expect("Failed to create .gitignore file");
    }

    Ok(repo)
}

pub fn create_minimal_folder_structure(base_folder: Option<&str>, silent: bool) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => {
            // Get the current working directory
            env::current_dir().expect("Failed to get current directory")
        }
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
        info!("📂 s4n project initialisation successfully:");
        info!("{} (Base)", base_dir.display());
        info!("  ├── workflows");
    }

    Ok(())
}

pub fn create_arc_folder_structure(base_folder: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => {
            // Get the current working directory
            env::current_dir().expect("Failed to get current directory")
        }
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

    info!("📂 s4n project initialisation successfully:");
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
        worksheet.write_string(i as u32, 0, col)?;
    }

    // Save the workbook to the specified file path
    workbook.save(excel_path)?;

    Ok(())
}
