use crate::cwl::types::CWLType;
use serde_yml::Value;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

pub fn get_filename_without_extension(relative_path: &str) -> Option<String> {
    let path = Path::new(relative_path);

    path.file_name().and_then(|name| {
        name.to_str()
            .map(|s| s.split('.').next().unwrap_or(s).to_string())
    })
}

pub fn create_and_write_file(filename: &str, contents: &str) -> Result<(), io::Error> {
    let mut file = fs::File::create(filename)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

pub fn guess_type(value: &str) -> CWLType {
    let path = Path::new(value);
    if path.exists() {
        if path.is_file() {
            return CWLType::File;
        }
        if path.is_dir() {
            return CWLType::Directory;
        }
    }
    //we do not have to check for files that do not exist yet, as CWLTool would run into a failure
    let yaml_value: Value = serde_yml::from_str(value).unwrap();
    match yaml_value {
        Value::Null => CWLType::Null,
        Value::Bool(_) => CWLType::Boolean,
        Value::Number(number) => {
            if number.is_f64() {
                CWLType::Float
            } else {
                CWLType::Int
            }
        }
        Value::String(_) => CWLType::String,
        _ => CWLType::String,
    }
}
