use crate::{
    environment::RuntimeEnvironment,
    io::{copy_dir, copy_file, create_and_write_file, get_random_filename, make_relative_to},
    InputObject,
};
use commonwl::{
    inputs::CommandInputParameter,
    requirements::{Requirement, WorkDirItem},
    CWLDocument, CWLType, DefaultValue, Directory, Entry, File, PathItem,
};
use glob::glob;
use pathdiff::diff_paths;
use std::{
    env,
    error::Error,
    fs,
    io::{self},
    path::{Path, PathBuf, MAIN_SEPARATOR_STR},
    vec,
};
use urlencoding::decode;

pub(crate) fn stage_required_files<P: AsRef<Path>, Q: AsRef<Path>, R: AsRef<Path>>(
    tool: &CWLDocument,
    input_values: &InputObject,
    runtime: &mut RuntimeEnvironment,
    tool_path: P,
    path: Q,
    out_dir: R,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut staged_files: Vec<String> = vec![];
    //stage requirements
    staged_files.extend(stage_requirements(&input_values.requirements, tool_path.as_ref(), path.as_ref())?);

    //stage inputs
    staged_files.extend(stage_input_files(
        &tool.inputs,
        runtime,
        tool_path.as_ref(),
        path.as_ref(),
        out_dir.as_ref(),
    )?);
    //do not remove file multiple times if input matches InitialWorkDirRequirement filename
    staged_files.sort_unstable();
    staged_files.dedup();

    Ok(staged_files)
}

fn stage_requirements(requirements: &[Requirement], tool_path: &Path, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut staged_files = vec![];

    for requirement in requirements {
        if let Requirement::InitialWorkDirRequirement(iwdr) = requirement {
            for listing in &iwdr.listing {
                let into_path = match listing {
                    WorkDirItem::Dirent(dirent) => {
                        if let Some(entryname) = &dirent.entryname {
                            path.join(entryname)
                        } else {
                            let eval = match &dirent.entry {
                                Entry::Source(src) => Path::new(src),
                                Entry::Include(include) => &get_iwdr_src(tool_path, &include.include)?,
                            };
                            path.join(eval.file_name().unwrap())
                        }
                    }
                    WorkDirItem::FileOrDirectory(val) => match &**val {
                        DefaultValue::File(file) => {
                            let location = Path::new(file.location.as_ref().unwrap());
                            path.join(location.file_name().unwrap())
                        }
                        DefaultValue::Directory(directory) => {
                            let location = Path::new(directory.location.as_ref().unwrap());
                            path.join(location.file_name().unwrap())
                        }
                        _ => unreachable!(),
                    },
                    WorkDirItem::Expression(_) => unreachable!(), //resolved before!
                };
                //stage as listing's entry name
                let path_str = &into_path.to_string_lossy();
                match &listing {
                    WorkDirItem::Dirent(dirent) => match &dirent.entry {
                        Entry::Source(src) => {
                            if fs::exists(src).unwrap_or(false) {
                                let src = Path::new(src); //is safer ;)
                                if src.is_file() {
                                    copy_file(src, &into_path).map_err(|e| format!("Failed to copy file from {src:?} to {path_str}: {e}"))?;
                                } else {
                                    copy_dir(src, &into_path).map_err(|e| format!("Failed to copy dir from {src:?} to {path_str}: {e}"))?;
                                }
                            } else {
                                create_and_write_file(&into_path, src).map_err(|e| format!("Failed to create file {into_path:?}: {e}"))?;
                            }
                        }
                        Entry::Include(include) => {
                            let path = get_iwdr_src(tool_path, &include.include)?;
                            copy_file(&path, &into_path).map_err(|e| format!("Failed to copy file from {path:?} to {into_path:?}: {e}"))?;
                        }
                    },
                    WorkDirItem::FileOrDirectory(val) => match &**val {
                        DefaultValue::File(file) => {
                            let path = get_iwdr_src(tool_path, file.location.as_ref().unwrap())?;
                            copy_file(&path, &into_path).map_err(|e| format!("Failed to copy file from {path:?} to {into_path:?}: {e}"))?;
                        }
                        DefaultValue::Directory(directory) => {
                            let path = get_iwdr_src(tool_path, directory.location.as_ref().unwrap())?;
                            copy_dir(&path, &into_path).map_err(|e| format!("Failed to copy dir from {path:?} to {into_path:?}: {e}"))?;
                        }
                        _ => unreachable!(),
                    },
                    WorkDirItem::Expression(_) => unreachable!(),
                }
                staged_files.push(path_str.clone().into_owned());
            }
        } else if let Requirement::DockerRequirement(dr) = requirement {
            if let Some(Entry::Include(file)) = &dr.docker_file {
                let destination_file = path.join("Dockerfile");
                copy_file(tool_path.join(&file.include), &destination_file)?;
                staged_files.push(destination_file.to_string_lossy().into_owned());
            }
        }
    }

    Ok(staged_files)
}

