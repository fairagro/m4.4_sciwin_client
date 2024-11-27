use super::util::evaluate_input;
use crate::{
    cwl::{
        clt::CommandLineTool,
        inputs::CommandInputParameter,
        outputs::CommandOutputParameter,
        requirements::Requirement,
        types::{CWLType, DefaultValue, Entry},
    },
    io::{copy_dir, copy_file, create_and_write_file},
};
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs,
    path::{Path, PathBuf, MAIN_SEPARATOR_STR},
    vec,
};

pub fn stage_required_files(tool: &CommandLineTool, input_values: &Option<HashMap<String, DefaultValue>>, tool_path: &Path, path: PathBuf) -> Result<Vec<String>, Box<dyn Error>> {
    let mut staged_files: Vec<String> = vec![];
    //stage requirements
    staged_files.extend(stage_requirements(&tool.requirements, tool_path, &path)?);

    //stage inputs
    staged_files.extend(stage_input_files(&tool.inputs, input_values, tool_path, &path)?);

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

fn stage_requirements(requirements: &Option<Vec<Requirement>>, tool_path: &Path, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
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
                            let include_path = tool_path.join(&include.include);
                            copy_file(include_path.to_str().unwrap(), path_str).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", include_path, into_path, e))?;
                        }
                    }
                    staged_files.push(path_str.clone().into_owned());
                }
            }
        }
    }

    Ok(staged_files)
}

