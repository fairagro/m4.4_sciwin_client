use crate::io::{copy_file, get_first_file_with_prefix};
use cwl::{
    clt::CommandLineTool,
    et::ExpressionTool,
    inputs::CommandInputParameter,
    outputs::CommandOutputParameter,
    types::{CWLType, DefaultValue, Directory, File},
};
use fancy_regex::Regex;
use serde_yaml::Value;
use std::{collections::HashMap, env, error::Error, fmt::Debug, fs, path::Path, process::Command};

///Either gets the default value for input or the provided one (preferred)
pub(crate) fn evaluate_input_as_string(
    input: &CommandInputParameter,
    input_values: &HashMap<String, DefaultValue>,
) -> Result<String, Box<dyn Error>> {
    Ok(evaluate_input(input, input_values)?.as_value_string())
}

///Either gets the default value for input or the provided one (preferred)
pub(crate) fn evaluate_input(input: &CommandInputParameter, input_values: &HashMap<String, DefaultValue>) -> Result<DefaultValue, Box<dyn Error>> {
    if let Some(value) = input_values.get(&input.id) {
        if (matches!(input.type_, CWLType::Any) || input.type_.is_optional()) && matches!(value, DefaultValue::Any(Value::Null)) {
            if let Some(default_) = &input.default {
                return Ok(default_.clone());
            }
        }

        if value.has_matching_type(&input.type_) {
            return Ok(value.clone());
        } else {
            Err(format!(
                "CWLType '{:?}' is not matching input type. Input was: \n{:#?}",
                &input.type_, value
            ))?
        }
    } else if let Some(default_) = &input.default {
        return Ok(default_.clone());
    }

    if let CWLType::Optional(_) = input.type_ {
        return Ok(DefaultValue::Any(Value::Null));
    } else {
        Err(format!("You did not include a value for {}", input.id).as_str())?;
    }

    Err(format!("Could not evaluate input: {}. Expected type: {:?}", input.id, input.type_))?
}

pub(crate) fn evaluate_expression_outputs(tool: &ExpressionTool, value: Value) -> Result<HashMap<String, DefaultValue>, Box<dyn Error>> {
    let mut outputs = HashMap::new();
    for output in &tool.outputs {
        if let Some(result) = value.get(&output.id) {
            match value {
                Value::Null if output.type_.is_optional() => {
                    outputs.insert(output.id.clone(), DefaultValue::Any(serde_yaml::Value::Null));
                }
                _ => {
                    let value = serde_yaml::from_str(&serde_json::to_string(&result)?)?;
                    outputs.insert(output.id.clone(), DefaultValue::Any(value));
                }
            }
        }
    }
    Ok(outputs)
}

///Copies back requested outputs and writes to commandline
pub(crate) fn evaluate_command_outputs(tool: &CommandLineTool, initial_dir: &Path) -> Result<HashMap<String, DefaultValue>, Box<dyn Error>> {
    //copy back requested output
    let mut outputs: HashMap<String, DefaultValue> = HashMap::new();
    for output in &tool.outputs {
        match &output.type_ {
            CWLType::Optional(inner) => {
                evaluate_output_impl(output, inner, initial_dir, &tool.stdout, &tool.stderr, &mut outputs).ok();
                //ignores all errors
            }
            _ => evaluate_output_impl(output, &output.type_, initial_dir, &tool.stdout, &tool.stderr, &mut outputs)?,
        }
    }
    Ok(outputs)
}

