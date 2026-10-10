//! Read the active executable's version without blocking About rendering.
//! This presentation cache is keyed by the selected path and file identity;
//! it does not modify dependency selection, ownership, or update targets.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use eframe::egui::Context;

use crate::infrastructure::runtime_paths::application_root;
use crate::infrastructure::{DependencyTool, probe_dependency_tool_version};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct ProbeKey {
    tool: DependencyTool,
    selected_path: String,
    modified: Option<SystemTime>,
    length: Option<u64>,
}

type VersionCache = HashMap<ProbeKey, Option<Option<String>>>;
static VERSIONS: OnceLock<Mutex<VersionCache>> = OnceLock::new();

pub(super) fn selected_version(
    ctx: &Context,
    tool: DependencyTool,
    configured_path: &str,
) -> Option<String> {
    if configured_path.trim().is_empty() {
        return None;
    }
    // Mirror the config/installer's support-path resolution so the displayed
    // version always belongs to the exact executable the app selected.
    let selected = PathBuf::from(configured_path.trim());
    let resolved = if selected.is_absolute() {
        selected
    } else {
        application_root().join(selected)
    };
    let metadata = resolved.metadata().ok()?;
    if !metadata.is_file() {
        return None;
    }
    let key = ProbeKey {
        tool,
        selected_path: resolved.display().to_string(),
        modified: metadata.modified().ok(),
        length: Some(metadata.len()),
    };
    let cache = VERSIONS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut cache) = cache.lock() {
        if let Some(result) = cache.get(&key) {
            return result.clone().flatten();
        }
        // None marks an in-flight worker (Some(None) marks an unsuccessful probe).
        cache.retain(|old, _| old.tool != tool);
        cache.insert(key.clone(), None);
    } else {
        return None;
    }

    let context = ctx.clone();
    let configured = key.selected_path.clone();
    std::thread::spawn(move || {
        let version = probe_dependency_tool_version(tool, &configured);
        if let Some(cache) = VERSIONS.get()
            && let Ok(mut cache) = cache.lock()
        {
            // A later selection may have replaced this key while we probed.
            if cache.contains_key(&key) {
                cache.insert(key, Some(version));
            }
        }
        context.request_repaint();
    });
    None
}
