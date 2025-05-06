use rand::{distr::Alphanumeric, Rng};
use std::{
    cell::RefCell,
    ffi::OsStr,
    fs::{self},
    io::{self, Write},
    path::{Path, MAIN_SEPARATOR_STR},
    process::Command,
};

pub fn create_and_write_file<P: AsRef<Path>>(filename: P, contents: &str) -> io::Result<()> {
    create_and_write_file_internal(filename, contents, false)
}
pub fn create_and_write_file_forced<P: AsRef<Path>>(filename: P, contents: &str) -> io::Result<()> {
    create_and_write_file_internal(filename, contents, true)
}

fn create_and_write_file_internal<P: AsRef<Path>>(filename: P, contents: &str, overwrite: bool) -> io::Result<()> {
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

pub fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    let path = to.as_ref();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(from, to)?;
    Ok(())
}

pub fn copy_dir<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dest: Q) -> io::Result<Vec<String>> {
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

pub(crate) fn get_file_size<P: AsRef<Path>>(path: P) -> io::Result<u64> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

pub(crate) fn get_shell_command() -> Command {
    let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let param = if cfg!(target_os = "windows") { "/C" } else { "-c" };
    let mut cmd = Command::new(shell);
    cmd.arg(param);
    cmd
}

pub fn get_file_property(filename: impl AsRef<Path>, property_name: &str) -> String {
    match property_name {
        "size" => get_file_size(filename).unwrap_or(1).to_string(),
        "basename" => make_string(filename.as_ref().file_name()),
        "nameroot" => make_string(filename.as_ref().file_stem()),
        "nameext" => format!(".{}", make_string(filename.as_ref().extension())), //needs leading dot
        "path" => filename.as_ref().to_string_lossy().into_owned(),
        "dirname" => {
            let path = filename.as_ref();
            let parent = path.parent().unwrap_or(path).to_string_lossy().into_owned();
            if parent.is_empty() {
                return ".".to_string();
            }
            parent
        }
        _ => filename.as_ref().to_string_lossy().into_owned(),
    }
}

fn make_string(input: Option<&OsStr>) -> String {
    input.unwrap_or_default().to_string_lossy().to_string()
}

pub fn get_random_filename(prefix: &str, extension: &str) -> String {
    let rnd: String = rand::rng().sample_iter(&Alphanumeric).take(10).map(char::from).collect();
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

pub fn join_path_string<P: AsRef<Path>>(path: P, location: &str) -> String {
    let new_location = path.as_ref().join(location);
    new_location.to_string_lossy().into_owned()
}
