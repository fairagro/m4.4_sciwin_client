use std::{fs, path::Path};

pub(crate) fn split_ranges(s: &str, delim: char) -> Vec<(usize, usize)> {
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

pub fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> std::io::Result<()> {
    if let Some(parent) = to.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(from, to)?;
    Ok(())
}

pub fn create_file<P: AsRef<Path>>(file: P, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = file.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(file, contents)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_ranges() {
        let input = "Costs: $3 $5 $6";
        let expected = ["Costs: ", "$3 ", "$5 ", "$6"];
        let result = split_ranges(input, '$');

        for (i, &(start, end)) in result.iter().enumerate() {
            assert_eq!(&input[start..end], expected[i])
        }
    }
}
