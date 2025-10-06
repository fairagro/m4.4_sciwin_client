use std::error::Error;
use dialoguer::{Input};
use serde_yaml::{Mapping, Value};
use crate::{
    common::{parse_cwl, annotate, ts_recommendations, write_updated_yaml},
};
use crate::consts::{ARC_SCHEMA, ARC_NAMESPACE, MAX_RECOMMENDATIONS};

#[derive(Debug, Clone)]
pub struct ProcessArgs {
    pub cwl_name: String,
    pub name: String,
    pub input: Option<String>,
    pub output: Option<String>,
    pub parameter: Option<String>,
    pub value: Option<String>,
}

pub async fn annotate_process_step(args: &ProcessArgs) -> Result<(), Box<dyn Error>> {
    // Ensure ARC namespace and schema are defined
    annotate(&args.cwl_name, "$schemas", None, Some(ARC_SCHEMA))?;
    annotate(&args.cwl_name, "$namespaces", Some("arc"), Some(ARC_NAMESPACE))?;
    let mut yaml = parse_cwl(&args.cwl_name)?;
    let mut args = args.clone();

    let param_provided = args.parameter.is_some();
    let value_provided = args.value.is_some();

    if param_provided || value_provided {
        // If only parameter or value is provided, ask for the other
        if !param_provided {
            let param: String = Input::new().with_prompt("Enter process step parameter").interact_text()?;
            if !param.is_empty() {
                args.parameter = Some(param);
            }
        }
        if !value_provided {
            let val: String = Input::new().with_prompt("Enter process step value").interact_text()?;
            if !val.is_empty() {
                args.value = Some(val);
            }
        }
    }
    if let Value::Mapping(ref mut mapping) = yaml {
        let process_seq_key = Value::String("arc:has process sequence".to_string());
        let process_sequences = get_or_init_sequence(mapping, &process_seq_key)?;

        // Find index of process sequence with the same name
        let found_idx = process_sequences.iter().position(|item| {
            if let Value::Mapping(proc_map) = item {
                proc_map.get(Value::String("arc:name".to_string()))
                    == Some(&Value::String(args.name.clone()))
            } else {
                false
            }
        });
        if let Some(idx) = found_idx {
            // Extend the existing process sequence with new input/output/parameter value if provided
            if let Value::Mapping(proc_map) = &mut process_sequences[idx] {
                add_input_output(proc_map, "arc:has input", "arc:data", args.input.as_deref());
                add_input_output(proc_map, "arc:has output", "arc:data", args.output.as_deref());
                // Add parameter value pair
                if args.parameter.is_some() && args.value.is_some() {
                    let param_name = args.parameter.as_deref().unwrap_or("Generic");
                    let param_val = build_parameter_value(param_name, args.value.as_deref()).await?;
                    let key = Value::String("arc:has parameter value".to_string());
                    match proc_map.get_mut(&key) {
                        Some(Value::Sequence(seq)) => seq.push(Value::Mapping(param_val)),
                        _ => {
                            proc_map.insert(key, Value::Sequence(vec![Value::Mapping(param_val)]));
                        }
                    }
                }
            }
        } else {
            // Create a new process sequence mapping
            let mut new_seq = Mapping::new();
            new_seq.insert(Value::String("class".to_string()), Value::String("arc:process sequence".to_string()));
            new_seq.insert(Value::String("arc:name".to_string()), Value::String(args.name.clone()));

            // Add input/output if present
            add_input_output(&mut new_seq, "arc:has input", "arc:data", args.input.as_deref());
            add_input_output(&mut new_seq, "arc:has output", "arc:data", args.output.as_deref());

            // Add parameter value pair if both are present
            if args.parameter.is_some() && args.value.is_some() {
                let param_name = args.parameter.as_deref().unwrap_or("");
                let param_val = build_parameter_value(param_name, args.value.as_deref()).await?;
                new_seq.insert(
                    Value::String("arc:has parameter value".to_string()),
                    Value::Sequence(vec![Value::Mapping(param_val)]),
                );
            }
            process_sequences.push(Value::Mapping(new_seq));
        }
    }
    write_updated_yaml(&args.cwl_name, &yaml)
}

