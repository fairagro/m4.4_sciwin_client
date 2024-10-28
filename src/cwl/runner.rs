use super::clt::{Command, CommandInputParameter, CommandLineTool, DefaultValue, Entry, Requirement};
use crate::{
    cwl::types::CWLType,
    io::{copy_file, create_and_write_file},
};
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::{self, create_dir_all},
    path::Path,
    process::Command as SystemCommand,
};
use tempfile::tempdir;

pub fn run_commandlinetool(tool: &CommandLineTool, input_values: Option<HashMap<String, DefaultValue>>, cwl_path: Option<&str>) -> Result<(), Box<dyn Error>> {
    //TODO: handle container
    let dir = tempdir()?;
    println!("📁 Created staging directory: {:?}", dir.path());

    //save current dir
    let current = env::current_dir()?;
    let tool_path = if let Some(file) = cwl_path { Path::new(file).parent().unwrap() } else { Path::new(".") };
    //change to cwl dir as paths are given relative to here
    env::set_current_dir(current.join(tool_path))?;

    //stage initial workdir
    if let Some(req) = &tool.requirements {
        for item in req {
            if let Requirement::InitialWorkDirRequirement(iwdr) = item {
                for listing in &iwdr.listing {
                    let path = dir.path().join(tool_path).join(&listing.entryname);
                    match &listing.entry {
                        Entry::Source(src) => {
                            create_and_write_file(&path.to_string_lossy(), src).map_err(|e| format!("Failed to create and write file {:?}: {}", path, e))?;
                        }
                        Entry::Include(f) => {
                            copy_file(&f.include, &path.to_string_lossy()).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", f.include, path, e))?;
                        }
                    }
                }
            }
        }
    }

    //stage inputs
    for input in &tool.inputs {
        //TODO: Handle directories
        if input.type_ == CWLType::File {
            let in_file = evaluate_input(input, &input_values)?;
            let file = in_file.trim_start_matches("../");
            let path = dir.path().join(&file);
            copy_file(&in_file, &path.to_string_lossy()).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", file, path, e))?;
        }
    }

    //change working directory and run command
    let tmp_tool_dir = dir.path().join(tool_path);
    create_dir_all(&tmp_tool_dir)?;
    env::set_current_dir(tmp_tool_dir)?;

    run_command(tool, input_values).map_err(|e| format!("Could not execute tool command: {}", e))?;

    //copy back requested output
    for output in &tool.outputs {
        if let Some(binding) = &output.output_binding {
            let path = &current.join(&binding.glob);
            fs::copy(&binding.glob, path).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", &binding.glob, path, e))?;
            println!("📜 Wrote output file: {:?}", path);
        }
    }

    env::set_current_dir(&current)?;

    println!("✔️  Command Executed with status: success!");
    Ok(())
}

///Either gets the default value for input or the provided one (preferred)
fn evaluate_input(input: &CommandInputParameter, input_values: &Option<HashMap<String, DefaultValue>>) -> Result<String, Box<dyn Error>> {
    if let Some(ref values) = input_values {
        if let Some(value) = values.get(&input.id) {
            if !value.has_matching_type(&input.type_) {
                //change handling accordingly in utils on main branch!
                eprintln!("CWLType is not matching input type");
                Err("CWLType is not matching input type")?;
            }
            return Ok(value.as_value_string());
        }
    } else if let Some(default_) = &input.default {
        return Ok(default_.as_value_string());
    } else {
        eprintln!("You did not include a value for {}", input.id);
        Err(format!("You did not include a value for {}", input.id).as_str())?;
    }
    Err(format!("Could not evaluate input: {}", input.id))?
}

pub fn run_command(tool: &CommandLineTool, input_values: Option<HashMap<String, DefaultValue>>) -> Result<(), Box<dyn Error>> {
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
        inputs.push(evaluate_input(input, &input_values)?);
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
