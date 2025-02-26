use cwl::{
    clt::CommandLineTool,
    outputs::CommandOutputParameter,
    requirements::Requirement,
    types::{CWLType, DefaultValue, EnviromentDefs, File},
};
use glob::glob;
use pathdiff::diff_paths;
use serde_yaml::Value;
use std::{collections::HashMap, path::PathBuf};

use crate::util::copy_file;

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
                let relative_path = diff_paths(file, &runtime.runtime["outdir"]).unwrap_or(PathBuf::from(&file.file_name().unwrap()));
                let destination = outdir.join(relative_path);
                copy_file(file, &destination)?;
                map.insert(output.id.clone(), DefaultValue::File(File::from_file(destination, output.format.clone())));
            }
        }
        _ => {}
    }
    Ok(())
}
