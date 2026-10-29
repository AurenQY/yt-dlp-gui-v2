use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use serde::de::DeserializeOwned;

pub(crate) fn read_yaml_file<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    serde_yaml::from_str(&text).ok()
}

pub(crate) fn write_yaml_file<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let _storage_use = super::storage_paths::guard_for_path(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
    }
    let text = serde_yaml::to_string(value)
        .map_err(|error| format!("Could not serialize {}: {error}", path.display()))?;
    fs::write(path, text).map_err(|error| format!("Could not write {}: {error}", path.display()))
}

pub(crate) fn write_persistent_yaml<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let _storage_use = super::storage_paths::guard_for_path(path)?;
    let text = serde_yaml::to_string(value)
        .map_err(|e| format!("Could not serialize {}: {e}", path.display()))?;
    atomic_write(path, text.as_bytes())
}

/// Publish a complete document without truncating an existing destination on failure.
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    atomic_publish(path, bytes, true)
}

pub(crate) fn atomic_write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    atomic_publish(path, bytes, false)
}

fn atomic_publish(path: &Path, bytes: &[u8], replace: bool) -> Result<(), String> {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .ok_or("Storage document has no parent folder")?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    let temp = parent.join(format!(
        ".storage-{}-{}.tmp",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| format!("Could not create {}: {e}", temp.display()))?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| format!("Could not write {}: {e}", temp.display()))?;
        drop(file);
        publish(&temp, path, replace)
            .map_err(|e| format!("Could not publish {}: {e}", path.display()))
    })();
    let _ = fs::remove_file(&temp);
    result
}

#[cfg(not(windows))]
fn publish(source: &Path, target: &Path, replace: bool) -> std::io::Result<()> {
    if replace {
        fs::rename(source, target)
    } else {
        fs::hard_link(source, target)
    }
}

#[cfg(windows)]
fn publish(source: &Path, target: &Path, replace: bool) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<_> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(source: *const u16, target: *const u16, flags: u32) -> i32;
    }
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), 0x8 | u32::from(replace)) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