/// Get or initialize a sequence for a given key in a mapping
fn get_or_init_sequence<'a>(mapping: &'a mut Mapping, key: &Value) -> Result<&'a mut Vec<Value>, Box<dyn Error>> {
    if mapping.get_mut(key).is_some() {
        if let Some(Value::Sequence(seq)) = mapping.get_mut(key) {
            return Ok(seq);
        }
        return Err("Key exists but is not a sequence.".into());
    }
    mapping.insert(key.clone(), Value::Sequence(vec![]));
    if let Some(Value::Sequence(seq)) = mapping.get_mut(key) {
        Ok(seq)
    } else {
        Err("Failed to initialize process sequence.".into())
    }
}

/// Add input or output to a mapping if present
fn add_input_output(mapping: &mut Mapping, key: &str, class: &str, value: Option<&str>) {
    if let Some(val) = value {
        let mut m = Mapping::new();
        m.insert(Value::String("class".to_string()), Value::String(class.to_string()));
        m.insert(Value::String("arc:name".to_string()), Value::String(val.to_string()));
        let v = Value::Mapping(m);
        let k = Value::String(key.to_string());
        match mapping.get_mut(&k) {
            Some(Value::Sequence(seq)) => {
                if !seq.iter().any(|x| x == &v) {
                    seq.push(v);
                }
            }
            _ => {
                mapping.insert(k, Value::Sequence(vec![v]));
            }
        }
    }
}

/// Build parameter value mapping, including annotation and value if present
async fn build_parameter_value(parameter: &str, value: Option<&str>) -> Result<Mapping, Box<dyn Error>> {
    let mut parameter_value = Mapping::new();
    parameter_value.insert(Value::String("class".to_string()), Value::String("arc:process parameter value".to_string()));

    let mut protocol_parameter = Mapping::new();
    protocol_parameter.insert(Value::String("class".to_string()), Value::String("arc:protocol parameter".to_string()));

    let mut parameter_name = Mapping::new();
    parameter_name.insert(Value::String("class".to_string()), Value::String("arc:parameter name".to_string()));

    let annotation_value = process_annotation_with_mapping(parameter, parameter_name.clone(), true).await?;
    parameter_name.extend(annotation_value);
    protocol_parameter.insert(
        Value::String("arc:has parameter name".to_string()),
        Value::Sequence(vec![Value::Mapping(parameter_name)]),
    );

    parameter_value.insert(
        Value::String("arc:has parameter".to_string()),
        Value::Sequence(vec![Value::Mapping(protocol_parameter)]),
    );

    if let Some(val) = value {
        let mut value_name = Mapping::new();
        value_name.insert(Value::String("class".to_string()), Value::String("arc:ontology annotation".to_string()));
        let annotation_value = process_annotation_with_mapping(val, value_name.clone(), true).await?;
        value_name.extend(annotation_value);
        parameter_value.insert(Value::String("arc:value".to_string()), Value::Sequence(vec![Value::Mapping(value_name)]));
    }

    Ok(parameter_value)
}


pub async fn process_annotation_with_mapping(value: &str, mut parameter_name: Mapping, complete: bool) -> Result<Mapping, Box<dyn Error>> {
    match ts_recommendations(value, MAX_RECOMMENDATIONS).await {
        Ok((annotation_value, source_ref, term_accession)) => {
            let mut annotation_mapping = Mapping::new();
            annotation_mapping.insert(Value::String("arc:term accession".to_string()), Value::String(term_accession));
            if complete {
                annotation_mapping.insert(Value::String("arc:term source REF".to_string()), Value::String(source_ref));
            }
            annotation_mapping.insert(Value::String("arc:annotation value".to_string()), Value::String(annotation_value));
            parameter_name.extend(annotation_mapping);
        }
        Err(e) => return Err(format!("Failed to process annotation value  {value}: {e}").into()),
    }

    Ok(parameter_name)
}