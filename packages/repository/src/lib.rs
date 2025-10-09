use git2::{Commit, Error, IndexAddOption, Status, StatusOptions};
use std::{iter, path::Path};
mod commit;
mod ini;
pub mod submodule;

pub use commit::*;
// Re-export git2::Repository for external use
pub use git2::Repository;
