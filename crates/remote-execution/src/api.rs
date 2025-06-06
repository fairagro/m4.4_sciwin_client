use crate::utils::{collect_files_recursive, get_location, load_cwl_yaml, load_yaml_file, resolve_input_file_path, sanitize_path};
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use serde_json::json;
use serde_yaml::Value;
use std::collections::HashSet;
use std::path::Path;
use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::{Read, Write},
    path::PathBuf,
};

pub fn create_workflow(reana_server: &str, reana_token: &str, workflow: &serde_json::Value) -> Result<Value, Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse()?);

    let client = Client::builder().default_headers(headers).danger_accept_invalid_certs(true).build()?;

    let url = format!("{reana_server}/api/workflows?access_token={reana_token}");

    let response = client.post(&url).json(workflow).send()?.error_for_status()?;

    Ok(response.json()?)
}

pub fn ping_reana(reana_server: &str) -> Result<Value, Box<dyn Error>> {
    let ping_url = format!("{reana_server}/api/ping");
    // Invalid certs part is needed for our locahost test instance
    let client = Client::builder().danger_accept_invalid_certs(true).build()?;

    let response = client.get(&ping_url).send()?;
    let json_response: Value = response.json()?;
    Ok(json_response)
}

pub fn start_workflow(
    reana_server: &str,
    reana_token: &str,
    workflow_name: &str,
    operational_parameters: Option<HashMap<String, Value>>,
    input_parameters: Option<HashMap<String, Value>>,
    restart: bool,
    reana_specification: Value,
) -> Result<Value, Box<dyn Error>> {
    let mut headers = HeaderMap::new();

    headers.insert("Content-Type", "application/json".parse()?);

    // Invalid certs part is needed for our locahost test instance
    let client = ClientBuilder::new().danger_accept_invalid_certs(true).build()?;

    // Construct the request body with optional parameters
    let body = json!({
        "operational_options": operational_parameters.unwrap_or_default(),
        "input_parameters": input_parameters.unwrap_or_default(),
        "restart": restart,
        "reana_specification": reana_specification
    });

    let url = format!("{}/api/workflows/{}/start?access_token={}", &reana_server, workflow_name, reana_token);

    // Send the POST request
    let response = client.post(&url).headers(headers).json(&body).send()?;

    let json_response: Value = response.json()?;
    Ok(json_response)
}

pub fn get_workflow_logs(reana_server: &str, reana_token: &str, workflow_id: &str) -> Result<Value, Box<dyn Error>> {
    let url = format!("{}/api/workflows/{}/logs?access_token={}", &reana_server, workflow_id, reana_token);

    let client = Client::builder().danger_accept_invalid_certs(true).build()?;

    let response = client.get(&url).send()?;
    let json_response: Value = response.json()?;

    Ok(json_response)
}

pub fn get_workflow_status(reana_server: &str, reana_token: &str, workflow_id: &str) -> Result<Value, Box<dyn Error>> {
    let url = format!("{}/api/workflows/{}/status?access_token={}", &reana_server, workflow_id, reana_token);

    let client = Client::builder().danger_accept_invalid_certs(true).build()?;

    let response = client.get(&url).send()?;
    let json_response: Value = response.json()?;

    Ok(json_response)
}

