//! Native gallery-dl ZIP output. A separate staging directory isolates the
//! post from existing downloads. XMP is embedded before this module packs media.
//! Only the completed ZIP is published to the V2 destination.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

pub(super) struct GalleryZipWorkspace {
    path: PathBuf,
}

impl GalleryZipWorkspace {
    pub(super) fn new(destination: &Path) -> Result<Self, String> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("Could not generate gallery ZIP workspace: {error}"))?
            .as_nanos();
        let path = destination.join(format!(
            ".yt-dlp-gui-gallery-zip-{}-{nonce}",
            std::process::id(),
        ));
        fs::create_dir(&path)
            .map_err(|error| format!("Could not create gallery ZIP workspace: {error}"))?;
        Ok(Self { path })
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for GalleryZipWorkspace {
    fn drop(&mut self) {
        // Only our own newly created staging directory can ever be removed.
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn is_incomplete_download(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.to_ascii_lowercase().ends_with(".part")
}

fn gallery_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![directory.to_path_buf()];
    let mut files = Vec::new();
    while let Some(parent) = pending.pop() {
        for entry in fs::read_dir(parent)
            .map_err(|error| format!("Could not list downloaded gallery files: {error}"))?
        {
            let entry =
                entry.map_err(|error| format!("Could not inspect gallery file: {error}"))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| format!("Could not inspect gallery file type: {error}"))?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file() && !is_incomplete_download(&path) {
                files.push(path);
            }
        }
    }
    files.sort();
    if files.is_empty() {
        return Err("gallery-dl did not download any media files to place in ZIP.".to_owned());
    }
    Ok(files)
}

fn safe_archive_name(title: &str) -> String {
    let name: String = title
        .chars()
        .filter(|c| !c.is_control() && !"\\/:*?\"<>|".contains(*c))
        .take(72)
        .collect();
    let name = name.trim().trim_matches('.');
    if name.is_empty() {
        "gallery".to_owned()
    } else {
        name.to_owned()
    }
}

pub(super) fn pack_gallery_directory(
    staging: &Path,
    destination: &Path,
    title: &str,
    cancelled: &AtomicBool,
) -> Result<PathBuf, String> {
    // Collect before opening the ZIP; do not accidentally archive the ZIP itself.
    let files = gallery_files(staging)?;
    let temporary_zip = staging.join("gallery-output.part.zip");
    let output = File::create(&temporary_zip)
        .map_err(|error| format!("Could not create gallery ZIP: {error}"))?;
    let mut zip = ZipWriter::new(output);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .large_file(true);
    for path in files {
        if cancelled.load(Ordering::Acquire) {
            return Err("Gallery ZIP creation was cancelled.".to_owned());
        }
        let relative = path
            .strip_prefix(staging)
            .map_err(|error| format!("Invalid gallery ZIP entry path: {error}"))?;
        let entry_name = relative.to_string_lossy().replace('\\', "/");
        zip.start_file(entry_name, options)
            .map_err(|error| format!("Could not add entry to gallery ZIP: {error}"))?;
        let mut source = File::open(&path)
            .map_err(|error| format!("Could not open gallery ZIP entry: {error}"))?;
        // Check cancellation throughout a large individual file, not only
        // between files. The ZIP writer and staging contents are discarded
        // on an error; no partial ZIP is published to the download folder.
        let mut buffer = [0u8; 128 * 1024];
        loop {
            if cancelled.load(Ordering::Acquire) {
                return Err("Gallery ZIP creation was cancelled.".to_owned());
            }
            let count = source
                .read(&mut buffer)
                .map_err(|error| format!("Could not read gallery ZIP entry: {error}"))?;
            if count == 0 {
                break;
            }
            zip.write_all(&buffer[..count])
                .map_err(|error| format!("Could not write gallery ZIP entry: {error}"))?;
        }
    }
    zip.finish()
        .map_err(|error| format!("Could not finish gallery ZIP: {error}"))?;
    if cancelled.load(Ordering::Acquire) {
        return Err("Gallery ZIP creation was cancelled.".to_owned());
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Could not generate gallery ZIP name: {error}"))?
        .as_nanos();
    let archive = destination.join(format!("{}-{nonce}.zip", safe_archive_name(title)));
    fs::rename(&temporary_zip, &archive).map_err(|error| {
        format!("Could not move completed gallery ZIP to download folder: {error}")
    })?;
    Ok(archive)
}
