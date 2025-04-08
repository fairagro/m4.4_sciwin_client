use serde_yaml:: Value;
use std::{
    collections::HashMap,
    error::Error,
    fs,
    env,
    path::PathBuf,
    io::{self, Read, Write}
};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE, COOKIE};
use serde_json::json;
use serde_yaml::{Mapping};
use std::path::Path;
use std::path::MAIN_SEPARATOR;
use std::collections::HashSet;
use reqwest::blocking::ClientBuilder;
use serde::{Deserialize, Serialize};
use crate::commands::execute::RemoteExecuteArgs;

#[derive(Debug, Deserialize)]
struct ReanaYaml {
    inputs: ReanaInputs,
    outputs: ReanaOutputs,
    workflow: ReanaWorkflow,
    version: String,
}


#[derive(Debug, Deserialize)]
struct ReanaInputs {
    files: Vec<String>,
    directories: Vec<String>,
    parameters: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReanaOutputs {
    files: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ReanaWorkflow {
    r#type: String,
    file: String,
}

#[derive(Debug, Serialize)]
struct WorkflowJson {
    inputs: WorkflowInputs,
    outputs: ReanaOutputs,
    version: String,
    workflow: WorkflowSpec,
}

#[derive(Debug, Serialize)]
struct WorkflowSpec {
    file: String,
    specification: CWLGraph,
    r#type: String,
}

#[derive(Debug, Serialize)]
struct WorkflowInputs {
    directories: Vec<String>,
    files: Vec<String>,
    parameters: serde_yaml::Value,
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CWLWorkflow {
    cwl_version: String,
    class: String,
    inputs: Vec<CWLInput>,  
    outputs: Vec<CWLOutput>,
    #[serde(default)]
    steps: Vec<CWLStep>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CWLFile {
    class: String,
    location: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CWLStep {
    id: String,
    r#in: HashMap<String, String>, 
    run: String,
    out: Vec<CWLStepOutput>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CWLStepInput {
    id: String,
    source: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum CWLStepOutput {
    Simple(String), 
    Detailed { id: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CWLInput {
    id: String,
    r#type: String,
    default: Option<CWLFile>,
    input_binding: Option<CWLInputBinding>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CWLOutput {
    id: String,
    output_source: Option<String>,
    r#type: Option<String>,
    output_binding: Option<CWLOutputBinding>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CWLInputBinding {
    prefix: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CWLOutputBinding {
    glob: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Parameter {
    pub r#class: String, 
    pub path: String,
}

#[derive(Debug, Serialize)]
struct CWLGraph {
#[serde(rename = "$graph")]
graph: Vec<serde_json::Value>,
#[serde(rename = "cwlVersion")]
cwl_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CWLCommandLineTool {
    cwl_version: String,
    class: String,
    base_command: Vec<String>,
    inputs: Vec<CWLInput>,
    outputs: Vec<CWLOutput>,
    requirements: Option<Vec<CWLRequirement>>,
    label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CWLRequirement {
    class: String,
    docker_pull: Option<String>,
    listing: Option<Vec<CWLListing>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CWLListing {
    entryname: String,
    entry: CWLEntry,
}

#[derive(Debug, Serialize, Deserialize)]
struct CWLListingEntry {
    entryname: String,
    entry: CWLEntry,
}

impl CWLStepOutput {
    fn id(&self) -> &str {
        match self {
            CWLStepOutput::Simple(s) => s,
            CWLStepOutput::Detailed { id } => id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum CWLEntry {
    Include { #[serde(rename = "$include")] include: String },
    Expression(String),
}   

pub fn create_workflow(args: &RemoteExecuteArgs, workflow: &serde_json::Value) -> Result<Value, Box<dyn Error>> {
    let reana_server = &args.instance;
    let reana_token = &args.token;

    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, args.cookie_value.parse()?);
   
    headers.insert(AUTHORIZATION, format!("Bearer {}", reana_token).parse()?);
    headers.insert(CONTENT_TYPE, "application/json".parse()?);
    let client = Client::builder()
    .default_headers(headers.clone())
    .danger_accept_invalid_certs(true)
    .build()?;


    // Send the request to create the workflow
    let response = client
        .post(format!("{}/api/workflows", reana_server))
        .headers(headers)
        .json(&workflow)
        .send()?;

    let json_response: Value = response.json()?;
    
    Ok(json_response)
}


pub fn ping_reana(args: &RemoteExecuteArgs) -> Result<Value, Box<dyn Error>> {
    let reana_server = &args.instance;
    let ping_url = format!("{}/api/ping", reana_server);

    let headers = HeaderMap::new();

    // Invalid certs part is needed for our locahost test instance
    let client = Client::builder()
        .default_headers(headers)
        .danger_accept_invalid_certs(true)
        .build()?;

    let response = client.get(&ping_url).send()?;

    let json_response: Value = response.json()?;
    
    Ok(json_response)
}

pub fn start_workflow(
    args: &RemoteExecuteArgs,
    workflow_name: &str,
    operational_options: Option<HashMap<String, Value>>,
    input_parameters: Option<HashMap<String, Value>>,
    restart: Option<bool>,
    reana_specification: Option<Value>,
) -> Result<(), Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    
    // Set Authorization and Cookie headers
    headers.insert(COOKIE, args.cookie_value.parse()?);
    headers.insert(AUTHORIZATION, format!("Bearer {}", &args.token).parse()?);
    headers.insert("Content-Type", "application/json".parse()?);

    // Invalid certs part is needed for our locahost test instance
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)  
        .build()?;

    // Construct the request body with optional parameters
    let body = json!({
        "operational_options": operational_options.unwrap_or_default(),
        "input_parameters": input_parameters.unwrap_or_default(),
        "restart": restart.unwrap_or(false),
        "reana_specification": reana_specification.unwrap_or_default()
    });

    let url = format!("{}/api/workflows/{}/start", &args.instance, workflow_name);

    // Send the POST request
    let response = client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()?;

    // Check for a successful response
    if response.status().is_success() {
        let response_text = response.text()?;
        println!("Start workflow response: {}", response_text);
    } else {
        // Print error or throw it
        let error_message = response.text()?;
        eprintln!("Failed to start workflow. Error: {}", error_message);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, error_message)));
    }

    Ok(())
}


pub fn get_workflow_status(
    args: &RemoteExecuteArgs,
    workflow_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let url = format!("{}/api/workflows/{}/status", &args.instance, workflow_id);

    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, args.cookie_value.parse()?);
    headers.insert(AUTHORIZATION, format!("Bearer {}", &args.token).parse()?);

    let client = Client::builder()
        .default_headers(headers)
        .danger_accept_invalid_certs(true)
        .build()?;

    let response = client.get(&url).send()?;
    let json_response: Value = response.json()?;
    
    Ok(json_response)
}



fn sanitize_path(path: &str) -> String {
    let path = Path::new(path);
    
    let sanitized_path = path
        .components()
        .filter_map(|comp| match comp {
            std::path::Component::ParentDir => None, 
            _ => Some(comp.as_os_str()),
        })
        .collect::<std::path::PathBuf>();

    sanitized_path.to_string_lossy().replace("\\", std::path::MAIN_SEPARATOR_STR)
}

pub fn convert_reana_to_json(reana_path: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let reana_content = std::fs::read_to_string(reana_path)?;
    let reana_data: ReanaYaml = serde_yaml::from_str(&reana_content)?;

    let mut parameters = std::collections::HashMap::new();

    if let Some(ref param_value) = reana_data.inputs.parameters {
        if let Some(param_map) = param_value.as_mapping() {
            for (key, value) in param_map {
                match value {
                    serde_yaml::Value::String(param_file_path) => {

                        let file_content = std::fs::read_to_string(param_file_path)?;
                        let parsed: std::collections::HashMap<String, Parameter> = serde_yaml::from_str(&file_content)?;
                        parameters.extend(parsed);
                    }
                    _ => {

                        let key_str = key.as_str().unwrap_or("").to_string();
                        let param: Parameter = serde_yaml::from_value(value.clone())?;
                        parameters.insert(key_str, param);
                    }
                }
            }
        }
    }

    let workflow_inputs = WorkflowInputs {
        directories: reana_data.inputs.directories.clone(),
        files: reana_data.inputs.files.clone(),
        parameters: serde_yaml::to_value(parameters)?,
    };

    let workflow_cwl_path = &reana_data.workflow.file;
    let workflow_content = std::fs::read_to_string(workflow_cwl_path)?;
    let workflow_spec: CWLWorkflow = serde_yaml::from_str(&workflow_content)?;
    let mut graph = Vec::new();

    if let Ok(json_string) = convert_cwl_to_json(workflow_cwl_path) {
        graph.push(json_string);
    }

    for step in &workflow_spec.steps { 
        let l_path = std::path::Path::new(&step.run);                    
        let step_location_cwl = get_location(workflow_cwl_path, l_path)?;
        if let Ok(json_string) = convert_command_line_tool_cwl_to_json(&step_location_cwl) {
            graph.push(json_string);
        }
    }
    let workflow_json = WorkflowJson {
        inputs: workflow_inputs,
        outputs: reana_data.outputs,
        version: reana_data.version,
        workflow: WorkflowSpec {
            file: workflow_cwl_path.clone(),
            specification: CWLGraph {
                graph,
                cwl_version: workflow_spec.cwl_version.clone(),
            },
            r#type: reana_data.workflow.r#type,
        },
    };
    println!("workflow_json {:?}", workflow_json);
    let json_content = serde_json::to_string_pretty(&workflow_json)?; 
    fs::write("workflow.json", json_content)?; 
    Ok(serde_json::json!(workflow_json))
}


fn convert_cwl_to_json(cwl_path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let cwl_content = fs::read_to_string(cwl_path)?;
    let current_dir = env::current_dir()?;
    let full_cwl_path = current_dir.join(cwl_path);
    
    let workflow: CWLWorkflow = match serde_yaml::from_str(&cwl_content) {
        Ok(parsed) => parsed,
        Err(e) => {
            println!("❌ Failed to parse CWL YAML: {}", e);
            return Err(Box::new(e));
        }
    };
 
    let formatted_inputs: Vec<_> = workflow.inputs.iter().map(|input| {
        let mut input_json = serde_json::json!({
            "id": format!("#main/{}", input.id),
            "type": input.r#type
        });
    
        if let Some(default) = &input.default {
            let l_path = Path::new(default.location.as_str());
            let location = match get_location(&full_cwl_path.to_string_lossy(), l_path) {
                Ok(loc) => {
                    format!("file://{}", loc)
                }
                Err(e) => {
                    println!("⚠️ Could not resolve location for '{}': {}", input.id, e);
                    "file://No location".to_string()
                }
            };
    
            input_json["default"] = serde_json::json!({
                "class": "File",
                "location": location
            });
        }
    
        input_json
    }).collect();

    let formatted_outputs: Vec<_> = workflow.outputs.iter().map(|output| {
        serde_json::json!({
            "id": format!("#main/{}", output.id),
            "outputSource": format!("#main/{}/output", output.id.replace("_output", "")),
            "type": "File"
        })
    }).collect();

    let formatted_steps: Vec<_> = workflow.steps.iter().map(|step| {
        let formatted_inputs: Vec<_> = step.r#in.iter().map(|(key, value)| {
            serde_json::json!({
                "id": format!("#main/{}/{}", step.id, key),
                "source": format!("#main/{}", value)
            })
        }).collect();

        let formatted_outputs: Vec<_> = step.out.iter().map(|output| {
            serde_json::json!({
                "id": format!("#main/{}/{}", step.id, output.id())
            })
        }).collect();
        
        let run_parts: Vec<&str> = step.run.split(MAIN_SEPARATOR).collect();
        
        serde_json::json!({
            "id": format!("#main/{}", step.id),
            "in": formatted_inputs,
            "out": formatted_outputs,
            "run": format!("#{}", run_parts.get(1).copied().unwrap_or(step.run.as_str()))
        })
    }).collect();

    let workflow_json = serde_json::json!({
        "class": "Workflow",
        "id": "#main",
        "inputs": formatted_inputs,
        "outputs": formatted_outputs,
        "steps": formatted_steps,
        "cwlVersion": workflow.cwl_version
    });
    Ok(workflow_json)
}

fn read_file_content(file_path: &str) -> Result<String, io::Error> {
    let mut file = std::fs::File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn convert_command_line_tool_cwl_to_json(cwl_path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let cwl_content = fs::read_to_string(cwl_path)?;
    let current_dir = env::current_dir()?;
    let full_cwl_path = current_dir.join(cwl_path);

    let cwl_path_parts: Vec<&str> = cwl_path.split(MAIN_SEPARATOR).collect();
    let tool_name = cwl_path_parts.last().copied().unwrap_or(cwl_path);

    let command_line_tool: CWLCommandLineTool = match serde_yaml::from_str(&cwl_content) {
        Ok(parsed) => parsed,
        Err(e) => {
            println!("❌ Failed to parse CWL YAML: {}", e);
            return Err(Box::new(e));
        }
    };

    let formatted_inputs: Vec<_> = command_line_tool.inputs.iter().map(|input| {
        let input_id = format!("#{}/{}", tool_name, input.id);

        let mut input_json = serde_json::json!({
            "id": input_id,
            "type": input.r#type
        });

        if let Some(default_file) = &input.default {
            let location_path = Path::new(&default_file.location);
            if let Ok(resolved_location) = get_location(&full_cwl_path.to_string_lossy(), location_path) {
                input_json["default"] = serde_json::json!({
                    "class": "File",
                    "location": format!("file://{}", resolved_location)
                });
            }
        }

        if let Some(binding) = &input.input_binding {
            input_json["inputBinding"] = serde_json::json!({
                "prefix": binding.prefix
            });
        }

        input_json
    }).collect();

        
    let formatted_outputs: Vec<_> = command_line_tool.outputs.iter().map(|output| {
        serde_json::json!( {
            "id": format!("#{}/{}",tool_name, output.id),
            "outputBinding": {
                "glob": output.output_binding.as_ref().map_or("".to_string(), |binding| binding.glob.clone())
            },
            "type": "File"
        })
    }).collect();

    let formatted_requirements: Vec<_> = command_line_tool.requirements.as_ref().map_or(vec![], |reqs| {
        reqs.iter().map(|req| {
            match req.class.as_str() {
                "DockerRequirement" => {
                    serde_json::json!({
                        "class": "DockerRequirement",
                        "dockerPull": req.docker_pull
                    })
                },
                "InitialWorkDirRequirement" => {
                    if let Some(listing) = &req.listing {
                        let formatted_listing: Result<Vec<_>, io::Error> = listing.iter().map(|entry| {
                            match &entry.entry {
                                CWLEntry::Include { include } => {
                                    let entry_path = Path::new(include);
                                    let file_location = get_location(&full_cwl_path.to_string_lossy(), entry_path)
.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
                                    read_file_content(&file_location).map(|file_contents| {
                                        serde_json::json!({
                                            "entry": file_contents,
                                            "entryname": entry.entryname
                                        })
                                    })
                                }
                                CWLEntry::Expression(expr) => {
                                    Ok(serde_json::json!({
                                        "entry": expr,
                                        "entryname": entry.entryname
                                    }))
                                }
                            }
                        }).collect();
                        match formatted_listing {
                            Ok(list) => serde_json::json!({
                                "class": "InitialWorkDirRequirement",
                                "listing": list
                            }),
                            Err(e) => {
                                eprintln!("Error reading file contents: {}", e);
                                serde_json::json!({
                                    "class": "InitialWorkDirRequirement"
                                })
                            }
                        }
                    } else {
                        serde_json::json!( {
                            "class": "InitialWorkDirRequirement"
                        })
                    }
                },
                _ => serde_json::json!({})
            }
        }).collect()
    });

    let command_line_tool_json = serde_json::json!( {
        "class": "CommandLineTool",
        "id": format!("#{}", tool_name),
        "baseCommand": command_line_tool.base_command,
        "inputs": formatted_inputs,
        "outputs": formatted_outputs,
        "requirements": formatted_requirements,
        "label": command_line_tool.label,
    });

    Ok(command_line_tool_json)
}

fn collect_files_recursive(dir: &Path, files: &mut Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_path = entry.path();

        if file_path.is_dir() {
            collect_files_recursive(&file_path, files)?;
        } else if file_path.is_file() {
            if let Some(file_str) = file_path.to_str() {
                files.push(file_str.to_string());
            }
        }
    }
    Ok(())
}

pub fn upload_files(
    args: &RemoteExecuteArgs,
    workflow_name: &str,
    reana_yaml: &Value,
) -> Result<(), Box<dyn Error>> {
    let input_yaml = &args.input_file;
    let mut files = if let Some(inputs) = reana_yaml.get("inputs") {
        if let Some(Value::Sequence(file_list)) = inputs.get("files") {
            file_list.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    if let Some(inputs) = reana_yaml.get("inputs") {
        if let Some(Value::Sequence(dir_list)) = inputs.get("directories") {
            for dir in dir_list {
                if let Some(dir_path) = dir.as_str() {
                    let path = Path::new(dir_path);

                    if path.exists() && path.is_dir() {
                        for entry in fs::read_dir(path)? {
                            let entry = entry?;
                            let file_path = entry.path();
                            if file_path.is_dir() {
                                collect_files_recursive(&file_path, &mut files)?;
                            } else if file_path.is_file() {
                                if let Some(file_str) = file_path.to_str() {
                                    files.push(file_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    files.push("reana.yaml".to_string());
    if files.is_empty() {
        println!("No files to upload found in reana.yaml.");
        return Ok(());
    }

   let mut headers = HeaderMap::new();
   headers.insert(COOKIE, args.cookie_value.parse()?);
   headers.insert(CONTENT_TYPE, "application/octet-stream".parse()?);
   headers.insert(AUTHORIZATION, format!("Bearer {}", &args.token).parse()?);

   let client = ClientBuilder::new()
   .default_headers(headers.clone())
   .danger_accept_invalid_certs(true)
   .build()?;

    // Upload each file
    for file_name in files {
        let mut file_path = PathBuf::from(&file_name); 

        // If file doesn't exist, attempt to reconstruct the path
        if !file_path.exists() {
            if let Some(input_yaml_path) = input_yaml {
                if let Some(parent) = Path::new(input_yaml_path).parent() {
                    file_path = parent.join(file_name.clone()); 
                }
            }
            else {
                let cwl_path = &args.file; 
                if let Some(parent) = Path::new(cwl_path).parent() {
                    file_path = parent.join(file_name.clone()); 
                }
            }
        }
    
        if !file_path.exists() {
            eprintln!("Warning: File not found - {:?}", file_path);
        }

        let mut file = std::fs::File::open(&file_path)?;
        let mut file_content = Vec::new();
        file.read_to_end(&mut file_content)?;
    
        let upload_url = format!("{}/api/workflows/{}/workspace?file_name={}", &args.instance, workflow_name, file_name);
        
        let response = client
            .post(&upload_url)
            .headers(headers.clone())
            .body(file_content)
            .send()?;
    
        let response_text = response.text()?;
        println!("File Upload Response: {}", response_text);
    }

    Ok(())
}

pub fn download_files(args: &RemoteExecuteArgs, workflow_name: &str, reana_yaml: &Value,) -> Result<(), Box<dyn Error>> {
    let files = if let Some(outputs) = reana_yaml.get("outputs") {
        if let Some(Value::Sequence(file_list)) = outputs.get("files") {
            file_list.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    if files.is_empty() {
        println!("No files to download found in reana.yaml.");
        return Ok(());
    }

    let client = Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, args.cookie_value.parse()?);
    headers.insert(AUTHORIZATION, format!("Bearer {}", &args.token).parse()?);

    for file_name in files {
        let url = format!("{}/api/workflows/{}/workspace/outputs/{}", &args.instance, workflow_name, file_name);

        let response = client.get(&url)
            .headers(headers.clone())
            .send()?;

        if response.status().is_success() {
            let file_path = Path::new(&file_name)
                .file_name()
                .ok_or("Failed to extract file name")?
                .to_str()
                .ok_or("Invalid UTF-8 in file name")?
                .to_string();

            let mut file = std::fs::File::create(&file_path)?;
            let content = response.bytes()?;
            file.write_all(&content)?;

            println!("Downloaded: {}", file_path);
        } else {
            println!("Failed to download {}. Response: {:?}", file_name, response.text()?);
        }
    }

    Ok(())
}

fn build_inputs_yaml(input_yaml_path: &str) -> Result<Mapping, Box<dyn Error>> {
    let input_yaml = fs::read_to_string(input_yaml_path)?;
    let input_value: Value = serde_yaml::from_str(&input_yaml)?;

    let mut files: HashSet<String> = HashSet::new();
    let mut directories: HashSet<String> = HashSet::new();
    
    let mut parameters: HashMap<String, Value> = HashMap::new();

    if let Value::Mapping(mapping) = input_value {
        for (key, value) in mapping {
            if let Value::String(key_str) = key {
                if let Value::Mapping(sub_mapping) = value {
                    if let Some(Value::String(class)) = sub_mapping.get(Value::String("class".to_string())) {
                        let location = sub_mapping
                            .get(Value::String("location".to_string()))
                            .and_then(|v| v.as_str())
                            .or_else(|| sub_mapping.get(Value::String("path".to_string())).and_then(|v| v.as_str()));
                        if let Some(location) = location {
                            let sanitized_location = sanitize_path(location);
                            match class.as_str() {
                                "File" => {
                                    let parent_dir = Path::new(&sanitized_location).parent().and_then(|p| p.to_str());
                                    if let Some(parent) = parent_dir {
                                        if !directories.contains(parent) {
                                            files.insert(sanitized_location);
                                        }
                                    } else {
                                        files.insert(sanitized_location);
                                    }
                                },
                                "Directory" => {
                                    directories.insert(sanitized_location);
                                },
                                _ => {}
                            }
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

    parameters.insert("input".to_string(), Value::String(input_yaml_path.to_string()));
    if let Some(parent) = Path::new(input_yaml_path).parent() {
        if let Some(parent_str) = parent.to_str() {
            directories.insert(parent_str.to_string());
        }
    }
    // Build the inputs mapping.
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

fn find_input_location(cwl_file_path: &str, id: &str) -> Result<Option<String>, Box<dyn Error>> {
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

fn load_cwl_file(base_path: &str, cwl_file_path: &Path) -> Result<Value, Box<dyn Error>> {
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

fn get_location(base_path: &str, cwl_file_path: &Path) -> Result<String, Box<dyn Error>> {
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


fn build_inputs_cwl(cwl_input_path: &str, inputs_yaml: Option<&String>) -> Result<Mapping, Box<dyn Error>> {
    let cwl_content = fs::read_to_string(cwl_input_path)?;
    let cwl_value: Value = serde_yaml::from_str(&cwl_content)?;

    let mut files: HashSet<String> = HashSet::new();
    let mut directories: HashSet<String> = HashSet::new();
    let mut parameters: HashMap<String, Value> = HashMap::new();

    if let Some(inputs) = cwl_value.get("inputs").and_then(|v| v.as_sequence()) {
        for input in inputs {
            if let Some(id) = input.get("id").and_then(|v| v.as_str()) {
                if let Some(input_type) = input.get("type").and_then(|v| v.as_str()) {
                    if input_type == "File" || input_type == "Directory" {
                        if let Some(default) = input.get("default") {
                            if let Some(location) = default.get("location").and_then(|v| v.as_str()) {
                                let sanitized_location = sanitize_path(location);
                                match input_type {
                                    "File" => {
                                        let parent_dir = Path::new(&sanitized_location).parent().and_then(|p| p.to_str());
                                        if let Some(parent) = parent_dir {
                                            if !directories.contains(parent) {
                                                files.insert(sanitized_location);
                                            }
                                        } else {
                                            files.insert(sanitized_location);
                                        }
                                    },
                                    "Directory" => {
                                        let mut should_add = true;
                                        for file in &files {
                                            if file.starts_with(&sanitized_location) {
                                                should_add = false;
                                                break;
                                            }
                                        }
                                        if should_add {
                                            directories.insert(sanitized_location);
                                        }
                                    },
                                    _ => {}
                                }
                            }
                        } else {
                            let location = find_input_location(cwl_input_path, id)?;
                            
                            if let Some(location) = location {
                                let sanitized_location = sanitize_path(&location);
                                match input_type {
                                    "File" => files.insert(sanitized_location),
                                    "Directory" => directories.insert(sanitized_location),
                                    _ => None::<Value>.is_some(),
                                };
                            } else {
                                println!("Input with id {} has no location and no default!", id);
                            }
                        }
                    } else {
                        parameters.insert(id.to_string(), input.clone());
                    }
                }
            }
        }
    }
    if let Some(yaml_path) = inputs_yaml {
        parameters.insert("inputs.yaml".to_string(), Value::String(yaml_path.to_string()));
    }
    if let Some(parent) = Path::new(cwl_input_path).parent() {
        if let Some(parent_str) = parent.to_str() {
            directories.insert(parent_str.to_string());
        }
    }
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


pub fn create_reana_yaml(args: &RemoteExecuteArgs) -> Result<Value, Box<dyn Error>> {
    let inputs_section = if let Some(input_yaml) = &args.input_file {
        build_inputs_yaml(input_yaml)?
    } else {
        build_inputs_cwl(
            args.file.to_str().ok_or("Invalid workflow file path")?,
            args.input_file.as_ref(),
        )?
    };
    let workflow_file = args.file.to_str().ok_or("Invalid UTF-8 in workflow path")?;
    let output_files: Vec<Value> = get_all_outputs(workflow_file)?
        .into_iter()
        .map(|(_, glob_value)| Value::String(glob_value))
        .collect();

    let reana_yaml = {
        let mut yaml = Mapping::new();
        yaml.insert(
            Value::String("inputs".to_string()),
            Value::Mapping(inputs_section),
        );
        let mut outputs_section = Mapping::new();
        outputs_section.insert(Value::String("files".to_string()), Value::Sequence(output_files));
        yaml.insert(
            Value::String("outputs".to_string()),
            Value::Mapping(outputs_section),
        );
        let mut workflow_section = Mapping::new();
        workflow_section.insert(Value::String("type".to_string()), Value::String("cwl".to_string()));
        workflow_section.insert(Value::String("file".to_string()), Value::String(workflow_file.to_string()));
        yaml.insert(
            Value::String("workflow".to_string()),
            Value::Mapping(workflow_section),
        );
        yaml.insert(
            Value::String("version".to_string()),
            Value::String("0.9.3".to_string()),
        );
        yaml
    };

    let yaml_string = serde_yaml::to_string(&reana_yaml)?;
    fs::write("reana.yaml", &yaml_string)?;

    Ok(Value::Mapping(reana_yaml))
}

fn get_all_outputs(main_workflow_path: &str) -> Result<Vec<(String, String)>, Box<dyn Error>> {
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

/*
fn get_workflow_specification(
    args: &RemoteExecuteArgs,
    workflow_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let url = format!("{}/api/workflows/{}/specification", &args.instance, workflow_id);

    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, args.cookie_value.parse()?);
    headers.insert(AUTHORIZATION, format!("Bearer {}", &args.token).parse()?);

    let client = Client::builder()
        .default_headers(headers)
        .danger_accept_invalid_certs(true)
        .build()?;

    let response = client.get(&url).send()?;
    println!("response {:?}", response);

    if response.status().is_success() {
        let json: Value = response.json()?;
        Ok(json)
    } else {
        let err_msg = response.text()?;
        Err(err_msg.into())
    }
}
*/