fn evaluate_output_impl(
    output: &CommandOutputParameter,
    type_: &CWLType,
    initial_dir: &Path,
    tool_stdout: &Option<String>,
    tool_stderr: &Option<String>,
    outputs: &mut HashMap<String, DefaultValue>,
) -> Result<(), Box<dyn Error>> {
    match type_ {
        CWLType::File | CWLType::Stdout | CWLType::Stderr => {
            if let Some(binding) = &output.output_binding {
                let path = &initial_dir.join(&binding.glob);
                fs::copy(&binding.glob, path).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", &binding.glob, path, e))?;
                eprintln!("📜 Wrote output file: {:?}", path);
                outputs.insert(output.id.clone(), DefaultValue::File(get_file_metadata(path, output.format.clone())));
            } else {
                let filename = match output.type_ {
                    CWLType::Stdout if tool_stdout.is_some() => tool_stdout.as_ref().unwrap(),
                    CWLType::Stderr if tool_stderr.is_some() => tool_stderr.as_ref().unwrap(),
                    _ => {
                        let mut file_prefix = output.id.clone();
                        file_prefix += match output.type_ {
                            CWLType::Stdout => "_stdout",
                            CWLType::Stderr => "_stderr",
                            _ => "",
                        };
                        &get_first_file_with_prefix(".", &file_prefix).unwrap_or_default()
                    }
                };
                let path = &initial_dir.join(filename);
                fs::copy(filename, path).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", &filename, path, e))?;
                eprintln!("📜 Wrote output file: {:?}", path);
                outputs.insert(output.id.clone(), DefaultValue::File(get_file_metadata(path, output.format.clone())));
            }
        }
        CWLType::Directory => {
            if let Some(binding) = &output.output_binding {
                let dir = if &binding.glob != "." {
                    &initial_dir.join(&binding.glob)
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
                outputs.insert(output.id.clone(), DefaultValue::Directory(out_dir));
            }
        }
        _ => {
            //string and has binding -> read file
            if let Some(binding) = &output.output_binding {
                let contents = fs::read_to_string(&binding.glob)?;
                outputs.insert(output.id.clone(), DefaultValue::Any(Value::String(contents)));
            }
        }
    }
    Ok(())
}

pub(crate) fn get_file_metadata<P: AsRef<Path> + Debug>(path: P, format: Option<String>) -> File {
    let mut f = File::from_location(&path.as_ref().to_string_lossy().to_string());
    f.format = format;
    f.snapshot()
}

pub(crate) fn get_diretory_metadata<P: AsRef<Path>>(path: P) -> Directory {
    Directory {
        location: Some(format!("file://{}", path.as_ref().display())),
        basename: Some(path.as_ref().file_name().unwrap().to_string_lossy().into_owned()),
        path: Some(path.as_ref().to_string_lossy().into_owned()),
        ..Default::default()
    }
}

pub(crate) fn copy_output_dir<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dest: Q) -> Result<Directory, std::io::Error> {
    fs::create_dir_all(&dest)?;
    let mut dir = get_diretory_metadata(&dest);

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.as_ref().join(entry.file_name());
        if src_path.is_dir() {
            let sub_dir = copy_output_dir(src_path, dest_path)?;
            if let Some(listing) = &mut dir.listing {
                listing.push(DefaultValue::Directory(sub_dir));
            } else {
                dir.listing = Some(vec![DefaultValue::Directory(sub_dir)])
            }
        } else {
            copy_file(src_path, &dest_path)?;
            if let Some(listing) = &mut dir.listing {
                listing.push(DefaultValue::File(get_file_metadata(dest_path, None)));
            } else {
                dir.listing = Some(vec![DefaultValue::File(get_file_metadata(dest_path, None))])
            }
        }
    }
    Ok(dir)
}

