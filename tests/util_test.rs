use s4n::util::get_filename_without_extension;

#[test]
pub fn test_filename_without_extension() {
    let inputs = &["results.csv", "/some/relative/path.txt", "some/archive.tar.gz"];
    let outputs = &["results", "path", "archive"];

    for i in 0..inputs.len() {
        let result = get_filename_without_extension(inputs[i]).expect("operation failed");
        assert_eq!(result, outputs[i]);
    }
}
