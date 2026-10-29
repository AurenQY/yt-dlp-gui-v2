//! Keeps About version history in the same YAML as the running executable.
//! Both the settings UI and update worker use one lock so a stale AppConfig
//! save cannot erase a just-completed background version check.
use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

use serde_yaml::Value;

use super::{AppConfig, config_file_path, document};

fn config_file_mutex() -> &'static Mutex<()> {
    static CONFIG_FILE_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    CONFIG_FILE_MUTEX.get_or_init(|| Mutex::new(()))
}

pub(super) fn lock_config_file() -> MutexGuard<'static, ()> {
    config_file_mutex()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn read_config_from_path(path: &Path) -> Result<Option<AppConfig>, String> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Could not read {}: {error}", path.display())),
    };
    document::decode(&content)
        .map(Some)
        .map_err(|error| format!("Could not parse {}: {error}", path.display()))
}

pub(super) fn read_component_update_cache_from_path(path: &Path) -> Result<Option<Value>, String> {
    Ok(read_config_from_path(path)?.and_then(|config| config.component_update_cache))
}

pub(crate) fn read_runtime_component_update_cache() -> Option<Value> {
    let _guard = lock_config_file();
    read_component_update_cache_from_path(&config_file_path())
        .ok()
        .flatten()
}

pub(crate) fn write_runtime_component_update_cache(cache: &Value) -> Result<(), String> {
    write_component_update_cache_to_path(&config_file_path(), cache)
}

fn write_component_update_cache_to_path(path: &Path, cache: &Value) -> Result<(), String> {
    let _guard = lock_config_file();
    let mut config = read_config_from_path(path)?.unwrap_or_default();
    config.component_update_cache = Some(cache.clone());
    let document = document::encode(&config)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
    }
    super::super::yaml_store::atomic_write(path, document.as_bytes())
}
