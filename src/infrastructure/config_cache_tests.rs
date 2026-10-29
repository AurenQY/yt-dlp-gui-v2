use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[test]
fn output_settings_field_fallback_preserves_unrelated_values() {
    let config = document::decode("storage:\n  download_dir: custom-output\n  output:\n    naming: wrong\n    existing_file: overwrite\n    template: '{unknown}'\nui:\n  language: en-US\n").unwrap();
    assert_eq!(config.download_dir, "custom-output");
    assert_eq!(
        config.output_preferences(),
        super::super::output_policy::OutputPreferences::default()
    );
    assert!(
        config
            .output_settings_error
            .unwrap()
            .contains("storage.output.template")
    );
    let config = document::decode("storage:\n  download_dir: kept\n  output: 123\n").unwrap();
    assert_eq!(config.download_dir, "kept");
    assert!(config.output_settings_error.is_some());
}

#[test]
fn output_settings_save_restart_preserves_latest_unrelated_settings() {
    use super::super::output_policy::*;
    let config = config_fixture("output-save");
    config.save().unwrap();
    let mut latest = config.clone();
    latest.download_dir = "changed-elsewhere".into();
    latest.component_update_cache = Some(serde_yaml::from_str("revision: 19").unwrap());
    latest.save().unwrap();
    let preferences = OutputPreferences {
        naming: OutputNamingMode::Custom,
        existing: ExistingFilePolicy::Skip,
        template: "{{{title}}} [{id}] 100%".into(),
    };
    let saved = config.with_saved_output(&preferences).unwrap();
    assert_eq!(config.output_preferences(), OutputPreferences::default());
    assert_eq!(saved.output_preferences(), preferences);
    let reloaded =
        document::decode(&fs::read_to_string(saved.config_path.as_ref().unwrap()).unwrap())
            .unwrap();
    assert_eq!(reloaded.output_preferences(), preferences);
    assert_eq!(reloaded.download_dir, "changed-elsewhere");
    assert_eq!(
        reloaded.component_update_cache,
        latest.component_update_cache
    );
}

#[cfg(windows)]
#[test]
fn output_settings_failed_save_preserves_saved_policy_and_draft() {
    use super::super::output_policy::*;
    use std::os::windows::fs::OpenOptionsExt;
    let config = config_fixture("output-failure");
    config.save().unwrap();
    let path = config.config_path.as_ref().unwrap();
    let before = fs::read(path).unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(path)
        .unwrap();
    let draft = OutputPreferences {
        naming: OutputNamingMode::TitleId,
        existing: ExistingFilePolicy::Skip,
        ..Default::default()
    };
    assert!(config.with_saved_output(&draft).is_err());
    assert_eq!(config.output_preferences(), OutputPreferences::default());
    assert_eq!(draft.naming, OutputNamingMode::TitleId);
    assert_eq!(fs::read(path).unwrap(), before);
    drop(lock);
    assert_eq!(
        config
            .with_saved_output(&draft)
            .unwrap()
            .output_preferences(),
        draft
    );
}

fn config_fixture(name: &str) -> AppConfig {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/cache-location-validation")
        .join(format!(
            "config-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    fs::create_dir_all(&root).unwrap();
    let mut config = AppConfig::default();
    config.config_path = Some(root.join("settings.yaml"));
    config
}

#[test]
fn storage_legacy_default_persists_canonical_mode_and_notice_is_not_repeated() {
    let mut config = document::decode("storage:\n  cache:\n    mode: yt_dlp_default\n").unwrap();
    config.normalize_cache_mode();
    assert!(config.legacy_cache_notice);
    assert_eq!(
        config.cache_location_mode,
        SerializableCacheLocationMode::V2Cache
    );
    let encoded = document::encode(&config).unwrap();
    let mut reloaded = document::decode(&encoded).unwrap();
    reloaded.normalize_cache_mode();
    assert!(!reloaded.legacy_cache_notice);
    assert_eq!(
        reloaded.cache_location_mode,
        SerializableCacheLocationMode::V2Cache
    );
}

#[test]
fn storage_saved_candidate_preserves_current_configuration_and_latest_history() {
    let config = config_fixture("save");
    config.save().unwrap();
    let history: serde_yaml::Value = serde_yaml::from_str("revision: 7\n").unwrap();
    let mut latest = config.clone();
    latest.component_update_cache = Some(history.clone());
    latest.save().unwrap();
    let candidate = config
        .with_saved_cache_location(CacheLocationMode::WindowsTemp)
        .unwrap();
    assert_eq!(
        config.cache_location_mode,
        SerializableCacheLocationMode::V2Cache
    );
    assert_eq!(
        candidate.cache_location_mode,
        SerializableCacheLocationMode::WindowsTemp
    );
    let saved =
        document::decode(&fs::read_to_string(config.config_path.unwrap()).unwrap()).unwrap();
    assert_eq!(
        saved.cache_location_mode,
        SerializableCacheLocationMode::WindowsTemp
    );
    assert_eq!(saved.component_update_cache, Some(history));
}

#[cfg(windows)]
#[test]
fn storage_save_failure_preserves_previous_preference_and_document() {
    let config = config_fixture("save-failure");
    config.save().unwrap();
    let path = config.config_path.as_ref().unwrap();
    let original = fs::read(path).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions).unwrap();
    assert!(
        config
            .with_saved_cache_location(CacheLocationMode::WindowsTemp)
            .is_err()
    );
    assert_eq!(
        config.cache_location_mode,
        SerializableCacheLocationMode::V2Cache
    );
    assert_eq!(fs::read(path).unwrap(), original);
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions).unwrap();
}
