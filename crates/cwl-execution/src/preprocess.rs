use serde_yaml::{Mapping, Value};
use std::{error::Error, fs, path::Path};

pub fn preprocess_cwl<P: AsRef<Path>>(contents: &str, path: P) -> Result<String, Box<dyn Error>> {
    let mut yaml: Value = serde_yaml::from_str(contents)?;
    let path = path.as_ref().parent().unwrap_or_else(|| Path::new("."));
    resolve_imports(&mut yaml, path)?;
    resolve_shortcuts(&mut yaml);
    Ok(serde_yaml::to_string(&yaml)?)
}

fn resolve_imports(value: &mut Value, base_path: &Path) -> Result<(), Box<dyn Error>> {
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

fn resolve_shortcuts(value: &mut Value) {
    //get inputs block
    let mut stdin_id: Option<String> = None;
    if let Value::Mapping(cwl) = value {
        let inputs = cwl.get_mut("inputs").unwrap(); //block is mandatory!
        if let Value::Mapping(map) = inputs {
            for (id, map_val) in map {
                //if shortcut of shortcut expand first time
                if map_val == &Value::String("stdin".to_string()) {
                    let mut mapping = Mapping::new();
                    mapping.insert(Value::String("type".to_string()), Value::String("stdin".to_string()));
                    *map_val = Value::Mapping(mapping);
                }
                if let Value::Mapping(map_map) = map_val {
                    process_stdin_input(map_map, id, &mut stdin_id);
                }
            }
        } else if let Value::Sequence(seq) = inputs {
            for item in seq {
                if let Value::Mapping(map) = item {
                    let id_val = map.get("id").cloned().unwrap();
                    process_stdin_input(map, &id_val, &mut stdin_id);
                }
            }
        }

        if let Some(stdin_id) = stdin_id {
            cwl.insert(Value::String("stdin".to_string()), Value::String(format!("$(inputs.{stdin_id}.path)")));
        }
    }
}

fn process_stdin_input(map: &mut Mapping, id: &Value, stdin_id: &mut Option<String>) {
    if let Some(Value::String(type_str)) = map.get_mut(Value::String("type".to_string())) {
        if type_str == "stdin" {
            *type_str = "File".to_string();
            map.insert(Value::String("streamable".to_string()), Value::Bool(true));
            if let Value::String(id_str) = id {
                *stdin_id = Some(id_str.clone());
            }
        }
    }
}