pub fn upload_files(
    reana_server: &str,
    reana_token: &str,
    input_yaml: &Option<String>,
    file: &PathBuf,
    workflow_name: &str,
    workflow_json: &serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    let mut files: HashSet<String> = HashSet::new();
    let i: Option<Value> = if let Some(ref input_file_path) = &input_yaml {
        Some(load_yaml_file(Path::new(input_file_path))?)
    } else {
        None
    };
    let base_path = std::env::current_dir()?.to_string_lossy().to_string();
    let cwl_yaml: Value = load_cwl_yaml(&base_path, file)?;
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
                } else {
                    let resolved = resolve_input_file_path(path.to_string_lossy().as_ref(), i.as_ref(), Some(&cwl_yaml));
                    if let Some(resolved_path) = resolved {
                        let cwd = std::env::current_dir()?;

                        let base_path = if let Some(input_yaml_str) = &input_yaml {
                            cwd.join(input_yaml_str)
                        } else {
                            cwd.join(file)
                        };

                        let path_str = base_path.to_string_lossy().to_string();

                        let l = get_location(&path_str, Path::new(&resolved_path))?;
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
    headers.insert(CONTENT_TYPE, "application/octet-stream".parse()?);

    let client = ClientBuilder::new().default_headers(headers).danger_accept_invalid_certs(true).build()?;

    for file_name in files {
        let mut file_path = PathBuf::from(&file_name);
        if !file_path.exists() {
            let resolved = resolve_input_file_path(file_path.to_string_lossy().as_ref(), i.as_ref(), Some(&cwl_yaml));
            if let Some(resolved_path) = resolved {
                let cwd = std::env::current_dir()?;
                let base_path = if let Some(input_yaml_str) = &input_yaml {
                    cwd.join(input_yaml_str)
                } else {
                    cwd.join(file)
                };

                let path_str = base_path.to_string_lossy().to_string();

                let l = get_location(&path_str, Path::new(&resolved_path))?;
                file_path = std::path::PathBuf::from(l);
            }
        }

        let mut file = std::fs::File::open(&file_path)?;
        let mut file_content = Vec::new();
        file.read_to_end(&mut file_content)?;
        let name = pathdiff::diff_paths(&file_name, std::env::current_dir()?).unwrap_or_else(|| Path::new(&file_name).to_path_buf());

        let upload_url = format!(
            "{}/api/workflows/{}/workspace?file_name={}&access_token={}",
            &reana_server,
            workflow_name,
            sanitize_path(&name.to_string_lossy()),
            reana_token
        );

        let response = client.post(&upload_url).body(file_content).send()?;

        let _response_text = response.text()?;
    }

    Ok(())
}

pub fn get_workflow_workspace(reana_server: &str, reana_token: &str, workflow_id: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let url = format!("{}/api/workflows/{}/workspace?access_token={}", &reana_server, workflow_id, reana_token);
    let client = Client::builder().danger_accept_invalid_certs(true).build()?;
    let response = client.get(&url).send()?;
    let json_response: serde_json::Value = response.json()?;

    Ok(json_response)
}

pub fn download_files(
    reana_server: &str,
    reana_token: &str,
    workflow_name: &str,
    files: &[String],
    folder: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    if files.is_empty() {
        println!("ℹ️ No files to download.");
        return Ok(());
    }
    let client = ClientBuilder::new().danger_accept_invalid_certs(true).build()?;
    if let Some(ref dir) = folder {
        std::fs::create_dir_all(dir)?;
    }
    let mut downloaded_files = vec![];
    for file_name in files {
        let url = format!("{reana_server}/api/workflows/{workflow_name}/workspace/{file_name}?access_token={reana_token}");
        let response = client.get(&url).send()?;
        if response.status().is_success() {
            let file_path_name = Path::new(file_name)
                .file_name()
                .ok_or("❌ Failed to extract file name")?
                .to_str()
                .ok_or("❌ Invalid UTF-8 in file name")?
                .to_string();

            let output_path = match &folder {
                Some(dir) => std::path::Path::new(dir).join(&file_path_name),
                None => std::path::PathBuf::from(&file_path_name),
            };
            let mut file = std::fs::File::create(&output_path)?;
            let content = response.bytes()?;
            file.write_all(&content)?;
            println!("✅ Downloaded: {}", output_path.display());
            downloaded_files.push(output_path.to_string_lossy().to_string());
        } else {
            println!("❌ Failed to download {}. Response: {:?}", file_name, response.text()?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use mockito::{self, Matcher, Server};
    use serde_json::json;
    use std::fs::{create_dir_all, write};
    use tempfile::{tempdir, NamedTempFile};

    #[test]
    fn test_ping_reana_success() {
        let mut server = Server::new();
        let _mock = server
            .mock("GET", "/api/ping")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "ok"}"#)
            .create();
        let response: serde_yaml::Value = ping_reana(&server.url()).unwrap();
        assert_eq!(response["status"], "ok");
    }

    #[test]
    fn test_start_workflow_failure() {
        use reqwest::blocking::Client;
        use serde_yaml;

        let mut server = Server::new();

        let workflow_id = "nonexistent-workflow";
        let token = "test-token";

        let expected_json = json!({
            "operational_options": {},
            "input_parameters": {},
            "restart": false,
            "reana_specification": {
                "version": "0.9.4",
                "workflow": {
                    "type": "serial",
                    "specification": {
                        "steps": []
                    }
                },
                "inputs": {},
                "outputs": {}
            }
        });

        let expected_yaml: serde_yaml::Value = serde_yaml::to_value(&expected_json).unwrap();

        let _mock = server
            .mock("POST", format!("/api/workflows/{workflow_id}/start?access_token={token}").as_str())
            .match_header("authorization", "Bearer test_token")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(expected_json.clone()))
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "Workflow not found."}"#)
            .create();

        let client = Client::new();
        let res = client
            .post(format!("{}/api/workflows/{}/start?access_token={}", &server.url(), workflow_id, token))
            .header("authorization", "Bearer test_token")
            .header("content-type", "application/json")
            .json(&expected_json)
            .send()
            .expect("request failed");

        assert_eq!(res.status(), 404);
        let json: serde_json::Value = res.json().unwrap();
        assert_eq!(json["message"], "Workflow not found.");

        let result = start_workflow(&server.url(), "test_token", workflow_id, None, None, false, expected_yaml);

        assert!(result.is_err(), "Expected error, but got Ok.");
    }

    #[test]
    fn test_start_workflow_success() {
        use reqwest::blocking::Client;
        let mut server = Server::new();
        let workflow_id = "test-workflow";
        let token = "test-token";

        let expected_json = json!({
            "operational_options": {},
            "input_parameters": {},
            "restart": false,
            "reana_specification": {
                "version": "0.9.4",
                "workflow": {
                    "type": "serial",
                    "specification": {
                        "steps": []
                    }
                },
                "inputs": {},
                "outputs": {}
            }
        });
        let _mock = server
            .mock("POST", format!("/api/workflows/{workflow_id}/start?access_token={token}").as_str())
            .match_header("authorization", "Bearer test_token")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(expected_json.clone()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "Workflow started successfully", "status": "started"}"#)
            .create();

        let client = Client::new();
        let res = client
            .post(format!("{}/api/workflows/{}/start?access_token={}", &server.url(), workflow_id, token))
            .header("authorization", "Bearer test_token")
            .header("content-type", "application/json")
            .json(&expected_json)
            .send()
            .expect("request failed");

        assert_eq!(res.status(), 200);
        let json: serde_json::Value = res.json().unwrap();
        assert_eq!(json["message"], "Workflow started successfully");
        assert_eq!(json["status"], "started");
    }

    #[test]
    fn test_create_workflow_success() {
        let server = MockServer::start();
        let workflow_payload = json!({
            "name": "test-workflow",
            "type": "serial",
            "specification": {
                "steps": []
            }
        });

        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/workflows")
                .query_param("access_token", "test-token")
                .header("content-type", "application/json")
                .json_body(workflow_payload.clone());
            then.status(201).json_body(json!({
                "message": "Workflow created",
                "workflow_id": "1234"
            }));
        });

        let result = create_workflow(&server.base_url(), "test-token", &workflow_payload);

        assert!(result.is_ok());
        let json_response = result.unwrap();
        assert_eq!(json_response["message"], "Workflow created");
        assert_eq!(json_response["workflow_id"], "1234");

        mock.assert();
    }

    #[test]
    fn test_create_workflow_failure_invalid_token() {
        let server = MockServer::start();

        let workflow_payload = json!({
            "name": "fail-case",
            "type": "serial",
            "specification": {
                "steps": []
            }
        });

        let _mock = server.mock(|when, then| {
            when.method(POST).path("/api/workflows");
            then.status(401).json_body(json!({ "message": "Unauthorized" }));
        });

        let result = create_workflow(&server.base_url(), "invalid_token", &workflow_payload);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_workflow_status_success() {
        let server = MockServer::start();

        let workflow_id = "123";
        let access_token = "test_token";

        let _mock = server.mock(|when, then| {
            when.method("GET")
                .path(format!("/api/workflows/{workflow_id}/status"))
                .query_param("access_token", access_token);
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"status": "completed"}"#);
        });

        let result = get_workflow_status(&server.base_url(), access_token, workflow_id);

        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["status"], "completed");
    }

    #[test]
    fn test_get_workflow_status_failure() {
        let server = MockServer::start();

        let workflow_id = "999";
        let access_token = "test_token";

        let _mock = server.mock(|when, then| {
            when.method("GET")
                .path(format!("/api/workflows/{workflow_id}/status"))
                .query_param("access_token", access_token);
            then.status(404)
                .header("content-type", "application/json")
                .body(r#"{"error": "workflow not found"}"#);
        });

        let result = get_workflow_status(&server.base_url(), access_token, workflow_id);

        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["error"], "workflow not found");
    }

    #[test]
    fn test_upload_files() {
        let server = MockServer::start();

        let reana_token = "test-token";
        let workflow_name = "my_workflow";

        let base_dir = tempdir().unwrap();
        let data_dir = base_dir.path().join("data");
        let wf_dir = base_dir.path().join("tests/test_data/hello_world/workflows");

        create_dir_all(&data_dir).unwrap();
        create_dir_all(&wf_dir).unwrap();

        let pop_file = data_dir.join("population.csv");
        let spk_file = data_dir.join("speakers_revised.csv");
        let dir_file = wf_dir.join("hello.txt");

        write(&pop_file, "data").unwrap();
        write(&spk_file, "data").unwrap();
        write(&dir_file, "workflow file").unwrap();

        let _mock_upload = server.mock(|when, then| {
            when.method(POST)
                .path(format!("/api/workflows/{workflow_name}/workspace"))
                .query_param("access_token", reana_token)
                .query_param_exists("file_name");
            then.status(200).header("content-type", "text/plain").body("uploaded");
        });

        let workflow_json = json!({
            "inputs": {
                "directories": [ wf_dir.to_str().unwrap() ],
                "files": [
                    pop_file.to_str().unwrap(),
                    spk_file.to_str().unwrap()
                ],
                "parameters": {
                    "population": {
                        "class": "File",
                        "location": pop_file.to_str().unwrap()
                    },
                    "speakers": {
                        "class": "File",
                        "location": spk_file.to_str().unwrap()
                    }
                }
            }
        });

        let dummy_cwl = NamedTempFile::new().unwrap();
        write(dummy_cwl.path(), "cwlVersion: v1.2").unwrap();

        let result = upload_files(
            &server.base_url(),
            reana_token,
            &None,
            &dummy_cwl.path().to_path_buf(),
            workflow_name,
            &workflow_json,
        );

        assert!(result.is_ok(), "upload_files failed: {:?}", result.err());
        _mock_upload.assert_hits(3);
    }

    #[test]
    fn test_download_files_no_files() {
        let server = MockServer::start();
        let reana_token = "test-token";
        let workflow_name = "my_workflow";

        let files = vec![];

        let result = download_files(&server.base_url(), reana_token, workflow_name, &files, None);

        assert!(result.is_ok(), "download_files failed: {:?}", result.err());
    }

    #[test]
    fn test_download_files_success() {
        use httpmock::MockServer;
        use std::env;
        use std::fs;
        use tempfile::tempdir;

        let server = MockServer::start();
        let reana_token = "test-token";
        let workflow_name = "my_workflow";
        let test_filename = "results.svg";
        let test_content = "<svg>mock-content</svg>";

        let _mock = server.mock(|when, then| {
            when.method("GET")
                .path(format!("/api/workflows/{workflow_name}/workspace/{test_filename}"))
                .query_param("access_token", reana_token);
            then.status(200).header("content-type", "image/svg+xml").body(test_content);
        });
        let original_dir = env::current_dir().expect("Failed to get current dir");

        let temp_dir = tempdir().expect("Failed to create temp dir");
        env::set_current_dir(&temp_dir).expect("Failed to set current dir");
        let files = vec!["results.svg".to_string()];

        let result = download_files(&server.base_url(), reana_token, workflow_name, &files, None);

        env::set_current_dir(&original_dir).expect("Failed to restore original dir");

        assert!(result.is_ok(), "download_files failed: {:?}", result.err());

        let downloaded_path = temp_dir.path().join(test_filename);
        let contents = fs::read_to_string(&downloaded_path).expect("Failed to read downloaded file");

        assert_eq!(contents, test_content);
        _mock.assert_hits(1);
    }

    #[test]
    fn test_download_files_failure() {
        let server = MockServer::start();
        let reana_token = "test-token";
        let workflow_name = "my_workflow";
        let test_filename = "results.svg";

        let _mock = server.mock(|when, then| {
            when.method("GET")
                .path(format!("/api/workflows/{workflow_name}/workspace/{test_filename}"))
                .query_param("access_token", reana_token);
            then.status(404)
                .header("content-type", "application/json")
                .body(r#"{"error": "File not found"}"#);
        });

        let files = vec![test_filename.to_string()];
        let result = download_files(&server.base_url(), reana_token, workflow_name, &files, None);

        assert!(result.is_ok(), "download_files failed: {:?}", result.err());
        _mock.assert_hits(1);
    }
}
