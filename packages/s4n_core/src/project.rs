use crate::{
    config::Config,
    repo::{commit, get_modified_files, initial_commit, stage_all},
};
use git2::Repository;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use std::{fs::File, io::Write};

pub fn initialize_project(folder_name: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let folder = folder_name.as_deref();
    let repo = if is_git_repo(folder) {
        Repository::open(folder.unwrap_or("."))?
    } else {
        init_git_repo(folder)?
    };

    create_minimal_folder_structure(folder)?;

    write_config(folder)?;

    let files = get_modified_files(&repo);
    if !files.is_empty() {
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

const GITIGNORE_CONTENT: &str = include_str!("../resources/default.gitignore");

pub fn init_git_repo(base_folder: Option<&str>) -> Result<Repository, Box<dyn std::error::Error>> {
    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => env::current_dir()?,
    };
    let base_dir = if base_dir.is_symlink() { fs::read_link(&base_dir)? } else { base_dir };

    if !base_dir.exists() {
        fs::create_dir_all(&base_dir)?;
    }
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

pub fn create_minimal_folder_structure(base_folder: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = match base_folder {
        Some(folder) => PathBuf::from(folder),
        None => env::current_dir()?,
    };
    let base_dir = if base_dir.is_symlink() { fs::read_link(&base_dir)? } else { base_dir };

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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::{Builder, NamedTempFile, tempdir};
    use test_utils::check_git_user;

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

    #[test]
    #[serial]
    fn test_create_minimal_folder_structure() {
        let temp_dir = Builder::new().prefix("minimal_folder").tempdir().unwrap();

        let base_folder = Some(temp_dir.path().to_str().unwrap());

        let result = create_minimal_folder_structure(base_folder);

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
    fn test_create_minimal_folder_structure_invalid() {
        //create an invalid file input
        let temp_file = NamedTempFile::new().unwrap();
        let base_folder = Some(temp_file.path().to_str().unwrap());

        eprintln!("Base folder path: {base_folder:?}");
        //path to file instead of a directory, assert that it fails
        let result = create_minimal_folder_structure(base_folder);
        assert!(result.is_err(), "Expected failed initialization");
    }

    #[test]
    #[serial]
    fn test_init_s4n_without_folder() {
        //create a temp dir
        let temp_dir = tempdir().expect("Failed to create a temporary directory");
        eprintln!("Temporary directory: {temp_dir:?}");
        check_git_user().unwrap();
        // Create a subdirectory in the temporary directory
        std::fs::create_dir_all(&temp_dir).expect("Failed to create test directory");

        // Change to the temporary directory
        env::set_current_dir(&temp_dir).unwrap();
        eprintln!("Current directory changed to: {}", env::current_dir().unwrap().display());

        // test method without folder name and do not create arc folders
        let folder_name: Option<String> = None;
        let result = initialize_project(&folder_name);

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
    fn test_init_s4n_minimal() {
        let temp_dir = Builder::new().prefix("init_without_arc_test").tempdir().unwrap();
        check_git_user().unwrap();

        let base_folder = Some(temp_dir.path().to_str().unwrap().to_string());

        //call method with temp dir
        let result = initialize_project(&base_folder);
        eprintln!("{result:#?}");
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
}
