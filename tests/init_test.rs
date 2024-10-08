use calamine::{open_workbook, Reader, Xlsx};
use s4n::init::{
    check_git_installation, create_arc_folder_structure, create_investigation_excel_file,
    create_minimal_folder_structure, init_git_repo, init_s4n, is_git_repo,
};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::{Builder, NamedTempFile};

#[test]
fn test_valid_git_repo() {
    // Arrange
    let repo_dir = Builder::new().prefix("valid_git_repo").tempdir().unwrap();

    let repo_dir_str = repo_dir.path().to_str().unwrap();
    let repo_dir_string = String::from(repo_dir_str);

    // Create a simple Git repository
    let init_script = r#"
            mkdir -p {repo_dir}
            cd {repo_dir}
            git init
            echo "Hello World" > file.txt
            git add .
        "#;

    let output = Command::new("bash")
        .arg("-c")
        .arg(init_script.replace("{repo_dir}", &repo_dir_str))
        .status()
        .expect("Failed to execute bash script");

    assert!(output.success(), "Expected success, got {:?}", output);

    // Act
    let result = is_git_repo(Some(&repo_dir_string));

    // Assert
    assert!(result, "Expected true, got false");
}

#[test]
fn test_non_git_repo() {
    // Arrange
    let non_git_dir = Builder::new().prefix("non_git_repo").tempdir().unwrap();

    let non_git_dir_str = non_git_dir.path().to_str().unwrap();
    let non_git_dir_string = String::from(non_git_dir_str);

    // Check if the directory already exists
    if Path::new(&non_git_dir_str).exists() {
        println!(
            "Directory {} already exists, skipping creation",
            non_git_dir_str
        );
    } else {
        std::fs::create_dir(&non_git_dir.path()).expect("Failed to create directory");
    }

    // Act
    let result = is_git_repo(Some(&non_git_dir_string));

    // Assert
    assert!(!result, "Expected false, got true");
}

#[test]
fn test_check_git_installation_success() {
    // Arrange
    let mut mock_output = Vec::new();
    mock_output.extend_from_slice(b"status=0\nmessage=\"OK\"\nrepository=http://github.com/example/repo.git\nHEAD -> master\n\n");

    // Act
    let result = check_git_installation();

    // Assert
    assert!(result.is_ok(), "Expected success, got {:?}", result);
}

#[test]
fn test_empty_directory() {
    // Arrange
    let empty_dir = Builder::new().prefix("empty_repo").tempdir().unwrap();

    let empty_dir_str = empty_dir.path().to_str().unwrap();
    let empty_dir_string = String::from(empty_dir_str);

    // Act
    let result = is_git_repo(Some(&empty_dir_string));

    // Assert
    assert!(!result, "Expected false, got true");
}

#[test]
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
fn test_create_minimal_folder_structure_invalid_file_input() {
    //create an invalid file input
    let temp_file = NamedTempFile::new().unwrap();
    let base_folder = Some(temp_file.path().to_str().unwrap());

    println!("Base folder path: {:?}", base_folder.as_deref());
    //path to file instead of a directory
    let result = create_minimal_folder_structure(base_folder.as_deref());
    assert!(!result.is_ok(), "Expected failed initialization");
}

#[test]
fn test_create_minimal_folder_structure_valid_input() {
    let temp_dir = Builder::new().prefix("minimal_folder").tempdir().unwrap();

    let base_folder = Some(temp_dir.path().to_str().unwrap());

    let result = create_minimal_folder_structure(base_folder.as_deref());

    assert!(result.is_ok(), "Expected successful initialization");

    let expected_dirs = vec![
        PathBuf::from(temp_dir.path()).join("workflows"),
        PathBuf::from(temp_dir.path())
            .join("workflows")
            .join("tools"),
        PathBuf::from(temp_dir.path()).join("workflows").join("wf"),
    ];

    for dir in &expected_dirs {
        assert!(dir.exists(), "Directory {} does not exist", dir.display());
    }
}

#[test]
fn test_create_investigation_excel_file() {
    // Test setup
    let temp_dir = Builder::new()
        .prefix("investigation_excel_test_")
        .tempdir()
        .unwrap();
    let directory = temp_dir.path().to_str().unwrap();

    // Call the function under test
    assert!(
        create_investigation_excel_file(directory).is_ok(),
        "Created Excel file"
    );

    // Verify file exists
    let excel_path = PathBuf::from(directory).join("isa_investigation.xlsx");
    assert!(excel_path.exists(), "Excel file does not exist");

    println!("Excel file path: {:?}", excel_path);

    let workbook: Xlsx<_> = open_workbook(excel_path).expect("Cannot open file");

    let sheets = workbook.sheet_names().to_owned();

    assert_eq!(sheets.len(), 1, "Expected file to have one sheet");
    assert_eq!(
        sheets[0], "isa_investigation",
        "Worksheet name is incorrect"
    );
}

#[test]
fn test_create_arc_folder_structure_with_invalid_base_folder(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp_file = NamedTempFile::new().unwrap();
    let base_path = Some(temp_file.path().to_str().unwrap());

    let result = create_arc_folder_structure(base_path.as_deref());
    assert!(!result.is_ok(), "Expected failed initialization");

    Ok(())
}

#[test]
fn test_init_s4n_creates_folders_and_gitignore() {
    let folder_name = Some("test_folder".to_string());
    let arc = Some(true);

    assert!(init_s4n(folder_name.clone(), arc).is_ok());
}

#[test]
fn test_create_arc_folder_structure_valid_input() {
    let temp_dir = Builder::new().prefix("arc_folder_test").tempdir().unwrap();

    let base_folder = Some(temp_dir.path().to_str().unwrap());

    let result = create_arc_folder_structure(base_folder.as_deref());

    assert!(result.is_ok(), "Expected successful initialization");

    let expected_dirs = vec!["assays", "studies", "workflows", "runs"];

    for dir in &expected_dirs {
        let full_path = PathBuf::from(temp_dir.path()).join(dir);
        assert!(full_path.exists(), "Directory {} does not exist", dir);
    }
}
