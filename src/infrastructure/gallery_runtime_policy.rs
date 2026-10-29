//! Compatibility/provenance boundary for the standalone dependency (ADR-0002).
use super::{sha256, storage_paths};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[path = "gallery_runtime_install.rs"]
mod install;
pub(crate) use install::{install_managed, install_managed_with_cancel, recover_if_needed};

pub(crate) const INCOMPATIBLE: &str = "This gallery-dl build cannot keep playlist work in the selected cache folder. Install the compatible gallery-dl runtime.";
pub(crate) const NO_BUNDLE: &str =
    "No approved gallery-dl bundle is available locally. The current runtime was kept.";
pub(crate) const PINNED: &str = "This gallery-dl build is pinned for cache compatibility. Unverified upstream updates are not installed.";
const PROTOCOL: &str = "yt-dlp-gui-cache-policy/v1";
const LOCK: &str = include_str!("../../dependencies/gallery-dl-cache-policy/runtime.lock.json");

// Validation builds record real boundary/thread counts; absent from normal builds.
#[cfg(any(test, cache_review_gui))]
static REVIEW_CHECKS: std::sync::OnceLock<std::sync::Mutex<Vec<serde_json::Value>>> =
    std::sync::OnceLock::new();
#[cfg(any(test, cache_review_gui))]
fn review_check(stage: &str) {
    REVIEW_CHECKS
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .push(serde_json::json!({
            "stage":stage,"thread":format!("{:?}", thread::current().id())
        }));
}
#[cfg(any(test, cache_review_gui))]
pub(crate) fn review_check_samples() -> Vec<serde_json::Value> {
    REVIEW_CHECKS
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .clone()
}

#[derive(Deserialize)]
struct RuntimeLock {
    upstream_commit: String,
    upstream_version: String,
    patch_revision: u32,
    yt_dlp_version: String,
    platform: String,
    bundles: Vec<ApprovedBundle>,
}
#[derive(Deserialize)]
struct ApprovedBundle {
    bundle_id: String,
    inventory_sha256: String,
    platform: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct BundleManifest {
    pub schema: u32,
    pub bundle_id: String,
    pub upstream_commit: String,
    pub upstream_version: String,
    pub patch_revision: u32,
    pub yt_dlp_version: String,
    pub platform: String,
    pub entrypoint: String,
    pub protocol: String,
    pub inventory_sha256: String,
    pub files: BTreeMap<String, String>,
}

fn approved_lock() -> Result<RuntimeLock, String> {
    serde_json::from_str(LOCK).map_err(|e| format!("Invalid gallery runtime lock: {e}"))
}
fn incompatible(path: &Path, reason: impl std::fmt::Display) -> String {
    format!("{INCOMPATIBLE} Path: {}. {reason}", path.display())
}
fn safe_relative(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.contains(['\\', ':', '\0'])
        || name
            .split('/')
            .any(|c| c.is_empty() || c == "." || c == "..")
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Unsafe gallery bundle path.".to_owned());
    }
    Ok(())
}

/// Authenticate inventory metadata before executing any candidate or trusting old ownership.
pub(crate) fn manifest_identity(root: &Path) -> Result<BundleManifest, String> {
    storage_paths::reject_links(root)?;
    let path = root.join("bundle.manifest.json");
    storage_paths::reject_links(&path)?;
    if fs::metadata(&path)
        .map_err(|e| incompatible(root, e))?
        .len()
        > 4 * 1024 * 1024
    {
        return Err(incompatible(root, "Bundle manifest is too large."));
    }
    let manifest: BundleManifest =
        serde_json::from_slice(&fs::read(&path).map_err(|e| incompatible(root, e))?)
            .map_err(|e| incompatible(root, e))?;
    let lock = approved_lock()?;
    let matching = lock.bundles.iter().any(|b| {
        b.bundle_id == manifest.bundle_id
            && b.inventory_sha256 == manifest.inventory_sha256
            && b.platform == manifest.platform
    });
    if !matching
        || manifest.schema != 1
        || manifest.upstream_commit != lock.upstream_commit
        || manifest.upstream_version != lock.upstream_version
        || manifest.patch_revision != lock.patch_revision
        || manifest.yt_dlp_version != lock.yt_dlp_version
        || manifest.platform != lock.platform
        || manifest.protocol != PROTOCOL
        || manifest.entrypoint != "gallery-dl.exe"
    {
        return Err(incompatible(root, "The bundle identity is not approved."));
    }
    let mut identities = BTreeSet::new();
    let mut canonical = String::new();
    for (name, hash) in &manifest.files {
        safe_relative(name)?;
        if !identities.insert(name.to_lowercase())
            || hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(incompatible(
                root,
                "Invalid or duplicate bundle inventory entry.",
            ));
        }
        canonical.push_str(&format!("{name}\0{hash}\n"));
    }
    if !manifest.files.contains_key(&manifest.entrypoint)
        || sha256::sha256_hex_bytes(canonical.as_bytes()) != manifest.inventory_sha256
    {
        return Err(incompatible(root, "Bundle inventory digest mismatch."));
    }
    Ok(manifest)
}

fn scan_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    storage_paths::reject_links(current)?;
    for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        storage_paths::reject_links(&path)?;
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if metadata.is_dir() {
            scan_files(root, &path, files)?;
        } else if metadata.is_file() {
            if !path.starts_with(root) {
                return Err("Gallery bundle containment failed.".to_owned());
            }
            files.push(path);
        } else {
            return Err("Unsupported gallery bundle file type.".to_owned());
        }
    }
    Ok(())
}

