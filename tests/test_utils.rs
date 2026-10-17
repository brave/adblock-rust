//! Convenience functions used for tests across different build targets. Import via `#[path = ]` if
//! needed outside of this directory.

#[cfg(not(target_arch = "wasm32"))]
pub fn rules_from_lists(list_files: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let mut contents = String::new();
    for file in list_files {
        contents.push_str(&std::fs::read_to_string(file.as_ref()).unwrap());
        contents.push('\n');
    }
    contents.shrink_to_fit();
    contents
}
