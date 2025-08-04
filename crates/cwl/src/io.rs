use std::env;
use std::path::{Path, PathBuf};

pub(crate) fn normalize_path<P: AsRef<Path>>(input: P) -> std::io::Result<PathBuf> {
    let current_dir = env::current_dir()?;
    let full_path = current_dir.join(input);
    Ok(full_path.components().fold(PathBuf::new(), |mut acc, comp| {
        use std::path::Component::*;
        match comp {
            RootDir => acc.push(comp),
            Prefix(prefix) => acc.push(prefix.as_os_str()),
            CurDir => {}
            ParentDir => {
                acc.pop();
            }
            Normal(c) => acc.push(c),
        }
        acc
    }))
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_normalize_path(){
        let res = normalize_path("../../some/../path/or/what/../new_file").unwrap();

        let current = env::current_dir().unwrap();
        let expected = current.parent().unwrap().parent().unwrap().join("path/or/new_file");
        assert_eq!(res, expected);
    }

    #[test]
    fn test_normalize_path_with_dot() {
        let cwd = env::current_dir().unwrap();

        let input = "./foo/./bar";
        let expected = cwd.join("foo/bar");

        let result = normalize_path(input).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_normalize_path_with_trailing_parent() {
        let cwd = env::current_dir().unwrap();

        let input = "foo/bar/..";
        let expected = cwd.join("foo");

        let result = normalize_path(input).unwrap();
        assert_eq!(result, expected);
    }
}