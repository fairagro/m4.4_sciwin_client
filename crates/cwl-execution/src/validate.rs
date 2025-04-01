use crate::{
    environment::RuntimeEnvironment,
    io::{get_file_property, make_relative_to},
};
use cwl::{
    clt::{Argument, Command, CommandLineTool},
    inputs::CommandInputParameter,
    requirements::Requirement,
    types::{DefaultValue, Directory, Entry, EnviromentDefs, File, PathItem},
    CWLDocument,
};
use fancy_regex::Regex;
use pathdiff::diff_paths;
use std::{collections::HashMap, env};

/// Replaces placeholders like $(inputs.test) or $(runtime.cpu) with its actual evaluated values
pub(crate) fn set_placeholder_values(cwl: &mut CWLDocument, runtime: &RuntimeEnvironment) {
    let inputs = &cwl.inputs.clone();
    //set values in requirements
    if let Some(requirements) = &mut cwl.requirements {
        set_placeholder_values_requirements(requirements, runtime, inputs);
    }

    //set values in hints
    if let Some(requirements) = &mut cwl.hints {
        set_placeholder_values_requirements(requirements, runtime, inputs);
    }

    if let CWLDocument::CommandLineTool(clt) = cwl {
        set_placeholder_values_tool(clt, runtime);
    }
}

pub(crate) fn rewire_paths(cwl: &mut CWLDocument, input_values: &mut HashMap<String, DefaultValue>, staged_files: &[String], home_dir: &str) {
    //rewire in inputs
    for input in cwl.inputs.iter_mut() {
        if let Some(default) = &mut input.default {
            let mut new_default = default.clone();
            for staged_file in staged_files {
                new_default = rewire_default_value(new_default, staged_file, home_dir)
            }
            *default = new_default;
        }

        //rewire in values
        if let Some(existing_value) = input_values.get(&input.id) {
            let mut new_value = existing_value.clone();
            for staged_file in staged_files {
                new_value = rewire_default_value(new_value.clone(), staged_file, home_dir);
            }
            input_values.insert(input.id.clone(), new_value);
        }
    }
}

fn rewire_default_value(value: DefaultValue, staged_file: &String, home_dir: &str) -> DefaultValue {
    match value {
        DefaultValue::File(file) => {
            let file_loc = file.get_location();
            let location = make_relative_to(&file_loc, home_dir).trim_start_matches("../");
            let test = env::current_dir().unwrap().join(location);
            if let Some(diff) = diff_paths(test, staged_file) {
                if diff.to_str() == Some("") {
                    let new_location = staged_file;
                    DefaultValue::File(File::from_location(new_location))
                } else {
                    DefaultValue::File(file)
                }
            } else {
                DefaultValue::File(file)
            }
        }
        DefaultValue::Directory(directory) => {
            let dir_loc = directory.get_location();
            let location = make_relative_to(&dir_loc, home_dir).trim_start_matches("../");
            let test = env::current_dir().unwrap().join(location);
            if let Some(diff) = diff_paths(test, staged_file) {
                if diff.to_str() == Some("") {
                    let new_location = staged_file;
                    DefaultValue::Directory(Directory::from_location(new_location))
                } else {
                    DefaultValue::Directory(directory)
                }
            } else {
                DefaultValue::Directory(directory)
            }
        }
        DefaultValue::Any(value) => DefaultValue::Any(value),
    }
}

fn set_placeholder_values_tool(clt: &mut CommandLineTool, runtime: &RuntimeEnvironment) {
    //set values in baseCommand
    clt.base_command = match &clt.base_command {
        Command::Single(cmd) => Command::Single(set_placeholder_values_in_string(cmd, runtime, &clt.base.inputs)),
        Command::Multiple(vec) => {
            let mut new_command = vec![];
            for item in vec {
                new_command.push(set_placeholder_values_in_string(item, runtime, &clt.base.inputs));
            }
            Command::Multiple(new_command)
        }
    };

    //set values in arguments
    if let Some(args) = &mut clt.arguments {
        for arg in args.iter_mut() {
            *arg = match arg {
                Argument::String(str) => {
                    let new_str = set_placeholder_values_in_string(str, runtime, &clt.base.inputs);
                    Argument::String(new_str)
                }
                Argument::Binding(binding) => {
                    let mut new_binding = binding.clone();
                    if let Some(value_from) = &mut new_binding.value_from {
                        *value_from = set_placeholder_values_in_string(value_from, runtime, &clt.base.inputs);
                    }
                    Argument::Binding(new_binding)
                }
            }
        }
    }

    //set values in output glob
    for output in clt.outputs.iter_mut() {
        if let Some(binding) = &mut output.output_binding {
            let glob = set_placeholder_values_in_string(&binding.glob, runtime, &clt.base.inputs);
            binding.glob = glob;
        }
    }

    //set values in output format
    for output in clt.outputs.iter_mut() {
        if let Some(format) = &mut output.format {
            let format = set_placeholder_values_in_string(format, runtime, &clt.base.inputs);
            output.format = Some(format);
        }
    }

    //set values in stdin
    if let Some(stdin) = &mut clt.stdin {
        *stdin = set_placeholder_values_in_string(stdin, runtime, &clt.base.inputs);
    }
}

