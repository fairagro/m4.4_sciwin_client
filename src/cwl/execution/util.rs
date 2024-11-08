use crate::{
    cwl::{
        clt::{CommandInputParameter, CommandOutputParameter, DefaultValue},
        types::{CWLType, OutputDirectory, OutputFile, OutputItem},
    },
    io::{copy_file, get_file_checksum, get_file_size},
};
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

///Either gets the default value for input or the provided one (preferred)
pub fn evaluate_input_as_string(input: &CommandInputParameter, input_values: &Option<HashMap<String, DefaultValue>>) -> Result<String, Box<dyn Error>> {
    Ok(evaluate_input(input, input_values)?.as_value_string())
}

///Either gets the default value for input or the provided one (preferred)
pub fn evaluate_input(input: &CommandInputParameter, input_values: &Option<HashMap<String, DefaultValue>>) -> Result<DefaultValue, Box<dyn Error>> {
    if let Some(ref values) = input_values {
        if let Some(value) = values.get(&input.id) {
            if !value.has_matching_type(&input.type_) {
                //change handling accordingly in utils on main branch!
                eprintln!("CWLType is not matching input type");
                Err("CWLType is not matching input type")?;
            }
            return Ok(value.clone());
        }
    } else if let Some(default_) = &input.default {
        return Ok(default_.clone());
    } else {
        eprintln!("You did not include a value for {}", input.id);
        Err(format!("You did not include a value for {}", input.id).as_str())?;
    }
    Err(format!("Could not evaluate input: {}", input.id))?
}

///Copies back requested outputs and writes to commandline
pub fn evaluate_outputs(tool_outputs: &Vec<CommandOutputParameter>, initial_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    //copy back requested output
    let mut outputs: HashMap<&String, OutputItem> = HashMap::new();
    for output in tool_outputs {
        if output.type_ == CWLType::File {
            if let Some(binding) = &output.output_binding {
                let path = &initial_dir.join(&binding.glob);
                fs::copy(&binding.glob, path).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", &binding.glob, path, e))?;
                eprintln!("📜 Wrote output file: {:?}", path);
                outputs.insert(&output.id, OutputItem::OutputFile(get_file_metadata(path.into(), output.format.clone())));
            }
        } else if output.type_ == CWLType::Directory {
            if let Some(binding) = &output.output_binding {
                let dir = if &binding.glob != "." {
                    initial_dir
                } else {
                    let working_dir = env::current_dir()?;
                    let raw_basename = working_dir.file_name().unwrap().to_string_lossy();
                    let glob_name = if let Some(stripped) = raw_basename.strip_prefix(".") {
                        stripped.to_owned()
                    } else {
                        raw_basename.into_owned()
                    };
                    &initial_dir.join(&glob_name)
                };
                fs::create_dir_all(dir)?;

                let out_dir = copy_output_dir(&binding.glob, dir.to_str().unwrap()).map_err(|e| format!("Failed to copy: {}", e))?;
                outputs.insert(&output.id, OutputItem::OutputDirectory(out_dir));
            }
        }
    }
    //print output metadata
    let json = serde_json::to_string_pretty(&outputs)?;
    println!("{}", json);
    Ok(())
}

fn get_file_metadata(path: PathBuf, format: Option<String>) -> OutputFile {
    let basename = path.file_name().and_then(|n| n.to_str()).unwrap().to_string();
    let size = get_file_size(&path).unwrap_or_else(|_| panic!("Could not get filesize: {:?}", path));
    let checksum = format!("sha1${}", get_file_checksum(&path).unwrap_or_else(|_| panic!("Could not get checksum: {:?}", path)));

    OutputFile {
        location: format!("file://{}", path.display()),
        basename,
        class: "File".to_string(),
        checksum,
        size,
        path: path.to_string_lossy().into_owned(),
        format: resolve_format(format),
    }
}

fn resolve_format(format: Option<String>) -> Option<String> {
    if let Some(format) = format {
        let edam_url = "http://edamontology.org/";
        Some(format.replace("edam:", edam_url))
    } else {
        None
    }
}

fn get_diretory_metadata(path: PathBuf) -> OutputDirectory {
    OutputDirectory {
        location: format!("file://{}", path.display()),
        basename: path.file_name().unwrap().to_string_lossy().into_owned(),
        class: "Directory".to_string(),
        listing: vec![],
        path: path.to_string_lossy().into_owned(),
    }
}

