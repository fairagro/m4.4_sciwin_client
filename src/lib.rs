pub mod cli;
pub mod commands;
pub mod config;
pub mod cwl;
pub mod parser;
pub mod util;

use colored::Colorize;
use log::info;
use similar::{ChangeTag, TextDiff};
use std::fmt;

pub fn print_list(list: &Vec<String>) {
    for item in list {
        info!("\t- {item}");
    }
}

pub fn split_vec_at<T: PartialEq + Clone, C: AsRef<[T]>>(vec: C, split_at: &T) -> (Vec<T>, Vec<T>) {
    let slice = vec.as_ref();
    if let Some(index) = slice.iter().position(|x| x == split_at) {
        let lhs = slice[..index].to_vec();
        let rhs = slice[index + 1..].to_vec();
        (lhs, rhs)
    } else {
        (slice.to_vec(), vec![])
    }
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
            println!("{:-^1$}", "-", 80); //print line to separate groups
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

                print!("{old_line} {new_line} | {styled_line}");
            }
        }
    }
}
