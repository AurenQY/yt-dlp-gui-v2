//! One startup snapshot for cache consumers, independent of later settings saves.
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use super::tools::CacheLocationMode;

pub(crate) const DISPOSABLE_DIRS: &[&str] = &[
    "audio",
    "queue-thumbnails-v1",
    "yt-dlp",
    "yt-dlp-temp",
    "gallery-dl-temp",
    "transcode-temp",
    "temp/tool-install",
    "temp/cookie-rescue",
    "temp/native-cookies",
    "temp/manifest",
];

#[derive(Clone, Debug)]
pub(crate) struct RuntimeStoragePaths {
    pub application_root: PathBuf,
    pub installation_id: String,
    pub active_mode: CacheLocationMode,
    pub cache_root: PathBuf,
    pub state_root: PathBuf,
    pub portable_root: PathBuf,
    pub temporary_root: PathBuf,
}

struct RuntimeStorage {
    paths: RuntimeStoragePaths,
    error: Option<String>,
    _session: Option<File>,
}

static STORAGE: OnceLock<RuntimeStorage> = OnceLock::new();

pub(crate) fn normalized(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(part.as_os_str()),
        }
    }
    result
}

fn identity(path: &Path) -> Result<String, String> {
    let canonical = path.canonicalize().map_err(|e| {
        format!(
            "Could not resolve application folder {}: {e}",
            path.display()
        )
    })?;
    let text = canonical
        .to_str()
        .ok_or("Application folder is not valid Unicode")?;
    #[cfg(windows)]
    let text = text
        .strip_prefix(r"\\?\")
        .unwrap_or(text)
        .replace('/', "\\")
        .to_lowercase();
    Ok(super::sha256::sha256_hex_bytes(text.as_bytes())[..32].to_owned())
}

fn requested_paths(
    app: &Path,
    temp: &Path,
    mode: CacheLocationMode,
    portable: &str,
) -> Result<RuntimeStoragePaths, String> {
    let app = normalized(app);
    let portable = if portable.trim().is_empty() {
        app.join("cache")
    } else {
        let path = PathBuf::from(portable.trim());
        normalized(&if path.is_absolute() {
            path
        } else {
            app.join(path)
        })
    };
    let installation_id = identity(&app)?;
    let temporary = temp.join("yt-dlp-gui").join(&installation_id).join("cache");
    Ok(RuntimeStoragePaths {
        application_root: app.clone(),
        installation_id,
        active_mode: mode,
        cache_root: if mode == CacheLocationMode::WindowsTemp {
            temporary.clone()
        } else {
            portable.clone()
        },
        state_root: app.join("state"),
        portable_root: portable,
        temporary_root: temporary,
    })
}

pub(crate) fn resolve(
    app: &Path,
    temp: &Path,
    mode: CacheLocationMode,
    portable: &str,
) -> Result<RuntimeStoragePaths, String> {
    let paths = requested_paths(app, temp, mode, portable)?;
    validate_root(&paths)?;
    Ok(paths)
}

pub(crate) fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

/// Inspect every existing ancestor, including junctions and the requested root.
pub(crate) fn reject_links(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if is_link(&metadata) => {
                return Err(format!(
                    "Storage links are not supported: {}",
                    ancestor.display()
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Could not inspect {}: {e}", ancestor.display())),
        }
    }
    Ok(())
}

fn validate_root(paths: &RuntimeStoragePaths) -> Result<(), String> {
    let root = &paths.cache_root;
    if !root.is_absolute() || root.parent().is_none() {
        return Err(format!(
            "Choose an absolute application-owned cache folder: {}",
            root.display()
        ));
    }
    reject_links(root)?;
    let mut protected = vec![
        paths.application_root.clone(),
        paths.state_root.clone(),
        paths.application_root.join("tools"),
        paths.application_root.join("download"),
    ];
    if let Some(temp) = paths
        .temporary_root
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
    {
        protected.push(temp.to_owned());
    }
    for name in ["USERPROFILE", "HOME"] {
        if let Some(profile) = std::env::var_os(name) {
            let profile = PathBuf::from(profile);
            protected.push(profile.clone());
            for child in [
                "Desktop",
                "Downloads",
                "Documents",
                "Pictures",
                "Videos",
                "Music",
            ] {
                protected.push(profile.join(child));
            }
        }
    }
    let comparable = |p: &Path| p.to_string_lossy().replace('/', "\\").to_lowercase();
    if protected.iter().any(|p| {
        comparable(p) == comparable(root) || comparable(p).starts_with(&(comparable(root) + "\\"))
    }) || root.starts_with(&paths.state_root)
        || root.starts_with(paths.application_root.join("tools"))
    {
        return Err(format!(
            "Refusing to use a protected folder as cache: {}",
            root.display()
        ));
    }
    Ok(())
}

pub(crate) fn open_lock(path: &Path) -> Result<File, String> {
    reject_links(path)?;
    fs::create_dir_all(path.parent().ok_or("Storage lock has no parent")?)
        .map_err(|e| format!("Could not create storage coordination folder: {e}"))?;
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("Could not open storage lock {}: {e}", path.display()))
}

pub(crate) fn initialize(mode: CacheLocationMode, portable: &str, output: &str) {
    initialize_at(
        &super::runtime_paths::application_root(),
        &std::env::temp_dir(),
        mode,
        portable,
        output,
    );
}

fn initialize_at(app: &Path, temp: &Path, mode: CacheLocationMode, portable: &str, output: &str) {
    STORAGE.get_or_init(|| {
        let app = app.to_owned();
        let temp = temp.to_owned();
        // A root-resolution failure is reported without selecting an alternate cache.
        let paths =
            requested_paths(&app, &temp, mode, portable).unwrap_or_else(|_| RuntimeStoragePaths {
                application_root: app.clone(),
                installation_id: String::new(),
                active_mode: mode,
                cache_root: if mode == CacheLocationMode::WindowsTemp {
                    PathBuf::new()
                } else {
                    app.join(portable)
                },
                state_root: app.join("state"),
                portable_root: app.join(portable),
                temporary_root: PathBuf::new(),
            });
        let result = (|| {
            let _ = resolve(&app, &temp, mode, portable)?;
            let output = Path::new(output);
            validate_output_for_paths(&paths, output)?;
            ensure_cache_owner(&paths)?;
            reject_links(&paths.state_root)?;
            let session = open_lock(&paths.state_root.join("runtime/storage-session.lock"))?;
            let migrated = super::storage_migration::is_complete(&paths)?;
            if !migrated {
                session.try_lock().map_err(|_| {
                    "Storage migration is busy. Close other application instances and restart."
                        .to_owned()
                })?;
                super::storage_migration::migrate(&paths)?;
                session
                    .unlock()
                    .map_err(|e| format!("Could not finish storage migration: {e}"))?;
            }
            session.try_lock_shared().map_err(|_| {
                "Storage initialization is busy. Restart after the other operation finishes."
                    .to_owned()
            })?;
            fs::create_dir_all(&paths.cache_root).map_err(|e| {
                format!(
                    "Could not create cache folder {}: {e}",
                    paths.cache_root.display()
                )
            })?;
            ensure_cache_owner(&paths)?;
            let probe = paths
                .cache_root
                .join(format!(".storage-probe-{}", std::process::id()));
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&probe)
                .map_err(|e| {
                    format!(
                        "Cache folder is not writable {}: {e}",
                        paths.cache_root.display()
                    )
                })?;
            drop(file);
            fs::remove_file(&probe)
                .map_err(|e| format!("Could not finish cache write probe: {e}"))?;
            Ok::<_, String>(session)
        })();
        match result {
            Ok(file) => RuntimeStorage {
                paths,
                error: None,
                _session: Some(file),
            },
            Err(error) => RuntimeStorage {
                paths,
                error: Some(error),
                _session: None,
            },
        }
    });
}

