//! Native-only safe image replacement. Never truncate/delete the original first.
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn replace_image(path: &Path, original: &[u8], output: &[u8]) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.permissions().readonly()
    {
        return Err("XMP write needs a writable regular file; original left unchanged".into());
    }
    let parent = path.parent().ok_or("Image has no parent folder")?;
    let mut temporary = None;
    // create_new also avoids touching a stale temporary left by an earlier process.
    for _ in 0..16 {
        let candidate = parent.join(format!(
            ".gallery-xmp-{}-{}.tmp",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Could not stage XMP: {error}")),
        }
    }
    let (temporary, mut file) = temporary.ok_or("Could not allocate XMP temporary file")?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(output)?;
        file.set_permissions(metadata.permissions())?;
        let times = fs::FileTimes::new().set_modified(metadata.modified()?);
        let times = if let Ok(accessed) = metadata.accessed() {
            times.set_accessed(accessed)
        } else {
            times
        };
        file.set_times(times)?;
        file.sync_all()?;
        drop(file);
        if fs::read(path)? != original {
            return Err(std::io::Error::other("Image changed during XMP processing"));
        }
        replace(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| {
        format!(
            "Could not safely write image XMP {}: {error}",
            path.display()
        )
    })
}
#[cfg(not(windows))]
fn replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}
#[cfg(windows)]
fn replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn ReplaceFileW(
            replaced: *const u16,
            replacement: *const u16,
            backup: *const u16,
            flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
    }
    let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<_> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // ReplaceFile preserves the target's creation time/security/alternate streams.
    // Do not fall back to delete+rename when sharing/ACL/filesystem constraints reject it.
    if unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            source.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
