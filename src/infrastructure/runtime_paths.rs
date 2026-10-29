use std::path::PathBuf;

#[cfg(test)]
static FIXTURE_ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

#[cfg(test)]
pub(crate) fn set_fixture_root(root: &std::path::Path) {
    if let Some(previous) = FIXTURE_ROOT.get() {
        assert_eq!(previous, root);
    } else {
        FIXTURE_ROOT.set(root.to_owned()).unwrap();
    }
}

/// Stable application root for bundled tools, configuration, caches and updates.
pub(crate) fn application_root() -> PathBuf {
    #[cfg(cache_review_gui)]
    {
        return PathBuf::from(env!("YT_DLP_GUI_REVIEW_ROOT"));
    }
    #[cfg(test)]
    if let Some(root) = FIXTURE_ROOT.get() {
        return root.clone();
    }
    #[cfg(all(debug_assertions, not(cache_review_gui)))]
    {
        return PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    }

    #[cfg(all(not(debug_assertions), not(cache_review_gui)))]
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
