use crate::{
    collect_inputs,
    expression::{set_self, unset_self},
    replace_expressions,
    util::get_shell_command,
    RuntimeEnvironment,
};
use cwl::{
    clt::{Argument, Command, CommandLineTool},
    inputs::CommandLineBinding,
    types::DefaultValue,
};
use serde_yaml::Value;
use std::{collections::HashMap, process::Command as SystemCommand};

pub fn build_command(tool: &CommandLineTool, runtime: Option<RuntimeEnvironment>) -> Result<SystemCommand, Box<dyn std::error::Error>> {
    let mut args: Vec<String> = vec![];
    let inputs = if let Some(rt) = &runtime {
        rt.inputs.clone()
    } else {
        collect_inputs(tool, &HashMap::new())? //for tool create!
    };

    //get executable
    let cmd = match &tool.base_command {
        Command::Single(cmd) => cmd,
        Command::Multiple(vec) => &vec[0],
    };

    args.push(cmd.to_string());
    //append rest of base command as args
    if let Command::Multiple(ref vec) = &tool.base_command {
        args.extend(vec[1..].iter().cloned());
    }

    let mut bindings: Vec<(isize, usize, CommandLineBinding)> = vec![];

    //handle arguments field...
    if let Some(arguments) = &tool.arguments {
        for (i, arg) in arguments.iter().enumerate() {
            match arg {
                Argument::String(str) => {
                    let binding = CommandLineBinding {
                        value_from: Some(str.clone()),
                        ..Default::default()
                    };
                    bindings.push((0, i, binding));
                }
                Argument::Binding(binding) => {
                    let position = binding.position.unwrap_or_default();
                    bindings.push((position, i, binding.clone()));
                }
            }
        }
    }
    let index = bindings.len() + 1;

    //handle inputs
    for (i, input) in tool.inputs.iter().enumerate() {
        if let Some(ref binding) = &input.input_binding {
            let value = inputs.get(&input.id);
            set_self(&value)?;
            let mut binding = binding.clone();
            let position = binding.position.unwrap_or_default(); //TODO: allow expressions
            if let Some(value_from) = &binding.value_from {
                if let Some(val) = value {
                    if let DefaultValue::Any(Value::Null) = val {
                        binding.value_from = Some(String::new())
                    } else {
                        binding.value_from = Some(replace_expressions(value_from).unwrap_or(value_from.to_string()));
                    }
                }
            } else {
                binding.value_from = Some(inputs.get(&input.id).unwrap_or(&DefaultValue::Any(Value::Null)).as_value_string());
            }
            binding.value_from = binding.value_from.map(|v| v.replace("'", ""));
            unset_self()?;

            bindings.push((position, i + index, binding))
        }
    }

    //do sorting
    bindings.sort_by(|a, b| {
        let cmp = a.0.cmp(&b.0);
        if cmp == std::cmp::Ordering::Equal {
            a.1.cmp(&b.1)
        } else {
            cmp
        }
    });

    //add bindings
    let inputs: Vec<CommandLineBinding> = bindings.iter().map(|(_, _, binding)| binding.clone()).collect();
    for input in &inputs {
        if let Some(prefix) = &input.prefix {
            args.push(prefix.to_string());
        }
        if let Some(value) = &input.value_from {
            if tool.has_shell_command_requirement() {
                if let Some(shellquote) = input.shell_quote {
                    if shellquote {
                        args.push(format!("\"{}\"", value));
                    } else {
                        args.push(value.to_string())
                    }
                } else {
                    args.push(value.to_string())
                }
            } else {
                args.push(value.to_string())
            }
        }
    }

    //remove empty args
    args.retain(|s| !s.is_empty());

    let mut command = if tool.has_shell_command_requirement() {
        let joined_args = args.iter().map(|s| s.as_str()).collect::<Vec<&str>>().join(" ");
        let mut cmd = get_shell_command();
        cmd.arg(joined_args);
        cmd
    } else {
        let mut cmd = SystemCommand::new(args[0].clone());
        for arg in &args[1..] {
            cmd.arg(arg);
        }
        cmd
    };

    //append stdin i guess?
    if let Some(stdin) = &tool.stdin {
        command.arg(stdin);
    }

    //put in env vars
    if let Some(runtime) = runtime {
        command.envs(runtime.environment);
    }

    Ok(command)
}
