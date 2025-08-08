use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
};
use serde_yaml::{Mapping, Value};
use crate::common::{write_updated_yaml,parse_cwl, annotate};
use crate::consts::{ARC_SCHEMA, ARC_NAMESPACE};

pub fn annotate_container(cwl_name: &str, container_value: &str) -> Result<(), Box<dyn Error>> {
    // Ensure ARC schema and namespace annotations
    annotate(cwl_name, "$schemas", None, Some(ARC_SCHEMA))?;
    annotate(cwl_name, "$namespaces", Some("arc"), Some(ARC_NAMESPACE))?;

    // Prepare container annotation block
    let mut container_info = Mapping::new();
    container_info.insert(Value::String("class".into()), Value::String("arc:technology type".into()));
    container_info.insert(Value::String("arc:annotation value".into()), Value::String(container_value.into()));

    let mut yaml = parse_cwl(cwl_name)?;

    // Safely update or insert `arc:has technology type`
    let mapping = yaml.as_mapping_mut().ok_or("Root YAML is not a mapping")?;
    let key = Value::String("arc:has technology type".into());

    match mapping.get_mut(&key) {
        Some(Value::Sequence(seq)) => {
            let already_exists = seq.iter().any(|item| item == &Value::Mapping(container_info.clone()));
            if !already_exists {
                seq.push(Value::Mapping(container_info));
            }
        }
        _ => {
            mapping.insert(key, Value::Sequence(vec![Value::Mapping(container_info)]));
        }
    }
    write_updated_yaml(cwl_name, &yaml)
}

pub fn contains_docker_requirement(file_path: &str) -> Result<bool, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        if line?.contains("DockerRequirement") {
            return Ok(true);
        }
    }

    Ok(false)
}