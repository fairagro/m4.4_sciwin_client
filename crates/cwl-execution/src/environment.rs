use cwl::{
    requirements::Requirement,
    types::{DefaultValue, EnviromentDefs}, CWLDocument,
};
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct RuntimeEnvironment {
    pub runtime: HashMap<String, String>,
    pub inputs: HashMap<String, DefaultValue>,
    pub environment: HashMap<String, String>,
    pub time_limit: u64,
}

pub(crate) fn collect_environment(tool: &CWLDocument) -> HashMap<String, String> {
    tool.requirements
        .iter()
        .chain(tool.hints.iter())
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
