use crate::{
    expression::{evaluate_expression, set_self, unset_self},
    util::{copy_dir, copy_file},
};
use cwl::{
    clt::CommandLineTool,
    outputs::CommandOutputParameter,
    requirements::Requirement,
    types::{CWLType, DefaultValue, Directory, EnviromentDefs, File, OutputItem},
};
use glob::glob;
use pathdiff::diff_paths;
use serde_yaml::Value;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Clone)]
pub struct RuntimeEnvironment {
    pub inputs: HashMap<String, DefaultValue>,
    pub runtime: HashMap<String, String>,
    pub environment: HashMap<String, String>,
    pub time_limit: u64
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
                    return Ok((i.id.clone(), value.clone()));
                } else {
                    Err(format!("CWLType {:?} is not matching input value: \n{:#?}", i.type_, value))?
                }
            } else if let Some(default) = &i.default {
                return Ok((i.id.clone(), default.clone()));
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

pub(crate) fn collect_outputs(
    tool: &CommandLineTool,
    outdir: &Path,
    runtime: &RuntimeEnvironment,
) -> Result<HashMap<String, OutputItem>, Box<dyn std::error::Error>> {
    let mut map = HashMap::new();
    for output in &tool.outputs {
        match &output.type_ {
            CWLType::Optional(inner) => {
                evaluate_output(output, inner, outdir, runtime, &tool.stdout, &tool.stderr, &mut map).ok();
            }
            _ => evaluate_output(output, &output.type_, outdir, runtime, &tool.stdout, &tool.stderr, &mut map)?,
        }
    }
    Ok(map)
}

fn evaluate_output(
    output: &CommandOutputParameter,
    type_: &CWLType,
    outdir: &Path,
    runtime: &RuntimeEnvironment,
    tool_stdout: &Option<String>,
    tool_stderr: &Option<String>,
    map: &mut HashMap<String, OutputItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    match type_ {
        CWLType::File | CWLType::Stdout | CWLType::Stderr => file_processing(output, outdir, runtime, tool_stdout, tool_stderr, map)?,
        CWLType::Optional(inner) if matches!(**inner, CWLType::File) => file_processing(output, outdir, runtime, tool_stdout, tool_stderr, map)?,
        CWLType::Directory => directory_processing(output, outdir, runtime, map)?,
        CWLType::Optional(inner) if matches!(**inner, CWLType::Directory) => directory_processing(output, outdir, runtime, map)?,
        CWLType::Array(inner) if matches!(**inner, CWLType::File) => {
            if let Some(binding) = &output.output_binding {
                let pattern = format!("{}/{}", &runtime.runtime["outdir"], &binding.glob.as_ref().unwrap());
                let files = glob(&pattern)?.collect::<Result<Vec<_>, glob::GlobError>>()?;

                let mut output_result = vec![];
                for file in &files {
                    if !file.is_file() {
                        let metadata = fs::metadata(file)?;
                        return Err(format!("File requested, got: {:?}", metadata.file_type()).into());
                    }
                    let relative_path = diff_paths(file, &runtime.runtime["outdir"]).unwrap_or(PathBuf::from(&file.file_name().unwrap()));
                    let destination = outdir.join(relative_path);
                    copy_file(file, &destination)?;
                    output_result.push(DefaultValue::File(File::from_file(destination, output.format.clone())));
                }
                map.insert(output.id.clone(), OutputItem::Vec(output_result));
            }
        }
        CWLType::Array(inner) if matches!(**inner, CWLType::Directory) => {
            if let Some(binding) = &output.output_binding {
                let pattern = format!("{}/{}", &runtime.runtime["outdir"], &binding.glob.as_ref().unwrap());
                let dirs = glob(&pattern)?.collect::<Result<Vec<_>, glob::GlobError>>()?;

                let mut output_result = vec![];
                for dir in &dirs {
                    if !dir.is_dir() {
                        let metadata = fs::metadata(dir)?;
                        return Err(format!("Directory requested, got: {:?}", metadata.file_type()).into());
                    }
                    let relative_path = diff_paths(dir, &runtime.runtime["outdir"]).unwrap_or(PathBuf::from(&dir.file_name().unwrap()));
                    let destination = outdir.join(relative_path);
                    copy_dir(dir, &destination)?;
                    output_result.push(DefaultValue::Directory(Directory::from_path(&destination)));
                }
                map.insert(output.id.clone(), OutputItem::Vec(output_result));
            }
        }
        _ => {
            if let Some(binding) = &output.output_binding {
                //if there is a binding we can read the file
                if let Some(glob_) = &binding.glob {
                    let pattern = format!("{}/{}", &runtime.runtime["outdir"], glob_);
                    let file = &glob(&pattern)?.collect::<Result<Vec<_>, glob::GlobError>>()?[0];

                    let content = fs::read_to_string(file)?;
                    if let Some(expression) = &binding.output_eval {
                        let mut me = File::from_file(file, None);
                        me.contents = Some(content);
                        set_self(&vec![me])?;
                        let result = evaluate_expression(expression)?;
                        let value = serde_yaml::from_str(&serde_json::to_string(&result)?)?;
                        map.insert(output.id.clone(), OutputItem::Value(DefaultValue::Any(value)));
                        unset_self()?;
                    } else {
                        map.insert(output.id.clone(), OutputItem::Value(DefaultValue::Any(Value::String(content))));
                    }
                } else if let Some(expression) = &binding.output_eval {
                    let result = evaluate_expression(expression)?;
                    let value = serde_yaml::from_str(&serde_json::to_string(&result)?)?;
                    map.insert(output.id.clone(), OutputItem::Value(DefaultValue::Any(value)));
                }
            } else if output.type_.is_array() {
                map.insert(output.id.clone(), OutputItem::Value(DefaultValue::Any(Value::Sequence(vec![]))));
            }
        }
    }
    Ok(())
}

fn file_processing(
    output: &CommandOutputParameter,
    outdir: &Path,
    runtime: &RuntimeEnvironment,
    tool_stdout: &Option<String>,
    tool_stderr: &Option<String>,
    map: &mut HashMap<String, OutputItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(binding) = &output.output_binding {
        let pattern = format!("{}/{}", &runtime.runtime["outdir"], &binding.glob.as_ref().unwrap());
        let files = &glob(&pattern)?.collect::<Result<Vec<_>, glob::GlobError>>()?;
        if let CWLType::Optional(_) = output.type_ {
            if files.is_empty() {
                map.insert(output.id.clone(), OutputItem::Value(DefaultValue::Any(Value::Null)));
                return Ok(());
            }
        }
        let file = &files[0];
        if !file.is_file() {
            let metadata = fs::metadata(file)?;
            return Err(format!("File requested, got: {:?}", metadata.file_type()).into());
        }
        let relative_path = diff_paths(file, &runtime.runtime["outdir"]).unwrap_or(PathBuf::from(&file.file_name().unwrap()));
        let destination = outdir.join(relative_path);
        copy_file(file, &destination)?;
        map.insert(
            output.id.clone(),
            OutputItem::Value(DefaultValue::File(File::from_file(destination, output.format.clone()))),
        );
    } else {
        let filename = match output.type_ {
            CWLType::Stdout if tool_stdout.is_some() => &format!("{}/{}", &runtime.runtime["outdir"], tool_stdout.as_ref().unwrap()),
            CWLType::Stderr if tool_stderr.is_some() => &format!("{}/{}", &runtime.runtime["outdir"], tool_stderr.as_ref().unwrap()),
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
        map.insert(
            output.id.clone(),
            OutputItem::Value(DefaultValue::File(File::from_file(destination, output.format.clone()))),
        );
    }
    Ok(())
}

fn directory_processing(
    output: &CommandOutputParameter,
    outdir: &Path,
    runtime: &RuntimeEnvironment,
    map: &mut HashMap<String, OutputItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(binding) = &output.output_binding {
        let pattern = format!("{}/{}", &runtime.runtime["outdir"], &binding.glob.as_ref().unwrap());
        let dirs = &glob(&pattern)?.collect::<Result<Vec<_>, glob::GlobError>>()?;
        if let CWLType::Optional(_) = output.type_ {
            if dirs.is_empty() {
                map.insert(output.id.clone(), OutputItem::Value(DefaultValue::Any(Value::Null)));
                return Ok(());
            }
        }
        let dir = &dirs[0];
        if !dir.is_dir() {
            let metadata = fs::metadata(dir)?;
            return Err(format!("Directory requested, got: {:?}", metadata.file_type()).into());
        }
        let relative_path = diff_paths(dir, &runtime.runtime["outdir"]).unwrap_or(PathBuf::from(&dir.file_name().unwrap()));
        let destination = outdir.join(relative_path);
        copy_dir(dir, &destination)?;
        map.insert(
            output.id.clone(),
            OutputItem::Value(DefaultValue::Directory(Directory::from_path(&destination))),
        );
    }
    Ok(())
}
