use crate::{
    collect_inputs,
    expression::{set_self, unset_self},
    replace_expressions,
    util::{create_file, get_random_filename, get_shell_command},
    RuntimeEnvironment,
};
use cwl::{
    clt::{Argument, Command, CommandLineTool},
    inputs::CommandLineBinding,
    types::{CWLType, DefaultValue},
};
use serde_yaml::Value;
use std::{collections::HashMap, env, path::PathBuf, process::Command as SystemCommand};

pub fn run_command(tool: &CommandLineTool, runtime: Option<&RuntimeEnvironment>) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = build_command(tool, runtime)?;

    //execute command
    let output = command.output()?;
    let dir = if let Some(runtime) = runtime {
        PathBuf::from(runtime.runtime["outdir"].clone())
    } else {
        env::current_dir()?
    };

    //handle redirection of stdout
    if !output.stdout.is_empty() {
        let out = &String::from_utf8_lossy(&output.stdout);
        if let Some(stdout) = &tool.stdout {
            create_file(dir.join(stdout), out)?;
        } else if tool.has_stdout_output() {
            let output = tool.outputs.iter().filter(|o| matches!(o.type_, CWLType::Stdout)).collect::<Vec<_>>()[0];
            let filename = if let Some(binding) = &output.output_binding {
                &binding.glob
            } else {
                &get_random_filename(&format!("{}_stdout", output.id), "out")
            };
            create_file(dir.join(filename), out)?;
        } else {
            eprintln!("{}", out);
        }
    }

    //handle redirection of stderr
    if !output.stderr.is_empty() {
        let out = &String::from_utf8_lossy(&output.stderr);
        if let Some(stderr) = &tool.stderr {
            create_file(dir.join(stderr), out)?;
        } else if tool.has_stderr_output() {
            let output = tool.outputs.iter().filter(|o| matches!(o.type_, CWLType::Stderr)).collect::<Vec<_>>()[0];
            let filename = if let Some(binding) = &output.output_binding {
                &binding.glob
            } else {
                &get_random_filename(&format!("{}_stderr", output.id), "out")
            };
            create_file(dir.join(filename), out)?;
        } else {
            eprintln!("❌ {}", out);
        }
    }

    match output.status.success() {
        true => Ok(()),
        false => Err(format!("command returned with code {:?}", output.status.code().unwrap_or(1)).into()),
    }
}

fn build_command(tool: &CommandLineTool, runtime: Option<&RuntimeEnvironment>) -> Result<SystemCommand, Box<dyn std::error::Error>> {
    let mut args: Vec<String> = vec![];
    let inputs = if let Some(rt) = &runtime {
        rt.inputs.clone()
    } else {
        collect_inputs(tool, &HashMap::new())? //for tool create!
    };

    //get executable
    let cmd = match &tool.base_command {
        Command::Single(cmd) => cmd,
        Command::Multiple(vec) => {
            if !vec.is_empty() {
                &vec[0]
            } else {
                &String::new()
            }
        }
    };

    if !cmd.is_empty() {
        args.push(cmd.to_string());

        //append rest of base command as args
        if let Command::Multiple(ref vec) = &tool.base_command {
            args.extend(vec[1..].iter().cloned());
        }
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
                let val = inputs.get(&input.id).unwrap_or(&DefaultValue::Any(Value::Null));
                if let DefaultValue::Any(Value::Sequence(vec)) = val {
                    for item in vec {
                        binding.value_from = Some(serde_yaml::to_string(item)?.trim().to_string()); //will not support all types, TODO!!
                        if vec.last().unwrap() != item {
                            bindings.push((position, i + index, binding.clone()));
                        }
                    }
                } else {
                    binding.value_from = Some(val.as_value_string())
                };
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
        command.envs(runtime.environment.clone());
        command.env("HOME", &runtime.runtime["outdir"]);
        command.env("TMPDIR", &runtime.runtime["tmpdir"]);
        command.current_dir(&runtime.runtime["outdir"]);
    }

    Ok(command)
}
