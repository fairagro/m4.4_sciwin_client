use dialoguer::Input;
use git2::Config;
use log::warn;
use std::error::Error;

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

        let name: String = Input::new().with_prompt("Enter your name").interact_text()?;
        config.set_str("user.name", name.trim())?;

        let mail: String = Input::new().with_prompt("Enter your email").interact_text()?;
        config.set_str("user.email", mail.trim())?;
    }
    Ok(())
}
