use super::{
    clt::{Command, CommandInputParameter, CommandLineBinding, CommandLineTool, CommandOutputBinding, CommandOutputParameter, DefaultValue, InitialWorkDirRequirement, Requirement},
    types::{CWLType, Directory, File},
};
use crate::io::get_filename_without_extension;
use serde_yml::Value;
use slugify::slugify;
use std::path::Path;

//TODO complete list
static SCRIPT_EXECUTORS: &[&str] = &["python", "Rscript"];

pub fn parse_command_line(command: Vec<&str>) -> CommandLineTool {
    let base_command = get_base_command(&command);
    let inputs = get_inputs(match &base_command {
        Command::Single(_) => &command[1..],
        Command::Multiple(ref vec) => &command[vec.len()..],
    });

    let tool = CommandLineTool::default().with_base_command(base_command.clone()).with_inputs(inputs);

    match base_command {
        Command::Single(_) => tool,
        Command::Multiple(ref vec) => tool.with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(&vec[1]))]),
    }
}

pub fn parse_command_line_inputs(commands: Vec<&str>, inputs: Vec<&str>) -> CommandLineTool {
    let base_command = get_base_command(&commands);
    let tool;

    let input = get_inputs(match &base_command {
        Command::Single(_) => &inputs[1..],
        Command::Multiple(_) => &inputs[0..],
        //Command::Multiple(ref vec) => &inputs[vec.len()..],
    });
    tool = CommandLineTool::default().with_base_command(base_command.clone()).with_inputs(input);
    match base_command {
        Command::Single(_) => tool,
        Command::Multiple(ref vec) => tool.with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(&vec[1]))]),
    }
}

pub fn get_outputs(files: Vec<String>) -> Vec<CommandOutputParameter> {
    files
        .iter()
        .map(|f| {
            CommandOutputParameter::default()
                .with_type(CWLType::File)
                .with_id(get_filename_without_extension(f).unwrap_or(f.to_string()).as_str())
                .with_binding(CommandOutputBinding { glob: f.clone() })
        })
        .collect()
}


pub(crate) fn get_base_command(command: &[&str]) -> Command {
    if command.is_empty() {
        return Command::Single(String::from(""));
    };

    let mut base_command = vec![command[0].to_string()];

    if SCRIPT_EXECUTORS.iter().any(|&exec| command[0].starts_with(exec)) {
        base_command.push(command[1].to_string());
    }

    match base_command.len() {
        1 => Command::Single(command[0].to_string()),
        _ => Command::Multiple(base_command),
    }
}

pub(crate) fn get_inputs(args: &[&str]) -> Vec<CommandInputParameter> {
    let mut inputs = vec![];
    let mut i = 0;
    while i < args.len() {
        let arg = args[i];
        let input: CommandInputParameter;
        if arg.starts_with('-') {
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                //is not a flag, as next one is a value
                input = get_option(arg, args[i + 1]);
                i += 1
            } else {
                input = get_flag(arg)
            }
        } else {
            input = get_positional(arg, i);
        }
        inputs.push(input);
        i += 1;
    }
    inputs
}

fn get_positional(current: &str, index: usize) -> CommandInputParameter {
    let cwl_type = guess_type(current);
    let default_value = match cwl_type {
        CWLType::File => DefaultValue::File(File::from_location(&current.to_string())),
        CWLType::Directory => DefaultValue::Directory(Directory::from_location(&current.to_string())),
        _ => DefaultValue::Any(serde_yml::from_str(current).unwrap()),
    };
    CommandInputParameter::default()
        .with_id(slugify!(&current).as_str())
        .with_type(guess_type(current))
        .with_default_value(default_value)
        .with_binding(CommandLineBinding::default().with_position(index))
}

fn get_flag(current: &str) -> CommandInputParameter {
    let id = current.replace("-", "");
    CommandInputParameter::default()
        .with_binding(CommandLineBinding::default().with_prefix(&current.to_string()))
        .with_id(slugify!(&id).as_str())
        .with_type(CWLType::Boolean)
        .with_default_value(DefaultValue::Any(Value::Bool(true)))
}

fn get_option(current: &str, next: &str) -> CommandInputParameter {
    let id = current.replace("-", "");
    let cwl_type = guess_type(next);
    let default_value = match cwl_type {
        CWLType::File => DefaultValue::File(File::from_location(&next.to_string())),
        CWLType::Directory => DefaultValue::Directory(Directory::from_location(&next.to_string())),
        _ => DefaultValue::Any(serde_yml::from_str(next).unwrap()),
    };

    CommandInputParameter::default()
        .with_binding(CommandLineBinding::default().with_prefix(&current.to_string()))
        .with_id(slugify!(&id).as_str())
        .with_type(cwl_type)
        .with_default_value(default_value)
}

pub fn guess_type(value: &str) -> CWLType {
    let path = Path::new(value);
    if path.exists() {
        if path.is_file() {
            return CWLType::File;
        }
        if path.is_dir() {
            return CWLType::Directory;
        }
    }
    //we do not have to check for files that do not exist yet, as CWLTool would run into a failure
    let yaml_value: Value = serde_yml::from_str(value).unwrap();
    match yaml_value {
        Value::Null => CWLType::Null,
        Value::Bool(_) => CWLType::Boolean,
        Value::Number(number) => {
            if number.is_f64() {
                CWLType::Float
            } else {
                CWLType::Int
            }
        }
        Value::String(_) => CWLType::String,
        _ => CWLType::String,
    }
}
