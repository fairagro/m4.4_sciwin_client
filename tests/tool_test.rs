use s4n;
use s4n::tool::parser::parse_command_line;

pub static TEST_CASES: &[&str] = &[
    "python script.py",
    "python script.py --option1 value1",
    "python script.py --option1 \"value with spaces\"",
    "python script.py positional1 --option1 value1",
    "python script.py --flag1",
    "python script.py -o value1",
    "Rscript script.R",
];

#[test]
fn test_command_line_parser() {
    for case in TEST_CASES {
        let args = shlex::split(*case).expect("Parsing test case failed");
        parse_command_line(args);
    }
}
