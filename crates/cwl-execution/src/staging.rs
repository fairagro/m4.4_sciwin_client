use cwl::{clt::CommandLineTool, requirements::Requirement, types::Entry};
use std::{fs, path::Path};

use crate::util::{copy_file, create_file};

pub(crate) fn stage_required_files(tool: &CommandLineTool, outdir: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    for requirement in tool.requirements.iter().chain(tool.hints.iter()).flatten() {
        if let Requirement::InitialWorkDirRequirement(iwdr) = requirement {
            for listing in &iwdr.listing {
                let filename = outdir.as_ref().join(&listing.entryname);
                match &listing.entry {
                    Entry::Source(src) => {
                        if fs::exists(src).unwrap_or(false) {
                            copy_file(src, filename)?;
                        } else {
                            create_file(filename, src)?;
                        }
                    }
                    Entry::Include(include) => {
                        copy_file(&include.include, filename)?;
                    },
                }
            }
        }
    }
    Ok(())
}
