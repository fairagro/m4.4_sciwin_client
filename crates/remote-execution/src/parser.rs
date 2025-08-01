use crate::utils::{build_inputs_cwl, build_inputs_yaml, get_all_outputs, get_location, load_cwl_yaml, read_file_content, sanitize_path};
use anyhow::{Context, Result};
use commonwl::{prelude::*, Command as BaseCommand};
use serde::Deserializer;
use serde::{Deserialize, Serialize, de};
use serde_json::json;
use serde_yaml::{Mapping, Value};
use std::path::MAIN_SEPARATOR;
use std::path::Path;
use std::{
    collections::{HashMap, HashSet},
    env, fs,
};

#[derive(Debug, Serialize, Deserialize)]
struct WorkflowOutputs {
    files: Vec<String>,
}

#[derive(Debug, Serialize)]
struct WorkflowJson {
    inputs: WorkflowInputs,
    outputs: WorkflowOutputs,
    version: String,
    workflow: WorkflowSpec,
}

#[derive(Debug, Serialize)]
struct WorkflowSpec {
    file: String,
    specification: CWLGraph,
    #[serde(rename = "type")]
    r#type: String,
}

#[derive(Debug, Serialize, Deserialize)]
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
#[serde(rename_all = "camelCase")]
struct CWLCommandLineTool {
    cwl_version: String,
    class: String,
    base_command: BaseCommand,
    inputs: Vec<CWLInput>,
    outputs: Vec<CWLOutput>,
    requirements: Option<Vec<CWLRequirement>>,
    label: Option<String>,
    stdout: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CWLInput {
    id: String,
    #[serde(rename = "type")]
    input_type: CWLType,
    default: Option<serde_yaml::Value>,
    input_binding: Option<CWLInputBinding>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CWLOutput {
    id: String,
    #[serde(rename = "type")]
    output_type: Option<CWLType>,
    #[serde(rename = "outputSource")]
    output_source: Option<serde_yaml::Value>,
    output_binding: Option<CWLOutputBinding>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CWLStep {
    id: String,
    #[serde(rename = "in")]
    inputs: CWLStepInputs,
    run: String,
    out: Vec<CWLStepOutput>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum CWLStepInputs {
    Map(HashMap<String, CWLStepInputSource>),
    List(Vec<CWLStepInput>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum CWLStepInputSource {
    Simple(String),
    Detailed {
        source: Option<serde_yaml::Value>,
        value_from: Option<String>,
        default: Option<serde_yaml::Value>,
    },
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

impl CWLStepOutput {
    pub fn id(&self) -> &str {
        match self {
            CWLStepOutput::Simple(id) => id,
            CWLStepOutput::Detailed { id } => id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CWLInputBinding {
    prefix: Option<String>,
    position: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CWLOutputBinding {
    glob: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CWLRequirement {
    class: String,
    docker_pull: Option<String>,
    listing: Option<Vec<CWLListing>>,
    network_access: Option<bool>,
    expression_lib: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CWLListing {
    entryname: Option<String>,
    entry: Option<serde_yaml::Value>,
}

#[derive(Debug, Serialize)]
struct CWLGraph {
    #[serde(rename = "$graph")]
    graph: Vec<serde_json::Value>,
    #[serde(rename = "cwlVersion")]
    cwl_version: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Parameter {
    pub r#class: String,
    pub location: String,
}

impl<'de> Deserialize<'de> for Parameter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(rename = "class")]
            r#class: String,
            location: Option<String>,
            path: Option<String>,
        }

        let helper = Helper::deserialize(deserializer)?;
        let location = helper
            .location
            .or(helper.path)
            .ok_or_else(|| de::Error::missing_field("location or path"))?;

        Ok(Parameter {
            r#class: helper.r#class,
            location,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    Structured(Parameter),
    Scalar(String),
}

pub fn generate_workflow_json_from_cwl(file: &Path, input_file: &Option<String>) -> Result<serde_json::Value> {
    let mut params = HashMap::new();
    let cwl_path = file.to_str().with_context(|| format!("Invalid UTF-8 in CWL file path: {file:?}"))?;

    let base_dir = env::current_dir().context("Failed to get current working directory")?;
    let base_dir_str = base_dir.to_string_lossy();

    let inputs_yaml_data = match input_file {
        Some(yaml_file) => build_inputs_yaml(cwl_path, yaml_file).with_context(|| format!("Failed to build inputs YAML from file '{yaml_file}'"))?,
        None => build_inputs_cwl(cwl_path, None).with_context(|| format!("Failed to build inputs from CWL file '{cwl_path}'"))?,
    };

    let workflow_spec_json =
        convert_cwl_to_json(cwl_path, &inputs_yaml_data).with_context(|| format!("Failed to convert CWL '{cwl_path}' and inputs to JSON"))?;

    let cwl_yaml = load_cwl_yaml(&base_dir_str, file).with_context(|| format!("Failed to load CWL YAML from '{}'", file.display()))?;

    let mut files: Vec<String> = Vec::new();
    let mut tool_output_files: HashSet<String> = HashSet::new();

    let cwl_version = workflow_spec_json
        .get("cwlVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("v1.2")
        .to_string();

    let mut graph = vec![workflow_spec_json];

    if let serde_json::Value::Object(_) = &graph[0] {
        if let Some(steps) = cwl_yaml.get("steps").and_then(|v| v.as_sequence()) {
            for step in steps {
                if let Some(run_val) = step.get("run").and_then(|v| v.as_str()) {
                    let run_path = Path::new(run_val.trim_start_matches('#'));
                    let resolved_path =
                        get_location(cwl_path, run_path).with_context(|| format!("Failed to resolve 'run' path '{run_val}' in CWL"))?;

                    let (tool_json, tool_files) = convert_command_line_tool_cwl_to_json(&resolved_path, &inputs_yaml_data, &mut params)
                        .with_context(|| format!("Failed to convert command line tool CWL at '{}'", Path::new(&resolved_path).display()))?;

                    files.extend(tool_files);

                    // Collect output globs from the CommandLineTool
                    if let Some(outputs) = tool_json.get("outputs").and_then(|v| v.as_array()) {
                        for output in outputs {
                            if let Some(glob) = output.get("outputBinding").and_then(|b| b.get("glob"))
                                && let Some(glob_str) = glob.as_str()
                            {
                                tool_output_files.insert(glob_str.to_string());
                            }
                        }
                    }

                    graph.push(tool_json);
                }
            }
        }
    }

    let mut inputs_value = serde_yaml::from_value::<WorkflowInputs>(Value::Mapping(inputs_yaml_data.clone()))
        .context("Failed to deserialize inputs YAML into WorkflowInputs")?;

    if !params.is_empty() {
        inputs_value.parameters = serde_yaml::to_value(&params)?;
    }

    let output_files: Vec<String> = get_all_outputs(cwl_path)
        .with_context(|| format!("Failed to get all outputs from CWL file '{cwl_path}'"))?
        .into_iter()
        .map(|(_, glob)| glob)
        .collect();

    let outputs = WorkflowOutputs { files: output_files };

    let workflow_json = WorkflowJson {
        inputs: inputs_value,
        outputs,
        version: "0.9.4".to_string(),
        workflow: WorkflowSpec {
            file: cwl_path.to_string(),
            specification: CWLGraph { graph, cwl_version },
            r#type: "cwl".to_string(),
        },
    };

    let serialized = serde_json::to_value(&workflow_json).context("Failed to serialize workflow JSON")?;

    Ok(serialized)
}

fn convert_cwl_to_json(cwl_path: &str, inputs_yaml: &Mapping) -> Result<serde_json::Value> {
    let cwl_content = fs::read_to_string(cwl_path).with_context(|| format!("Failed to read CWL file from path: {cwl_path}"))?;

    let full_cwl_path = env::current_dir()
        .with_context(|| "Failed to get current working directory")?
        .join(cwl_path);

    let workflow: CWLWorkflow = serde_yaml::from_str(&cwl_content).with_context(|| format!("Failed to parse CWL YAML at {full_cwl_path:?}"))?;

    // Prepare inputs
    let formatted_inputs: Vec<_> = workflow
        .inputs
        .iter()
        .map(|input| {
            let default_value = input.default.clone().or_else(|| {
                inputs_yaml.get("parameters").and_then(|params| match params {
                    Value::Mapping(map) => map.get(Value::String(input.id.clone())).cloned(),
                    _ => None,
                })
            });

            json!({
                "id": format!("#main/{}", input.id),
                "type": input.input_type,
                "default": default_value
            })
        })
        .collect();

    // Prepare outputs
    let mut output_source_map = HashMap::new();
    let formatted_outputs: Vec<_> = workflow
        .outputs
        .iter()
        .filter_map(|output| {
            output.output_source.as_ref()?.as_str().map(|src| {
                let full_src = format!("#main/{src}");
                output_source_map.insert(output.id.clone(), full_src.clone());
                json!({
                    "id": format!("#main/{}", output.id),
                    "outputSource": full_src,
                    "type": output.output_type
                })
            })
        })
        .collect();

    // Prepare steps
    let formatted_steps: Vec<_> = workflow
        .steps
        .iter()
        .map(|step| {
            // Step inputs
            let formatted_inputs: Vec<_> = match &step.inputs {
                CWLStepInputs::Map(map) => map
                    .iter()
                    .flat_map(|(key, input_source)| match input_source {
                        CWLStepInputSource::Simple(source) => vec![json!({
                            "id": format!("#main/{}/{}", step.id, key),
                            "source": format!("#main/{}", source)
                        })],
                        CWLStepInputSource::Detailed { source, .. } => {
                            let sources: Vec<String> = match source {
                                Some(Value::String(s)) => vec![s.clone()],
                                Some(Value::Sequence(seq)) => seq.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
                                _ => vec![],
                            };
                            sources
                                .into_iter()
                                .map(|s| {
                                    json!({
                                        "id": format!("#main/{}/{}", step.id, key),
                                        "source": format!("#main/{}", s)
                                    })
                                })
                                .collect()
                        }
                    })
                    .collect(),

                CWLStepInputs::List(list) => list
                    .iter()
                    .map(|input| {
                        json!({
                            "id": format!("#main/{}/{}", step.id, input.id),
                            "source": format!("#main/{}", input.source)
                        })
                    })
                    .collect(),
            };

            // Step outputs
            let formatted_outputs: Vec<_> = step
                .out
                .iter()
                .map(|output| {
                    let output_id = output.id();
                    let resolved_id = output_source_map
                        .get(output_id)
                        .cloned()
                        .unwrap_or_else(|| format!("#main/{}/{}", step.id, output_id));

                    json!({ "id": resolved_id })
                })
                .collect();

            // Step run path
            let run_str = sanitize_path(&step.run);
            let run_file = run_str.rsplit(MAIN_SEPARATOR).next().unwrap_or(&run_str);

            json!({
                "id": format!("#main/{}", step.id),
                "in": formatted_inputs,
                "out": formatted_outputs,
                "run": format!("#{}", run_file)
            })
        })
        .collect();

    Ok(json!({
        "class": "Workflow",
        "id": "#main",
        "inputs": formatted_inputs,
        "outputs": formatted_outputs,
        "steps": formatted_steps,
        "cwlVersion": workflow.cwl_version
    }))
}

fn convert_command_line_tool_cwl_to_json(
    cwl_path: &str,
    inputs_yaml: &serde_yaml::Mapping,
    parameters: &mut HashMap<String, String>,
) -> Result<(serde_json::Value, Vec<String>)> {
    let cwl_content = fs::read_to_string(cwl_path).with_context(|| format!("Failed to read CommandLineTool CWL file at path: {cwl_path}"))?;

    let current_dir = env::current_dir().context("Failed to get current working directory")?;
    let full_cwl_path = current_dir.join(cwl_path);

    let tool_name = Path::new(cwl_path).file_name().and_then(|s| s.to_str()).unwrap_or(cwl_path);

    let command_line_tool: CWLCommandLineTool =
        serde_yaml::from_str(&cwl_content).with_context(|| format!("Failed to parse CommandLineTool YAML at {full_cwl_path:?}"))?;

    let mut files = Vec::new();
    let mut replaced_entrynames = HashMap::new();

    let formatted_inputs = parse_inputs(&command_line_tool.inputs, tool_name, inputs_yaml, &full_cwl_path, &mut files)
        .with_context(|| format!("Failed to parse inputs for tool: {tool_name}"))?;

    // Only insert parameters that are not already in the HashMap
    for input in &command_line_tool.inputs {
        let id = input.id.trim_start_matches('#').to_string();
        if parameters.contains_key(&id) {
            continue;
        }

        // Skip File or Directory types
        let input_type_str = input.input_type.to_string().to_lowercase();
        if input_type_str.contains("file") || input_type_str.contains("directory") {
            continue;
        }

        if let Some(default_value) = &input.default {
            let value_str = match default_value {
                serde_yaml::Value::String(s) => s.clone(),
                serde_yaml::Value::Number(n) => n.to_string(),
                serde_yaml::Value::Bool(b) => b.to_string(),
                _ => "unsupported".to_string(),
            };
            parameters.insert(id, value_str);
        }
    }
    let formatted_outputs = parse_outputs(&command_line_tool.outputs, tool_name);

    let formatted_requirements = parse_requirements(&command_line_tool.requirements, &full_cwl_path, &mut replaced_entrynames, &mut files)
        .with_context(|| format!("Failed to parse requirements for tool: {tool_name}"))?;

    let base_command: Vec<String> = match &command_line_tool.base_command {
        BaseCommand::Single(cmd) => vec![cmd.clone()],
        BaseCommand::Multiple(cmds) => cmds
            .iter()
            .map(|cmd| {
                replaced_entrynames
                    .get(cmd)
                    .map_or(cmd.as_str(), |opt| opt.as_deref().unwrap_or(cmd.as_str()))
                    .to_string()
            })
            .collect(),
    };

    let tool_json = serde_json::json!({
        "class": "CommandLineTool",
        "id": format!("#{}", tool_name),
        "baseCommand": base_command,
        "inputs": formatted_inputs,
        "outputs": formatted_outputs,
        "requirements": formatted_requirements,
        "stdout": command_line_tool.stdout,
        "label": command_line_tool.label,
    });

    Ok((tool_json, files))
}

fn parse_inputs(
    inputs: &[CWLInput],
    tool_name: &str,
    inputs_yaml: &serde_yaml::Mapping,
    full_cwl_path: &Path,
    files: &mut Vec<String>,
) -> Result<Vec<serde_json::Value>> {
    inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let input_id = format!("#{}/{}", tool_name, input.id);

            let default_value = input.default.clone().or_else(|| {
                inputs_yaml.get("parameters").and_then(|params| {
                    if let serde_yaml::Value::Mapping(map) = params {
                        map.get(serde_yaml::Value::String(input.id.clone())).cloned()
                    } else {
                        None
                    }
                })
            });

            let mut input_json = serde_json::json!({
                "id": input_id,
                "type": input.input_type,
                "default": default_value
            });

            if let Some(default_file) = &input.default
                && let Some(location_str) = default_file.get("location").and_then(|v| v.as_str())
            {
                let location_path = Path::new(location_str);
                let resolved_location = get_location(&full_cwl_path.to_string_lossy(), location_path)
                    .with_context(|| format!("Failed to resolve location '{}' relative to '{}'", location_str, full_cwl_path.display()))?;

                input_json["default"] = serde_json::json!({
                    "class": "File",
                    "location": format!("file://{}", resolved_location)
                });
                files.push(resolved_location);
            }

            // Add inputBinding
            if let Some(binding) = &input.input_binding {
                let mut input_binding_json = serde_json::Map::new();

                if let Some(prefix) = &binding.prefix {
                    input_binding_json.insert("prefix".to_string(), serde_json::json!(prefix));
                }
                if let Some(position) = binding.position {
                    input_binding_json.insert("position".to_string(), serde_json::json!(position));
                }
                if input_binding_json.is_empty() {
                    input_binding_json.insert("position".to_string(), serde_json::json!(index as i64));
                }

                input_json["inputBinding"] = serde_json::Value::Object(input_binding_json);
            } else {
                input_json["inputBinding"] = serde_json::json!({ "position": index as i64 });
            }

            Ok(input_json)
        })
        .collect()
}

fn parse_outputs(outputs: &[CWLOutput], tool_name: &str) -> Vec<serde_json::Value> {
    outputs
        .iter()
        .map(|output| {
            serde_json::json!({
                "id": format!("#{}/{}", tool_name, output.id),
                "outputBinding": {
                    "glob": output.output_binding
                        .as_ref()
                        .map_or(String::new(), |binding| binding.glob.clone())
                },
                "type": output.output_type
            })
        })
        .collect()
}

fn parse_requirements(
    requirements: &Option<Vec<CWLRequirement>>,
    full_cwl_path: &Path,
    replaced_entrynames: &mut HashMap<String, Option<String>>,
    files: &mut Vec<String>,
) -> Result<Vec<serde_json::Value>> {
    let Some(reqs) = requirements else { return Ok(vec![]) };

    let mut parsed = Vec::new();

    for req in reqs {
        let json_req = match req.class.as_str() {
            "DockerRequirement" => json!({
                "class": "DockerRequirement",
                "dockerPull": req.docker_pull
            }),
            "NetworkAccess" => json!({
                "class": "NetworkAccess",
                "networkAccess": req.network_access.unwrap_or(false)
            }),
            "InlineJavascriptRequirement" => {
                let mut map = serde_json::Map::new();
                map.insert("class".to_string(), serde_json::json!("InlineJavascriptRequirement"));
                if let Some(lib) = &req.expression_lib {
                    map.insert("expressionLib".to_string(), serde_json::json!(lib));
                }
                serde_json::Value::Object(map)
            }
            "InitialWorkDirRequirement" => {
                let Some(listing) = &req.listing else {
                    parsed.push(json!({ "class": "InitialWorkDirRequirement" }));
                    continue;
                };

                let mut formatted_listing = Vec::new();

                for entry in listing {
                    match &entry.entry {
                        Some(serde_yaml::Value::Mapping(map)) => {
                            if let Some(include) = map.get(serde_yaml::Value::String("$include".into())) {
                                if let Some(include_path) = include.as_str() {
                                    let entryname = entry.entryname.clone().unwrap_or_else(|| {
                                        Path::new(include_path)
                                            .file_name()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("unknown")
                                            .to_string()
                                    });

                                    let new_name = Path::new(&entryname).file_name().and_then(|s| s.to_str()).map(|s| s.to_string());

                                    if let Some(new_name) = &new_name
                                        && &entryname != new_name
                                    {
                                        replaced_entrynames.insert(entryname.clone(), Some(new_name.clone()));
                                    }

                                    let loc = get_location(&full_cwl_path.to_string_lossy(), Path::new(include_path))
                                        .with_context(|| format!("Failed to resolve location for include '{include_path}'"))?;

                                    let content = read_file_content(&loc).with_context(|| format!("Failed to read included file '{loc}'"))?;
                                    files.push(loc);

                                    formatted_listing.push(json!({
                                        "entry": content,
                                        "entryname": new_name
                                    }));
                                }
                            } else {
                                // treat as inline YAML
                                formatted_listing.push(json!({
                                    "entry": entry.entry.clone(),
                                    "entryname": entry.entryname
                                }));
                            }
                        }
                        Some(_) | None => {
                            formatted_listing.push(json!({
                                "entry": entry.entry.clone(),
                                "entryname": entry.entryname
                            }));
                        }
                    }
                }

                json!({
                    "class": "InitialWorkDirRequirement",
                    "listing": formatted_listing
                })
            }

            _ => continue,
        };

        parsed.push(json_req);
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[test]
    fn test_generate_workflow_json_from_cwl_minimal() {
        let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let cwl_path = base_dir.join("tests/test_data/hello_world/workflows/main/main.cwl");
        assert!(Path::new(&cwl_path).exists(), "Test cwl file does not exists");
        let result = generate_workflow_json_from_cwl(&cwl_path, &None);

        assert!(result.is_ok(), "Expected generation to succeed");
        let json = result.unwrap();

        // Basic assertions
        assert_eq!(json["version"], "0.9.4");
        assert_eq!(json["workflow"]["type"], "cwl");
        assert_eq!(json["workflow"]["file"], cwl_path.to_str().unwrap());

        let inputs = &json["inputs"];
        assert!(inputs.is_object(), "Inputs should be an object");

        // Check 'directories'
        assert!(inputs["directories"].is_array(), "directories should be an array");
        assert_eq!(inputs["directories"].as_array().unwrap().len(), 1);

        // Check 'files'
        assert!(inputs["files"].is_array(), "files should be an array");

        // Check parameters
        let parameters = &inputs["parameters"];
        assert!(parameters.is_object(), "parameters should be an object");

        assert_eq!(parameters["population"]["class"], "File");

        // Try 'location' key, fallback to 'path'
        let population_path_value = parameters["population"].get("location").or_else(|| parameters["population"].get("path"));
        let population_path = population_path_value
            .and_then(|v| v.as_str())
            .expect("Expected parameters['population'] to have 'location' or 'path' as a string");

        assert_eq!(normalize_path(population_path), "data/population.csv");

        assert_eq!(parameters["speakers"]["class"], "File");

        let speakers_path_value = parameters["speakers"].get("location").or_else(|| parameters["speakers"].get("path"));
        let speakers_path = speakers_path_value
            .and_then(|v| v.as_str())
            .expect("Expected parameters['speakers'] to have 'location' or 'path' as a string");

        assert_eq!(normalize_path(speakers_path), "data/speakers_revised.csv");

        // Check outputs
        let outputs = &json["outputs"];
        assert!(outputs.is_object(), "Outputs should be an object");
        assert!(outputs["files"].is_array(), "outputs.files should be an array");
        assert_eq!(outputs["files"].as_array().unwrap().len(), 1);
        assert_eq!(outputs["files"][0], "results.svg");

        // Check workflow steps
        let steps = &json["workflow"]["specification"]["$graph"][0]["steps"];
        assert!(steps.is_array(), "Steps should be an array");

        assert!(!steps.as_array().unwrap().is_empty(), "Steps array should not be empty");

        let calculation_exists = steps.as_array().unwrap().iter().any(|step| step["id"] == "#main/calculation");
        assert!(calculation_exists, "'calculation' step is missing");

        let plot_exists = steps.as_array().unwrap().iter().any(|step| step["id"] == "#main/plot");
        assert!(plot_exists, "'plot' step is missing");
    }

    #[test]
    fn test_convert_command_line_tool_cwl_to_json() {
        let cwl_template = r#"
            #!/usr/bin/env cwl-runner

            cwlVersion: v1.2
            class: CommandLineTool

            requirements:
            - class: InitialWorkDirRequirement
              listing:
              - entryname: code/download_election_data.py
                entry:
                  $include: code/download_election_data.py

            inputs:
            - id: ags
              type: string
              default: '03101000'
              inputBinding:
                prefix: --ags
            - id: election
              type: string
              default: Bundestagswahl 2025
              inputBinding:
                prefix: --election

            outputs:
            - id: data
              type: File
              outputBinding:
                glob: data.csv
            stdout: data.csv

            baseCommand:
            - python
            - code/download_election_data.py
        "#;
        let inputs_yaml_data = r#"
            election: Bundestagswahl 2025
            ags: 03101000
            shapes:
              class: Directory
              location: data/braunschweig
            feature: F3
        "#;
        let inputs_yaml_value: Value = serde_yaml::from_str(inputs_yaml_data).expect("Failed to parse YAML");

        let inputs_yaml_mapping = inputs_yaml_value.as_mapping().expect("Expected top-level YAML to be a mapping");

        // Create a temporary directory
        let temp_dir = tempdir().expect("failed to create temp dir");
        let code_dir = temp_dir.path().join("code");
        fs::create_dir_all(&code_dir).expect("failed to create code dir");

        // Write dummy script file
        let script_path = code_dir.join("download_election_data.py");
        fs::write(&script_path, "print('Hello from script')").expect("failed to write script");

        // Write CWL file
        let cwl_path = temp_dir.path().join("tool.cwl");
        fs::write(&cwl_path, cwl_template).expect("failed to write cwl");

        // Save and change current dir
        let old_dir = env::current_dir().expect("could not get current dir");
        env::set_current_dir(temp_dir.path()).expect("could not change to temp dir");
        let mut params = HashMap::new();
        // Run the function
        let (json_output, files) = convert_command_line_tool_cwl_to_json("tool.cwl", inputs_yaml_mapping, &mut params).expect("Conversion failed");

        // Restore the original working directory
        env::set_current_dir(old_dir).expect("could not reset current dir");

        // Assertions
        let base_command = json_output["baseCommand"].as_array().unwrap();
        assert_eq!(base_command[0], "python");
        assert_eq!(base_command[1], "download_election_data.py");

        let inputs = json_output["inputs"].as_array().unwrap();
        assert!(inputs.iter().any(|i| i["id"].as_str().unwrap().contains("ags")));
        assert!(inputs.iter().any(|i| i["id"].as_str().unwrap().contains("election")));

        assert!(
            files.iter().any(|f| f.ends_with("download_election_data.py")),
            "Expected download_election_data.py to be in files list, got: {files:?}"
        );

        temp_dir.close().expect("failed to clean up temp dir");
    }

    fn normalize_path(path: &str) -> String {
        Path::new(path).to_str().unwrap_or_default().replace("\\", "/")
    }

    #[test]
    fn test_generate_workflow_json_from_cwl_with_inputs_yaml() {
        let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let cwl_path = base_dir.join("tests/test_data/hello_world/workflows/main/main.cwl");
        let inputs_yaml_path = base_dir.join("tests/test_data/hello_world/inputs.yml");

        assert!(cwl_path.exists(), "CWL file not found at {cwl_path:?}");
        assert!(inputs_yaml_path.exists(), "Inputs YAML file not found at {inputs_yaml_path:?}");

        let result = generate_workflow_json_from_cwl(&cwl_path, &Some(inputs_yaml_path.to_string_lossy().to_string()));

        assert!(result.is_ok(), "Expected generation to succeed");
        let json = result.expect("Failed to generate workflow JSON");

        assert_eq!(json["version"], "0.9.4");
        assert_eq!(json["workflow"]["type"], "cwl");
        assert_eq!(json["workflow"]["file"], cwl_path.to_str().unwrap());

        let inputs = &json["inputs"];
        assert!(inputs.is_object(), "Inputs should be an object");

        let parameters = &inputs["parameters"];
        assert!(parameters.is_object(), "parameters should be an object");
        assert_eq!(parameters["population"]["class"], "File");
        assert_eq!(
            normalize_path(parameters["population"]["location"].as_str().unwrap()),
            "data/population.csv"
        );
        assert_eq!(parameters["speakers"]["class"], "File");
        assert_eq!(
            normalize_path(parameters["speakers"]["location"].as_str().unwrap()),
            "data/speakers_revised.csv"
        );
        let outputs = &json["outputs"];
        assert!(outputs.is_object(), "Outputs should be an object");
        assert!(outputs["files"].is_array(), "outputs.files should be an array");
        assert_eq!(outputs["files"].as_array().unwrap().len(), 1);
        assert_eq!(outputs["files"][0], "results.svg");

        let cwl_files = &json["workflow"]["specification"]["$graph"];
        assert!(cwl_files.is_array(), "Steps should be an array");
        assert_eq!(cwl_files.as_array().unwrap().len(), 3);

        let steps = &json["workflow"]["specification"]["$graph"][0]["steps"];
        assert!(steps.is_array(), "Steps should be an array");

        assert!(!steps.as_array().unwrap().is_empty(), "Steps array should not be empty");

        let calculation_exists = steps.as_array().unwrap().iter().any(|step| step["id"] == "#main/calculation");
        assert!(calculation_exists, "'calculation' step is missing");

        let plot_exists = steps.as_array().unwrap().iter().any(|step| step["id"] == "#main/plot");
        assert!(plot_exists, "'plot' step is missing");
    }
}
