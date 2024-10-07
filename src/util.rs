use colored::Colorize;
use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::exit,
};

pub fn get_filename_without_extension(relative_path: &str) -> Option<String> {
    let path = Path::new(relative_path);

    path.file_name().and_then(|name| name.to_str().map(|s| s.split('.').next().unwrap_or(s).to_string()))
}

pub fn get_workflows_folder() -> String {
    "workflows/".to_string()
}

pub fn create_and_write_file(filename: &str, contents: &str) -> Result<(), io::Error> {
    let path = Path::new(filename);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?
    }
    
    let mut file = fs::File::create(filename)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

pub fn print_error_and_exit(message: &str, code: i32) {
    println!("❌ {}: {}", "Error".red().bold(), message.red());
    exit(code);
}

pub fn warn(message: &str) {
    println!("⚠️ {}", message.yellow());
}

pub fn print_files(files: &Vec<String>) {
    for file in files {
        println!("\t- {}", file)
    }
}
