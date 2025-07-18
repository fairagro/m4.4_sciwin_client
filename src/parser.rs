use crate::{io::get_filename_without_extension, split_vec_at};
use cwl::{
    clt::{Argument, Command, CommandLineTool},
    inputs::{CommandInputParameter, CommandLineBinding},
    outputs::{CommandOutputBinding, CommandOutputParameter},
    requirements::{InitialWorkDirRequirement, InlineJavascriptRequirement, Requirement},
    types::{guess_type, CWLType, DefaultValue, Directory, File},
};
use rand::{distr::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use slugify::slugify;
use std::{collections::HashSet, fs, path::Path};

//TODO complete list
static SCRIPT_EXECUTORS: &[&str] = &["python", "Rscript", "node"];

static BAD_WORDS: &[&str] = &["sql", "postgres", "mysql", "password"];

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
        let (cmd, piped) = split_vec_at(remainder, &"|");

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
        tool = tool.append_requirement(Requirement::ShellCommandRequirement);
    }
    tool
}

fn parse_input(input: &str) -> (&str, CWLType) {
    if let Some((hint, name)) = input.split_once(':') {
        if hint.len() == 1 {
            let type_ = match hint {
                "f" => CWLType::File,
                "d" => CWLType::Directory,
                "s" => CWLType::String,
                "r" => CWLType::Float,
                "i" => CWLType::Int,
                "l" => CWLType::Long,
                "b" => CWLType::Boolean,
                _ => CWLType::Any, //whatever
            };
            (name, type_)
        } else {
            (input, guess_type(input))
        }
    } else {
        (input, guess_type(input))
    }
}

pub fn add_fixed_inputs(tool: &mut CommandLineTool, inputs: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    for input in inputs {
        let (input, type_) = parse_input(input);

        //todo: add requiement for directory also or add new --mount param and remove block from here
        if matches!(type_, CWLType::File) {
            for item in &mut tool.requirements {
                if let Requirement::InitialWorkDirRequirement(req) = item {
                    req.add_files(inputs);
                    break;
                }
            }
        }

        let default = match type_ {
            CWLType::File => DefaultValue::File(File::from_location(input)),
            CWLType::Directory => DefaultValue::Directory(Directory::from_location(input)),
            _ => DefaultValue::Any(serde_yaml::from_str(input)?),
        };
        let id = slugify!(input, separator = "_");

        tool.inputs
            .push(CommandInputParameter::default().with_id(&id).with_type(type_).with_default_value(default));
    }

    Ok(())
}

pub fn get_outputs(files: &[String]) -> Vec<CommandOutputParameter> {
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
                    glob: Some(f.to_string()),
                    ..Default::default()
                })
        })
        .collect()
}

pub fn get_base_command(command: &[&str]) -> Command {
    if command.is_empty() {
        return Command::Single(String::new());
    }

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
                i += 1;
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
    let (current, cwl_type) = parse_input(current);
    let default_value = parse_default_value(current, &cwl_type);

    //check id for bad words
    let mut id = slugify!(&current, separator = "_");
    if BAD_WORDS.iter().any(|&word| current.to_lowercase().contains(word)) {
        let rnd: String = rand::rng().sample_iter(&Alphanumeric).take(2).map(char::from).collect();
        id = format!("secret_{rnd}");
    }

    CommandInputParameter::default()
        .with_id(&id)
        .with_type(cwl_type)
        .with_default_value(default_value)
        .with_binding(CommandLineBinding::default().with_position(index))
}

fn get_flag(current: &str) -> CommandInputParameter {
    let id = current.replace('-', "");
    CommandInputParameter::default()
        .with_binding(CommandLineBinding::default().with_prefix(current))
        .with_id(slugify!(&id, separator = "_").as_str())
        .with_type(CWLType::Boolean)
        .with_default_value(DefaultValue::Any(Value::Bool(true)))
}

