use chrono::Utc;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use uuid::Uuid;

type ScriptStep = (String, Vec<(String, String)>, Vec<(String, String)>, Option<String>);
type StepTimestamp = HashMap<String, (Option<String>, Option<String>)>;

pub fn create_root_dataset_entity(conforms_to: &[&str], license: &str, name: &str, description: &str, parts: &[&str], mentions: &str) -> Value {
    let has_part: Vec<Value> = parts.iter().map(|id| json!({ "@id": id })).collect();
    let now = Utc::now();
    let timestamp = now.to_rfc3339();
    json!({
        "@id": "./",
        "@type": "Dataset",
        "datePublished": timestamp,
        "description": description,
        "conformsTo": conforms_to.iter().map(|id| json!({ "@id": id })).collect::<Vec<_>>(),
        "hasPart": has_part,
        "license": license,
        "mainEntity": { "@id": "workflow.json" },
        "name": name,
        "mentions": { "@id": mentions },
    })
}

fn extract_workflow_steps(json_data: &Value) -> Vec<(String, String)> {
    let mut steps = Vec::new();
    if let Some(graph) = json_data.pointer("/workflow/specification/$graph").and_then(|v| v.as_array()) {
        for item in graph {
            if item.get("class").and_then(Value::as_str) == Some("Workflow") {
                if let Some(step_array) = item.get("steps").and_then(|v| v.as_array()) {
                    for step in step_array {
                        if let (Some(id), Some(run)) = (step.get("id").and_then(Value::as_str), step.get("run").and_then(Value::as_str)) {
                            steps.push((id.to_string(), run.to_string()));
                        }
                    }
                }
            }
        }
    }
    steps
}

pub fn create_workflow_entity(connections: &[String], step_ids: &[&str], input_ids: &[String], output_ids: &[String], tool_ids: &[&str]) -> Value {
    json!({
        "@id": "workflow.json",
        "@type": [
            "File",
            "SoftwareSourceCode",
            "ComputationalWorkflow",
            "HowTo"
        ],
        "connection": connections.iter().filter_map(|id| id.rsplit('/').next()).map(|id| json!({ "@id": id })).collect::<Vec<_>>(),
        "hasPart": step_ids.iter().filter_map(|id| id.rsplit('/').next()).map(|id| json!({ "@id": format!("workflow.json#{id}") })).collect::<Vec<_>>(),
        "input": input_ids.iter().map(|id| json!({ "@id": format!("workflow.json{id}") })).collect::<Vec<_>>(),
        "name": "workflow.json",
        "output": output_ids.iter().map(|id| json!({ "@id": format!("workflow.json{id}") })).collect::<Vec<_>>(),
        "programmingLanguage": {
            "@id": "https://w3id.org/workflowhub/workflow-ro-crate#cwl"
        },
        "step": tool_ids.iter().map(|id| json!({ "@id": format!("workflow.json{id}") })).collect::<Vec<_>>()
    })
}

pub struct Action<'a> {
    pub action_type: &'a str,
    pub id: &'a str,
    pub name: &'a str,
    pub instrument_id: &'a str,
    pub object_ids: Vec<String>,
    pub result_ids: Option<Vec<&'a str>>,
    pub start_time: Option<&'a str>,
    pub end_time: Option<&'a str>,
    pub container_image_id: Option<&'a str>,
}

pub fn create_action(a: Action) -> Value {
    let mut action = json!({
        "@id": a.id,
        "@type": a.action_type,
        "name": a.name,
        "instrument": { "@id": a.instrument_id }
    });
    if !a.object_ids.is_empty() {
        let objects: Vec<Value> = a.object_ids.iter().map(|id| json!({ "@id": id })).collect();
        action["object"] = if objects.len() == 1 { objects[0].clone() } else { Value::Array(objects) };
    }
    if let Some(results) = a.result_ids {
        let result_json: Vec<Value> = results.iter().map(|id| json!({ "@id": id })).collect();
        if !result_json.is_empty() {
            action["result"] = if result_json.len() == 1 {
                result_json[0].clone()
            } else {
                Value::Array(result_json)
            };
        }
    }
    if let Some(start) = a.start_time {
        action["startTime"] = json!(start);
    }
    if let Some(end) = a.end_time {
        action["endTime"] = json!(end);
    }
    if let Some(container_id) = a.container_image_id {
        action["containerImage"] = json!({ "@id": container_id });
    }
    action
}

