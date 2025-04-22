use serde_yaml:: Value;
use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::PathBuf,
    io::{self, Read}
};
use serde_yaml::{Mapping};
use std::path::Path;
use std::collections::HashSet;
use std::collections::BTreeSet;

pub fn sanitize_path(path: &str) -> String {
    let path = Path::new(path.trim()); 
    let mut sanitized_path = PathBuf::new();
    
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                sanitized_path.pop();
            },
            _ => {
                sanitized_path.push(comp.as_os_str());
            }
        }
    }
    sanitized_path.to_string_lossy().replace("\\", std::path::MAIN_SEPARATOR_STR)
}

pub fn get_location(base_path: &str, cwl_file_path: &Path) -> Result<String, Box<dyn Error>> {
    let base_path = Path::new(base_path);
    let base_path = base_path.parent().unwrap_or(base_path);
    let mut combined_path = base_path.to_path_buf();
    for component in cwl_file_path.components() {
        match component {
            std::path::Component::Normal(name) => {
                combined_path.push(name);
            }
            std::path::Component::ParentDir => {
                if let Some(parent) = combined_path.parent() {
                    combined_path = parent.to_path_buf();
                }
            }
            _ => {}
        }
    }
    Ok(combined_path.to_string_lossy().to_string())
}

pub fn find_common_directory(paths: &BTreeSet<PathBuf>) -> Result<PathBuf, Box<dyn Error>> {
    let components: Vec<_> = paths
        .iter()
        .map(|p| p.components().collect::<Vec<_>>())
        .collect();

    if components.is_empty() {
        return Err("No paths provided".into());
    }

    let mut common_path = PathBuf::new();
    let first = &components[0];

    for i in 0..first.len() {
        let part = &first[i];
        if components.iter().all(|c| c.len() > i && &c[i] == part) {
            common_path.push(part.as_os_str());
        } else {
            break;
        }
    }

    Ok(common_path)
}

pub fn remove_files_contained_in_directories(
    files: &mut HashSet<String>,
    directories: &HashSet<String>,
) {
    let mut to_remove = Vec::new();

    for file in files.iter() {
        for dir in directories {
            if file.starts_with(dir) {
                to_remove.push(file.clone());
                break;
            }
        }
    }

    for file in to_remove {
        files.remove(&file);
    }
}


pub fn file_matches(requested_file: &str, candidate_path: &str) -> bool {
    Path::new(requested_file)
        .file_name()
        .and_then(|f| f.to_str())
        .map(|file_name| candidate_path.ends_with(file_name))
        .unwrap_or(false)
}

pub fn collect_files_recursive(dir: &Path, files: &mut HashSet<String>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_path = entry.path();

        if file_path.is_dir() {
            collect_files_recursive(&file_path, files)?;
        } else if file_path.is_file() {
            if let Some(file_str) = file_path.to_str() {
                files.insert(file_str.to_string());
            }
        }
    }
    Ok(())
}

pub fn load_cwl_file2(base_path: &str, cwl_file_path: &Path) -> Result<Value, Box<dyn Error>> {
    let full_path = if cwl_file_path.is_absolute() {
        cwl_file_path.to_path_buf()
    } else {
        Path::new(base_path).join(cwl_file_path)
    };

    let contents = fs::read_to_string(full_path)?;
    let yaml: Value = serde_yaml::from_str(&contents)?;
    Ok(yaml)
}

pub fn load_yaml_file(path: &Path) -> Result<Value, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let yaml: Value = serde_yaml::from_str(&contents)?;
    Ok(yaml)
}

