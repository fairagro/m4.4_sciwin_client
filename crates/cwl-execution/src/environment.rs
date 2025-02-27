use cwl::{
    clt::CommandLineTool,
    outputs::CommandOutputParameter,
    requirements::Requirement,
    types::{CWLType, DefaultValue, Directory, EnviromentDefs, File},
};
use glob::glob;
use pathdiff::diff_paths;
use serde_yaml::Value;
use std::{collections::HashMap, fs, path::PathBuf};

use crate::util::{copy_dir, copy_file};

#[derive(Debug, Default)]
pub struct RuntimeEnvironment {
    pub inputs: HashMap<String, DefaultValue>,
    pub runtime: HashMap<String, String>,
    pub environment: HashMap<String, String>,
}

pub(crate) fn collect_inputs(
    tool: &CommandLineTool,
    inputs: &HashMap<String, DefaultValue>,
) -> Result<HashMap<String, DefaultValue>, Box<dyn std::error::Error>> {
    tool.inputs
        .iter()
        .map(|i| {
            if let Some(value) = inputs.get(&i.id) {
                if value.has_matching_type(&i.type_) {
                    return Ok((i.id.clone(), value.load()));
                } else {
                    Err(format!("CWLType {:?} is not matching input value: \n{:#?}", i.type_, value))?
                }
            } else if let Some(default) = &i.default {
                return Ok((i.id.clone(), default.load()));
            }

            if i.type_.is_optional() {
                return Ok((i.id.clone(), DefaultValue::Any(Value::Null)));
            }
            Err(format!("No Input provided for {:?}", i.id))?
        })
        .collect::<Result<HashMap<_, _>, Box<dyn std::error::Error>>>()
}

pub(crate) fn collect_env_vars(tool: &CommandLineTool) -> HashMap<String, String> {
    tool.requirements
        .iter()
        .chain(tool.hints.iter())
        .flatten()
        .filter_map(|r| {
            if let Requirement::EnvVarRequirement(evr) = r {
                match &evr.env_def {
                    EnviromentDefs::Vec(vec) => Some(vec.iter().map(|d| (d.env_name.clone(), d.env_value.clone())).collect::<HashMap<_, _>>()),
                    EnviromentDefs::Map(map) => Some(map.clone()),
                }
            } else {
                None
            }
        })
        .flatten()
        .collect::<HashMap<_, _>>()
}

pub(crate) fn collect_outputs(tool: &CommandLineTool, outdir: &PathBuf, runtime: &RuntimeEnvironment) -> Result<(), Box<dyn std::error::Error>> {
    let mut map = HashMap::new();
    for output in &tool.outputs {
        match &output.type_ {
            CWLType::Optional(inner) => {
                evaluate_output(output, inner, outdir, runtime, &tool.stdout, &tool.stderr, &mut map).ok();
            }
            _ => evaluate_output(output, &output.type_, outdir, runtime, &tool.stdout, &tool.stderr, &mut map)?,
        }
    }
    println!("{:#?}", map);
    Ok(())
}

fn evaluate_output(
    output: &CommandOutputParameter,
    type_: &CWLType,
    outdir: &PathBuf,
    runtime: &RuntimeEnvironment,
    tool_stdout: &Option<String>,
    tool_stderr: &Option<String>,
    map: &mut HashMap<String, DefaultValue>,
) -> Result<(), Box<dyn std::error::Error>> {
    match type_ {
        CWLType::File | CWLType::Stdout | CWLType::Stderr => {
            if let Some(binding) = &output.output_binding {
                let pattern = format!("{}/{}", &runtime.runtime["outdir"], &binding.glob);
                let file = &glob(&pattern)?.collect::<Result<Vec<_>, glob::GlobError>>()?[0];
                if !file.is_file() {
                    let metadata = fs::metadata(file)?;
                    return Err(format!("File requested, got: {:?}", metadata.file_type()).into());
                }
                let relative_path = diff_paths(file, &runtime.runtime["outdir"]).unwrap_or(PathBuf::from(&file.file_name().unwrap()));
                let destination = outdir.join(relative_path);
                copy_file(file, &destination)?;
                map.insert(output.id.clone(), DefaultValue::File(File::from_file(destination, output.format.clone())));
            } else {
                let filename = match output.type_ {
                    CWLType::Stdout if tool_stdout.is_some() => tool_stdout.as_ref().unwrap(),
                    CWLType::Stderr if tool_stderr.is_some() => tool_stderr.as_ref().unwrap(),
                    _ => {
                        let mut file_prefix = output.id.clone();
                        file_prefix += match output.type_ {
                            CWLType::Stdout => "_stdout",
                            CWLType::Stderr => "_stderr",
                            _ => "",
                        };
                        let pattern = format!("{}/{}*", &runtime.runtime["outdir"], file_prefix);
                        &glob(&pattern)?.collect::<Result<Vec<_>, glob::GlobError>>()?[0]
                            .to_string_lossy()
                            .into_owned()
                    }
                };
                let relative_path = diff_paths(filename, &runtime.runtime["outdir"]).unwrap_or(PathBuf::from(&filename));
                let destination = outdir.join(relative_path);
                copy_file(filename, &destination)?;
                map.insert(output.id.clone(), DefaultValue::File(File::from_file(destination, output.format.clone())));
            }
        }
        CWLType::Directory => {
            if let Some(binding) = &output.output_binding {
                let pattern = format!("{}/{}", &runtime.runtime["outdir"], &binding.glob);
                let dir = &glob(&pattern)?.collect::<Result<Vec<_>, glob::GlobError>>()?[0];
                if !dir.is_dir() {
                    let metadata = fs::metadata(dir)?;
                    return Err(format!("Directory requested, got: {:?}", metadata.file_type()).into());
                }
                let relative_path = diff_paths(dir, &runtime.runtime["outdir"]).unwrap_or(PathBuf::from(&dir.file_name().unwrap()));
                let destination = outdir.join(relative_path);
                copy_dir(dir, &destination)?;
                map.insert(output.id.clone(), DefaultValue::Directory(Directory::from_path(&destination)));
               
            }
        }
        CWLType::Array(inner) if matches!(**inner, CWLType::File) => {}
        CWLType::Array(inner) if matches!(**inner, CWLType::Directory) => {
            if let Some(binding) = &output.output_binding {
                let pattern = format!("{}/{}", &runtime.runtime["outdir"], &binding.glob);
                let dirs = glob(&pattern)?.collect::<Result<Vec<_>, glob::GlobError>>()?;
                for dir in &dirs {
                    if !dir.is_dir() {
                        let metadata = fs::metadata(dir)?;
                        return Err(format!("Directory requested, got: {:?}", metadata.file_type()).into());
                    }
                }

                println!("{dirs:#?}");
            }
        }
        _ => {}
    }
    Ok(())
}
