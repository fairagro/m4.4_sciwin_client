use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use git2::Repository;
use serde_json::{json, Value};
#[cfg(all(unix, not(target_os = "macos")))]
use std::os::unix::fs::MetadataExt;
use serde_yaml::Value as YamlValue;

use crate::arc_entities::{
    ArcWorkflow, ArcRun, WorkflowInvocation, WorkflowProtocol, ArcRoCrate, MainEntity, RoleEntity,
    PersonEntity, OrganizationEntity, CreativeWorkEntity, RootDataEntity, ComputerLanguageEntity,
    PropertyValueEntity
};

const CREATIVE_WORKS: &[(&str, &str, &str)] = &[
    ("https://w3id.org/ro/wfrun/process/0.1", "Process Run Crate", "0.1"),
    ("https://w3id.org/ro/wfrun/workflow/0.1", "Workflow Run Crate", "0.1"),
    ("https://w3id.org/workflowhub/workflow-ro-crate/1.0", "Workflow RO-Crate", "1.0"),
];

fn ensure_creativeworks(graph: &mut Vec<Value>) -> Vec<Value> {
    let mut ids = Vec::new();
    for (id, name, version) in CREATIVE_WORKS {
        ids.push(json!({ "@id": id }));
        if !graph.iter().any(|v| v.get("@id").and_then(|s| s.as_str()) == Some(*id)) {
            let entity = CreativeWorkEntity {
                id: id.to_string(),
                type_: "CreativeWork".to_string(),
                name: Some(name.to_string()),
                version: Some(version.to_string()),
            };
            graph.push(serde_json::to_value(entity).unwrap());
        }
    }
    ids
}

fn prompt(message: &str) -> Option<String> {
    print!("{message} ");
    io::stdout().flush().ok()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok()?;
    let s = buf.trim();
    if s.is_empty() { None } else { Some(s.to_string()) }
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace("\\", "/")
}

fn ensure_person(graph: &mut Vec<serde_json::Value>, person: PersonEntity) -> serde_json::Value {
    if !graph.iter().any(|v| v.get("@id").and_then(|x| x.as_str()) == Some(&person.id)) {
        // Add affiliation if present
        if let Some(aff) = &person.affiliation {
            let oid = aff.get("@id").and_then(|v| v.as_str()).unwrap();
            if !graph.iter().any(|v| v.get("@id").and_then(|x| x.as_str()) == Some(oid)) {
                let org_entity = OrganizationEntity {
                    id: oid.to_string(),
                    type_: "Organization".to_string(),
                    name: aff.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                };
                graph.push(serde_json::to_value(&org_entity).unwrap());
            }
        }
        // Push the person entity itself
        graph.push(serde_json::to_value(&person).unwrap());
    }
    serde_json::json!({ "@id": person.id })
}


pub fn extract_persons(yaml: &YamlValue, graph: &mut Vec<serde_json::Value>,
    keys: &[&str], class_filter: Option<&str>) -> Option<Vec<serde_json::Value>> {
    let top_map = yaml.as_mapping()?;
    let value = keys.iter()
        .find_map(|k| top_map.get(YamlValue::String(k.to_string())))?;
    let entries = value.as_sequence().cloned().unwrap_or_else(|| vec![value.clone()]);
    let mut refs = Vec::new();
    for entry in entries {
        let map = match entry { YamlValue::Mapping(m) => m, _ => continue };
        if class_filter.is_some_and(|f| map.get(YamlValue::String("class".into()))
            .and_then(|v| v.as_str()) != Some(f)) { continue }
        let get_str = |keys: &[&str]| keys.iter()
            .find_map(|k| map.get(YamlValue::String(k.to_string())))
            .and_then(|v| v.as_str().map(|s| s.trim().trim_matches('"').to_string()));
        let given = get_str(&["arc:first name"]);
        let family = get_str(&["arc:last name"]);
        let name = get_str(&["s:name","name"]);
        let email = get_str(&["s:email","email","arc:email"])
            .map(|s| s.strip_prefix("mailto:").unwrap_or(&s).to_string());
        let address = get_str(&["arc:address"]);
        let affiliation = get_str(&["s:affiliation","affiliation","organization","arc:affiliation"])
            .map(|org| {
                let oid = format!("#Organization_{}", org.replace(' ', "_"));
                if !graph.iter().any(|v| v["@id"] == oid) {
                    graph.push(serde_json::to_value(OrganizationEntity {
                        id: oid.clone(), type_: "Organization".into(), name: org,
                    }).unwrap());
                }
                serde_json::json!({ "@id": oid })
            });
        let job_title = map.get(YamlValue::String("arc:has role".into()))
            .and_then(|roles| roles.as_sequence().unwrap_or(&vec![roles.clone()]).iter().find_map(|r| {
                r.as_mapping().and_then(|rm| {
                    rm.get(YamlValue::String("arc:term accession".into()))?.as_str().map(|id| {
                        let id = id.to_string();
                        let name = rm.get(YamlValue::String("arc:annotation value".into()))
                            .and_then(|v| v.as_str()).unwrap_or(&id).to_string();
                        if !graph.iter().any(|v| v["@id"] == id) {
                            graph.push(serde_json::to_value(RoleEntity {
                                id: id.clone(), type_: "DefinedTerm".into(),
                                name, term_code: id.clone(),
                            }).unwrap());
                        }
                        serde_json::json!({ "@id": id })
                    })
                })
            }));
        let pid = name.as_ref()
            .map(|n| format!("#Person_{}", n.replace(' ', "_")))
            .unwrap_or_else(|| format!("#Person_{}_{}", given.clone().unwrap_or_default(), family.clone().unwrap_or_default()).replace(' ', "_"));
        refs.push(ensure_person(graph, PersonEntity {
            id: pid, type_: "Person".into(),
            given_name: given, family_name: family,
            additional_name: None, email,
            affiliation, job_title, address,
        }));
    }
    (!refs.is_empty()).then_some(refs)
}

