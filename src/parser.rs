use crate::{cwl::resolve_filename, io::get_filename_without_extension, split_vec_at};
use cwl::{
    clt::{Argument, Command, CommandLineTool},
    inputs::{CommandInputParameter, CommandLineBinding},
    outputs::{self, CommandOutputBinding, CommandOutputParameter},
    requirements::{InitialWorkDirRequirement, Requirement},
    types::{CWLType, DefaultValue, Directory, File},
};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use slugify::slugify;
use std::error::Error;
use std::{
    collections::HashMap,
    env, fs,
    io::Write,
    path::{Path, MAIN_SEPARATOR},
};

//TODO complete list
static SCRIPT_EXECUTORS: &[&str] = &["python", "Rscript"];

#[derive(Serialize, Deserialize, Debug)]
struct FileEntry {
    class: String,
    path: String,
}

pub fn create_input_yml(inputs: &[&str], command: &str) -> Result<String, Box<dyn Error>> {
    let mut yaml_data: HashMap<String, FileEntry> = HashMap::new();

    // Generate a base filename
    let base_name = get_filename_without_extension(command).unwrap_or_else(|| "command".to_string());
    let yaml_name = resolve_filename(&base_name);
    let yaml_filename = match yaml_name.rsplit_once('.') {
        Some((base, _)) => format!("{}_inputs.yml", base),
        None => format!("{}_inputs.yml", yaml_name),
    };

    // Construct full path in the current directory
    let yaml_path = env::current_dir()?.join(&yaml_filename);

    // Populate YAML data
    for input in inputs {
        let key = input.replace('.', "_");
        let file_type = guess_type(input);
        let type_str = match file_type {
            CWLType::File => "File",
            CWLType::Directory => "Directory",
            CWLType::Float => "Float",
            CWLType::Int => "Int",
            CWLType::Boolean => "Boolean",
            CWLType::Null => "Null",
            _ => "String",
        };

        yaml_data.insert(
            key,
            FileEntry {
                class: type_str.to_string(),
                path: format!("..{}..{}{}", MAIN_SEPARATOR, MAIN_SEPARATOR, input),
            },
        );
    }

    // Convert to YAML
    let yaml_string = serde_yaml::to_string(&yaml_data)?;

    // Write to file safely
    let mut file = fs::File::create(&yaml_path)?;
    file.write_all(yaml_string.as_bytes())?;

    Ok(yaml_path.display().to_string())
}

pub fn parse_command_line(commands: Vec<&str>, inputs: Option<Vec<&str>>) -> CommandLineTool {
    let base_command = get_base_command(&commands);

    let remainder = match &base_command {
        Command::Single(_) => &commands[1..],
        Command::Multiple(ref vec) => &commands[vec.len()..],
    };

    let mut tool = CommandLineTool::default().with_base_command(base_command.clone());

    if let Some(inputs) = &inputs {
        let test_in = InitialWorkDirRequirement::from_files(inputs, commands[1]);
        tool = tool.with_requirements(vec![Requirement::InitialWorkDirRequirement(test_in)]);
        let mut updated_inputs = inputs.clone();
        if !updated_inputs.contains(&commands[1]) {
            updated_inputs.push(commands[1]);
        }
        let initial_dir_req = InitialWorkDirRequirement::from_files(&updated_inputs, commands[1]);
        let updated_commands = update_commands_with_entrynames(commands.clone(), &initial_dir_req);
        let base_command_updated = get_base_command(&updated_commands.iter().map(String::as_str).collect::<Vec<_>>());
        let inputs_cmd = get_inputs(&commands[2..]);
        let mut input_parameters: Vec<CommandInputParameter> = inputs
            .iter()
            .map(|current| {
                CommandInputParameter::default()
                    .with_id(&current.replace(".", "_"))
                    .with_type(guess_type(current))
            })
            .collect();
        input_parameters.extend(inputs_cmd);
        tool = tool.with_inputs(input_parameters);

        tool = tool.with_base_command(base_command_updated);
        return tool;
    } else if !remainder.is_empty() {
        let (cmd, piped) = split_vec_at(remainder, "|");

        let stdout_pos = cmd.iter().position(|i| *i == ">").unwrap_or(cmd.len());
        let stderr_pos = cmd.iter().position(|i| *i == "2>").unwrap_or(cmd.len());
        let first_redir_pos = usize::min(stdout_pos, stderr_pos);

        let stdout = handle_redirection(&cmd[stdout_pos..]);
        let stderr = handle_redirection(&cmd[stderr_pos..]);

        let inputs = get_inputs(&cmd[..first_redir_pos]);

        let args = collect_arguments(&piped, &inputs);

        tool = tool.with_inputs(inputs).with_stdout(stdout).with_stderr(stderr).with_arguments(args);
    }

    tool = match base_command {
        Command::Single(_) => tool,
        Command::Multiple(ref vec) => tool.with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(
            &vec[1],
        ))]),
    };

    if tool.arguments.is_some() {
        if let Some(requirements) = &mut tool.requirements {
            requirements.push(Requirement::ShellCommandRequirement);
        } else {
            tool = tool.with_requirements(vec![Requirement::ShellCommandRequirement])
        }
    }
    tool
}

