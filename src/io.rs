use crate::cwl::clt::Command;
use std::{
    fs,
    io::{Error, Write},
    path::Path,
};

pub fn get_filename_without_extension(relative_path: &str) -> Option<String> {
    let path = Path::new(relative_path);

    path.file_name().and_then(|name| name.to_str().map(|s| s.split('.').next().unwrap_or(s).to_string()))
}

pub fn get_workflows_folder() -> String {
    "workflows/".to_string()
}

pub fn create_and_write_file(filename: &str, contents: &str) -> Result<(), Error> {
    let path = Path::new(filename);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(filename)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

pub fn copy_file(from: &str, to: &str) -> Result<(), Error> {
    let path = Path::new(to);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?
    }

    fs::copy(from, to)?;
    Ok(())
}

pub fn resolve_path(filename: &str, relative_to: &str) -> String {
    let path = Path::new(filename);
    let relative_path = Path::new(relative_to);
    let base_dir = match relative_path.extension() {
        Some(_) => relative_path.parent().unwrap_or_else(|| Path::new(".")),
        None => relative_path,
    };

    pathdiff::diff_paths(path, base_dir).expect("to be valid").to_string_lossy().into_owned()
}

pub fn get_qualified_filename(command: &Command, the_name: Option<String>) -> String {
    //decide over filename
    let mut filename = match &command {
        Command::Multiple(cmd) => get_filename_without_extension(cmd[1].as_str()).unwrap_or(cmd[1].clone()),
        Command::Single(cmd) => cmd.to_string(),
    };

    if let Some(name) = the_name {
        filename = name.clone();
        if filename.ends_with(".cwl") {
            filename = filename.replace(".cwl", "");
        }
    }

    let foldername = filename.clone();
    filename.push_str(".cwl");

    get_workflows_folder() + &foldername + "/" + &filename
}
