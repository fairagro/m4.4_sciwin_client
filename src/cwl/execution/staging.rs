use clap::builder::Str;

use super::util::evaluate_input;
use crate::{
    cwl::{
        clt::{CommandInputParameter, CommandLineTool, CommandOutputParameter, DefaultValue, Entry, Requirement},
        types::CWLType,
    },
    io::{copy_dir, copy_file, create_and_write_file},
};
use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    vec,
};

pub fn stage_required_files(tool: &CommandLineTool, input_values: &Option<HashMap<String, DefaultValue>>, path: PathBuf) -> Result<Vec<String>, Box<dyn Error>> {
    let mut staged_files: Vec<String> = vec![];
    //stage requirements
    staged_files.extend(stage_requirements(&tool.requirements, &path)?);

    //stage inputs
    staged_files.extend(stage_input_files(&tool.inputs, input_values, &path)?);

    Ok(staged_files)
}

pub fn unstage_files(staged_files: &[String], tmp_dir: &Path, outputs: &[CommandOutputParameter]) -> Result<(), Box<dyn Error>> {
    for file in staged_files {
        let mut should_remove = true;

        for output in outputs {
            if let Some(binding) = &output.output_binding {
                let binding_path = tmp_dir.join(&binding.glob);
                if binding_path.to_str().unwrap().matches(file).next().is_some() {
                    should_remove = false;
                    break;
                }
            }
        }

        if should_remove {
            let path = Path::new(file);
            if path.is_dir() {
                fs::remove_dir_all(file).map_err(|e| format!("Could not remove staged dir {}: {}", file, e))?;
            } else {
                fs::remove_file(file).map_err(|e| format!("Could not remove staged file {}: {}", file, e))?;
            }
        }
    }
    Ok(())
}

fn stage_requirements(requirements: &Option<Vec<Requirement>>, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut staged_files = vec![];

    if let Some(requirements) = &requirements {
        for requirement in requirements {
            if let Requirement::InitialWorkDirRequirement(iwdr) = requirement {
                for listing in &iwdr.listing {
                    let into_path = path.join(&listing.entryname); //stage as listing's entry name
                    let path_str = &into_path.to_string_lossy();
                    match &listing.entry {
                        Entry::Source(src) => {
                            create_and_write_file(path_str, src).map_err(|e| format!("Failed to create file {:?}: {}", into_path, e))?;
                        }
                        Entry::Include(include) => {
                            copy_file(&include.include, path_str).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", include.include, into_path, e))?;
                        }
                    }
                    staged_files.push(path_str.clone().into_owned());
                }
            }
        }
    }

    Ok(staged_files)
}

fn stage_input_files(inputs: &[CommandInputParameter], input_values: &Option<HashMap<String, DefaultValue>>, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut staged_files = vec![];

    for input in inputs {
        //step ahead if not file or dir
        if input.type_ != CWLType::File && input.type_ != CWLType::Directory {
            continue;
        }
        let incoming_data = evaluate_input(input, input_values)?;
        let incoming_file = incoming_data.as_value_string();
        let outcoming_file = handle_filename(&incoming_data);
        let outcoming_file_stripped = outcoming_file.trim_start_matches("../").to_string();
        let into_path = path.join(&outcoming_file_stripped);
        let path_str = &into_path.to_string_lossy();

        if input.type_ == CWLType::File {
            copy_file(&incoming_file, path_str).map_err(|e| format!("Failed to copy file from {} to {}: {}", incoming_file, path_str, e))?;
            staged_files.push(path_str.clone().into_owned());
        } else if input.type_ == CWLType::Directory {
            copy_dir(&incoming_file, path_str).map_err(|e| format!("Failed to copy directory from {} to {}: {}", incoming_file, path_str, e))?;
            staged_files.push(path_str.clone().into_owned());
        }
        staged_files.extend(stage_secondary_files(incoming_data, path)?);
    }
    Ok(staged_files)
}

fn stage_secondary_files(incoming_data: DefaultValue, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut staged_files = vec![];
    match &incoming_data {
        DefaultValue::File(file) => {
            if let Some(secondary_files) = &file.secondary_files {
                for value in secondary_files {
                    let incoming_file = value.as_value_string();
                    let outcoming_file = handle_filename(&value);
                    let outcoming_file_stripped = outcoming_file.trim_start_matches("../").to_string();
                    let into_path = path.join(&outcoming_file_stripped);
                    let path_str = &into_path.to_string_lossy();
                    match value {
                        DefaultValue::File(_) => {
                            copy_file(&incoming_file, path_str).map_err(|e| format!("Failed to copy file from {} to {}: {}", incoming_file, path_str, e))?;
                            staged_files.push(path_str.clone().into_owned());
                        }
                        DefaultValue::Directory(_) => {
                            copy_dir(&incoming_file, path_str).map_err(|e| format!("Failed to copy directory from {} to {}: {}", incoming_file, path_str, e))?;
                            staged_files.push(path_str.clone().into_owned());
                        }
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    }
    Ok(staged_files)
}

fn handle_filename(value: &DefaultValue) -> String {
    let join_with_basename = |location: &str, basename: &Option<String>| {
        if let Some(basename) = basename {
            let path = Path::new(location);
            let parent = path.parent().unwrap_or_else(|| Path::new(""));
            parent.join(basename).to_string_lossy().into_owned()
        } else {
            location.to_string()
        }
    };

    match value {
        DefaultValue::File(item) => join_with_basename(&item.location, &item.basename),
        DefaultValue::Directory(item) => join_with_basename(&item.location, &item.basename),
        DefaultValue::Any(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cwl::{
        clt::InitialWorkDirRequirement,
        types::{Directory, File},
    };
    use std::vec;
    use tempfile::tempdir;

    #[test]
    fn test_stage_requirement() {
        //create tmp_dir
        let tmp_dir = tempdir().unwrap();

        let test_file = "tests/test_data/input.txt";

        let requirement = Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(test_file));
        let list = stage_requirements(&Some(vec![requirement]), &tmp_dir.path().to_path_buf()).unwrap();

        let expected_path = tmp_dir.path().join(test_file);

        assert_eq!(list.len(), 1);
        assert_eq!(list[0], expected_path.to_string_lossy().into_owned());
    }

    #[test]
    fn test_stage_input_files_dir() {
        //create tmp_dir
        let tmp_dir = tempdir().unwrap();

        let test_dir = "tests/";

        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::Directory)
            .with_default_value(DefaultValue::Directory(Directory::from_location(&test_dir.to_string())));

        let list = stage_input_files(&[input], &None, &tmp_dir.path().to_path_buf()).unwrap();

        let expected_path = tmp_dir.path().join(test_dir);

        assert_eq!(list.len(), 1);
        assert_eq!(list[0], expected_path.to_string_lossy().into_owned());
    }

    #[test]
    fn test_stage_input_files_file() {
        //create tmp_dir
        let tmp_dir = tempdir().unwrap();

        let test_dir = "tests/test_data/input.txt";

        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::File)
            .with_default_value(DefaultValue::File(File::from_location(&test_dir.to_string())));

        let list = stage_input_files(&[input], &None, &tmp_dir.path().to_path_buf()).unwrap();

        let expected_path = tmp_dir.path().join(test_dir);

        assert_eq!(list.len(), 1);
        assert_eq!(list[0], expected_path.to_string_lossy().into_owned());
    }
}