fn update_commands_with_entrynames(commands: Vec<&str>, initial_work_dir: &InitialWorkDirRequirement) -> Vec<String> {
    let entry_map: HashMap<&str, &str> = initial_work_dir
        .listing
        .iter()
        .map(|listing| (listing.entryname.as_str(), listing.entryname.as_str()))
        .collect();

    commands
        .into_iter()
        .map(|command| {
            let mut updated_command = command.to_string();
            for (entry, entryname) in &entry_map {
                if command.contains(entry) {
                    updated_command = updated_command.replace(command, entryname);
                }
            }
            updated_command
        })
        .collect()
}

pub fn get_outputs(files: Vec<String>) -> Vec<CommandOutputParameter> {
    files
        .iter()
        .enumerate()
        .map(|(index, f)| {
            let filename = get_filename_without_extension(f).unwrap_or_else(|| f.to_string());
            let id = if filename.starts_with('$') {
                format!("output{}", index)
            } else {
                filename
            };
            let output_type = if Path::new(f).extension().is_some() && !f.contains("$(runtime.outdir)") {
                CWLType::File
            } else {
                CWLType::Directory
            };

            CommandOutputParameter::default()
                .with_type(output_type)
                .with_id(&id)
                .with_binding(CommandOutputBinding { glob: f.clone() })
        })
        .collect()
}

pub fn get_base_command(command: &[&str]) -> Command {
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

pub fn get_inputs(args: &[&str]) -> Vec<CommandInputParameter> {
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
                input = get_flag(arg);
            }
        } else {
            input = get_positional(arg, i.try_into().unwrap());
        }
        inputs.push(input);
        i += 1;
    }
    inputs
}

fn get_positional(current: &str, index: isize) -> CommandInputParameter {
    let cwl_type = guess_type(current);
    let default_value = parse_default_value(current, &cwl_type);

    CommandInputParameter::default()
        .with_id(slugify!(&current, separator = "_").as_str())
        .with_type(guess_type(current))
        .with_default_value(default_value)
        .with_binding(CommandLineBinding::default().with_position(index))
}

fn get_flag(current: &str) -> CommandInputParameter {
    let id = current.replace('-', "");
    CommandInputParameter::default()
        .with_binding(CommandLineBinding::default().with_prefix(&current.to_string()))
        .with_id(slugify!(&id, separator = "_").as_str())
        .with_type(CWLType::Boolean)
        .with_default_value(DefaultValue::Any(Value::Bool(true)))
}

fn get_option(current: &str, next: &str) -> CommandInputParameter {
    let id = current.replace('-', "");
    let cwl_type = guess_type(next);
    let default_value = parse_default_value(next, &cwl_type);

    CommandInputParameter::default()
        .with_binding(CommandLineBinding::default().with_prefix(&current.to_string()))
        .with_id(slugify!(&id, separator = "_").as_str())
        .with_type(cwl_type)
        .with_default_value(default_value)
}

fn parse_default_value(value: &str, cwl_type: &CWLType) -> DefaultValue {
    match cwl_type {
        CWLType::File => DefaultValue::File(File::from_location(&value.to_string())),
        CWLType::Directory => DefaultValue::Directory(Directory::from_location(&value.to_string())),
        CWLType::String => DefaultValue::Any(Value::String(value.to_string())),
        _ => DefaultValue::Any(serde_yaml::from_str(value).unwrap()),
    }
}

fn handle_redirection(remaining_args: &[&str]) -> Option<String> {
    if remaining_args.is_empty() {
        return None;
    }
    //hopefully? most cases are only `some_command > some_file.out`
    //remdirect comes at pos 0, discard that
    let out_file = remaining_args[1];
    Some(out_file.to_string())
}

fn collect_arguments(piped: &[&str], inputs: &[CommandInputParameter]) -> Option<Vec<Argument>> {
    if piped.is_empty() {
        return None;
    }

    let piped_args = piped.iter().enumerate().map(|(i, &x)| {
        Argument::Binding(CommandLineBinding {
            prefix: None,
            position: Some((inputs.len() + i).try_into().unwrap_or_default()),
            value_from: Some(x.to_string()),
            shell_quote: None,
        })
    });

    let mut args = vec![Argument::Binding(CommandLineBinding {
        prefix: None,
        position: Some(inputs.len().try_into().unwrap_or_default()),
        value_from: Some("|".to_string()),
        shell_quote: Some(false),
    })];
    args.extend(piped_args);

    Some(args)
}

