use cwl::clt::Command;
use std::path::Path;

pub fn get_filename_without_extension(relative_path: impl AsRef<Path>) -> String {
    let filename = relative_path.as_ref().file_name().map(|f| f.to_string_lossy()).unwrap_or(relative_path.as_ref().to_string_lossy());
    filename.split('.').next().unwrap_or(&filename).to_string()
}

pub fn get_workflows_folder() -> String {
    "workflows/".to_string()
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

pub fn get_qualified_filename(command: &Command, the_name: Option<String>) -> String {
    //decide over filename
    let mut filename = match &command {
        Command::Multiple(cmd) => get_filename_without_extension(cmd[1].as_str()),
        Command::Single(cmd) => get_filename_without_extension(cmd.as_str()),
    };

    filename = Path::new(&filename).file_name().unwrap_or_default().to_string_lossy().into_owned();

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

pub fn join_path_string<P: AsRef<Path>>(path: P, location: &str) -> String {
    let new_location = path.as_ref().join(location);
    new_location.to_string_lossy().into_owned()
}