fn get_option(current: &str, next: &str) -> CommandInputParameter {
    let id = current.replace('-', "");

    let (next, cwl_type) = parse_input(next);
    let default_value = parse_default_value(next, &cwl_type);

    CommandInputParameter::default()
        .with_binding(CommandLineBinding::default().with_prefix(current))
        .with_id(slugify!(&id, separator = "_").as_str())
        .with_type(cwl_type)
        .with_default_value(default_value)
}

fn parse_default_value(value: &str, cwl_type: &CWLType) -> DefaultValue {
    match cwl_type {
        CWLType::File => DefaultValue::File(File::from_location(value)),
        CWLType::Directory => DefaultValue::Directory(Directory::from_location(value)),
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
            position: Some((inputs.len() + i).try_into().unwrap_or_default()),
            value_from: Some(x.to_string()),
            ..Default::default()
        })
    });

    let mut args = vec![Argument::Binding(CommandLineBinding {
        position: Some(inputs.len().try_into().unwrap_or_default()),
        value_from: Some("|".to_string()),
        shell_quote: Some(false),
        ..Default::default()
    })];
    args.extend(piped_args);

    Some(args)
}

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
    use cwl_execution::{environment::RuntimeEnvironment, runner::run_command};
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
                .with_binding(CommandLineBinding::default().with_prefix("--argument1"))
                .with_default_value(DefaultValue::Any(Value::String("value1".to_string()))),
            CommandInputParameter::default()
                .with_id("flag")
                .with_type(CWLType::Boolean)
                .with_binding(CommandLineBinding::default().with_prefix("--flag"))
                .with_default_value(DefaultValue::Any(Value::Bool(true))),
            CommandInputParameter::default()
                .with_id("a")
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_prefix("-a"))
                .with_default_value(DefaultValue::Any(Value::String("value2".to_string()))),
            CommandInputParameter::default()
                .with_id("positional1")
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_position(5))
                .with_default_value(DefaultValue::Any(Value::String("positional1".to_string()))),
            CommandInputParameter::default()
                .with_id("v")
                .with_type(CWLType::Int)
                .with_binding(CommandLineBinding::default().with_prefix("-v"))
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
            .with_binding(CommandLineBinding::default().with_prefix("-v"))
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
                .with_binding(CommandLineBinding::default().with_prefix("--option1"))
                .with_default_value(DefaultValue::Any(Value::String("value1".to_string())))])
            .with_requirements(vec![Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("script.py"))])
    )]
    #[case("python script.py --option1 \"value with spaces\"", CommandLineTool::default()
            .with_base_command(Command::Multiple(vec!["python".to_string(), "script.py".to_string()]))
            .with_inputs(vec![CommandInputParameter::default()
                .with_id("option1")
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_prefix("--option1"))
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
                    .with_binding(CommandLineBinding::default().with_prefix("--option1"))
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
                    .with_binding(CommandLineBinding::default().with_prefix("--test"))
                    .with_default_value(DefaultValue::File(File::from_location("tests/test_data/input.txt")))])
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
        assert!(run_command(&cwl, &mut RuntimeEnvironment::default()).is_ok());
    }

    #[test]
    pub fn test_get_outputs() {
        let files = vec!["my-file.txt".to_string(), "archive.tar.gz".to_string()];
        let expected = vec![
            CommandOutputParameter::default()
                .with_type(CWLType::File)
                .with_id("my-file")
                .with_binding(CommandOutputBinding {
                    glob: Some("my-file.txt".to_string()),
                    ..Default::default()
                }),
            CommandOutputParameter::default()
                .with_type(CWLType::File)
                .with_id("archive")
                .with_binding(CommandOutputBinding {
                    glob: Some("archive.tar.gz".to_string()),
                    ..Default::default()
                }),
        ];

        let outputs = get_outputs(&files);
        assert_eq!(outputs, expected);
    }

    #[test]
    pub fn test_badwords() {
        let tool = parse_command("pg_dump postgres://postgres:password@localhost:5432/test \\> dump.sql");
        println!("{:?}", tool.inputs[0].id);
        assert!(BAD_WORDS.iter().any(|&word| tool.inputs.iter().any(|i| !i.id.contains(word))));
    }

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
