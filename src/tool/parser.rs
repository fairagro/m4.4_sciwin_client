use super::{
    cli_tool::Tool,
    input::{Input, OptionType},
};
use slugify::slugify;

//TODO: complete list
static SCRIPT_EXECUTORS: &[&str] = &["python", "Rscript"];

pub fn parse_command_line(command: Vec<&str>) -> Tool {
    let base_command = get_base_command(&command);
    let args = &command[base_command.len()..];
    let inputs = get_inputs(args);
    Tool {
        base_command,
        inputs,
        outputs: vec![],
    }
}

fn get_base_command(command: &[&str]) -> Vec<String> {
    if command.is_empty() {
        return vec![];
    };

    let mut base_command = vec![command[0].to_string()];

    if SCRIPT_EXECUTORS.iter().any(|&exec| command[0].starts_with(exec)) {
        base_command.push(command[1].to_string());
    }

    base_command
}

fn get_inputs(args: &[&str]) -> Vec<Input> {
    let mut inputs = vec![];
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let mut input = Input::new();

        if arg.starts_with('-') {
            //not a positional
            let id = arg.replace("-", "");
            input.prefix = Some(arg.to_string());
            input.id = slugify!(&id);

            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                //is not a flag, as next one is a value
                input.value = Some(args[i + 1].to_string());
                i += 1
            } else {
                input.r#type = OptionType::Flag;
            }
        } else {
            input.id = slugify!(&arg);
            input.value = Some(arg.to_string());
            input.r#type = OptionType::Positional;
            input.index = Some(i);
        }
        inputs.push(input);
        i += 1;
    }
    inputs
}
