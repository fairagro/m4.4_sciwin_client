use crate::io::get_filename_without_extension;
use commonwl::{
    CWLType, SingularPlural,
    outputs::{CommandOutputBinding, CommandOutputParameter},
};
use std::path::Path;

pub(crate) fn get_outputs(files: &[String]) -> Vec<CommandOutputParameter> {
    files
        .iter()
        .map(|f| {
            let filename = get_filename_without_extension(f);
            let output_type = if Path::new(f).extension().is_some() {
                CWLType::File
            } else {
                CWLType::Directory
            };
            CommandOutputParameter::default()
                .with_type(output_type)
                .with_id(&filename)
                .with_binding(CommandOutputBinding {
                    glob: Some(SingularPlural::Singular(f.to_string())),
                    ..Default::default()
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_get_outputs() {
        let files = vec!["my-file.txt".to_string(), "archive.tar.gz".to_string()];
        let expected = vec![
            CommandOutputParameter::default()
                .with_type(CWLType::File)
                .with_id("my-file")
                .with_binding(CommandOutputBinding {
                    glob: Some(commonwl::SingularPlural::Singular("my-file.txt".to_string())),
                    ..Default::default()
                }),
            CommandOutputParameter::default()
                .with_type(CWLType::File)
                .with_id("archive")
                .with_binding(CommandOutputBinding {
                    glob: Some(commonwl::SingularPlural::Singular("archive.tar.gz".to_string())),
                    ..Default::default()
                }),
        ];

        let outputs = get_outputs(&files);
        assert_eq!(outputs, expected);
    }
}
