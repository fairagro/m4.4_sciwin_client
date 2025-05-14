use crate::{get_available_disk_space, get_available_ram, get_processor_count, util::evaluate_input, validate::set_placeholder_values_in_string};
use cwl::{
    inputs::CommandInputParameter,
    requirements::{check_timelimit, Requirement, ResourceRequirement, StringOrNumber},
    types::{DefaultValue, EnviromentDefs},
    CWLDocument,
};
use std::{
    collections::HashMap,
    error::Error,
    fs::{self},
    path::Path,
};

#[derive(Debug, Default, Clone)]
pub struct RuntimeEnvironment {
    pub runtime: HashMap<String, String>,
    pub inputs: HashMap<String, DefaultValue>,
    pub environment: HashMap<String, String>,
    pub time_limit: u64,
}

impl RuntimeEnvironment {
    pub fn with_inputs(mut self, inputs: HashMap<String, DefaultValue>) -> Self {
        self.inputs = inputs;
        self
    }

    pub fn with_environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }
    pub fn with_runtime(mut self, runtime: HashMap<String, String>) -> Self {
        self.runtime = runtime;
        self
    }
    pub fn with_time_limit(mut self, time_limit: u64) -> Self {
        self.time_limit = time_limit;
        self
    }

    pub fn initialize(
        tool: &CWLDocument,
        input_values: HashMap<String, DefaultValue>,
        outdir: impl AsRef<Path>,
        tooldir: impl AsRef<Path>,
        tmpdir: impl AsRef<Path>,
    ) -> Result<Self, Box<dyn Error>> {
        let runtime = HashMap::from([
            ("tooldir".to_string(), tooldir.as_ref().to_string_lossy().into_owned()),
            ("outdir".to_string(), outdir.as_ref().to_string_lossy().into_owned()),
            ("tmpdir".to_string(), tmpdir.as_ref().to_string_lossy().into_owned()),
            ("outdirSize".to_string(), get_available_disk_space().to_string()),
            ("tmpdirSize".to_string(), get_available_disk_space().to_string()),
            ("cores".to_string(), get_processor_count().to_string()),
            ("ram".to_string(), get_available_ram().to_string()),
        ]);

        let inputs = collect_inputs(tool, input_values, tooldir)?;

        let mut environment = RuntimeEnvironment {
            runtime,
            time_limit: check_timelimit(tool).unwrap_or(0),
            inputs,
            ..Default::default()
        };

        if let Some(rr) = tool.get_requirement::<ResourceRequirement>() {
            if let Some(cores) = &rr.cores_min {
                environment
                    .runtime
                    .insert("cores".to_string(), evaluate(cores, &environment, &tool.inputs)?.to_string());
            }
            if let Some(ram) = &rr.ram_min {
                environment
                    .runtime
                    .insert("ram".to_string(), evaluate(ram, &environment, &tool.inputs)?.to_string());
            }
            if let Some(dir_size) = &rr.outdir_min {
                environment
                    .runtime
                    .insert("outdirSize".to_string(), evaluate(dir_size, &environment, &tool.inputs)?.to_string());
            }
            if let Some(tmp_size) = &rr.tmpdir_min {
                environment
                    .runtime
                    .insert("tmpdirSize".to_string(), evaluate(tmp_size, &environment, &tool.inputs)?.to_string());
            }
        }

        Ok(environment)
    }
}

fn evaluate(val: &StringOrNumber, runtime: &RuntimeEnvironment, inputs: &[CommandInputParameter]) -> Result<u64, Box<dyn Error>> {
    match val {
        StringOrNumber::String(str) => Ok(set_placeholder_values_in_string(str, runtime, inputs).parse()?),
        StringOrNumber::Integer(uint) => Ok(*uint),
        StringOrNumber::Decimal(float) => Ok(float.ceil() as u64),
    }
}

pub(crate) fn collect_environment(tool: &CWLDocument) -> HashMap<String, String> {
    tool.hints
        .iter()
        .chain(tool.requirements.iter())
        .flatten()
        .filter_map(|req| {
            if let Requirement::EnvVarRequirement(env) = req {
                match &env.env_def {
                    EnviromentDefs::Vec(vec) => Some(vec.iter().map(|i| (i.env_name.clone(), i.env_value.clone())).collect::<HashMap<_, _>>()),
                    EnviromentDefs::Map(map) => Some(map.clone()),
                }
            } else {
                None
            }
        })
        .flatten()
        .collect()
}

pub(crate) fn collect_inputs(
    tool: &CWLDocument,
    input_values: HashMap<String, DefaultValue>,
    tool_dir: impl AsRef<Path>,
) -> Result<HashMap<String, DefaultValue>, Box<dyn Error>> {
    let mut inputs = HashMap::new();
    for input in &tool.inputs {
        let mut result_input = evaluate_input(input, &input_values)?;
        if let DefaultValue::File(f) = &mut result_input {
            if input.load_contents {
                f.contents = Some(fs::read_to_string(f.location.as_ref().expect("Could not read file"))?);
            }
            //load file meta
            f.load(&tool_dir);
        } else if let DefaultValue::Directory(d) = &mut result_input {
            d.load(&tool_dir);
        }
        inputs.insert(input.id.clone(), result_input);
    }
    Ok(inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cwl::{clt::CommandLineTool, requirements::EnvVarRequirement};

    #[test]
    fn test_requirements_overwrite_hints() {
        let hint = EnvVarRequirement {
            env_def: EnviromentDefs::Map(HashMap::from([("MY_ENV".to_string(), "HINT".to_string())])),
        };
        let requirement = EnvVarRequirement {
            env_def: EnviromentDefs::Map(HashMap::from([("MY_ENV".to_string(), "REQUIREMENT".to_string())])),
        };

        let tool = CommandLineTool::default()
            .with_requirements(vec![Requirement::EnvVarRequirement(requirement)])
            .with_hints(vec![Requirement::EnvVarRequirement(hint)]);

        let environment = collect_environment(&CWLDocument::CommandLineTool(tool));

        assert_eq!(environment["MY_ENV"], "REQUIREMENT".to_string())
    }
}
