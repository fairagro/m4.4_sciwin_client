use git2::Repository;
use s4n::repo::{initial_commit, stage_all};
use std::{
    env::{self},
    fs::{copy, create_dir_all},
    path::{Path, PathBuf},
};
use tempfile::{tempdir, TempDir};

/// converts \\ to /
pub fn normalize_path(path: &str) -> String {
    Path::new(path).to_string_lossy().replace("\\", "/")
}

/// Sets up a temporary repository with test data
fn set_up_repository() -> TempDir {
    let dir = tempdir().expect("Failed to create a temporary directory");
    create_dir_all(dir.path().join(Path::new("scripts"))).expect("Failed to create scripts-dir");
    create_dir_all(dir.path().join(Path::new("data"))).expect("Failed to create data-dir");

    let source_files: [(PathBuf, &str); 3] = [
        (PathBuf::from("./tests/test_data/echo.py"), "scripts/echo.py"),
        (PathBuf::from("./tests/test_data/input.txt"), "data/input.txt"),
        (PathBuf::from("./tests/test_data/Dockerfile"), "Dockerfile"),
    ];

    for (src, target) in source_files.iter() {
        let target_path = dir.path().join(target);
        match copy(src, &target_path) {
            Ok(_) => {
                println!("Copied {:?} to {:?}", src, target_path);
            }
            Err(e) => {
                eprintln!("Failed to copy file from {:?} to {:?}: {}", src, target_path, e);
                panic!("Error occurred while copying files.");
            }
        }
    }
    let repo = Repository::init(&dir).expect("Failed to create a blank repository");
    stage_all(&repo).expect("Could not stage files");

    if repo.signature().is_err() {
        let mut cfg = repo.config().expect("Could not get config");
        cfg.set_str("user.name", "Derp").expect("Could not set name");
        cfg.set_str("user.email", "derp@google.de").expect("Could not set email");
    }
    initial_commit(&repo).expect("Could not create inital commit");

    dir
}

/// Sets up a repository with the files in tests/test_data in tmp folder.
/// You *must* specify `#[serial]` for those tests
pub fn with_temp_repository<F>(test: F)
where
    F: FnOnce(&TempDir),
{
    let dir = set_up_repository();
    let current_dir = env::current_dir().expect("Could not get current working directory");
    env::set_current_dir(dir.path()).expect("Could not set current dir");

    test(&dir);

    env::set_current_dir(current_dir).expect("Could not reset current dir");
    dir.close().unwrap()
}
