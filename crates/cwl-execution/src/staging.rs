use crate::util::{copy_file, create_file};
use cwl::{clt::CommandLineTool, requirements::Requirement, types::Entry};
use glob::glob;
use std::{fs, path::Path};

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
                    }
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn unstage_required_files(tool: &CommandLineTool, outdir: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut files = vec![];
    for requirement in tool.requirements.iter().chain(tool.hints.iter()).flatten() {
        if let Requirement::InitialWorkDirRequirement(iwdr) = requirement {
            for listing in &iwdr.listing {
                let filename = outdir.as_ref().join(&listing.entryname);
                files.push(filename);
            }
        }
    }

    for output in &tool.outputs {
        if let Some(binding) = &output.output_binding {
            let pattern = outdir.as_ref().join(&binding.glob);
            let pattern = pattern.to_string_lossy();
            for entry in glob(&pattern)? {
                let entry = entry?;
                if files.contains(&entry) {
                    //all items not being entry remaining
                    files.retain(|f| *f != entry);
                }
            }
        }
    }
    for file in files {
        fs::remove_file(file)?;
    }
    Ok(())
}