#[cfg(test)]
pub(crate) fn initialize_fixture(app: &Path, temp: &Path, mode: CacheLocationMode, portable: &str) {
    initialize_output_fixture(app, temp, mode, portable, &app.join("download"));
}

#[cfg(test)]
pub(crate) fn initialize_output_fixture(
    app: &Path,
    temp: &Path,
    mode: CacheLocationMode,
    portable: &str,
    output: &Path,
) {
    super::runtime_paths::set_fixture_root(app);
    initialize_at(app, temp, mode, portable, &output.display().to_string());
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;

pub(crate) fn paths() -> &'static RuntimeStoragePaths {
    &STORAGE
        .get()
        .expect("Storage must be initialized at application startup")
        .paths
}

pub(crate) fn error() -> Option<&'static str> {
    STORAGE.get().and_then(|storage| storage.error.as_deref())
}

pub(crate) fn require_ready() -> Result<&'static RuntimeStoragePaths, String> {
    let storage = STORAGE
        .get()
        .ok_or("Application storage has not been initialized")?;
    if let Some(error) = &storage.error {
        return Err(error.clone());
    }
    Ok(&storage.paths)
}

/// Each handle has an independent OS lock, including between threads/processes.
#[derive(Debug)]
pub(crate) struct StorageUseGuard {
    _file: File,
}