pub fn extract_creator(yaml: &YamlValue, graph: &mut Vec<serde_json::Value>) -> Option<Vec<serde_json::Value>> {
    extract_persons(yaml, graph, &["s:creator", "creator", "s:author", "author", "arc:performer"], None)
}

pub fn extract_performer(yaml: &YamlValue, graph: &mut Vec<serde_json::Value>) -> Option<Vec<serde_json::Value>> {
    extract_persons(yaml, graph, &["arc:performer"], Some("arc:Person"))
}

fn git_creator(graph: &mut Vec<serde_json::Value>) -> Option<Vec<serde_json::Value>> {
    if let Ok(repo) = Repository::discover(".") {
        if let Ok(config) = repo.config() {
            let name = config.get_string("user.name").ok().unwrap_or_default();
            let email = config.get_string("user.email").ok().unwrap_or_default();
            let person_entity = PersonEntity {
                id: format!("#Person_{}", name.replace(' ', "_")),
                type_: "Person".to_string(),
                given_name: Some(name),
                family_name: None,
                additional_name: None,
                email: Some(email),
                affiliation: None,
                job_title: None,
                address: None,
            };
            return Some(vec![ensure_person(graph, person_entity)]);
        }
    }
    None
}

pub fn extract_parameter_values(yaml: &YamlValue, graph: &mut Vec<Value>) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(process_seq) = yaml.get("arc:has process sequence").and_then(|v| v.as_sequence()) {
        for process in process_seq {
            if let Some(param_values) = process.get("arc:has parameter value").and_then(|v| v.as_sequence()) {
                for param_value in param_values {
                    let param = param_value.get("arc:has parameter").and_then(|v| v.as_sequence()).and_then(|seq| seq.first());
                    let param_name = param.and_then(|p| p.get("arc:has parameter name")).and_then(|v| v.as_sequence()).and_then(|seq| seq.first());
                    let name_value = param_name.and_then(|pn| pn.get("arc:annotation value")).and_then(|v| v.as_str()).unwrap_or("unknown").to_lowercase();
                    let property_id = param_name.and_then(|pn| pn.get("arc:term accession")).and_then(|v| v.as_str()).unwrap_or("");
                    if let Some(value_entries) = param_value.get("arc:value").and_then(|v| v.as_sequence()) {
                        for value_entry in value_entries {
                            let value_str = value_entry.get("arc:annotation value").and_then(|v| v.as_str()).unwrap_or("");
                            let value_ref = value_entry.get("arc:term accession").and_then(|v| v.as_str()).unwrap_or("");
                            let id = format!("#ParameterValue_{}_{}", name_value.replace(' ', "_"), value_str.replace(' ', "_"));
                            let entity = serde_json::to_value(PropertyValueEntity {
                                id: id.clone(),
                                type_: "PropertyValue".to_string(),
                                additional_type: Some("ParameterValue".to_string()),
                                name: Some(name_value.clone()),
                                value: Some(value_str.to_string()),
                                property_id: if property_id.is_empty() { None } else { Some(property_id.to_string()) },
                                value_reference: if value_ref.is_empty() { None } else { Some(value_ref.to_string()) },
                                column_index: None,
                            }).unwrap();
                            if !graph.iter().any(|v| v.get("@id").and_then(|s| s.as_str()) == Some(id.as_str())) {
                                graph.push(entity);
                            }
                            ids.push(id);
                        }
                    }
                }
            }
        }
    }
    ids
}