pub fn post_process_cwl(tool: &mut CommandLineTool) {
    fn process_input(input: &CommandInputParameter) -> String {
        if input.type_ == CWLType::File || input.type_ == CWLType::Directory {
            format!("$(inputs.{}.path)", input.id)
        } else {
            format!("$(inputs.{})", input.id)
        }
    }

    let mut processed_once = false;
    for input in &tool.inputs {
        if let Some(default) = &input.default {
            for output in tool.outputs.iter_mut() {
                if let Some(binding) = &mut output.output_binding {                    
                    if binding.glob == default.as_value_string() {
                        binding.glob = process_input(input);
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

    for output in tool.outputs.iter_mut() {
        if let Some(binding) = &mut output.output_binding{
            if binding.glob == "." {
                output.id = "output_directory".to_string();
                binding.glob = "$(runtime.outdir)".into();
            }
        }
    }

    if processed_once {
        if let Some(requirements) = &mut tool.requirements {
            requirements.push(Requirement::InlineJavascriptRequirement);
        } else {
            tool.requirements = Some(vec![Requirement::InlineJavascriptRequirement]);
        }
    }
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
    let yaml_value: Value = serde_yaml::from_str(value).unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Number;
    //test private cwl api here
    #[test]
    pub fn test_get_base_command() {
        let commands = ["python script.py --arg1 hello", "echo 'Hello World!'", "Rscript lol.R", ""];
        let expected = [
            Command::Multiple(vec!["python".to_string(), "script.py".to_string()]),
            Command::Single("echo".to_string()),
            Command::Multiple(vec!["Rscript".to_string(), "lol.R".to_string()]),
            Command::Single(String::new()),
        ];

        for i in 0..commands.len() {
            let args = shlex::split(commands[i]).unwrap();
            let args_slice: Vec<&str> = args.iter().map(AsRef::as_ref).collect();

            let result = get_base_command(&args_slice);
            assert_eq!(result, expected[i]);
        }
    }

    #[test]
    pub fn test_get_inputs() {
        let inputs = "--argument1 value1 --flag -a value2 positional1 -v 1";
        let expected = vec![
            CommandInputParameter::default()
                .with_id("argument1")
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_prefix(&"--argument1".to_string()))
                .with_default_value(DefaultValue::Any(Value::String("value1".to_string()))),
            CommandInputParameter::default()
                .with_id("flag")
                .with_type(CWLType::Boolean)
                .with_binding(CommandLineBinding::default().with_prefix(&"--flag".to_string()))
                .with_default_value(DefaultValue::Any(Value::Bool(true))),
            CommandInputParameter::default()
                .with_id("a")
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_prefix(&"-a".to_string()))
                .with_default_value(DefaultValue::Any(Value::String("value2".to_string()))),
            CommandInputParameter::default()
                .with_id("positional1")
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_position(5))
                .with_default_value(DefaultValue::Any(Value::String("positional1".to_string()))),
            CommandInputParameter::default()
                .with_id("v")
                .with_type(CWLType::Int)
                .with_binding(CommandLineBinding::default().with_prefix(&"-v".to_string()))
                .with_default_value(DefaultValue::Any(serde_yaml::from_str("1").unwrap())),
        ];

        let inputs_vec = shlex::split(inputs).unwrap();
        let inputs_slice: Vec<&str> = inputs_vec.iter().map(AsRef::as_ref).collect();

        let result = get_inputs(&inputs_slice);

        assert_eq!(result, expected);
    }

    #[test]
    pub fn test_get_default_value_number() {
        let commandline_args = "-v 42";
        let expected = CommandInputParameter::default()
            .with_id("v")
            .with_type(CWLType::Int)
            .with_binding(CommandLineBinding::default().with_prefix(&"-v".to_string()))
            .with_default_value(DefaultValue::Any(Value::Number(Number::from(42))));

        let args = shlex::split(commandline_args).unwrap();
        let result = get_inputs(&args.iter().map(AsRef::as_ref).collect::<Vec<&str>>());

        assert_eq!(result[0], expected);
    }

    #[test]
    pub fn test_get_default_value_json_str() {
        let arg = "{\"message\": \"Hello World\"}";
        let expected = CommandInputParameter::default()
            .with_id("message_hello_world")
            .with_type(CWLType::String)
            .with_binding(CommandLineBinding::default().with_position(0))
            .with_default_value(DefaultValue::Any(Value::String(arg.to_string())));
        let result = get_inputs(&[arg]);
        assert_eq!(result[0], expected);
    }
}