pub(crate) fn verify_bundle(root: &Path) -> Result<BundleManifest, String> {
    #[cfg(any(test, cache_review_gui))]
    review_check("inventory");
    let manifest = manifest_identity(root)?;
    for (name, hash) in &manifest.files {
        let path = root.join(name);
        storage_paths::reject_links(&path)?;
        if !path.is_file() || sha256::sha256_hex_file(&path)? != *hash {
            return Err(incompatible(
                root,
                format!("Changed or missing runtime file: {name}"),
            ));
        }
    }
    let mut files = Vec::new();
    scan_files(root, root, &mut files)?;
    for path in files {
        let relative = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(
            extension.as_str(),
            "exe" | "dll" | "pyd" | "py" | "pyc" | "pyo" | "zip"
        ) && !manifest.files.contains_key(&relative)
        {
            return Err(incompatible(
                root,
                format!("Unrecognized loadable runtime file: {relative}"),
            ));
        }
    }
    Ok(manifest)
}

pub(crate) fn local_bundle() -> Result<PathBuf, String> {
    let lock = approved_lock()?;
    let app = &storage_paths::require_ready()?.application_root;
    for bundle in lock.bundles {
        safe_relative(&bundle.bundle_id)?;
        let root = app.join("tool-bundles/gallery-dl").join(bundle.bundle_id);
        if root.exists() {
            verify_bundle(&root)?;
            return Ok(root);
        }
    }
    Err(NO_BUNDLE.to_owned())
}

pub(crate) fn capability(program: &Path, manifest: &BundleManifest) -> Result<(), String> {
    #[cfg(any(test, cache_review_gui))]
    review_check("capability");
    let mut command = Command::new(program);
    super::configure_background_command(&mut command);
    let mut child = command
        .arg("--yt-dlp-gui-cache-capabilities")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| incompatible(program, e))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Missing gallery capability output")?;
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(16_385).read_to_end(&mut bytes).map(|_| bytes)
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if start.elapsed() < Duration::from_secs(10) => {
                thread::sleep(Duration::from_millis(10))
            }
            outcome => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(format!(
                    "Gallery capability probe failed or timed out: {outcome:?}"
                ));
            }
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| "Gallery capability reader failed")?
        .map_err(|e| e.to_string())?;
    if !status?.success() || bytes.len() > 16_384 {
        return Err(incompatible(program, "Invalid capability output."));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| incompatible(program, e))?;
    for (key, expected) in [
        ("protocol", PROTOCOL),
        ("source_commit", manifest.upstream_commit.as_str()),
        ("upstream_version", manifest.upstream_version.as_str()),
        ("yt_dlp_version", manifest.yt_dlp_version.as_str()),
        ("platform", manifest.platform.as_str()),
    ] {
        if value[key].as_str() != Some(expected) {
            return Err(incompatible(
                program,
                format!("Incompatible capability field: {key}"),
            ));
        }
    }
    if value["patch_revision"].as_u64() != Some(manifest.patch_revision as u64) {
        return Err(incompatible(program, "Patch revision mismatch."));
    }
    for feature in [
        "owned-part-directory",
        "playlist-artifact-finalization",
        "actual-artifact-events",
        "metadata-no-media-write",
    ] {
        if !value["features"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(feature)))
        {
            return Err(incompatible(
                program,
                "Required cache capability is absent.",
            ));
        }
    }
    Ok(())
}

pub(crate) struct GalleryRuntimeGuard {
    _runtime: File,
    _storage: storage_paths::StorageUseGuard,
}

pub(crate) fn acquire(program: &Path) -> Result<GalleryRuntimeGuard, String> {
    #[cfg(any(test, cache_review_gui))]
    review_check("acquire");
    let storage = storage_paths::use_cache()?;
    recover_if_needed()?;
    let runtime = loop {
        let runtime = storage_paths::open_lock(&storage_paths::state_path(
            "runtime/gallery-dl-runtime.lock",
        ))?;
        runtime.try_lock_shared().map_err(
            |_| "gallery-dl is being updated. Try again when tool maintenance has finished.",
        )?;
        if !install::pending()? {
            break runtime;
        }
        drop(runtime);
        recover_if_needed()?;
    };
    // A retained-tree interruption can temporarily leave no executable at the
    // managed path. Recovery must run before interpreting absence as missing.
    if !program.is_file() {
        return Err(format!(
            "gallery-dl executable was not found: {}",
            program.display()
        ));
    }
    let root = program
        .parent()
        .ok_or("Gallery executable has no containing folder")?;
    if program.file_name().and_then(|s| s.to_str()) != Some("gallery-dl.exe") {
        return Err(incompatible(
            program,
            "A complete standalone bundle is required.",
        ));
    }
    let manifest = verify_bundle(root)?;
    capability(program, &manifest)?;
    Ok(GalleryRuntimeGuard {
        _runtime: runtime,
        _storage: storage,
    })
}

pub(crate) fn readiness(program: &Path) -> Result<(), String> {
    #[cfg(any(test, cache_review_gui))]
    let started = Instant::now();
    let result = acquire(program).map(drop);
    #[cfg(any(test, cache_review_gui))]
    REVIEW_CHECKS
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .push(serde_json::json!({
            "stage":"status-finished", "thread":format!("{:?}", thread::current().id()),
            "elapsed_ms":started.elapsed().as_millis(), "result":result, "path":program
        }));
    result
}

#[cfg(test)]
#[path = "gallery_runtime_tests.rs"]
mod tests;
