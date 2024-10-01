use rust_xlsxwriter::Workbook;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn init_s4n(
    folder_name: Option<String>,
    arc: Option<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let folder = folder_name.as_deref().unwrap_or("").to_string();
    let _ = create_minimal_folder_structure(Some(folder.clone()));
    check_git_installation()?;
    if folder != "" {
        let is_git_repo_result = is_git_repo(Some(folder.clone()));
        if !is_git_repo_result {
            init_git_repo(Some(folder.clone()))?;
        }
        if arc.is_some() && arc.unwrap_or(true) {
            let _ = create_arc_folder_structure(Some(folder.clone()));
        }
    } else {
        let is_git_repo_result = is_git_repo(None);
        if !is_git_repo_result {
            init_git_repo(None)?;
        }
        if arc.is_some() && arc.unwrap_or(true) {
            let _ = create_arc_folder_structure(None);
        }
    }

    Ok(())
}

fn check_git_installation() -> Result<(), Box<dyn std::error::Error>> {
    if !Command::new("git").output().is_ok() {
        eprintln!("Git is not installed or not in PATH");
        std::process::exit(1);
    }
    Ok(())
}

fn is_git_repo(base_folder: Option<String>) -> bool {
    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => {
            // Get the current working directory
            let current_dir = env::current_dir().expect("Failed to get current directory");
            current_dir
        }
    };
    println!("Base dir {}", base_dir.display());
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .current_dir(base_dir)
        .output()
        .expect("Failed to execute git command");

    output.status.success()
}

fn init_git_repo(base_folder: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking if git repo: {:?}", base_folder);

    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => {
            // Get the current working directory
            let current_dir = env::current_dir().expect("Failed to get current directory");
            current_dir
        }
    };
    let git_path = which::which("git").ok().expect("Git not found");
    println!("Current working directory: {}", base_dir.display());

    Command::new(git_path)
        .arg("init")
        .current_dir(base_dir)
        .output()
        .expect("Failed to execute git init command");

    println!("Git repository initialized successfully");

    let gitignore_path = PathBuf::from(".gitignore");
    fs::File::create(&gitignore_path).expect("Failed to create .gitignore file");

    Ok(())
}

fn create_minimal_folder_structure(
    base_folder: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => {
            // Get the current working directory
            let current_dir = env::current_dir()?;
            current_dir
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
    let tools_dir = base_dir.join("workflows").join("tools");
    if !tools_dir.exists() {
        fs::create_dir_all(&tools_dir)?;
    }
    let wf_dir = base_dir.join("workflows").join("wf");
    if !wf_dir.exists() {
        fs::create_dir_all(&wf_dir)?;
    }

    println!("Folder structure created successfully:");
    println!("{} (Base)", base_dir.display());
    println!("  ├── workflows");
    println!("│   └── wf/");
    println!("│   └── tools/");

    Ok(())
}

fn create_arc_folder_structure(
    base_folder: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => {
            // Get the current working directory
            let current_dir = env::current_dir()?;
            current_dir
        }
    };

    // Create the base directory
    if !base_dir.exists() {
        fs::create_dir_all(&base_dir)?;
    }

    create_investigation_excel_file(&base_dir.to_str().unwrap_or(""))?;
    // Check and create subdirectories
    let assays_dir = base_dir.join("assays");
    if !assays_dir.exists() {
        fs::create_dir_all(&assays_dir)?;
    }
    let studies_dir = base_dir.join("studies");
    if !studies_dir.exists() {
        fs::create_dir_all(&studies_dir)?;
    }
    let workflows_dir = base_dir.join("workflows");
    if !workflows_dir.exists() {
        fs::create_dir_all(&workflows_dir)?;
    }
    let runs_dir = base_dir.join("runs");
    if !runs_dir.exists() {
        fs::create_dir_all(&runs_dir)?;
    }

    println!("Folder structure created successfully:");
    println!("{} (Base)", base_dir.display());
    println!("  ├── assays");
    println!("  ├── studies");
    println!("  ├── workflows");
    println!("  └── runs");

    Ok(())
}

fn create_investigation_excel_file(directory: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Construct the full path for the Excel file
    let excel_path = PathBuf::from(directory).join("isa_investigation.xlsx");

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&excel_path.parent().unwrap())?;
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
