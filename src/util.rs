use colored::Colorize;
use std::{num::NonZero, process::Command, thread};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};
use sysinfo::System;

pub fn error(message: &str) -> String {
    format!("❌ {}: {}", "Error".red().bold(), message.red())
}

pub fn warn(message: &str) {
    eprintln!("⚠️  {}", message.yellow());
}

pub fn print_list(list: &Vec<String>) {
    for item in list {
        println!("\t- {item}");
    }
}

pub fn highlight_cwl(yaml: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension("yaml").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["InspiredGitHub"]);

    for line in LinesWithEndings::from(yaml) {
        let ranges = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("{escaped}")
    }
}

pub fn get_processor_count() -> usize {
    thread::available_parallelism().map(NonZero::get).unwrap_or(1)
}

pub fn get_available_ram() -> u64 {
    let mut system = System::new_all();
    system.refresh_all();
    system.free_memory() / 1024
}

pub fn format_command(command: &Command) -> String {
    let program = command.get_program().to_string_lossy();

    let args: Vec<String> = command
        .get_args()
        .map(|arg| {
            let arg_str = arg.to_string_lossy();
            arg_str.to_string()
        })
        .collect();

    format!("{} {}", program, args.join(" "))
}

pub fn split_vec_at<T: PartialEq + Clone, C: AsRef<[T]>>(vec: C, split_at: T) -> (Vec<T>, Vec<T>) {
    let slice = vec.as_ref();
    if let Some(index) = slice.iter().position(|x| *x == split_at) {
        let lhs = slice[..index].to_vec();
        let rhs = slice[index + 1..].to_vec();
        (lhs, rhs)
    } else {
        (slice.to_vec(), vec![])
    }
}
