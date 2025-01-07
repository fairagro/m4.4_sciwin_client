use colored::Colorize;

pub fn error(message: &str) -> String {
    format!("❌ {}: {}", "Error".red().bold(), message.red())
}