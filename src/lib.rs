pub mod cli;
pub mod commands;
pub mod config;
pub mod cwl;
pub mod io;
pub mod log;
pub mod parser;
pub mod repo;

use ::log::info;

pub fn print_list(list: &Vec<String>) {
    for item in list {
        info!("\t- {item}");
    }
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
