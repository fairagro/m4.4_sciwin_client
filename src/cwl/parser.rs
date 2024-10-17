use super::{
    clt::{Command, CommandInputParameter, CommandLineBinding, CommandLineTool, CommandOutputBinding, CommandOutputParameter, DefaultValue, InitialWorkDirRequirement, Requirement},
    types::{CWLType, Directory, File},
};
use crate::io::get_filename_without_extension;
use serde_yml::Value;
use slugify::slugify;
use std::path::Path;
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref STDOUT_FILE: Mutex<String> = Mutex::new(String::new());
}

fn modify_stdout_file(new_value: &str) {
    *STDOUT_FILE.lock().unwrap() = new_value.to_string();
}

pub fn get_stdout_file() -> String {
    STDOUT_FILE.lock().unwrap().clone()
}


//TODO complete list
static SCRIPT_EXECUTORS: &[&str] = &["python", "Rscript"];

pub fn parse_command_line(command: Vec<&str>) -> CommandLineTool {
    let base_command = get_base_command(&command);
    let inputs; 
    let tool; 
    if command.iter().any(|s| s.contains('>')) {
        println!("conatins >");
        inputs = get_inputs(match &base_command {
            Command::Single(_) => &command[1..],
            Command::Multiple(_) => &command[0..],
        });
        let base = match &base_command {
            Command::Single(cmd) => &cmd,
            Command::Multiple(vec) => &vec[0],
        };
        tool = CommandLineTool::default().with_base_command(Command::Single(base.clone())).with_inputs(inputs);
        match base_command {
            Command::Single(_) => tool,
            Command::Multiple(_) => tool,
        }
    }
    else{
  
        inputs = get_inputs(match &base_command {
            Command::Single(_) => &command[1..],
            Command::Multiple(ref vec) => &command[vec.len()..],
        });
        tool = CommandLineTool::default().with_base_command(base_command.clone()).with_inputs(inputs);
        match base_command {
            Command::Single(_) => tool,
            Command::Multiple(ref vec) => tool.with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(&vec[1]))]),
        }
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

pub fn get_outputs_stdout(files: Vec<String>) -> Vec<CommandOutputParameter> {
    files
        .iter()
        .map(|f| {
            CommandOutputParameter::default()
                .with_type(CWLType::File)
                .with_id((f.to_string()).as_str())
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
    else if contains_greater_than(command){
        base_command = command[0].to_string().split_whitespace().map(String::from).collect();
    }


    match base_command.len() {
        1 => Command::Single(command[0].to_string()),
        _ => Command::Multiple(base_command),
    }
}

pub(crate) fn get_inputs(args: &[&str]) -> Vec<CommandInputParameter> {
    let mut inputs = vec![];
    let mut i = 0;
    let mut args2: Vec<&str> = args.to_vec();
    //split at whitespace, requires input similar to "echo 'hello world' > file.txt" with spaces
    if !args.is_empty() && args2.iter().any(|arg| arg.contains(">")){
        args2 = args.iter().flat_map(|&arg| arg.split_whitespace()).collect();
        i = 1; 
    }
    while i < args2.len() {
        let arg = args2[i];
        let input: CommandInputParameter;
        if arg.starts_with('-') {
            if i + 1 < args2.len() && !args2[i + 1].starts_with('-') {
                //is not a flag, as next one is a value
                //added position to option, was required for one of my test commands "samtools view -b aln.sam > aln.bam"
                input = get_option(arg, args2[i + 1], i);
                i += 1
            } else {
                input = get_flag(arg)
            }
        }     
        else if args2[i].contains('>') {
            i += 1;
            modify_stdout_file(args2[i]);
            continue;
        }
        else {
            let mut s = String::from(arg);
            //in case there is a string with whitespace like in command "echo 'hello world'", improve
            if arg.contains("'"){
                i += 1; 
                while i < args2.len() {
                    let arg2 = String::new() + " " +args2[i];
                    s.push_str(&arg2);               
                    if arg2.contains("'"){
                        break;
                    }
                    i += 1;
                }
            }
                input = get_positional(&s, i);
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

fn get_option(current: &str, next: &str, index: usize) -> CommandInputParameter {
    let id = current.replace("-", "");
    let cwl_type = guess_type(next);
    let default_value = match cwl_type {
        CWLType::File => DefaultValue::File(File::from_location(&next.to_string())),
        CWLType::Directory => DefaultValue::Directory(Directory::from_location(&next.to_string())),
        _ => DefaultValue::Any(serde_yml::from_str(next).unwrap()),
    };

    CommandInputParameter::default()
        .with_binding(CommandLineBinding::default().with_prefix(&current.to_string()).with_position(index))
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

fn contains_greater_than(args: &[&str]) -> bool {
    args.iter().any(|arg| arg.contains('>'))
}
