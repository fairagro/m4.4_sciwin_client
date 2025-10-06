use std::path::Path;

mod log;
pub mod repo;

use configparser::ini::Ini;
pub use log::*;

pub(crate) fn remove_ini_section<P: AsRef<Path>>(file: P, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Ini::new();
    config.load(&file)?;
    config.remove_section(name);
    config.write(&file)?;
    Ok(())
}
