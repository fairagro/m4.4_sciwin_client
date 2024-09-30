use s4n;
use s4n::tool::{input::Input, input::OptionType, parser::parse_command_line, tool::Tool};

pub fn test_cases() -> Vec<(String, Tool)> {
    return vec![
        (
            "python script.py".to_string(),
            Tool {
                base_command: vec!["python".to_string(), "script.py".to_string()],
                inputs: vec![],
            },
        ),
        (
            "python script.py --option1 value1".to_string(),
            Tool {
                base_command: vec!["python".to_string(), "script.py".to_string()],
                inputs: vec![Input::input("option1", Some("value1"), OptionType::Option, Some("--option1"), None)],
            },
        ),
        (
            "python script.py --option1 \"value with spaces\"".to_string(),
            Tool {
                base_command: vec!["python".to_string(), "script.py".to_string()],
                inputs: vec![Input::input("option1", Some("value with spaces"), OptionType::Option, Some("--option1"), None)],
            },
        ),
        (
            "python script.py positional1 --option1 value1".to_string(),
            Tool {
                base_command: vec!["python".to_string(), "script.py".to_string()],
                inputs: vec![
                    Input::input("positional1", Some("positional1"), OptionType::Positional, None, Some(0)),
                    Input::input("option1", Some("value1"), OptionType::Option, Some("--option1"), None),
                ],
            },
        ),
        (
            "python script.py --flag1".to_string(),
            Tool {
                base_command: vec!["python".to_string(), "script.py".to_string()],
                inputs: vec![Input::input("flag1", None, OptionType::Flag, Some("--flag1"), None)],
            },
        ),
        (
            "python script.py -o value1".to_string(),
            Tool {
                base_command: vec!["python".to_string(), "script.py".to_string()],
                inputs: vec![Input::input("o", Some("value1"), OptionType::Option, Some("-o"), None)],
            },
        ),
        (
            "Rscript script.R".to_string(),
            Tool {
                base_command: vec!["Rscript".to_string(), "script.R".to_string()],
                inputs: vec![],
            },
        ),
    ]
}

#[test]
fn test_command_line_parser() {
    for (input, expected) in test_cases() {
        let args = shlex::split(input.as_str()).expect("Parsing test case failed");
        let result = parse_command_line(args.iter().map(|x| x.as_ref()).collect());
        assert_eq!(result, expected);
    }
}
