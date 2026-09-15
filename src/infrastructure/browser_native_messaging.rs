//! Chromium Native Messaging boundary for the browser extension.
//!
//! The browser process is intentionally short lived: it loads the current sparse
//! configuration, handles one analysis or download operation, streams framed
//! events, then exits. Analysis and download accept one follow-up cancellation frame. The
//! regular GUI never needs to be running.

use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use super::{
    AppConfig, DownloadRequest, DownloadTargetKind, FINAL_OUTPUT_PATH_PREFIX, PostProcessMode,
    ToolPaths, open_output_folder, resolve_output_dir,
};
use crate::domain::DownloadTimeRange;

pub const HOST_NAME: &str = "kannagi0303.yt_dlp_gui";
const HOST_NAMES: [&str; 1] = [HOST_NAME];
pub const EXTENSION_ID: &str = "jnhdidpednnfdphbcbeignbhceemminn";
const EXTENSION_ORIGIN: &str = "chrome-extension://jnhdidpednnfdphbcbeignbhceemminn/";
const MAX_REQUEST_BYTES: usize = 512 * 1024;
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_URL_BYTES: usize = 8192;
const DEFAULT_MULTI_ITEMS: usize = 32;
const MAX_MULTI_ITEMS: usize = 100;
const MAX_FORMATS_PER_ITEM: usize = 300;
const MAX_SUBTITLES_PER_ITEM: usize = 100;
const MAX_CHAPTERS_PER_ITEM: usize = 300;
const BROWSER_YOUTUBE_PLAYER_CLIENTS: &str = "player_client=default,android_vr";
const NATIVE_WIRE_VERSION: u32 = 1;
const NATIVE_CAPABILITIES: &[&str] = &[
    "fields.ignore-unknown.v1",
    "events.ignore-unknown.v1",
    "analysis.stream.v1",
    "analysis.cancel.v1",
    "analysis.playlist-limit.v1",
    "download.stream.v1",
    "download.cancel.v1",
    "download.custom-filename.v1",
    "download.time-range.v1",
    "download.chapter-range.v1",
    "download.aria2-choice.v1",
    "cookies.browser-transfer.v1",
    "formats.explicit-pair.v1",
    "postprocess.subtitle-chapter.v1",
    "postprocess.subtitle-mode.v1",
    "postprocess.thumbnail-mode.v1",
    "postprocess.chapter-mode.v1",
    "directory.choose.v1",
    "directory.open.v1",
];

const BROWSERS: [(&str, &str); 3] = [
    ("chrome", r"Google\Chrome"),
    ("brave", r"BraveSoftware\Brave-Browser"),
    ("edge", r"Microsoft\Edge"),
];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserIntegrationStatus {
    pub enabled: bool,
    pub chrome_registered: bool,
    pub brave_registered: bool,
    pub edge_registered: bool,
    pub extension_id: &'static str,
    pub manifest_path: String,
}

#[derive(Serialize)]
struct NativeHostManifest<'a> {
    name: &'a str,
    description: &'a str,
    path: String,
    #[serde(rename = "type")]
    host_type: &'a str,
    allowed_origins: [&'a str; 1],
}

#[derive(Serialize)]
struct NativeResponse<'a> {
    ok: bool,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

fn host_protocol_info() -> Value {
    json!({
        "wireVersion": NATIVE_WIRE_VERSION,
        "capabilities": NATIVE_CAPABILITIES,
        "limits": {
            "requestBytes": MAX_REQUEST_BYTES,
            "responseBytes": MAX_RESPONSE_BYTES,
            "urlBytes": MAX_URL_BYTES,
            "playlistItems": MAX_MULTI_ITEMS,
            "formatsPerItem": MAX_FORMATS_PER_ITEM,
            "subtitlesPerItem": MAX_SUBTITLES_PER_ITEM,
            "chaptersPerItem": MAX_CHAPTERS_PER_ITEM
        }
    })
}

fn integration_dir() -> Result<PathBuf, String> {
    let local = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "LOCALAPPDATA is unavailable.".to_owned())?;
    Ok(PathBuf::from(local)
        .join("yt-dlp-gui-v2")
        .join("NativeMessaging"))
}

fn manifest_path(host_name: &str) -> Result<PathBuf, String> {
    Ok(integration_dir()?.join(format!("{host_name}.json")))
}

fn current_executable() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|error| format!("Could not locate the application: {error}"))
}

#[cfg(windows)]
fn registry_key(browser: &str, host_name: &str) -> String {
    format!(r"Software\{browser}\NativeMessagingHosts\{host_name}")
}

#[cfg(windows)]
fn registered_manifest(browser: &str, host_name: &str) -> Option<PathBuf> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(registry_key(browser, host_name))
        .ok()?
        .get_value::<String, _>("")
        .ok()
        .map(PathBuf::from)
}

#[cfg(not(windows))]
fn registered_manifest(_browser: &str, _host_name: &str) -> Option<PathBuf> {
    None
}

#[cfg(windows)]
fn set_registered_manifest(browser: &str, host_name: &str, path: &Path) -> Result<(), String> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(registry_key(browser, host_name))
        .map_err(|error| format!("Could not create the {browser} Native Host key: {error}"))?;
    key.set_value("", &path.to_string_lossy().as_ref())
        .map_err(|error| format!("Could not register the {browser} Native Host: {error}"))
}

#[cfg(windows)]
fn remove_registered_manifest(browser: &str, host_name: &str) -> Result<(), String> {
    use std::io::ErrorKind;
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(registry_key(browser, host_name)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "Could not remove the {browser} Native Host: {error}"
        )),
    }
}

fn manifest_is_current(path: &Path, executable: &Path, host_name: &str) -> bool {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .is_some_and(|value| {
            value.get("name").and_then(Value::as_str) == Some(host_name)
                && value.get("type").and_then(Value::as_str) == Some("stdio")
                && value
                    .get("path")
                    .and_then(Value::as_str)
                    .is_some_and(|path| PathBuf::from(path) == executable)
                && value
                    .get("allowed_origins")
                    .and_then(Value::as_array)
                    .is_some_and(|origins| {
                        origins.len() == 1 && origins[0].as_str() == Some(EXTENSION_ORIGIN)
                    })
        })
}

pub fn browser_integration_status() -> Result<BrowserIntegrationStatus, String> {
    let manifest = manifest_path(HOST_NAME)?;
    let executable = current_executable()?;
    let registered = BROWSERS.map(|(_, browser)| {
        HOST_NAMES.iter().all(|host_name| {
            let Ok(path) = manifest_path(host_name) else {
                return false;
            };
            registered_manifest(browser, host_name).as_deref() == Some(path.as_path())
        })
    });
    let manifest_valid = HOST_NAMES.iter().all(|host_name| {
        manifest_path(host_name)
            .is_ok_and(|path| manifest_is_current(&path, &executable, host_name))
    });
    Ok(BrowserIntegrationStatus {
        enabled: manifest_valid && registered.iter().all(|value| *value),
        chrome_registered: registered[0],
        brave_registered: registered[1],
        edge_registered: registered[2],
        extension_id: EXTENSION_ID,
        manifest_path: manifest.display().to_string(),
    })
}

#[cfg(windows)]
pub fn synchronize_browser_integration(
    preferred_enabled: bool,
) -> Result<BrowserIntegrationStatus, String> {
    let status = browser_integration_status()?;
    if preferred_enabled {
        return if status.enabled {
            Ok(status)
        } else {
            enable_browser_integration()
        };
    }

    let has_registration = BROWSERS.iter().any(|(_, browser)| {
        HOST_NAMES
            .iter()
            .any(|host_name| registered_manifest(browser, host_name).is_some())
    });
    let has_manifest = HOST_NAMES
        .iter()
        .any(|host_name| manifest_path(host_name).is_ok_and(|path| path.exists()));
    if has_registration || has_manifest {
        disable_browser_integration()
    } else {
        Ok(status)
    }
}

#[cfg(not(windows))]
pub fn synchronize_browser_integration(
    _preferred_enabled: bool,
) -> Result<BrowserIntegrationStatus, String> {
    Err("Browser integration currently supports Windows only.".to_owned())
}

#[cfg(windows)]
pub fn enable_browser_integration() -> Result<BrowserIntegrationStatus, String> {
    let executable = current_executable()?;
    for host_name in HOST_NAMES {
        let path = manifest_path(host_name)?;
        fs::create_dir_all(path.parent().expect("manifest has parent"))
            .map_err(|error| format!("Could not create the Native Host folder: {error}"))?;
        let manifest = NativeHostManifest {
            name: host_name,
            description: "yt-dlp GUI browser integration",
            path: executable.display().to_string(),
            host_type: "stdio",
            allowed_origins: [EXTENSION_ORIGIN],
        };
        let mut bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| format!("Could not serialize the Native Host manifest: {error}"))?;
        bytes.push(b'\n');
        fs::write(&path, bytes)
            .map_err(|error| format!("Could not write the Native Host manifest: {error}"))?;
        for (_, browser) in BROWSERS {
            set_registered_manifest(browser, host_name, &path)?;
        }
    }
    browser_integration_status()
}

