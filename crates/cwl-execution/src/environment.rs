use crate::{
    get_available_disk_space, get_available_ram, get_processor_count, util::evaluate_input, validate::set_placeholder_values_in_string, InputObject,
};
use cwl::{
    inputs::CommandInputParameter,
    requirements::{EnvVarRequirement, NetworkAccess, ResourceRequirement, ToolTimeLimit},
    types::{DefaultValue, EnviromentDefs},
    CWLDocument, StringOrNumber,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    error::Error,
    fs::{self},
    path::Path,
};

#[derive(Serialize, Debug, Default, Clone)]
pub struct RuntimeEnvironment {
    pub runtime: HashMap<String, StringOrNumber>,
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
    pub fn with_runtime(mut self, runtime: HashMap<String, StringOrNumber>) -> Self {
        self.runtime = runtime;
        self
    }
    pub fn with_time_limit(mut self, time_limit: u64) -> Self {
        self.time_limit = time_limit;
        self
    }

    pub fn initialize(
        tool: &CWLDocument,
        input_values: &InputObject,
        outdir: impl AsRef<Path>,
        tooldir: impl AsRef<Path>,
        tmpdir: impl AsRef<Path>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut runtime = HashMap::from([
            (
                "tooldir".to_string(),
                StringOrNumber::String(tooldir.as_ref().to_string_lossy().into_owned()),
            ),
            (
                "outdir".to_string(),
                StringOrNumber::String(outdir.as_ref().to_string_lossy().into_owned()),
            ),
            (
                "tmpdir".to_string(),
                StringOrNumber::String(tmpdir.as_ref().to_string_lossy().into_owned()),
            ),
            ("outdirSize".to_string(), StringOrNumber::Integer(get_available_disk_space())),
            ("tmpdirSize".to_string(), StringOrNumber::Integer(get_available_disk_space())),
            ("cores".to_string(), StringOrNumber::Integer(get_processor_count() as u64)),
            ("ram".to_string(), StringOrNumber::Integer(get_available_ram())),
        ]);

        runtime.insert(
            "network".to_string(),
            if input_values.get_requirement::<NetworkAccess>().is_some() {
                StringOrNumber::Integer(1)
            } else {
                StringOrNumber::Integer(0)
            },
        );

        let inputs = collect_inputs(tool, &input_values.inputs, tooldir)?;

        let mut environment = RuntimeEnvironment {
            runtime,
            time_limit: input_values.get_requirement::<ToolTimeLimit>().map(|tt| tt.timelimit).unwrap_or(0),
            inputs,
            ..Default::default()
        };

        if let Some(rr) = input_values.get_requirement::<ResourceRequirement>() {
            if let Some(cores) = &rr.cores_min {
                environment
                    .runtime
                    .insert("cores".to_string(), StringOrNumber::Integer(evaluate(cores, &environment, &tool.inputs)?));
            }
            if let Some(ram) = &rr.ram_min {
                environment
                    .runtime
                    .insert("ram".to_string(), StringOrNumber::Integer(evaluate(ram, &environment, &tool.inputs)?));
            }
            if let Some(dir_size) = &rr.outdir_min {
                environment.runtime.insert(
                    "outdirSize".to_string(),
                    StringOrNumber::Integer(evaluate(dir_size, &environment, &tool.inputs)?),
                );
            }
            if let Some(tmp_size) = &rr.tmpdir_min {
                environment.runtime.insert(
                    "tmpdirSize".to_string(),
                    StringOrNumber::Integer(evaluate(tmp_size, &environment, &tool.inputs)?),
                );
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

pub(crate) fn collect_environment(input_values: &InputObject) -> HashMap<String, String> {
    if let Some(env) = input_values.get_requirement::<EnvVarRequirement>() {
        match &env.env_def {
            EnviromentDefs::Vec(vec) => vec.iter().map(|i| (i.env_name.clone(), i.env_value.clone())).collect::<HashMap<_, _>>(),
            EnviromentDefs::Map(map) => map.clone(),
        }
    } else {
        HashMap::new()
    }
}

pub(crate) fn collect_inputs(
    tool: &CWLDocument,
    input_values: &HashMap<String, DefaultValue>,
    tool_dir: impl AsRef<Path>,
) -> Result<HashMap<String, DefaultValue>, Box<dyn Error>> {
    let mut inputs = HashMap::new();
    for input in &tool.inputs {
        let mut result_input = evaluate_input(input, input_values)?;
        if let DefaultValue::File(f) = &mut result_input {
            if input.load_contents {
                if fs::metadata(f.location.as_ref().expect("Could not read file"))?.len() > 64 * 1024 {
                    return Err("File is too large to load contents (see: https://www.commonwl.org/v1.2/CommandLineTool.html#CommandInputParameter)".into());
                }
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
    use cwl::requirements::{EnvVarRequirement, Requirement};

    #[test]
    fn test_requirements_overwrite_hints() {
        let hint = EnvVarRequirement {
            env_def: EnviromentDefs::Map(HashMap::from([("MY_ENV".to_string(), "HINT".to_string())])),
        };
        let requirement = EnvVarRequirement {
            env_def: EnviromentDefs::Map(HashMap::from([("MY_ENV".to_string(), "REQUIREMENT".to_string())])),
        };
        let input_values = InputObject {
            requirements: vec![Requirement::EnvVarRequirement(requirement.clone())],
            hints: vec![Requirement::EnvVarRequirement(hint.clone())],
            ..Default::default()
        };

        let environment = collect_environment(&input_values);

        assert_eq!(environment["MY_ENV"], "REQUIREMENT".to_string())
    }
}
