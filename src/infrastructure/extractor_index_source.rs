//! Replaceable dataset; never silently hide a corrupt external index.
use std::path::Path;

pub(super) fn load_from(root: &Path, embedded: &str) -> Result<String, String> {
    let path = root.join("resources").join("extractor-index.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(embedded.to_owned()),
        Err(error) => Err(format!(
            "Could not read extractor index {}: {error}",
            path.display()
        )),
    }
}
