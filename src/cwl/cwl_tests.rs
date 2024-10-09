use super::clt::Command;
use crate::cwl::parser::get_base_command;

//test private cwl api here
#[test]
pub fn test_get_base_command() {
    let commands = vec!["python script.py --arg1 hello", "echo 'Hello World!'", "Rscript lol.R", ""];
    let expected = vec![
        Command::Multiple(vec!["python".to_string(), "script.py".to_string()]),
        Command::Single("echo".to_string()),
        Command::Multiple(vec!["Rscript".to_string(), "lol.R".to_string()]),
        Command::Single("".to_string()),
    ];

    for i in 0..commands.len() {
        let args = shlex::split(commands[i]).unwrap();
        let args_convert: Vec<&str> = args.iter().map(|x| x.as_ref()).collect();

        let result = get_base_command(&args_convert);
        assert_eq!(result, expected[i])
    }
}
