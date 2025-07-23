use colored::{ColoredString, Colorize};
use git2::Config;
use log::warn;
use std::{
    error::Error,
    io::{self, stdin, stdout, Write},
};

mod annotate;
mod execute;
mod init;
mod tool;
mod workflow;

pub use annotate::*;
pub use execute::*;
pub use init::*;
pub use tool::*;
pub use workflow::*;

pub fn check_git_config() -> Result<(), Box<dyn Error>> {
    let mut config = Config::open_default()?;
    if config.get_string("user.name").is_err() || config.get_string("user.email").is_err() {
        warn!("User configuration not found!");

        let name = prompt(&"Enter your name: ".bold().green())?;
        config.set_str("user.name", name.trim())?;

        let mail = prompt(&"Enter your email: ".bold().green())?;
        config.set_str("user.email", mail.trim())?;
    }
    Ok(())
}
fn prompt(message: &ColoredString) -> io::Result<String> {
    print!("{message}");
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    Ok(input)
}
