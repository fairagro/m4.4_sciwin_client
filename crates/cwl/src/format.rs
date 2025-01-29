use serde_yaml::{Mapping, Value};
use std::{collections::HashMap, error::Error};

const HASH_BANG: &str = "#!/usr/bin/env cwl-runner\n\n";
const HASH_BANG_PRE: &str = "#!/usr/bin/env ";
const KEYS_WITH_NEWLINES: [&str; 7] = ["inputs", "outputs", "steps", "requirements", "hints", "baseCommand", "$schemas"];

/// formats cwl document in an oppinionated way. Heavily inspired by <https://github.com/rabix/cwl-format>
pub fn format_cwl(raw_cwl: &str) -> Result<String, Box<dyn Error>> {
    let cwl = &serde_yaml::from_str(raw_cwl)?;

    let comment = add_leading_comment(raw_cwl);

    let formatted_node = format_node(cwl);
    let mut formatted_cwl = serde_yaml::to_string(&formatted_node)?;
    formatted_cwl = add_space_between_main_sections(&formatted_cwl);

    Ok(format!("{}{}", comment, formatted_cwl))
}

fn format_node(cwl: &Value) -> Value {
    match cwl {
        Value::Mapping(map) => {
            let node_type = infer_type(map);
            let reordered_map = reorder_node(map, node_type);

            let formatted_map: Mapping = reordered_map.into_iter().map(|(k, v)| (k, format_node(&v))).collect();
            Value::Mapping(formatted_map)
        }
        Value::Sequence(seq) => Value::Sequence(seq.iter().map(format_node).collect()),
        _ => cwl.clone(),
    }
}

fn infer_type(cwl: &Mapping) -> &str {
    if let Some(Value::String(class)) = cwl.get(Value::String("class".to_string())) {
        class
    } else {
        "generic-ordering"
    }
}

fn reorder_node(cwl: &Mapping, node_type: &str) -> Mapping {
    let key_order_dict = get_key_order();
    let key_order = key_order_dict
        .get(node_type)
        .or_else(|| key_order_dict.get("generic-ordering"))
        .expect("Key order not found");

    let mut ordered_map = Mapping::new();
    let mut extra_keys = vec![];

    for key in key_order {
        let kv = Value::String(key.to_string());
        if let Some(value) = cwl.get(&kv) {
            ordered_map.insert(kv, value.clone());
        }
    }

    //extra keys to be pushed to end
    for (k, v) in cwl {
        if !key_order.contains(&k.as_str().unwrap_or("")) {
            extra_keys.push((k.clone(), v.clone()));
        }
    }

    for (k, v) in extra_keys {
        ordered_map.insert(k, v);
    }

    ordered_map
}

/// Gets static order of yaml keys according to https://github.com/rabix/cwl-format/blob/master/cwlformat/keyorder.yml
fn get_key_order() -> HashMap<&'static str, Vec<&'static str>> {
    let mut key_order_dict = HashMap::new();

    key_order_dict.insert(
        "generic-ordering",
        vec![
            "id",
            "label",
            "name",
            "doc",
            "class",
            "type",
            "format",
            "default",
            "secondaryFiles",
            "inputBinding",
            "prefix",
            "position",
            "valueFrom",
            "separate",
            "itemSeparator",
            "shellQuote",
            "outputBinding",
            "glob",
            "outputEval",
            "loadContents",
            "loadListing",
            "dockerPull",
            "entryname",
            "writable",
            "in",
            "scatter",
            "scatterMethod",
            "run",
            "when",
            "out",
            "requirements",
            "hints",
            "source",
            "outputSource",
            "linkMerge",
        ],
    );

    key_order_dict.insert(
        "CommandLineTool",
        vec![
            "cwlVersion",
            "class",
            "label",
            "doc",
            "$namespaces",
            "requirements",
            "inputs",
            "outputs",
            "stdout",
            "stderr",
            "baseCommand",
            "arguments",
            "hints",
            "id",
        ],
    );

    key_order_dict.insert(
        "ExpressionTool",
        vec!["cwlVersion", "class", "label", "doc", "requirements", "inputs", "outputs", "expression", "hints", "id"],
    );

    key_order_dict.insert(
        "Workflow",
        vec![
            "cwlVersion",
            "class",
            "label",
            "doc",
            "$namespaces",
            "requirements",
            "inputs",
            "outputs",
            "steps",
            "hints",
            "id",
        ],
    );

    key_order_dict
}

fn add_leading_comment(raw_cwl: &str) -> String {
    let mut top_comment = Vec::new();

    if !raw_cwl.is_empty() && !raw_cwl.trim_start().starts_with('{') {
        for line in raw_cwl.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                top_comment.push(line.to_string());
            } else {
                break;
            }
        }
    }

    if top_comment.is_empty() || !top_comment[0].starts_with(HASH_BANG_PRE) {
        top_comment.insert(0, HASH_BANG.to_string());
    }

    top_comment.join("\n")
}

fn add_space_between_main_sections(raw_cwl: &str) -> String {
    let mut result = String::new();
    let mut was_special_key = false;

    for line in raw_cwl.lines() {
        let trimmed_line = line.trim();

        if KEYS_WITH_NEWLINES.iter().any(|&key| trimmed_line.starts_with(key)) {
            if !was_special_key {
                result.push('\n');
            }
            was_special_key = true;
        } else {
            was_special_key = false;
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}