pub fn copy_output_dir(src: &str, dest: &str) -> Result<OutputDirectory, std::io::Error> {
    fs::create_dir_all(dest)?;
    let mut dir = get_diretory_metadata(dest.into());

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = Path::new(dest).join(entry.file_name());
        if src_path.is_dir() {
            let sub_dir = copy_output_dir(src_path.to_str().unwrap(), dest_path.to_str().unwrap())?;
            dir.listing.push(OutputItem::OutputDirectory(sub_dir));
        } else {
            copy_file(src_path.to_str().unwrap(), dest_path.to_str().unwrap())?;
            dir.listing.push(OutputItem::OutputFile(get_file_metadata(dest_path, None)))
        }
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cwl::clt::{CommandLineBinding, CommandOutputBinding},
        io::copy_dir,
    };
    use serde_yml::value;
    use serial_test::serial;
    use tempfile::tempdir;

    #[test]
    pub fn test_evaluate_input() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::String)
            .with_binding(CommandLineBinding::default().with_prefix(&"--arg".to_string()));
        let mut values = HashMap::new();
        values.insert("test".to_string(), DefaultValue::Any(value::Value::String("Hello!".to_string())));

        let evaluation = evaluate_input(&input, &Some(values.clone())).unwrap();

        assert_eq!(evaluation, values["test"]);
    }

    #[test]
    pub fn test_evaluate_input_as_string() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::String)
            .with_binding(CommandLineBinding::default().with_prefix(&"--arg".to_string()));
        let mut values = HashMap::new();
        values.insert("test".to_string(), DefaultValue::Any(value::Value::String("Hello!".to_string())));

        let evaluation = evaluate_input_as_string(&input, &Some(values.clone())).unwrap();

        assert_eq!(evaluation, values["test"].as_value_string());
    }

    #[test]
    #[serial]
    pub fn test_evaluate_outputs() {
        let dir = tempdir().unwrap();
        let current = env::current_dir().unwrap();

        let output = CommandOutputParameter::default()
            .with_id("out")
            .with_type(CWLType::File)
            .with_binding(CommandOutputBinding {
                glob: "tests/test_data/file.txt".to_string(),
            });

        fs::create_dir_all(dir.path().join("tests/test_data")).expect("Could not create folders");
        fs::copy("tests/test_data/file.txt", dir.path().join("tests/test_data/file.txt")).expect("Unable to copy file");
        env::set_current_dir(dir.path()).unwrap();

        let result = evaluate_outputs(&vec![output], &current);
        assert!(result.is_ok());

        env::set_current_dir(current).unwrap();
    }

    #[test]
    #[serial]
    pub fn test_get_file_metadata() {
        let path = env::current_dir().unwrap().join("tests/test_data/file.txt");
        let result = get_file_metadata(path.to_path_buf(), None);
        let expected = OutputFile {
            location: format!("file://{}", path.to_string_lossy().into_owned()),
            basename: "file.txt".to_string(),
            class: "File".to_string(),
            checksum: "sha1$2c3cafa4db3f3e1e51b3dff4303502dbe42b7a89".to_string(),
            size: 4,
            path: path.to_string_lossy().into_owned(),
            format: None,
        };

        assert_eq!(result, expected);
    }

    #[test]
    #[serial]
    pub fn test_get_directory_metadata() {
        let path = env::current_dir().unwrap().join("tests/test_data");
        let result = get_diretory_metadata(path.clone());
        let expected = OutputDirectory {
            location: format!("file://{}", path.to_string_lossy().into_owned()),
            basename: path.file_name().unwrap().to_string_lossy().into_owned(),
            class: "Directory".to_string(),
            listing: vec![],
            path: path.to_string_lossy().into_owned(),
        };
        assert_eq!(result, expected);
    }

    #[test]
    #[serial]
    pub fn test_copy_output_dir() {
        let dir = tempdir().unwrap();
        let stage = dir.path().join("tests/test_data/test_dir");
        let current = env::current_dir().unwrap().join("tests/test_data/test_dir");
        let cwd = current.to_str().unwrap();
        copy_dir(cwd, stage.to_str().unwrap()).unwrap();

        let result = copy_output_dir(stage.to_str().unwrap(), cwd).expect("could not copy dir");
        let expected = OutputDirectory {
            location: format!("file://{}", cwd),
            basename: "test_dir".to_string(),
            class: "Directory".to_string(),
            listing: vec![
                OutputItem::OutputFile(OutputFile {
                    location: format!("file://{}/file.txt", cwd),
                    basename: "file.txt".to_string(),
                    class: "File".to_string(),
                    checksum: "sha1$2c3cafa4db3f3e1e51b3dff4303502dbe42b7a89".to_string(),
                    size: 4,
                    path: format!("{}/file.txt", cwd),
                    format: None,
                }),
                OutputItem::OutputFile(OutputFile {
                    location: format!("file://{}/input.txt", cwd),
                    basename: "input.txt".to_string(),
                    class: "File".to_string(),
                    checksum: "sha1$22959e5335b177539ffcd81a5426b9eca4f4cbec".to_string(),
                    size: 26,
                    path: format!("{}/input.txt", cwd),
                    format: None,
                }),
            ],
            path: cwd.to_string(),
        };

        assert_eq!(result, expected);
    }

    #[test]
    pub fn test_resolve_format() {
        let result = resolve_format(Some("edam:format_1234".to_string())).unwrap();
        let expected = "http://edamontology.org/format_1234";

        assert_eq!(result, expected.to_string())
    }
}