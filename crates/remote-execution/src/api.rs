use serde_yaml:: Value;
use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::PathBuf,
    io::{Read, Write}
};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE, COOKIE};
use serde_json::json;
use std::path::Path;
use std::collections::HashSet;
use reqwest::blocking::ClientBuilder;
use crate::utils::{collect_files_recursive, sanitize_path, get_location, load_cwl_file2, load_yaml_file, resolve_input_file_path};

pub fn create_workflow(reana_server: &str, reana_token: &str, cookie_value: &str, workflow: &serde_json::Value) -> Result<Value, Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, cookie_value.parse()?);
   
    headers.insert(AUTHORIZATION, format!("Bearer {}", reana_token).parse()?);
    headers.insert(CONTENT_TYPE, "application/json".parse()?);
    let client = Client::builder()
    .default_headers(headers.clone())
    //.danger_accept_invalid_certs(true)
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


pub fn ping_reana(reana_server: &str) -> Result<Value, Box<dyn Error>> {
    let ping_url = format!("{}/api/ping", reana_server);

    let headers = HeaderMap::new();

    // Invalid certs part is needed for our locahost test instance
    let client = Client::builder()
        .default_headers(headers)
        //.danger_accept_invalid_certs(true)
        .build()?;

    let response = client.get(&ping_url).send()?;
    let json_response: Value = response.json()?;
    Ok(json_response)
}

pub fn start_workflow(reana_server: &str, reana_token: &str, cookie_value: &str,
    workflow_name: &str, operational_options: Option<HashMap<String, Value>>,
    input_parameters: Option<HashMap<String, Value>>, restart: bool, reana_specification: Value,
) -> Result<Value, Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    
    // Set Authorization and Cookie headers
    headers.insert(COOKIE, cookie_value.parse()?);
    headers.insert(AUTHORIZATION, format!("Bearer {}", reana_token).parse()?);
    headers.insert("Content-Type", "application/json".parse()?);

    // Invalid certs part is needed for our locahost test instance
    let client = ClientBuilder::new()
        //.danger_accept_invalid_certs(true)  
        .build()?;

    // Construct the request body with optional parameters
    let body = json!({
        "operational_options": operational_options.unwrap_or_default(),
        "input_parameters": input_parameters.unwrap_or_default(),
        "restart": restart,
        "reana_specification": reana_specification
    });

    let url = format!("{}/api/workflows/{}/start", &reana_server, workflow_name);

    // Send the POST request
    let response = client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()?;

    let json_response: Value = response.json()?;
    Ok(json_response)
}


pub fn get_workflow_status(reana_server: &str, reana_token: &str, cookie_value: &str, workflow_id: &str) -> Result<Value, Box<dyn Error>> {
    let url = format!("{}/api/workflows/{}/status", &reana_server, workflow_id);

    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, cookie_value.parse()?);
    headers.insert(AUTHORIZATION, format!("Bearer {}", &reana_token).parse()?);

    let client = Client::builder()
        .default_headers(headers)
        //.danger_accept_invalid_certs(true)
        .build()?;

    let response = client.get(&url).send()?;
    let json_response: Value = response.json()?;
    
    Ok(json_response)
}