fn create_howto_steps(
    steps: &[(String, String)],
    connections: &[(String, String, String)],
    inputs: Vec<String>,
    id: &str,
    formal_parameters: &[Value],
) -> Vec<serde_json::Value> {
    let input_set: HashSet<&str> = inputs.iter().map(|s| s.as_str()).collect();
    let formal_ids: HashSet<String> = formal_parameters
        .iter()
        .filter_map(|fp| {
            let default_value = fp.get("defaultValue").and_then(Value::as_str)?;
            let id = fp.get("@id").and_then(Value::as_str)?;
            if input_set.contains(default_value) {
                Some(id.to_string())
            } else {
                None
            }
        })
        .collect();
    if formal_ids.is_empty() {
        return Vec::new();
    }
    let new_connections = connections
        .iter()
        .filter_map(|(source, target, conn_id)| {
            if formal_ids.contains(source) || formal_ids.contains(target) {
                Some(json!({ "@id": conn_id }))
            } else {
                None
            }
        })
        .collect::<Vec<Value>>();
    steps
        .iter()
        .enumerate()
        .filter_map(|(i, (step_id, step_id_match))| {
            if step_id_match == id {
                Some(json!({
                    "@id": format!("workflow.json{}", step_id),
                    "@type": "HowToStep",
                    "position": i.to_string(),
                    "connection": new_connections,
                    "workExample": {
                        "@id": format!("workflow.json#{}", id.rsplit('/').next().unwrap_or(""))
                    }
                }))
            } else {
                None
            }
        })
        .collect()
}

fn create_software_application(id: &str, inputs: &[String], outputs: &[String]) -> Value {
    let formatted_inputs: Vec<Value> = inputs.iter().map(|input| json!({ "@id": format!("workflow.json{input}") })).collect();
    let formatted_outputs: Vec<Value> = outputs.iter().map(|output| json!({ "@id": format!("workflow.json{output}") })).collect();
    json!({
        "@id": format!("workflow.json#{id}"),
        "@type": "SoftwareApplication",
        "name": id,
        "input": formatted_inputs,
        "output": formatted_outputs
    })
}

fn create_parameter_connection(id: &str, source: &str, target: &str) -> Value {
    json!({
        "@id": id,
        "@type": "ParameterConnection",
        "sourceParameter": { "@id": source },
        "targetParameter": { "@id": target }
    })
}

fn create_instruments(id: &str, type_str: &str, name: &str) -> Value {
    json!({
        "@id": id,
        "@type": type_str,
        "name": name
    })
}

fn create_formal_parameter(id: &str, additional_type: &str, default_value: Option<&str>) -> Value {
    let fixed_id = if id.starts_with("workflow.json") {
        id.to_string()
    } else {
        format!("workflow.json{id}")
    };
    let name = id.rsplit('/').next().unwrap_or(id);
    let mut obj = json!({
        "@id": fixed_id,
        "@type": "FormalParameter",
        "additionalType": additional_type,
        "name": name
    });
    if let Some(default) = default_value {
        obj["defaultValue"] = json!(default);
    }
    obj
}

fn create_cwl_entity(id: &str, type_str: &str, alt_name: &str, identifier: &str, name: &str, url: &str, version: &str) -> Value {
    json!({
        "@id": id,
        "@type": type_str,
        "alternateName": alt_name,
        "identifier": { "@id": identifier },
        "name": name,
        "url": { "@id": url },
        "version": version
    })
}
//create entities for each file
pub fn create_files(
    connections: &[(String, String, String)],
    parts: &[String],
    graph: &[Value],
) -> Vec<Value> {
    let mut file_entities = Vec::new();
    for (source_id, target_id, fallback_uuid) in connections {
        let name = target_id.rsplit(&['/', '#']).next().unwrap_or(target_id);
        let mut file_id = fallback_uuid.as_str();
        let mut alt_name = name;
        if let Some(part) = parts.iter().find(|p| p.contains(name)) {
            file_id = part;
            alt_name = part;
        } else if let Some(glob_or_loc) = find_glob_or_location_for_id(source_id, graph).or_else(|| find_glob_or_location_for_id(target_id, graph)) {
            if let Some(part) = parts.iter().find(|p| p.contains(&glob_or_loc)) {
                file_id = part;
                alt_name = part;
            }
        }
        let content_size = get_file_size(alt_name);
        let normalize_id = |id: &str| {
            if id.starts_with("workflow.json#") {
                id.to_string()
            } else {
                format!("workflow.json#{id}")
            }
        };
        let entity = json!({
            "@id": file_id,
            "@type": "File",
            "alternateName": alt_name,
            "contentSize": content_size,
            "exampleOfWork": [
                { "@id": normalize_id(source_id) },
                { "@id": normalize_id(target_id) }
            ],
        });
        file_entities.push(entity);
    }
    file_entities
}

