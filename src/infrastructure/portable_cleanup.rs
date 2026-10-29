use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

use serde::Deserialize;

use super::component_update::{
    PendingAppUpdateManifest, PendingAppUpdateState, other_registered_app_instances,
    pending_app_update_manifest,
};
use super::yaml_store::read_yaml_file;

const TRANSIENT_STALE_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const AUDIO_CACHE_STALE_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const APP_UPDATE_BACKUP_STALE_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const YT_DLP_SIGFUNCS_STALE_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const STARTUP_CLEANUP_DELAY: Duration = Duration::from_secs(2);

const CURRENT_CACHE_POLICIES: &[CacheCleanupPolicy] = &[
    CacheCleanupPolicy {
        relative_path: &["temp", "native-cookies"],
        action: CacheCleanupAction::StaleChildren,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["gallery-dl-temp"],
        action: CacheCleanupAction::StaleChildren,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["temp", "tool-install"],
        action: CacheCleanupAction::StaleChildren,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["temp", "cookie-rescue"],
        action: CacheCleanupAction::StaleChildren,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["temp", "runtime", "instances"],
        action: CacheCleanupAction::StaleChildren,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["temp", "manifest"],
        action: CacheCleanupAction::ManifestTransients,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["yt-dlp-temp"],
        action: CacheCleanupAction::StaleChildren,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["transcode-temp"],
        action: CacheCleanupAction::StaleChildren,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["yt-dlp", "youtube-sigfuncs"],
        action: CacheCleanupAction::StaleChildren,
        stale_age: YT_DLP_SIGFUNCS_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["manifest", "backup"],
        action: CacheCleanupAction::AppUpdateBackups,
        stale_age: APP_UPDATE_BACKUP_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["audio"],
        action: CacheCleanupAction::AudioStreamCache,
        stale_age: AUDIO_CACHE_STALE_AGE,
    },
];

const LEGACY_CACHE_POLICIES: &[CacheCleanupPolicy] = &[
    CacheCleanupPolicy {
        relative_path: &["tool-install"],
        action: CacheCleanupAction::StaleChildren,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["cookie-rescue"],
        action: CacheCleanupAction::StaleChildren,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["runtime", "instances"],
        action: CacheCleanupAction::StaleChildren,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["temp", "component-update"],
        action: CacheCleanupAction::ManifestTransients,
        stale_age: TRANSIENT_STALE_AGE,
    },
    CacheCleanupPolicy {
        relative_path: &["component-update"],
        action: CacheCleanupAction::ManifestTransients,
        stale_age: TRANSIENT_STALE_AGE,
    },
];

const OBSOLETE_V2_CACHE_JSON_FILES: &[&[&str]] = &[
    &["manifest", "state.json"],
    &["manifest", "installed.json"],
    &["manifest", "pending_app_update.json"],
    &["manifest", "installed_app_version.json"],
    &["audio-playlist.json"],
    &["music-playlist.json"],
    &["component-update", "component_state.json"],
    &["component-update", "state.json"],
    &["component-update", "installed_components.json"],
    &["component-update", "installed.json"],
    &["component-update", "pending_app_update.json"],
    &["component-update", "installed_app_version.json"],
];

#[derive(Clone, Copy, Debug)]
struct CacheCleanupPolicy {
    relative_path: &'static [&'static str],
    action: CacheCleanupAction,
    stale_age: Duration,
}

