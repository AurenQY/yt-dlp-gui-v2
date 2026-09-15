use super::super::{FormatPickerViewPreference, WindowPosition, WindowSize};
use super::*;
use std::collections::BTreeSet;

#[test]
fn defaults_and_empty_groups_produce_only_the_commented_empty_mapping() {
    let config = decode("interface: {}\nnetwork: {}\n").unwrap();
    assert_eq!(encode(&config).unwrap(), format!("{HEADER}{{}}\n"));
    assert_eq!(
        encode(&decode("").unwrap()).unwrap(),
        format!("{HEADER}{{}}\n")
    );
}

#[test]
fn graphics_comments_and_default_pruning_follow_present_branches() {
    let mut config = decode("interface:\n  graphics:\n    backend: wgpu\n").unwrap();
    let yaml = encode(&config).unwrap();
    assert!(yaml.contains("# Graphics settings. Changes take effect on the next launch."));
    assert!(yaml.contains("    backend: wgpu"));
    assert!(!yaml.contains("wgpu_api"));
    assert!(!yaml.contains("network:"));
    assert!(!yaml.contains("version:"));
    config.render_backend = Default::default();
    assert_eq!(encode(&config).unwrap(), format!("{HEADER}{{}}\n"));
}

#[test]
fn browser_integration_is_enabled_by_default_and_only_explicit_disable_is_saved() {
    let mut config = AppConfig::default();
    assert!(config.browser_integration_enabled);
    assert!(!encode(&config).unwrap().contains("integration:"));

    config.browser_integration_enabled = false;
    let yaml = encode(&config).unwrap();
    assert!(yaml.contains("# Connections to external applications."));
    assert!(yaml.contains("# Browser extension connection."));
    assert!(yaml.contains("# Automatically register and repair the browser extension connection."));
    assert!(yaml.contains("enabled: false"));
    assert!(!decode(&yaml).unwrap().browser_integration_enabled);
}

#[test]
fn disabled_features_keep_nondefault_preferences_and_strings_round_trip() {
    let mut config = AppConfig::default();
    config.proxy_url = "http://127.0.0.1:8080/#test".into();
    config.batch_limit_count = 27;
    config.wgpu_api = crate::infrastructure::wgpu_api::WgpuApi::Vulkan;
    for value in [
        "C:\\Custom tools\\yt-dlp.exe",
        "true",
        "null",
        "a: b # c",
        "first line\nsecond: line\n# literal comment\n",
        "line one\n\n",
        "single ' and double \" quotes",
    ] {
        config.youtube_extractor_args = value.into();
        let yaml = encode(&config).unwrap();
        let restored = decode(&yaml).unwrap();
        assert_eq!(restored.youtube_extractor_args, value);
        assert_eq!(restored.proxy_url, config.proxy_url);
        assert_eq!(restored.batch_limit_count, 27);
        assert_eq!(restored.wgpu_api, config.wgpu_api);
        assert!(!restored.proxy_enabled);
        assert!(!restored.batch_limit_enabled);
        assert!(!yaml.contains("enabled:"));
        assert!(!yaml.contains("backend:"));
        assert_eq!(encode(&restored).unwrap(), yaml);
    }
}

#[test]
fn standard_paths_are_sparse_and_custom_paths_are_retained() {
    let mut config = AppConfig::default();
    config.download_dir = ".\\download".into();
    config.cache_dir = ".\\cache".into();
    config.yt_dlp_path = DependencyTool::YtDlp.default_portable_path().into();
    config.ffmpeg_path = DependencyTool::Ffmpeg.default_portable_path().into();
    config.aria2c_path = DependencyTool::Aria2c.default_portable_path().into();
    config.deno_path = DependencyTool::Deno.default_portable_path().into();
    assert_eq!(encode(&config).unwrap(), format!("{HEADER}{{}}\n"));
    config.download_dir = "D:\\My Downloads".into();
    config.ffmpeg_path = "D:\\Custom\\ffmpeg.exe".into();
    let restored = decode(&encode(&config).unwrap()).unwrap();
    assert_eq!(restored.download_dir, config.download_dir);
    assert_eq!(restored.ffmpeg_path, config.ffmpeg_path);
}

fn collect_paths(value: &Value, prefix: &str, result: &mut BTreeSet<String>) {
    if let Some(mapping) = value.as_mapping() {
        for (key, child) in mapping {
            let key = key.as_str().unwrap();
            let path = if prefix.is_empty() {
                key.into()
            } else {
                format!("{prefix}.{key}")
            };
            // Positions and sizes are typed compound leaves in the persistence schema.
            if matches!(path.as_str(), "window_position" | "window_size") {
                result.insert(path);
            } else {
                collect_paths(child, &path, result);
            }
        }
    } else {
        result.insert(prefix.into());
    }
}

