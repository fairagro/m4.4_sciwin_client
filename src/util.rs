use colored::Colorize;

pub fn error(message: &str) {
    panic!("❌ {}: {}", "Error".red().bold(), message.red())
}

pub fn warn(message: &str) {
    eprintln!("⚠️  {}", message.yellow());
}

pub fn print_list(list: &Vec<String>) {
    for item in list {
        println!("\t- {}", item)
    }
}