fn extract_has_part(cwl_path: &Path, yaml: &serde_yaml::Value) -> Option<Vec<String>> {
    let mut files: HashSet<String> = HashSet::new();
    files.insert(cwl_path.to_string_lossy().to_string());
    if yaml.get("class").and_then(|v| v.as_str()) == Some("Workflow") {
        if let Some(steps) = yaml.get("steps").and_then(|v| v.as_sequence()) {
            let main_folder = cwl_path.parent().unwrap_or(Path::new(""));
            steps.iter()
                .filter_map(|step| step.get("run").and_then(|v| v.as_str()))
                .for_each(|run_val| {
                    let run_path = main_folder.join(run_val)
                        .canonicalize()
                        .unwrap_or_else(|_| main_folder.join(run_val));
                    files.insert(run_path.to_string_lossy().to_string());
                });
        }
    }
    let has_part_vec: Vec<String> = files.into_iter().map(|f| {
        let f_norm = f.replace("\\", "/");
        let start_pos = f_norm.find("workflows/").unwrap_or(0);
        let trimmed = &f_norm[start_pos..];
        let mut parts: Vec<&str> = trimmed.split('/').collect();
        if parts.len() > 3 && parts[1] == parts[2] {
            parts.remove(2);
        }
        parts.join("/")
    }).collect();
    (!has_part_vec.is_empty()).then_some(has_part_vec)
}

pub fn cwl_to_arc_workflow(cwl_path: &Path, graph: &mut Vec<Value>,yaml: &YamlValue) -> Option<ArcWorkflow> {
    let main_entity = normalize_path(cwl_path);
    let identifier = Path::new(&main_entity).file_stem()?.to_string_lossy().to_string();
    let creator: Vec<serde_json::Value> = 
        extract_creator(yaml, graph).into_iter().flatten()
        .chain(extract_performer(yaml, graph).into_iter().flatten())
        .collect();
    Some(ArcWorkflow {
        id: Path::new(&main_entity)
            .parent()
            .map(|p| format!("{}/", p.to_string_lossy()))?,
        type_: "Dataset".to_string(),
        additional_type: "Workflow".to_string(),
        identifier,
        main_entity: MainEntity { id: main_entity },
        name: yaml
            .get("label")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| prompt("Enter a workflow name:")),
        description: yaml
            .get("doc")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| prompt("Enter a workflow description:")),
        has_part: extract_has_part(cwl_path, yaml),
        url: None,
        creator: if creator.is_empty() { None } else { Some(creator) },
    })
}

pub fn cwl_to_arc_run(cwl_path: &Path, cwl_rel_path: &Path, raw_inputs: &[String],
    foldername: &str, yaml: &YamlValue, graph: &mut Vec<Value>) -> Option<ArcRun> {
    let id = format!("runs/{foldername}/");
    let wf_name = cwl_rel_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("Workflow")
        .to_string();
    let mut has_part = vec![normalize_path(cwl_rel_path)];
    let base_dir = cwl_path.parent().unwrap_or(Path::new("")).to_path_buf();
    let collected_inputs = extract_inputs(raw_inputs, cwl_path, yaml);
    let mut dataset_id = String::new();
    collected_inputs.iter().for_each(|input| {
        let input_path = Path::new(input);
        if input_path.is_file() {
            let rel = input_path.strip_prefix(&base_dir)
                .unwrap_or(input_path)
                .to_string_lossy()
                .to_string();
            has_part.push(rel);
        }
        if let Some(parent) = input_path.parent() {
            let parent_str = normalize_path(parent);
            if !graph.iter().any(|v| v.get("@id").and_then(|s| s.as_str()) == Some(&parent_str)) {
                let identifier = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");
                let latest_modified = fs::read_dir(parent)
                    .ok()
                    .into_iter()
                    .flat_map(|entries| entries.flatten())
                    .filter_map(|e| e.metadata().ok())
                    .filter_map(|m| m.modified().ok())
                    .max()
                    .unwrap_or_else(|| chrono::Utc::now().into());
                let additional_type = if parent_str.contains("assays") { "Assay" } else { "Study" };
                if let Some(idx) = parent_str.find("studies/").or_else(|| parent_str.find("assays/")) {
                    let after = &parent_str[idx..];
                    let parts = after.split('/').collect::<Vec<_>>();
                    if parts.len() > 1 && !parts[1].is_empty() {
                        dataset_id = format!("{}/{}/", parts[0], parts[1]);
                    } else {
                        dataset_id = parts[0].to_string();
                    }
                } else {
                    dataset_id = parent_str.clone();
                }
                let dataset_entity = json!({
                    "@id": dataset_id,
                    "@type": "Dataset",
                    "additionalType": additional_type,
                    "identifier": dataset_id.trim_end_matches('/').split('/').next_back().unwrap_or(identifier),
                    "dateModified": chrono::DateTime::<chrono::Utc>::from(latest_modified).to_rfc3339()
                });
                graph.push(dataset_entity);
            }
        }
    });
    // Add Investigation Dataset to graph?
    let investigation_entity = json!({
        "@id": "./",
        "@type": "Dataset",
        "additionalType": "Investigation",
        "identifier": wf_name,
        "datePublished": data_created(".")
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
            .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339()),
        "hasPart": [
            { "@id": id },
            { "@id": dataset_id },
            { "@id": format!("workflows/{}/", wf_name) }
        ],
        "name": wf_name,
        "license": "ALL RIGHTS RESERVED BY THE AUTHORS"
    });
    graph.push(investigation_entity);
    let outputs = find_output_files(yaml, &base_dir);
    has_part.extend(outputs.into_iter().map(|o| Path::new(foldername).join(o).to_string_lossy().to_string()));
    let conforms_to = ensure_creativeworks(graph);
    Some(ArcRun {
        id,
        type_: "Dataset".to_string(),
        additional_type: "Run".to_string(),
        identifier: foldername.to_string(),
        name: prompt("Enter a name of the ARC Run:"),
        description: prompt("Enter a description of the ARC Run:"),
        about: Some(json!({ "@id": format!("#WorkflowInvocation_{}_{}_0", foldername, wf_name) })),
        mentions: Some(json!({ "@id": format!("#WorkflowInvocation_{}_{}_0", foldername, wf_name) })),
        creator: git_creator(graph),
        has_part: Some(has_part.into_iter().map(|p| json!({ "@id": p })).collect()),
        measurement_method: None,
        measurement_technique: None,
        conforms_to: Some(conforms_to),
        url: None,
        variable_measured: None,
    })
}


