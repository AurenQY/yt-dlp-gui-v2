//! Application-data queue storage. No cache/legacy lookup or migration.
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) fn queue_snapshot_path() -> PathBuf {
    super::runtime_paths::application_root()
        .join("data")
        .join("queue.yaml")
}

pub(crate) fn read_queue_snapshot<T: serde::de::DeserializeOwned>(
    path: &Path,
) -> Result<Option<T>, String> {
    match fs::read_to_string(path) {
        Ok(text) => serde_yaml::from_str(&text)
            .map(Some)
            .map_err(|_| format!("Invalid queue snapshot: {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "Could not read queue snapshot {}: {error}",
            path.display()
        )),
    }
}

pub(crate) fn write_queue_snapshot<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), String> {
    let text = serde_yaml::to_string(value)
        .map_err(|_| "Could not serialize queue snapshot".to_owned())?;
    write_queue_snapshot_text(path, &text)
}

pub(crate) fn write_queue_snapshot_text(path: &Path, text: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Queue snapshot has no parent directory")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create queue directory: {error}"))?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(".queue-{}-{sequence}.tmp", std::process::id()));
    let mut created = false;
    let result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        created = true;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        replace_snapshot(&temporary, path)
    })();
    if created && result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("Could not save queue snapshot {}: {error}", path.display()))
}

#[cfg(not(windows))]
fn replace_snapshot(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_snapshot(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(source: *const u16, destination: *const u16, flags: u32) -> i32;
    }
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // Same-directory replace; never delete the previous snapshot first.
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0x1 | 0x8) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
