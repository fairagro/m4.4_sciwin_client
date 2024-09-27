use super::{input, tool::Tool};
use input::{Input, OptionType};

//TODO: complete list
static SCRIPT_EXECUTORS: &[&str] = &["python", "Rscript"];

pub fn parse_command_line(command: Vec<String>) -> Tool {
    let base_command = get_base_command(&command);
    let args = command[base_command.len()..].to_vec();
    let inputs = get_inputs(args);
    return Tool {
        base_command,
        inputs,
    };
}

fn get_base_command(command: &Vec<String>) -> Vec<String> {
    if command.is_empty() {
        return vec![];
    };

    let mut base_command = vec![command[0].clone()];

    if SCRIPT_EXECUTORS.contains(&command[0].as_str()) {
        base_command.push(command[1].clone());
    }

    return base_command;
}

fn get_inputs(args: Vec<String>) -> Vec<Input> {
    let mut inputs = vec![];
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let mut input = Input::new();

        if arg.starts_with('-') {
            //not a positional
            let id = arg.replace("-", "");
            input.prefix = Some(arg.clone());
            input.id = Some(id);

            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                //is not a flag, as next one is a value
                input.value = Some(args[i + 1].clone());
                i += 1
            } else {
                input.r#type = OptionType::Flag;
            }
        } else {
            input.id = Some(arg.clone());
            input.value = Some(arg.clone());
            input.r#type = OptionType::Positional;
            input.index = Some(i);
        }
        inputs.push(input);
        i += 1;
    }
    return inputs;
}
