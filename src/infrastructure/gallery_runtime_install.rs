//! Whole-bundle publication/recovery; never falls back to an upstream EXE.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Serialize, Deserialize)]
struct Journal {
    transaction_id: String,
    phase: String,
    had_previous: bool,
    #[serde(default)]
    previous_state: Option<String>,
    #[serde(default)]
    previous_bundle: Option<(String, String)>,
    #[serde(default)]
    new_bundle: Option<(String, String)>,
}
fn installed_state() -> PathBuf {
    storage_paths::state_path("component-update/gallery-dl-installed.yaml")
}
fn record_installed(manifest: &BundleManifest) -> Result<(), String> {
    let record = serde_json::json!({"bundle_id": manifest.bundle_id, "upstream_version":manifest.upstream_version,
        "upstream_commit":manifest.upstream_commit, "patch_revision":manifest.patch_revision,
        "inventory_sha256":manifest.inventory_sha256,"yt_dlp_version":manifest.yt_dlp_version,"platform":manifest.platform});
    super::super::yaml_store::write_persistent_yaml(&installed_state(), &record)
}
fn journal_path() -> PathBuf {
    storage_paths::state_path("component-update/gallery-dl-install.yaml")
}
fn locations(id: &str) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        return Err("Invalid gallery install transaction identity.".to_owned());
    }
    let paths = storage_paths::require_ready()?;
    Ok((
        paths.application_root.join("tools/gallery-dl"),
        paths
            .application_root
            .join(format!("tools/.gallery-dl-install-{id}")),
        paths
            .state_root
            .join("component-update/gallery-dl-backup")
            .join(id),
    ))
}
fn exclusive() -> Result<File, String> {
    let file = storage_paths::open_lock(&storage_paths::state_path(
        "runtime/gallery-dl-runtime.lock",
    ))?;
    file.try_lock()
        .map_err(|_| "gallery-dl is in use. Try again when current gallery tasks have finished.")?;
    Ok(file)
}
fn save(journal: &Journal) -> Result<(), String> {
    super::super::yaml_store::write_persistent_yaml(&journal_path(), journal)
}
fn read_journal() -> Result<Option<Journal>, String> {
    let path = journal_path();
    storage_paths::reject_links(&path)?;
    match fs::read_to_string(path) {
        Ok(text) => {
            let journal: Journal = serde_yaml::from_str(&text)
                .map_err(|e| format!("Invalid gallery install recovery journal: {e}"))?;
            if !matches!(
                journal.phase.as_str(),
                "preparing" | "prepared" | "retained" | "published" | "verified"
            ) {
                return Err("Unknown gallery install recovery phase.".to_owned());
            }
            Ok(Some(journal))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
fn clear_journal() -> Result<(), String> {
    storage_paths::reject_links(&journal_path())?;
    fs::remove_file(journal_path()).map_err(|e| e.to_string())
}
fn recover() -> Result<(), String> {
    let Some(journal) = read_journal()? else {
        return Ok(());
    };
    let (installed, prepared, backup) = locations(&journal.transaction_id)?;
    for path in [&installed, &prepared, &backup] {
        storage_paths::reject_links(path)?;
    }
    if journal.phase == "verified" && installed.exists() {
        let manifest = verify_bundle(&installed)?;
        capability(&installed.join(&manifest.entrypoint), &manifest)?;
        record_installed(&manifest)?;
        storage_paths::remove_owned_tree(&prepared)?;
        return clear_journal();
    }
    if backup.exists() {
        storage_paths::remove_owned_tree(&installed)?;
        fs::rename(&backup, &installed)
            .map_err(|e| format!("Could not restore the previous gallery runtime: {e}"))?;
    } else if !journal.had_previous && matches!(journal.phase.as_str(), "published" | "retained") {
        storage_paths::remove_owned_tree(&installed)?;
    } else if journal.had_previous && !installed.exists() {
        return Err(
            "The gallery install recovery tree is missing. Tool maintenance requires recovery."
                .to_owned(),
        );
    }
    storage_paths::remove_owned_tree(&prepared)?;
    if let Some(previous) = &journal.previous_state {
        super::super::yaml_store::atomic_write(&installed_state(), previous.as_bytes())?;
    } else if matches!(journal.phase.as_str(), "published" | "retained")
        && installed_state().exists()
    {
        storage_paths::reject_links(&installed_state())?;
        fs::remove_file(installed_state()).map_err(|e| e.to_string())?;
    }
    clear_journal()
}
pub(super) fn pending() -> Result<bool, String> {
    storage_paths::reject_links(&journal_path())?;
    Ok(journal_path().exists())
}
pub(crate) fn recover_if_needed() -> Result<(), String> {
    if !pending()? {
        return Ok(());
    }
    let _exclusive = exclusive()?;
    recover()
}
#[cfg(test)]
pub(super) fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    copy_checked(source, destination, &|| false)
}
fn copy_checked(
    source: &Path,
    destination: &Path,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if cancelled() {
        return Err("Dependency deployment cancelled.".to_owned());
    }
    storage_paths::reject_links(source)?;
    storage_paths::reject_links(destination)?;
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        storage_paths::reject_links(&from)?;
        if from.is_dir() {
            copy_checked(&from, &to, cancelled)?;
        } else {
            fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn preserve_unknown(installed: &Path, prepared: &Path) -> Result<(), String> {
    if !installed.exists() {
        return Ok(());
    }
    let old_manifest = manifest_identity(installed).ok();
    let owned_manifest = old_manifest.is_some();
    let managed = old_manifest
        .map(|m| m.files)
        .unwrap_or_else(|| BTreeMap::from([("gallery-dl.exe".to_owned(), String::new())]));
    let mut files = Vec::new();
    scan_files(installed, installed, &mut files)?;
    for source in files {
        let relative = source.strip_prefix(installed).map_err(|e| e.to_string())?;
        let name = relative.to_string_lossy().replace('\\', "/");
        if (name == "bundle.manifest.json" && owned_manifest) || managed.contains_key(&name) {
            continue;
        }
        let target = prepared.join(relative);
        if target.exists() {
            return Err(format!(
                "Unrelated file conflicts with the new gallery runtime: {name}"
            ));
        }
        fs::create_dir_all(target.parent().ok_or("Unknown tool file has no parent")?)
            .map_err(|e| e.to_string())?;
        fs::copy(source, target).map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(crate) fn install_managed() -> Result<PathBuf, String> {
    install_managed_with_cancel(&|| false)
}
pub(crate) fn install_managed_with_cancel(cancelled: &dyn Fn() -> bool) -> Result<PathBuf, String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let _storage = storage_paths::use_cache()?;
    let _exclusive = exclusive()?;
    recover()?;
    let source = local_bundle()?;
    let manifest = verify_bundle(&source)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let id = format!(
        "{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let (installed, prepared, backup) = locations(&id)?;
    for path in [&installed, &prepared, &backup] {
        storage_paths::reject_links(path)?;
    }
    let cached = storage_paths::cache_directory("temp/tool-install")?.join(&id);
    storage_paths::reject_links(&cached)?;
    if prepared.exists() || backup.exists() || cached.exists() {
        return Err("Gallery installation transaction paths already exist.".to_owned());
    }
    storage_paths::reject_links(&installed_state())?;
    let mut journal = Journal {
        transaction_id: id,
        phase: "preparing".to_owned(),
        had_previous: installed.exists(),
        previous_state: match fs::read_to_string(installed_state()) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        },
        previous_bundle: manifest_identity(&installed)
            .ok()
            .map(|m| (m.bundle_id, m.inventory_sha256)),
        new_bundle: Some((
            manifest.bundle_id.clone(),
            manifest.inventory_sha256.clone(),
        )),
    };
    save(&journal)?;
    let result = (|| {
        copy_checked(&source, &cached, cancelled)?;
        verify_bundle(&cached)?;
        copy_checked(&cached, &prepared, cancelled)?;
        preserve_unknown(&installed, &prepared)?;
        verify_bundle(&prepared)?;
        if cancelled() {
            return Err("Dependency deployment cancelled.".to_owned());
        }
        journal.phase = "prepared".to_owned();
        save(&journal)?;
        if installed.exists() {
            fs::create_dir_all(backup.parent().ok_or("Gallery backup has no parent")?)
                .map_err(|e| e.to_string())?;
            fs::rename(&installed, &backup)
                .map_err(|e| format!("Could not retain the previous gallery runtime: {e}"))?;
        }
        journal.phase = "retained".to_owned();
        save(&journal)?;
        fs::rename(&prepared, &installed)
            .map_err(|e| format!("Could not publish the gallery runtime: {e}"))?;
        journal.phase = "published".to_owned();
        save(&journal)?;
        let actual = verify_bundle(&installed)?;
        capability(&installed.join(&manifest.entrypoint), &actual)?;
        record_installed(&manifest)?;
        journal.phase = "verified".to_owned();
        save(&journal)?;
        clear_journal()?;
        Ok(installed.join(&manifest.entrypoint))
    })();
    let _ = storage_paths::remove_owned_tree(&cached);
    if let Err(error) = result {
        return match recover() {
            Ok(()) => Err(error),
            Err(recovery) => Err(format!("{error}; recovery failed: {recovery}")),
        };
    }
    result
}