pub fn extract_inputs(raw_inputs: &[String], cwl_path: &Path,yaml: &serde_yaml::Value) -> Vec<String> {
    // Collect workflow input IDs
    let workflow_inputs: Vec<_> = yaml
        .get("inputs")
        .and_then(|v| v.as_sequence())
        .into_iter()
        .flatten()
        .filter_map(|i| i.get("id").and_then(|id| id.as_str()))
        .map(|s| s.trim_start_matches('#').to_string())
        .collect();
    let mut collected = Vec::new();
    // inputs.yml
    if let Some(yml_path) = raw_inputs.iter().find(|r| r.ends_with(".yml")) {
        if let Ok(yml_str) = fs::read_to_string(yml_path) {
            if let Ok(yml) = serde_yaml::from_str::<HashMap<String, serde_yaml::Value>>(&yml_str) {
                collected.extend(workflow_inputs.iter().filter_map(|id| {
                    yml.get(id)
                        .and_then(|e| e.get("location"))
                        .and_then(|l| l.as_str())
                        .map(|s| s.to_string())
                }));
            }
        }
    }
    // CLI flags
    collected.extend(
        raw_inputs
            .windows(2)
            .filter_map(|w| match w {
                [flag, val] if flag.starts_with("--") => {
                    workflow_inputs
                        .iter()
                        .any(|id| id == flag.trim_start_matches("--"))
                        .then(|| val.clone())
                }
                _ => None,
            }),
    );
    // Defaults from step CWLs (only if nothing found yet)
    if collected.is_empty() {
        collected.extend(
            yaml.get("steps")
                .and_then(|v| v.as_sequence())
                .into_iter()
                .flatten()
                .filter_map(|step| step.get("run").and_then(|v| v.as_str()))
                .map(|run| cwl_path.parent().unwrap_or(Path::new("")).join(run))
                .filter_map(|p| fs::read_to_string(&p).ok())
                .filter_map(|s| serde_yaml::from_str::<serde_yaml::Value>(&s).ok())
                .flat_map(|step_yaml| {
                    step_yaml
                        .get("inputs")
                        .and_then(|v| v.as_sequence())
                        .into_iter()
                        .flatten()
                        .filter_map(|input| {
                            input.get("id").and_then(|v| v.as_str()).map(|id| (id, input))
                        })
                        .filter(|(id, _)| workflow_inputs.contains(&id.trim_start_matches('#').to_string()))
                        .filter_map(|(_, input)| {
                            input.get("default")
                                .and_then(|d| d.get("location").and_then(|l| l.as_str()).or_else(|| d.as_str()))
                                .map(|s| s.to_string())
                        })
                        .collect::<Vec<_>>()
                }),
        );
    }
    collected
}

pub fn data_created(file_path: &str) -> io::Result<SystemTime> {
    let path = Path::new(file_path);
    let metadata = fs::metadata(path)?;
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        let created = metadata.creation_time();
        let duration_since_windows_epoch = std::time::Duration::from_nanos(created * 100);
        let windows_epoch = SystemTime::UNIX_EPOCH
            .checked_sub(std::time::Duration::from_secs(11644473600))
            .unwrap();
        Ok(windows_epoch + duration_since_windows_epoch)
    }
    #[cfg(target_os = "macos")]
    {
        metadata.created()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let ctime = metadata.ctime();
        if ctime > 0 {
            Ok(SystemTime::UNIX_EPOCH + std::time::Duration::new(ctime as u64, 0))
        } else {
            Err(io::Error::other("Creation time not available"))
        }
    }
}

fn load_tool_yaml(base_path: &Path, run_path: &str) -> Option<YamlValue> {
    let tool_path: PathBuf = base_path.join(run_path)
        .canonicalize()
        .unwrap_or_else(|_| base_path.join(run_path));
    fs::read_to_string(&tool_path).ok()
        .and_then(|s| serde_yaml::from_str(&s).ok())
}

