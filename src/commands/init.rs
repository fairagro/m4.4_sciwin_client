use clap::Args;
use rust_xlsxwriter::Workbook;
use std::{env, fs, path::Path, path::PathBuf, process::Command};

#[derive(Args, Debug)]
pub struct InitArgs {
    #[arg(short = 'p', long = "project", help = "Name of the project")]
    project: Option<String>,
    #[arg(short = 'a', long = "arc", help = "Option to create basic arc folder structure")]
    arc: bool,
}

pub fn handle_init_command(args: &InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    init_s4n(args.project.clone(), args.arc)?;
    Ok(())
}

pub fn init_s4n(folder_name: Option<String>, arc: bool) -> Result<(), Box<dyn std::error::Error>> {
    let folder = folder_name.as_deref();
    check_git_installation()?;
    let is_git_repo_result = is_git_repo(folder);
    if !is_git_repo_result {
        init_git_repo(folder)?;
    }
    if arc {
        create_arc_folder_structure(folder)?;
    } else {
        create_minimal_folder_structure(folder)?;
    }

    Ok(())
}

pub fn check_git_installation() -> Result<(), Box<dyn std::error::Error>> {
    if Command::new("git").output().is_err() {
        eprintln!("Git is not installed or not in PATH");
        std::process::exit(1);
    }
    Ok(())
}

pub fn is_git_repo(path: Option<&str>) -> bool {
    // Determine the base directory from the provided path or use the current directory
    let base_dir = match path {
        Some(folder) => Path::new(folder).to_path_buf(),
        None => {
            // Get the current working directory
            std::env::current_dir().expect("Failed to get current directory")
        }
    };

    // Build the path to the `.git` directory
    let git_dir = base_dir.join(".git");

    // Check if the `.git` directory exists
    let is_repo = git_dir.exists() && git_dir.is_dir();
    println!("Checking if {} is a git repository: {}", base_dir.display(), is_repo);

    is_repo
}

pub fn init_git_repo(base_folder: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking if git repo: {:?}", base_folder);

    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => env::current_dir().expect("Failed to get current directory"),
    };

    std::fs::create_dir_all(&base_dir)?;

    let git_path = which::which("git").expect("Git not found");
    println!("Current working directory: {}", base_dir.display());

    Command::new(git_path).arg("init").current_dir(&base_dir).output().expect("Failed to execute git init command");

    println!("Git repository initialized successfully");

    let gitignore_path = base_dir.join(PathBuf::from(".gitignore"));
    std::fs::File::create(gitignore_path).expect("Failed to create .gitignore file");

    Ok(())
}

pub fn create_minimal_folder_structure(base_folder: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
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

    let _ = create_investigation_excel_file(base_dir.to_str().unwrap_or(""));
    // Check and create subdirectories
    let assays_dir = base_dir.join("assays");
    if !assays_dir.exists() {
        fs::create_dir_all(&assays_dir)?;
    }
    let studies_dir = base_dir.join("studies");
    if !studies_dir.exists() {
        fs::create_dir_all(&studies_dir)?;
    }
    
    //create workflows folder
    create_minimal_folder_structure(base_folder)?;
    
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

pub fn create_investigation_excel_file(directory: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Construct the full path for the Excel file
    let excel_path = PathBuf::from(directory).join("isa_investigation.xlsx");

    // Create the directory if it doesn't exist
    let _ = std::fs::create_dir_all(excel_path.parent().unwrap());
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
