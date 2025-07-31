use std::fs::File;
use std::io::Read;
use serde_json::Value;
use crate::arc_entities::{ArcWorkflow, ArcRun, WorkflowInvocation,
    WorkflowProtocol, ArcRoCrate, MainEntity};
use crate::api::get_reana_user;
use crate::rocrate::get_or_prompt_credential;
use walkdir::WalkDir;


pub fn workflow_json_to_arc_workflow(json: &serde_json::Value) -> Option<ArcWorkflow> {
    // cwl workflow file, e.g. "workflows/main/main.cwl"
    let main_entity = json.get("workflow").and_then(|w| w.get("file"))
        .and_then(|f| f.as_str())
        .unwrap_or("")
        .to_string();
    // path to folder where the workflow is located, e.g."workflows/main"
    let id = {
        let mut parts: Vec<&str> = main_entity.split('/').collect();
        if parts.len() > 1 {
            parts.pop();
            parts.join("/") + "/"
        } else {
            main_entity.clone()
        }
    };
    // folder located under "workflows", e.g. "main"
    let identifier = {
            let mut parts: Vec<&str> = main_entity.split('/').collect();
            if parts.len() > 1 {
                parts.pop();
                parts.last().copied().unwrap_or(main_entity.as_str())
            } else {
                main_entity.as_str()
            }
    }.to_string();
    let additional_type = "Workflow".to_string();
    let type_ = "Dataset".to_string();

    // Try to read the main_entity file (if it exists) and extract label/name and doc from it if they exist
    let (name, description) = if !main_entity.is_empty() && std::path::Path::new(&main_entity).exists() {
        match std::fs::read_to_string(&main_entity) {
            Ok(contents) => {
                if let Ok(main_json) = serde_yaml::from_str::<serde_yaml::Value>(&contents) {
                    let name = main_json.get("label")
                        .or_else(|| main_json.get("name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let description = main_json.get("doc")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    (name, description)
                } else {
                    (None, None)
                }
            }
            Err(_) => {
                println!("Failed to read {main_entity} file");
                (None, None)
            }
        }
    } else {
        println!("{main_entity} file not found");
        (None, None)
    };

    // has_part: TODO, check if main_entity is using all subworkflows and compliance with Workflow Protocol profile
    // has_part: all files in the workflows folder
    let workflows_folder = "workflows";
    let mut has_part = Vec::new();
    if let Ok(entries) = WalkDir::new(workflows_folder).into_iter().collect::<Result<Vec<_>, _>>() {
        for entry in entries {
            let path = entry.path();
            if path.is_file() {
                // Store the path relative to the workflows folder
                if let Ok(rel_path) = path.strip_prefix(workflows_folder) {
                    let rel_path_str = rel_path.to_string_lossy();
                    if !rel_path_str.is_empty() {
                        has_part.push(format!("{workflows_folder}/{rel_path_str}"));
                    }
                }
            }
        }
    }
    let has_part = if has_part.is_empty() { None } else { Some(has_part) };

    // url: TODO
    let url = None;

    Some(ArcWorkflow {
        id,
        type_,
        additional_type,
        identifier,
        main_entity: MainEntity { id: main_entity },
        name,
        description,
        has_part,
        url,
    })
}

/// helper function to get affiliation and ORCID from the ORCID public API
fn get_affiliation_and_orcid(reana_user_name: &str) -> (Option<String>, Option<String>) {
    let mut affiliation = None;
    let mut orcid = None;

    // Try to get ORCID using the user's name via the ORCID public API
    if !reana_user_name.is_empty() {
        let name_parts: Vec<&str> = reana_user_name.split_whitespace().collect();
        if name_parts.len() >= 2 {
            let given_names = name_parts[0];
            let family_name = name_parts[1];
            let query = format!("given-names:{given_names} AND family-name:{family_name}");
            let search_url = format!("https://pub.orcid.org/v3.0/expanded-search/?q={query}");
            let client = reqwest::blocking::Client::new();
            if let Ok(resp) = client
                .get(&search_url)
                .header("Accept", "application/json")
                .send()
            {
                if let Ok(json) = resp.json::<serde_json::Value>() {
                    if let Some(results) = json.get("expanded-result").and_then(|v| v.as_array()) {
                        // Get the first result, TODO: change this to ask user for confirmation
                        if let Some(first_result) = results.first() {
                            orcid = first_result.get("orcid-id").and_then(|v| v.as_str()).map(|s| s.to_string());
                            affiliation = first_result
                                .get("institution-name")
                                .and_then(|v| {
                                    if v.is_array() {
                                        v.as_array()
                                            .and_then(|arr| arr.first())
                                            .and_then(|first| first.as_str())
                                            .map(|s| s.to_string())
                                    } else {
                                        v.as_str().map(|s| s.to_string())
                                    }
                                });
                        }
                    }
                }
            }
        }
    }
    (affiliation, orcid)
}


pub fn workflow_json_to_arc_run(foldername: &str, graph: &mut Vec<Value>) -> Option<ArcRun> {

    let id = format!("runs/{foldername}/");
    let type_ = "Dataset".to_string();
    let additional_type = "Run".to_string();
    let identifier = foldername.to_string();
    // Prompt user for name and description of the ARC Run
    println!("Enter a name of the ARC Run:");
    let mut input_name = String::new();
    std::io::stdin().read_line(&mut input_name).ok();
    let input_name = input_name.trim();
    let name = if !input_name.is_empty() {
        Some(input_name.to_string())
    } else {
        None
    };
    println!("Enter a description of the ARC Run:");
    let mut input_description = String::new();
    std::io::stdin().read_line(&mut input_description).ok();
    let input_description = input_description.trim();
    let description = if !input_description.is_empty() {
        Some(input_description.to_string())
    } else {
        None
    };
    // about and mentions: reference to WorkflowInvocation
    // TODO change folder naming process if multiple runs with same name
    let invocation_id = format!("#WorkflowInvocation_{foldername}_0");
    let about = Some(vec![invocation_id.clone()]);
    let mentions = Some(vec![invocation_id.clone()]);

    // Get REANA user information
    let reana_instance = get_or_prompt_credential("reana", "instance", "Enter REANA instance URL: ").ok()?;
    let reana_token = get_or_prompt_credential("reana", "token", "Enter REANA access token: ").ok()?;
    let reana_user = get_reana_user(&reana_instance, &reana_token).ok()?;
    let reana_user_name = reana_user["full_name"].as_str().unwrap_or("");
    // Create a PersonEntity from reana_user, performer of the ARC Run
    let (given_name, family_name) = {
        let mut parts = reana_user_name.split_whitespace();
        let given = parts.next().unwrap_or("").to_string();
        let family = parts.next().unwrap_or("").to_string();
        (Some(given), Some(family))
    };

    // Try to get affiliation and ORCID via the ORCID public API
    // TODO use ORCID if found
    let (affiliation, _orcid) = if !reana_user_name.is_empty() {
        get_affiliation_and_orcid(reana_user_name)
    } else {
        (None, None)
    };
    let email = reana_user["email"].as_str().map(|s| s.to_string());
    // Create a PersonEntity with the reana_user_name, affiliation, and email and add this to graph
    let person_entity = crate::arc_entities::PersonEntity {
        id: format!("#Person_{}", reana_user_name.replace(' ', "_")),
        type_: "Person".to_string(),
        given_name,
        family_name,
        additional_name: None,
        affiliation: affiliation.as_ref().map(|aff| serde_json::json!({ "@id": format!("#Organization_{}", aff.replace(' ', "_")) })),
        email,
        job_title: None,
        address: None,
    };
    graph.push(serde_json::to_value(person_entity).unwrap());
    // If affiliation of performer is available via ORCID, create an OrganizationEntity and add it to the graph
    if let Some(ref aff) = affiliation {
        // Create an OrganizationEntity if affiliation is available
        let organization_entity = crate::arc_entities::OrganizationEntity {
            id: format!("#Organization_{}", aff.replace(' ', "_")),
            type_: "Organization".to_string(),
            name: aff.to_string(),
        };
        graph.push(serde_json::to_value(organization_entity).unwrap());
    }
    let creator = Some(vec![format!("#Person_{}", reana_user_name.replace(' ', "_"))]);
    // has_part: all files in the run folder
    // Collect all files and directories inside the run folder
    let run_folder = format!("runs/{foldername}");
    let mut has_part = Vec::new();
    if let Ok(entries) = WalkDir::new(&run_folder).into_iter().collect::<Result<Vec<_>, _>>() {
        for entry in entries {
            let path = entry.path();
            if path.is_file() {
                if let Ok(rel_path) = path.strip_prefix(&run_folder) {
                    let rel_path_str = rel_path.to_string_lossy();
                    if !rel_path_str.is_empty() {
                        has_part.push(format!("runs/{foldername}/{rel_path_str}"));
                    }
                }
            }
        }
    }
    // TODO: check compliance with latest Workflow RO-Crate profile 1.2, change to not hard code conforms_to
    let conforms_to = vec![
        "https://w3id.org/ro/wfrun/process/0.5".to_string(),
        "https://w3id.org/ro/wfrun/workflow/0.5".to_string(),
        "https://w3id.org/workflowhub/workflow-ro-crate/1.1".to_string()
    ];

    // add CreativeWork entities to the graph based on conforms_to URLs
    let creative_works: Vec<(String, String, String)> = conforms_to
        .iter()
        .filter_map(|url| {
            let parts: Vec<&str> = url.split('/').collect();
            if parts.len() < 2 {
                return None;
            }
            let version = parts.last().unwrap_or(&"").to_string();
            let name = if parts.get(parts.len().wrapping_sub(2)) == Some(&"process") {
                "Process Run Crate"
            } else if parts.get(parts.len().wrapping_sub(2)) == Some(&"workflow-ro-crate") {
                "Workflow RO-Crate"
            } else if parts.get(parts.len().wrapping_sub(2)) == Some(&"workflow") {
                "Workflow Run Crate"
            } else {
                ""
            };
            Some((url.clone(), name.to_string(), version))
        })
        .collect();

    for (url, name, version) in creative_works {
        let creative_work = crate::arc_entities::CreativeWorkEntity {
            id: url.to_string(),
            type_: "CreativeWork".to_string(),
            name: Some(name.to_string()),
            version: Some(version.to_string()),
        };
        graph.push(serde_json::to_value(creative_work).unwrap());
    }

    Some(ArcRun {
        id,
        type_,
        additional_type,
        identifier,
        name,
        description,
        about: about.map(|ids| ids.into_iter().next().map(|id| serde_json::json!({ "@id": id })).unwrap_or(serde_json::Value::Null)),
        mentions: mentions.map(|ids| ids.into_iter().next().map(|id| serde_json::json!({ "@id": id })).unwrap_or(serde_json::Value::Null)),
        creator,
        has_part: Some(has_part),
        measurement_method: None,
        measurement_technique: None,
        conforms_to: Some(conforms_to),
        url: None,
        variable_measured: None,
    })
}


pub fn workflow_json_to_invocation(json: &serde_json::Value, foldername: &str) -> Option<WorkflowInvocation> {
    let name = foldername.to_string();
    //TODO: multiple runs with same name, change folder naming process
    let id = format!("#WorkflowInvocation_{foldername}_0");
    let type_ = vec!["https://bioschemas.org/CreateAction".to_string(), "LabProcess".to_string()];
    let additional_type = "WorkflowInvocation".to_string();

    // instrument and executes_lab_protocol: main workflow file
    let instrument = json.get("workflow")
        .and_then(|w| w.get("file"))
        .and_then(|f| f.as_str())
        .unwrap_or("")
        .to_string();
    let executes_lab_protocol = instrument.clone();
    // Extract result files from the new structure
    let mut result_files = Vec::new();
    let mut object_files = Vec::new();

    // Try to get inputs and outputs
    if let Some(inputs) = json.get("inputs") {
        if let Some(files) = inputs.get("files").and_then(|f| f.as_array()) {
            for file in files {
                if let Some(file_str) = file.as_str() {
                    object_files.push(file_str.to_string());
                }
            }
        }
    }

    if let Some(outputs) = json.get("outputs") {
        if let Some(files) = outputs.get("files").and_then(|f| f.as_array()) {
            for file in files {
                if let Some(file_str) = file.as_str() {
                    result_files.push(format!("runs/{foldername}/{file_str}"));
                }
            }
        }
    }

    // parameter_value: TODO
    let parameter_value = vec!["".to_string()];

    Some(WorkflowInvocation {
        id,
        type_,
        additional_type,
        instrument: vec![serde_json::json!({ "@id": instrument })],
        executes_lab_protocol: serde_json::json!({ "@id": executes_lab_protocol }),
        result: result_files.into_iter().map(|file| serde_json::json!({ "@id": file })).collect(),
        object: object_files.into_iter().map(|file| serde_json::json!({ "@id": file })).collect(),
        name,
        parameter_value: vec![serde_json::json!({ "@id": parameter_value })].into(),
        description: None,
    })
}



pub fn workflow_json_to_protocol(json: &serde_json::Value, graph: &mut Vec<Value>) -> Option<WorkflowProtocol> {
    
    let _type = vec![
        "File".to_string(),
        "ComputationalWorkflow".to_string(),
        "SoftwareSourceCode".to_string(),
        "LabProtocol".to_string(),
    ];
    let id = json.get("workflow")
        .and_then(|w| w.get("file"))
        .and_then(|f| f.as_str())
        .unwrap_or("")
        .to_string();

    let mut input: Option<Vec<serde_json::Value>> = None;
    let mut output: Option<Vec<serde_json::Value>> = None;

    // Find the Workflow object in the $graph array with class == "Workflow" and file == id
    if let Some(specification) = json.get("workflow")
        .and_then(|w| w.get("specification"))
        .and_then(|s| s.get("$graph"))
        .and_then(|g| g.as_array())
    {
        // Find the workflow object with class == "Workflow" and file == id
        let workflow_obj = specification.iter().find(|obj| {
            obj.get("class").and_then(|c| c.as_str()) == Some("Workflow")
        });

        if let Some(wf) = workflow_obj {
            // Extract inputs
            if let Some(inputs) = wf.get("inputs").and_then(|v| v.as_array()) {
                input = Some(inputs.clone());
            }
            // Extract outputs
            if let Some(outputs) = wf.get("outputs").and_then(|v| v.as_array()) {
                output = Some(outputs.clone());
            }
        }
    }
    // Convert input/output to IDs of the form "#FormalParameter_{workflow_name}_{parameter_name}"
    let input_ids = input.as_ref().map(|inputs| {
        inputs
            .iter()
            .filter_map(|inp| inp.get("id").or_else(|| inp.get("name")).or_else(|| inp.get("label")))
            .filter_map(|v| v.as_str())
            .map(|param| {
                let formatted = param.trim_start_matches('#').replace('/', "_");
                serde_json::json!({ "@id": format!("#FormalParameter_{}", formatted) })
            })
            .collect::<Vec<_>>()
    });

    let output_ids = output.as_ref().map(|outputs| {
        outputs
            .iter()
            .filter_map(|out| out.get("id").or_else(|| out.get("name")).or_else(|| out.get("label")))
            .filter_map(|v| v.as_str())
            .map(|param| {
                let formatted = param.trim_start_matches('#').replace('/', "_");
                serde_json::json!({ "@id": format!("#FormalParameter_{}", formatted) })
            })
            .collect::<Vec<_>>()
    });

    // Combine input and output parameters, check for type File, and only add if type is File
    for param in input.as_ref().into_iter().chain(output.as_ref()) {
        for p in param {
            // Check if type is File
            let is_file = match p.get("type") {
                Some(ty) => {
                    if ty.is_array() {
                        ty.as_array().unwrap().iter().any(|t| t == "File")
                    } else {
                        ty == "File"
                    }
                }
                None => false,
            };
            if is_file {
                // Get id, name, or label for identifier
                let id_val = p.get("id").or_else(|| p.get("name")).or_else(|| p.get("label"));
                if let Some(id_str) = id_val.and_then(|v| v.as_str()) {
                    let formatted = match id_str.find('/') {
                        Some(pos) => id_str[pos + 1..].to_string(),
                        None => id_str.trim_start_matches('#').to_string(),
                    };
                    let formal_id = format!("#FormalParameter_{formatted}");
                    let formal_param = crate::arc_entities::FormalParameterEntity {
                        id: formal_id.clone(),
                        type_: "FormalParameter".to_string(),
                        additional_type: Some("File".to_string()),
                        name: Some(formatted),
                        value_required: None,
                    };
                    graph.push(serde_json::to_value(formal_param).unwrap());
                }
            }
        }
    }
    // Programming language currently only CWL, TODO add other languages that are used
    let (programming_language, computer_language_entity) = if id.contains("cwl") {
        let lang_id = "https://w3id.org/workflowhub/workflow-ro-crate#cwl".to_string();
        let entity = crate::arc_entities::ComputerLanguageEntity {
            id: lang_id.clone(),
            type_: "ComputerLanguage".to_string(),
            name: Some("Common Workflow Language".to_string()),
            alternate_name: Some("CWL".to_string()),
            identifier: Some(serde_json::json!({ "@id": "https://w3id.org/cwl/v1.2/" })),
            url: Some(serde_json::json!({ "@id": "https://www.commonwl.org/" })),
        };
        (Some(lang_id), Some(entity))
    } else {
        (None, None)
    };

    if let Some(entity) = computer_language_entity {
        graph.push(serde_json::to_value(entity).unwrap());
    }
    //TODO
    Some(WorkflowProtocol {
        context: "".to_string(),
        type_: _type,
        additional_type: "WorkflowProtocol".to_string(),
        id,
        input: input_ids,
        output: output_ids,
        dct_conforms_to: None,
        creator: None,
        date_created: None,
        license: None,
        name: None,
        programming_language: programming_language.map(|lang| vec![lang]),
        sd_publisher: None,
        url: None,
        version: None,
        description: None,
        has_part: None,
        intended_use: None,
        comment: None,
        computational_tool: None,
    })
}

pub fn read_workflow_json(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let json: Value = serde_json::from_str(&contents)?;
    Ok(json)
}

pub fn workflow_json_to_arc_rocrate(json: &Value, folder_name: &str) -> ArcRoCrate {
    // Context as array of Value, TODO extend and not hardcode
    let context = vec![
        serde_json::json!("https://w3id.org/ro/crate/1.1/context"),
        serde_json::json!({
            "ComputationalWorkflow": "https://bioschemas.org/ComputationalWorkflow",
            "SoftwareSourceCode": "http://schema.org/SoftwareSourceCode",
            "LabProtocol": "https://bioschemas.org/LabProtocol"
        }),
    ];
    let mut graph: Vec<Value> = Vec::new();

    // Generate entities, TODO: muliple entiies, e.g. workflow for each folder?
    // TODO remove option or think of case where entity cannot be created
    let workflow = workflow_json_to_arc_workflow(json);

    let run = workflow_json_to_arc_run(folder_name, &mut graph);

    let workflow_invocation = workflow_json_to_invocation(json, folder_name);

    let worfklow_protocol = workflow_json_to_protocol(json, &mut graph);

    if let Some(w) = workflow {
        graph.push(serde_json::to_value(w).unwrap());
    }
    if let Some(r) = run {
        graph.push(serde_json::to_value(r).unwrap());
    }
    if let Some(i) = workflow_invocation {
        graph.push(serde_json::to_value(i).unwrap());
    }
    if let Some(p) = worfklow_protocol {
        graph.push(serde_json::to_value(p).unwrap());
    }

    let root_data_entity = crate::arc_entities::RootDataEntity {
        id: "ro-crate-metadata.json".to_string(),
        type_: "CreativeWork".to_string(),
        conforms_to: Some(vec![
            //this is also the first element of context
            serde_json::json!({ "@id": "https://w3id.org/ro/crate/1.1" }),
            serde_json::json!({ "@id": "https://w3id.org/workflowhub/workflow-ro-crate/1.0" }),
        ]),
        about: Some(serde_json::json!({ "@id": "./" })),
    };
    graph.push(serde_json::to_value(root_data_entity).unwrap());


    let rocrate = ArcRoCrate {
        context,
        graph,
    };


    // Write to runs/{folder_name}/ro-crate-metadata.json
    let output_dir = format!("runs/{folder_name}");
    std::fs::create_dir_all(&output_dir).ok();

    let output_path = format!("{output_dir}/ro-crate-metadata.json");
    let _ = write_arc_rocrate_metadata(&rocrate, &output_path);

    rocrate
}

/// Writes an ArcRoCrate struct to a ro-crate-metadata.json file.
pub fn write_arc_rocrate_metadata(rocrate: &ArcRoCrate, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, rocrate)?;
    Ok(())
}

/// High-level function: reads workflow.json, converts, and writes ro-crate-metadata.json.
pub fn generate_rocrate_metadata(workflow_json_path: &str, rocrate_metadata_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let workflow_json = read_workflow_json(workflow_json_path)?;
    let rocrate = workflow_json_to_arc_rocrate(&workflow_json, rocrate_metadata_path);
    write_arc_rocrate_metadata(&rocrate, rocrate_metadata_path)?;
    Ok(())
}
