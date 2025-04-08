use crate::{io::get_filename_without_extension, split_vec_at};
use cwl::{
    clt::{Argument, Command, CommandLineTool},
    inputs::{CommandInputParameter, CommandLineBinding},
    outputs::{CommandOutputBinding, CommandOutputParameter},
    requirements::{InitialWorkDirRequirement, Requirement},
    types::{guess_type, CWLType, DefaultValue, Directory, File},
};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use slugify::slugify;
use std::{fs, path::Path};

//TODO complete list
static SCRIPT_EXECUTORS: &[&str] = &["python", "Rscript"];

#[derive(Serialize, Deserialize, Debug)]
struct FileEntry {
    class: String,
    path: String,
}

pub fn parse_command_line(commands: &[&str]) -> CommandLineTool {
    let base_command = get_base_command(commands);

    let remainder = match &base_command {
        Command::Single(_) => &commands[1..],
        Command::Multiple(ref vec) => &commands[vec.len()..],
    };

    let mut tool = CommandLineTool::default().with_base_command(base_command.clone());

    if !remainder.is_empty() {
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
        Command::Single(cmd) => {
            //if command is an existing file, add to requirements
            if fs::exists(&cmd).unwrap_or_default() {
                return tool.with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file(&cmd))]);
            }
            tool
        }
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

pub fn add_fixed_inputs(tool: &mut CommandLineTool, inputs: Vec<&str>) {
    if let Some(req) = &mut tool.requirements {
        for item in req.iter_mut() {
            if let Requirement::InitialWorkDirRequirement(req) = item {
                req.add_files(&inputs);
                break;
            }
        }
    } else {
        tool.requirements = Some(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_files(
            &inputs,
        ))])
    }

    let params = inputs
        .iter()
        .map(|i| {
            CommandInputParameter::default()
                .with_id(&slugify!(i, separator = "_"))
                .with_type(guess_type(i))
        })
        .collect::<Vec<_>>();
    tool.inputs.extend(params);
}