fn get_iwdr_src(tool_path: &Path, basepath: &String) -> Result<PathBuf, Box<dyn Error + 'static>> {
    let mut path = tool_path.join(basepath);
    if !path.exists() {
        let current = env::current_dir()?;
        let file_path: String = basepath.clone().trim_start_matches(|c: char| !c.is_alphabetic()).to_string();
        path = current.join(file_path.clone());
        if !path.exists() {
            path = current.join(tool_path).join(file_path);
        }
    }

    Ok(path)
}

fn stage_input_files(
    inputs: &[CommandInputParameter],
    runtime: &mut RuntimeEnvironment,
    tool_path: &Path,
    path: &Path,
    out_dir: &Path,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut staged_files = vec![];

    for input in inputs {
        //step ahead if not file or dir
        if input.type_ != CWLType::File && input.type_ != CWLType::Directory {
            continue;
        }

        //get correct data
        let mut data = runtime.inputs.remove(&input.id).unwrap();

        //handle file literals
        if let DefaultValue::File(ref mut f) = data {
            if f.location.is_none() {
                if let Some(contents) = &f.contents {
                    let dest = path.join(get_random_filename(".literal", ""));
                    fs::write(&dest, contents)?;
                    f.location = Some(dest.to_string_lossy().into_owned());

                    runtime.inputs.insert(input.id.clone(), data);
                    continue;
                }
            } else if let Some(location) = &f.location {
                if location.starts_with("https://") || location.starts_with("http://") {
                    //download file
                    let client = reqwest::blocking::Client::new();
                    let mut res = client.get(location).send()?;
                    if res.status() != reqwest::StatusCode::OK {
                        return Err(format!("Failed to download file from {}: {}", location, res.status()).into());
                    }

                    //get file name from url
                    if let Some(segment) = res
                        .url()
                        .path_segments()
                        .and_then(|mut segments| segments.next_back())
                        .map(|filename| Path::new(&runtime.runtime["tmpdir"].to_string()).join(filename))
                    {
                        let path = Path::new(&runtime.runtime["tmpdir"].to_string()).join(segment);
                        let mut out = fs::File::create(&path)?;
                        io::copy(&mut res, &mut out)?;

                        //set updated path:
                        f.location = Some(path.to_string_lossy().into_owned());
                    } else {
                        return Err("Could not extract filename from URL.".into());
                    }
                }
            }
        }

        let data_location = decode(&data.as_value_string()).unwrap().to_string();
        let mut data_path = PathBuf::from(&data_location);

        //check exists? otherwise search relative to tool
        if !data_path.exists() {
            data_path = tool_path.join(data_path);
        }

        let mut staged_filename = handle_filename(&data);
        if let Some(tmpdir) = runtime.runtime.get("tmpdir") {
            if let Some(diff) = diff_paths(&staged_filename, tmpdir.to_string()) {
                staged_filename = diff.to_string_lossy().into_owned();
            }
        }
        let staged_filename_relative = make_relative_to(&staged_filename, out_dir.to_str().unwrap_or_default());

        let staged_filename_relative = staged_filename_relative
            .trim_start_matches(&("..".to_owned() + MAIN_SEPARATOR_STR))
            .to_string();

        let staged_path = path.join(staged_filename_relative);
        let staged_path_str = staged_path.to_string_lossy().into_owned();

        let staged_data = match &data {
            DefaultValue::File(file) => DefaultValue::File(File {
                location: Some(staged_path_str.clone()),
                ..file.clone()
            }),
            DefaultValue::Directory(dir) => DefaultValue::Directory(Directory {
                location: Some(staged_path_str.clone()),
                ..dir.clone()
            }),
            _ => data.clone(),
        };
        runtime.inputs.insert(input.id.clone(), staged_data);

        if input.type_ == CWLType::File {
            copy_file(&data_path, &staged_path).map_err(|e| format!("Failed to copy file from {data_path:?} to {staged_path:?}: {e}"))?;
            staged_files.push(staged_path_str.clone());
            staged_files.extend(stage_secondary_files(&data, path)?);
            staged_files.extend(stage_secondary_inputs(&data, path, input)?);
        } else if input.type_ == CWLType::Directory {
            copy_dir(&data_path, &staged_path).map_err(|e| format!("Failed to copy directory from {data_path:?} to {staged_path:?}: {e}"))?;
            staged_files.push(staged_path_str.clone());
        }
    }
    Ok(staged_files)
}

