#![allow(clippy::disallowed_macros)]
use calamine::{open_workbook, Reader, Xlsx};
use s4n::commands::*;
use serial_test::serial;
use std::{
    env,
    path::{Path, PathBuf},
};
use tempfile::{tempdir, Builder, NamedTempFile};
use test_utils::check_git_user;

#[test]
#[serial]
fn test_init_s4n_without_folder() {
    //create a temp dir
    let temp_dir = tempdir().expect("Failed to create a temporary directory");
    println!("Temporary directory: {temp_dir:?}");
    check_git_user().unwrap();
    // Create a subdirectory in the temporary directory
    std::fs::create_dir_all(&temp_dir).expect("Failed to create test directory");

    // Change to the temporary directory
    env::set_current_dir(&temp_dir).unwrap();
    println!("Current directory changed to: {}", env::current_dir().unwrap().display());

    // test method without folder name and do not create arc folders
    let folder_name: Option<String> = None;
    let arc = false;
    let result = initialize_project(&folder_name, arc);

    // Assert results is ok and folders exist/ do not exist
    assert!(result.is_ok());

    let expected_dirs = vec!["workflows"];
    //check that other directories are not created
    let unexpected_dirs = vec!["assays", "studies", "runs"];

    //assert minimal folders do exist
    for dir in &expected_dirs {
        let full_path = PathBuf::from(&temp_dir.path()).join(dir);
        assert!(full_path.exists(), "Directory {dir} does not exist");
    }
    //assert other arc folders do not exist
    for dir in &unexpected_dirs {
        let full_path = PathBuf::from(&temp_dir.path()).join(dir);
        assert!(!full_path.exists(), "Directory {dir} does exist, but should not exist");
    }
}

#[test]
#[serial]
fn test_cleanup_no_folder() {
    let temp_dir = tempdir().expect("Failed to create a temporary directory");
    println!("Temporary directory: {temp_dir:?}");
    check_git_user().unwrap();
    // Create a subdirectory in the temporary directory
    std::fs::create_dir_all(&temp_dir).expect("Failed to create test directory");

    // Change to the temporary directory
    env::set_current_dir(&temp_dir).unwrap();
    println!("Current directory changed to: {}", env::current_dir().unwrap().display());

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
    println!("Temporary directory: {:?}", temp_dir.path());
    check_git_user().unwrap();

    // Change current dir to the temporary directory to not create workflow folders etc in sciwin-client dir
    env::set_current_dir(temp_dir.path()).unwrap();
    println!("Current directory changed to: {}", env::current_dir().unwrap().display());

    // test method without folder name and do not create arc folders
    let folder_name: Option<String> = None;
    let arc = true;

    let result = initialize_project(&folder_name, arc);

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
    let temp_dir = tempfile::tempdir().unwrap();
    let test_folder = temp_dir.path().join("my_repo");
    let result = init_git_repo(Some(test_folder.to_str().unwrap()));
    assert!(result.is_ok(), "Expected successful initialization");
    assert!(Path::new(&test_folder).exists());
    let git_dir = test_folder.join(".git");
    assert!(git_dir.exists(), "Expected .git directory to be created");
    git_cleanup(Some(test_folder.display().to_string()));
    assert!(!Path::new(&test_folder).exists());
    assert!(!git_dir.exists(), "Expected .git directory to be deleted");
}

#[test]
#[serial]
fn test_create_minimal_folder_structure_invalid() {
    //create an invalid file input
    let temp_file = NamedTempFile::new().unwrap();
    let base_folder = Some(temp_file.path().to_str().unwrap());

    println!("Base folder path: {base_folder:?}");
    //path to file instead of a directory, assert that it fails
    let result = create_minimal_folder_structure(base_folder, false);
    assert!(result.is_err(), "Expected failed initialization");
}

#[test]
#[serial]
fn test_create_minimal_folder_structure() {
    let temp_dir = Builder::new().prefix("minimal_folder").tempdir().unwrap();

    let base_folder = Some(temp_dir.path().to_str().unwrap());

    let result = create_minimal_folder_structure(base_folder, false);

    //test if result is ok
    assert!(result.is_ok(), "Expected successful initialization");

    let expected_dirs = vec!["workflows"];
    //assert that folders exist
    for dir in &expected_dirs {
        let full_path = PathBuf::from(temp_dir.path()).join(dir);
        assert!(full_path.exists(), "Directory {dir} does not exist");
    }
}

#[test]
#[serial]
fn test_create_investigation_excel_file() {
    //create directory
    let temp_dir = Builder::new().prefix("investigation_excel_test_").tempdir().unwrap();
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
fn test_create_arc_folder_structure() {
    let temp_dir = Builder::new().prefix("arc_folder_test").tempdir().unwrap();

    let base_folder = Some(temp_dir.path().to_str().unwrap());

    let result = create_arc_folder_structure(base_folder);

    assert!(result.is_ok(), "Expected successful initialization");

    let expected_dirs = vec!["assays", "studies", "workflows", "runs"];
    //assert that folders are created
    for dir in &expected_dirs {
        let full_path = PathBuf::from(temp_dir.path()).join(dir);
        assert!(full_path.exists(), "Directory {dir} does not exist");
    }
}

#[test]
#[serial]
fn test_init_s4n_with_arc() {
    let temp_dir = Builder::new().prefix("init_with_arc_test").tempdir().unwrap();
    check_git_user().unwrap();
    let arc = true;

    let base_folder = Some(temp_dir.path().to_str().unwrap().to_string());

    //call method with temp dir
    let result = initialize_project(&base_folder, arc);

    assert!(result.is_ok(), "Expected successful initialization");

    //check if directories were created
    let expected_dirs = vec!["workflows", "assays", "studies", "runs"];

    for dir in &expected_dirs {
        let full_path = PathBuf::from(temp_dir.path()).join(dir);
        assert!(full_path.exists(), "Directory {dir} does not exist");
    }
}
#[test]
#[serial]
fn test_init_s4n_minimal() {
    let temp_dir = Builder::new().prefix("init_without_arc_test").tempdir().unwrap();
    check_git_user().unwrap();
    let arc = false;

    let base_folder = Some(temp_dir.path().to_str().unwrap().to_string());

    //call method with temp dir
    let result = initialize_project(&base_folder, arc);
    println!("{result:#?}");
    assert!(result.is_ok(), "Expected successful initialization");

    //check if directories were created
    let expected_dirs = vec!["workflows"];
    //check that other directories are not created
    let unexpected_dirs = vec!["assays", "studies", "runs"];

    //assert minimal folders do exist
    for dir in &expected_dirs {
        let full_path = PathBuf::from(temp_dir.path()).join(dir);
        assert!(full_path.exists(), "Directory {dir} does not exist");
    }
    //assert other arc folders do not exist
    for dir in &unexpected_dirs {
        let full_path = PathBuf::from(temp_dir.path()).join(dir);
        assert!(!full_path.exists(), "Directory {dir} does exist, but should not exist");
    }
}