#[cfg(not(windows))]
pub fn enable_browser_integration() -> Result<BrowserIntegrationStatus, String> {
    Err("Browser integration currently supports Windows only.".to_owned())
}

#[cfg(windows)]
pub fn disable_browser_integration() -> Result<BrowserIntegrationStatus, String> {
    for host_name in HOST_NAMES {
        for (_, browser) in BROWSERS {
            remove_registered_manifest(browser, host_name)?;
        }
        match fs::remove_file(manifest_path(host_name)?) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Could not remove the Native Host manifest: {error}"
                ));
            }
        }
    }
    browser_integration_status()
}

#[cfg(not(windows))]
pub fn disable_browser_integration() -> Result<BrowserIntegrationStatus, String> {
    Err("Browser integration currently supports Windows only.".to_owned())
}

fn validated_url(value: &Value) -> Result<&str, String> {
    let url = value
        .as_str()
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .ok_or_else(|| "The request has no URL.".to_owned())?;
    let authority = url
        .split_once("://")
        .map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or_default())
        .unwrap_or_default();
    if url.len() > MAX_URL_BYTES
        || !(url.starts_with("https://") || url.starts_with("http://"))
        || url.chars().any(|ch| ch.is_control() || ch.is_whitespace())
        || authority.is_empty()
        || authority.contains('@')
    {
        return Err("The request URL is invalid.".to_owned());
    }
    Ok(url)
}

fn apply_browser_youtube_format_coverage(
    tools: &mut ToolPaths,
    url: &str,
    youtube_authenticated: bool,
    allow_supplemental_client: bool,
) {
    let authority = url
        .split_once("://")
        .map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or_default())
        .unwrap_or_default();
    let host = authority
        .split(':')
        .next()
        .unwrap_or_default()
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    let is_youtube = host == "youtube.com" || host.ends_with(".youtube.com") || host == "youtu.be";
    if allow_supplemental_client
        && is_youtube
        && !youtube_authenticated
        && tools.youtube_extractor_args.trim().is_empty()
    {
        tools.youtube_extractor_args = BROWSER_YOUTUBE_PLAYER_CLIENTS.to_owned();
    }
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_owned()
}

fn format_audio_presence(format: &Value) -> &'static str {
    match format
        .get("acodec")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|codec| !codec.is_empty())
    {
        Some("none") => "absent",
        Some(_) => "present",
        None => {
            let has_audio_metadata = format
                .get("audio_ext")
                .and_then(Value::as_str)
                .is_some_and(|ext| !ext.is_empty() && ext != "none")
                || format
                    .get("asr")
                    .and_then(Value::as_f64)
                    .is_some_and(|rate| rate > 0.0)
                || format
                    .get("audio_channels")
                    .and_then(Value::as_u64)
                    .is_some_and(|channels| channels > 0);
            if has_audio_metadata {
                "present"
            } else {
                "unknown"
            }
        }
    }
}

fn format_kind(format: &Value) -> Option<&'static str> {
    let video = format
        .get("vcodec")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|codec| !codec.is_empty())
        .map(|codec| codec != "none")
        .unwrap_or_else(|| {
            format
                .get("video_ext")
                .and_then(Value::as_str)
                .is_some_and(|ext| !ext.is_empty() && ext != "none")
                || format.get("width").and_then(Value::as_u64).is_some()
                || format.get("height").and_then(Value::as_u64).is_some()
        });
    let audio = format_audio_presence(format) == "present";
    match (video, audio) {
        (true, true) => Some("muxed"),
        (true, false) => Some("video"),
        (false, true) => Some("audio"),
        _ => None,
    }
}

fn compact_formats(value: &Value) -> Vec<Value> {
    value
        .get("formats")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|format| {
            let kind = format_kind(format)?;
            let audio_presence = format_audio_presence(format);
            let id = format.get("format_id").and_then(Value::as_str)?;
            let width = format.get("width").and_then(Value::as_u64);
            let height = format.get("height").and_then(Value::as_u64);
            let resolution = match (width, height) {
                (Some(width), Some(height)) => format!("{width}x{height}"),
                _ => string_field(format, "resolution"),
            };
            let ext = format
                .get(if kind == "audio" {
                    "audio_ext"
                } else {
                    "video_ext"
                })
                .or_else(|| format.get("ext"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let codec = format
                .get(if kind == "audio" { "acodec" } else { "vcodec" })
                .and_then(Value::as_str)
                .unwrap_or_default();
            let audio_codec = format
                .get("acodec")
                .and_then(Value::as_str)
                .filter(|codec| *codec != "none")
                .unwrap_or_default();
            let audio_ext = format
                .get("audio_ext")
                .and_then(Value::as_str)
                .filter(|ext| *ext != "none")
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    if audio_codec.starts_with("mp4a") || audio_codec == "aac" {
                        "m4a".to_owned()
                    } else if audio_codec == "opus" {
                        "opus".to_owned()
                    } else if audio_codec == "vorbis" {
                        "ogg".to_owned()
                    } else if audio_codec == "mp3" {
                        "mp3".to_owned()
                    } else {
                        String::new()
                    }
                });
            let exact_filesize = format.get("filesize").and_then(json_number_as_u64);
            let approximate_filesize = format.get("filesize_approx").and_then(json_number_as_u64);
            let filesize = exact_filesize.or(approximate_filesize);
            let bitrate = if kind == "audio" {
                format.get("abr").and_then(Value::as_f64)
            } else {
                format
                    .get("vbr")
                    .and_then(Value::as_f64)
                    .or_else(|| format.get("tbr").and_then(Value::as_f64))
            };
            Some(json!({
                "id": id, "kind": kind, "resolution": resolution,
                "fps": format.get("fps").and_then(Value::as_f64),
                "ext": ext, "codec": codec, "audioCodec": audio_codec, "audioExt": audio_ext,
                "language": string_field(format, "language"),
                "audioPresence": audio_presence,
                "sampleRate": format.get("asr").and_then(Value::as_f64),
                "bitrate": bitrate,
                "filesize": filesize,
                "filesizeApproximate": exact_filesize.is_none() && approximate_filesize.is_some()
            }))
        })
        .take(MAX_FORMATS_PER_ITEM)
        .collect()
}

fn json_number_as_u64(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        value
            .as_f64()
            .filter(|number| number.is_finite() && *number > 0.0 && *number <= u64::MAX as f64)
            .map(|number| number.round() as u64)
    })
}

fn compact_subtitles(value: &Value) -> Vec<Value> {
    let mut result = Vec::new();
    for (key, automatic) in [("subtitles", false), ("automatic_captions", true)] {
        let Some(languages) = value.get(key).and_then(Value::as_object) else {
            continue;
        };
        for (language, tracks) in languages {
            if language.eq_ignore_ascii_case("live_chat") {
                continue;
            }
            if result
                .iter()
                .any(|item: &Value| item["language"].as_str() == Some(language.as_str()))
            {
                continue;
            }
            let label = tracks
                .as_array()
                .and_then(|tracks| tracks.first())
                .and_then(|track| track.get("name"))
                .and_then(Value::as_str)
                .filter(|label| !label.trim().is_empty())
                .unwrap_or(language);
            result.push(json!({
                "language": language,
                "label": label,
                "automatic": automatic
            }));
            if result.len() >= MAX_SUBTITLES_PER_ITEM {
                return result;
            }
        }
    }
    result
}

fn non_negative_seconds_as_millis(value: Option<&Value>) -> Option<u64> {
    value
        .and_then(Value::as_f64)
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
        .map(|seconds| (seconds * 1_000.0).round() as u64)
}

fn compact_chapters(value: &Value) -> Vec<Value> {
    let Some(raw_chapters) = value.get("chapters").and_then(Value::as_array) else {
        return Vec::new();
    };
    let media_end = non_negative_seconds_as_millis(value.get("duration"));
    raw_chapters
        .iter()
        .enumerate()
        .filter_map(|(index, chapter)| {
            let start_millis = non_negative_seconds_as_millis(chapter.get("start_time"))?;
            let next_start = raw_chapters
                .get(index + 1)
                .and_then(|next| non_negative_seconds_as_millis(next.get("start_time")));
            let end_millis = non_negative_seconds_as_millis(chapter.get("end_time"))
                .or(next_start)
                .or(media_end)
                .filter(|end| *end > start_millis)?;
            let title = string_field(chapter, "title")
                .chars()
                .take(300)
                .collect::<String>();
            Some(json!({
                "title": title,
                "startMillis": start_millis,
                "endMillis": end_millis
            }))
        })
        .take(MAX_CHAPTERS_PER_ITEM)
        .collect()
}

