use std::path::Path;
use crate::clt::Command;
use s4n_core::io::{get_filename_without_extension, get_workflows_folder};

pub fn get_qualified_filename(command: &Command, the_name: Option<String>) -> String {
    //decide over filename
    let mut filename = match &command {
        Command::Multiple(cmd) => get_filename_without_extension(cmd[1].as_str()).unwrap_or_else(|| cmd[1].clone()),
        Command::Single(cmd) => cmd.to_string(),
    };

    if let Some(name) = the_name {
        filename.clone_from(&name);
        if filename.ends_with(".cwl") {
            filename = filename.replace(".cwl", "");
        }
    }

    let foldername = filename.clone();
    filename.push_str(".cwl");

    get_workflows_folder() + &foldername + "/" + &filename
}

pub fn resolve_path<P: AsRef<Path>, Q: AsRef<Path>>(filename: P, relative_to: Q) -> String {
    let path = filename.as_ref();
    let relative_path = Path::new(relative_to.as_ref());
    let base_dir = match relative_path.extension() {
        Some(_) => relative_path.parent().unwrap_or_else(|| Path::new(".")),
        None => relative_path,
    };

    pathdiff::diff_paths(path, base_dir)
        .expect("path diffs not valid")
        .to_string_lossy()
        .into_owned()
}