fn extract_tool_output(tool_yaml: &YamlValue, step_out: &str) -> Option<String> {
    let tool_outputs = tool_yaml.get("outputs")?.as_sequence()?;
    let output = tool_outputs.iter()
        .find(|o| o.get("id").and_then(|v| v.as_str()) == Some(step_out))?;

    let glob = output.get("outputBinding")
        .and_then(|b| b.get("glob"))
        .and_then(|g| g.as_str())?;

    if glob.starts_with("$(inputs.") {
        let param = glob.trim_start_matches("$(inputs.").trim_end_matches(')');
        let inputs = tool_yaml.get("inputs")?.as_sequence()?;
        let default = inputs.iter()
            .find(|i| i.get("id").and_then(|id| id.as_str()) == Some(param))?
            .get("default")?;

        default.as_str().map(|s| s.to_string())
    } else {
        Some(glob.to_string())
    }
}

pub fn find_output_files(workflow_yaml: &YamlValue, base_path: &Path) -> Vec<String> {
    let outputs = workflow_yaml.get("outputs").and_then(|v| v.as_sequence());
    let steps = workflow_yaml.get("steps").and_then(|v| v.as_sequence());
    let (Some(outputs), Some(steps)) = (outputs, steps) else {
        return Vec::new();
    };
    outputs.iter().filter_map(|output| {
        let output_source = output.get("outputSource")?.as_str()?;
        let mut parts = output_source.split('/');
        let (step_id, step_out) = (parts.next()?, parts.next()?);

        let step = steps.iter()
            .find(|st| st.get("id").and_then(|v| v.as_str()) == Some(step_id))?;
        let run_path = step.get("run").and_then(|r| r.as_str())?;

        let tool_yaml = load_tool_yaml(base_path, run_path)?;
        extract_tool_output(&tool_yaml, step_out)
    }).collect()
}

fn detect_language(id: &str) -> (Option<String>, Option<ComputerLanguageEntity>) {
    if id.contains("cwl") {
        let lang_id = "https://w3id.org/workflowhub/workflow-ro-crate#cwl".to_string();
        let entity = ComputerLanguageEntity {
            id: lang_id.clone(),
            type_: "ComputerLanguage".to_string(),
            name: Some("Common Workflow Language".to_string()),
            alternate_name: Some("CWL".to_string()),
            identifier: Some(json!({ "@id": "https://w3id.org/cwl/v1.2/" })),
            url: Some(json!({ "@id": "https://www.commonwl.org/" })),
        };
        (Some(lang_id), Some(entity))
    } else {
        (None, None)
    }
}

pub fn cwl_to_invocation(cwl_path: &Path,cwl_rel_path: &Path, yaml: &serde_yaml::Value,
    foldername: &str, raw_inputs: &[String], graph: &mut Vec<Value>,) -> Option<WorkflowInvocation> {
    let name = Path::new(foldername).file_name().and_then(|s| s.to_str()).unwrap_or(foldername).to_string();
    let wf_name = cwl_rel_path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()).unwrap_or("Workflow").to_string();
    let id = format!("#WorkflowInvocation_{foldername}_{wf_name}_0");
    let type_ = vec![
        "https://bioschemas.org/CreateAction".to_string(),
        "LabProcess".to_string(),
    ];
    let inputs = extract_inputs(raw_inputs, cwl_rel_path, yaml);
    let base_dir = cwl_path.parent().unwrap_or(Path::new(""));
    let outputs = find_output_files(yaml, base_dir);

    let push_file_entity = |graph: &mut Vec<Value>, file_id: &str, property_key: &str, property_val: &str| {
        if !graph.iter().any(|v| v.get("@id").and_then(|s| s.as_str()) == Some(file_id)) {
            let file_entity = json!({
                "@id": file_id,
                "@type": "File",
                "name": file_id,
                property_key: { "@id": property_val }
            });
            graph.push(file_entity);
        }
        json!({ "@id": file_id })
    };
    let input_entities: Vec<Value> = inputs
        .iter()
        .enumerate()
        .map(|(i, input)| push_file_entity(graph, input, "exampleOfWork", &format!("#FormalParameter_W_In_{wf_name}_{i}")))
        .collect();
    let output_entities: Vec<Value> = outputs
        .iter()
        .enumerate()
        .map(|(i, output)| push_file_entity(graph, output, "additionalProperty", &format!("#FactorValue_W_Out_{wf_name}_{i}")))
        .collect();
    let parameter_value = extract_parameter_values(yaml, graph);
    Some(WorkflowInvocation {
        id,
        type_,
        additional_type: "WorkflowInvocation".to_string(),
        instrument: vec![json!({ "@id": cwl_rel_path.to_string_lossy() })],
        executes_lab_protocol: json!({ "@id": cwl_rel_path.to_string_lossy() }),
        result: output_entities,
        object: input_entities,
        name,
        parameter_value: Some(vec![json!({ "@id": parameter_value })]),
        description: None,
    })
}