pub fn load_cwl_file(base_path: &str, cwl_file_path: &Path) -> Result<Value, Box<dyn Error>> {
    let base_path = Path::new(base_path);
    let base_path = base_path.parent().unwrap_or(base_path);

    let mut combined_path = base_path.to_path_buf();

    for component in cwl_file_path.components() {
        match component {
            std::path::Component::Normal(name) => {
                combined_path.push(name); 
            }
            std::path::Component::ParentDir => {
                if let Some(parent) = combined_path.parent() {
                    combined_path = parent.to_path_buf();
                }
            }
            _ => {}
        }
    }
    if !combined_path.exists() {
        return Err(format!("CWL file not found: {}", combined_path.display()).into());
    }
    let mut file_content = String::new();
    let mut file = std::fs::File::open(&combined_path)?;
    file.read_to_string(&mut file_content)?;
    let cwl: Value = serde_yaml::from_str(&file_content)?;
    Ok(cwl)
}


pub fn read_file_content(file_path: &str) -> Result<String, io::Error> {
    let mut file = std::fs::File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}


pub fn build_inputs_yaml(cwl_input_path: &str, input_yaml_path: &str) -> Result<Mapping, Box<dyn Error>> {
    let input_yaml = fs::read_to_string(input_yaml_path)?;
    let input_value: Value = serde_yaml::from_str(&input_yaml)?;

    let cwl_content = fs::read_to_string(cwl_input_path)?;
    let cwl_value: Value = serde_yaml::from_str(&cwl_content)?;

    let mut files: HashSet<String> = HashSet::new();
    let mut directories: HashSet<String> = HashSet::new();
    
    let mut parameters: HashMap<String, Value> = HashMap::new();

    let main_cwl_path = Path::new(cwl_input_path);
    let main_dir = main_cwl_path.parent().unwrap_or_else(|| Path::new("."));
    let mut referenced_paths: HashSet<PathBuf> = HashSet::new();
    if let Value::Mapping(mapping) = input_value {
        for (key, value) in mapping {
            if let Value::String(key_str) = key {
                if let Value::Mapping(mut sub_mapping) = value.clone() {
                    let class = sub_mapping
                        .get(Value::String("class".to_string()))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
    
                    let location = sub_mapping
                        .get(Value::String("location".to_string()))
                        .or_else(|| sub_mapping.get(Value::String("path".to_string())))
                        .and_then(|v| v.as_str());
    
                    if let (Some(class), Some(location)) = (class, location) {
                        let sanitized_location = sanitize_path(location);
    
                        if sub_mapping.contains_key(Value::String("location".to_string())) {
                            sub_mapping.insert(
                                Value::String("location".to_string()),
                                Value::String(sanitized_location.clone()),
                            );
                        } else if sub_mapping.contains_key(Value::String("path".to_string())) {
                            sub_mapping.insert(
                                Value::String("path".to_string()),
                                Value::String(sanitized_location.clone()),
                            );
                        }
    
                        match class.as_str() {
                            "File" => {
                                files.insert(sanitized_location);
                                parameters.insert(key_str, Value::Mapping(sub_mapping));
                            },
                            "Directory" => {
                                directories.insert(sanitized_location);
                                parameters.insert(key_str, Value::Mapping(sub_mapping));
                            },
                            _ => {}
                        }
                    } else {
                        parameters.insert(key_str, Value::Mapping(sub_mapping));
                    }
                } else {
                    parameters.insert(key_str, value);
                }
            }
        }
    }
    if let Some(steps) = cwl_value.get("steps").and_then(|v| v.as_sequence()) {
        for step in steps {
            if let Some(run_path_str) = step.get("run").and_then(|v| v.as_str()) {
                let full_path = main_dir.join(run_path_str).canonicalize()?;
                referenced_paths.insert(full_path);
            }
        }
    }
    referenced_paths.insert(fs::canonicalize(main_cwl_path)?);

    if !referenced_paths.is_empty() {
        let common_root = find_common_directory(&referenced_paths.iter().cloned().collect::<BTreeSet<_>>())?;
        let relative_root = pathdiff::diff_paths(&common_root, std::env::current_dir()?)
            .unwrap_or(common_root.clone());
    
        let relative_str = relative_root.to_string_lossy().to_string();
        if !relative_str.is_empty() {
            directories.insert(relative_str);
        }
        else {
            let current_dir = std::env::current_dir()?;
            for entry in fs::read_dir(&current_dir)? {
                let entry = entry?;
                let path = entry.path();
        
                if path.is_dir() {
                    if let Some(str_path) = path.strip_prefix(&current_dir).ok().and_then(|p| p.to_str()) {
                        directories.insert(str_path.to_string());
                    }
                } else if path.is_file() {
                    if let Some(str_path) = path.strip_prefix(&current_dir).ok().and_then(|p| p.to_str()) {
                        files.insert(str_path.to_string());
                    }
                }
            }
        }
    }

    remove_files_contained_in_directories(&mut files, &directories);
    let mut inputs_mapping = Mapping::new();
    inputs_mapping.insert(
        Value::String("files".to_string()),
        Value::Sequence(files.into_iter().map(Value::String).collect()),
    );
    inputs_mapping.insert(
        Value::String("directories".to_string()),
        Value::Sequence(directories.into_iter().map(Value::String).collect()),
    );
    inputs_mapping.insert(
        Value::String("parameters".to_string()),
        Value::Mapping(
            parameters
                .into_iter()
                .map(|(k, v)| (Value::String(k), v))
                .collect(),
        ),
    );

    Ok(inputs_mapping)
}