pub fn preprocess_cwl<P: AsRef<Path>>(contents: &str, path: P) -> String {
    let import_regex = Regex::new(r#"(?P<indent>[\p{Z}-]*)\{*"*\$import"*: (?P<file>[\w\.\-_]*)\}*"#).unwrap();
    import_regex
        .replace_all(contents, |captures: &fancy_regex::Captures| {
            let filename = captures.name("file").map_or("", |m| m.as_str());
            let indent = captures.name("indent").map_or("", |m| m.as_str());
            let indent_level: String = " ".repeat(indent.len());
            let path = path
                .as_ref()
                .parent()
                .map(|parent| parent.join(filename))
                .unwrap_or_else(|| Path::new(filename).to_path_buf());

            match fs::read_to_string(&path) {
                Ok(contents) => {
                    let mut lines = contents.lines();
                    let first_line = lines.next().unwrap_or_default();
                    let mut result = format!("{}{}", indent, first_line);
                    for line in lines {
                        result.push('\n');
                        result.push_str(&format!("{}{}", indent_level, line));
                    }
                    result
                }
                Err(_) => format!("{{\"error\": \"failed to load {}\"}}", filename),
            }
        })
        .to_string()
}

pub fn is_docker_installed() -> bool {
    let output = Command::new("docker").arg("--version").output();

    matches!(output, Ok(output) if output.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::copy_dir;
    use cwl::{
        inputs::CommandLineBinding,
        outputs::{CommandOutputBinding, CommandOutputParameter},
    };
    use serde_yaml::{value, Value};
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

        let evaluation = evaluate_input(&input, &values.clone()).unwrap();

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

        let evaluation = evaluate_input_as_string(&input, &values.clone()).unwrap();

        assert_eq!(evaluation, values["test"].as_value_string());
    }

    #[test]
    pub fn test_evaluate_input_empty_values() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::String)
            .with_binding(CommandLineBinding::default().with_prefix(&"--arg".to_string()))
            .with_default_value(DefaultValue::Any(Value::String("Nice".to_string())));
        let values = HashMap::new();
        let evaluation = evaluate_input_as_string(&input, &values.clone()).unwrap();

        assert_eq!(evaluation, "Nice".to_string());
    }

    #[test]
    pub fn test_evaluate_input_no_values() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::String)
            .with_binding(CommandLineBinding::default().with_prefix(&"--arg".to_string()))
            .with_default_value(DefaultValue::Any(Value::String("Nice".to_string())));
        let evaluation = evaluate_input_as_string(&input, &HashMap::new()).unwrap();

        assert_eq!(evaluation, "Nice".to_string());
    }

    #[test]
    pub fn test_evaluate_input_any() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::Any)
            .with_binding(CommandLineBinding::default().with_prefix(&"--arg".to_string()))
            .with_default_value(DefaultValue::Any(Value::String("Nice".to_string())));
        let evaluation = evaluate_input_as_string(&input, &HashMap::new()).unwrap();

        assert_eq!(evaluation, "Nice".to_string());
    }

    #[test]
    pub fn test_evaluate_input_any_null() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::Any)
            .with_binding(CommandLineBinding::default().with_prefix(&"--arg".to_string()))
            .with_default_value(DefaultValue::Any(Value::String("Nice".to_string())));
        let evaluation = evaluate_input_as_string(&input, &HashMap::from([("test".to_string(), DefaultValue::Any(Value::Null))])).unwrap();
        //if any and null, take default
        assert_eq!(evaluation, "Nice".to_string());
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
        fs::copy("../../tests/test_data/file.txt", dir.path().join("tests/test_data/file.txt")).expect("Unable to copy file");
        env::set_current_dir(dir.path()).unwrap();

        let tool = CommandLineTool::default().with_outputs(vec![output]);

        let result = evaluate_command_outputs(&tool, &current.join("../../"));
        assert!(result.is_ok());

        env::set_current_dir(current).unwrap();
    }

    #[test]
    #[serial]
    pub fn test_get_file_metadata() {
        let path = env::current_dir().unwrap().join("../../tests").join("test_data").join("file.txt");
        let result = get_file_metadata(path.clone(), None);
        let expected = File {
            location: Some(format!("file://{}", path.to_string_lossy().into_owned())),
            basename: Some("file.txt".to_string()),
            class: "File".to_string(),
            nameext: Some(".txt".into()),
            nameroot: Some("file".into()),
            checksum: Some("sha1$2c3cafa4db3f3e1e51b3dff4303502dbe42b7a89".to_string()),
            size: Some(4),
            path: Some(path.to_string_lossy().into_owned()),
            ..Default::default()
        };

        assert_eq!(result, expected);
    }

    #[test]
    #[serial]
    pub fn test_get_directory_metadata() {
        let path = env::current_dir().unwrap().join("../../tests/test_data");
        let result = get_diretory_metadata(path.clone());
        let expected = Directory {
            location: Some(format!("file://{}", path.to_string_lossy().into_owned())),
            basename: Some(path.file_name().unwrap().to_string_lossy().into_owned()),
            path: Some(path.to_string_lossy().into_owned()),
            ..Default::default()
        };
        assert_eq!(result, expected);
    }

    #[test]
    #[serial]
    pub fn test_copy_output_dir() {
        let dir = tempdir().unwrap();
        let stage = dir.path().join("tests").join("test_data").join("test_dir");
        let current = env::current_dir().unwrap().join("../../tests").join("test_data").join("test_dir");
        let cwd = current.to_str().unwrap();
        copy_dir(cwd, stage.to_str().unwrap()).unwrap();

        let mut result = copy_output_dir(stage.to_str().unwrap(), cwd).expect("could not copy dir");
        result.listing = result.listing.map(|mut listing| {
            listing.sort_by_key(|item| match item {
                DefaultValue::File(file) => file.basename.clone(),
                _ => Some(String::new()),
            });
            listing
        });

        let file = current.join("file.txt").to_string_lossy().into_owned();
        let input = current.join("input.txt").to_string_lossy().into_owned();

        let expected = Directory {
            location: Some(format!("file://{cwd}")),
            basename: Some("test_dir".to_string()),
            listing: Some(vec![
                DefaultValue::File(File {
                    class: "File".into(),
                    location: Some(format!("file://{file}")),
                    nameroot: Some("file".into()),
                    nameext: Some(".txt".into()),
                    basename: Some("file.txt".into()),
                    checksum: Some("sha1$2c3cafa4db3f3e1e51b3dff4303502dbe42b7a89".to_string()),
                    size: Some(4),
                    path: Some(file),
                    ..Default::default()
                }),
                DefaultValue::File(File {
                    class: "File".to_string(),
                    location: Some(format!("file://{input}")),
                    nameroot: Some("input".into()),
                    nameext: Some(".txt".into()),
                    basename: Some("input.txt".to_string()),
                    checksum: Some("sha1$22959e5335b177539ffcd81a5426b9eca4f4cbec".to_string()),
                    size: Some(26),
                    path: Some(input),
                    ..Default::default()
                }),
            ]),
            path: Some(cwd.to_string()),
            ..Default::default()
        };

        assert_eq!(result, expected);
    }
}