pub fn cwl_to_workflow_protocol(cwl_path: &Path, cwl_rel_path: &Path, args: &[String],
    graph: &mut Vec<Value>, yaml: &serde_yaml::Value) -> Option<WorkflowProtocol> {
    let _type = vec![
        "File".to_string(),
        "ComputationalWorkflow".to_string(),
        "SoftwareSourceCode".to_string(),
        "LabProtocol".to_string(),
    ];
    let id = cwl_rel_path.to_string_lossy().to_string();
    let wf_name = Path::new(&id).parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()).unwrap_or("Workflow");
    let inputs = extract_inputs(args, cwl_rel_path, yaml);
    let base_dir = cwl_path.parent().unwrap_or(Path::new(""));
    let outputs = find_output_files(yaml, base_dir);
    let param_sources = if args.is_empty() { inputs.clone() } else { args.to_vec() };
    let push_formal_parameter = |graph: &mut Vec<Value>, wf_name: &str, arg: &str, position: usize| -> Value {
        let param_name = arg.trim_start_matches('-');
        let position_id = format!("#FormalParameter_W_{wf_name}_{param_name}_position");
        let prefix_id = format!("#FormalParameter_W_{wf_name}_{param_name}_prefix");
        let formal_param_id = format!("#FormalParameter_W_{wf_name}_{param_name}");
        graph.push(json!({ "@id": position_id, "@type": "PropertyValue", "name": "Position", "value": position }));
        graph.push(json!({ "@id": prefix_id, "@type": "PropertyValue", "name": "Prefix", "value": arg }));
        graph.push(json!({ "@id": formal_param_id, "@type": "FormalParameter", "name": param_name, "identifier": [{ "@id": position_id }, { "@id": prefix_id }] }));
        Value::String(formal_param_id)
    };
    let input_ids: Vec<Value> = param_sources.iter()
        .enumerate()
        .filter(|(_, arg)| arg.starts_with('-'))
        .map(|(i, arg)| push_formal_parameter(graph, wf_name, arg, i))
        .collect();
    let output_ids: Vec<Value> = outputs.iter()
        .map(|output| {
            let output_name = Path::new(output)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(output);
            let output_id = format!("#FormalParameter_W_{wf_name}_{output_name}");
            graph.push(json!({ "@id": output_id, "@type": "FormalParameter", "additionalType": "File", "name": output_name }));
            Value::String(output_id)
        })
        .collect();
    let (programming_language, computer_language_entity) = detect_language(&id);
    if let Some(entity) = computer_language_entity {
        graph.push(serde_json::to_value(entity).unwrap());
    }
    let license = yaml.as_mapping()
        .and_then(|map| map.iter().find_map(|(k, v)| {
            k.as_str().filter(|s| s.ends_with(":license"))
                .and_then(|_| v.as_str().map(|s| vec![s.to_string()]))
        }));
    let date_created = data_created(&id).ok().map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
    let creator: Vec<Value> = extract_creator(yaml, graph).into_iter().flatten()
        .chain(extract_performer(yaml, graph).into_iter().flatten())
        .collect();
    Some(WorkflowProtocol {
        context: "ComputationalWorkflow".to_string(),
        type_: _type,
        additional_type: "WorkflowProtocol".to_string(),
        id,
        input: Some(input_ids.into_iter().map(|id| json!({ "@id": id.as_str().unwrap_or_default() })).collect()),
        output: Some(output_ids.into_iter().map(|id| json!({ "@id": id.as_str().unwrap_or_default() })).collect()),
        dct_conforms_to: Some("https://bioschemas.org/profiles/ComputationalWorkflow/1.0-RELEASE".to_string()),
        creator: if creator.is_empty() { None } else { Some(creator) },
        date_created,
        license,
        name: yaml.get("label").or_else(|| yaml.get("name")).and_then(|v| v.as_str()).map(String::from),
        programming_language: programming_language.map(|lang| vec![json!({ "@id": lang })]),
        sd_publisher: Some("SciWIn".to_string()),
        url: None,
        version: None,
        description: yaml.get("doc").and_then(|v| v.as_str()).map(String::from),
        has_part: extract_has_part(cwl_path, yaml),
        intended_use: None,
        comment: yaml.get("s:comment").and_then(|v| v.as_str()).map(|s| vec![s.to_string()]),
        computational_tool: None,
    })
}

