use std::{fs, io::{self, Write}, path::Path};

pub fn get_filename_without_extension(relative_path: &str) -> Option<String> {
    let path = Path::new(relative_path);

    path.file_name().and_then(|name| {
        name.to_str()
            .map(|s| s.split('.').next().unwrap_or(s).to_string())
    })
}

pub fn create_and_write_file(filename: &str, contents: &str) -> Result<(), io::Error> {
    let mut file = fs::File::create(filename)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}