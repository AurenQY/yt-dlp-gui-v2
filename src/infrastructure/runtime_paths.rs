use std::path::PathBuf;

/// Stable application root for bundled tools, configuration, caches and updates.
pub(crate) fn application_root() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        return PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    }

    #[cfg(not(debug_assertions))]
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(ToOwned::to_owned))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(crate) fn set_application_current_dir() -> Result<(), String> {
    let root = application_root();
    std::env::set_current_dir(&root)
        .map_err(|error| format!("Could not use application root {}: {error}", root.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_application_root_is_the_compiled_workspace() {
        #[cfg(debug_assertions)]
        assert_eq!(
            application_root(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        );
    }
}
