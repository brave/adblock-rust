//! Test helper utilities for the adblock-rust project.
//! Used in tests and benchmarks.

#[cfg(not(target_arch = "wasm32"))]
pub fn rules_from_lists(
    lists: impl IntoIterator<Item = impl AsRef<str>>,
) -> impl Iterator<Item = String> {
    fn read_file_lines(filename: &str) -> impl Iterator<Item = String> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let reader = BufReader::new(File::open(filename).unwrap());
        reader.lines().map(|r| r.unwrap())
    }

    lists
        .into_iter()
        .flat_map(|filename| read_file_lines(filename.as_ref()))
}
