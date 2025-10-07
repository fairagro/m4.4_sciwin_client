pub mod cli;
pub mod commands;
pub mod config;
pub mod cwl;
pub mod logger;
mod reana;
pub mod repo;

use colored::Colorize;
use log::info;
use similar::{ChangeTag, TextDiff};
use std::{fmt, path::Path};

pub fn print_list(list: &Vec<String>) {
    for item in list {
        info!("\t- {item}");
    }
}

use configparser::ini::Ini;

pub(crate) fn remove_ini_section<P: AsRef<Path>>(file: P, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Ini::new();
    config.load(&file)?;
    config.remove_section(name);
    config.write(&file)?;
    Ok(())
}

struct Line(Option<usize>);
impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(n) => write!(f, "{:>4}", n + 1),
            None => write!(f, "    "),
        }
    }
}

pub fn print_diff(old: &str, new: &str) {
    let diff = TextDiff::from_lines(old, new);
    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            eprintln!("{:-^1$}", "-", 80); //print line to separate groups
        }

        for op in group {
            for change in diff.iter_changes(op) {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };

                let (old_line, new_line) = (Line(change.old_index()), Line(change.new_index()));

                let styled_line = match change.tag() {
                    ChangeTag::Equal => format!("{sign} {}", change.value()).dimmed(),
                    ChangeTag::Delete => format!("{sign} {}", change.value()).red(),
                    ChangeTag::Insert => format!("{sign} {}", change.value()).green(),
                };

                eprint!("{old_line} {new_line} | {styled_line}");
            }
        }
    }
}
