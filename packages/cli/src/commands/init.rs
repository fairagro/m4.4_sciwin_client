use anyhow::anyhow;
use clap::Args;
use log::{info, warn};
use rust_xlsxwriter::Workbook;
use std::{
    env,
    fs::{self, File},
    path::PathBuf,
};

#[derive(Args, Debug, Default)]
pub struct InitArgs {
    #[arg(short = 'p', long = "project", help = "Name of the project")]
    pub project: Option<String>,
    #[arg(short = 'a', long = "arc", help = "Option to create basic arc folder structure")]
    pub arc: bool,
}

pub fn handle_init_command(args: &InitArgs) -> anyhow::Result<()> {
    let base_dir = match &args.project {
        Some(folder) => PathBuf::from(folder),
        None => env::current_dir()?,
    };

    if args.arc {
        create_arc_folder_structure(args.project.as_deref()).map_err(|e| anyhow::anyhow!("Could not create ARC folder structure: {e}"))?;
    }
    if let Err(e) = s4n_core::project::initialize_project(&base_dir) {
        git_cleanup(args.project.clone());
        return Err(anyhow!("Could not initialize Project: {e}"));
    }
    info!("📂 Project Initialization successful");
    Ok(())
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

fn create_arc_folder_structure(base_folder: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}

fn create_investigation_excel_file(directory: &str) -> Result<(), Box<dyn std::error::Error>> {
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
    use calamine::{Reader, Xlsx, open_workbook};
    use s4n_core::project::initialize_project;
    use serial_test::serial;
    use std::path::Path;
    use tempfile::{NamedTempFile, tempdir};
    use test_utils::check_git_user;

    #[test]
    #[serial]
    fn test_create_investigation_excel_file() {
        //create directory
        let temp_dir = tempdir().unwrap();
        let directory = temp_dir.path().to_str().unwrap();

        //call the function
        assert!(
            create_investigation_excel_file(directory).is_ok(),
            "Unexpected function create_investigation_excel fail"
        );

        //verify file exists
        let excel_path = PathBuf::from(directory).join("isa_investigation.xlsx");
        assert!(excel_path.exists(), "Excel file does not exist");

        let workbook: Xlsx<_> = open_workbook(excel_path).expect("Cannot open file");

        let sheets = workbook.sheet_names();

        //verify sheet name
        assert_eq!(sheets[0], "isa_investigation", "Worksheet name is incorrect");
    }

    #[test]
    #[serial]
    fn test_create_arc_folder_structure() {
        let temp_dir = tempdir().unwrap();

        let base_folder = Some(temp_dir.path().to_str().unwrap());

        let result = create_arc_folder_structure(base_folder);

        assert!(result.is_ok(), "Expected successful initialization");

        let expected_dirs = vec!["assays", "studies", "runs"];
        //assert that folders are created
        for dir in &expected_dirs {
            let full_path = PathBuf::from(temp_dir.path()).join(dir);
            assert!(full_path.exists(), "Directory {dir} does not exist");
        }
    }

    #[test]
    #[serial]
    fn test_create_arc_folder_structure_invalid() {
        //this test only gives create_arc_folder_structure a file instead of a directory
        let temp_file = NamedTempFile::new().unwrap();
        let base_path = Some(temp_file.path().to_str().unwrap());

        let result = create_arc_folder_structure(base_path);
        //result should not be okay because of invalid input
        assert!(result.is_err(), "Expected failed initialization");
    }

    #[test]
    #[serial]
    fn test_cleanup_no_folder() {
        let temp_dir = tempdir().expect("Failed to create a temporary directory");
        eprintln!("Temporary directory: {temp_dir:?}");
        check_git_user().unwrap();
        // Create a subdirectory in the temporary directory
        std::fs::create_dir_all(&temp_dir).expect("Failed to create test directory");

        // Change to the temporary directory
        env::set_current_dir(&temp_dir).unwrap();
        eprintln!("Current directory changed to: {}", env::current_dir().unwrap().display());

        let git_folder = ".git";
        std::fs::create_dir(git_folder).unwrap();
        assert!(Path::new(git_folder).exists());

        git_cleanup(None);
        assert!(!Path::new(git_folder).exists());
    }

    #[test]
    #[serial]
    fn test_init_s4n_without_folder_with_arc() {
        //create a temp dir
        let temp_dir = tempdir().expect("Failed to create a temporary directory");
        eprintln!("Temporary directory: {:?}", temp_dir.path());
        check_git_user().unwrap();

        // Change current dir to the temporary directory to not create workflow folders etc in sciwin-client dir
        env::set_current_dir(temp_dir.path()).unwrap();
        eprintln!("Current directory changed to: {}", env::current_dir().unwrap().display());

        // test method without folder name and do not create arc folders
        let folder_name: Option<String> = None;
        let arc = true;

        let result = handle_init_command(&InitArgs { project: folder_name, arc });

        // Assert results is ok and folders exist/ do not exist
        assert!(result.is_ok());

        assert!(PathBuf::from("workflows").exists());
        assert!(PathBuf::from(".git").exists());
        assert!(PathBuf::from("assays").exists());
        assert!(PathBuf::from("studies").exists());
        assert!(PathBuf::from("runs").exists());
    }

    #[test]
    #[serial]
    fn test_cleanup_failed_init() {
        let temp_dir = tempdir().unwrap();
        let test_folder = temp_dir.path().join("my_repo");
        let result = initialize_project(&test_folder);
        if let Err(e) = &result {
            eprintln!("Error initializing git repo: {}", e);
        }
        assert!(result.is_ok(), "Expected successful initialization");
        assert!(Path::new(&test_folder).exists());
        let git_dir = test_folder.join(".git");
        assert!(git_dir.exists(), "Expected .git directory to be created");
        git_cleanup(Some(test_folder.display().to_string()));
        assert!(!Path::new(&test_folder).exists());
        assert!(!git_dir.exists(), "Expected .git directory to be deleted");
        temp_dir.close().unwrap();
    }

    #[test]
    #[serial]
    fn test_init_s4n_with_arc() {
        let temp_dir = tempdir().unwrap();
        check_git_user().unwrap();
        let arc = true;

        let base_folder = Some(temp_dir.path().to_str().unwrap().to_string());

        //call method with temp dir
        let result = handle_init_command(&InitArgs { project: base_folder, arc });

        assert!(result.is_ok(), "Expected successful initialization");

        //check if directories were created
        let expected_dirs = vec!["workflows", "assays", "studies", "runs"];

        for dir in &expected_dirs {
            let full_path = PathBuf::from(temp_dir.path()).join(dir);
            assert!(full_path.exists(), "Directory {dir} does not exist");
        }
    }
}
