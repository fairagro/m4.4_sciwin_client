use crate::utils::{build_inputs_cwl, build_inputs_yaml, get_all_outputs, strip_temp_prefix};
use anyhow::{anyhow, bail, Context, Result};
use commonwl::execution::utils::{clone_from_rocrate_or_cwl, find_cwl_in_rocrate, unzip_rocrate, verify_cwl_references};
use commonwl::{load_doc, packed::pack_workflow, CWLDocument};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowJson {
    pub inputs: WorkflowInputs,
    pub outputs: WorkflowOutputs,
    pub version: String,
    pub workflow: WorkflowSpec,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowOutputs {
    pub files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowSpec {
    pub file: String,
    pub specification: commonwl::packed::PackedCWL,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowInputs {
    pub directories: Vec<String>,
    pub files: Vec<String>,
    pub parameters: Value,
}

pub fn generate_workflow_json_from_input(
    input: &Path,
    input_yaml: Option<&Path>,
) -> Result<(WorkflowJson, PathBuf, Option<TempDir>, Option<PathBuf>)> {
    let mut temp_dir: Option<TempDir> = None;
    let mut chosen_input_yaml: Option<PathBuf> = input_yaml.map(|p| p.to_path_buf());
    if input.is_dir() {
        if chosen_input_yaml.is_none() {
            let auto_yaml = ["inputs.yml", "inputs.yaml"].iter().map(|n| input.join(n)).find(|p| p.exists());
            chosen_input_yaml = auto_yaml;
        }
        let cwl_path = find_cwl_in_rocrate(input)?;
        if !verify_cwl_references(&cwl_path)? {
            let (tmp, cloned_cwl, cloned_inputs) = clone_from_rocrate_or_cwl(&input.join("ro-crate-metadata.json"), &cwl_path)?;
            temp_dir = Some(tmp);
            if let Some(cloned) = cloned_cwl {
                let wf = build_workflow_json_from_cwl(&cloned, cloned_inputs.as_deref())?;
                return Ok((wf, cloned, temp_dir, cloned_inputs));
            }
            if let Some(cloned_inputs) = cloned_inputs {
                chosen_input_yaml = Some(cloned_inputs);
            }
        }
        let wf = build_workflow_json_from_cwl(&cwl_path, chosen_input_yaml.as_deref())?;
        return Ok((wf, cwl_path, temp_dir, chosen_input_yaml));
    }
    if input.is_file() {
        let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "zip" => {
                let tmp = tempdir().context("Failed to create temporary directory")?;
                unzip_rocrate(input, tmp.path())?;
                let file_stem = input
                    .file_stem()
                    .map(|s| s.to_os_string())
                    .unwrap_or_else(|| input.as_os_str().to_os_string());
                let rocrate_root = tmp.path().join(file_stem);
                if chosen_input_yaml.is_none() {
                    let auto_yaml = ["inputs.yml", "inputs.yaml"].iter().map(|n| rocrate_root.join(n)).find(|p| p.exists());
                    chosen_input_yaml = auto_yaml;
                }
                let cwl_path = find_cwl_in_rocrate(&rocrate_root)?;
                if !verify_cwl_references(&cwl_path)? {
                    let (cloned_tmp, cloned_cwl, cloned_inputs) = clone_from_rocrate_or_cwl(&rocrate_root.join("ro-crate-metadata.json"), &cwl_path)?;
                    temp_dir = Some(cloned_tmp);
                    if let Some(cloned) = cloned_cwl {
                        let wf = build_workflow_json_from_cwl(&cloned, cloned_inputs.as_deref())?;
                        return Ok((wf, cloned, temp_dir, cloned_inputs));
                    }
                    if let Some(cloned_inputs) = cloned_inputs {
                        chosen_input_yaml = Some(cloned_inputs);
                    }
                } else {
                    temp_dir = Some(tmp);
                }
                let wf = build_workflow_json_from_cwl(&cwl_path, chosen_input_yaml.as_deref())?;
                return Ok((wf, cwl_path, temp_dir, chosen_input_yaml));
            }

            "cwl" => {
                let wf = build_workflow_json_from_cwl(input, chosen_input_yaml.as_deref())?;
                return Ok((wf, input.to_path_buf(), None, chosen_input_yaml));
            }

            _ => bail!("Unsupported input type: {input:?}"),
        }
    }
    bail!("Invalid input path: {input:?}")
}

fn build_workflow_json_from_cwl(cwl_path: &Path, input_yaml: Option<&Path>) -> Result<WorkflowJson> {
    let cwl_str = cwl_path.to_str().ok_or_else(|| anyhow!("Non-UTF8 CWL path: {}", cwl_path.display()))?;
    let inputs_mapping = if let Some(yaml_path) = input_yaml {
        build_inputs_yaml(cwl_str, yaml_path).with_context(|| format!("Failed to build inputs from {yaml_path:?}"))?
    } else {
        build_inputs_cwl(cwl_str, None).with_context(|| format!("Failed to build inputs from CWL {cwl_path:?}"))?
    };
    let doc = load_doc(cwl_path).map_err(|e| anyhow!("Failed to load CWL document {cwl_path:?}: {e}"))?;
    let CWLDocument::Workflow(workflow) = doc else {
        bail!("CWL document is not a Workflow: {cwl_path:?}");
    };
    let packed = pack_workflow(&workflow, cwl_path, None).map_err(|e| anyhow!("Failed to pack CWL workflow: {e}"))?;
    let outputs: Vec<String> = get_all_outputs(&workflow)?.into_iter().map(|(_, f)| f).collect();
    let inputs: WorkflowInputs = serde_yaml::from_value(Value::Mapping(inputs_mapping)).context("Invalid inputs YAML")?;

    Ok(WorkflowJson {
        inputs,
        outputs: WorkflowOutputs { files: outputs },
        version: "0.9.4".to_string(),
        workflow: WorkflowSpec {
            file: cwl_str.to_string(),
            specification: packed,
            r#type: "cwl".to_string(),
        },
    })
}

pub fn normalize_workflow_paths(mut json: WorkflowJson, crate_root: &Path) -> WorkflowJson {
    fn strip_prefix(p: &str, root: &Path) -> String {
        let abs = Path::new(p);
        abs.strip_prefix(root).unwrap_or(abs).to_string_lossy().to_string()
    }
    json.inputs.directories = json
        .inputs
        .directories
        .into_iter()
        .map(|d| strip_temp_prefix(&strip_prefix(&d, crate_root)))
        .collect();
    json.inputs.files = json
        .inputs
        .files
        .into_iter()
        .map(|f| strip_temp_prefix(&strip_prefix(&f, crate_root)))
        .collect();
    json.workflow.file = strip_prefix(&json.workflow.file, crate_root);
    json
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn normalize_path(path: &str) -> String {
        Path::new(path).to_str().unwrap_or_default().replace("\\", "/")
    }

    #[test]
    fn test_generate_workflow_json_from_input_minimal() {
        use std::path::PathBuf;

        let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workflow_dir = base_dir.join("../../testdata/hello_world/workflows/main/main.cwl");
        assert!(workflow_dir.exists(), "Test workflow directory does not exist");

        // Call the new function
        let (workflow_json, cwl_file, _temp_dir, _input_yaml_file) =
            generate_workflow_json_from_input(&workflow_dir, None).expect("Expected workflow JSON generation to succeed");

        let json = serde_json::to_value(workflow_json).unwrap();

        // Basic assertions
        assert_eq!(json["version"], "0.9.4");
        assert_eq!(json["workflow"]["type"], "cwl");
        assert_eq!(json["workflow"]["file"], cwl_file.to_str().unwrap());

        let inputs = &json["inputs"];
        assert!(inputs.is_object(), "Inputs should be an object");
        println!("json {:?}", json);

        // Check 'directories'
        assert!(inputs["directories"].is_array(), "directories should be an array");
        assert_eq!(inputs["directories"].as_array().unwrap().len(), 0);
        assert_eq!(inputs["files"].as_array().unwrap().len(), 2);

        // Check 'files'
        assert!(inputs["files"].is_array(), "files should be an array");

        // Check parameters
        let parameters = &inputs["parameters"];
        assert!(parameters.is_object(), "parameters should be an object");

        assert_eq!(parameters["population"]["class"], "File");

        let population_path_value = parameters["population"].get("location").or_else(|| parameters["population"].get("path"));
        let population_path = population_path_value
            .and_then(|v| v.as_str())
            .expect("Expected parameters['population'] to have 'location' or 'path'");

        assert_eq!(normalize_path(population_path), "data/population.csv");

        assert_eq!(parameters["speakers"]["class"], "File");

        let speakers_path_value = parameters["speakers"].get("location").or_else(|| parameters["speakers"].get("path"));
        let speakers_path = speakers_path_value
            .and_then(|v| v.as_str())
            .expect("Expected parameters['speakers'] to have 'location' or 'path'");

        assert_eq!(normalize_path(speakers_path), "data/speakers_revised.csv");

        // Check outputs
        let outputs = &json["outputs"];
        assert!(outputs.is_object(), "Outputs should be an object");
        assert!(outputs["files"].is_array(), "outputs.files should be an array");
        assert_eq!(outputs["files"].as_array().unwrap().len(), 1);
        assert_eq!(outputs["files"][0], "plot/o_results");

        // Check workflow steps
        let graph = &json["workflow"]["specification"]["$graph"];
        let main = graph
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == serde_json::Value::String("#main".to_string()))
            .unwrap();
        let steps = &main["steps"];
        assert!(steps.is_array(), "Steps should be an array");
        assert!(!steps.as_array().unwrap().is_empty(), "Steps array should not be empty");

        let calculation_exists = steps.as_array().unwrap().iter().any(|step| step["id"] == "#main/calculation");
        assert!(calculation_exists, "'calculation' step is missing");

        let plot_exists = steps.as_array().unwrap().iter().any(|step| step["id"] == "#main/plot");
        assert!(plot_exists, "'plot' step is missing");
    }

    #[test]
    fn test_generate_workflow_json_from_input_with_inputs_yaml() {
        let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../");
        let workflow_dir = base_dir.join("testdata/hello_world/workflows/main/main.cwl");
        let inputs_yaml_path = base_dir.join("testdata/hello_world/inputs.yml");

        assert!(workflow_dir.exists(), "Workflow directory not found at {:?}", workflow_dir);
        assert!(inputs_yaml_path.exists(), "Inputs YAML file not found at {:?}", inputs_yaml_path);

        // Call the new function with input YAML
        let (workflow_json, cwl_file, _temp_dir, input_yaml_file) =
            generate_workflow_json_from_input(&workflow_dir, Some(&inputs_yaml_path)).expect("Expected workflow JSON generation to succeed");

        // Sanity checks
        assert!(input_yaml_file.is_some(), "Expected chosen input YAML to be returned");

        let json = serde_json::to_value(workflow_json).unwrap();

        assert_eq!(json["version"], "0.9.4");
        assert_eq!(json["workflow"]["type"], "cwl");
        assert_eq!(json["workflow"]["file"], cwl_file.to_str().unwrap());

        let inputs = &json["inputs"];
        assert!(inputs.is_object(), "Inputs should be an object");

        let parameters = &inputs["parameters"];
        assert!(parameters.is_object(), "parameters should be an object");

        assert_eq!(parameters["population"]["class"], "File");
        let population_path_value = parameters["population"]
            .get("location")
            .or_else(|| parameters["population"].get("path"))
            .expect("Expected parameters['population'] to have 'location' or 'path'");
        assert_eq!(normalize_path(population_path_value.as_str().unwrap()), "data/population.csv");

        assert_eq!(parameters["speakers"]["class"], "File");
        let speakers_path_value = parameters["speakers"]
            .get("location")
            .or_else(|| parameters["speakers"].get("path"))
            .expect("Expected parameters['speakers'] to have 'location' or 'path'");
        assert_eq!(normalize_path(speakers_path_value.as_str().unwrap()), "data/speakers_revised.csv");

        let outputs = &json["outputs"];
        assert!(outputs.is_object(), "Outputs should be an object");
        assert!(outputs["files"].is_array(), "outputs.files should be an array");
        assert_eq!(outputs["files"].as_array().unwrap().len(), 1);
        assert_eq!(outputs["files"][0], "plot/o_results");

        // Check workflow steps
        let graph = &json["workflow"]["specification"]["$graph"];
        assert!(graph.is_array(), "$graph should be an array");
        assert!(!graph.as_array().unwrap().is_empty(), "Graph should not be empty");

        let main = graph
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == serde_json::Value::String("#main".to_string()))
            .expect("Main workflow step '#main' missing");

        let steps = &main["steps"];
        assert!(steps.is_array(), "Steps should be an array");
        assert!(!steps.as_array().unwrap().is_empty(), "Steps array should not be empty");

        let calculation_exists = steps.as_array().unwrap().iter().any(|step| step["id"] == "#main/calculation");
        assert!(calculation_exists, "'calculation' step is missing");

        let plot_exists = steps.as_array().unwrap().iter().any(|step| step["id"] == "#main/plot");
        assert!(plot_exists, "'plot' step is missing");
    }
}
