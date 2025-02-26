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