use git2::{Config, Repository};
use s4n::repo::{initial_commit, stage_all};
use core::panic;
use std::{
    env::{self},
    fs::{copy, create_dir_all},
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::{tempdir, TempDir};

#[allow(dead_code)]
pub fn setup_python(dir_str: &str) -> (String, String) {
    //windows stuff
    let ext = if cfg!(target_os = "windows") { ".exe" } else { "" };
    let path_sep = if cfg!(target_os = "windows") { ";" } else { ":" };
    let venv_scripts = if cfg!(target_os = "windows") { "Scripts" } else { "bin" };

    //set up python venv
    let _ = Command::new("python").arg("-m").arg("venv").arg(".venv").output();
    let old_path = env::var("PATH").unwrap();
    let python_path = format!("{dir_str}/.venv/{venv_scripts}");
    let new_path = format!("{python_path}{path_sep}{old_path}");

    //install packages
    let req_path = format!("{dir_str}/requirements.txt");
    let _ = Command::new(python_path + "/pip" + ext).arg("install").arg("-r").arg(req_path).output();

    (new_path, old_path)
}

#[allow(dead_code)]
pub fn check_git_user() -> Result<(), git2::Error> {
    let mut config = Config::open_default()?;
    if config.get_string("user.name").is_err() {
        config.remove_multivar("user.name", ".*").ok();
        config.set_str("user.name", &whoami::username()).expect("Could not set name");
    }

    if config.get_string("user.email").is_err() {
        config
            .set_str("user.email", &format!("{}@example.com", whoami::username()))
            .expect("Could not set email");
    }

    Ok(())
}

/// Sets up a temporary repository with test data
#[allow(dead_code)]
fn set_up_repository() -> TempDir {
    let dir = tempdir().expect("Failed to create a temporary directory");
    create_dir_all(dir.path().join(Path::new("scripts"))).expect("Failed to create scripts-dir");
    create_dir_all(dir.path().join(Path::new("data"))).expect("Failed to create data-dir");

    let source_files: [(PathBuf, &str); 8] = [
        (Path::new("./tests/test_data/echo.py").to_path_buf(), "scripts/echo.py"),
        (Path::new("./tests/test_data/echo2.py").to_path_buf(), "scripts/echo2.py"),
        (Path::new("./tests/test_data/echo3.py").to_path_buf(), "scripts/echo3.py"),
        (Path::new("./tests/test_data/script_test.py").to_path_buf(), "scripts/script_test.py"),
        (Path::new("./tests/test_data/echo_inline.py").to_path_buf(), "scripts/echo_inline.py"),
        (Path::new("./tests/test_data/input.txt").to_path_buf(), "data/input.txt"),
        (Path::new("./tests/test_data/input2.txt").to_path_buf(), "data/input2.txt"),
        (Path::new("./tests/test_data/Dockerfile").to_path_buf(), "Dockerfile"),
    ];

    for (src, target) in &source_files {
        let target_path = dir.path().join(target);
        match copy(src, &target_path) {
            Ok(_) => {
                println!("Copied {src:?} to {target_path:?}");
            }
            Err(e) => {
                eprintln!("Failed to copy file from {src:?} to {target_path:?}: {e}");
                panic!("Error occurred while copying files.");
            }
        }
    }
    check_git_user().unwrap();
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

/// Sets up a repository with the files in `tests/test_data` in tmp folder.
/// You *must* specify `#[serial]` for those tests
#[allow(dead_code)]
pub fn with_temp_repository<F>(test: F)
where F: FnOnce(&TempDir) + panic::UnwindSafe,
{
    let dir = set_up_repository();
    let current_dir = env::current_dir().expect("Could not get current working directory");
    env::set_current_dir(dir.path()).expect("Could not set current dir");

    test(&dir);

    env::set_current_dir(current_dir).expect("Could not reset current dir");
    dir.close().unwrap();
}

#[allow(dead_code)]
pub fn os_path(path: &str) -> String {
    if cfg!(target_os = "windows") {
        Path::new(path).to_string_lossy().replace('/', "\\")
    } else {
        path.to_string()
    }
}
