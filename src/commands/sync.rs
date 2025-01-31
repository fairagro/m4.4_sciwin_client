use log::error;
use std::error::Error;

pub fn handle_sync() -> Result<(), Box<dyn Error>> {
    error!("Sync command not implemented yet");
    Ok(())
}