fn set_placeholder_values_requirements(requirements: &mut Vec<Requirement>, runtime: &RuntimeEnvironment, inputs: &[CommandInputParameter]) {
    for requirement in requirements {
        if let Requirement::EnvVarRequirement(env_req) = requirement {
            env_req.env_def = match &mut env_req.env_def {
                EnviromentDefs::Vec(vec) => {
                    for env_def in vec.iter_mut() {
                        env_def.env_value = set_placeholder_values_in_string(&env_def.env_value, runtime, inputs)
                    }
                    EnviromentDefs::Vec(vec.clone())
                }
                EnviromentDefs::Map(hash_map) => {
                    for (_key, value) in hash_map.iter_mut() {
                        *value = set_placeholder_values_in_string(value, runtime, inputs);
                    }
                    EnviromentDefs::Map(hash_map.clone())
                }
            }
        }

        if let Requirement::InitialWorkDirRequirement(wd_req) = requirement {
            for listing in wd_req.listing.iter_mut() {
                listing.entryname = set_placeholder_values_in_string(&listing.entryname, runtime, inputs);
                listing.entry = match &mut listing.entry {
                    Entry::Source(src) => {
                        *src = set_placeholder_values_in_string(src, runtime, inputs);
                        Entry::Source(src.clone())
                    }
                    Entry::Include(include) => {
                        let updated_include = set_placeholder_values_in_string(&include.include, runtime, inputs);
                        include.include = updated_include;
                        Entry::Include(include.clone())
                    }
                }
            }
        }
    }
}

fn set_placeholder_values_in_string(text: &str, runtime: &RuntimeEnvironment, inputs: &[CommandInputParameter]) -> String {
    let in_re = Regex::new(r"\$\(inputs.([\w.]*)\)").unwrap();
    let run_re = Regex::new(r"\$\(runtime.([\w]*)\)").unwrap();
    let result = in_re.replace_all(text, |caps: &fancy_regex::Captures| {
        let placeholder = &caps[1];
        if let Some((base, suffix)) = placeholder.rsplit_once('.') {
            let mut input_value =
                get_input_value(base, &runtime.inputs, inputs, suffix).unwrap_or_else(|| panic!("Input not provided for {}", placeholder));
            if suffix == "dirname" {
                if let Some(diff) = diff_paths(&input_value, &runtime.runtime["tooldir"]) {
                    if let Some(diff_str) = diff.to_str() {
                        input_value = format!("./{}", input_value.trim_start_matches(diff_str));
                    }
                }
            }
            input_value
        } else {
            get_input_value(placeholder, &runtime.inputs, inputs, "").unwrap_or_else(|| panic!("Input not provided for {}", placeholder))
        }
    });
    run_re
        .replace_all(&result, |caps: &fancy_regex::Captures| {
            let placeholder = &caps[1];
            runtime.runtime[placeholder].clone()
        })
        .to_string()
}

