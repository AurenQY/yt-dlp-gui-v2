//! Non-destructive import of persistent records formerly stored below cache.
use super::storage_paths::{RuntimeStoragePaths, reject_links};
use super::yaml_store::atomic_write;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Default, Serialize, Deserialize)]
struct Journal {
    version: u32,
    complete: bool,
    imported: BTreeSet<String>,
}

fn journal(paths: &RuntimeStoragePaths) -> Result<Journal, String> {
    let file = paths.state_root.join("storage-migration.yaml");
    match fs::read_to_string(&file) {
        Ok(text) => serde_yaml::from_str(&text)
            .map_err(|e| format!("Could not read migration journal {}: {e}", file.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Journal::default()),
        Err(e) => Err(format!("Could not read {}: {e}", file.display())),
    }
}

pub(crate) fn is_complete(paths: &RuntimeStoragePaths) -> Result<bool, String> {
    let journal = journal(paths)?;
    if journal.version > 1 {
        return Err("Storage layout was created by a newer application version.".to_owned());
    }
    Ok(journal.version == 1 && journal.complete)
}

pub(crate) fn legacy_instance_busy(paths: &RuntimeStoragePaths) -> Result<bool, String> {
    let root = paths.application_root.join("cache/temp/runtime/instances");
    reject_links(&root)?;
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "Could not inspect legacy application instances: {error}"
            ));
        }
    };
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age.as_secs() < 86_400)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn save_journal(paths: &RuntimeStoragePaths, journal: &Journal) -> Result<(), String> {
    let text = serde_yaml::to_string(journal).map_err(|e| e.to_string())?;
    atomic_write(
        &paths.state_root.join("storage-migration.yaml"),
        text.as_bytes(),
    )
}

fn import_yaml(source: &Path, destination: &Path) -> Result<bool, String> {
    reject_links(source)?;
    reject_links(destination)?;
    if destination.exists() {
        validate_yaml(destination)?;
        return Ok(false);
    }
    if !source.exists() {
        return Ok(false);
    }
    let value = validate_yaml(source)?;
    let text = serde_yaml::to_string(&value).map_err(|e| e.to_string())?;
    atomic_write(destination, text.as_bytes())?;
    Ok(true)
}

fn validate_yaml(path: &Path) -> Result<serde_yaml::Value, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("Could not read migration source {}: {e}", path.display()))?;
    let value: serde_yaml::Value = serde_yaml::from_slice(&bytes)
        .map_err(|e| format!("Invalid state document {}: {e}", path.display()))?;
    if !value.is_mapping() {
        return Err(format!(
            "State document is not a YAML mapping: {}",
            path.display()
        ));
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    match name {
        "installed.yaml" | "installed_app_version.yaml" => {
            super::component_update::validate_persistent_update_document(name, value.clone())?
        }
        "music-cue-memory.yaml" => {
            crate::app::state::validate_music_cue_memory(value.clone())?;
            if !value
                .get("entries")
                .is_some_and(serde_yaml::Value::is_mapping)
                || value
                    .get("version")
                    .and_then(serde_yaml::Value::as_u64)
                    .is_some_and(|version| version != 1)
            {
                return Err(format!(
                    "Invalid music cue-memory schema: {}",
                    path.display()
                ));
            }
        }
        _ => {}
    }
    Ok(value)
}

fn import_backups(source: &Path, destination: &Path) -> Result<(), String> {
    reject_links(source)?;
    reject_links(destination)?;
    if !source.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        reject_links(&path)?;
        let target = destination.join(entry.file_name());
        reject_links(&target)?;
        if path.is_dir() {
            import_backups(&path, &target)?;
        } else {
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            if target.exists() {
                if fs::read(&target).map_err(|e| e.to_string())? != bytes {
                    return Err(format!("Conflicting recovery backup: {}", target.display()));
                }
            } else {
                atomic_write(&target, &bytes)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn migrate(paths: &RuntimeStoragePaths) -> Result<(), String> {
    let mut journal = journal(paths)?;
    if journal.version == 1 && journal.complete {
        return Ok(());
    }
    let legacy = paths.application_root.join("cache");
    // Old releases do not know the new session lock. Treat their recent markers conservatively.
    for root in [
        legacy.join("temp/runtime/instances"),
        paths.state_root.join("runtime/instances"),
    ] {
        reject_links(&root)?;
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .is_some_and(|age| age.as_secs() < 86_400)
                {
                    return Err(
                        "Storage migration is busy. Close other application instances and restart."
                            .to_owned(),
                    );
                }
            }
        }
    }
    let old_update = legacy.join("manifest");
    let new_update = paths.state_root.join("component-update");
    import_backups(&old_update.join("backup"), &new_update.join("backup"))?;
    for name in ["installed.yaml", "installed_app_version.yaml"] {
        if !journal.imported.contains(name) {
            import_yaml(&old_update.join(name), &new_update.join(name))?;
            journal.imported.insert(name.to_owned());
            save_journal(paths, &journal)?;
        }
    }
    let pending_source = old_update.join("pending_app_update.yaml");
    let pending_dest = new_update.join("pending_app_update.yaml");
    if !journal.imported.contains("pending_app_update.yaml") {
        if pending_source.exists() {
            let mut old: super::component_update::PendingAppUpdateManifest =
                serde_yaml::from_value(validate_yaml(&pending_source)?)
                    .map_err(|e| format!("Invalid pending update manifest: {e}"))?;
            if old.state == super::component_update::PendingAppUpdateState::Applying {
                return Err("An update may still be applying. Restart after it finishes before migrating storage.".to_owned());
            }
            if let Ok(relative) = old.backup_exe.strip_prefix(old_update.join("backup")) {
                old.backup_exe = new_update.join("backup").join(relative);
            }
            if pending_dest.exists() {
                let current: super::component_update::PendingAppUpdateManifest =
                    serde_yaml::from_value(validate_yaml(&pending_dest)?)
                        .map_err(|e| e.to_string())?;
                if old.target_exe != current.target_exe
                    || old.staged_exe != current.staged_exe
                    || old.version != current.version
                {
                    return Err("Conflicting pending update transactions; legacy sources have been retained.".to_owned());
                }
            } else {
                let text = serde_yaml::to_string(&old).map_err(|e| e.to_string())?;
                atomic_write(&pending_dest, text.as_bytes())?;
            }
        }
        journal
            .imported
            .insert("pending_app_update.yaml".to_owned());
        save_journal(paths, &journal)?;
    }
    if !journal.imported.contains("music-cue-memory.yaml") {
        let custom = paths.portable_root.join("state/music-cue-memory.yaml");
        let source = if custom.exists() {
            custom
        } else {
            legacy.join("state/music-cue-memory.yaml")
        };
        import_yaml(&source, &paths.state_root.join("music-cue-memory.yaml"))?;
        journal.imported.insert("music-cue-memory.yaml".to_owned());
    }
    // state.yaml is a legacy About-history input. Keep it available to the existing importer.
    journal.version = 1;
    journal.complete = true;
    save_journal(paths, &journal)
}