fn compact_item(value: &Value, fallback_url: &str) -> Value {
    let formats = compact_formats(value);
    let subtitles = compact_subtitles(value);
    let chapters = compact_chapters(value);
    let chapter_count = value
        .get("chapters")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let requested = value
        .get("requested_downloads")
        .or_else(|| value.get("requested_formats"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("format_id").and_then(Value::as_str))
        .collect::<Vec<_>>();
    let default_video = requested
        .iter()
        .find(|id| {
            formats
                .iter()
                .any(|format| format["id"] == **id && format["kind"] != "audio")
        })
        .copied()
        .or_else(|| {
            formats
                .iter()
                .rev()
                .find(|f| f["kind"] == "video" || f["kind"] == "muxed")
                .and_then(|f| f["id"].as_str())
        })
        .unwrap_or_default();
    let video_is_muxed = formats
        .iter()
        .find(|f| f["id"] == default_video)
        .is_some_and(|f| f["kind"] == "muxed");
    let default_audio = if video_is_muxed {
        default_video
    } else {
        requested
            .iter()
            .find(|id| {
                formats
                    .iter()
                    .any(|format| format["id"] == **id && format["kind"] == "audio")
            })
            .copied()
            .or_else(|| {
                formats
                    .iter()
                    .rev()
                    .find(|f| f["kind"] == "audio")
                    .and_then(|f| f["id"].as_str())
            })
            .unwrap_or_default()
    };
    let creator = [
        string_field(value, "channel"),
        string_field(value, "uploader"),
        string_field(value, "creator"),
    ]
    .into_iter()
    .find(|text| !text.is_empty())
    .unwrap_or_default();
    json!({
        "id": string_field(value, "id"),
        "url": value.get("webpage_url").and_then(Value::as_str).unwrap_or(fallback_url),
        "title": string_field(value, "title"),
        "creator": creator,
        "duration": string_field(value, "duration_string"),
        "thumbnail": string_field(value, "thumbnail"),
        "formats": formats,
        "subtitles": subtitles,
        "chapters": chapters,
        "chapterCount": chapter_count,
        "defaultVideo": default_video,
        "defaultAudio": default_audio,
        "videoIsMuxed": video_is_muxed
    })
}

fn browser_analysis_item_limit(request: &Value) -> Result<usize, String> {
    match request.get("itemLimit") {
        None | Some(Value::Null) => Ok(DEFAULT_MULTI_ITEMS),
        Some(value) => {
            let limit = value
                .as_u64()
                .ok_or_else(|| "The analysis item limit must be an integer.".to_owned())?
                as usize;
            if !(1..=MAX_MULTI_ITEMS).contains(&limit) {
                return Err(format!(
                    "The analysis item limit must be between 1 and {MAX_MULTI_ITEMS}."
                ));
            }
            Ok(limit)
        }
    }
}

fn playlist_analysis_is_truncated(value: &Value, playlist_scope: &str, item_limit: usize) -> bool {
    let reported_count = value
        .get("playlist_count")
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    let returned_count = value
        .get("entries")
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter(|entry| !entry.is_null()).count())
        .unwrap_or_default();
    playlist_scope == "playlist" && (reported_count > item_limit || returned_count > item_limit)
}

fn analyze(request: &Value) -> Result<Value, String> {
    let url = validated_url(request.get("url").unwrap_or(&Value::Null))?;
    let playlist_scope = browser_playlist_scope(request)?;
    let item_limit = browser_analysis_item_limit(request)?;
    let (config, mut tools) = AppConfig::load_runtime();
    let cookies = super::browser_native_cookies::NativeCookies::from_request(request)?;
    let youtube_authenticated = cookies
        .as_ref()
        .is_some_and(|cookies| cookies.youtube_authenticated());
    apply_browser_youtube_format_coverage(
        &mut tools,
        url,
        youtube_authenticated,
        playlist_scope != "playlist",
    );
    tools.native_cookie_override = cookies.as_ref().map(|cookies| cookies.path());
    tools.validate_cookie_setup(config.use_browser_cookies)?;
    let output = if playlist_scope == "playlist" {
        tools.analyze_playlist_url_detailed(url, config.use_browser_cookies, item_limit + 1)
    } else {
        tools.analyze_url_detailed(url, config.use_browser_cookies)
    }
    .map_err(|error| error.message)?;
    let truncated = playlist_analysis_is_truncated(&output.json, playlist_scope, item_limit);
    let entries = output
        .json
        .get("entries")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| !entry.is_null())
                .take(item_limit)
                .map(|entry| compact_item(entry, url))
                .collect::<Vec<_>>()
        })
        .filter(|entries| !entries.is_empty());
    let items = entries.unwrap_or_else(|| vec![compact_item(&output.json, url)]);
    let output_directory = configured_download_directory(&config)?;
    Ok(json!({
        "items": items,
        "itemLimit": item_limit,
        "truncated": truncated,
        "outputDirectory": output_directory.to_string_lossy(),
        "thumbnailMode": config.thumbnail_mode,
        "subtitleMode": config.subtitle_mode,
        "chapterMode": config.chapter_mode,
        "useAria2": config.use_aria2,
        "aria2Available": tools.aria2_is_available()
    }))
}

fn stream_analysis(
    request: &Value,
    writer: &mut impl Write,
    cancelled: &AtomicBool,
) -> Result<Value, String> {
    write_response(
        &mut *writer,
        &NativeResponse {
            ok: true,
            message: "Validating analysis request and tools",
            data: Some(json!({
                "event": "phase",
                "phase": "validatingHost",
                "protocol": host_protocol_info()
            })),
        },
    )?;

    let url = validated_url(request.get("url").unwrap_or(&Value::Null))?;
    let playlist_scope = browser_playlist_scope(request)?;
    let item_limit = browser_analysis_item_limit(request)?;
    let (config, mut tools) = AppConfig::load_runtime();
    let cookies = super::browser_native_cookies::NativeCookies::from_request(request)?;
    let youtube_authenticated = cookies
        .as_ref()
        .is_some_and(|cookies| cookies.youtube_authenticated());
    apply_browser_youtube_format_coverage(
        &mut tools,
        url,
        youtube_authenticated,
        playlist_scope != "playlist",
    );
    tools.native_cookie_override = cookies.as_ref().map(|cookies| cookies.path());
    tools.validate_cookie_setup(config.use_browser_cookies)?;
    let output_directory = configured_download_directory(&config)?;

    write_response(
        &mut *writer,
        &NativeResponse {
            ok: true,
            message: "Analysis started",
            data: Some(json!({
                "event": "started",
                "phase": "startingYtDlp",
                "itemLimit": item_limit,
                "outputDirectory": output_directory.to_string_lossy(),
                "thumbnailMode": config.thumbnail_mode,
                "subtitleMode": config.subtitle_mode,
                "chapterMode": config.chapter_mode,
                "useAria2": config.use_aria2,
                "aria2Available": tools.aria2_is_available()
            })),
        },
    )?;

    write_response(
        &mut *writer,
        &NativeResponse {
            ok: true,
            message: "yt-dlp is reading media information",
            data: Some(json!({
                "event": "phase",
                "phase": if playlist_scope == "playlist" {
                    "readingPlaylist"
                } else {
                    "readingMedia"
                }
            })),
        },
    )?;

    let mut delivered = 0;
    let mut truncated = false;
    if playlist_scope == "playlist" {
        let mut received = 0;
        tools
            .analyze_playlist_url_streaming_detailed(
                url,
                config.use_browser_cookies,
                item_limit + 1,
                cancelled,
                |item| {
                    received += 1;
                    if received > item_limit {
                        truncated = true;
                        return Ok(());
                    }
                    delivered += 1;
                    write_response(
                        &mut *writer,
                        &NativeResponse {
                            ok: true,
                            message: "Analysis item ready",
                            data: Some(json!({
                                "event": "item",
                                "index": delivered - 1,
                                "item": compact_item(&item, url)
                            })),
                        },
                    )
                },
            )
            .map_err(|error| error.message)?;
    } else {
        let output = tools
            .analyze_url_detailed(url, config.use_browser_cookies)
            .map_err(|error| error.message)?;
        let values = output
            .json
            .get("entries")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|entry| !entry.is_null())
                    .take(item_limit)
                    .collect::<Vec<_>>()
            })
            .filter(|entries| !entries.is_empty())
            .unwrap_or_else(|| vec![&output.json]);
        for item in values {
            write_response(
                &mut *writer,
                &NativeResponse {
                    ok: true,
                    message: "Analysis item ready",
                    data: Some(json!({
                        "event": "item",
                        "index": delivered,
                        "item": compact_item(item, url)
                    })),
                },
            )?;
            delivered += 1;
        }
    }

    Ok(json!({
        "event": "complete",
        "itemCount": delivered,
        "itemLimit": item_limit,
        "truncated": truncated,
        "outputDirectory": output_directory.to_string_lossy(),
        "thumbnailMode": config.thumbnail_mode,
        "subtitleMode": config.subtitle_mode,
        "chapterMode": config.chapter_mode,
        "useAria2": config.use_aria2,
        "aria2Available": tools.aria2_is_available()
    }))
}