/// Callers hold a storage-use lease while creating or consuming this directory.
pub(crate) fn cache_directory(relative: &str) -> Result<PathBuf, String> {
    if !DISPOSABLE_DIRS.contains(&relative) {
        return Err("Unrecognized managed cache directory.".to_owned());
    }
    let path = require_ready()?.cache_root.join(relative);
    reject_links(&path)?;
    fs::create_dir_all(&path)
        .map_err(|error| format!("Could not create cache folder {}: {error}", path.display()))?;
    reject_links(&path)?;
    Ok(path)
}

/// Completed user outputs must never acquire disposable-cache ownership.
pub(crate) fn validate_output_destination(path: &Path) -> Result<(), String> {
    validate_output_for_paths(paths(), path)
}

/// Resolve the deepest existing ancestor so filesystem aliases are compared by
/// their normalized on-disk names, including when the final directory is absent.
fn ownership_path(path: &Path) -> Result<PathBuf, String> {
    #[cfg(windows)]
    for component in path.components() {
        match component {
            Component::Normal(name) if name.to_string_lossy().ends_with(['.', ' ']) => {
                return Err(format!(
                    "Ambiguous Windows output/storage path: {}",
                    path.display()
                ));
            }
            Component::Prefix(prefix) => {
                use std::path::Prefix;
                if !matches!(
                    prefix.kind(),
                    Prefix::Disk(_)
                        | Prefix::VerbatimDisk(_)
                        | Prefix::UNC(_, _)
                        | Prefix::VerbatimUNC(_, _)
                ) {
                    return Err(format!(
                        "Unsupported Windows output/storage path: {}",
                        path.display()
                    ));
                }
            }
            _ => {}
        }
    }
    reject_links(path)?;
    let mut suffix = Vec::new();
    let mut ancestor = path;
    loop {
        match ancestor.canonicalize() {
            Ok(mut resolved) => {
                for component in suffix.iter().rev() {
                    resolved.push(component);
                }
                return Ok(normalized(&resolved));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let component = ancestor.components().next_back().ok_or_else(|| {
                    format!("Could not resolve output/storage path: {}", path.display())
                })?;
                suffix.push(component.as_os_str().to_owned());
                ancestor = ancestor.parent().ok_or_else(|| {
                    format!("Could not resolve output/storage path: {}", path.display())
                })?;
            }
            Err(error) => {
                return Err(format!(
                    "Could not resolve output/storage path {}: {error}",
                    path.display()
                ));
            }
        }
    }
}

