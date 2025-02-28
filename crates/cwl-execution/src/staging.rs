use crate::{environment::RuntimeEnvironment, util::{copy_file, create_file}};
use cwl::{clt::CommandLineTool, requirements::Requirement, types::{DefaultValue, Entry, File}};
use glob::glob;
use pathdiff::diff_paths;
use std::{fs, path::{Path, PathBuf}};

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

pub (crate) fn stage_inputs(runtime: &mut RuntimeEnvironment, outdir: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    for (key, input) in runtime.inputs.iter_mut() {
        match input {
            DefaultValue::File(file) => {
                let path = file.path.clone().unwrap(); //we can unwrap here safely, because path has been filled prior!
                let relative_path = diff_paths(&path, &runtime.runtime["tooldir"]).unwrap();
                let destination = outdir.as_ref().join(relative_path);
                copy_file(&path, &destination)?;
                let mut secondary_files = file.secondary_files.clone(); //need to copy!
                let mut new_file = File::from_file(path, file.format.clone());
                for sec_file in secondary_files.iter_mut() {
                    let path = sec_file.path.clone().unwrap();
                }
                new_file.secondary_files = secondary_files;

                *file = new_file;
            },
            DefaultValue::Directory(directory) => todo!(),
            _ => {},
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