fn stage_secondary_inputs(incoming_data: &DefaultValue, path: &Path, input: &CommandInputParameter) -> Result<Vec<String>, Box<dyn Error>> {
    let mut staged_files = vec![];

    if let DefaultValue::File(file) = &incoming_data {
        let file_loc = file.location.as_ref().unwrap().trim_start_matches("../").to_string();
        let file_dir = if let Some(dir_name) = &file.dirname {
            Path::new(dir_name)
        } else {
            Path::new(&file_loc).parent().unwrap_or_else(|| Path::new(""))
        };
        for item in &input.secondary_files {
            let mut matched = false;
            let pattern = if item.pattern.starts_with(file_dir.to_string_lossy().as_ref()) {
                &item.pattern
            } else {
                &file_dir.join(format!("*{}", item.pattern)).to_string_lossy().into_owned()
            };
            let pattern = pattern.trim();

            for res in glob(pattern)? {
                let res = res?;
                let dest = path.join(&res);
                copy_file(&res, &dest).map_err(|e| format!("Failed to copy file from {res:?} to {dest:?}: {e}"))?;
                staged_files.push(dest.to_string_lossy().into_owned());
                matched = true;
            }
            if !matched && item.required {
                return Err(format!("Required secondary file pattern {} not found in {:?}", item.pattern, file_dir).into());
            }
        }
    }

    Ok(staged_files)
}
fn stage_secondary_files(incoming_data: &DefaultValue, path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut staged_files = vec![];
    if let DefaultValue::File(file) = &incoming_data {
        let file_loc = file.location.as_ref().unwrap().trim_start_matches("../").to_string();
        let file_dir = Path::new(&file_loc).parent().unwrap_or_else(|| Path::new(""));

        if let Some(secondary_files) = &file.secondary_files {
            for value in secondary_files {
                let incoming_file = value.as_value_string();
                let outcoming_file = handle_filename(value);
                let outcoming_file_stripped = outcoming_file.trim_start_matches("../").to_string();
                let into_path = if outcoming_file_stripped.starts_with(file_dir.to_str().unwrap_or_default()) {
                    path.join(&outcoming_file_stripped)
                } else {
                    path.join(file_dir).join(&outcoming_file_stripped)
                };

                let path_str = &into_path.to_string_lossy();
                match value {
                    DefaultValue::File(_) => {
                        copy_file(&incoming_file, &into_path)
                            .map_err(|e| format!("Failed to copy file from {incoming_file} to {into_path:?}: {e}"))?;
                        staged_files.push(path_str.clone().into_owned());
                    }
                    DefaultValue::Directory(_) => {
                        copy_dir(&incoming_file, &into_path)
                            .map_err(|e| format!("Failed to copy directory from {incoming_file} to {into_path:?}: {e}"))?;
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
            if !location.ends_with(basename) {
                basename.to_string()
            } else {
                location.to_string()
            }
        } else {
            location.to_string()
        }
    };

    match value {
        DefaultValue::File(item) => join_with_basename(&item.get_location(), &item.basename),
        DefaultValue::Directory(item) => join_with_basename(&item.get_location(), &item.basename),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commonwl::{requirements::InitialWorkDirRequirement, Directory, File, StringOrNumber};
    use serial_test::serial;
    use std::{collections::HashMap, path::PathBuf, vec};
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn test_stage_requirement() {
        //create tmp_dir
        let tmp_dir = tempdir().unwrap();

        let test_file = "tests/test_data/input.txt";

        let requirement = Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(test_file));
        let list = stage_requirements(&[requirement], Path::new("../../"), tmp_dir.path()).unwrap();

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
        let list = stage_requirements(&[requirement], Path::new("../../"), tmp_dir.path()).unwrap();

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

        let value = DefaultValue::Directory(Directory::from_location(test_dir));
        let input = CommandInputParameter::default().with_id("test").with_type(CWLType::Directory);

        let list = stage_input_files(
            &[input],
            &mut RuntimeEnvironment::default().with_inputs(HashMap::from([("test".to_string(), value)])),
            Path::new("../../"),
            tmp_dir.path(),
            &PathBuf::from(""),
        )
        .unwrap();
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

        let value = DefaultValue::File(File::from_location(test_dir));
        let input = CommandInputParameter::default().with_id("test").with_type(CWLType::File);

        let list = stage_input_files(
            &[input],
            &mut RuntimeEnvironment::default().with_inputs(HashMap::from([("test".to_string(), value)])),
            Path::new("../../"),
            tmp_dir.path(),
            &PathBuf::from(""),
        )
        .unwrap();

        let expected_path = tmp_dir.path().join(test_dir);

        assert_eq!(list.len(), 1);
        assert_eq!(list[0], expected_path.to_string_lossy().into_owned());
    }

    #[test]
    #[serial]
    fn test_stage_secondary_files() {
        let tmp_dir = tempdir().unwrap();

        let test_file = "../../tests/test_data/input.txt";
        let secondary_file = "../../tests/test_data/echo.py";
        let mut file = File::from_location(test_file);
        file.secondary_files = Some(vec![DefaultValue::File(File::from_location(secondary_file))]);
        let data = DefaultValue::File(file);
        let list = stage_secondary_files(&data, tmp_dir.path()).unwrap();

        let expected_path = tmp_dir.path().join(secondary_file.strip_prefix("../../").unwrap());
        //secondary file should be there
        assert_eq!(list, vec![expected_path.to_string_lossy()]);
        assert!(expected_path.exists());
    }

    #[test]
    #[serial]
    fn test_stage_remote_files() {
        //create tmp_dir
        let temp = tempdir().unwrap();
        let working = tempdir().unwrap();

        let file = "https://raw.githubusercontent.com/fairagro/m4.4_sciwin_client/refs/heads/main/README.md";
        let value = DefaultValue::File(File::from_location(file));
        let input = CommandInputParameter::default().with_id("test").with_type(CWLType::File);

        let list = stage_input_files(
            &[input],
            &mut RuntimeEnvironment::default()
                .with_inputs(HashMap::from([("test".to_string(), value)]))
                .with_runtime(HashMap::from([(
                    "tmpdir".to_string(),
                    StringOrNumber::String(temp.path().to_string_lossy().into_owned()),
                )])),
            Path::new("../../"),
            working.path(),
            &PathBuf::from(""),
        )
        .unwrap();

        let expected_path = working.path().join("README.md");

        assert_eq!(list.len(), 1);
        assert_eq!(list[0], expected_path.to_string_lossy().into_owned());
    }
}
