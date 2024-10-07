use colored::Colorize;
use std::process::exit;

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