#[derive(Clone, Copy, Debug)]
enum CacheCleanupAction {
    StaleChildren,
    ManifestTransients,
    AppUpdateBackups,
    AudioStreamCache,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct AudioCacheCleanupManifest {
    updated_unix_seconds: u64,
}

#[derive(Clone, Debug, Default)]
pub struct PortableCleanupSummary {
    pub removed_entries: u64,
    pub removed_bytes: u64,
    pub errors: Vec<String>,
}

impl PortableCleanupSummary {
    pub fn has_work(&self) -> bool {
        self.removed_entries > 0 || !self.errors.is_empty()
    }
}

pub fn cleanup_startup_transient_files() -> PortableCleanupSummary {
    let mut summary = PortableCleanupSummary::default();
    let Ok(_guard) = super::storage_paths::exclusive_cleanup() else {
        return summary;
    };

    if other_registered_app_instances() > 0 {
        return summary;
    }

    let cache_root = super::storage_paths::paths().cache_root.clone();
    if !cache_root.is_dir() {
        return summary;
    }

    let pending_app_update = pending_app_update_manifest();
    cleanup_current_transient_locations(&cache_root, pending_app_update.as_ref(), &mut summary);

    summary
}

/// Recovery retention belongs to the updater lifecycle, not generic cache clearing.
pub(crate) fn cleanup_update_recovery_backups() {
    let pending = pending_app_update_manifest();
    let mut summary = PortableCleanupSummary::default();
    cleanup_app_update_backups(
        &super::storage_paths::state_path("component-update/backup"),
        pending.as_ref(),
        APP_UPDATE_BACKUP_STALE_AGE,
        &mut summary,
    );
    for error in summary.errors {
        eprintln!("[update-recovery] {error}");
    }
}

pub fn schedule_startup_transient_cleanup() {
    let spawn_result = thread::Builder::new()
        .name("startup-transient-cleanup".to_owned())
        .spawn(|| {
            thread::sleep(STARTUP_CLEANUP_DELAY);
            let summary = cleanup_startup_transient_files();
            if !summary.has_work() {
                return;
            }
            eprintln!(
                "[portable-cleanup] removed {} stale transient entries ({} bytes)",
                summary.removed_entries, summary.removed_bytes
            );
            for error in summary.errors {
                eprintln!("[portable-cleanup] {error}");
            }
        });
    if let Err(error) = spawn_result {
        eprintln!("[portable-cleanup] could not schedule startup cleanup: {error}");
    }
}

fn cleanup_current_transient_locations(
    cache_root: &Path,
    pending_app_update: Option<&PendingAppUpdateManifest>,
    summary: &mut PortableCleanupSummary,
) {
    cleanup_cache_policies(
        cache_root,
        CURRENT_CACHE_POLICIES,
        pending_app_update,
        summary,
    );
}

fn cleanup_legacy_transient_locations(
    cache_root: &Path,
    pending_app_update: Option<&PendingAppUpdateManifest>,
    summary: &mut PortableCleanupSummary,
) {
    cleanup_cache_policies(
        cache_root,
        LEGACY_CACHE_POLICIES,
        pending_app_update,
        summary,
    );
}

fn cleanup_cache_policies(
    cache_root: &Path,
    policies: &[CacheCleanupPolicy],
    pending_app_update: Option<&PendingAppUpdateManifest>,
    summary: &mut PortableCleanupSummary,
) {
    for policy in policies {
        let root = policy_path(cache_root, policy.relative_path);
        if policy.relative_path == ["manifest", "backup"]
            || policy.relative_path == ["temp", "runtime", "instances"]
        {
            continue;
        }
        if super::storage_paths::reject_links(&root).is_err() {
            continue;
        }
        if policy.relative_path == ["temp", "manifest"] && pending_app_update.is_some() {
            continue;
        }
        match policy.action {
            CacheCleanupAction::StaleChildren => {
                cleanup_stale_children_older_than(&root, policy.stale_age, summary);
            }
            CacheCleanupAction::ManifestTransients => {
                cleanup_manifest_transients(&root, pending_app_update, policy.stale_age, summary);
            }
            CacheCleanupAction::AppUpdateBackups => {
                cleanup_app_update_backups(&root, pending_app_update, policy.stale_age, summary);
            }
            CacheCleanupAction::AudioStreamCache => {
                cleanup_audio_stream_cache(&root, policy.stale_age, summary);
            }
        }
    }
}

fn policy_path(cache_root: &Path, relative_path: &[&str]) -> PathBuf {
    let mut path = cache_root.to_path_buf();
    for segment in relative_path {
        path.push(segment);
    }
    path
}

fn cleanup_manifest_transients(
    root: &Path,
    pending_app_update: Option<&PendingAppUpdateManifest>,
    stale_age: Duration,
    summary: &mut PortableCleanupSummary,
) {
    cleanup_stale_children_older_than(&root.join("downloads"), stale_age, summary);
    cleanup_stale_component_staging(&root.join("staged"), pending_app_update, stale_age, summary);
    cleanup_stale_children_older_than(&root.join("runner"), stale_age, summary);
    remove_empty_dir(root, summary);
}

fn cleanup_stale_component_staging(
    staged_root: &Path,
    pending_app_update: Option<&PendingAppUpdateManifest>,
    stale_age: Duration,
    summary: &mut PortableCleanupSummary,
) {
    let Ok(entries) = fs::read_dir(staged_root) else {
        return;
    };

    let protected_app_stage = pending_app_update.and_then(active_pending_app_stage_dir);
    for entry in entries.flatten() {
        let path = entry.path();
        if protected_app_stage
            .as_ref()
            .is_some_and(|protected| paths_equivalent_or_child(&path, protected))
        {
            continue;
        }
        remove_stale_child_older_than(&path, stale_age, summary);
    }
    remove_empty_dir(staged_root, summary);
}

fn active_pending_app_stage_dir(manifest: &PendingAppUpdateManifest) -> Option<PathBuf> {
    if !matches!(
        manifest.state,
        PendingAppUpdateState::DownloadedStaged
            | PendingAppUpdateState::ApplyRequested
            | PendingAppUpdateState::Applying
            | PendingAppUpdateState::Failed
    ) {
        return None;
    }
    manifest.staged_exe.parent().map(Path::to_path_buf)
}

fn cleanup_app_update_backups(
    backup_root: &Path,
    pending_app_update: Option<&PendingAppUpdateManifest>,
    stale_age: Duration,
    summary: &mut PortableCleanupSummary,
) {
    if super::storage_paths::reject_links(backup_root).is_err() {
        return;
    }
    let Ok(entries) = fs::read_dir(backup_root) else {
        return;
    };

    let protected_backup = pending_app_update.and_then(active_pending_app_backup_path);
    for entry in entries.flatten() {
        let path = entry.path();
        if protected_backup
            .as_ref()
            .is_some_and(|protected| paths_equivalent_or_child(&path, protected))
        {
            continue;
        }
        remove_stale_child_older_than(&path, stale_age, summary);
    }
    remove_empty_dir(backup_root, summary);
}

fn active_pending_app_backup_path(manifest: &PendingAppUpdateManifest) -> Option<PathBuf> {
    if !matches!(
        manifest.state,
        PendingAppUpdateState::ApplyRequested
            | PendingAppUpdateState::Applying
            | PendingAppUpdateState::Failed
    ) {
        return None;
    }
    Some(manifest.backup_exe.clone())
}

fn cleanup_audio_stream_cache(
    audio_root: &Path,
    stale_age: Duration,
    summary: &mut PortableCleanupSummary,
) {
    let Ok(entries) = fs::read_dir(audio_root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || audio_cache_reserved_dir(&path) {
            continue;
        }
        if audio_cache_child_is_expired(&path, stale_age) {
            remove_counted_path(&path, summary);
        }
    }
    remove_empty_dir(audio_root, summary);
}

fn audio_cache_reserved_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| matches!(name, "covers" | "lyrics"))
}

