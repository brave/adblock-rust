//! Convenience functions used for tests across different build targets. Import via `#[path = ]` if
//! needed outside of this directory.

#[cfg(not(target_arch = "wasm32"))]
pub fn rules_from_lists(lists: impl IntoIterator<Item=impl AsRef<str>>) -> impl Iterator<Item=String> {
    fn read_file_lines(filename: &str) -> impl Iterator<Item=String> {
        use std::io::{BufRead, BufReader};
        use std::fs::File;

        let reader = BufReader::new(File::open(filename).unwrap());
        reader.lines().map(|r| r.unwrap())
    }

    lists
        .into_iter()
        .map(|filename| read_file_lines(filename.as_ref()))
        .flatten()
}