fn sanitized_file_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim()
        .trim_end_matches(['.', ' '])
        .to_owned()
}

fn browser_download_file_name(request: &Value, output_format: Option<&str>) -> String {
    let requested = string_field(request, "fileName");
    let source = if requested.trim().is_empty() {
        string_field(request, "title")
    } else {
        requested
    };
    let mut sanitized = sanitized_file_name(&source)
        .chars()
        .take(200)
        .collect::<String>()
        .trim_end_matches(['.', ' '])
        .to_owned();
    if let Some(output_format) = output_format {
        let path = Path::new(&sanitized);
        let has_output_extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case(output_format));
        if has_output_extension {
            sanitized = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .trim_end_matches(['.', ' '])
                .to_owned();
        }
    }
    if sanitized.is_empty() {
        "download".to_owned()
    } else {
        sanitized
    }
}

fn yt_dlp_output_literal(value: &str) -> String {
    value.replace('%', "%%")
}

fn browser_output_template(file_name: &str, section_count: usize) -> String {
    let base_name = yt_dlp_output_literal(file_name);
    if section_count > 1 {
        format!("{base_name} - %(section_number)02d.%(ext)s")
    } else {
        format!("{base_name}.%(ext)s")
    }
}

fn forward_process_lines(
    stream: impl Read,
    is_stderr: bool,
    tx: std::sync::mpsc::Sender<(bool, String)>,
) {
    let mut reader = BufReader::new(stream);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let line = String::from_utf8_lossy(&bytes)
                    .trim_end_matches(['\r', '\n'])
                    .to_owned();
                if tx.send((is_stderr, line)).is_err() {
                    break;
                }
            }
        }
    }
}
fn progress_event(line: &str) -> Option<Value> {
    let payload = line.trim().strip_prefix("[yt-dlp],")?;
    let fields = payload.split(',').map(str::trim).collect::<Vec<_>>();
    let percent_index = if fields.first()?.contains('%') { 0 } else { 1 };
    let percent = fields
        .get(percent_index)?
        .trim_end_matches('%')
        .trim()
        .parse::<f32>()
        .ok()?
        .clamp(0.0, 100.0);
    Some(json!({
        "event": "progress",
        "percent": percent,
        "eta": fields.get(percent_index + 1).copied().unwrap_or_default(),
        "downloaded": fields.get(percent_index + 2).copied().unwrap_or_default(),
        "total": fields.get(percent_index + 3).copied().unwrap_or_default(),
        "speed": fields.get(percent_index + 4).copied().unwrap_or_default()
    }))
}

fn configured_download_directory(config: &AppConfig) -> Result<PathBuf, String> {
    let path = resolve_output_dir(&config.download_dir)?
        .components()
        .collect::<PathBuf>();
    if !path.is_absolute() || !path.is_dir() {
        return Err(
            "The v2 download folder does not exist. Choose another folder in Options or the extension."
                .to_owned(),
        );
    }
    Ok(path)
}

fn validated_subtitle_language(request: &Value) -> Result<Option<String>, String> {
    let language = string_field(request, "subtitleLanguage");
    if language.is_empty() {
        return Ok(None);
    }
    if language.len() > 128 || language.chars().any(char::is_control) {
        return Err("The subtitle language is invalid.".to_owned());
    }
    Ok(Some(language))
}

fn validated_browser_download_section_args(request: &Value) -> Result<Vec<String>, String> {
    const MAX_RANGE_MILLIS: u64 = 366 * 24 * 60 * 60 * 1_000;
    fn range_argument(start_millis: u64, end_millis: u64) -> Result<String, String> {
        if end_millis > MAX_RANGE_MILLIS {
            return Err("The download range exceeds the supported duration.".to_owned());
        }
        DownloadTimeRange::new(start_millis, end_millis)
            .map(|range| range.yt_dlp_argument())
            .ok_or_else(|| {
                "The download range end time must be later than its start time.".to_owned()
            })
    }

    let time_range = request
        .get("downloadTimeRange")
        .filter(|range| !range.is_null());
    let chapter_ranges = request
        .get("downloadChapterRanges")
        .and_then(Value::as_array)
        .filter(|ranges| !ranges.is_empty());
    if time_range.is_some() && chapter_ranges.is_some() {
        return Err("A custom time range and chapter ranges cannot be used together.".to_owned());
    }
    if let Some(range) = time_range {
        let start_seconds = range
            .get("startSeconds")
            .and_then(Value::as_u64)
            .ok_or_else(|| "The download range start time is invalid.".to_owned())?;
        let end_seconds = range
            .get("endSeconds")
            .and_then(Value::as_u64)
            .ok_or_else(|| "The download range end time is invalid.".to_owned())?;
        return Ok(vec![range_argument(
            start_seconds.saturating_mul(1_000),
            end_seconds.saturating_mul(1_000),
        )?]);
    }
    let Some(chapter_ranges) = chapter_ranges else {
        return Ok(Vec::new());
    };
    if chapter_ranges.len() > MAX_CHAPTERS_PER_ITEM {
        return Err("Too many chapter ranges were selected.".to_owned());
    }
    let mut result = Vec::with_capacity(chapter_ranges.len());
    let mut previous_end = 0;
    for range in chapter_ranges {
        let start_millis = range
            .get("startMillis")
            .and_then(Value::as_u64)
            .ok_or_else(|| "A chapter range start time is invalid.".to_owned())?;
        let end_millis = range
            .get("endMillis")
            .and_then(Value::as_u64)
            .ok_or_else(|| "A chapter range end time is invalid.".to_owned())?;
        if !result.is_empty() && start_millis < previous_end {
            return Err("Chapter ranges must be ordered and cannot overlap.".to_owned());
        }
        result.push(range_argument(start_millis, end_millis)?);
        previous_end = end_millis;
    }
    Ok(result)
}

fn browser_output_format(
    request: &Value,
    target_kind: DownloadTargetKind,
) -> Result<Option<String>, String> {
    let format = string_field(request, "outputFormat")
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    if format.is_empty() {
        return Ok(None);
    }
    let allowed = match target_kind {
        DownloadTargetKind::Normal => matches!(format.as_str(), "mkv" | "mp4" | "webm"),
        DownloadTargetKind::Audio => matches!(
            format.as_str(),
            "mp3" | "m4a" | "flac" | "opus" | "aac" | "vorbis" | "alac"
        ),
        _ => false,
    };
    if allowed {
        Ok(Some(format))
    } else {
        Err("The browser output format is invalid for this item.".to_owned())
    }
}

fn browser_chapter_output_argument(output_dir: &Path, file_name: &str) -> String {
    format!(
        "chapter:{}",
        output_dir
            .join(yt_dlp_output_literal(file_name))
            .join("%(section_number)02d - %(section_title)s.%(ext)s")
            .display()
    )
}

fn apply_browser_post_process_choices(
    command: &mut std::process::Command,
    output_dir: &Path,
    file_name: &str,
    subtitle_language: Option<&str>,
    subtitle_automatic: bool,
    subtitle_mode: PostProcessMode,
    chapter_mode: PostProcessMode,
    chapter_mode_is_explicit: bool,
) {
    command
        .arg("--no-write-subs")
        .arg("--no-write-auto-subs")
        .arg("--no-embed-subs");
    if let Some(language) = subtitle_language.filter(|_| subtitle_mode.writes()) {
        command.arg("--sub-langs").arg(language);
        if subtitle_automatic {
            command.arg("--write-auto-subs");
        } else if !subtitle_mode.embeds() {
            command.arg("--write-subs");
        }
        if subtitle_mode.embeds() {
            command.arg("--embed-subs");
        } else {
            command.arg("--convert-subs").arg("srt");
        }
    }

    command.arg("--no-split-chapters");
    if chapter_mode_is_explicit {
        command.arg("--no-embed-chapters");
    }
    if chapter_mode.writes() {
        command
            .arg("--split-chapters")
            .arg("--output")
            .arg(browser_chapter_output_argument(output_dir, file_name));
    }
    if chapter_mode.embeds() {
        command.arg("--embed-chapters");
    }
}

