use std::path::Path;

pub fn get_filename_without_extension(relative_path: &str) -> Option<String> {
    let path = Path::new(relative_path);

    path.file_name().and_then(|name| {
        name.to_str()
            .map(|s| s.split('.').nth(0).unwrap_or(s).to_string())
    })
}
