use fancy_regex::{Captures, Regex};
use std::{fs, path::Path};

pub(crate) fn preprocess_imports(contents: &str, path: impl AsRef<Path>) -> String{
    let import_regex = Regex::new(r#"(?P<indent>[\p{Z}-]*)\{*"*\$import"*: (?P<file>[\w\.\-_]*)\}*"#).unwrap();

    import_regex.replace_all(contents, |captures: &Captures| {
        let filename = captures.name("file").map_or("", |m| m.as_str());
        let indent = captures.name("indent").map_or("", |m| m.as_str());
        let indent_level: String = " ".repeat(indent.len());

        let path = path
            .as_ref()
            .parent()
            .map(|p| p.join(filename))
            .unwrap_or_else(|| Path::new(filename).to_path_buf());
        
        fs::read_to_string(&path).map(|c| {
            let mut lines = c.lines();
            let first = lines.next().unwrap_or_default();
            let mut result = format!("{indent}{first}");
            for line in lines {
                result.push('\n');
                result.push_str(&format!("{indent_level}{line}"));
            }
            result
        }).unwrap_or_default()
    }).to_string()
}

pub(crate) fn split_ranges(s:&str, delim: char) -> Vec<(usize, usize)>{
    let mut slices = Vec::new();
    let mut last_index = 0;

    for (idx, _) in s.match_indices(delim) {
        if last_index != idx {
            slices.push((last_index, idx));
        }
        last_index = idx;
    }
    
    if last_index < s.len() {
        slices.push((last_index, s.len()));
    }

    slices
}