fn validate_output_for_paths(paths: &RuntimeStoragePaths, path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        && (path.has_root() || matches!(path.components().next(), Some(Component::Prefix(_))))
    {
        return Err("Choose an absolute output path or an application-relative folder.".to_owned());
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        paths.application_root.join(path)
    };
    let resolved = ownership_path(&absolute)?;
    let comparable = |value: &Path| {
        value
            .to_string_lossy()
            .replace('/', "\\")
            .trim_start_matches(r"\\?\")
            .to_lowercase()
    };
    let destination = comparable(&resolved);
    for root in [
        &paths.cache_root,
        &paths.portable_root,
        &paths.temporary_root,
    ] {
        if root.as_os_str().is_empty() {
            continue;
        }
        let root = comparable(&ownership_path(root)?);
        if destination == root || destination.starts_with(&(root + "\\")) {
            return Err(format!(
                "Completed downloads must be outside application cache folders: {}",
                absolute.display()
            ));
        }
    }
    Ok(())
}

/// Owns one operation's work files until its subprocess and output consumers finish.
pub(crate) struct CacheWorkDirectory {
    pub path: PathBuf,
    _lease: StorageUseGuard,
}

impl CacheWorkDirectory {
    pub(crate) fn new() -> Result<Self, String> {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let lease = use_cache()?;
        let parent = cache_directory("gallery-dl-temp")?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Could not determine gallery work timestamp.")?
            .as_nanos();
        for _ in 0..16 {
            let path = parent.join(format!(
                "{}-{stamp}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            reject_links(&path)?;
            match fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        _lease: lease,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!(
                        "Could not create gallery work folder {}: {error}",
                        path.display()
                    ));
                }
            }
        }
        Err("Could not allocate a unique gallery work folder.".to_owned())
    }
}

impl Drop for CacheWorkDirectory {
    fn drop(&mut self) {
        if let Err(error) = remove_owned_tree(&self.path) {
            eprintln!(
                "[gallery-dl] Could not remove work folder {}: {error}",
                self.path.display()
            );
        }
    }
}

pub(crate) fn use_cache_if_initialized() -> Result<Option<StorageUseGuard>, String> {
    if STORAGE.get().is_some() {
        use_cache().map(Some)
    } else {
        Ok(None)
    }
}

pub(crate) fn guard_for_path(path: &Path) -> Result<Option<StorageUseGuard>, String> {
    if let Some(storage) = STORAGE.get() {
        if path.starts_with(&storage.paths.state_root) {
            require_ready()?;
        }
        if path.starts_with(&storage.paths.cache_root) {
            reject_links(path)?;
            return use_cache().map(Some);
        }
    }
    Ok(None)
}

pub(crate) fn use_cache() -> Result<StorageUseGuard, String> {
    let paths = require_ready()?;
    validate_root(paths)?;
    let file = open_lock(&paths.state_root.join("runtime/cache-use.lock"))?;
    file.try_lock_shared()
        .map_err(|_| "Cache cleanup is in progress. Try again when it has finished.".to_owned())?;
    ensure_cache_owner(paths)?;
    Ok(StorageUseGuard { _file: file })
}

pub(crate) fn exclusive_cleanup() -> Result<StorageUseGuard, String> {
    let paths = require_ready()?;
    validate_root(paths)?;
    let file = open_lock(&paths.state_root.join("runtime/cache-use.lock"))?;
    file.try_lock()
        .map_err(|_| "Cache is in use. Try again when current tasks have finished.".to_owned())?;
    if super::component_update::other_registered_app_instances() > 0 {
        return Err("Cache is in use by another application instance.".to_owned());
    }
    if super::storage_migration::legacy_instance_busy(paths)? {
        return Err(
            "Cache may be in use by an older application instance. Close it before cleanup."
                .to_owned(),
        );
    }
    ensure_cache_owner(paths)?;
    Ok(StorageUseGuard { _file: file })
}