pub fn build_inputs_cwl(cwl_input_path: &str, inputs_yaml: Option<&String>) -> Result<Mapping, Box<dyn Error>> {
    let cwl_content = fs::read_to_string(cwl_input_path)?;
    let cwl_value: Value = serde_yaml::from_str(&cwl_content)?;

    let mut files: HashSet<String> = HashSet::new();
    let mut directories: HashSet<String> = HashSet::new();
    let mut parameters: HashMap<String, Value> = HashMap::new();
    let mut referenced_paths: HashSet<PathBuf> = HashSet::new();

    let main_cwl_path = Path::new(cwl_input_path);
    let main_dir = main_cwl_path.parent().unwrap_or_else(|| Path::new("."));
    if let Some(inputs) = cwl_value.get("inputs").and_then(|v| v.as_sequence()) {
        for input in inputs {
            if let Some(id) = input.get("id").and_then(|v| v.as_str()) {
                if let Some(input_type_val) = input.get("type") {
                    let input_type = input_type_val
                        .as_str()
                        .unwrap_or_else(|| input_type_val.get("type").and_then(|t| t.as_str()).unwrap_or(""));
    
                    if input_type == "File" || input_type == "Directory" {
                        if let Some(default) = input.get("default") {
                            if let Value::Mapping(default_map) = default {
                                let mut sanitized_map = default_map.clone();
                                
                                if let Some(location_val) = sanitized_map.get_mut(Value::String("location".to_string())) {
                                    if let Some(location) = location_val.as_str() {
                                        let sanitized_location = sanitize_path(location);
                                        *location_val = Value::String(sanitized_location.clone());
    
                                        match input_type {
                                            "File" => {
                                                files.insert(sanitized_location);
                                            }
                                            "Directory" => {
                                                directories.insert(sanitized_location);
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                parameters.insert(id.to_string(), Value::Mapping(sanitized_map));
                            } else {
                                parameters.insert(id.to_string(), default.clone());
                            }
                        } else {
                            let location = find_input_location(cwl_input_path, id)?;
                            if let Some(location) = location {
                                let sanitized_location = sanitize_path(&location);
                                match input_type {
                                    "File" => files.insert(sanitized_location.clone()),
                                    "Directory" => directories.insert(sanitized_location.clone()),
                                    _ => None::<Value>.is_some(),
                                };
    
                                let mut param_map = Mapping::new();
                                param_map.insert(Value::String("class".to_string()), Value::String(input_type.to_string()));
                                if input_type == "Directory" {
                                    param_map.insert(Value::String("location".to_string()), Value::String(sanitized_location));
                                } else {
                                    param_map.insert(Value::String("path".to_string()), Value::String(sanitized_location));
                                }
                                parameters.insert(id.to_string(), Value::Mapping(param_map));
                            }
                        }
                    } else if let Some(default) = input.get("default") {
                        parameters.insert(id.to_string(), default.clone());
                    }
                }
            }
        }
    }

    if let Some(steps) = cwl_value.get("steps").and_then(|v| v.as_sequence()) {
        for step in steps {
            if let Some(run_path_str) = step.get("run").and_then(|v| v.as_str()) {
                let full_path = main_dir.join(run_path_str).canonicalize()?;
                referenced_paths.insert(full_path);
            }
        }
    }

    referenced_paths.insert(fs::canonicalize(main_cwl_path)?);

    if !referenced_paths.is_empty() {
        let common_root = find_common_directory(&referenced_paths.iter().cloned().collect::<BTreeSet<_>>())?;
        let relative_root = pathdiff::diff_paths(&common_root, std::env::current_dir()?)
            .unwrap_or(common_root.clone());
    
        let relative_str = relative_root.to_string_lossy().to_string();
        if !relative_str.is_empty() {
            directories.insert(relative_str);
        }
    }
    if directories.is_empty() {
        let current_dir = std::env::current_dir()?;
        for entry in fs::read_dir(&current_dir)? {
            let entry = entry?;
            let path = entry.path();
    
            if path.is_dir() {
                if let Some(str_path) = path.strip_prefix(&current_dir).ok().and_then(|p| p.to_str()) {
                    directories.insert(str_path.to_string());
                }
            } else if path.is_file() {
                if let Some(str_path) = path.strip_prefix(&current_dir).ok().and_then(|p| p.to_str()) {
                    files.insert(str_path.to_string());
                }
            }
        }
    }

    if let Some(yaml_path) = inputs_yaml {
        parameters.insert("inputs.yaml".to_string(), Value::String(yaml_path.to_string()));
    }

    let mut inputs_mapping = Mapping::new();
    remove_files_contained_in_directories(&mut files, &directories);

    inputs_mapping.insert(
        Value::String("files".to_string()),
        Value::Sequence(files.into_iter().map(Value::String).collect()),
    );

    inputs_mapping.insert(
        Value::String("directories".to_string()),
        Value::Sequence(directories.into_iter().map(Value::String).collect()),
    );

    let mut parameter_mapping = serde_yaml::Mapping::new();

    for (key, value) in parameters {
        if let Some(class) = value.get("class") {
            let mut param_map = Mapping::new();
            if let Some(class_str) = class.as_str() {
                param_map.insert(Value::String("class".to_string()), Value::String(class_str.to_string()));
            }
            if let Some(location) = value.get("location") {
                param_map.insert(Value::String("location".to_string()), location.clone());
            }
            if let Some(path) = value.get("path") {
                param_map.insert(Value::String("path".to_string()), path.clone());
            }
            parameter_mapping.insert(Value::String(key), Value::Mapping(param_map));
        } else {
            parameter_mapping.insert(Value::String(key), value);
        }
    }
inputs_mapping.insert(
    Value::String("parameters".to_string()),
    Value::Mapping(parameter_mapping),
);

    Ok(inputs_mapping)
}

pub fn get_all_outputs(main_workflow_path: &str) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let main_yaml_str = fs::read_to_string(main_workflow_path)?;
    let main_yaml: Value = serde_yaml::from_str(&main_yaml_str)?;

    let outputs_section = main_yaml.get("outputs")
        .ok_or("No 'outputs' section in main workflow")?
        .as_sequence()
        .ok_or("'outputs' section is not a sequence")?;
    
    let steps_section = main_yaml.get("steps")
        .ok_or("No 'steps' section in main workflow")?
        .as_sequence()
        .ok_or("'steps' section is not a sequence")?;
    
    let mut results = Vec::new();
    for output in outputs_section {
        let output_source = output.get("outputSource")
            .and_then(|v| v.as_str())
            .ok_or("Output missing 'outputSource' field or not a string")?;
        
        let parts: Vec<&str> = output_source.split('/').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid outputSource format for output: {}", output_source).into());
        }
        let step_id = parts[0];
        let output_id = parts[1];

        let mut run_file_path = None;
        for step in steps_section {
            if let Some(id) = step.get("id").and_then(|v| v.as_str()) {
                if id == step_id {
                    run_file_path = step.get("run")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    break;
                }
            }
        }
        let run_file_path = run_file_path.ok_or(format!("Step with id {} not found or missing 'run'", step_id))?;
        let main_workflow_path = std::path::Path::new(main_workflow_path);
        let main_workflow_dir = main_workflow_path
            .parent()
            .ok_or("Failed to get parent directory of main workflow file")?;
        let full_run_file_path = main_workflow_dir.join(&run_file_path).canonicalize()?;
        let tool_yaml_str = fs::read_to_string(&full_run_file_path)?;
        let tool_yaml: Value = serde_yaml::from_str(&tool_yaml_str)?;
        let tool_outputs = tool_yaml.get("outputs")
            .ok_or(format!("No 'outputs' section in tool file {}", run_file_path))?
            .as_sequence()
            .ok_or(format!("'outputs' section in tool file {} is not a sequence", run_file_path))?;
        let mut glob_value = None;
        for tool_output in tool_outputs {
            if let Some(tid) = tool_output.get("id").and_then(|v| v.as_str()) {
                if tid == output_id {
                    if let Some(binding) = tool_output.get("outputBinding") {
                        if let Some(glob) = binding.get("glob").and_then(|v| v.as_str()) {
                            glob_value = Some(glob.to_string());
                            break;
                        }
                    }
                }
            }
        }
        let glob_value = glob_value.ok_or(format!("Output {} not found in tool file {} or missing glob", output_id, run_file_path))?;
        
        results.push((output_id.to_string(), glob_value));
    }
    Ok(results)
}

pub fn find_input_location(cwl_file_path: &str, id: &str) -> Result<Option<String>, Box<dyn Error>> {
    let mut main_file = std::fs::File::open(cwl_file_path)?;
    let mut main_file_content = String::new();
    main_file.read_to_string(&mut main_file_content)?;

    let main_cwl: Value = serde_yaml::from_str(&main_file_content)?;

    if let Some(steps) = main_cwl["steps"].as_sequence() {
        for step in steps {
            if let Some(inputs) = step["in"].as_mapping() {
                if inputs.contains_key(id) {
                    if let Some(run) = step["run"].as_str() {
                        let run_path = Path::new(run);
                        let run_file = load_cwl_file(cwl_file_path, run_path)?;
                        if let Some(inputs_section) = run_file["inputs"].as_sequence() {
                            for input in inputs_section {
                                if let Some(input_id) = input["id"].as_str() {
                                    if input_id == id {
                                        if let Some(default) = input["default"].as_mapping() {
                                            if let Some(location) = default.get("location").and_then(|v| v.as_str()) {
                                                return Ok(Some(location.to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

pub fn resolve_input_file_path(
    requested_file: &str,
    input_yaml: Option<&Value>,
    cwl_value: Option<&Value>,
) -> Option<String> {
    let requested_path = Path::new(requested_file);
    if requested_path.exists() {
        return Some(requested_file.to_string());
    }

    // Search in input_yaml
    if let Some(Value::Mapping(mapping)) = input_yaml {
        for (_key, value) in mapping {
            if let Value::Mapping(file_entry) = value {
                for field in &["location", "path"] {
                    if let Some(Value::String(path_str)) = file_entry.get(&Value::String(field.to_string())) {
                        if file_matches(requested_file, path_str) {
                            return Some(path_str.to_string());
                        }
                    }
                }
            }
        }
    }

    // Search in cwl inputs
    if let Some(cwl) = cwl_value {
        if let Some(inputs) = cwl.get("inputs").and_then(|v| v.as_sequence()) {
            for input in inputs {
                if let Some(Value::Mapping(default_map)) = input.get("default") {
                    for field in &["location", "path"] {
                        if let Some(Value::String(loc)) = default_map.get(&Value::String(field.to_string())) {
                            if file_matches(requested_file, loc) {
                                return Some(loc.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
