use s4n::tool::{
    cli_tool::Tool,
    input::{Input, OptionType},
};

#[test]
pub fn test_cwl_conversion() {
    let tool = Tool {
        base_command: vec!["python".to_string(), "script.py".to_string()],
        inputs: vec![Input::new_with(
            "option1",
            Some("input.rdf"),
            OptionType::Option,
            Some("--option1"),
            None,
        )],
        outputs: vec!["results.csv".to_string()],
    };
    let cwl = tool.to_cwl();
    let yaml = serde_yml::to_string(&cwl);
    println!("{:?}", cwl);
    println!("{:?}", yaml);
    assert!(yaml.is_ok());
}