use colored::Colorize;
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

pub fn error(message: &str) {
    panic!("❌ {}: {}", "Error".red().bold(), message.red())
}

pub fn warn(message: &str) {
    eprintln!("⚠️  {}", message.yellow());
}

pub fn print_list(list: &Vec<String>) {
    for item in list {
        println!("\t- {}", item)
    }
}

pub fn highlight_cwl(yaml: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension("yaml").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["InspiredGitHub"]);

    for line in LinesWithEndings::from(yaml) {
        let ranges = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("{}", escaped)
    }
}
