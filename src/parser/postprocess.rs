use cwl::{
    inputs::CommandInputParameter,
    requirements::{InlineJavascriptRequirement, Requirement},
    types::{CWLType, DefaultValue},
    Argument, CommandLineTool,
};
use std::collections::HashSet;

/// Applies some postprocessing to the cwl CommandLineTool
pub fn post_process_cwl(tool: &mut CommandLineTool) {
    detect_array_inputs(tool);
    post_process_variables(tool);
    post_process_ids(tool);
}

/// Transforms duplicate key and type entries into an array type input
fn detect_array_inputs(tool: &mut CommandLineTool) {
    let mut seen = HashSet::new();
    let mut inputs = Vec::new();

    for input in std::mem::take(&mut tool.inputs) {
        let key = (input.id.clone(), input.type_.clone());
        if seen.insert(key.clone()) {
            inputs.push(input);
        } else if let Some(existing) = inputs.iter_mut().find(|i| i.id == key.0) {
            // Convert to array type if not already
            if !matches!(existing.type_, CWLType::Array(_)) {
                existing.type_ = CWLType::Array(Box::new(input.type_.clone()));

                if let Some(default) = &existing.default {
                    existing.default = Some(DefaultValue::Array(vec![default.clone()]));
                }
            }

            // Append additional default value if present
            if let Some(DefaultValue::Array(defaults)) = &mut existing.default {
                if let Some(default) = input.default {
                    defaults.push(default);
                }
            }
        }
    }
    tool.inputs = inputs;
}

/// Handles translation to CWL Variables like $(inputs.myInput.path) or $(runtime.outdir)
fn post_process_variables(tool: &mut CommandLineTool) {
    fn process_input(input: &CommandInputParameter) -> String {
        if input.type_ == CWLType::File || input.type_ == CWLType::Directory {
            format!("$(inputs.{}.path)", input.id)
        } else {
            format!("$(inputs.{})", input.id)
        }
    }

    let mut processed_once = false;
    let inputs = tool.inputs.clone();
    for input in &inputs {
        if let Some(default) = &input.default {
            for output in &mut tool.outputs {
                if let Some(binding) = &mut output.output_binding {
                    if binding.glob == Some(default.as_value_string()) {
                        binding.glob = Some(process_input(input));
                        processed_once = true;
                    }
                }
            }
            if let Some(stdout) = &tool.stdout {
                if *stdout == default.as_value_string() {
                    tool.stdout = Some(process_input(input));
                    processed_once = true;
                }
            }
            if let Some(stderr) = &tool.stderr {
                if *stderr == default.as_value_string() {
                    tool.stderr = Some(process_input(input));
                    processed_once = true;
                }
            }
            if let Some(arguments) = &mut tool.arguments {
                for argument in arguments.iter_mut() {
                    match argument {
                        Argument::String(s) => {
                            if *s == default.as_value_string() {
                                *s = process_input(input);
                                processed_once = true;
                            }
                        }
                        Argument::Binding(binding) => {
                            if let Some(from) = &mut binding.value_from {
                                if *from == default.as_value_string() {
                                    *from = process_input(input);
                                    processed_once = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for output in &mut tool.outputs {
        if let Some(binding) = &mut output.output_binding {
            if matches!(binding.glob.as_deref(), Some(".")) {
                output.id = "output_directory".to_string();
                binding.glob = Some("$(runtime.outdir)".to_string());
            }
        }
    }

    if processed_once {
        tool.requirements
            .push(Requirement::InlineJavascriptRequirement(InlineJavascriptRequirement::default()));
    }
}

/// Post-processes output IDs to ensure they do not conflict with input IDs
fn post_process_ids(tool: &mut CommandLineTool) {
    let input_ids = tool.inputs.iter().map(|i| i.id.clone()).collect::<HashSet<_>>();
    for output in &mut tool.outputs {
        if input_ids.contains(&output.id) {
            output.id = format!("o_{}", output.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    #[test]
    pub fn test_post_process_inputs() {
        let mut tool = CommandLineTool::default().with_inputs(vec![
            CommandInputParameter::default()
                .with_id("arr")
                .with_type(CWLType::String)
                .with_default_value(DefaultValue::Any(Value::String("first".to_string()))),
            CommandInputParameter::default()
                .with_id("arr")
                .with_type(CWLType::String)
                .with_default_value(DefaultValue::Any(Value::String("second".to_string()))),
            CommandInputParameter::default()
                .with_id("arr")
                .with_type(CWLType::String)
                .with_default_value(DefaultValue::Any(Value::String("third".to_string()))),
            CommandInputParameter::default().with_id("int").with_type(CWLType::Int),
        ]);

        assert_eq!(tool.inputs.len(), 4);
        detect_array_inputs(&mut tool);
        assert_eq!(tool.inputs.len(), 2);

        let of_interest = tool.inputs.first().unwrap();
        assert_eq!(of_interest.type_, CWLType::Array(Box::new(CWLType::String)));
        assert_eq!(
            of_interest.default,
            Some(DefaultValue::Array(vec![
                DefaultValue::Any(Value::String("first".to_string())),
                DefaultValue::Any(Value::String("second".to_string())),
                DefaultValue::Any(Value::String("third".to_string()))
            ]))
        );

        let other = &tool.inputs[1];
        assert_eq!(other.type_, CWLType::Int);
    }
}
