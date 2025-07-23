use crate::split_vec_at;
use cwl::{
    inputs::{CommandInputParameter, CommandLineBinding},
    requirements::{InitialWorkDirRequirement, Requirement},
    Argument, Command, CommandLineTool,
};
use serde::{Deserialize, Serialize};
use std::fs;

mod inputs;
mod outputs;
mod postprocess;
pub use inputs::*;
pub use outputs::*;
pub use postprocess::post_process_cwl;

//TODO complete list
static SCRIPT_EXECUTORS: &[&str] = &["python", "Rscript", "node"];

pub(crate) static BAD_WORDS: &[&str] = &["sql", "postgres", "mysql", "password"];

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

#[cfg(test)]
mod tests {
    use super::*;
    use cwl::{CWLType, DefaultValue, File};
    use cwl_execution::{environment::RuntimeEnvironment, runner::run_command};
    use rstest::rstest;
    use serde_yaml::Value;

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
    pub fn test_badwords() {
        let tool = parse_command("pg_dump postgres://postgres:password@localhost:5432/test \\> dump.sql");
        println!("{:?}", tool.inputs[0].id);
        assert!(BAD_WORDS.iter().any(|&word| tool.inputs.iter().any(|i| !i.id.contains(word))));
    }
}