pub fn workflow_cwl_to_arc_rocrate(cwl_path: &Path, cwl_rel_path: &Path, raw_inputs: &[String], 
    folder: &str, cwl_yaml: &serde_yaml::Value) -> ArcRoCrate {
    let context = vec![
        json!("https://w3id.org/ro/crate/1.1/context"),
        json!({
            "Sample": "https://bioschemas.org/Sample",
            "additionalProperty": "http://schema.org/additionalProperty",
            "intendedUse": "https://bioschemas.org/intendedUse",
            "computationalTool": "https://bioschemas.org/computationalTool",
            "labEquipment": "https://bioschemas.org/labEquipment",
            "reagent": "https://bioschemas.org/reagent",
            "LabProtocol": "https://bioschemas.org/LabProtocol",
            "executesLabProtocol": "https://bioschemas.org/executesLabProtocol",
            "parameterValue": "https://bioschemas.org/parameterValue",
            "LabProcess": "https://bioschemas.org/LabProcess",
            "measurementMethod": "http://schema.org/measurementMethod",
            "FormalParameter": "https://bioschemas.org/FormalParameter",
            "ComputationalWorkflow": "https://bioschemas.org/ComputationalWorkflow",
            "SoftwareSourceCode": "http://schema.org/SoftwareSourceCode",
            "input": "https://bioschemas.org/input",
            "output": "https://bioschemas.org/output"
        }),
    ];
    let mut graph: Vec<Value> = Vec::new();
    let workflow = cwl_to_arc_workflow(cwl_rel_path, &mut graph, cwl_yaml);
    let run = cwl_to_arc_run(cwl_path, cwl_rel_path, raw_inputs, folder, cwl_yaml, &mut graph);
    let invocation = cwl_to_invocation(cwl_path, cwl_rel_path, cwl_yaml, folder, raw_inputs, &mut graph);
    let (workflow_creator, workflow_name) = if let Some(ref w) = workflow {
        (w.creator.clone(), w.name.clone())
    } else { (None, None) };
    let mut workflow_protocol = cwl_to_workflow_protocol(cwl_path, cwl_rel_path, raw_inputs, &mut graph, cwl_yaml);
    if let Some(ref mut protocol) = workflow_protocol {
        if protocol.creator.is_none() { protocol.creator = workflow_creator; }
        if protocol.name.is_none() { protocol.name = workflow_name; }
    }
    if let Some(w) = workflow { graph.push(serde_json::to_value(w).unwrap()); }
    if let Some(r) = run { graph.push(serde_json::to_value(r).unwrap()); }
    if let Some(i) = invocation { graph.push(serde_json::to_value(i).unwrap()); }
    if let Some(p) = workflow_protocol { graph.push(serde_json::to_value(p).unwrap()); }
    let root_data_entity = RootDataEntity {
        id: "ro-crate-metadata.json".to_string(),
        type_: "CreativeWork".to_string(),
        conforms_to: Some(vec![
            json!({ "@id": "https://w3id.org/ro/crate/1.1" }),
            json!({ "@id": "https://w3id.org/workflowhub/workflow-ro-crate/1.0" }),
        ]),
        about: Some(json!({ "@id": "./" })),
    };
    graph.push(serde_json::to_value(root_data_entity).unwrap());
    let rocrate = ArcRoCrate { context, graph };
    std::fs::create_dir_all(folder).ok();
    let output_path = format!("{folder}/ro-crate-metadata.json");
    let _ = write_arc_rocrate_metadata(&rocrate, &output_path);
    rocrate
}

