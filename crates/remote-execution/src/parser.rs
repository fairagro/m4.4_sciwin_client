use serde_yaml:: Value;
use std::{
    collections::HashMap,
    error::Error,
    fs,
    env,
    path::PathBuf,
    io
};
use std::path::Path;
use std::path::MAIN_SEPARATOR;
use serde::{de::self, Deserialize, Serialize};
use serde::{ Deserializer};
use crate::utils::{read_file_content, get_location, sanitize_path, get_all_outputs, build_inputs_cwl, build_inputs_yaml, load_cwl_file2};

#[derive(Debug, Deserialize, Serialize)]
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
    default: Option<serde_yaml::Value>,
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


pub fn generate_workflow_json_from_cwl(
    file: &PathBuf, input_file: &Option<String>
) -> Result<serde_json::Value, Box<dyn Error>> {
    let cwl_path = file.to_str().unwrap();
    let inputs_yaml = input_file.as_deref();
    let base_path = std::env::current_dir()?.to_string_lossy().to_string();
    let workflow_spec_json = convert_cwl_to_json(cwl_path)?;
    let cwl_yaml: Value = load_cwl_file2(&base_path, &file)?;
    let cwl_version = workflow_spec_json
        .get("cwlVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("v1.2")
        .to_string();
    let mut graph = vec![workflow_spec_json];
    if let serde_json::Value::Object(_) = &graph[0] {
        if let Some(steps_array) = cwl_yaml.get("steps").and_then(|v| v.as_sequence()) {
            for step in steps_array {    
                if let Some(run_val) = step.get("run").and_then(|v| v.as_str()) {
                    let run_val_clean = run_val.trim_start_matches('#');
                    let step_path = Path::new(run_val_clean);   
                    let resolved = get_location(cwl_path, step_path)?;     
                    let tool_json = convert_command_line_tool_cwl_to_json(&resolved)?;
                    graph.push(tool_json);
                }
            }
        }
    }

    let inputs_yaml_data = if let Some(yaml_file) = inputs_yaml {
        build_inputs_yaml(cwl_path, yaml_file)?
    } else {
        build_inputs_cwl(cwl_path, None)?
    };

    let inputs_value = serde_yaml::from_value::<WorkflowInputs>(Value::Mapping(inputs_yaml_data.clone()))?;

    let output_files: Vec<Value> = get_all_outputs(cwl_path)?
        .into_iter()
        .map(|(_, glob_value)| Value::String(glob_value))
        .collect();

    let outputs = WorkflowOutputs {
        files: output_files
            .into_iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
    };

    let json = WorkflowJson {
        inputs: inputs_value,
        outputs,
        version: "0.9.3".to_string(),
        workflow: WorkflowSpec {
            file: cwl_path.to_string(),
            specification: CWLGraph {
                graph,
                cwl_version,
            },
            r#type: "cwl".to_string(),
        },
    };

    let serialized = serde_json::to_value(&json)?;
    Ok(serialized)
}



fn convert_cwl_to_json(cwl_path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let cwl_content = fs::read_to_string(cwl_path)?;
    let current_dir = env::current_dir()?;
    let full_cwl_path = current_dir.join(cwl_path);
    let cwd = std::env::current_dir()?;
    
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
            if let Some(location_str) = default.get("location").and_then(|v| v.as_str()) {
                let l_path = Path::new(location_str);
                let location = match get_location(&full_cwl_path.to_string_lossy(), l_path) {
                    Ok(loc) => {  
                        let rel_path = pathdiff::diff_paths(&loc, &cwd).unwrap_or_else(|| std::path::Path::new(&loc).to_path_buf());
                        rel_path.to_string_lossy().to_string()
                    }
                    Err(e) => {
                        println!("⚠️ Could not resolve location for '{}': {}", input.id, e);
                        "file://No location".to_string()
                    }
                };
        
                input_json["default"] = serde_json::json!({
                    "class": input.r#type,
                    "location": sanitize_path(&location)
                });
            }
        }
    
        input_json
    }).collect();

    let mut output_source_map: HashMap<String, String> = HashMap::new();
    let formatted_outputs: Vec<_> = workflow.outputs.iter().map(|output| {
        output_source_map.insert(output.id.clone(),  format!("#main/{}", output.output_source.clone().unwrap_or_default()));
        serde_json::json!({
            "id": output.id,
            "outputSource": format!("#main/{}", output.output_source.clone().unwrap_or_default()),
            "type": output.r#type
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

        let matched_workflow_output_id = output_source_map
            .get(output.id())
            .cloned()
            .unwrap_or_else(|| format!("#main/{}/{}", step.id, output.id()));
        
            serde_json::json!({
                "id": matched_workflow_output_id
            })
        }).collect();

        serde_json::json!({
            "id": format!("#main/{}", step.id),
            "in": formatted_inputs,
            "out": formatted_outputs,
            "run": format!("#{}", sanitize_path(&step.run)),
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


fn convert_command_line_tool_cwl_to_json(cwl_path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let cwl_content = fs::read_to_string(cwl_path)?;
    let current_dir = env::current_dir()?;
    let full_cwl_path = current_dir.join(cwl_path);

    let cwl_path_parts: Vec<&str> = cwl_path.split(MAIN_SEPARATOR).collect();
    let tool_name = cwl_path_parts.last().copied().unwrap_or(cwl_path);
    let tool_base = Path::new(tool_name).file_stem().unwrap_or_default().to_string_lossy();

    let command_line_tool: CWLCommandLineTool = match serde_yaml::from_str(&cwl_content) {
        Ok(parsed) => parsed,
        Err(e) => {
            println!("❌ Failed to parse CWL YAML: {}", e);
            return Err(Box::new(e));
        }
    };

    let formatted_inputs: Vec<_> = command_line_tool.inputs.iter().map(|input| {
        let input_id = format!("#{}/{}/{}",tool_base, tool_name, input.id);

        let mut input_json = serde_json::json!({
            "id": input_id,
            "type": input.r#type
        });

        if let Some(default_file) = &input.default {
            if let Some(location_str) = default_file.get("location").and_then(|v| v.as_str()) {
                let location_path = Path::new(location_str);
                if let Ok(resolved_location) = get_location(&full_cwl_path.to_string_lossy(), location_path) {
                    input_json["default"] = serde_json::json!({
                        "class": "File",
                        "location": format!("file://{}", resolved_location)
                    });
                }
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
            "id": format!("#{}/{}/{}",tool_base, tool_name, output.id),
            "outputBinding": {
                "glob": output.output_binding.as_ref().map_or("".to_string(), |binding| binding.glob.clone())
            },
            "type": output.r#type
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

    let tool_base = Path::new(tool_name).file_stem().unwrap_or_default().to_string_lossy();
    let command_line_tool_json = serde_json::json!( {
        "class": "CommandLineTool",
        "id":  format!("#{}/{}", tool_base, tool_name),
        "baseCommand": command_line_tool.base_command,
        "inputs": formatted_inputs,
        "outputs": formatted_outputs,
        "requirements": formatted_requirements,
        "label": command_line_tool.label,
    });

    Ok(command_line_tool_json)
}
