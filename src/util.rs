use colored::Colorize;
use std::process::exit;

pub fn print_error_and_exit(message: &str, code: i32) {
    eprintln!("❌ {}: {}", "Error".red().bold(), message.red());
    exit(code);
}

pub fn warn(message: &str) {
    eprintln!("⚠️  {}", message.yellow());
}

pub fn print_list(list: &Vec<String>) {
    for item in list {
        println!("\t- {}", item)
    }
}
