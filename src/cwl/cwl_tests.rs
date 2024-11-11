use super::{clt::Command, parser::get_inputs, types::CWLType, types::DefaultValue};
use crate::cwl::{
    inputs::{CommandInputParameter, CommandLineBinding},
    parser::get_base_command,
};
use serde_yml::Value;

//test private cwl api here
#[test]
pub fn test_get_base_command() {
    let commands = ["python script.py --arg1 hello", "echo 'Hello World!'", "Rscript lol.R", ""];
    let expected = [
        Command::Multiple(vec!["python".to_string(), "script.py".to_string()]),
        Command::Single("echo".to_string()),
        Command::Multiple(vec!["Rscript".to_string(), "lol.R".to_string()]),
        Command::Single("".to_string()),
    ];

    for i in 0..commands.len() {
        let args = shlex::split(commands[i]).unwrap();
        let args_slice: Vec<&str> = args.iter().map(|x| x.as_ref()).collect();

        let result = get_base_command(&args_slice);
        assert_eq!(result, expected[i])
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
            .with_default_value(DefaultValue::Any(serde_yml::from_str("1").unwrap())),
    ];

    let inputs_vec = shlex::split(inputs).unwrap();
    let inputs_slice: Vec<&str> = inputs_vec.iter().map(|x| x.as_ref()).collect();

    let result = get_inputs(&inputs_slice);

    assert_eq!(result, expected);
}
