use cwl::{
    requirements::Requirement,
    types::{DefaultValue, EnviromentDefs},
    CWLDocument,
};
use std::{
    collections::HashMap,
    error::Error,
    fs::{self},
};

use crate::util::evaluate_input;

#[derive(Debug, Default, Clone)]
pub struct RuntimeEnvironment {
    pub runtime: HashMap<String, String>,
    pub inputs: HashMap<String, DefaultValue>,
    pub environment: HashMap<String, String>,
    pub time_limit: u64,
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
) -> Result<HashMap<String, DefaultValue>, Box<dyn Error>> {
    let mut inputs = HashMap::new();
    for input in &tool.inputs {
        let mut result_input = evaluate_input(input, &input_values)?;
        if let DefaultValue::File(f) = &mut result_input {
            if input.load_contents {
                f.contents = Some(fs::read_to_string(f.location.as_ref().expect("Could not read file"))?);
            }
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