//search for path of file
pub fn find_glob_or_location_for_id(target_id: &str, graph: &[Value]) -> Option<String> {
    for entry in graph {
        for key in ["outputs", "inputs"] {
            if let Some(array) = entry.get(key).and_then(|v| v.as_array()) {
                for item in array {
                    if let Some(item_id) = item.get("id").and_then(Value::as_str) {
                        let target_fragment = target_id.rsplit_once('#').map_or(target_id, |(_, frag)| frag);

                        if item_id.ends_with(target_fragment) {
                            if let Some(glob) = item.pointer("/outputBinding/glob").and_then(Value::as_str) {
                                return Some(glob.to_string());
                            }
                            if let Some(loc) = item.pointer("/default/location").and_then(Value::as_str) {
                                return Some(loc.rsplit('/').next().unwrap_or(loc).to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
//for all conforms to parts
fn create_creative_work(id: &str) -> Value {
    let version = id.rsplit('/').next().unwrap_or(id);
    let name = match id {
        s if s.contains("process") => "Process Run Crate",
        s if s.contains("workflow/0.5") => "Workflow Run Crate",
        s if s.contains("provenance") => "Provenance Run Crate",
        s if s.contains("workflow-ro-crate") => "Workflow RO-Crate",
        _ => "Unknown Crate",
    };
    json!({
        "@id": id,
        "@type": "CreativeWork",
        "name": name,
        "version": version
    })
}

fn create_ro_crate_metadata(id: &str, about_id: Option<&str>, conforms_to_ids: &[&str]) -> Value {
    let about_id = about_id.unwrap_or("./");
    let conforms_to: Vec<Value> = conforms_to_ids.iter().map(|&uri| json!({ "@id": uri })).collect();
    json!({
        "@id": id,
        "@type": "CreativeWork",
        "about": { "@id": about_id },
        "conformsTo": conforms_to
    })
}

fn generate_id_with_hash() -> String {
    format!("#{}", Uuid::new_v4())
}

//check if there is a name, description and license or ask the user to provide them
pub fn extract_or_prompt_metadata(graph: &[Value]) -> (String, String, String) {
    fn prompt_or_default(prompt_msg: &str, default: &str) -> String {
        let input = prompt(prompt_msg);
        if input.trim().is_empty() {
            default.to_string()
        } else {
            input
        }
    }
    let workflow = graph.iter().find(|item| item.get("class").and_then(Value::as_str) == Some("Workflow"));
    let name = workflow
        .and_then(|w| w.get("name").and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| prompt_or_default("Enter workflow name: ", "run of workflow.json"));
    let description = workflow
        .and_then(|w| w.get("description").and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| prompt_or_default("Enter workflow description: ", "run of workflow.json"));
    let license = workflow
        .and_then(|w| w.get("license").and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| prompt_or_default("Enter workflow license: ", "notspecified"));

    (name, description, license)
}

fn prompt(message: &str) -> String {
    print!("{message}");
    std::io::stdout().flush().expect("Failed to flush stdout");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Failed to read input");
    input.trim().to_string()
}

//get all files
fn extract_parts(script_structure: &[ScriptStep]) -> Vec<String> {
    let mut parts = HashSet::new();
    for (_, inputs, outputs, _) in script_structure {
        for (_, path) in inputs.iter().chain(outputs.iter()) {
            if let Some(file_name) = std::path::Path::new(path).file_name().and_then(|f| f.to_str()) {
                parts.insert(file_name.to_string());
            }
        }
    }
    parts.insert("workflow.json".to_string());
    let mut parts_vec: Vec<String> = parts.into_iter().collect();
    parts_vec.sort();
    parts_vec
}

//use reana log files to extract start and end times of stepts
fn extract_times_from_logs(contents: &str) -> Result<StepTimestamp, Box<dyn std::error::Error>> {
    let re_timestamp = Regex::new(r"(?P<timestamp>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2},\d{3})").unwrap();
    let re_workflow_start = Regex::new(r"running workflow on context").unwrap();
    let re_workflow_end = Regex::new(r"workflow done").unwrap();
    let re_step_start = Regex::new(r"starting step (?P<step>\w+)").unwrap();
    let re_step_end = Regex::new(r"\[step (?P<step>\w+)\] completed success").unwrap();
    let mut workflow_start = None;
    let mut workflow_end = None;
    let mut steps: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
    for line in contents.lines() {
        if let Some(cap) = re_timestamp.captures(line) {
            let timestamp = cap["timestamp"].to_string();
            if re_workflow_start.is_match(line) {
                workflow_start = Some(timestamp.clone());
            }
            if re_workflow_end.is_match(line) {
                workflow_end = Some(timestamp.clone());
            }
            if let Some(cap_step) = re_step_start.captures(line) {
                let step = cap_step["step"].to_string();
                steps.entry(step).or_insert((None, None)).0 = Some(timestamp.clone());
            }
            if let Some(cap_step) = re_step_end.captures(line) {
                let step = cap_step["step"].to_string();
                steps.entry(step).or_insert((None, None)).1 = Some(timestamp.clone());
            }
        }
    }
    steps.insert("workflow".to_string(), (workflow_start, workflow_end));
    Ok(steps)
}

fn get_file_size(path: &str) -> String {
    if let Ok(meta) = std::fs::metadata(path) {
        meta.len().to_string()
    } else {
        "unknown".to_string()
    }
}

pub fn get_workflow_structure(workflow_json: &Value) -> Vec<ScriptStep> {
    let mut results = Vec::new();
    let mut docker_map = HashMap::new();

    let elements = workflow_json
        .pointer("/workflow/specification/$graph")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    // docker info for CommandLineTool
    for e in &elements {
        if e.get("class") == Some(&Value::String("CommandLineTool".into())) {
            if let Some(id) = e.get("id").and_then(Value::as_str) {
                let docker = e.get("requirements").and_then(Value::as_array).and_then(|r| {
                    r.iter().find_map(|req| {
                        (req.get("class") == Some(&Value::String("DockerRequirement".into())))
                            .then(|| req.get("dockerPull")?.as_str())
                            .flatten()
                    })
                });
                if let Some(img) = docker {
                    docker_map.insert(id.to_string(), img.to_string());
                }
            }
        }
    }
    // inputs and outputs
    for e in &elements {
        let id = e.get("id").and_then(Value::as_str).unwrap_or("unknown").to_string();
        let io = |key: &str, val_key: &str| {
            e.get(key)
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            let k = item.get("id")?.as_str()?.to_string();
                            let v = item
                                .get(val_key)?
                                .get(if key == "inputs" { "location" } else { "glob" })?
                                .as_str()?
                                .to_string();
                            Some((k, v))
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let inputs: Vec<_> = io("inputs", "default");
        let outputs = io("outputs", "outputBinding");
        let docker = docker_map.get(&id).cloned();
        if !inputs.is_empty() || !outputs.is_empty() {
            results.push((id, inputs, outputs, docker));
        }
    }
    // Workflow inputs
    let wf_inputs = workflow_json
        .pointer("/inputs/parameters")
        .and_then(Value::as_object)
        .map(|p| {
            p.iter()
                .filter_map(|(k, v)| v.get("location").and_then(Value::as_str).map(|loc| (k.clone(), loc.to_string())))
                .collect()
        })
        .unwrap_or_default();
    // Workflow outputs
    let wf_outputs = workflow_json
        .pointer("/workflow/specification/$graph")
        .and_then(Value::as_array)
        .and_then(|g| g.iter().find(|n| n.get("class") == Some(&Value::String("Workflow".into()))))
        .and_then(|wf| wf.get("outputs").and_then(Value::as_array))
        .map(|outs| {
            let empty = vec![];
            let files = workflow_json.pointer("/outputs/files").and_then(Value::as_array).unwrap_or(&empty);
            outs.iter()
                .zip(files)
                .filter_map(|(o, f)| Some((o.get("id")?.as_str()?.to_string(), f.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    results.push(("#main".into(), wf_inputs, wf_outputs, None));
    results
}

fn create_container_image(id: &str, name: &str, tag: &str, registry: &str) -> Value {
    json!({
        "@id": id,
        "@type": "ContainerImage",
        "additionalType": {
            "@id": "https://w3id.org/ro/terms/workflow-run#DockerImage"
        },
        "name": name,
        "registry": registry,
        "tag": tag
    })
}

fn extract_paths_with_ids(script_structure: &[ScriptStep]) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for (_step_id, inputs, outputs, _) in script_structure {
        for (id, path) in inputs.iter().chain(outputs.iter()) {
            if path.starts_with("file://") {
                result.push((id.clone(), path.clone()));
            }
        }
    }
    result
}

//find connections of inputs, outputs and between workflow steps/CommandLineTools
fn generate_connections(script_structure: &[ScriptStep]) -> Vec<(String, String, String)> {
    let mut conns = vec![];
    for (in_id, _, from_outputs, _) in script_structure {
        for (out_id, to_inputs, _, _) in script_structure {
            for (out_path, _) in from_outputs {
                for (in_path, _) in to_inputs {
                    if out_path == in_path {
                        let id = out_path.trim_start_matches('#').rsplit('/').next().unwrap_or(out_path);
                        let input = format!(
                            "workflow.json#{}/{}",
                            in_id.trim_start_matches('#').rsplit('/').next().unwrap_or(in_id),
                            id
                        );
                        let output = format!(
                            "workflow.json#{}/{}",
                            out_id.trim_start_matches('#').rsplit('/').next().unwrap_or(out_id),
                            id
                        );
                        conns.push((input, output, generate_id_with_hash()));
                    }
                }
            }
        }
    }
    let (main, others): (Vec<_>, Vec<_>) = script_structure.iter().partition(|(id, _, _, _)| id == "#main");
    let Some((_, main_inputs, main_outputs, _)) = main.first() else {
        return vec![];
    };
    let other_ports: Vec<(String, String)> = others
        .iter()
        .flat_map(|(id, inputs, outputs, _)| {
            let step = id.trim_start_matches('#').rsplit('/').next().unwrap_or(id);
            inputs.iter().chain(outputs).map(move |(port, path)| {
                let name = port.rsplit('/').next().unwrap_or(port);
                (format!("workflow.json#{step}/{name}"), path.clone())
            })
        })
        .collect();
    for (name, path, flip) in main_inputs
        .iter()
        .map(|(n, p)| (n, p, false))
        .chain(main_outputs.iter().map(|(n, p)| (n, p, true)))
    {
        let main_id = format!("workflow.json#main/{name}");
        for (other_id, other_path) in &other_ports {
            if path.ends_with(other_path) || other_path.ends_with(path) {
                let (a, b) = if flip {
                    (other_id.clone(), main_id.clone())
                } else {
                    (main_id.clone(), other_id.clone())
                };
                conns.push((a, b, generate_id_with_hash()));
            }
        }
    }
    conns
}

pub fn create_ro_crate_metadata_json(json_data: &serde_json::Value, logs: &str, conforms_to: &[&str]) -> Result<Value, Box<dyn std::error::Error>> {
    let graph_json = json_data
        .get("workflow")
        .and_then(|w| w.get("specification"))
        .and_then(|s| s.get("$graph"))
        .ok_or("Missing '$graph' field in workflow specification")?
        .as_array()
        .ok_or("'$graph' must be an array")?;

    // if $graph is empty, return minimal valid RO-Crate
    if graph_json.is_empty() {
        return Ok(json!({
            "@context": "https://w3id.org/ro/crate/1.1/context",
            "@graph": []
        }));
    }

    // extract connections, steps, parts, etc
    let steps = extract_workflow_steps(json_data);
    let step_ids: Vec<&str> = steps.iter().map(|(_, step)| step.as_str()).collect();
    let step_files: Vec<&str> = steps.iter().map(|(file, _)| file.as_str()).collect();
    let script_structure = get_workflow_structure(json_data);
    let parts = extract_parts(&script_structure);
    let connections = generate_connections(&script_structure);
    let connections_slice: &[(String, String, String)] = connections.as_slice();
    let parts_ref: Vec<&str> = parts.iter().map(String::as_str).collect();
    let (name, description, license) = extract_or_prompt_metadata(graph_json);

    // create main rocrate metadata and CWL entity
    let ro_crate_metadata = create_ro_crate_metadata(
        "ro-crate-metadata.json",
        Some("./"),
        &["https://w3id.org/ro/crate/1.1", "https://w3id.org/workflowhub/workflow-ro-crate/1.0"],
    );

    let cwl_entity = create_cwl_entity(
        "https://w3id.org/workflowhub/workflow-ro-crate#cwl",
        "ComputerLanguage",
        "CWL",
        "https://w3id.org/cwl/v1.2/",
        "Common Workflow Language",
        "https://www.commonwl.org/",
        "v1.2",
    );

    // create CreativeWork entities based on conforms_to
    let creative_works: Vec<Value> = conforms_to.iter().map(|id| create_creative_work(id)).collect();

    // create parameter connections from connection triples
    let parameter_connections: Vec<Value> = connections
        .iter()
        .map(|(source, target, id)| create_parameter_connection(id, source, target))
        .collect();

    let mut graph = vec![ro_crate_metadata, cwl_entity];
    graph.extend(creative_works);

    // extract timestamps from logs for action timing
    let timestamps = extract_times_from_logs(logs)?;
    let mut organize_obj_ids = Vec::new();
    let mut organize_res_ids = Vec::new();
    // map file paths to ids
    let paths_with_ids = extract_paths_with_ids(&script_structure);
    let path_map: HashMap<_, _> = paths_with_ids.into_iter().collect();

    for (id, inputs, outputs, docker) in &script_structure {
        let script_name = id.trim_start_matches('#').rsplit('/').next().unwrap_or(id).to_string();
        let create_id = generate_id_with_hash();
        let control_id = generate_id_with_hash();
        let mut docker_id = None;
        let mut formal_params: Vec<Value> = Vec::new();
        //if CommandLineTool
        let step_name = if id.ends_with(".cwl") {
            //OrganizeAction has control_id of CommandLineTools
            organize_obj_ids.push(control_id.clone());
            //create docker_entity
            if let Some(docker_pull) = docker {
                let parts: Vec<&str> = docker_pull.split(':').collect();
                if parts.len() != 2 {
                    return Err("Invalid Docker image format, expected 'name:tag'".into());
                }
                docker_id = Some(generate_id_with_hash());
                let docker_entity = create_container_image(docker_id.as_ref().unwrap(), parts[0], parts[1], "docker.io");
                graph.push(docker_entity);
                //create software_application
                let input_names = inputs
                    .iter()
                    .map(|(id, _)| id.split_once('/').map(|(_, tail)| format!("#{tail}")).unwrap_or_else(|| id.to_string()))
                    .collect::<Vec<_>>();
                let output_names = outputs
                    .iter()
                    .map(|(id, _)| id.split_once('/').map(|(_, tail)| format!("#{tail}")).unwrap_or_else(|| id.to_string()))
                    .collect::<Vec<_>>();
                let software_application = create_software_application(&script_name, &input_names, &output_names);
                graph.push(software_application);
                let mut seen_ids = HashSet::new();
                //create formal_parameters for inputs and outputs
                for (id, _) in inputs.iter().chain(outputs.iter()) {
                    if seen_ids.insert(id) {
                        let param_id = id.find('/').map(|pos| format!("#{}", &id[pos + 1..])).unwrap_or_else(|| id.to_string());

                        if let Some(path) = path_map.get(id) {
                            formal_params.push(create_formal_parameter(&param_id, "File", Some(path.as_str())));
                        }
                    }
                }
            }
            id.trim_start_matches('#')
                .trim_end_matches(".cwl")
                .rsplit('/')
                .next()
                .unwrap_or(id)
                .to_string()
        } else if id == "workflow.json" || id == "#main" {
            let modified_inputs: Vec<(String, String)> = inputs
                .clone()
                .into_iter()
                .map(|(input_id, loc)| {
                    let new_id = format!("{id}/{input_id}");
                    let path = std::path::Path::new(&loc);
                    let classification = if path.extension().is_some() {
                        "File"
                    } else if path.is_dir() {
                        "Directory"
                    } else {
                        "String"
                    };
                    (new_id, classification.to_string())
                })
                .collect();
            for (input_id, classification) in modified_inputs.iter().chain(outputs.iter()) {
                formal_params.push(create_formal_parameter(input_id, classification, None));
            }
            let output_ids: Vec<String> = outputs.iter().map(|(out_id, _)| format!("workflow.json{id}/{out_id}")).collect();
            let mut conn_ids = Vec::new();
            let out_ids: Vec<String> = output_ids
                .iter()
                .filter_map(|target_id| {
                    parameter_connections.iter().find_map(|conn| {
                        let target_param = conn.get("targetParameter").and_then(|tp| tp.get("@id")).and_then(Value::as_str);

                        let source_param = conn.get("sourceParameter").and_then(|sp| sp.get("@id")).and_then(Value::as_str);

                        if target_param == Some(target_id.as_str()) {
                            if let Some(conn_id) = conn.get("@id").and_then(Value::as_str) {
                                conn_ids.push(conn_id.to_string());
                            }
                            source_param.map(|s| s.strip_prefix("workflow.json").expect("could not strip prefix").to_string())
                        } else {
                            None
                        }
                    })
                })
                .collect();
            let input_ids: Vec<String> = modified_inputs.iter().map(|(i, _)| i.to_string()).collect();
            //create workflow_entity
            let workflow_entity = create_workflow_entity(&conn_ids, &step_ids, &input_ids, &out_ids, &step_files);
            graph.push(workflow_entity);
            organize_res_ids.push(create_id.clone());
            //create root_dataset
            let root_dataset = create_root_dataset_entity(conforms_to, &license, &name, &description, &parts_ref, &create_id);
            graph.push(root_dataset);
            "workflow".to_string()
        } else {
            continue;
        };
        //create how_to_steps
        let how_to_steps = create_howto_steps(
            &steps,
            connections_slice,
            inputs.iter().map(|(_, v)| v.to_string()).collect(),
            id,
            &formal_params,
        );
        graph.extend(how_to_steps);
        let step_opt = steps.iter().find(|(_, step_id)| *step_id == *id).map(|(file, _)| file.to_string());
        let (start, end) = if step_name == "workflow" {
            timestamps.get("workflow").cloned().unwrap_or((None, None))
        } else {
            timestamps.get(&step_name).cloned().unwrap_or((None, None))
        };
        let output_refs: Vec<&str> = outputs.iter().map(|(_, v)| v.as_str()).collect();
        let input_file_names: Vec<String> = inputs
            .iter()
            .filter_map(|(_, path)| {
                let clean_path = path.strip_prefix("file://").unwrap_or(path);
                std::path::Path::new(clean_path)
                    .file_name()
                    .and_then(|os_str| os_str.to_str())
                    .map(String::from)
            })
            .collect();
        let formatted = if id != "workflow.json" && id != "#main" {
            &format!("workflow.json#{}", id.rsplit('/').next().unwrap_or(id))
        } else {
            "workflow.json"
        };
        //createAction
        let create = Action {
            action_type: "CreateAction",
            id: &create_id,
            name: &format!("Run of workflow.json{}", step_opt.as_deref().unwrap_or("")),
            instrument_id: formatted,
            object_ids: input_file_names,
            result_ids: Some(output_refs),
            start_time: start.as_deref(),
            end_time: end.as_deref(),
            container_image_id: docker_id.as_deref(),
        };
        //ControlAction
        let create_action_obj = create_action(create);
        if let Some(step_val) = step_opt {
            let control_action = Action {
                action_type: "ControlAction",
                id: &control_id,
                name: &format!("orchestrate workflow.json#{}", id.rsplit('/').next().unwrap_or("")),
                instrument_id: &format!("workflow.json{step_val}"),
                object_ids: vec![create_id.clone()],
                result_ids: None,
                start_time: None,
                end_time: None,
                container_image_id: None,
            };
            graph.push(create_action(control_action));
        }
        graph.push(create_action_obj);
        graph.extend(formal_params);
    }
    graph.extend(parameter_connections);
    // Extract instrument version from logs
    let re = Regex::new(r"cwltool (\S+)")?;
    let instrument_id = generate_id_with_hash();
    if let Some(caps) = re.captures(logs) {
        let version = format!("cwltool {}", &caps[1]);
        let instrument = create_instruments(&instrument_id, "SoftwareApplication", &version);
        graph.push(instrument.clone());
        //OrganizeAction
        let organize_id = generate_id_with_hash();
        let organize_res_str: Vec<&str> = organize_res_ids.iter().map(String::as_str).collect();
        let (start_org, end_org) = timestamps.get("workflow").cloned().unwrap_or((None, None));
        let organize_action = Action {
            action_type: "OrganizeAction",
            id: &organize_id,
            name: &format!("Run of {version}"),
            instrument_id: &instrument_id,
            object_ids: organize_obj_ids,
            result_ids: Some(organize_res_str),
            start_time: start_org.as_deref(),
            end_time: end_org.as_deref(),
            container_image_id: None,
        };
        //files entity with alternateName
        let files = create_files(connections_slice, &parts, graph_json);
        graph.extend(files);
        graph.push(create_action(organize_action));
    }
    Ok(json!({
        "@context": ["https://w3id.org/ro/crate/1.1/context","https://w3id.org/ro/terms/workflow-run"],
        "@graph": graph
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::fs;

    //uuids and datePublished differ: remove datePublished and replace uuid by other id that keeps track of ordering
    fn normalize_uuids_and_strip_date_published(
        value: &Value,
        uuid_map: &mut HashMap<String, String>,
        uuid_re: &Regex,
        counter: &mut usize,
    ) -> Value {
        match value {
            Value::String(s) => {
                if uuid_re.is_match(s) {
                    let entry = uuid_map.entry(s.clone()).or_insert_with(|| {
                        let label = format!("UUID-{}", counter);
                        *counter += 1;
                        label
                    });
                    Value::String(entry.clone())
                } else {
                    Value::String(s.clone())
                }
            }
            Value::Array(arr) => Value::Array(
                arr.iter()
                    .map(|v| normalize_uuids_and_strip_date_published(v, uuid_map, uuid_re, counter))
                    .collect(),
            ),
            Value::Object(map) => {
                let new_map = map
                    .iter()
                    .filter(|(k, _)| k != &"datePublished")
                    .map(|(k, v)| (k.clone(), normalize_uuids_and_strip_date_published(v, uuid_map, uuid_re, counter)))
                    .collect();
                Value::Object(new_map)
            }
            _ => value.clone(),
        }
    }

    #[test]
    fn test_workflow_structure_similarity() -> Result<(), Box<dyn std::error::Error>> {
        let workflow_json_str = fs::read_to_string("../../tests/test_data/workflow.json").unwrap();
        let workflow_json: Value = serde_json::from_str(&workflow_json_str).unwrap();
        let logs_str = std::fs::read_to_string("../../tests/test_data/reana_logs.txt").unwrap();

        // Read expected output json
        let expected_str = fs::read_to_string("../../tests/test_data/ro-crate-metadata.json").unwrap();
        let expected_json: Value = serde_json::from_str(&expected_str)?;

        let conforms_to = [
            "https://w3id.org/ro/wfrun/process/0.5",
            "https://w3id.org/ro/wfrun/workflow/0.5",
            "https://w3id.org/ro/wfrun/provenance/0.5",
            "https://w3id.org/workflowhub/workflow-ro-crate/1.0",
        ];

        // generate rocrate
        let result = create_ro_crate_metadata_json(&workflow_json, &logs_str, &conforms_to).expect("Function should return Ok");
        let generated_json: Value = serde_json::to_value(result)?;

        let uuid_re = Regex::new(r"#?[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}").unwrap();

        let mut uuid_map1 = HashMap::new();
        let mut counter1 = 0;
        let normalized_expected = normalize_uuids_and_strip_date_published(&expected_json, &mut uuid_map1, &uuid_re, &mut counter1);

        let mut uuid_map2 = HashMap::new();
        let mut counter2 = 0;
        let normalized_generated = normalize_uuids_and_strip_date_published(&generated_json, &mut uuid_map2, &uuid_re, &mut counter2);
        //compare expected and generated json files with replaced uuids
        assert_eq!(normalized_expected, normalized_generated, "structures do not match");
        Ok(())
    }

    #[test]
    fn test_create_ro_crate_metadata_json_with_empty_graph() {
        let input_json = json!({
            "workflow": {
                "specification": {
                    "$graph": []
                }
            }
        });

        let logs = "";
        let conforms_to = &[];

        let result = create_ro_crate_metadata_json(&input_json, logs, conforms_to);

        assert!(result.is_ok(), "Function should succeed even with empty $graph");

        let output = result.unwrap();

        assert!(output.is_object(), "Output should be a JSON object");

        let graph = output.get("@graph").unwrap_or(&json!(null));
        assert!(graph.is_array(), "@graph should be an array");
    }
}
