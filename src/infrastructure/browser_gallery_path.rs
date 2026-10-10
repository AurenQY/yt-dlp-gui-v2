//! Preserve gallery-dl's formatted path; only remove the analysis destination.
//! Never derive directories or filenames from extractor metadata ourselves.

use std::path::{Component, Path, PathBuf};

fn without_verbatim_prefix(path: &Path) -> PathBuf {
    let value = path.to_string_lossy();
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{unc}"))
    } else if let Some(value) = value.strip_prefix(r"\\?\") {
        PathBuf::from(value)
    } else {
        path.to_path_buf()
    }
}

pub(crate) fn gallery_browser_relative_path(
    formatted_path: &Path,
    destination: &Path,
) -> Result<String, String> {
    let path = without_verbatim_prefix(formatted_path);
    let base = without_verbatim_prefix(destination);
    let relative = path.strip_prefix(&base).map_err(|_| {
        "gallery-dl formatted a browser path outside its analysis destination.".to_owned()
    })?;
    let mut segments = Vec::new();
    for component in relative.components() {
        let Component::Normal(segment) = component else {
            return Err("gallery-dl browser path contains a non-relative component.".to_owned());
        };
        let segment = segment
            .to_str()
            .ok_or_else(|| "gallery-dl browser path is not valid Unicode.".to_owned())?;
        if segment.is_empty() || segment.contains(['/', '\\', ':']) {
            return Err("gallery-dl browser path contains an invalid component.".to_owned());
        }
        segments.push(segment);
    }
    if segments.is_empty() {
        return Err("gallery-dl browser path has no filename.".to_owned());
    }
    Ok(segments.join("/"))
}
