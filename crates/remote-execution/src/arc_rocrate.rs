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

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace("\\", "/")
}

fn ensure_person(graph: &mut Vec<serde_json::Value>, person: &PersonEntity) -> serde_json::Value {
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
        graph.push(serde_json::to_value(person).unwrap());
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
        let name = get_str(&["s:name","name"]);
        let mut given = get_str(&["arc:first name"]);
        let mut family = get_str(&["arc:last name"]);
        // If name exists but given/family do not, try to split name
        if name.is_some() && given.is_none() && family.is_none() {
            let parts: Vec<&str> = name.as_ref().unwrap().split_whitespace().collect();
            if parts.len() >= 2 {
            given = Some(parts[0].to_string());
            family = Some(parts[1..].join(" "));
            } else if parts.len() == 1 {
            given = Some(parts[0].to_string());
            }
        }
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
        refs.push(ensure_person(graph, &PersonEntity {
            id: pid, type_: "Person".into(),
            given_name: given, family_name: family,
            additional_name: None, email,
            affiliation, job_title, address,
        }));
    }
    (!refs.is_empty()).then_some(refs)
}

pub fn extract_creator(yaml: &YamlValue, graph: &mut Vec<serde_json::Value>) -> Option<Vec<serde_json::Value>> {
    extract_persons(yaml, graph, &["s:creator", "creator", "s:author", "author"], None)
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
            return Some(vec![ensure_person(graph, &person_entity)]);
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

fn normalize_path(base: &Path, relative: &str) -> PathBuf {
    let mut components: Vec<&str> = base.components()
        .map(|c| c.as_os_str().to_str().unwrap())
        .collect();

    for part in relative.split('/') {
        match part {
            ".." => { components.pop(); },
            "." => {},
            p => components.push(p),
        }
    }

    components.iter().collect()
}

fn extract_has_part(cwl_path: &Path, yaml: &YamlValue) -> Option<Vec<String>> {
    let mut files: HashSet<String> = HashSet::new();
    files.insert(cwl_path.to_string_lossy().replace("\\", "/"));

    if yaml.get("class").and_then(|v| v.as_str()) == Some("Workflow") {
        if let Some(steps) = yaml.get("steps").and_then(|v| v.as_sequence()) {
            let main_folder = cwl_path.parent().unwrap_or(Path::new(""));

            for step in steps {
                if let Some(run_val) = step.get("run").and_then(|v| v.as_str()) {
                    let run_path = normalize_path(main_folder, run_val);
                    files.insert(run_path.to_string_lossy().replace("\\", "/"));
                }
            }
        }
    }

    let mut has_part_vec: Vec<String> = files.into_iter().map(|f| {
        let start_pos = f.find("workflows/").unwrap_or(0);
        let trimmed = &f[start_pos..];
        trimmed.to_string()
    }).collect();

    has_part_vec.sort();
    (!has_part_vec.is_empty()).then_some(has_part_vec)
}

pub fn cwl_to_arc_workflow(cwl_path: &Path, graph: &mut Vec<Value>,yaml: &YamlValue) -> Option<ArcWorkflow> {
    let main_entity = normalize(cwl_path);
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
    let mut has_part = vec![normalize(cwl_rel_path)];
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
            let parent_str = normalize(parent);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value as YamlValue;
    use serde_json::json;
    use git2::Repository;
    use tempfile::{tempdir, NamedTempFile};
    use std::fs;
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn test_cwl_to_workflow_protocol() {
        let temp_cwl = NamedTempFile::new().unwrap();
        let cwl_path = temp_cwl.path();
        let cwl_rel_path = Path::new("workflows/test_workflow.cwl");
        let args = vec!["--population".to_string(), "data/population.csv".to_string()];

        let yaml_str = r#"
        #!/usr/bin/env cwl-runner
        cwlVersion: v1.2
        class: Workflow
        inputs:
        - id: population
          type: File
        - id: speakers
          type: File
        outputs:
        - id: out
          type: File
          outputSource: plot/results

        steps:
        - id: calculation
          in:
            population: population
            speakers: speakers
          run: '../calculation/calculation.cwl'
          out:
          - results
        - id: plot
          in:
            results: calculation/results
          run: '../plot/plot.cwl'
          out:
          - results
        "#;
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let mut graph: Vec<Value> = Vec::new();
        let workflow_opt = cwl_to_workflow_protocol(cwl_path, cwl_rel_path, &args, &mut graph, &yaml);
        assert!(workflow_opt.is_some());
        let workflow = workflow_opt.unwrap();
        assert_eq!(workflow.context, "ComputationalWorkflow");
        assert_eq!(workflow.additional_type, "WorkflowProtocol");
        assert_eq!(workflow.id, cwl_rel_path.to_string_lossy());

        let input_ids: Vec<String> = workflow.input.unwrap()
            .iter()
            .filter_map(|v| v.get("@id").and_then(|s| s.as_str()).map(|s| s.to_string()))
            .collect();
        assert!(input_ids.iter().any(|id| id.contains("population")));

        let formal_params: Vec<&Value> = graph.iter()
            .filter(|v| v.get("@type").and_then(|t| t.as_str()) == Some("FormalParameter"))
            .collect();
        assert!(!formal_params.is_empty());

        assert!(workflow.programming_language.is_some());
        let lang_id = &workflow.programming_language.unwrap()[0]["@id"];
        assert_eq!(lang_id, "https://w3id.org/workflowhub/workflow-ro-crate#cwl");
    }

    #[test]
    fn test_cwl_to_invocation_with_yaml_only() {
        let temp_cwl = NamedTempFile::new().unwrap();
        let cwl_path = temp_cwl.path();
        let cwl_rel_path = std::path::Path::new("workflows/test_workflow/test_workflow.cwl");
        let mut temp_inputs = NamedTempFile::new().unwrap();
        write!(
            temp_inputs,
            r#"
input1:
  default:
    location: file1.txt
input2:
  default:
    location: file2.txt
"#
        )
        .unwrap();
        let raw_inputs = vec!["--input1".to_string(), "file1.txt".to_string(),
                              "--input2".to_string(), "file2.txt".to_string()];
        let yaml_str = r#"
inputs:
- id: input1
- id: input2

outputs:
- id: output1
  type: File
  outputSource: plot/output1
- id: output2
  type: File
  outputSource: plot/output2
"#;
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let foldername = "run1";
        let mut graph: Vec<Value> = Vec::new();
        let invocation = cwl_to_invocation(cwl_path, cwl_rel_path, &yaml, foldername, &raw_inputs, &mut graph)
            .expect("WorkflowInvocation creation failed");
        assert_eq!(invocation.name, "run1");
        assert_eq!(invocation.id, "#WorkflowInvocation_run1_test_workflow_0");
        assert_eq!(invocation.additional_type, "WorkflowInvocation");
        let input_ids: Vec<String> = invocation.object.iter()
            .filter_map(|v| v.get("@id").and_then(|s| s.as_str()).map(String::from))
            .collect();
        assert!(input_ids.contains(&"file1.txt".to_string()));
        assert!(input_ids.contains(&"file2.txt".to_string()));
        let graph_ids: Vec<String> = graph.iter()
            .filter_map(|v| v.get("@id").and_then(|s| s.as_str()).map(String::from))
            .collect();
        println!("Graph IDs: {:?}", graph_ids);
        assert!(graph_ids.contains(&"file1.txt".to_string()));
        assert!(graph_ids.contains(&"file2.txt".to_string()));
    }

    #[test]
    fn test_detect_language_cwl() {
        let id = "workflows/my_workflow.cwl";
        let (lang_id_opt, entity_opt) = detect_language(id);
        assert!(lang_id_opt.is_some());
        let lang_id = lang_id_opt.unwrap();
        assert_eq!(lang_id, "https://w3id.org/workflowhub/workflow-ro-crate#cwl");
        assert!(entity_opt.is_some());
        let entity = entity_opt.unwrap();
        assert_eq!(entity.id, lang_id);
        assert_eq!(entity.type_, "ComputerLanguage");
        assert_eq!(entity.name.unwrap(), "Common Workflow Language");
        assert_eq!(entity.alternate_name.unwrap(), "CWL");
        assert_eq!(
            entity.identifier.unwrap(),
            json!({ "@id": "https://w3id.org/cwl/v1.2/" })
        );
        assert_eq!(
            entity.url.unwrap(),
            json!({ "@id": "https://www.commonwl.org/" })
        );
    }

    #[test]
    fn test_data_created_returns_valid_time() {
        let temp_file = NamedTempFile::new().unwrap();
        let path_str = temp_file.path().to_string_lossy();
        let created_time = data_created(&path_str).expect("Failed to get creation time");
        let now = SystemTime::now();
        assert!(created_time <= now, "Creation time is in the future");
        assert!(
            now.duration_since(created_time).unwrap() < Duration::from_secs(20),
            "Creation time is too far in the past"
        );
    }

    #[test]
    fn test_data_created_nonexistent_file() {
        let result = data_created("this_file_should_not_exist.txt");
        assert!(result.is_err(), "Expected error for nonexistent file");
    }

    #[test]
    fn test_extract_inputs_from_cli_flags() {
        let workflow_yaml: YamlValue = serde_yaml::from_str(
            r#"
inputs:
  - id: input1
  - id: input2
steps: []
"#,
        )
        .unwrap();
        let raw_inputs = vec![
            "--input1".to_string(),
            "cli_file1.txt".to_string(),
            "--input2".to_string(),
            "cli_file2.txt".to_string(),
        ];

        let temp_file = NamedTempFile::new().unwrap();
        let result = extract_inputs(&raw_inputs, temp_file.path(), &workflow_yaml);
        assert_eq!(result, vec!["cli_file1.txt", "cli_file2.txt"]);
    }

    #[test]
    fn test_cwl_to_arc_run_basic() {
        let cwl_path = Path::new("/home/user/project/workflows/main/main.cwl");
        let cwl_rel_path = Path::new("workflows/main/main.cwl");
        let tmp_dir = tempdir().unwrap();
        let data_dir = tmp_dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let population = data_dir.join("population.csv");
        let speakers = data_dir.join("speakers.csv");
        fs::File::create(&population).unwrap();
        fs::File::create(&speakers).unwrap();

        fs::create_dir_all(cwl_path.parent().unwrap()).unwrap();
        fs::File::create(&cwl_path).unwrap();

        let raw_inputs = vec![
            "--population".to_string(), population.to_string_lossy().to_string(),
            "--speakers".to_string(), speakers.to_string_lossy().to_string()
    ];
        let foldername = "run1";
        let yaml_str = r#"
        #!/usr/bin/env cwl-runner
        cwlVersion: v1.2
        class: Workflow
        inputs:
        - id: population
          type: File
        - id: speakers
          type: File
        outputs:
        - id: out
          type: File
          outputSource: plot/results

        steps:
        - id: calculation
          in:
            population: population
            speakers: speakers
          run: '../calculation/calculation.cwl'
          out:
          - results
        - id: plot
          in:
            results: calculation/results
          run: '../plot/plot.cwl'
          out:
          - results
        "#;
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let mut graph: Vec<Value> = Vec::new();
        let arc_run_opt = cwl_to_arc_run(&cwl_path, cwl_rel_path, &raw_inputs, foldername, &yaml, &mut graph);
        assert!(arc_run_opt.is_some());
        let arc_run = arc_run_opt.unwrap();
        assert_eq!(arc_run.id, "runs/run1/");
        assert_eq!(arc_run.type_, "Dataset");
        assert_eq!(arc_run.additional_type, "Run");
        assert_eq!(arc_run.identifier, "run1");
        let has_part_ids: Vec<String> = arc_run.has_part.unwrap().iter()
            .filter_map(|v| v.get("@id").and_then(|s| s.as_str()).map(String::from))
            .collect();
        assert!(has_part_ids.contains(&"workflows/main/main.cwl".to_string()));
        assert!(has_part_ids.contains(&population.to_string_lossy().to_string()));
        assert!(has_part_ids.contains(&speakers.to_string_lossy().to_string()));
        let investigation = graph.iter()
            .find(|v| v.get("@type").and_then(|t| t.as_str()) == Some("Dataset")
                && v.get("additionalType").and_then(|at| at.as_str()) == Some("Investigation"));
        assert!(investigation.is_some());
    }

     #[test]
    fn test_cwl_to_arc_workflow() {
        let yaml_str = r#"
        class: Workflow
        label: Example Workflow
        doc: This is a test workflow.
        s:creator:
          - class: s:Person
            s:name: Jane Doe
            s:email: mailto:jane@example.com
        arc:performer:
          - class: arc:Person
            arc:first name: John
            arc:last name: Doe
            arc:email: john@example.com
        steps:
          - id: step1
            run: ../tool1/tool1.cwl
            out: [output1]
          - id: step2
            run: ../tool2/tool2.cwl
            out: [output2]
        "#;
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let cwl_path = Path::new("/home/user/project/workflows/main/main.cwl");
        let mut graph: Vec<Value> = Vec::new();
        let workflow_opt = cwl_to_arc_workflow(cwl_path, &mut graph, &yaml);
        assert!(workflow_opt.is_some());
        let workflow = workflow_opt.unwrap();
        assert_eq!(workflow.main_entity.id, "/home/user/project/workflows/main/main.cwl");
        assert_eq!(workflow.identifier, "main");
        assert_eq!(workflow.name.unwrap(), "Example Workflow");
        assert_eq!(workflow.description.unwrap(), "This is a test workflow.");
        let has_part = workflow.has_part.unwrap();
        assert!(has_part.contains(&"workflows/main/main.cwl".to_string()));
        assert!(has_part.contains(&"workflows/tool1/tool1.cwl".to_string()));
        assert!(has_part.contains(&"workflows/tool2/tool2.cwl".to_string()));
        let ids: Vec<String> = graph.iter().filter_map(|v| v.get("@id").and_then(|s| s.as_str()).map(String::from)).collect();
        assert!(ids.iter().any(|id| id.contains("Jane_Doe")));
        assert!(ids.iter().any(|id| id.contains("John_Doe")));
    }

    #[test]
    fn test_extract_has_part_with_steps() {
        let yaml_str = r#"
        class: Workflow
        steps:
        - id: shuffleseq
          in:
          - id: sequence
            source: sequence
          run: ../shuffleseq/shuffleseq.cwl
          out:
          - Yeast_shuffled
        - id: fasta_to_tabular
          in:
          - id: yeast_shuffled_fasta
            source: shuffleseq/Yeast_shuffled
          run: ../fasta_to_tabular/fasta_to_tabular.cwl
          out:
          - Yeast_shuffled
        "#;
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let main_path = Path::new("/home/user/project/workflows/main/main.cwl");
        let result = super::extract_has_part(main_path, &yaml).unwrap();
        let mut normalized: Vec<String> = result.iter().map(|s| s.replace("\\", "/")).collect();
        normalized.sort();
        assert!(normalized.iter().any(|p| p.ends_with("workflows/main/main.cwl")));
        assert!(normalized.iter().any(|p| p.ends_with("workflows/shuffleseq/shuffleseq.cwl")));
        assert!(normalized.iter().any(|p| p.ends_with("workflows/fasta_to_tabular/fasta_to_tabular.cwl")));
    }

    #[test]
    fn test_extract_has_part_no_steps_non_workflow() {
        let yaml_str = r#"
        class: CommandLineTool
        "#;
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let main_path = Path::new("workflows/tool/tool.cwl");
        let result = super::extract_has_part(main_path, &yaml).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("workflows/tool/tool.cwl"));
    }

    #[test]
    fn test_extract_has_part_removes_duplicate_folder_segment() {
        let yaml_str = r#"
        class: Workflow
        steps: []
        "#;
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let main_path = Path::new("workflows/main/main.cwl");
        let result = super::extract_has_part(main_path, &yaml).unwrap();
        assert_eq!(result[0], "workflows/main/main.cwl");
    }

    #[test]
    fn test_extract_parameter_values() {
        let yaml_str = r#"
            arc:has process sequence:
            - class: arc:process sequence
              arc:name: "script.fsx"
              arc:has parameter value: 
                - class: arc:process parameter value
                  arc:has parameter:
                    - class: arc:protocol parameter
                      arc:has parameter name: 
                      - class: arc:parameter name
                        arc:term accession: "http://purl.obolibrary.org/obo/NCIT_C43582"
                        arc:term source REF: "NCIT"
                        arc:annotation value: "Data Transformation"
                  arc:value: 
                    - class: arc:ontology annotation
                      arc:term accession: "http://purl.obolibrary.org/obo/NCIT_C64911"
                      arc:term source REF: "NCIT"
                      arc:annotation value: "Addition"
            "#;

        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let mut graph: Vec<Value> = Vec::new();
        let ids = super::extract_parameter_values(&yaml, &mut graph);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "#ParameterValue_data_transformation_Addition");
        let entity = graph.iter().find(|v| v["@id"] == ids[0]).unwrap();
        assert_eq!(entity["@type"], "PropertyValue");
        assert_eq!(entity["@additionalType"], "ParameterValue");
        assert_eq!(entity["name"], "data transformation");
        assert_eq!(entity["value"], "Addition");
        assert_eq!(entity["propertyID"], "http://purl.obolibrary.org/obo/NCIT_C43582");
        assert_eq!(entity["valueReference"], "http://purl.obolibrary.org/obo/NCIT_C64911");
    }

    #[test]
    fn test_git_creator_uses_local_config() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();
        let repo = Repository::init(repo_path).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        let orig_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo_path).unwrap();
        let mut graph: Vec<Value> = Vec::new();
        let result = super::git_creator(&mut graph);
        std::env::set_current_dir(orig_dir).unwrap();
        assert!(result.is_some());
        let persons = result.unwrap();
        assert_eq!(persons.len(), 1);
        let person = &persons[0];
        assert_eq!(person["@id"], "#Person_Test_User");
        let found = graph.iter().any(|v| v["@id"] == "#Person_Test_User");
        assert!(found, "Graph should contain the Test User person entity");
    }
   
    #[test]
    fn test_extract_creator_with_author() {
        let yaml_str = r#"
            s:author:
            - class: s:Person
              s:identifier: "https://orcid.org/0000-0000-0000-0000"
              s:email: "mailto:doe@mail.com"
              s:name: "Jane Doe"
            "#;
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let mut graph: Vec<serde_json::Value> = Vec::new();
        let refs = extract_creator(&yaml, &mut graph).expect("Should extract creator");
        let expected_id = "#Person_Jane_Doe";
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0]["@id"], expected_id);
        let person = graph.iter().find(|v| v["@id"] == expected_id).unwrap();
        assert_eq!(person["@type"], "Person");
        assert_eq!(person["givenName"], "Jane");
        assert_eq!(person["familyName"], "Doe");
        assert_eq!(person["email"], "doe@mail.com");
        assert_eq!(person["@id"], "#Person_Jane_Doe");
        assert!(person["affiliation"].is_null());
        assert!(person["jobTitle"].is_null());
    }

    #[test]
    fn test_extract_performer_full_entry() {
        let yaml_str = r#"
        arc:performer:
        - class: arc:Person
          arc:first name: "John"
          arc:last name: "Doe"
          arc:email: "doe@mail.de "
          arc:affiliation: "Institute xy"
          arc:has role:
          - class: arc:role
            arc:term accession: "https://credit.niso.org/contributor-roles/formal-analysis/"
            arc:annotation value: "Formal analysis"
        "#;
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let mut graph = Vec::new();
        let result = extract_performer(&yaml, &mut graph).unwrap();
        assert_eq!(result.len(), 1);
        let person_id = "#Person_John_Doe";
        let person = graph.iter().find(|v| v["@id"] == person_id).unwrap();
        assert_eq!(person["givenName"], "John");
        assert_eq!(person["familyName"], "Doe");
        assert_eq!(person["email"], "doe@mail.de");
        assert_eq!(person["@type"], "Person");
        let org_id = "#Organization_Institute_xy";
        let org = graph.iter().find(|v| v["@id"] == org_id).unwrap();
        assert_eq!(org["@type"], "Organization");
        assert_eq!(org["name"], "Institute xy");
        let role_id = "https://credit.niso.org/contributor-roles/formal-analysis/";
        let role = graph.iter().find(|v| v["@id"] == role_id).unwrap();
        assert_eq!(role["@type"], "DefinedTerm");
        assert_eq!(role["name"], "Formal analysis");
        assert_eq!(role["term_code"], role_id);
    }

    #[test]
    fn test_add_person_without_affiliation() {
        let mut graph = Vec::new();
        let person = PersonEntity {
            id: "person:123".to_string(),
            type_: "Person".to_string(),
            given_name: Some("Alice".to_string()),
            family_name: Some("Wonderland".to_string()),
            additional_name: None,
            email: Some("alice@example.com".to_string()),
            affiliation: None,
            job_title: None,
            address: None,
        };
        let result = ensure_person(&mut graph, &person);
        assert_eq!(result, json!({ "@id": "person:123" }));
        assert_eq!(graph.len(), 1);
        assert_eq!(graph[0]["@id"], "person:123");
        assert_eq!(graph[0]["@type"], "Person");
        assert_eq!(graph[0]["givenName"], "Alice");
        assert_eq!(graph[0]["familyName"], "Wonderland");
        assert_eq!(graph[0]["email"], "alice@example.com");
    }

    #[test]
    fn test_add_person_with_affiliation() {
        let mut graph = Vec::new();
        let affiliation = json!({
            "@id": "org:999",
            "name": "Institution xz"
        });
        let person = PersonEntity {
            id: "person:456".to_string(),
            type_: "Person".to_string(),
            given_name: Some("Bob".to_string()),
            family_name: Some("Smith".to_string()),
            affiliation: Some(affiliation),
            additional_name: None,
            email: Some("bob@example.com".to_string()),
            job_title: None,
            address: None,
        };
        ensure_person(&mut graph, &person);
        let ids: Vec<_> = graph.iter().map(|v| v["@id"].as_str().unwrap()).collect();
        assert!(ids.contains(&"org:999"));
        assert!(ids.contains(&"person:456"));
        let org = graph.iter().find(|v| v["@id"] == "org:999").unwrap();
        assert_eq!(org["@type"], "Organization");
        assert_eq!(org["name"], "Institution xz");
        let bob = graph.iter().find(|v| v["@id"] == "person:456").unwrap();
        assert_eq!(bob["@type"], "Person");
        assert_eq!(bob["givenName"], "Bob");
        assert_eq!(bob["familyName"], "Smith");
        assert_eq!(bob["email"], "bob@example.com");
    }

    #[test]
    fn test_no_duplicates_when_person_exists() {
        let mut graph = Vec::new();
        let person = PersonEntity {
            id: "person:789".to_string(),
            type_: "Person".to_string(),
            given_name: Some("Charlie".to_string()),
            family_name: Some("Brown".to_string()),
            additional_name: None,
            email: None,
            affiliation: None,
            job_title: None,
            address: None,
        };
        ensure_person(&mut graph, &person);
        let len_after_first = graph.len();
        ensure_person(&mut graph, &person);
        assert_eq!(graph.len(), len_after_first, "Graph should not grow with duplicates");
        let count = graph.iter().filter(|v| v["@id"] == "person:789").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_ensure_creativeworks_adds_missing_entities() {
        let mut graph = Vec::new();
        let ids = ensure_creativeworks(&mut graph);
        assert_eq!(
            ids,
            vec![
                json!({"@id": "https://w3id.org/ro/wfrun/process/0.1"}),
                json!({"@id": "https://w3id.org/ro/wfrun/workflow/0.1"}),
                json!({"@id": "https://w3id.org/workflowhub/workflow-ro-crate/1.0"})
            ]
        );
        assert_eq!(graph.len(), 3);
        let process_run = graph.iter().find(|v| v["@id"] == "https://w3id.org/ro/wfrun/process/0.1").unwrap();
        assert_eq!(process_run["@type"], "CreativeWork");
        assert_eq!(process_run["name"], "Process Run Crate");
        assert_eq!(process_run["version"], "0.1");
    }

    #[test]
    fn test_ensure_creativeworks_does_not_duplicate() {
        let mut graph = vec![json!({
            "@id": "https://w3id.org/ro/wfrun/process/0.1",
            "@type": "CreativeWork",
            "name": "Process Run Crate",
            "version": "0.1"
        })];
        let initial_len = graph.len();
        ensure_creativeworks(&mut graph);
        assert_eq!(graph.len(), initial_len + 2);
        let count = graph.iter().filter(|v| v["@id"] == "https://w3id.org/ro/wfrun/process/0.1").count();
        assert_eq!(count, 1);
    }

}
    