fn browser_subtitle_mode(
    request: &Value,
    has_subtitle_language: bool,
) -> Result<PostProcessMode, String> {
    let mode = match request.get("subtitleMode") {
        None | Some(Value::Null) => {
            if has_subtitle_language {
                PostProcessMode::Download
            } else {
                PostProcessMode::Off
            }
        }
        Some(Value::String(value)) => match value.as_str() {
            "off" => PostProcessMode::Off,
            "download" => PostProcessMode::Download,
            "embed" => PostProcessMode::Embed,
            _ => return Err("The browser subtitle mode is invalid.".to_owned()),
        },
        _ => return Err("The browser subtitle mode is invalid.".to_owned()),
    };
    if mode.writes() && !has_subtitle_language {
        return Err("Select a subtitle track before enabling subtitle handling.".to_owned());
    }
    Ok(mode)
}

fn browser_chapter_mode(request: &Value) -> Result<PostProcessMode, String> {
    match request.get("chapterMode") {
        None | Some(Value::Null) => Ok(
            if request
                .get("splitChapters")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                PostProcessMode::Download
            } else {
                PostProcessMode::Off
            },
        ),
        Some(Value::String(value)) => match value.as_str() {
            "off" => Ok(PostProcessMode::Off),
            "download" => Ok(PostProcessMode::Download),
            "embed" => Ok(PostProcessMode::Embed),
            _ => Err("The browser chapter mode is invalid.".to_owned()),
        },
        _ => Err("The browser chapter mode is invalid.".to_owned()),
    }
}

fn browser_thumbnail_mode(
    request: &Value,
    fallback: PostProcessMode,
) -> Result<PostProcessMode, String> {
    match request.get("thumbnailMode") {
        None | Some(Value::Null) => Ok(fallback),
        Some(Value::String(value)) => match value.as_str() {
            "off" => Ok(PostProcessMode::Off),
            "download" => Ok(PostProcessMode::Download),
            "embed" => Ok(PostProcessMode::Embed),
            _ => Err("The browser thumbnail mode is invalid.".to_owned()),
        },
        _ => Err("The browser thumbnail mode is invalid.".to_owned()),
    }
}

fn browser_use_aria2(request: &Value, fallback: bool) -> Result<(bool, bool), String> {
    match request.get("useAria2") {
        None | Some(Value::Null) => Ok((fallback, false)),
        Some(Value::Bool(value)) => Ok((*value, true)),
        _ => Err("The browser Aria2 choice is invalid.".to_owned()),
    }
}

fn apply_browser_thumbnail_mode(command: &mut std::process::Command, mode: PostProcessMode) {
    command
        .arg("--no-write-thumbnail")
        .arg("--no-embed-thumbnail");
    match mode {
        PostProcessMode::Off => {}
        PostProcessMode::Download => {
            command
                .arg("--write-thumbnail")
                .arg("--convert-thumbnails")
                .arg("jpg");
        }
        PostProcessMode::Embed => {
            command
                .arg("--embed-thumbnail")
                .arg("--convert-thumbnails")
                .arg("jpg");
        }
    }
}

fn apply_browser_audio_extraction(
    command: &mut std::process::Command,
    audio_extension: &str,
) -> Result<(), String> {
    let extension = audio_extension.trim();
    if extension.len() > 16 || extension.chars().any(|ch| !ch.is_ascii_alphanumeric()) {
        return Err("The audio output format is invalid.".to_owned());
    }
    command.arg("--extract-audio");
    if !extension.is_empty() {
        command.arg("--audio-format").arg(extension);
    }
    Ok(())
}

fn browser_playlist_scope(request: &Value) -> Result<&'static str, String> {
    match request
        .get("playlistScope")
        .and_then(Value::as_str)
        .unwrap_or("current")
    {
        "current" => Ok("current"),
        "playlist" => Ok("playlist"),
        _ => Err("The playlist scope is invalid.".to_owned()),
    }
}

fn apply_browser_playlist_scope(
    command: &mut std::process::Command,
    request: &Value,
) -> Result<(), String> {
    command.arg(match browser_playlist_scope(request)? {
        "current" => "--no-playlist",
        "playlist" => "--yes-playlist",
        _ => unreachable!("validated browser playlist scope"),
    });
    Ok(())
}

