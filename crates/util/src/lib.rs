use std::path::Path;

pub fn is_cwl_file(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("cwl"))
}
