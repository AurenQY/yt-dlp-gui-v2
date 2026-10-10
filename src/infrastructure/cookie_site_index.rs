use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::yaml_store::read_yaml_file;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct CookieSiteIndex {
    #[serde(default)]
    pub(crate) sites: Vec<CookieSiteIndexEntry>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct CookieSiteIndexEntry {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) display_name: String,
    #[serde(default)]
    pub(crate) login_url: String,
    #[serde(default)]
    pub(crate) match_domains: Vec<String>,
    #[serde(default)]
    pub(crate) cookie_domains: Vec<String>,
    #[serde(default)]
    pub(crate) cookie_file: String,
    #[serde(default)]
    pub(crate) builtin: bool,
    #[serde(default)]
    pub(crate) updated_unix: u64,
    #[serde(default)]
    pub(crate) auto_update_from_extension: bool,
    #[serde(default)]
    pub(crate) last_browser_snapshot_unix_ms: u64,
}

pub(crate) fn cookie_site_index_path(cookie_dir: &Path) -> PathBuf {
    cookie_dir.join("sites.yaml")
}

pub(crate) fn read_cookie_site_index(cookie_dir: &Path) -> Result<Option<CookieSiteIndex>, String> {
    let path = cookie_site_index_path(cookie_dir);
    if !path.is_file() {
        return Ok(None);
    }
    read_yaml_file(&path)
        .ok_or_else(|| format!("Could not parse cookie site index: {}", path.display()))
        .map(Some)
}

pub(crate) fn read_cookie_site_index_or_default(cookie_dir: &Path) -> CookieSiteIndex {
    read_yaml_file(&cookie_site_index_path(cookie_dir)).unwrap_or_default()
}

pub(crate) fn write_cookie_site_index(
    cookie_dir: &Path,
    index: &CookieSiteIndex,
) -> Result<(), String> {
    let text = serde_yaml::to_string(index)
        .map_err(|_| "Could not serialize cookie site index.".to_owned())?;
    write_cookie_store_file_atomically(&cookie_site_index_path(cookie_dir), text.as_bytes())
}

/// Hold across the entire read/modify/write, including the Cookie file.
/// Every writer uses the same cross-process lock; closing the File releases it.
pub(crate) fn lock_cookie_store(cookie_dir: &Path) -> Result<File, String> {
    fs::create_dir_all(cookie_dir).map_err(|_| "Could not create cookie directory.".to_owned())?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(cookie_dir.join(".cookie-store.lock"))
        .map_err(|_| "Could not open cookie store lock.".to_owned())?;
    lock.lock()
        .map_err(|_| "Could not lock cookie store.".to_owned())?;
    Ok(lock)
}

/// Stage on the same volume, then replace without deleting the last good file.
pub(crate) fn write_cookie_store_file_atomically(
    target: &Path,
    bytes: &[u8],
) -> Result<(), String> {
    let parent = target.parent().ok_or("Cookie save target is invalid.")?;
    fs::create_dir_all(parent).map_err(|_| "Could not create cookie directory.")?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "Clock unavailable.")?
        .as_nanos();
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Cookie save target is invalid.")?;
    let temporary = parent.join(format!(".{name}.{}-{stamp}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "Could not stage saved cookies.")?;
    let result = (|| {
        file.write_all(bytes)
            .map_err(|_| "Could not write saved cookies.")?;
        file.sync_all()
            .map_err(|_| "Could not flush saved cookies.")?;
        drop(file);
        fs::rename(&temporary, target).map_err(|_| "Could not replace saved cookies.")
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(str::to_owned)
}
