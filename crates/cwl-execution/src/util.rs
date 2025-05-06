use crate::{
    expression::{evaluate_expression, set_self, unset_self},
    io::{copy_dir, copy_file, get_first_file_with_prefix},
};
use anyhow::{bail, Context};
use cwl::{
    clt::CommandLineTool,
    et::ExpressionTool,
    inputs::CommandInputParameter,
    outputs::CommandOutputParameter,
    types::{CWLType, DefaultValue, Directory, File},
};
use glob::glob;
use serde_yaml::Value;
use std::{
    collections::HashMap,
    env,
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

///Either gets the default value for input or the provided one (preferred)
pub(crate) fn evaluate_input_as_string(input: &CommandInputParameter, input_values: &HashMap<String, DefaultValue>) -> anyhow::Result<String> {
    Ok(evaluate_input(input, input_values)?.as_value_string())
}

///Either gets the default value for input or the provided one (preferred)
pub(crate) fn evaluate_input(input: &CommandInputParameter, input_values: &HashMap<String, DefaultValue>) -> anyhow::Result<DefaultValue> {
    if let Some(value) = input_values.get(&input.id) {
        if (matches!(input.type_, CWLType::Any) || input.type_.is_optional()) && matches!(value, DefaultValue::Any(Value::Null)) {
            if let Some(default_) = &input.default {
                return Ok(default_.clone());
            }
        }

        if value.has_matching_type(&input.type_) {
            return Ok(value.clone());
        } else {
            bail!("CWLType '{:?}' is not matching input type. Input was: \n{:#?}", &input.type_, value);
        }
    } else if let Some(default_) = &input.default {
        return Ok(default_.clone());
    }

    if let CWLType::Optional(_) = input.type_ {
        Ok(DefaultValue::Any(Value::Null))
    } else {
        bail!("You did not include a value for {}", input.id);
    }
}

pub(crate) fn evaluate_expression_outputs(tool: &ExpressionTool, value: Value) -> anyhow::Result<HashMap<String, DefaultValue>> {
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
pub(crate) fn evaluate_command_outputs(tool: &CommandLineTool, initial_dir: &Path) -> anyhow::Result<HashMap<String, DefaultValue>> {
    //check for cwl.output.json
    // If the output directory contains a file named "cwl.output.json", that file must be loaded and used as the output object.
    let check = Path::new("cwl.output.json");
    if check.exists() {
        let contents = fs::read_to_string(check)?;
        let mut values: HashMap<String, DefaultValue> = serde_json::from_str(&contents)?;
        values.retain(|k, _| tool.outputs.iter().any(|o| o.id == *k));
        for (_, value) in values.iter_mut() {
            match value {
                DefaultValue::File(file) => {
                    if let Some(path) = &file.location {
                        let path = path.strip_prefix("file://").unwrap_or(path);
                        let path = PathBuf::from(path);
                        let path = &pathdiff::diff_paths(&path, env::current_dir()?).unwrap_or(path);
                        let dest = &initial_dir.join(path);
                        fs::copy(path, dest)?;
                        eprintln!("📜 Wrote output file: {:?}", &initial_dir.join(dest));
                        file.location = Some(dest.to_string_lossy().into_owned());
                        *file = file.snapshot();
                    }
                }
                DefaultValue::Directory(dir) => {
                    if let Some(path) = &dir.location {
                        let path = PathBuf::from(path);
                        let path = &pathdiff::diff_paths(&path, env::current_dir()?).unwrap_or(path);
                        let dest = &initial_dir.join(path);
                        copy_dir(path, dest)?;
                        eprintln!("📜 Wrote output directory: {:?}", &dest);
                    }
                }
                _ => (),
            }
        }
        return Ok(values);
    }

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
) -> anyhow::Result<()> {
    match type_ {
        CWLType::File | CWLType::Stdout | CWLType::Stderr => {
            if let Some(binding) = &output.output_binding {
                let mut result = glob(&binding.glob)?;
                if let Some(entry) = result.next() {
                    let entry = &entry?;
                    outputs.insert(output.id.clone(), handle_file_output(entry, initial_dir, output)?);
                } else {
                    bail!("Could not evaluate glob: {}", binding.glob);
                }
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
                fs::copy(filename, path).with_context(|| format!("Failed to copy file from {:?} to {:?}", &filename, path))?;
                eprintln!("📜 Wrote output file: {:?}", path);
                outputs.insert(output.id.clone(), DefaultValue::File(get_file_metadata(path, output.format.clone())));
            }
        }
        CWLType::Array(inner) if matches!(&**inner, CWLType::File) || matches!(&**inner, CWLType::Directory) => {
            if let Some(binding) = &output.output_binding {
                let result = glob(&binding.glob)?;
                let values: anyhow::Result<Vec<_>> = result
                    .map(|entry| {
                        let entry = entry?;
                        match **inner {
                            CWLType::File => handle_file_output(&entry, initial_dir, output),
                            CWLType::Directory => handle_dir_output(&entry, initial_dir),
                            _ => unreachable!(),
                        }
                    })
                    .collect();
                outputs.insert(output.id.clone(), DefaultValue::Array(values?));
            }
        }
        CWLType::Directory => {
            if let Some(binding) = &output.output_binding {
                let mut result = glob(&binding.glob)?;
                if let Some(entry) = result.next() {
                    let entry = &entry?;
                    outputs.insert(output.id.clone(), handle_dir_output(entry, initial_dir)?);
                } else {
                    bail!("Could not evaluate glob: {}", binding.glob);
                }
            }
        }
        _ => {
            //string and has binding -> read file
            if let Some(binding) = &output.output_binding {
                let contents = fs::read_to_string(&binding.glob)?;

                if let Some(expression) = &binding.output_eval {
                    let mut ctx = File::from_location(&binding.glob);
                    ctx.format = output.format.clone();
                    let mut ctx = ctx.snapshot();
                    ctx.contents = Some(contents);
                    set_self(&vec![&ctx])?;
                    let result = evaluate_expression(expression)?;
                    let value = serde_yaml::from_str(&serde_json::to_string(&result)?)?;
                    outputs.insert(output.id.clone(), DefaultValue::Any(value));
                    unset_self()?;
                } else {
                    outputs.insert(output.id.clone(), DefaultValue::Any(Value::String(contents)));
                }
            }
        }
    }
    Ok(())
}

fn handle_file_output(entry: &PathBuf, initial_dir: &Path, output: &CommandOutputParameter) -> anyhow::Result<DefaultValue> {
    let path = &initial_dir.join(entry);
    fs::copy(entry, path).with_context(|| format!("Failed to copy file from {:?} to {:?}", entry, path))?;
    eprintln!("📜 Wrote output file: {:?}", path);
    Ok(DefaultValue::File(get_file_metadata(path, output.format.clone())))
}

fn handle_dir_output(entry: &PathBuf, initial_dir: &Path) -> anyhow::Result<DefaultValue> {
    let path = &initial_dir.join(entry);
    fs::create_dir_all(path)?;
    let out_dir = copy_output_dir(entry, path).with_context(|| format!("Failed to copy: {:?} to {:?}", entry, path))?;
    Ok(DefaultValue::Directory(out_dir))
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

pub fn preprocess_cwl<P: AsRef<Path>>(contents: &str, path: P) -> anyhow::Result<String> {
    let mut yaml: Value = serde_yaml::from_str(contents)?;
    let path = path.as_ref().parent().unwrap_or_else(|| Path::new("."));
    resolve_imports(&mut yaml, path)?;

    Ok(serde_yaml::to_string(&yaml)?)
}

fn resolve_imports(value: &mut Value, base_path: &Path) -> anyhow::Result<()> {
    match value {
        Value::Mapping(map) => {
            if map.len() == 1 {
                if let Some(Value::String(file)) = map.get(Value::String("$import".to_string())) {
                    let path = base_path.join(file);
                    let contents = fs::read_to_string(&path)?;
                    let mut imported_value: Value = serde_yaml::from_str(&contents)?;
                    resolve_imports(&mut imported_value, path.parent().unwrap_or(base_path))?;
                    *value = imported_value;
                    return Ok(());
                }
            }
            for val in map.values_mut() {
                resolve_imports(val, base_path)?;
            }
        }
        Value::Sequence(seq) => {
            for val in seq.iter_mut() {
                resolve_imports(val, base_path)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn is_docker_installed() -> bool {
    let output = Command::new("docker").arg("--version").output();

    matches!(output, Ok(output) if output.status.success())
}

#[cfg(test)]
mod tests {
    use std::env;

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
                ..Default::default()
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