#[test]
fn schema_covers_every_persisted_field_once_including_optional_state() {
    let mut config = AppConfig::default();
    config.options_scroll_offset = 12.5;
    config.advance_scroll_offset = 30.0;
    config.format_picker_view_preference = Some(FormatPickerViewPreference::Table);
    config.window_position = WindowPosition::new(-100.0, 20.0);
    config.window_size = WindowSize::new(1000.0, 700.0);
    let mut paths = BTreeSet::new();
    collect_paths(&serde_yaml::to_value(&config).unwrap(), "", &mut paths);
    let schema_paths: BTreeSet<_> = schema::FIELDS.iter().map(|f| f.source.to_owned()).collect();
    assert_eq!(paths, schema_paths);
    assert_eq!(schema_paths.len(), schema::FIELDS.len());
    let disk_paths: BTreeSet<_> = schema::FIELDS.iter().map(|f| f.path).collect();
    assert_eq!(disk_paths.len(), schema::FIELDS.len());
    assert!(
        schema::FIELDS
            .iter()
            .all(|f| f.comment.is_ascii() && !f.comment.is_empty())
    );
}

#[test]
fn every_field_accepts_a_nondefault_value_and_survives_comment_serialization() {
    let defaults = serde_yaml::to_value(AppConfig::default()).unwrap();
    for field in schema::FIELDS {
        let old = read_path(&defaults, field.source).unwrap();
        let fixture = match field.source {
            "cache_location_mode" => "windows_temp",
            "file_time_mode" => "use_upload_date",
            "thumbnail_mode" | "subtitle_mode" | "chapter_mode" => "embed",
            "output_file_action_mode" => "open_file",
            "youtube_video_playlist_mode" => "ignore",
            "unknown_filesize_format_policy" => "filter",
            "transcode_intent.video_codec_policy" => "hevc",
            "transcode_intent.container_policy" => "mkv",
            "transcode_intent.audio_policy" => "flac",
            "transcode_intent.subtitle_policy" => "burn",
            "language" => "zh-TW",
            "theme_mode" => "dark",
            "theme_accent_color" => "green",
            "render_backend" => "wgpu",
            "wgpu_api" => "dx12",
            "format_picker_view_preference" => "table",
            "window_position" => "{x: -100.0, y: 25.0}",
            "window_size" => "{width: 1024.0, height: 768.0}",
            "options_scroll_offset" | "advance_scroll_offset" => "2.0",
            "app_mode" => "audio",
            "music_playback_mode" => "shuffle",
            _ => match old {
                Some(Value::Bool(true)) => "false",
                Some(Value::Bool(false)) => "true",
                Some(Value::Number(_)) => "2",
                Some(Value::String(_)) => "'custom value'",
                _ => panic!("Missing nondefault fixture for {}", field.source),
            },
        };
        let expected: Value = serde_yaml::from_str(fixture).unwrap();
        let mut document = Value::Mapping(Mapping::new());
        insert_path(&mut document, field.path, expected.clone());
        let config = decode(&serde_yaml::to_string(&document).unwrap()).unwrap();
        let yaml = encode(&config).unwrap();
        let saved: Value = serde_yaml::from_str(&yaml).unwrap();
        assert!(
            yaml.contains(&format!("# {}", field.comment)),
            "{}",
            field.path
        );
        assert!(
            read_path(&saved, field.path).unwrap().is_some(),
            "{}",
            field.path
        );
        let restored = decode(&yaml).unwrap();
        assert_eq!(
            read_path(&serde_yaml::to_value(&restored).unwrap(), field.source).unwrap(),
            read_path(&serde_yaml::to_value(&config).unwrap(), field.source).unwrap(),
            "{}",
            field.path
        );
    }
}

#[test]
fn save_always_writes_comments_and_removes_reset_settings() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("yt-dlp-config-{}-{nonce}.yaml", std::process::id()));
    let mut config = AppConfig::default();
    config.config_path = Some(path.clone());
    config.music_volume = 0.25;
    config.save().unwrap();
    let first = std::fs::read_to_string(&path).unwrap();
    assert!(first.contains("# Playback volume from 0.0 to 1.0."));
    assert_eq!(decode(&first).unwrap().music_volume, 0.25);
    config.music_volume = 0.5;
    config.save().unwrap();
    let second = std::fs::read_to_string(&path).unwrap();
    assert!(second.contains("# Playback volume from 0.0 to 1.0."));
    assert_eq!(decode(&second).unwrap().music_volume, 0.5);
    let (loaded, _) = AppConfig::load_from_path(path.clone());
    assert_eq!(loaded.music_volume, 0.5);
    let startup_saved = std::fs::read_to_string(&path).unwrap();
    assert!(startup_saved.contains("# Playback volume from 0.0 to 1.0."));
    assert!(!startup_saved.contains("download_dir:"));
    assert!(!startup_saved.contains("cache:"));
    config.music_volume = AppConfig::default().music_volume;
    config.save().unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        format!("{HEADER}{{}}\n")
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn flat_legacy_keys_are_not_loaded_and_invalid_group_shapes_fail() {
    let config = decode("render_backend: wgpu\nmusic_volume: 0.1\n").unwrap();
    assert_eq!(config.render_backend, AppConfig::default().render_backend);
    assert_eq!(config.music_volume, AppConfig::default().music_volume);
    assert!(decode("interface: invalid").is_err());
    assert!(decode("player:\n  volume: invalid").is_err());
    assert!(decode("- invalid").is_err());
}