fn stage_input_files(inputs: &[CommandInputParameter], input_values: &Option<HashMap<String, DefaultValue>>, tool_path: &Path, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut staged_files = vec![];

    for input in inputs {
        //step ahead if not file or dir
        if input.type_ != CWLType::File && input.type_ != CWLType::Directory {
            continue;
        }
        let incoming_data = evaluate_input(input, input_values)?;
        let mut incoming_file = incoming_data.as_value_string();

        //check exists? otherwise search relative to tool
        let mut incoming_path = Path::new(&incoming_file).to_path_buf();

        if !incoming_path.exists() {
            incoming_path = tool_path.join(&incoming_file);
        }
        incoming_file = incoming_path.to_string_lossy().to_string();

        let current_dir = env::current_dir()?.to_string_lossy().into_owned() + MAIN_SEPARATOR_STR;

        let outcoming_file = handle_filename(&incoming_data);
        let outcoming_file_relative = outcoming_file.strip_prefix(&current_dir).unwrap_or(&outcoming_file);
        let outcoming_file_stripped = outcoming_file_relative.trim_start_matches("../").to_string();

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
    if let DefaultValue::File(file) = &incoming_data {
        if let Some(secondary_files) = &file.secondary_files {
            for value in secondary_files {
                let incoming_file = value.as_value_string();
                let outcoming_file = handle_filename(value);
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
    Ok(staged_files)
}

fn handle_filename(value: &DefaultValue) -> String {
    let join_with_basename = |location: &str, basename: &Option<String>| {
        if let Some(basename) = basename {
            basename.to_string()
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
        outputs::CommandOutputBinding,
        requirements::InitialWorkDirRequirement,
        types::{Directory, File},
    };
    use serial_test::serial;
    use std::vec;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn test_stage_requirement() {
        //create tmp_dir
        let tmp_dir = tempdir().unwrap();

        let test_file = "tests/test_data/input.txt";

        let requirement = Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(test_file));
        let list = stage_requirements(&Some(vec![requirement]), Path::new("."), tmp_dir.path()).unwrap();

        let expected_path = tmp_dir.path().join(test_file);

        assert_eq!(list.len(), 1);
        assert_eq!(list[0], expected_path.to_string_lossy().into_owned());
    }

    #[test]
    #[serial]
    fn test_stage_requirement_inline() {
        //create tmp_dir
        let tmp_dir = tempdir().unwrap();

        let test_contents = "Hello fellow CWL-enjoyers";

        let requirement = Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_contents("input.txt", test_contents));
        let list = stage_requirements(&Some(vec![requirement]), Path::new("."), tmp_dir.path()).unwrap();

        let expected_path = tmp_dir.path().join("input.txt");

        assert_eq!(list.len(), 1);
        assert_eq!(list[0], expected_path.to_string_lossy().into_owned());

        //read contents
        let result = fs::read_to_string(expected_path).unwrap();
        assert_eq!(result, test_contents);
    }

    #[test]
    #[serial]
    fn test_stage_input_files_dir() {
        //create tmp_dir
        let tmp_dir = tempdir().unwrap();

        let test_dir = "tests/";

        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::Directory)
            .with_default_value(DefaultValue::Directory(Directory::from_location(&test_dir.to_string())));

        let list = stage_input_files(&[input], &None, Path::new("."), tmp_dir.path()).unwrap();

        let expected_path = tmp_dir.path().join(test_dir);

        assert_eq!(list.len(), 1);
        assert_eq!(list[0], expected_path.to_string_lossy().into_owned());
    }

    #[test]
    #[serial]
    fn test_stage_input_files_file() {
        //create tmp_dir
        let tmp_dir = tempdir().unwrap();

        let test_dir = "tests/test_data/input.txt";

        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::File)
            .with_default_value(DefaultValue::File(File::from_location(&test_dir.to_string())));

        let list = stage_input_files(&[input], &None, Path::new("."), tmp_dir.path()).unwrap();

        let expected_path = tmp_dir.path().join(test_dir);

        assert_eq!(list.len(), 1);
        assert_eq!(list[0], expected_path.to_string_lossy().into_owned());
    }

    #[test]
    #[serial]
    fn test_unstage_files() {
        let tmp_dir = tempdir().unwrap();

        let test_dir = "tests/test_data/input.txt";

        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::File)
            .with_default_value(DefaultValue::File(File::from_location(&test_dir.to_string())));

        let list = stage_input_files(&[input], &None, Path::new("."), tmp_dir.path()).unwrap();

        unstage_files(&list, tmp_dir.path(), &[]).unwrap();
        //file should be gone
        assert!(!Path::new(&list[0]).exists())
    }

    #[test]
    #[serial]
    fn test_unstage_files_dir() {
        let tmp_dir = tempdir().unwrap();

        let test_dir = "tests/test_data";

        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::Directory)
            .with_default_value(DefaultValue::Directory(Directory::from_location(&test_dir.to_string())));

        let list = stage_input_files(&[input], &None, Path::new("."), tmp_dir.path()).unwrap();

        unstage_files(&list, tmp_dir.path(), &[]).unwrap();
        //file should be gone
        assert!(!Path::new(&list[0]).exists())
    }

    #[test]
    #[serial]
    fn test_unstage_files_not_in_output() {
        let tmp_dir = tempdir().unwrap();

        let test_file = "tests/test_data/input.txt";

        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::File)
            .with_default_value(DefaultValue::File(File::from_location(&test_file.to_string())));

        let output = CommandOutputParameter::default().with_binding(CommandOutputBinding {
            glob: "tests/test_data/input.txt".to_string(),
        });

        let list = stage_input_files(&[input], &None, Path::new("."), tmp_dir.path()).unwrap();

        unstage_files(&list, tmp_dir.path(), &[output]).unwrap();
        //file should still be there
        assert!(Path::new(&list[0]).exists())
    }

    #[test]
    #[serial]
    fn test_stage_secondary_files() {
        let tmp_dir = tempdir().unwrap();

        let test_file = "tests/test_data/input.txt";
        let secondary_file = "tests/test_data/echo.py";
        let mut file = File::from_location(&test_file.to_string());
        file.secondary_files = Some(vec![DefaultValue::File(File::from_location(&secondary_file.to_string()))]);
        let data = DefaultValue::File(file);

        let list = stage_secondary_files(data, tmp_dir.path()).unwrap();

        let expected_path = tmp_dir.path().join(secondary_file);
        //secondary file should be there
        assert_eq!(list, vec![expected_path.to_string_lossy()]);
        assert!(expected_path.exists());
    }
}
