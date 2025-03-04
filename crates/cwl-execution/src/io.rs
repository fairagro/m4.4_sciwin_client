use rand::{distributions::Alphanumeric, Rng};
use std::{
    cell::RefCell,
    fs::{self},
    io::{self, Error, Write},
    path::{Path, MAIN_SEPARATOR_STR},
    process::Command,
};

pub fn get_filename_without_extension<S: AsRef<str>>(relative_path: S) -> Option<String> {
    let path = Path::new(relative_path.as_ref());

    path.file_name()
        .and_then(|name| name.to_str().map(|s| s.split('.').next().unwrap_or(s).to_string()))
}

fn get_basename<S: AsRef<str>>(filename: S) -> String {
    let path = Path::new(filename.as_ref());

    path.file_name().unwrap_or_default().to_string_lossy().into_owned()
}

fn get_extension<S: AsRef<str>>(filename: S) -> String {
    let path = Path::new(filename.as_ref());

    path.extension().unwrap_or_default().to_string_lossy().into_owned()
}

pub fn create_and_write_file<P: AsRef<Path>>(filename: P, contents: &str) -> Result<(), Error> {
    create_and_write_file_internal(filename, contents, false)
}
pub fn create_and_write_file_forced<P: AsRef<Path>>(filename: P, contents: &str) -> Result<(), Error> {
    create_and_write_file_internal(filename, contents, true)
}

fn create_and_write_file_internal<P: AsRef<Path>>(filename: P, contents: &str, overwrite: bool) -> Result<(), Error> {
    let path = filename.as_ref();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = if overwrite {
        fs::File::create(filename)
    } else {
        fs::File::create_new(filename)
    }?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

pub fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<(), Error> {
    let path = to.as_ref();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(from, to)?;
    Ok(())
}

pub fn copy_dir<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dest: Q) -> Result<Vec<String>, Error> {
    let mut files = vec![];
    fs::create_dir_all(&dest)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = Path::new(dest.as_ref()).join(entry.file_name());
        if src_path.is_dir() {
            files.extend(copy_dir(src_path.to_str().unwrap(), dest_path.to_str().unwrap())?);
        } else {
            copy_file(src_path.to_str().unwrap(), dest_path.to_str().unwrap())?;
            files.push(dest_path.to_string_lossy().into_owned());
        }
    }
    Ok(files)
}

pub fn get_file_size<P: AsRef<Path>>(path: P) -> io::Result<u64> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

pub fn get_shell_command() -> Command {
    let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let param = if cfg!(target_os = "windows") { "/C" } else { "-c" };
    let mut cmd = Command::new(shell);
    cmd.arg(param);
    cmd
}

pub fn get_file_property(filename: &str, property_name: &str) -> String {
    match property_name {
        "size" => get_file_size(filename).unwrap_or(1).to_string(),
        "basename" => get_basename(filename),
        "nameroot" => get_filename_without_extension(filename).unwrap(),
        "nameext" => get_extension(filename),
        "path" => filename.to_string(),
        "dirname" => {
            let path = Path::new(filename);
            let parent = path.parent().unwrap_or(path).to_string_lossy().into_owned();
            if parent.is_empty() {
                return ".".to_string();
            }
            parent
        }
        _ => filename.to_string(),
    }
}

pub fn get_random_filename(prefix: &str, extension: &str) -> String {
    let rnd: String = rand::thread_rng().sample_iter(&Alphanumeric).take(10).map(char::from).collect();
    format!("{prefix}_{rnd}.{extension}")
}

pub fn get_first_file_with_prefix<P: AsRef<Path>>(location: P, prefix: &str) -> Option<String> {
    let path = location.as_ref();

    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            if filename_str.starts_with(prefix) {
                return Some(filename_str.into_owned());
            }
        }
    }

    None
}

pub fn make_relative_to<'a>(path: &'a str, dir: &str) -> &'a str {
    let prefix = if !dir.ends_with(MAIN_SEPARATOR_STR) {
        &format!("{dir}{MAIN_SEPARATOR_STR}")
    } else {
        dir
    };
    path.strip_prefix(prefix).unwrap_or(path)
}

thread_local!(static PRINT_OUTPUT: RefCell<bool> = const { RefCell::new(true) });

pub fn set_print_output(value: bool) {
    PRINT_OUTPUT.with(|print_output| {
        *print_output.borrow_mut() = value;
    });
}

pub fn print_output() -> bool {
    PRINT_OUTPUT.with(|print_output| *print_output.borrow())
}