pub fn get_outputs(files: Vec<String>) -> Vec<CommandOutputParameter> {
    files
        .iter()
        .map(|f| {
            let filename = get_filename_without_extension(f);
            let output_type = if Path::new(f).extension().is_some() {
                CWLType::File
            } else {
                CWLType::Directory
            };
            CommandOutputParameter::default()
                .with_type(output_type)
                .with_id(&filename)
                .with_binding(CommandOutputBinding {
                    glob: f.to_string(),
                    ..Default::default()
                })
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
    let inputs = tool.inputs.clone();
    for input in &inputs {
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
        if let Some(binding) = &mut output.output_binding {
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

#[cfg(test)]
mod tests {
    use super::*;
    use cwl_execution::runner::run_command;
    use rstest::rstest;
    use serde_yaml::Number;

    fn parse_command(command: &str) -> CommandLineTool {
        let cmd = shlex::split(command).unwrap();
        parse_command_line(&cmd.iter().map(|s| s.as_str()).collect::<Vec<_>>())
    }

    #[rstest]
    #[case("python script.py --arg1 hello", Command::Multiple(vec!["python".to_string(), "script.py".to_string()]))]
    #[case("echo 'Hello World!'", Command::Single("echo".to_string()))]
    #[case("Rscript lol.R", Command::Multiple(vec!["Rscript".to_string(), "lol.R".to_string()]))]
    #[case("", Command::Single(String::new()))]
    pub fn test_get_base_command(#[case] command: &str, #[case] expected: Command) {
        let args = shlex::split(command).unwrap();
        let args_slice: Vec<&str> = args.iter().map(AsRef::as_ref).collect();

        let result = get_base_command(&args_slice);
        assert_eq!(result, expected);
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

    #[rstest]
    #[case("python script.py", CommandLineTool::default()
            .with_base_command(Command::Multiple(vec!["python".to_string(), "script.py".to_string()]))
            .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("script.py"))])
        )]
    #[case("Rscript script.R", CommandLineTool::default()
            .with_base_command(Command::Multiple(vec!["Rscript".to_string(), "script.R".to_string()]))
            .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("script.R"))])
    )]
    #[case("python script.py --option1 value1", CommandLineTool::default()
            .with_base_command(Command::Multiple(vec!["python".to_string(), "script.py".to_string()]))
            .with_inputs(vec![CommandInputParameter::default()
                .with_id("option1")
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_prefix(&"--option1".to_string()))
                .with_default_value(DefaultValue::Any(Value::String("value1".to_string())))])
            .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("script.py"))])
    )]
    #[case("python script.py --option1 \"value with spaces\"", CommandLineTool::default()
            .with_base_command(Command::Multiple(vec!["python".to_string(), "script.py".to_string()]))
            .with_inputs(vec![CommandInputParameter::default()
                .with_id("option1")
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_prefix(&"--option1".to_string()))
                .with_default_value(DefaultValue::Any(Value::String("value with spaces".to_string())))])
            .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("script.py"))])
    )]
    #[case("python script.py positional1 --option1 value1",  CommandLineTool::default()
            .with_base_command(Command::Multiple(vec!["python".to_string(), "script.py".to_string()]))
            .with_inputs(vec![
                CommandInputParameter::default()
                    .with_id("positional1")
                    .with_default_value(DefaultValue::Any(Value::String("positional1".to_string())))
                    .with_type(CWLType::String)
                    .with_binding(CommandLineBinding::default().with_position(0)),
                CommandInputParameter::default()
                    .with_id("option1")
                    .with_type(CWLType::String)
                    .with_binding(CommandLineBinding::default().with_prefix(&"--option1".to_string()))
                    .with_default_value(DefaultValue::Any(Value::String("value1".to_string())))
            ])
            .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("script.py"))])
    )]
    #[case("python tests/test_data/echo.py --test tests/test_data/input.txt", CommandLineTool::default()
            .with_base_command(Command::Multiple(vec!["python".to_string(), "tests/test_data/echo.py".to_string()]))
            .with_inputs(vec![
                CommandInputParameter::default()
                    .with_id("test")
                    .with_type(CWLType::File)
                    .with_binding(CommandLineBinding::default().with_prefix(&"--test".to_string()))
                    .with_default_value(DefaultValue::File(File::from_location(&"tests/test_data/input.txt".to_string())))])
            .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("tests/test_data/echo.py"))])
    )]
    pub fn test_parse_command_line(#[case] input: &str, #[case] expected: CommandLineTool) {
        let result = parse_command(input);
        assert_eq!(result, expected);
    }

    #[test]
    pub fn test_parse_redirect() {
        let tool = parse_command("cat tests/test_data/input.txt \\> output.txt");
        assert!(tool.stdout == Some("output.txt".to_string()));
    }

    #[test]
    pub fn test_parse_redirect_stderr() {
        let tool = parse_command("cat tests/test_data/inputtxt 2\\> err.txt");
        assert!(tool.stderr == Some("err.txt".to_string()));
    }

    #[test]
    pub fn test_parse_pipe_op() {
        let tool = parse_command("df \\| grep --line-buffered tmpfs \\> df.log");

        assert!(tool.arguments.is_some());
        assert!(tool.has_shell_command_requirement());

        if let Some(args) = tool.arguments {
            if let Argument::Binding(pipe) = &args[0] {
                assert!(pipe.value_from == Some("|".to_string()));
            } else {
                panic!();
            }
        }

        assert!(tool.stdout.is_none()); //as it is in args!
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    pub fn test_cwl_execute_command_single() {
        let cwl = parse_command("ls -la .");
        assert!(run_command(&cwl, &Default::default()).is_ok());
    }

    #[test]
    pub fn test_get_outputs() {
        let files = vec!["my-file.txt".to_string(), "archive.tar.gz".to_string()];
        let expected = vec![
            CommandOutputParameter::default()
                .with_type(CWLType::File)
                .with_id("my-file")
                .with_binding(CommandOutputBinding {
                    glob: "my-file.txt".to_string(),
                    ..Default::default()
                }),
            CommandOutputParameter::default()
                .with_type(CWLType::File)
                .with_id("archive")
                .with_binding(CommandOutputBinding {
                    glob: "archive.tar.gz".to_string(),
                    ..Default::default()
                }),
        ];

        let outputs = get_outputs(files);
        assert_eq!(outputs, expected);
    }
}