/// Evaluate inputs and given parameters for given key
fn get_input_value(key: &str, input_values: &HashMap<String, DefaultValue>, inputs: &[CommandInputParameter], suffix: &str) -> Option<String> {
    let mut value = None;

    fn evaluate(value: &DefaultValue, suffix: &str) -> Option<String> {
        if let DefaultValue::File(file) = value {
            if suffix == "format" {
                file.format.clone()
            } else {
                Some(get_file_property(file.get_location(), suffix))
            }
        } else {
            Some(value.as_value_string())
        }
    }

    for input in inputs {
        if input.id == key {
            if let Some(default) = &input.default {
                value = evaluate(default, suffix);
            }
        }
    }

    if input_values.contains_key(key) {
        value = evaluate(&input_values[key], suffix);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::get_file_size;
    use cwl::types::{CWLType, File};
    use serde_yaml::Value;

    #[test]
    pub fn test_set_placeholder_values() {
        let cwl_str = r"class: CommandLineTool
cwlVersion: v1.2
baseCommand: $(runtime.true)
requirements:
  InitialWorkDirRequirement:
    listing:
      - entryname: $(inputs.newname)
        entry: $(inputs.srcfile)
inputs:
  srcfile: File
  newname: string
outputs:
  outfile:
    type: File
    outputBinding:
      glob: $(inputs.newname)";

        let expected_str = r#"class: CommandLineTool
cwlVersion: v1.2
baseCommand: "true"
requirements:
  InitialWorkDirRequirement:
    listing:
      - entryname: neuer_name.txt
        entry: tests/test_data/input.txt
inputs:
  srcfile: File
  newname: string
outputs:
  outfile:
    type: File
    outputBinding:
      glob: neuer_name.txt"#;

        let mut runtime = HashMap::new();
        runtime.insert("true".to_string(), "true".to_string());
        let mut input_values = HashMap::new();
        input_values.insert("newname".to_string(), DefaultValue::Any(Value::String("neuer_name.txt".to_string())));
        input_values.insert(
            "srcfile".to_string(),
            DefaultValue::File(File::from_location(&"tests/test_data/input.txt".to_string())),
        );

        let runtime = RuntimeEnvironment {
            runtime,
            inputs: input_values,
            ..Default::default()
        };

        let mut cwl_test: CWLDocument = serde_yaml::from_str(cwl_str).unwrap();
        set_placeholder_values(&mut cwl_test, &runtime);

        let cwl_expected: CWLDocument = serde_yaml::from_str(expected_str).unwrap();

        assert_eq!(cwl_test, cwl_expected);
    }

    #[test]
    pub fn test_set_placeholder_values_in_string() {
        let text = "Searching for file $(inputs.infile.path)";
        let file = "tests/test_data/input.txt";
        let runtime = Default::default();
        let inputs = vec![CommandInputParameter::default()
            .with_id("infile")
            .with_type(CWLType::File)
            .with_default_value(DefaultValue::File(File::from_location(&file.to_string())))];

        let result = set_placeholder_values_in_string(text, &runtime, &inputs);
        let expected = format!("Searching for file {file}");

        assert_eq!(result, expected);
    }

    #[test]
    pub fn test_set_placeholder_values_in_string_size() {
        let text = "File has size $(inputs.infile.size)";
        let file = "../../tests/test_data/input.txt";
        let size = get_file_size(file).unwrap();
        let runtime = Default::default();
        let inputs = vec![CommandInputParameter::default()
            .with_id("infile")
            .with_type(CWLType::File)
            .with_default_value(DefaultValue::File(File::from_location(&file.to_string())))];

        let result = set_placeholder_values_in_string(text, &runtime, &inputs);
        let expected = format!("File has size {size}");

        assert_eq!(result, expected);
    }

    #[test]
    pub fn test_set_placeholder_values_in_string_contents() {
        let text = "Greeting: $(inputs.infile)";
        let file = "tests/test_data/input.txt";
        let runtime = Default::default();
        let inputs = vec![CommandInputParameter::default()
            .with_id("infile")
            .with_type(CWLType::File)
            .with_default_value(DefaultValue::File(File::from_location(&file.to_string())))];

        let result = set_placeholder_values_in_string(text, &runtime, &inputs);
        let expected = "Greeting: tests/test_data/input.txt";

        assert_eq!(result, expected);
    }

    #[test]
    pub fn test_set_placeholder_values_in_string_input_values() {
        let text = "Greeting: $(inputs.infile)";
        let file = "tests/test_data/input.txt";

        let mut values: HashMap<String, DefaultValue> = HashMap::new();
        values.insert("infile".to_string(), DefaultValue::File(File::from_location(&file.to_string())));
        let runtime = RuntimeEnvironment {
            inputs: values,
            ..Default::default()
        };
        let inputs = vec![CommandInputParameter::default().with_id("infile").with_type(CWLType::File)];

        let result = set_placeholder_values_in_string(text, &runtime, &inputs);
        let expected = "Greeting: tests/test_data/input.txt";

        assert_eq!(result, expected);
    }

    #[test]
    pub fn test_set_placeholder_values_in_string_runtime() {
        let text = "Greeting: $(runtime.whatever_value)!";

        let mut runtime: HashMap<String, String> = HashMap::new();
        runtime.insert("whatever_value".to_string(), "Hello World".to_string());
        let runtime = RuntimeEnvironment {
            runtime,
            ..Default::default()
        };

        let result = set_placeholder_values_in_string(text, &runtime, &[]);
        let expected = "Greeting: Hello World!";

        assert_eq!(result, expected);
    }
}
