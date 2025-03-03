use crate::{
    environment::RuntimeEnvironment,
    util::{copy_dir, copy_file, create_file},
};
use cwl::{
    clt::CommandLineTool,
    requirements::Requirement,
    types::{DefaultValue, Directory, Entry, File},
};
use glob::glob;
use std::{
    fs,
    path::{Path, PathBuf},
};

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

pub(crate) fn stage_input_files(runtime: &mut RuntimeEnvironment, outdir: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut updates = vec![];

    for (key, value) in &runtime.inputs {
        match value {
            DefaultValue::File(file) => {
                let path = if let Some(location) = file.location.as_ref() {
                    Path::new(location)
                } else {
                    if let Some(contents) = &file.contents {
                        let path = Path::new(".literal");
                        fs::write(path, contents)?;
                        Ok::<&Path, Box<dyn std::error::Error>>(path)
                    } else {
                        return Err(Box::<dyn std::error::Error>::from(format!("Could not find file {:?}", file.location)));
                    }?
                };
                let relative = if let Some(basename) = file.basename.as_ref() {
                    let dirname = path.parent().unwrap_or(Path::new("."));
                    dirname.join(basename)
                } else {
                    PathBuf::from(path)
                };
                let destination = &outdir.as_ref().join(relative);
                copy_file(path, destination)?;
                let mut new_file = File::from_file(destination, file.format.clone());
                if let Some(secondary_files) = &file.secondary_files {
                    new_file.secondary_files = Some(stage_secondary_files(secondary_files, outdir.as_ref(), &new_file)?);
                }
                updates.push((key.to_string(), DefaultValue::File(new_file)));
            }
            DefaultValue::Directory(dir) => {
                let path = dir.location.as_ref().unwrap();
                let relative = if let Some(basename) = dir.basename.as_ref() {
                    let dirname = Path::new(path).parent().unwrap_or(Path::new("."));
                    dirname.join(basename)
                } else {
                    PathBuf::from(path)
                };
                let destination = outdir.as_ref().join(relative);
                copy_dir(path, &destination)?;
                let new_dir = Directory::from_path(destination);
                updates.push((key.to_string(), DefaultValue::Directory(new_dir)));
            }
            _ => {}
        }
    }

    for (key, value) in updates {
        runtime.inputs.insert(key, value);
    }
    Ok(())
}

fn stage_secondary_files(
    secondary_files: &[DefaultValue],
    outdir: impl AsRef<Path>,
    parent: &File,
) -> Result<Vec<DefaultValue>, Box<dyn std::error::Error>> {
    secondary_files
        .iter()
        .map(|file| match file {
            DefaultValue::File(file) => {
                let path = file.location.as_ref().unwrap();
                let relative = if let Some(basename) = file.basename.as_ref() {
                    if let Some(folder) = &parent.dirname {
                        PathBuf::from(folder).join(basename)
                    } else {
                        PathBuf::from(basename)
                    }
                } else {
                    PathBuf::from(path)
                };
                let destination = outdir.as_ref().join(relative);
                copy_file(path, &destination)?;
                let new_file = File::from_file(destination, file.format.clone());
                Ok(DefaultValue::File(new_file))
            }
            DefaultValue::Directory(dir) => {
                let path = dir.location.as_ref().unwrap();
                let relative = if let Some(basename) = dir.basename.as_ref() {
                    if let Some(folder) = &parent.dirname {
                        PathBuf::from(folder).join(basename)
                    } else {
                        PathBuf::from(basename)
                    }
                } else {
                    PathBuf::from(path)
                };
                let destination = outdir.as_ref().join(relative);
                copy_dir(path, &destination)?;
                let new_dir = Directory::from_path(destination);
                Ok(DefaultValue::Directory(new_dir))
            }
            DefaultValue::Any(a) => Ok(DefaultValue::Any(a.clone())),
        })
        .collect::<Result<Vec<_>, _>>()
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
            if let Some(glob_) = &binding.glob {
                let pattern = outdir.as_ref().join(glob_);
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
    }
    for file in files {
        fs::remove_file(file)?;
    }
    Ok(())
}