fn audio_cache_child_is_expired(path: &Path, stale_age: Duration) -> bool {
    let manifest_path = path.join("manifest.yaml");
    if let Some(manifest) = read_yaml_file::<AudioCacheCleanupManifest>(&manifest_path) {
        return unix_timestamp_is_stale(manifest.updated_unix_seconds, stale_age);
    }

    let obsolete_json_manifest = path.join("manifest.json");
    if obsolete_json_manifest.is_file() {
        return path_is_stale(path, TRANSIENT_STALE_AGE);
    }

    path_is_stale(path, stale_age)
}

fn cleanup_stale_children_older_than(
    root: &Path,
    stale_age: Duration,
    summary: &mut PortableCleanupSummary,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        remove_stale_child_older_than(&entry.path(), stale_age, summary);
    }
    remove_empty_dir(root, summary);
}

fn remove_stale_child_older_than(
    path: &Path,
    stale_age: Duration,
    summary: &mut PortableCleanupSummary,
) {
    if !path_is_stale(path, stale_age) {
        return;
    }

    let bytes = path_size_bytes(path);
    remove_path_with_known_size(path, bytes, summary);
}

fn path_is_stale(path: &Path, stale_age: Duration) -> bool {
    if super::storage_paths::reject_links(path).is_err() {
        return false;
    }
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age >= stale_age)
}

fn unix_timestamp_is_stale(updated_unix_seconds: u64, stale_age: Duration) -> bool {
    updated_unix_seconds == 0
        || now_unix_seconds().saturating_sub(updated_unix_seconds) >= stale_age.as_secs()
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn path_size_bytes(path: &Path) -> u64 {
    super::storage_paths::tree_size(path)
}

fn remove_path(path: &Path) -> std::io::Result<()> {
    super::storage_paths::remove_owned_tree(path).map_err(std::io::Error::other)
}

fn remove_counted_path(path: &Path, summary: &mut PortableCleanupSummary) {
    let bytes = path_size_bytes(path);
    remove_path_with_known_size(path, bytes, summary);
}

fn remove_path_with_known_size(path: &Path, bytes: u64, summary: &mut PortableCleanupSummary) {
    match remove_path(path) {
        Ok(()) => {
            summary.removed_entries = summary.removed_entries.saturating_add(1);
            summary.removed_bytes = summary.removed_bytes.saturating_add(bytes);
        }
        Err(error) => summary
            .errors
            .push(format!("Could not remove {}: {error}", path.display())),
    }
}

fn remove_empty_dir(path: &Path, summary: &mut PortableCleanupSummary) {
    let Ok(mut entries) = fs::read_dir(path) else {
        return;
    };
    if entries.next().is_some() {
        return;
    }
    match fs::remove_dir(path) {
        Ok(()) => {
            summary.removed_entries = summary.removed_entries.saturating_add(1);
        }
        Err(error) => summary.errors.push(format!(
            "Could not remove empty {}: {error}",
            path.display()
        )),
    }
}

fn cleanup_obsolete_v2_json_files(cache_root: &Path, summary: &mut PortableCleanupSummary) {
    for relative_path in OBSOLETE_V2_CACHE_JSON_FILES {
        let path = policy_path(cache_root, relative_path);
        if path.is_file() {
            remove_counted_path(&path, summary);
        }
    }
}

fn paths_equivalent_or_child(path: &Path, parent: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let parent = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    path == parent || path.starts_with(parent)
}