fn ensure_cache_owner(paths: &RuntimeStoragePaths) -> Result<(), String> {
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize)]
    struct Owner {
        application: String,
        installation_id: String,
    }
    let marker = paths.cache_root.join(".yt-dlp-gui-cache.yaml");
    reject_links(&marker)?;
    let mut replace = false;
    match fs::read_to_string(&marker) {
        Ok(text) => {
            let owner: Owner = serde_yaml::from_str(&text)
                .map_err(|e| format!("Invalid cache ownership record {}: {e}", marker.display()))?;
            if owner.application != "yt-dlp-gui" {
                return Err("Cache folder belongs to another application.".to_owned());
            }
            if owner.installation_id == paths.installation_id {
                return Ok(());
            }
            if !paths.cache_root.starts_with(&paths.application_root) {
                return Err("Cache folder belongs to a different installation. Choose another cache location.".to_owned());
            }
            replace = true;
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Could not read cache ownership record: {e}")),
    }
    let text = serde_yaml::to_string(&Owner {
        application: "yt-dlp-gui".to_owned(),
        installation_id: paths.installation_id.clone(),
    })
    .map_err(|e| e.to_string())?;
    if replace {
        super::yaml_store::atomic_write(&marker, text.as_bytes())
    } else {
        match super::yaml_store::atomic_write_new(&marker, text.as_bytes()) {
            Ok(()) => Ok(()),
            Err(_) if marker.exists() => {
                let current: Owner =
                    serde_yaml::from_str(&fs::read_to_string(&marker).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                if current.application == "yt-dlp-gui"
                    && current.installation_id == paths.installation_id
                {
                    Ok(())
                } else {
                    Err("Cache folder was claimed by another installation.".to_owned())
                }
            }
            Err(error) => Err(error),
        }
    }
}

pub(crate) fn cache_owned_by(root: &Path, installation_id: &str) -> Result<bool, String> {
    let marker = root.join(".yt-dlp-gui-cache.yaml");
    reject_links(&marker)?;
    let text = match fs::read_to_string(&marker) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "Could not inspect retained cache ownership: {error}"
            ));
        }
    };
    let owner: serde_yaml::Value = serde_yaml::from_str(&text)
        .map_err(|e| format!("Invalid retained cache ownership: {e}"))?;
    Ok(
        owner.get("application").and_then(serde_yaml::Value::as_str) == Some("yt-dlp-gui")
            && owner
                .get("installation_id")
                .and_then(serde_yaml::Value::as_str)
                == Some(installation_id),
    )
}

pub(crate) fn pending_root(mode: CacheLocationMode, portable: &str) -> Result<PathBuf, String> {
    let active = paths();
    let temp = active
        .temporary_root
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or("System temporary folder is unavailable")?;
    resolve(&active.application_root, temp, mode, portable).map(|paths| paths.cache_root)
}

pub(crate) fn state_path(relative: &str) -> PathBuf {
    paths().state_root.join(relative)
}

pub(crate) fn protected_update_paths() -> Vec<PathBuf> {
    super::component_update::pending_app_update_manifest()
        .filter(|m| m.state != super::component_update::PendingAppUpdateState::Applied)
        .map(|m| vec![m.staged_exe, m.backup_exe])
        .unwrap_or_default()
}

pub(crate) fn tree_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if is_link(&metadata) {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| tree_size(&e.path()))
                .sum()
        })
        .unwrap_or(0)
}

pub(crate) fn disposable_size(root: &Path) -> u64 {
    DISPOSABLE_DIRS
        .iter()
        .map(|name| tree_size(&root.join(name)))
        .sum()
}

pub(crate) fn clear_disposable(
    paths: &RuntimeStoragePaths,
    protected: &[PathBuf],
) -> Result<(u64, u64), String> {
    validate_root(paths)?;
    let mut count = 0;
    let mut bytes = 0;
    for relative in DISPOSABLE_DIRS {
        let child = paths.cache_root.join(relative);
        if *relative == "temp/manifest" && !protected.is_empty() {
            continue;
        }
        match fs::symlink_metadata(&child) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.to_string()),
            Ok(_) => {}
        }
        let size = tree_size(&child);
        remove_owned_tree(&child)?;
        count += 1;
        bytes += size;
    }
    Ok((count, bytes))
}

pub(crate) fn remove_owned_tree(path: &Path) -> Result<(), String> {
    reject_links(path)?;
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
    };
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            remove_owned_tree(&entry.map_err(|e| e.to_string())?.path())?;
        }
        fs::remove_dir(path).map_err(|e| e.to_string())
    } else {
        fs::remove_file(path).map_err(|e| e.to_string())
    }
}
