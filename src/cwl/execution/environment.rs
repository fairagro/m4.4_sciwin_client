use crate::cwl::clt::{CommandLineTool, EnvVarRequirement, EnviromentDefs, Requirement};
use std::env;

pub fn set_tool_environment_vars(tool: &CommandLineTool) -> Vec<String> {
    let mut keys = vec![];

    for req in tool.requirements.iter().chain(tool.hints.iter()).flatten() {
        if let Requirement::EnvVarRequirement(env_defs) = req {
            keys.extend(set_environment_vars(env_defs));
        }
    }
    keys
}

fn set_environment_vars(requirement: &EnvVarRequirement) -> Vec<String> {
    let mut keys = vec![];

    match &requirement.env_def {
        EnviromentDefs::Vec(vec) => {
            for def in vec {
                env::set_var(&def.env_name, &def.env_value);
                keys.push(def.env_name.to_string());
            }
        }
        EnviromentDefs::Map(map) => {
            for (key, value) in map {
                env::set_var(key, value);
                keys.push(key.to_string());
            }
        }
    }
    keys
}

pub fn unset_environment_vars(keys: Vec<String>) {
    for key in keys {
        env::remove_var(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, vec};

    #[test]
    fn test_set_environment_vars() {
        let mut current_vars = env::vars();
        assert!(!current_vars.any(|v| v.0 == "MY_COOL_VAR"));

        let mut env_map = HashMap::new();
        env_map.insert("MY_COOL_VAR".to_string(), "my awesome value".to_string());

        let requirement = EnvVarRequirement {
            env_def: EnviromentDefs::Map(env_map),
        };

        let keys = set_environment_vars(&requirement);
        assert_eq!(keys, vec!["MY_COOL_VAR"]);

        //exists now!
        let mut current_vars = env::vars();
        assert!(current_vars.any(|v| v.0 == "MY_COOL_VAR"));

        unset_environment_vars(keys);

        //gone again
        let mut current_vars = env::vars();
        assert!(!current_vars.any(|v| v.0 == "MY_COOL_VAR"));
    }
}