pub fn upload_files(reana_server: &str, reana_token: &str, cookie_value: &str, input_yaml: &Option<String>, file: &PathBuf,
    workflow_name: &str, workflow_json: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    //let input_yaml = &args.input_file;
    let mut files: HashSet<String> = HashSet::new();
    let i: Option<Value> = if let Some(ref input_file_path) = &input_yaml {
        Some(load_yaml_file(Path::new(input_file_path))?)
    } else {
        None
    };
    let base_path = std::env::current_dir()?.to_string_lossy().to_string();
    let cwl_yaml: Value = load_cwl_file2(&base_path, &file)?;
    if let Some(inputs) = workflow_json.get("inputs") {
        if let Some(serde_json::Value::Array(file_list)) = inputs.get("files") {
            for f in file_list.iter().filter_map(|v| v.as_str()) {
                files.insert(f.to_string());
            }
        }
        if let Some(serde_json::Value::Object(params)) = inputs.get("parameters") {
            for (_key, val) in params {
                if let Some(class) = val.get("class").and_then(|v| v.as_str()) {
                    if class == "File" || class == "Directory" {
                        if let Some(loc) = val.get("location").and_then(|v| v.as_str()) {
                            if class == "File" {
                                files.insert(loc.to_string());
                            }
                        }
                    }
                }
            }
        }

        if let Some(serde_json::Value::Array(dir_list)) = inputs.get("directories") {
            for dir in dir_list.iter().filter_map(|v| v.as_str()) {
                let path = Path::new(dir);
                if path.exists() && path.is_dir() {
                    for entry in fs::read_dir(path)? {
                        let entry = entry?;
                        let file_path = entry.path();
                        if file_path.is_dir() {
                            collect_files_recursive(&file_path, &mut files)?;
                        } else if file_path.is_file() {
                            if let Some(file_str) = file_path.to_str() {
                                files.insert(file_str.to_string());
                            }
                        }
                    }
                }
                else {
                    let resolved = resolve_input_file_path(
                        path.to_string_lossy().as_ref(),
                        i.as_ref(),     
                        Some(&cwl_yaml),
                    );           
                    if let Some(resolved_path) = resolved {
                        let cwd = std::env::current_dir()?;

                        let base_path = if let Some(input_yaml_str) = &input_yaml {
                            cwd.join(input_yaml_str)
                        } else {
                            cwd.join(&file)
                        };
                
                        let path_str = base_path.to_string_lossy().to_string();
                
                        let l = get_location(
                            &path_str,
                            Path::new(&resolved_path),
                        )?;
                        let path = std::path::PathBuf::from(l);              
                        if path.exists() && path.is_dir() {
                            for entry in fs::read_dir(path)? {
                                let entry = entry?;
                                let file_path = entry.path();
                                if file_path.is_dir() {
                                    collect_files_recursive(&file_path, &mut files)?;
                                } else if file_path.is_file() {
                                    let relative = file_path.strip_prefix(&cwd).unwrap_or(&file_path);
                                    if let Some(file_str) = relative.to_str() {
                                        files.insert(file_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if files.is_empty() {
        println!("No files to upload found in workflow JSON.");
        return Ok(());
    }

    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, cookie_value.parse()?);
    headers.insert(CONTENT_TYPE, "application/octet-stream".parse()?);
    headers.insert(AUTHORIZATION, format!("Bearer {}", reana_token).parse()?);

    let client = ClientBuilder::new()
        .default_headers(headers.clone())
        .build()?;

    for file_name in files {
        let mut file_path = PathBuf::from(&file_name);
        if !file_path.exists() {
            let resolved = resolve_input_file_path(
                file_path.to_string_lossy().as_ref(),
                i.as_ref(),        
                Some(&cwl_yaml),
            );
            if let Some(resolved_path) = resolved {
                let cwd = std::env::current_dir()?;
                let base_path = if let Some(input_yaml_str) = &input_yaml {
                    cwd.join(input_yaml_str)
                } else {
                    cwd.join(&file)
                };
        
                let path_str = base_path.to_string_lossy().to_string();
        
                let l =  get_location(
                    &path_str,
                    Path::new(&resolved_path),
                )?;
                file_path = std::path::PathBuf::from(l);
            }
        }

        let mut file = std::fs::File::open(&file_path)?;
        let mut file_content = Vec::new();
        file.read_to_end(&mut file_content)?;
        let name = pathdiff::diff_paths(&file_name, std::env::current_dir()?).unwrap_or_else(|| Path::new(&file_name).to_path_buf());           

        let upload_url = format!(
            "{}/api/workflows/{}/workspace?file_name={}",
            &reana_server, workflow_name, sanitize_path(&name.to_string_lossy())
        );

        let response = client
            .post(&upload_url)
            .headers(headers.clone())
            .body(file_content)
            .send()?;

        let _response_text = response.text()?;
    }

    Ok(())
}



pub fn download_files(reana_server: &str, reana_token: &str, cookie_value: &str, workflow_name: &str, workflow_json: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    let files = workflow_json
        .get("outputs")
        .and_then(|outputs| outputs.get("files"))
        .and_then(|files| files.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    if files.is_empty() {
        println!("No files to download found in workflow.json.");
        return Ok(());
    }

    let client = Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, cookie_value.parse()?);
    headers.insert(AUTHORIZATION, format!("Bearer {}", &reana_token).parse()?);

    for file_name in files {
        let url = format!(
            "{}/api/workflows/{}/workspace/outputs/{}",
            &reana_server, workflow_name, file_name
        );

        let response = client.get(&url).headers(headers.clone()).send()?;

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
            println!("❌ Failed to download {}. Response: {:?}", file_name, response.text()?);
        }
    }

    Ok(())
}