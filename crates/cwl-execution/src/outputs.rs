use crate::{
    expression::{output_eval, replace_expressions, set_self, unset_self},
    io::{copy_dir, copy_file, get_first_file_with_prefix},
};
use commonwl::{CWLType, CommandLineTool, DefaultValue, Directory, ExpressionTool, File, outputs::CommandOutputParameter};
use glob::glob;
use log::info;
use serde_yaml::Value;
use std::{
    collections::HashMap,
    env,
    error::Error,
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn evaluate_expression_outputs(tool: &ExpressionTool, value: &Value) -> Result<HashMap<String, DefaultValue>, Box<dyn Error>> {
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
    //check for cwl.output.json
    // If the output directory contains a file named "cwl.output.json", that file must be loaded and used as the output object.
    let check = Path::new("cwl.output.json");
    if check.exists() {
        let contents = fs::read_to_string(check)?;
        let mut values: HashMap<String, DefaultValue> = serde_json::from_str(&contents)?;
        values.retain(|k, _| tool.outputs.iter().any(|o| o.id == *k));
        for value in values.values_mut() {
            match value {
                DefaultValue::File(file) => {
                    if let Some(path) = &file.location {
                        let path = path.strip_prefix("file://").unwrap_or(path);
                        let path = PathBuf::from(path);
                        let path = &pathdiff::diff_paths(&path, env::current_dir()?).unwrap_or(path);
                        let dest = &initial_dir.join(path);
                        fs::copy(path, dest)?;
                        info!("📜 Wrote output file: {:?}", &initial_dir.join(dest));
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
                        info!("📜 Wrote output directory: {:?}", &dest);
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
) -> Result<(), Box<dyn Error>> {
    match type_ {
        CWLType::File | CWLType::Stdout | CWLType::Stderr => {
            if let Some(binding) = &output.output_binding {
                if let Some(glob_) = &binding.glob {
                    let mut result = glob(glob_)?;
                    if let Some(entry) = result.next() {
                        let entry = &entry?;
                        outputs.insert(output.id.clone(), handle_file_output(entry, initial_dir, output)?);
                    } else {
                        Err(format!("Could not evaluate glob: {glob_}"))?;
                    }
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
                fs::copy(filename, path).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", &filename, path, e))?;
                eprintln!("📜 Wrote output file: {path:?}");
                outputs.insert(output.id.clone(), DefaultValue::File(get_file_metadata(path, output.format.clone())));
            }
        }
        CWLType::Array(inner) if matches!(&**inner, CWLType::File) || matches!(&**inner, CWLType::Directory) => {
            if let Some(binding) = &output.output_binding
                && let Some(glob_) = &binding.glob
            {
                let result = glob(glob_)?;
                let values: Result<Vec<_>, Box<dyn Error>> = result
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
            if let Some(binding) = &output.output_binding
                && let Some(glob_) = &binding.glob
            {
                let mut result = glob(glob_)?;
                if let Some(entry) = result.next() {
                    let entry = &entry?;
                    outputs.insert(output.id.clone(), handle_dir_output(entry, initial_dir)?);
                } else {
                    Err(format!("Could not evaluate glob: {glob_}"))?;
                }
            }
        }
        _ => {
            //string and has binding -> read file
            if let Some(binding) = &output.output_binding {
                if let Some(glob_) = &binding.glob {
                    let contents = fs::read_to_string(glob_)?;

                    let value = if let Some(expression) = &binding.output_eval {
                        let mut ctx = File::from_location(glob_);
                        ctx.format.clone_from(&output.format);
                        let mut ctx = ctx.snapshot();
                        ctx.contents = Some(contents);
                        set_self(&vec![&ctx])?;
                        let result = output_eval(expression)?;
                        let value = serde_yaml::from_str(&serde_json::to_string(&result)?)?;
                        unset_self()?;
                        DefaultValue::Any(value)
                    } else {
                        DefaultValue::Any(Value::String(contents))
                    };
                    outputs.insert(output.id.clone(), value);
                } else if let Some(expression) = &binding.output_eval {
                    let result = output_eval(expression)?;
                    let value = serde_yaml::from_str(&serde_json::to_string(&result)?)?;
                    outputs.insert(output.id.clone(), DefaultValue::Any(value));
                }
            }
        }
    }
    Ok(())
}

fn handle_file_output(entry: &PathBuf, initial_dir: &Path, output: &CommandOutputParameter) -> Result<DefaultValue, Box<dyn Error>> {
    let current_dir = env::current_dir()?.to_string_lossy().into_owned();
    let path = &initial_dir.join(entry.strip_prefix(&current_dir).unwrap_or(entry));
    fs::copy(entry, path).map_err(|e| format!("Failed to copy file from {entry:?} to {path:?}: {e}"))?;
    info!("📜 Wrote output file: {path:?}");

    let mut file = get_file_metadata(path, output.format.clone());
    if !output.secondary_files.is_empty() {
        set_self(&file)?;
        let folder = entry.parent().unwrap_or(Path::new(""));
        let mut secondary_files = vec![];
        for secondary in &output.secondary_files {
            let pattern = replace_expressions(&secondary.pattern)?;
            let pattern = format!("{}/*{}", folder.to_string_lossy(), pattern);
            for entry in glob(&pattern)? {
                let entry = entry?;
                let sec_path = initial_dir.join(entry.strip_prefix(&current_dir).unwrap_or(&entry));
                fs::copy(&entry, &sec_path).map_err(|e| format!("Failed to copy file from {entry:?} to {sec_path:?}: {e}"))?;
                info!("📜 Wrote secondary file: {sec_path:?}");
                secondary_files.push(DefaultValue::File(get_file_metadata(&sec_path, None)));
            }
        }
        file.secondary_files = Some(secondary_files);
        unset_self()?;
    }

    Ok(DefaultValue::File(file))
}

fn handle_dir_output(entry: &PathBuf, initial_dir: &Path) -> Result<DefaultValue, Box<dyn Error>> {
    let current_dir = env::temp_dir().to_string_lossy().into_owned();
    let path = &initial_dir.join(entry.strip_prefix(current_dir).unwrap_or(entry));
    fs::create_dir_all(path)?;
    let out_dir = copy_output_dir(entry, path).map_err(|e| format!("Failed to copy: {e}"))?;
    Ok(DefaultValue::Directory(out_dir))
}

pub(crate) fn get_file_metadata<P: AsRef<Path> + Debug>(path: P, format: Option<String>) -> File {
    let mut f = File::from_location(&path.as_ref().to_string_lossy());
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
    dir.listing = Some(vec![]);

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.as_ref().join(entry.file_name());
        let entry = if src_path.is_dir() {
            let sub_dir = copy_output_dir(src_path, dest_path)?;
            DefaultValue::Directory(sub_dir)
        } else {
            copy_file(src_path, &dest_path)?;
            DefaultValue::File(get_file_metadata(dest_path, None))
        };

        if let Some(ref mut listing) = dir.listing {
            listing.push(entry);
        }
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;
    use crate::io::copy_dir;
    use commonwl::outputs::{CommandOutputBinding, CommandOutputParameter};
    use serial_test::serial;
    use tempfile::tempdir;

    #[test]
    #[serial]
    pub fn test_evaluate_outputs() {
        let dir = tempdir().unwrap();
        let current = env::current_dir().unwrap();

        let output = CommandOutputParameter::default()
            .with_id("out")
            .with_type(CWLType::File)
            .with_binding(CommandOutputBinding {
                glob: Some("tests/test_data/file.txt".to_string()),
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
        assert!(path.exists());

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
        if let Some(ref mut listing) = result.listing {
            listing.sort_by_key(|item| match item {
                DefaultValue::File(file) => file.basename.clone(),
                _ => Some(String::new()),
            });
        }
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
