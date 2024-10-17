use super::clt::{Command, CommandLineTool, DefaultValue};
use crate::io::create_and_write_file;
use std::{collections::HashMap, error::Error, process::Command as SystemCommand};

pub fn run_command_line_tool(tool: &CommandLineTool, input_values: Option<HashMap<String, DefaultValue>>) -> Result<(), Box<dyn Error>> {
    //build command

    //get executable
    let cmd = match &tool.base_command {
        Command::Single(cmd) => cmd,
        Command::Multiple(vec) => &vec[0],
    };

    let mut command = SystemCommand::new(cmd);
    //append rest of base command as args
    if let Command::Multiple(ref vec) = &tool.base_command {
        command.args(&vec[1..]);
    }

    //TODO: handle arguments field...

    //build inputs from either fn-args or default values.
    let mut inputs = vec![];
    for input in &tool.inputs {
        if let Some(binding) = &input.input_binding {
            if let Some(prefix) = &binding.prefix {
                inputs.push(prefix.clone());
            }
        }
        if let Some(ref values) = input_values {
            if let Some(value) = values.get(&input.id) {
                if !value.has_matching_type(&input.type_) {
                    //change handling accordingly in utils on main branch!
                    eprintln!("CWLType is not matching input type");
                    Err("CWLType is not matching input type")?;
                }
                inputs.push(value.as_value_string());
            }
        } else if let Some(default_) = &input.default {
            inputs.push(default_.as_value_string());
        } else {
            eprintln!("You did not include a value for {}", input.id);
            Err(format!("You did not include a value for {}", input.id).as_str())?;
        }
    }
    command.args(inputs);

    //run
    let output = command.output()?;

    //handle redirection of stdout
    if !output.stdout.is_empty() {
        if let Some(stdout) = &tool.stdout {
            let out = &String::from_utf8_lossy(&output.stdout);
            create_and_write_file(stdout, out)?;
        } else {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }

    //handle redirection of stderr
    if !output.stderr.is_empty() {
        if let Some(stderr) = &tool.stderr {
            let out = &String::from_utf8_lossy(&output.stderr);
            create_and_write_file(stderr, out)?;
        } else {
            eprintln!("❌ {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}