fn browser_format_selector(request: &Value) -> Result<String, String> {
    let video_id = string_field(request, "videoFormatId");
    let audio_id = string_field(request, "audioFormatId");
    let video_muxed = request
        .get("videoIsMuxed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let content_kind = request
        .get("contentKind")
        .and_then(Value::as_str)
        .unwrap_or("video");
    if !matches!(content_kind, "video" | "audio") {
        return Err("The download content kind is invalid.".to_owned());
    }
    let use_recommended = request
        .get("useRecommendedFormat")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    match (
        use_recommended,
        content_kind,
        video_id.is_empty(),
        audio_id.is_empty(),
        video_muxed,
    ) {
        (true, "video", _, _, _) => Ok("bestvideo*+bestaudio/best".to_owned()),
        (true, "audio", _, _, _) => Ok("bestaudio/best[acodec!=none]".to_owned()),
        (_, "audio", _, false, _) => Ok(audio_id),
        (_, "video", false, _, true) => Ok(video_id),
        (_, "video", false, false, false) => Ok(format!("{video_id}+{audio_id}")),
        (_, "video", false, true, false) => Ok(video_id),
        _ => Err("Select a format before downloading.".to_owned()),
    }
}

fn stream_download(
    request: &Value,
    writer: &mut impl Write,
    cancelled: &AtomicBool,
) -> Result<Value, String> {
    let url = validated_url(request.get("url").unwrap_or(&Value::Null))?.to_owned();
    let video_id = string_field(request, "videoFormatId");
    let audio_id = string_field(request, "audioFormatId");
    let video_muxed = request
        .get("videoIsMuxed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let content_kind = request
        .get("contentKind")
        .and_then(Value::as_str)
        .unwrap_or("video");
    let format_selector = browser_format_selector(request)?;
    let download_section_args = validated_browser_download_section_args(request)?;
    let (config, mut tools) = AppConfig::load_runtime();
    let output_dir = configured_download_directory(&config)?;
    let cookies = super::browser_native_cookies::NativeCookies::from_request(request)?;
    let youtube_authenticated = cookies
        .as_ref()
        .is_some_and(|cookies| cookies.youtube_authenticated());
    apply_browser_youtube_format_coverage(&mut tools, &url, youtube_authenticated, true);
    tools.native_cookie_override = cookies.as_ref().map(|cookies| cookies.path());
    tools.validate_cookie_setup(config.use_browser_cookies)?;
    if !download_section_args.is_empty() && tools.effective_config_owns_download_sections() {
        return Err(
            "The active yt-dlp config already controls download sections. Remove that option before selecting a browser time range."
                .to_owned(),
        );
    }
    let target_kind = if content_kind == "audio" {
        DownloadTargetKind::Audio
    } else {
        DownloadTargetKind::Normal
    };
    let output_format = browser_output_format(request, target_kind)?;
    let thumbnail_mode_is_explicit =
        !matches!(request.get("thumbnailMode"), None | Some(Value::Null));
    let thumbnail_mode = browser_thumbnail_mode(request, config.thumbnail_mode)?;
    let (use_aria2, use_aria2_is_explicit) = browser_use_aria2(request, config.use_aria2)?;
    if use_aria2_is_explicit && use_aria2 && !tools.aria2_is_available() {
        return Err(
            "Aria2 is enabled for this browser item, but aria2c is not installed.".to_owned(),
        );
    }
    if thumbnail_mode_is_explicit
        && output_format.as_deref() == Some("webm")
        && thumbnail_mode == PostProcessMode::Embed
    {
        return Err(
            "WebM does not support embedded thumbnails. Select MKV or download the thumbnail separately."
                .to_owned(),
        );
    }
    let file_name = browser_download_file_name(request, output_format.as_deref());
    let subtitle_language = if target_kind == DownloadTargetKind::Audio {
        None
    } else {
        validated_subtitle_language(request)?
    };
    let subtitle_automatic = request
        .get("subtitleAutomatic")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let subtitle_mode = browser_subtitle_mode(request, subtitle_language.is_some())?;
    let chapter_mode_is_explicit = !matches!(request.get("chapterMode"), None | Some(Value::Null));
    let chapter_mode = browser_chapter_mode(request)?;
    if chapter_mode.writes() && !download_section_args.is_empty() {
        return Err(
            "Chapter splitting and a custom download range cannot be used together.".to_owned(),
        );
    }
    let download = DownloadRequest {
        target_kind,
        url,
        format_selector,
        video_selector: video_id,
        audio_selector: audio_id,
        is_muxed_video: video_muxed,
        video_ext: string_field(request, "videoExt"),
        audio_ext: if target_kind == DownloadTargetKind::Audio {
            output_format
                .clone()
                .unwrap_or_else(|| string_field(request, "audioExt"))
        } else {
            string_field(request, "audioExt")
        },
        merge_output_container: None,
        output_extension_override: (target_kind == DownloadTargetKind::Normal)
            .then_some(output_format)
            .flatten(),
        upload_date: String::new(),
        subtitle_lang: subtitle_language.clone(),
        subtitle_ext: "srt".to_owned(),
        subtitle_source_ext: String::new(),
        subtitle_url: None,
        write_auto_subs: subtitle_automatic,
        subtitle_is_auto_translated: false,
        write_subtitles: subtitle_mode.writes(),
        embed_subtitles: subtitle_mode.embeds(),
        write_chapters: chapter_mode.writes(),
        embed_chapters: chapter_mode.embeds(),
        write_thumbnail: thumbnail_mode.writes(),
        embed_thumbnail: thumbnail_mode.embeds(),
        use_cookies: config.use_browser_cookies,
        use_aria2: use_aria2 && !use_aria2_is_explicit,
        emit_json: false,
        output_path: None,
        output_dir: output_dir.to_string_lossy().into_owned(),
        file_name,
        download_section_args,
    };
    let preserve_prepared_section_format = !download.download_section_args.is_empty()
        && tools.chapter_compatibility_mode
        && !tools.has_explicit_config()
        && download.format_selector.contains('+');
    let mut prepared = tools.prepare_download(&download)?;
    // Browser choices take precedence over yt-dlp config format/output defaults.
    // The shared range compatibility path intentionally replaces a separate
    // video+audio pair with a muxed selector so FFmpeg can seek reliably.
    if !preserve_prepared_section_format {
        prepared
            .command
            .arg("--format")
            .arg(&download.format_selector);
    }
    prepared
        .command
        .arg("--output")
        .arg(output_dir.join(browser_output_template(
            &download.file_name,
            download.download_section_args.len(),
        )));
    if target_kind == DownloadTargetKind::Audio {
        apply_browser_audio_extraction(&mut prepared.command, &download.audio_ext)?;
    }
    apply_browser_post_process_choices(
        &mut prepared.command,
        &output_dir,
        &download.file_name,
        subtitle_language.as_deref(),
        subtitle_automatic,
        subtitle_mode,
        chapter_mode,
        chapter_mode_is_explicit,
    );
    if thumbnail_mode_is_explicit {
        apply_browser_thumbnail_mode(&mut prepared.command, thumbnail_mode);
    }
    if use_aria2_is_explicit {
        if use_aria2 {
            if !tools.append_aria2_downloader_args(&mut prepared.command) {
                return Err("Aria2 became unavailable before the download started.".to_owned());
            }
        } else {
            prepared.command.arg("--downloader").arg("native");
        }
    }
    apply_browser_playlist_scope(&mut prepared.command, request)?;
    let expected = prepared.output_path.display().to_string();
    prepared
        .command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = prepared
        .command
        .spawn()
        .map_err(|error| format!("Could not start yt-dlp: {error}"))?;
    let _process_guard = super::track_child_process(&child, "yt-dlp browser download");
    if cancelled.load(Ordering::Acquire) {
        super::force_cleanup_tracked_processes();
    }
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Could not capture yt-dlp output.".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Could not capture yt-dlp errors.".to_owned())?;
    let (line_tx, line_rx) = std::sync::mpsc::channel();
    let stdout_tx = line_tx.clone();
    let stdout_reader = thread::spawn(move || forward_process_lines(stdout, false, stdout_tx));
    let stderr_reader = thread::spawn(move || forward_process_lines(stderr, true, line_tx));

    write_response(
        &mut *writer,
        &NativeResponse {
            ok: true,
            message: "Download started",
            data: Some(json!({
                "event": "started",
                "protocol": host_protocol_info()
            })),
        },
    )?;

    let mut final_path = None;
    let mut last_error = String::new();
    let mut last_percent = None;
    for (is_stderr, line) in line_rx {
        if is_stderr && !line.trim().is_empty() {
            last_error = line.clone();
        }
        if let Some(payload) = line.trim().strip_prefix(FINAL_OUTPUT_PATH_PREFIX) {
            final_path = serde_json::from_str::<String>(payload).ok();
        }
        let Some(event) = progress_event(&line) else {
            continue;
        };
        let percent = event["percent"].as_f64().map(|value| value.floor() as u8);
        if percent == last_percent {
            continue;
        }
        last_percent = percent;
        write_response(
            &mut *writer,
            &NativeResponse {
                ok: true,
                message: "Downloading",
                data: Some(event),
            },
        )?;
    }
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();

    let status = child
        .wait()
        .map_err(|error| format!("Could not wait for yt-dlp: {error}"))?;
    if cancelled.load(Ordering::Acquire) {
        return Ok(json!({ "event": "stopped" }));
    }
    if !status.success() {
        let detail = if last_error.is_empty() {
            "yt-dlp exited without an error message".to_owned()
        } else {
            last_error.chars().take(4096).collect()
        };
        return Err(format!("yt-dlp download failed: {detail}"));
    }
    Ok(json!({
        "event": "complete",
        "outputPath": final_path.unwrap_or(expected)
    }))
}
fn handle_request(raw: &str) -> Result<(&'static str, Value), String> {
    let request: Value = serde_json::from_str(raw)
        .map_err(|error| format!("The Native Messaging request is not valid JSON: {error}"))?;
    match request.get("action").and_then(Value::as_str) {
        Some("chooseDirectory") => {
            let (mut config, _) = AppConfig::load_runtime();
            let mut dialog = rfd::FileDialog::new();
            if let Ok(current) = configured_download_directory(&config) {
                dialog = dialog.set_directory(current);
            }
            let directory = dialog
                .pick_folder()
                .map(|path| path.to_string_lossy().into_owned());
            if let Some(directory) = directory.as_deref() {
                config.set_download_dir(directory);
                config.save()?;
            }
            Ok((
                "Folder selection complete",
                json!({ "directory": directory }),
            ))
        }
        Some("openDirectory") => {
            let (config, _) = AppConfig::load_runtime();
            let directory = configured_download_directory(&config)?;
            open_output_folder(&directory.to_string_lossy())?;
            Ok((
                "Download folder opened",
                json!({ "directory": directory.to_string_lossy() }),
            ))
        }
        Some("status") => {
            let (config, tools) = AppConfig::load_runtime();
            let output_directory = configured_download_directory(&config)?;
            Ok((
                "Connected",
                json!({
                    "host": HOST_NAME,
                    "ytDlpAvailable": tools.validate_yt_dlp_available().is_ok(),
                    "defaultDownloadType": if config.app_mode == "audio" { "audio" } else { "settings" },
                    "cookiesEnabled": config.use_browser_cookies,
                    "outputDirectory": output_directory.to_string_lossy(),
                    "thumbnailMode": config.thumbnail_mode,
                    "subtitleMode": config.subtitle_mode,
                    "chapterMode": config.chapter_mode,
                    "useAria2": config.use_aria2,
                    "aria2Available": tools.aria2_is_available(),
                    "protocol": host_protocol_info()
                }),
            ))
        }
        Some("analyze") => Ok(("Analysis complete", analyze(&request)?)),
        Some(_) => Err("The Native Messaging action is not supported.".to_owned()),
        None => Err("The Native Messaging request has no action.".to_owned()),
    }
}

fn read_message(mut reader: impl Read) -> Result<String, String> {
    let mut length = [0u8; 4];
    reader
        .read_exact(&mut length)
        .map_err(|error| format!("Could not read the message length: {error}"))?;
    let length = u32::from_le_bytes(length) as usize;
    if length == 0 || length > MAX_REQUEST_BYTES {
        return Err("The Native Messaging request size is invalid.".to_owned());
    }
    let mut bytes = vec![0; length];
    reader
        .read_exact(&mut bytes)
        .map_err(|error| format!("Could not read the message: {error}"))?;
    String::from_utf8(bytes).map_err(|_| "The Native Messaging request is not UTF-8.".to_owned())
}

fn write_response(mut writer: impl Write, response: &NativeResponse<'_>) -> Result<(), String> {
    let body = serde_json::to_vec(response)
        .map_err(|error| format!("Could not serialize the response: {error}"))?;
    if body.len() > MAX_RESPONSE_BYTES {
        return Err("The Native Messaging response is too large.".to_owned());
    }
    writer
        .write_all(&(body.len() as u32).to_le_bytes())
        .and_then(|_| writer.write_all(&body))
        .and_then(|_| writer.flush())
        .map_err(|error| format!("Could not write the Native Messaging response: {error}"))
}

#[cfg(windows)]
fn configure_binary_stdio() -> Result<(), String> {
    unsafe extern "C" {
        fn _setmode(file_descriptor: i32, mode: i32) -> i32;
    }
    const BINARY: i32 = 0x8000;
    if unsafe { _setmode(0, BINARY) } == -1 || unsafe { _setmode(1, BINARY) } == -1 {
        Err("Could not configure binary Native Messaging I/O.".to_owned())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn configure_binary_stdio() -> Result<(), String> {
    Ok(())
}

pub fn run_native_host_if_requested() -> bool {
    let origin = std::env::args()
        .skip(1)
        .find(|argument| argument.starts_with("chrome-extension://"));
    let Some(origin) = origin else {
        return false;
    };
    let mut writer = std::io::stdout().lock();
    let result = (|| -> Result<(), String> {
        configure_binary_stdio()?;
        if origin != EXTENSION_ORIGIN {
            return Err("This extension is not allowed to connect.".to_owned());
        }
        super::runtime_paths::set_application_current_dir()?;
        let raw = read_message(std::io::stdin().lock())?;
        let request: Value = serde_json::from_str(&raw)
            .map_err(|error| format!("The Native Messaging request is not valid JSON: {error}"))?;
        let action = request.get("action").and_then(Value::as_str);
        if action == Some("download") {
            let cancelled = Arc::new(AtomicBool::new(false));
            let cancellation = Arc::clone(&cancelled);
            thread::spawn(move || {
                let cancel_requested = read_message(std::io::stdin().lock())
                    .ok()
                    .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                    .and_then(|request| {
                        request
                            .get("action")
                            .and_then(Value::as_str)
                            .map(|action| action == "cancelDownload")
                    })
                    .unwrap_or(true);
                if cancel_requested {
                    cancellation.store(true, Ordering::Release);
                    super::force_cleanup_tracked_processes();
                }
            });
            let data = stream_download(&request, &mut writer, &cancelled)?;
            write_response(
                &mut writer,
                &NativeResponse {
                    ok: true,
                    message: "Download complete",
                    data: Some(data),
                },
            )
        } else if action == Some("analyze") {
            let cancelled = Arc::new(AtomicBool::new(false));
            let cancellation = Arc::clone(&cancelled);
            thread::spawn(move || {
                let cancel_requested = read_message(std::io::stdin().lock())
                    .ok()
                    .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                    .and_then(|request| {
                        request
                            .get("action")
                            .and_then(Value::as_str)
                            .map(|action| action == "cancelAnalysis")
                    })
                    .unwrap_or(true);
                if cancel_requested {
                    cancellation.store(true, Ordering::Release);
                    super::force_cleanup_tracked_processes();
                }
            });
            let data = stream_analysis(&request, &mut writer, &cancelled)?;
            write_response(
                &mut writer,
                &NativeResponse {
                    ok: true,
                    message: "Analysis complete",
                    data: Some(data),
                },
            )
        } else {
            let (message, data) = handle_request(&raw)?;
            write_response(
                &mut writer,
                &NativeResponse {
                    ok: true,
                    message,
                    data: Some(data),
                },
            )
        }
    })();
    if let Err(error) = result {
        let _ = write_response(
            &mut writer,
            &NativeResponse {
                ok: false,
                message: &error,
                data: Some(json!({ "event": "error" })),
            },
        );
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn manifest_limits_access_to_the_fixed_extension() {
        let manifest = NativeHostManifest {
            name: HOST_NAME,
            description: "test",
            path: "app.exe".to_owned(),
            host_type: "stdio",
            allowed_origins: [EXTENSION_ORIGIN],
        };
        let value = serde_json::to_value(manifest).unwrap();
        assert_eq!(value["allowed_origins"][0], EXTENSION_ORIGIN);
        assert_eq!(EXTENSION_ID, "jnhdidpednnfdphbcbeignbhceemminn");
    }

    #[test]
    fn framing_preserves_utf8_and_rejects_large_messages() {
        let body = br#"{"action":"status","title":"\u6e2c\u8a66"}"#;
        let mut frame = (body.len() as u32).to_le_bytes().to_vec();
        frame.extend_from_slice(body);
        assert_eq!(read_message(Cursor::new(frame)).unwrap().as_bytes(), body);
        let frame = ((MAX_REQUEST_BYTES + 1) as u32).to_le_bytes();
        assert!(read_message(Cursor::new(frame)).is_err());
    }

    #[test]
    fn progress_events_accept_template_lines_with_and_without_format_id() {
        let current = progress_event("[yt-dlp],137, 53.1%,00:05,1024,8192,2048,5").unwrap();
        assert!((current["percent"].as_f64().unwrap() - 53.1).abs() < 0.001);
        assert_eq!(current["downloaded"], "1024");
        let legacy = progress_event("[yt-dlp], 12.5%,00:05,1024,8192,2048,5").unwrap();
        assert!((legacy["percent"].as_f64().unwrap() - 12.5).abs() < 0.001);
        assert!(progress_event("[download] Destination: file.webm").is_none());
    }

    #[test]
    fn resolved_v2_directory_removes_current_directory_components() {
        let normalized = Path::new(r"D:\portable\.\download")
            .components()
            .collect::<PathBuf>();
        assert_eq!(normalized, PathBuf::from(r"D:\portable\download"));
    }
    #[test]
    fn compact_analysis_detects_multiple_items_and_strips_media_urls() {
        let raw = json!({"entries": [{"id":"one","title":"One","webpage_url":"https://x/1","formats":[{"format_id":"v","vcodec":"avc1","acodec":"none","url":"https://secret"}]},{"id":"two","title":"Two","webpage_url":"https://x/2"}]});
        let items = raw["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| compact_item(entry, "https://x"))
            .collect::<Vec<_>>();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["formats"][0]["id"], "v");
        assert!(items[0]["formats"][0].get("url").is_none());
    }

    #[test]
    fn compact_formats_infers_redgifs_video_when_codec_names_are_absent() {
        let raw = json!({
            "formats": [
                {
                    "format_id": "sd", "vcodec": null, "acodec": null,
                    "video_ext": "mp4", "audio_ext": "none",
                    "width": 270, "height": 480
                },
                {
                    "format_id": "hd", "vcodec": null, "acodec": null,
                    "video_ext": "mp4", "audio_ext": "none",
                    "width": 1080, "height": 1920
                }
            ]
        });

        let formats = compact_formats(&raw);
        assert_eq!(formats.len(), 2);
        assert_eq!(formats[0]["id"], "sd");
        assert_eq!(formats[0]["kind"], "video");
        assert_eq!(formats[0]["audioPresence"], "unknown");
        assert_eq!(formats[1]["id"], "hd");
        assert_eq!(formats[1]["kind"], "video");
        assert_eq!(formats[1]["audioPresence"], "unknown");
    }

    #[test]
    fn compact_formats_does_not_treat_empty_codecs_as_media_streams() {
        let raw = json!({
            "formats": [
                {
                    "format_id": "empty", "vcodec": "", "acodec": "",
                    "ext": "mp4"
                },
                {
                    "format_id": "audio", "vcodec": "none", "acodec": "mp4a.40.2",
                    "audio_ext": "m4a", "asr": 44100
                }
            ]
        });

        let formats = compact_formats(&raw);
        assert_eq!(formats.len(), 1);
        assert_eq!(formats[0]["id"], "audio");
        assert_eq!(formats[0]["kind"], "audio");
        assert_eq!(formats[0]["audioPresence"], "present");
    }

    #[test]
    fn compact_formats_preserves_explicit_absent_audio() {
        let raw = json!({
            "formats": [{
                "format_id": "video", "vcodec": "avc1", "acodec": "none",
                "video_ext": "mp4", "audio_ext": "none"
            }]
        });

        let formats = compact_formats(&raw);
        assert_eq!(formats[0]["kind"], "video");
        assert_eq!(formats[0]["audioPresence"], "absent");
    }

    #[test]
    fn compact_formats_falls_back_to_approximate_filesize_and_exposes_bitrate() {
        let raw = json!({
            "formats": [{
                "format_id": "137", "vcodec": "avc1.640028", "acodec": "none",
                "video_ext": "mp4", "width": 1920, "height": 1080,
                "vbr": 4123.5, "filesize": null, "filesize_approx": 12345678.0
            }]
        });

        let formats = compact_formats(&raw);
        assert_eq!(formats[0]["filesize"], 12_345_678);
        assert_eq!(formats[0]["filesizeApproximate"], true);
        assert_eq!(formats[0]["bitrate"], 4123.5);
    }

    #[test]
    fn browser_youtube_coverage_adds_muxed_client_without_overriding_user_args() {
        let mut defaults = ToolPaths::default();
        apply_browser_youtube_format_coverage(
            &mut defaults,
            "https://www.youtube.com/watch?v=video",
            false,
            true,
        );
        assert_eq!(
            defaults.youtube_extractor_args,
            BROWSER_YOUTUBE_PLAYER_CLIENTS
        );

        let mut custom = ToolPaths {
            youtube_extractor_args: "player_client=tv".to_owned(),
            ..ToolPaths::default()
        };
        apply_browser_youtube_format_coverage(&mut custom, "https://youtu.be/video", false, true);
        assert_eq!(custom.youtube_extractor_args, "player_client=tv");

        let mut other_site = ToolPaths::default();
        apply_browser_youtube_format_coverage(
            &mut other_site,
            "https://example.com/watch?v=video",
            false,
            true,
        );
        assert!(other_site.youtube_extractor_args.is_empty());

        let mut authenticated = ToolPaths::default();
        apply_browser_youtube_format_coverage(
            &mut authenticated,
            "https://www.youtube.com/watch?v=video",
            true,
            true,
        );
        assert!(authenticated.youtube_extractor_args.is_empty());

        let mut playlist = ToolPaths::default();
        apply_browser_youtube_format_coverage(
            &mut playlist,
            "https://www.youtube.com/playlist?list=playlist",
            false,
            false,
        );
        assert!(playlist.youtube_extractor_args.is_empty());
    }

    #[test]
    fn playlist_analysis_reports_when_configured_item_limit_is_exceeded() {
        let entries = (0..=DEFAULT_MULTI_ITEMS)
            .map(|index| json!({"id": index}))
            .collect::<Vec<_>>();
        assert!(playlist_analysis_is_truncated(
            &json!({"entries": entries}),
            "playlist",
            DEFAULT_MULTI_ITEMS
        ));
        assert!(playlist_analysis_is_truncated(
            &json!({"entries": [], "playlist_count": 100}),
            "playlist",
            DEFAULT_MULTI_ITEMS
        ));
        assert!(!playlist_analysis_is_truncated(
            &json!({"entries": [1]}),
            "current",
            DEFAULT_MULTI_ITEMS
        ));
    }

    #[test]
    fn browser_analysis_item_limit_defaults_and_rejects_unbounded_values() {
        assert_eq!(
            browser_analysis_item_limit(&json!({})).unwrap(),
            DEFAULT_MULTI_ITEMS
        );
        assert_eq!(
            browser_analysis_item_limit(&json!({"itemLimit": 64})).unwrap(),
            64
        );
        assert!(browser_analysis_item_limit(&json!({"itemLimit": 0})).is_err());
        assert!(browser_analysis_item_limit(&json!({"itemLimit": 101})).is_err());
        assert!(browser_analysis_item_limit(&json!({"itemLimit": 1.5})).is_err());
    }

    #[test]
    fn native_protocol_metadata_is_additive_and_future_client_fields_are_ignored() {
        let host = host_protocol_info();
        assert_eq!(host["wireVersion"], NATIVE_WIRE_VERSION);
        assert!(
            host["capabilities"]
                .as_array()
                .is_some_and(|values| values.iter().any(|value| value == "analysis.stream.v1"))
        );
        assert_eq!(host["limits"]["playlistItems"], MAX_MULTI_ITEMS);

        let future_client = json!({
            "client": {
                "wireVersion": 99,
                "capabilities": ["future.unknown.v9"],
                "futureField": true
            },
            "unknownTopLevel": {"value": 1}
        });
        assert_eq!(
            browser_analysis_item_limit(&future_client).unwrap(),
            DEFAULT_MULTI_ITEMS
        );
        assert_eq!(browser_playlist_scope(&future_client).unwrap(), "current");
    }

    #[test]
    fn compact_item_exposes_subtitle_and_chapter_choices_without_media_urls() {
        let raw = json!({
            "id": "one",
            "formats": [{
                "format_id": "muxed", "vcodec": "avc1", "acodec": "mp4a.40.2",
                "ext": "mp4", "url": "https://secret"
            }],
            "subtitles": {
                "zh-Hant": [{"name": "繁體中文", "url": "https://secret/sub"}],
                "live_chat": [{"name": "Live chat", "url": "https://secret/chat"}]
            },
            "automatic_captions": {
                "zh-Hant": [{"name": "duplicate"}],
                "en": [{"name": "English"}]
            },
            "chapters": [{"title": "One"}, {"title": "Two"}]
        });

        let item = compact_item(&raw, "https://example.test/video");

        assert_eq!(item["chapterCount"], 2);
        assert_eq!(item["subtitles"].as_array().unwrap().len(), 2);
        assert_eq!(item["subtitles"][0]["language"], "zh-Hant");
        assert_eq!(item["subtitles"][0]["automatic"], false);
        assert_eq!(item["subtitles"][1]["automatic"], true);
        assert_eq!(item["formats"][0]["audioExt"], "m4a");
        assert!(item["formats"][0].get("url").is_none());
    }

    #[test]
    fn browser_post_process_choices_override_config_owned_defaults() {
        let mut command = std::process::Command::new("yt-dlp");
        let output_dir = Path::new(r"D:\Downloads");

        apply_browser_post_process_choices(
            &mut command,
            output_dir,
            "custom file name",
            Some("zh-Hant"),
            false,
            PostProcessMode::Download,
            PostProcessMode::Download,
            true,
        );

        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.iter().any(|arg| arg == "--no-write-subs"));
        assert!(args.iter().any(|arg| arg == "--write-subs"));
        assert!(args.iter().any(|arg| arg == "zh-Hant"));
        assert!(args.iter().any(|arg| arg == "--no-split-chapters"));
        assert!(args.iter().any(|arg| arg == "--split-chapters"));
        assert!(args.iter().any(|arg| {
            arg.starts_with("chapter:")
                && arg.contains("%(section_number)02d - %(section_title)s.%(ext)s")
        }));
    }

    #[test]
    fn browser_audio_mode_extracts_selected_or_muxed_source_audio() {
        let mut command = std::process::Command::new("yt-dlp");
        apply_browser_audio_extraction(&mut command, "m4a").unwrap();
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(args, ["--extract-audio", "--audio-format", "m4a"]);

        let mut best = std::process::Command::new("yt-dlp");
        apply_browser_audio_extraction(&mut best, "").unwrap();
        assert_eq!(
            best.get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["--extract-audio"]
        );
        assert!(apply_browser_audio_extraction(&mut best, "../../mp3").is_err());
    }

    #[test]
    fn browser_item_output_format_is_kind_scoped_and_allowlisted() {
        assert_eq!(
            browser_output_format(&json!({"outputFormat":"mkv"}), DownloadTargetKind::Normal)
                .unwrap(),
            Some("mkv".to_owned())
        );
        assert_eq!(
            browser_output_format(&json!({"outputFormat":".mp3"}), DownloadTargetKind::Audio)
                .unwrap(),
            Some("mp3".to_owned())
        );
        assert!(
            browser_output_format(&json!({"outputFormat":"mp3"}), DownloadTargetKind::Normal)
                .is_err()
        );
        assert!(
            browser_output_format(&json!({"outputFormat":"mkv"}), DownloadTargetKind::Audio)
                .is_err()
        );
    }

    #[test]
    fn browser_playlist_scope_is_explicit_and_validated() {
        let mut current = std::process::Command::new("yt-dlp");
        apply_browser_playlist_scope(&mut current, &json!({"playlistScope": "current"})).unwrap();
        assert_eq!(
            current
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["--no-playlist"]
        );

        let mut playlist = std::process::Command::new("yt-dlp");
        apply_browser_playlist_scope(&mut playlist, &json!({"playlistScope": "playlist"})).unwrap();
        assert_eq!(
            playlist
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["--yes-playlist"]
        );
        assert!(
            apply_browser_playlist_scope(&mut playlist, &json!({"playlistScope": "all"})).is_err()
        );
    }

    #[test]
    fn browser_format_selector_accepts_silent_video_and_preserves_separate_audio_merge() {
        assert_eq!(
            browser_format_selector(&json!({
                "contentKind": "video",
                "videoFormatId": "hd",
                "audioFormatId": "",
                "videoIsMuxed": false
            }))
            .unwrap(),
            "hd"
        );
        assert_eq!(
            browser_format_selector(&json!({
                "contentKind": "video",
                "videoFormatId": "video",
                "audioFormatId": "audio",
                "videoIsMuxed": false
            }))
            .unwrap(),
            "video+audio"
        );
    }
}