pub fn write_arc_rocrate_metadata(rocrate: &ArcRoCrate, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, rocrate)?;
    Ok(())
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value as YamlValue;
    use serde_json::Value as JsonValue;
    use std::path::Path;

    fn sample_yaml_person() -> YamlValue {
        serde_yaml::from_str(r#"
s:creator:
  - s:name: "Alice Example"
    s:email: "mailto:alice@example.com"
    s:affiliation: "Example Org"
    arc:has role:
      - arc:term accession: "R1"
        arc:annotation value: "Researcher"
"#).unwrap()
    }

    fn sample_yaml_workflow() -> YamlValue {
        serde_yaml::from_str(r#"
label: "Test Workflow"
doc: "A test workflow description."
class: "Workflow"
steps:
  - id: "step1"
    run: "tool.cwl"
inputs:
  - id: "#input1"
outputs:
  - id: "out1"
    outputSource: "step1/out"
"#).unwrap()
    }

    #[test]
    fn test_ensure_creativeworks_adds_missing_entities() {
        let mut graph = Vec::new();
        let ids = ensure_creativeworks(&mut graph);
        assert_eq!(ids.len(), CREATIVE_WORKS.len());
        for id in ids {
            assert!(graph.iter().any(|v| v["@id"] == id["@id"]));
        }
    }

    #[test]
    fn test_ensure_person_adds_person_and_affiliation() {
        let mut graph = Vec::new();
        let person_id = "#Person_Alice".to_string();
        let person_ref = ensure_person(
            &mut graph,
            person_id.clone(),
            Some("Alice".to_string()),
            Some("Example".to_string()),
            Some("alice@example.com".to_string()),
            Some("Example Org".to_string()),
            None,
            Some("123 Street".to_string()),
        );

        assert_eq!(person_ref["@id"], person_id);
        assert!(graph.iter().any(|v| v["@id"] == person_id));
        assert!(graph.iter().any(|v| v["@id"].as_str().unwrap().starts_with("#Organization_")));
    }

    #[test]
    fn test_extract_creator_returns_person_ref() {
        let yaml = sample_yaml_person();
        let mut graph = Vec::new();
        let creators = extract_creator(&yaml, &mut graph).unwrap();
        assert_eq!(creators.len(), 1);
        assert!(graph.iter().any(|v| v["@id"] == creators[0]["@id"]));
    }

    #[test]
    fn test_extract_performer_filters_class() {
        let yaml = sample_yaml_person();
        let mut graph = Vec::new();
        let performers = extract_performer(&yaml, &mut graph);
        assert!(performers.is_none()); // class is not "arc:Person", so should be skipped
    }

    #[test]
    fn test_extract_parameter_values_creates_entities() {
        let yaml: YamlValue = serde_yaml::from_str(r#"
arc:has process sequence:
  - arc:has parameter value:
      - arc:has parameter:
          - arc:has parameter name:
              - arc:annotation value: "param1"
                arc:term accession: "PV1"
        arc:value:
          - arc:annotation value: "value1"
"#).unwrap();

        let mut graph = Vec::new();
        let ids = extract_parameter_values(&yaml, &mut graph);
        assert_eq!(ids.len(), 1);
        assert!(graph.iter().any(|v| v["@id"] == ids[0]));
    }

    #[test]
    fn test_cwl_to_arc_workflow_creates_workflow() {
        let yaml = sample_yaml_workflow();
        let mut graph = Vec::new();
        let workflow = cwl_to_arc_workflow(Path::new("workflows/test.cwl"), &mut graph, &yaml).unwrap();
        assert_eq!(workflow.identifier, "test");
        assert_eq!(workflow.name.as_deref(), Some("Test Workflow"));
        assert_eq!(workflow.description.as_deref(), Some("A test workflow description."));
    }


    #[test]
    fn test_workflow_cwl_to_arc_rocrate_creates_graph() {
        let yaml = sample_yaml_workflow();
        let folder = "test_rocrate";
        let raw_inputs: Vec<String> = vec![];
        let rocrate = workflow_cwl_to_arc_rocrate(Path::new("workflows/test.cwl"), "contents", Path::new("workflows/test.cwl"), &raw_inputs, folder, &yaml);
        // The RO-Crate should contain @graph
        assert!(!rocrate.graph.is_empty());
        // Should include the RootDataEntity
        assert!(rocrate.graph.iter().any(|v| v.get("@id").map(|id| id == "ro-crate-metadata.json").unwrap_or(false)));
    }

     fn sample_cwl_yaml() -> YamlValue {
        serde_yaml::from_str(r#"
class: Workflow
label: "Test Workflow"
doc: "A workflow for testing."
steps:
  - id: "step1"
    run: "tool.cwl"
inputs:
  - id: "#input1"
outputs:
  - id: "out1"
    outputSource: "step1/out"
"#).unwrap()
    }

    #[test]
    fn test_cwl_to_arc_run_creates_run_entity() {
        let yaml = sample_cwl_yaml();
        let cwl_path = Path::new("workflows/test.cwl");
        let cwl_rel_path = Path::new("workflows/test.cwl");
        let raw_inputs = vec!["--input1".to_string(), "file1.txt".to_string()];
        let foldername = "run_test";
        let mut graph: Vec<JsonValue> = Vec::new();

        let run = cwl_to_arc_run(cwl_path, cwl_rel_path, &raw_inputs, foldername, &yaml, &mut graph)
            .expect("Failed to create ArcRun");

        // Check basic properties
        assert_eq!(run.id, format!("runs/{}/", foldername));
        assert_eq!(run.type_, "Dataset");
        assert_eq!(run.additional_type, "Run");

        // The has_part should contain at least the CWL file
        assert!(run.has_part.as_ref().unwrap().iter()
            .any(|v| v["@id"].as_str().unwrap().contains("workflows/test.cwl")));

        // The conforms_to field should contain all three creative works
        let conforms = run.conforms_to.as_ref().unwrap();
        assert_eq!(conforms.len(), CREATIVE_WORKS.len());
    }

    #[test]
    fn test_cwl_to_invocation_creates_invocation_entity() {
        let yaml = sample_cwl_yaml();
        let cwl_path = Path::new("workflows/test.cwl");
        let cwl_rel_path = Path::new("workflows/test.cwl");
        let raw_inputs = vec!["--input1".to_string(), "file1.txt".to_string()];
        let foldername = "inv_test";
        let mut graph: Vec<JsonValue> = Vec::new();

        let invocation = cwl_to_invocation(cwl_path, cwl_rel_path, &yaml, foldername, &raw_inputs, &mut graph)
            .expect("Failed to create WorkflowInvocation");

        // Check id and type
        assert!(invocation.id.contains("WorkflowInvocation"));
        assert!(invocation.type_.contains(&"LabProcess".to_string()));

        // The result and object arrays should contain file references
        assert!(!invocation.result.is_empty());
        assert!(!invocation.object.is_empty());

        // Parameter values should be included if defined
        let param_values = invocation.parameter_value.as_ref().unwrap();
        assert!(!param_values.is_empty());
    }

}
    */