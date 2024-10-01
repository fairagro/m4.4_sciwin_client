use super::{
    clt::{
        Command, CommandInputParameter, CommandLineBinding, CommandLineTool, DefaultValue,
        InitialWorkDirRequirement, Requirement,
    },
    types::File,
};
use slugify::slugify;

//TODO complete list
static SCRIPT_EXECUTORS: &[&str] = &["python", "Rscript"];

pub fn parse_command_line(command: Vec<&str>) -> CommandLineTool {
    let base_command = get_base_command(&command);
    let inputs = get_inputs(match &base_command {
        Command::Single(_) => &command[1..],
        Command::Multiple(ref vec) => &command[vec.len()..],
    });

    let tool = CommandLineTool::default()
        .with_base_command(base_command.clone())
        .with_inputs(inputs);

    match base_command {
        Command::Single(_) => tool,
        Command::Multiple(ref vec) => {
            tool.with_requirements(vec![Requirement::InitialWorkDirRequirement(
                InitialWorkDirRequirement::from_file(&vec[1]),
            )])
        }
    }
}

fn get_base_command(command: &[&str]) -> Command {
    println!("{:?}", command);
    if command.is_empty() {
        return Command::Single(String::from(""));
    };

    let mut base_command = vec![command[0].to_string()];

    if SCRIPT_EXECUTORS
        .iter()
        .any(|&exec| command[0].starts_with(exec))
    {
        base_command.push(command[1].to_string());
    }

    match base_command.len() {
        1 => Command::Single(command[0].to_string()),
        _ => Command::Multiple(base_command)
    }
}

fn get_inputs(args: &[&str]) -> Vec<CommandInputParameter> {
    let mut inputs = vec![];
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let mut input = CommandInputParameter::default();
        //TODO add type to input
        if arg.starts_with('-') {
            //not a positional
            let id = arg.replace("-", "");
            input = input
                .with_binding(CommandLineBinding::default().with_prefix(&arg.to_string()))
                .with_id(slugify!(&id).as_str());

            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                //is not a flag, as next one is a value
                //TODO: support other stuff than file
                input = input.with_default_value(DefaultValue::File(File::from_location(
                    &args[i + 1].to_string(),
                )));
                i += 1
            }
        } else {
            input = input
                .with_id(slugify!(&arg).as_str())
                .with_default_value(DefaultValue::File(File::from_location(
                    &args[i + 1].to_string(),
                )))
                .with_binding(CommandLineBinding::default().with_position(i));
        }
        inputs.push(input);
        i += 1;
    }
    inputs
}
