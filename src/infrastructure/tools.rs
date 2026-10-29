use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::os::raw::c_void;
use std::process::{Command, Stdio};
#[cfg(target_os = "windows")]
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::cookie_site_index::read_cookie_site_index;
#[path = "yt_dlp_config_options.rs"]
mod config_options;
#[path = "output_preparation.rs"]
mod output_preparation;
use config_options::ConfigOption;

#[cfg(target_os = "windows")]
use std::os::windows::ffi::{OsStrExt, OsStringExt};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const DEFAULT_FORMAT_SELECTOR: &str = "bestvideo*+bestaudio/best";
// Audio Mode must never fall back to a muxed video format. Besides wasting
// bandwidth and cache space, a muxed fallback is not a valid audio cache entry.
const MUSIC_STREAM_FORMAT_SELECTOR: &str = "bestaudio[vcodec=none][acodec!=none]";
const SECTION_FORMAT_SELECTOR: &str =
    "best[protocol!*=dash][vcodec!=none][acodec!=none]/best[vcodec!=none][acodec!=none]/best";
const COOKIE_SOURCE_AUTO: &str = "auto";
const COOKIE_SOURCE_FILE: &str = "file";
pub const FINAL_OUTPUT_PATH_PREFIX: &str = "__YTDLPGUI_FINAL_PATH__=";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(target_os = "windows")]
const SHGFI_DISPLAYNAME: u32 = 0x0000_0200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowserCookieSourceOption {
    pub value: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserCookieProfileOption {
    pub value: String,
    pub label: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileTimeMode {
    #[serde(
        rename = "none",
        alias = "preserve_modified",
        alias = "Modified",
        alias = "modified"
    )]
    None,
    #[serde(rename = "use_upload_date", alias = "Upload", alias = "upload")]
    UseUploadDate,
    #[serde(
        rename = "use_download_time",
        alias = "use_current",
        alias = "Created",
        alias = "created"
    )]
    UseDownloadTime,
}

impl Default for FileTimeMode {
    fn default() -> Self {
        Self::None
    }
}

impl FileTimeMode {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::None => "advance.file_time.none",
            Self::UseUploadDate => "advance.file_time.upload_date",
            Self::UseDownloadTime => "advance.file_time.download_time",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheLocationMode {
    V2Cache,
    WindowsTemp,
}

impl CacheLocationMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::V2Cache => "Application folder (portable)",
            Self::WindowsTemp => "System temporary folder",
        }
    }
}

#[derive(Clone)]
pub struct DownloadRequest {
    pub output: super::output_policy::OutputRequest,
    pub target_kind: DownloadTargetKind,
    pub url: String,
    pub playlist_index: Option<usize>,
    pub format_selector: String,
    pub video_selector: String,
    pub audio_selector: String,
    pub is_muxed_video: bool,
    pub video_ext: String,
    pub audio_ext: String,
    pub merge_output_container: Option<String>,
    pub output_extension_override: Option<String>,
    pub upload_date: String,
    pub subtitle_lang: Option<String>,
    pub subtitle_ext: String,
    pub subtitle_source_ext: String,
    pub subtitle_url: Option<String>,
    pub write_auto_subs: bool,
    pub subtitle_is_auto_translated: bool,
    pub write_subtitles: bool,
    pub embed_subtitles: bool,
    pub write_chapters: bool,
    pub embed_chapters: bool,
    pub write_thumbnail: bool,
    pub embed_thumbnail: bool,
    pub use_cookies: bool,
    pub use_aria2: bool,
    pub emit_json: bool,
    pub output_path: Option<String>,
    pub output_dir: String,
    pub file_name: String,
    pub download_section_args: Vec<String>,
}

pub struct PreparedDownload {
    pub output_transaction:
        Option<std::sync::Arc<std::sync::Mutex<super::output_transaction::OutputTransaction>>>,
    pub command: Command,
    pub output_path: PathBuf,
    pub command_line: String,
}

pub struct AnalyzeOutput {
    pub json: Value,
    pub command_line: String,
}

pub struct AnalyzeError {
    pub message: String,
    pub command_line: Option<String>,
}

impl AnalyzeError {
    fn new(message: impl Into<String>, command_line: Option<String>) -> Self {
        Self {
            message: message.into(),
            command_line,
        }
    }
}

#[derive(Clone)]
pub(crate) struct OutputPlan {
    output_argument: String,
    final_output_path: PathBuf,
    remux_extension: Option<String>,
    extract_audio: bool,
    audio_format: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownloadTargetKind {
    Normal,
    Video,
    Audio,
    Subtitle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedCookieSource {
    File(PathBuf),
    Browser(String),
}

/// Parsed once for an output operation; external tools still read `path` themselves.
#[derive(Clone, Default)]
pub(crate) struct EffectiveYtDlpConfig {
    pub(crate) path: Option<PathBuf>,
    explicit: bool,
    options: Vec<ConfigOption>,
    pub(crate) output: (Option<String>, Option<String>),
}
impl EffectiveYtDlpConfig {
    fn owns(&self, names: &[&str]) -> bool {
        self.options
            .iter()
            .any(|option| names.contains(&option.name.as_str()))
    }
}
#[cfg(any(test, cache_review_gui))]
static CONFIG_READS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<PathBuf, [usize; 3]>>,
> = std::sync::OnceLock::new();
#[cfg(test)]
pub(crate) fn output_config_read_count(path: &Path) -> usize {
    output_config_observation_counts(path)[0]
}
#[cfg(any(test, cache_review_gui))]
pub(crate) fn output_config_observation_counts(path: &Path) -> [usize; 3] {
    CONFIG_READS
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .get(path)
        .copied()
        .unwrap_or_default()
}
#[cfg(any(test, cache_review_gui))]
fn record_config_observation(path: &Path, phase: usize) {
    CONFIG_READS
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .entry(path.to_owned())
        .or_default()[phase] += 1;
}

#[derive(Clone)]
pub struct ToolPaths {
    pub(crate) output_config: Option<std::sync::Arc<EffectiveYtDlpConfig>>,
    pub yt_dlp: String,
    pub gallery_dl: String,
    pub yt_dlp_config: String,
    pub ffmpeg: String,
    pub aria2c: String,
    pub deno: String,
    pub cache_mode: CacheLocationMode,
    pub cache_dir: String,
    pub browser_cookie_source: String,
    pub browser_cookie_profile: String,
    pub browser_cookie_file: String,
    pub youtube_subs_po_token: String,
    pub youtube_extractor_args: String,
    pub concurrent_fragments: usize,
    pub live_from_start: bool,
    pub proxy_enabled: bool,
    pub proxy_url: String,
    pub no_check_certificates: bool,
    pub impersonate: String,
    pub limit_rate: String,
    pub download_sections: String,
    pub chapter_compatibility_mode: bool,
    pub file_time_mode: FileTimeMode,
}

impl Default for ToolPaths {
    fn default() -> Self {
        Self {
            output_config: None,
            yt_dlp: ".\\tools\\yt-dlp\\yt-dlp.exe".to_owned(),
            gallery_dl: ".\\tools\\gallery-dl\\gallery-dl.exe".to_owned(),
            yt_dlp_config: String::new(),
            ffmpeg: ".\\tools\\ffmpeg\\ffmpeg.exe".to_owned(),
            aria2c: ".\\tools\\aria2c\\aria2c.exe".to_owned(),
            deno: ".\\tools\\deno\\deno.exe".to_owned(),
            cache_mode: CacheLocationMode::V2Cache,
            cache_dir: ".\\cache".to_owned(),
            browser_cookie_source: "chrome".to_owned(),
            browser_cookie_profile: String::new(),
            browser_cookie_file: String::new(),
            youtube_subs_po_token: String::new(),
            youtube_extractor_args: String::new(),
            concurrent_fragments: 1,
            live_from_start: true,
            proxy_enabled: false,
            proxy_url: String::new(),
            no_check_certificates: false,
            impersonate: String::new(),
            limit_rate: String::new(),
            download_sections: String::new(),
            chapter_compatibility_mode: true,
            file_time_mode: FileTimeMode::None,
        }
    }
}

fn apply_analyze_playlist_scope(command: &mut Command, playlist_item_limit: Option<usize>) {
    if let Some(limit) = playlist_item_limit {
        command
            .arg("--yes-playlist")
            .arg("--playlist-end")
            .arg(limit.max(1).to_string());
    } else {
        command.arg("--no-playlist");
    }
}

fn apply_download_playlist_index_arg(command: &mut Command, playlist_index: Option<usize>) {
    if let Some(index) = playlist_index.filter(|index| *index > 0) {
        command.arg("--playlist-items").arg(index.to_string());
    }
}

fn apply_analysis_selection_args(
    command: &mut Command,
    playlist_item_limit: Option<usize>,
    format_selector: Option<&str>,
) {
    apply_analyze_playlist_scope(command, playlist_item_limit);
    if let Some(selector) = format_selector.filter(|value| !value.trim().is_empty()) {
        command.arg("-f").arg(selector);
    }
}

impl ToolPaths {
    pub(crate) fn for_output(
        &self,
        output: &super::output_policy::OutputRequest,
    ) -> Result<Self, String> {
        if self.output_config.is_some() {
            return Ok(self.clone());
        }
        let retained = if let Some(owner) = &output.transaction {
            owner
                .lock()
                .map_err(|_| "Output owner unavailable")?
                .output_config
                .clone()
        } else {
            None
        };
        let Some(config) = retained else {
            return self.bind_protected_config();
        };
        let mut tools = self.clone();
        tools.output_config = Some(config);
        Ok(tools)
    }

    pub(crate) fn bind_protected_config(&self) -> Result<Self, String> {
        let mut tools = self.clone();
        if tools.output_config.is_none() {
            tools.output_config = Some(std::sync::Arc::new(self.read_effective_config(true)?));
        }
        Ok(tools)
    }

    fn prepare_metadata_command(&self, tool: &Path) -> Result<(Self, Command), String> {
        let tools = self.bind_protected_config()?;
        let mut command = Command::new(tool);
        configure_background_command(&mut command);
        tools.apply_common_yt_dlp_args(&mut command)?;
        // Config tokens precede CLI tokens. Metadata intent must override an
        // explicit --no-simulate rather than rely on JSON dumping's default.
        command.args(["--simulate", "--skip-download"]);
        Ok((tools, command))
    }
    fn read_effective_config(&self, protected: bool) -> Result<EffectiveYtDlpConfig, String> {
        let explicit = self.explicit_config_path();
        let path = explicit.clone().or_else(|| self.portable_config_path());
        let Some(path) = path else {
            return Ok(EffectiveYtDlpConfig::default());
        };
        #[cfg(any(test, cache_review_gui))]
        record_config_observation(&path, 0);
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Could not inspect yt-dlp configuration: {e}"))?;
        #[cfg(any(test, cache_review_gui))]
        record_config_observation(&path, 1);
        let arguments = config_arguments(&content)?;
        let options = config_options::interpret(&arguments)?;
        let output = if protected {
            #[cfg(any(test, cache_review_gui))]
            record_config_observation(&path, 2);
            validate_external_output_arguments(&options)?;
            output_preparation::external_output(&options)?
        } else {
            (None, None)
        };
        Ok(EffectiveYtDlpConfig {
            path: Some(path),
            explicit: explicit.is_some(),
            options,
            output,
        })
    }
    fn config_owns(&self, names: &[&str]) -> bool {
        if let Some(config) = &self.output_config {
            return config.owns(names);
        }
        self.read_effective_config(false)
            .is_ok_and(|c| c.owns(names))
    }

    pub(crate) fn music_output_extension(
        &self,
        url: &str,
        selector: &str,
        cookies: bool,
        cancel: &AtomicBool,
    ) -> Result<String, String> {
        let metadata = self.analyze_music_output(url, cookies, selector, cancel)?;
        let extension = metadata
            .json
            .get("ext")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or("Resolved audio has no output extension.")?;
        // Original mode retains the selected source container (no -x option).
        Ok(extension.to_owned())
    }
    pub(crate) fn analyze_music_output(
        &self,
        url: &str,
        cookies: bool,
        selector: &str,
        cancel: &AtomicBool,
    ) -> Result<AnalyzeOutput, String> {
        let tools = self.bind_protected_config()?;
        let tool = tools.validate_yt_dlp_available()?;
        // Match the music download's external-config format precedence.
        let selector = (!tools.effective_config_owns_format()).then_some(selector);
        tools
            .run_analyze_command_cancellable(&tool, url, cookies, selector, None, None, cancel)
            .map_err(|e| e.message)
    }
    pub(crate) fn music_output_destination(
        &self,
        output: &super::output_policy::OutputRequest,
        url: &str,
        directory: &Path,
        literal: &str,
        use_cookies: bool,
    ) -> Result<(PathBuf, String), String> {
        if self.output_config.is_none() {
            return self.for_output(output)?.music_output_destination(
                output,
                url,
                directory,
                literal,
                use_cookies,
            );
        }
        output_preparation::music_destination(self, output, url, directory, literal, use_cookies)
    }
    pub fn has_explicit_config(&self) -> bool {
        self.output_config.as_ref().map_or_else(
            || {
                !self.yt_dlp_config.trim().is_empty()
                    && resolve_support_path(&self.yt_dlp_config).is_file()
            },
            |c| c.explicit,
        )
    }

    pub fn effective_config_path(&self) -> Option<PathBuf> {
        self.output_config.as_ref().map_or_else(
            || {
                self.explicit_config_path()
                    .or_else(|| self.portable_config_path())
            },
            |c| c.path.clone(),
        )
    }

    pub fn effective_config_owns_output(&self) -> bool {
        self.config_owns(&["--output", "--paths"])
    }

    pub fn effective_config_owns_format(&self) -> bool {
        self.config_owns(&["--format"])
    }

    pub fn effective_config_owns_limit_rate(&self) -> bool {
        self.config_owns(&["--limit-rate"])
    }

    pub fn effective_config_owns_download_sections(&self) -> bool {
        self.config_owns(&["--download-sections"])
    }

    pub fn effective_config_owns_live_from_start(&self) -> bool {
        self.config_owns(&["--live-from-start", "--no-live-from-start"])
    }

    pub fn effective_config_owns_mtime(&self) -> bool {
        self.config_owns(&["--mtime", "--no-mtime"])
    }

    pub fn effective_proxy_url(&self) -> Option<&str> {
        let proxy_url = self.proxy_url.trim();
        (self.proxy_enabled && !proxy_url.is_empty()).then_some(proxy_url)
    }

    pub fn aria2_is_available(&self) -> bool {
        resolve_tool_path(&self.aria2c).is_file()
    }

    pub fn append_aria2_downloader_args(&self, command: &mut Command) -> bool {
        let aria2_path = resolve_tool_path(&self.aria2c);
        if !aria2_path.is_file() {
            return false;
        }
        command.arg("--downloader").arg(aria2_path);
        if let Some(args) =
            aria2_downloader_args(self.effective_proxy_url(), self.no_check_certificates)
        {
            command.arg("--downloader-args").arg(args);
        }
        true
    }

    pub fn available_browser_cookie_sources(&self) -> Vec<BrowserCookieSourceOption> {
        let mut items = Vec::new();
        items.push(BrowserCookieSourceOption {
            value: COOKIE_SOURCE_AUTO,
            label: "Auto by website",
        });
        items.push(BrowserCookieSourceOption {
            value: COOKIE_SOURCE_FILE,
            label: "Use file (cookies.txt)",
        });
        let mut has_browser_source = false;
        for option in browser_cookie_source_candidates() {
            if browser_cookie_source_exists(option.value) {
                has_browser_source = true;
                items.push(option);
            }
        }
        if !has_browser_source {
            items.push(BrowserCookieSourceOption {
                value: "chrome",
                label: "Chrome",
            });
        }
        items
    }

    pub fn available_browser_cookie_profiles(&self) -> Vec<BrowserCookieProfileOption> {
        if self.uses_cookie_file_source() || self.uses_auto_cookie_source() {
            return Vec::new();
        }
        browser_cookie_profiles(&self.browser_cookie_source)
    }

    pub fn prepare_batch_add_command(
        &self,
        url: &str,
        use_cookies: bool,
    ) -> Result<Command, String> {
        self.prepare_playlist_discovery_command(url, use_cookies, None)
    }

    pub(crate) fn prepare_playlist_discovery_command(
        &self,
        url: &str,
        use_cookies: bool,
        item_limit: Option<usize>,
    ) -> Result<Command, String> {
        let tool_path = self.validate_yt_dlp_available()?;

        let (tools, mut command) = self.prepare_metadata_command(&tool_path)?;
        tools.apply_cookie_args(&mut command, use_cookies, Some(url))?;
        command
            .arg("--flat-playlist")
            .arg("--dump-json")
            .arg("--lazy-playlist")
            .arg("--no-warnings")
            .arg("--ignore-errors");
        if let Some(limit) = item_limit {
            command
                .arg("--yes-playlist")
                .arg("--playlist-end")
                .arg(limit.max(1).to_string());
        }
        command.arg(url);

        eprintln!(
            "[batch-add] command: {}",
            format_command_line(&tool_path, &command)
        );

        Ok(command)
    }

    pub fn prepare_music_flat_update_command(
        &self,
        url: &str,
        use_cookies: bool,
    ) -> Result<Command, String> {
        let tool_path = self.validate_yt_dlp_available()?;

        let (tools, mut command) = self.prepare_metadata_command(&tool_path)?;
        tools.apply_cookie_args(&mut command, use_cookies, Some(url))?;
        command
            .arg("--flat-playlist")
            .arg("--dump-json")
            .arg("--lazy-playlist")
            .arg("--playlist-items")
            .arg("1")
            .arg("--no-warnings")
            .arg("--ignore-errors")
            .arg(url);

        eprintln!(
            "[music-flat-update] command: {}",
            format_command_line(&tool_path, &command)
        );

        Ok(command)
    }

    pub fn analyze_url(&self, url: &str, use_cookies: bool) -> Result<AnalyzeOutput, String> {
        self.analyze_url_detailed(url, use_cookies)
            .map_err(|error| error.message)
    }

    pub fn analyze_url_detailed(
        &self,
        url: &str,
        use_cookies: bool,
    ) -> Result<AnalyzeOutput, AnalyzeError> {
        let tool_path = self
            .validate_yt_dlp_available()
            .map_err(|error| AnalyzeError::new(error, None))?;

        self.run_analyze_command(&tool_path, url, use_cookies, None, None, None)
    }

    /// Desktop-only classification probe. Native extension workflows stay unchanged.
    pub(crate) fn prepare_source_probe_command(
        &self,
        url: &str,
        use_cookies: bool,
    ) -> Result<(Command, String), String> {
        let tool_path = self.validate_yt_dlp_available()?;
        let (tools, mut command) = self.prepare_metadata_command(&tool_path)?;
        tools.apply_cookie_args(&mut command, use_cookies, Some(url))?;
        tools.apply_youtube_extractor_args(&mut command);
        command
            .args([
                "--flat-playlist",
                "--playlist-end",
                "2",
                "--dump-single-json",
                "--no-warnings",
            ])
            .arg(url);
        let command_line = format_command_line(&tool_path, &command);
        Ok((command, command_line))
    }

    pub(crate) fn analyze_queue_item_detailed(
        &self,
        url: &str,
        use_cookies: bool,
        playlist_index: Option<usize>,
    ) -> Result<AnalyzeOutput, AnalyzeError> {
        if playlist_index.is_none() {
            return super::yt_dlp_source_analysis::analyze_desktop_source(self, url, use_cookies)
                .and_then(|output| match output {
                    super::yt_dlp_source_analysis::DesktopSourceAnalysis::Single(output) => Ok(output),
                    super::yt_dlp_source_analysis::DesktopSourceAnalysis::Playlist => Err(AnalyzeError::new(
                        "Saved source is a playlist without an entry index. Import it through Standard Mode Batch instead.", None)),
                });
        }
        let tool_path = self
            .validate_yt_dlp_available()
            .map_err(|error| AnalyzeError::new(error, None))?;
        self.run_analyze_command(&tool_path, url, use_cookies, None, None, playlist_index)
    }

    pub(crate) fn analyze_output_naming(
        &self,
        url: &str,
        cookies: bool,
        index: Option<usize>,
        cancel: &AtomicBool,
    ) -> Result<AnalyzeOutput, AnalyzeError> {
        if index.is_none() {
            return super::yt_dlp_source_analysis::analyze_desktop_source_cancellable(
                self, url, cookies, cancel,
            )
            .and_then(|output| match output {
                super::yt_dlp_source_analysis::DesktopSourceAnalysis::Single(output) => Ok(output),
                _ => Err(AnalyzeError::new(
                    "Saved source is a playlist without an entry index.",
                    None,
                )),
            });
        }
        let tool = self
            .validate_yt_dlp_available()
            .map_err(|e| AnalyzeError::new(e, None))?;
        self.run_analyze_command_cancellable(&tool, url, cookies, None, None, index, cancel)
    }

    pub fn analyze_playlist_url_detailed(
        &self,
        url: &str,
        use_cookies: bool,
        item_limit: usize,
    ) -> Result<AnalyzeOutput, AnalyzeError> {
        let mut items = Vec::new();
        let summary = self.analyze_playlist_url_streaming_detailed(
            url,
            use_cookies,
            item_limit,
            &AtomicBool::new(false),
            |item| {
                items.push(item);
                Ok(())
            },
        )?;
        Ok(AnalyzeOutput {
            json: serde_json::json!({
                "_type": "playlist", "entries": items,
                "analysisTruncated": summary.truncated,
                "analysisWarning": summary.warning,
            }),
            command_line: summary.command_line,
        })
    }

    pub(crate) fn analyze_playlist_url_streaming_detailed(
        &self,
        url: &str,
        use_cookies: bool,
        item_limit: usize,
        cancelled: &AtomicBool,
        on_item: impl FnMut(Value) -> Result<(), String>,
    ) -> Result<super::yt_dlp_playlist_analysis::PlaylistAnalysisSummary, AnalyzeError> {
        super::yt_dlp_playlist_analysis::analyze_playlist_formats(
            self,
            url,
            use_cookies,
            item_limit,
            cancelled,
            on_item,
        )
    }

    pub fn analyze_music_stream_url(
        &self,
        url: &str,
        use_cookies: bool,
    ) -> Result<AnalyzeOutput, String> {
        self.analyze_music_stream_url_with_selector(url, use_cookies, MUSIC_STREAM_FORMAT_SELECTOR)
    }

    pub fn analyze_music_stream_url_detailed(
        &self,
        url: &str,
        use_cookies: bool,
    ) -> Result<AnalyzeOutput, AnalyzeError> {
        self.analyze_music_stream_url_with_selector_detailed(
            url,
            use_cookies,
            MUSIC_STREAM_FORMAT_SELECTOR,
        )
    }

    pub fn analyze_music_stream_url_with_selector(
        &self,
        url: &str,
        use_cookies: bool,
        format_selector: &str,
    ) -> Result<AnalyzeOutput, String> {
        self.analyze_music_stream_url_with_selector_detailed(url, use_cookies, format_selector)
            .map_err(|error| error.message)
    }

    pub fn analyze_music_stream_url_with_selector_detailed(
        &self,
        url: &str,
        use_cookies: bool,
        format_selector: &str,
    ) -> Result<AnalyzeOutput, AnalyzeError> {
        let tools = self
            .bind_protected_config()
            .map_err(|error| AnalyzeError::new(error, None))?;
        // A saved/audio source can bypass Add. Do not resolve every creator item
        // merely to obtain one stream; retain the explicit audio selector below.
        if matches!(
            super::yt_dlp_source_analysis::preflight_desktop_source(&tools, url, use_cookies)?,
            super::yt_dlp_source_analysis::DesktopSourcePreflight::Playlist
        ) {
            return Err(AnalyzeError::new(
                "Audio playback requires a single track. Import this playlist first.",
                None,
            ));
        }
        let tool_path = tools
            .validate_yt_dlp_available()
            .map_err(|error| AnalyzeError::new(error, None))?;
        tools.run_analyze_command(
            &tool_path,
            url,
            use_cookies,
            Some(format_selector.trim()).filter(|value| !value.is_empty()),
            None,
            None,
        )
    }

    pub fn prepare_music_stream_cache_command(
        &self,
        url: &str,
        output_path: &Path,
        format_selector: &str,
        use_cookies: bool,
    ) -> Result<Command, String> {
        if self.output_config.is_none() {
            return self
                .for_output(&super::output_policy::OutputRequest::default())?
                .prepare_music_stream_cache_command(
                    url,
                    output_path,
                    format_selector,
                    use_cookies,
                );
        }
        let _storage_use = super::storage_paths::use_cache()?;
        let tool_path = self.validate_yt_dlp_available()?;
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create music cache folder: {error}"))?;
        }

        let mut command = Command::new(&tool_path);
        configure_background_command(&mut command);
        self.apply_common_yt_dlp_args(&mut command)?;
        command
            .arg("--no-playlist")
            .arg("--no-simulate")
            .arg("--no-part")
            .arg("--force-overwrites")
            .arg("--windows-filenames")
            .arg("--newline")
            .arg("--progress");
        apply_output_path(&mut command, &output_path.display().to_string())?;
        // Growing-cache playback reads this file before download completion.
        if let Some(parent) = output_path.parent() {
            command.arg("-P").arg(format!("temp:{}", parent.display()));
        }

        let selector = format_selector.trim();
        if !self.effective_config_owns_format() {
            command.arg("--format").arg(if selector.is_empty() {
                MUSIC_STREAM_FORMAT_SELECTOR
            } else {
                selector
            });
        }

        self.apply_cookie_args(&mut command, use_cookies, Some(url))?;
        self.apply_youtube_extractor_args(&mut command);
        command.arg(url);

        let command_line = format_command_line(&tool_path, &command);
        command
            .env("PYTHONIOENCODING", "utf-8")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        eprintln!("[music-cache] command: {command_line}");
        Ok(command)
    }

    pub fn prepare_music_audio_download_command(
        &self,
        url: &str,
        output_dir: &Path,
        audio_format: Option<&str>,
        format_selector: &str,
        embed_cover: bool,
        write_tags: bool,
        chapter_output_dir: Option<&Path>,
        use_cookies: bool,
        output: &super::output_policy::OutputRequest,
    ) -> Result<PreparedDownload, String> {
        let tool_path = self.validate_yt_dlp_available()?;
        if self.output_config.is_none() {
            return self
                .for_output(output)?
                .prepare_music_audio_download_command(
                    url,
                    output_dir,
                    audio_format,
                    format_selector,
                    embed_cover,
                    write_tags,
                    chapter_output_dir,
                    use_cookies,
                    output,
                );
        }
        if !output.validate_work(output_dir)? {
            return Err("Music output requires an owned staging transaction.".into());
        }
        if let Some(chapters) = chapter_output_dir {
            output.validate_work(chapters)?;
        }
        fs::create_dir_all(output_dir)
            .map_err(|error| format!("Could not create music download folder: {error}"))?;

        let target_format = audio_format.and_then(normalized_extension);
        let output_template = output_dir.join("output.%(ext)s").display().to_string();
        let expected_output = target_format
            .as_ref()
            .map(|format| output_dir.join(format!("output.{format}")))
            .unwrap_or_else(|| output_dir.join("output.%(ext)s"));

        if let Some(chapter_output_dir) = chapter_output_dir {
            fs::create_dir_all(chapter_output_dir)
                .map_err(|error| format!("Could not create chapter output folder: {error}"))?;
        }

        let mut command = Command::new(&tool_path);
        configure_background_command(&mut command);
        self.apply_common_yt_dlp_args(&mut command)?;
        command
            .arg("--no-playlist")
            .arg("--continue")
            .arg("--force-overwrites")
            .arg("--windows-filenames")
            .arg("--progress")
            .arg("--newline")
            .arg("--progress-template")
            .arg("[yt-dlp],%(progress._percent_str)s,%(progress._eta_str)s,%(progress.downloaded_bytes)s,%(progress.total_bytes)s,%(progress.speed)s,%(progress.eta)s")
            .arg("--no-simulate");

        if let Some(target_format) = target_format.as_ref() {
            command
                .arg("--extract-audio")
                .arg("--audio-format")
                .arg(target_format)
                .arg("--audio-quality")
                .arg("0");
        }

        if write_tags {
            command
                .arg("--embed-metadata")
                .arg("--metadata-from-title")
                .arg("%(artist)s - %(title)s");
            for parse_metadata_arg in music_album_parse_metadata_args() {
                command.arg("--parse-metadata").arg(parse_metadata_arg);
            }
        }

        if embed_cover
            && target_format
                .as_deref()
                .is_some_and(music_audio_download_format_supports_thumbnail)
        {
            command
                .arg("--embed-thumbnail")
                .arg("--convert-thumbnails")
                .arg("jpg");
        }

        apply_music_chapter_split_args(&mut command, chapter_output_dir);

        command
            .arg("--print")
            .arg(format!("after_move:{FINAL_OUTPUT_PATH_PREFIX}%(filepath)j"));
        apply_output_path(&mut command, &output_template)?;
        command
            .arg("-P")
            .arg(format!("temp:{}", output_dir.join(".incoming").display()));

        if !self.effective_config_owns_format() {
            let selector = if format_selector.trim().is_empty() {
                target_format
                    .as_deref()
                    .map(music_audio_download_format_selector)
                    .unwrap_or(MUSIC_STREAM_FORMAT_SELECTOR)
            } else {
                format_selector
            };
            command.arg("--format").arg(selector);
        }

        self.apply_cookie_args(&mut command, use_cookies, Some(url))?;
        self.apply_youtube_extractor_args(&mut command);
        command.arg(url);

        let command_line = format_command_line(&tool_path, &command);
        command
            .env("PYTHONIOENCODING", "utf-8")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        Ok(PreparedDownload {
            command,
            output_path: expected_output,
            command_line,
            output_transaction: output.transaction.clone(),
        })
    }

    pub fn prepare_music_lyrics_cache_command(
        &self,
        url: &str,
        lyrics_dir: &Path,
        language_code: &str,
        use_cookies: bool,
    ) -> Result<Command, String> {
        if self.output_config.is_none() {
            return self
                .for_output(&super::output_policy::OutputRequest::default())?
                .prepare_music_lyrics_cache_command(url, lyrics_dir, language_code, use_cookies);
        }
        let tool_path = self.validate_yt_dlp_available()?;
        fs::create_dir_all(lyrics_dir)
            .map_err(|error| format!("Could not create lyrics cache folder: {error}"))?;

        let output_template = lyrics_dir.join("lyrics.%(ext)s").display().to_string();
        let mut command = Command::new(&tool_path);
        configure_background_command(&mut command);
        self.apply_common_yt_dlp_args(&mut command)?;
        command
            .arg("--no-playlist")
            .arg("--skip-download")
            .arg("--force-overwrites")
            .arg("--windows-filenames")
            .arg("--write-subs")
            .arg("--sub-langs")
            .arg(language_code.trim())
            .arg("--sub-format")
            .arg("vtt/srt/ttml/srv3/srv2/srv1/best")
            .arg("--convert-subs")
            .arg("lrc");
        apply_output_path(&mut command, &output_template)?;

        self.apply_cookie_args(&mut command, use_cookies, Some(url))?;
        self.apply_youtube_extractor_args(&mut command);
        command.arg(url);

        command
            .env("PYTHONIOENCODING", "utf-8")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        eprintln!(
            "[music-lyrics] command: {}",
            format_command_line(&tool_path, &command)
        );
        Ok(command)
    }

    pub fn prepare_download(&self, request: &DownloadRequest) -> Result<PreparedDownload, String> {
        let tool_path = self.validate_yt_dlp_available()?;
        if self.output_config.is_none() {
            return self.for_output(&request.output)?.prepare_download(request);
        }

        let (output_plan, output_transaction) = output_preparation::prepare_plan(self, request)?;

        let mut command = Command::new(&tool_path);
        configure_background_command(&mut command);
        self.apply_common_yt_dlp_args(&mut command)?;
        command
            .arg("--no-playlist")
            .arg("--force-overwrites")
            .arg("--windows-filenames")
            .arg("--progress")
            .arg("--newline")
            .arg("--progress-template")
            .arg("[yt-dlp],%(info.format_id)s,%(progress._percent_str)s,%(progress._eta_str)s,%(progress.downloaded_bytes)s,%(progress.total_bytes)s,%(progress.speed)s,%(progress.eta)s");
        apply_download_playlist_index_arg(&mut command, request.playlist_index);
        if request.download_section_args.len() > 1 {
            command.args(["--autonumber-start", "1"]);
        }

        if request.target_kind != DownloadTargetKind::Subtitle {
            command
                .arg("--no-simulate")
                .arg("--print")
                .arg(format!("after_move:{FINAL_OUTPUT_PATH_PREFIX}%(filepath)j"));
        }

        apply_output_path(&mut command, &output_plan.output_argument)?;
        let work = output_transaction
            .lock()
            .map_err(|_| "Output transaction unavailable")?
            .work
            .clone();
        command
            .arg("-P")
            .arg(format!("temp:{}", work.join(".incoming").display()));
        for kind in [
            "thumbnail",
            "subtitle",
            "chapter",
            "description",
            "infojson",
            "link",
            "pl_thumbnail",
            "pl_description",
            "pl_infojson",
        ] {
            command.arg("-P").arg(format!("{kind}:{}", work.display()));
        }

        if request.target_kind != DownloadTargetKind::Subtitle
            && !request.use_aria2
            && !self.has_explicit_config()
            && self.concurrent_fragments > 1
        {
            command
                .arg("--concurrent-fragments")
                .arg(self.concurrent_fragments.to_string());
        }

        apply_live_from_start_arg(
            &mut command,
            request.target_kind,
            self.live_from_start,
            self.effective_config_owns_live_from_start(),
        );

        if request.target_kind != DownloadTargetKind::Subtitle
            && !self.effective_config_owns_limit_rate()
            && !self.limit_rate.trim().is_empty()
        {
            command.arg("--limit-rate").arg(self.limit_rate.trim());
        }

        let has_item_download_section = apply_download_section_args(
            &mut command,
            request.target_kind,
            &request.download_section_args,
            self.effective_config_owns_download_sections(),
        );

        let use_section_compatibility_mode = has_item_download_section
            && self.chapter_compatibility_mode
            && !self.has_explicit_config();

        if use_section_compatibility_mode {
            // Section downloads can trigger yt-dlp's direct-merge path, where the
            // process appears to stop at `[download] Destination:` and does not
            // emit normal per-format progress.  Force the classic separate
            // download + merge path for item-level chapter/range downloads.
            command.arg("--compat-options").arg("no-direct-merge");
        }

        if !self.effective_config_owns_mtime()
            && self.file_time_mode == FileTimeMode::UseDownloadTime
        {
            command.arg("--no-mtime");
        }

        if request.target_kind == DownloadTargetKind::Subtitle {
            command.arg("--skip-download");
        }

        if request.emit_json {
            command.arg("--print-json");
        }

        if output_plan.extract_audio {
            command.arg("--extract-audio");
            if let Some(audio_format) = output_plan.audio_format.as_deref() {
                command.arg("--audio-format").arg(audio_format);
            }
        }

        if request.target_kind != DownloadTargetKind::Subtitle
            && !self.effective_config_owns_format()
        {
            let requested_selector = request.format_selector.trim();
            let use_section_safe_selector = use_section_compatibility_mode
                && request.target_kind == DownloadTargetKind::Normal
                && (requested_selector.is_empty()
                    || selector_uses_separate_streams(requested_selector));
            let format_selector = if use_section_safe_selector {
                // Time-range downloads are handled by FFmpeg.  YouTube DASH
                // video+audio pairs can sit at `[download] Destination:` for a
                // long time with no visible progress, so prefer a muxed single
                // file for item-level chapter/range downloads.
                SECTION_FORMAT_SELECTOR
            } else if requested_selector.is_empty() {
                DEFAULT_FORMAT_SELECTOR
            } else {
                requested_selector
            };
            command.arg("--format").arg(format_selector);
        }

        apply_merge_output_container_arg(
            &mut command,
            request.target_kind,
            request.merge_output_container.as_deref(),
        );

        let forces_webm_container = request.target_kind == DownloadTargetKind::Normal
            && request
                .output_extension_override
                .as_deref()
                .or(request.merge_output_container.as_deref())
                .is_some_and(|container| container.trim().eq_ignore_ascii_case("webm"));
        if forces_webm_container {
            // WebM cannot carry the cover-art attachment used by yt-dlp's
            // thumbnail embedder. The explicit per-item container choice owns
            // this conflict even when a loaded config requests embedding.
            command.arg("--no-embed-thumbnail");
        }

        if !self.has_explicit_config() {
            if let Some(remux_extension) = output_plan.remux_extension.as_deref() {
                command.arg("--remux-video").arg(remux_extension);
            }
        }

        self.apply_cookie_args(
            &mut command,
            request.use_cookies,
            Some(request.url.as_str()),
        )?;
        self.apply_youtube_extractor_args(&mut command);

        if !self.has_explicit_config() && request.use_aria2 {
            self.append_aria2_downloader_args(&mut command);
        }

        let should_write_subtitles = request.target_kind == DownloadTargetKind::Subtitle
            || (request.write_subtitles && !self.has_explicit_config());
        apply_subtitle_download_args(&mut command, request, should_write_subtitles);

        if !self.has_explicit_config() {
            if request.target_kind != DownloadTargetKind::Subtitle {
                super::yt_dlp_chapters::apply_chapter_choices(
                    &mut command,
                    request.write_chapters,
                    request.embed_chapters,
                );
            }

            if request.write_thumbnail && request.embed_thumbnail && !forces_webm_container {
                command
                    .arg("--embed-thumbnail")
                    .arg("--convert-thumbnails")
                    .arg("jpg");
            } else if request.write_thumbnail {
                command
                    .arg("--write-thumbnail")
                    .arg("--convert-thumbnails")
                    .arg("jpg");
            }
        }

        command.arg(&request.url);

        let command_line = format_command_line(&tool_path, &command);
        command
            .env("PYTHONIOENCODING", "utf-8")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        Ok(PreparedDownload {
            command,
            output_path: output_plan.final_output_path,
            command_line,
            output_transaction: Some(output_transaction),
        })
    }

    pub fn validate_yt_dlp_available(&self) -> Result<PathBuf, String> {
        super::storage_paths::require_ready()?;
        let tool_path = resolve_tool_path(&self.yt_dlp);
        if tool_path.is_file() {
            return Ok(tool_path);
        }

        Err(format!(
            "yt-dlp was not found: {}. Install yt-dlp first, or handle dependency deployment in Options.",
            tool_path.display()
        ))
    }

    pub fn validate_cookie_setup(&self, use_cookies: bool) -> Result<(), String> {
        self.resolve_cookie_source(use_cookies, None, true)
            .map(|_| ())
    }

    fn apply_cookie_args(
        &self,
        command: &mut Command,
        use_cookies: bool,
        url: Option<&str>,
    ) -> Result<(), String> {
        let source = self.resolve_cookie_source(use_cookies, url, true)?;
        match source {
            Some(ResolvedCookieSource::File(path)) => {
                command.arg("--cookies").arg(path);
            }
            Some(ResolvedCookieSource::Browser(browser)) => {
                command.arg("--cookies-from-browser").arg(browser);
            }
            None => {}
        }
        Ok(())
    }

    pub(crate) fn resolve_shared_cookie_source(
        &self,
        use_cookies: bool,
        url: Option<&str>,
    ) -> Result<Option<ResolvedCookieSource>, String> {
        self.resolve_cookie_source(use_cookies, url, false)
    }

    fn resolve_cookie_source(
        &self,
        use_cookies: bool,
        url: Option<&str>,
        explicit_yt_dlp_config_owns_cookies: bool,
    ) -> Result<Option<ResolvedCookieSource>, String> {
        if !use_cookies || (explicit_yt_dlp_config_owns_cookies && self.has_explicit_config()) {
            return Ok(None);
        }

        if self.uses_auto_cookie_source() {
            return url
                .map(auto_cookie_file_for_url)
                .transpose()
                .map(Option::flatten)
                .map(|path| path.map(ResolvedCookieSource::File));
        }

        if self.uses_cookie_file_source() {
            return self.cookie_file_path()?.map(ResolvedCookieSource::File).map_or_else(
                || {
                    Err("Cookies are enabled and the cookie source is file, but no valid Netscape cookies.txt is selected.".to_owned())
                },
                |source| Ok(Some(source)),
            );
        }

        let browser =
            cookie_source_argument(&self.browser_cookie_source, &self.browser_cookie_profile);
        if browser.trim().is_empty() {
            return Err(
                "Cookies are enabled, but no browser or cookies.txt source is selected.".to_owned(),
            );
        }
        Ok(Some(ResolvedCookieSource::Browser(browser)))
    }

    fn uses_auto_cookie_source(&self) -> bool {
        self.browser_cookie_source
            .trim()
            .eq_ignore_ascii_case(COOKIE_SOURCE_AUTO)
    }

    fn uses_cookie_file_source(&self) -> bool {
        self.browser_cookie_source
            .trim()
            .eq_ignore_ascii_case(COOKIE_SOURCE_FILE)
    }

    fn apply_youtube_extractor_args(&self, command: &mut Command) {
        if self.has_explicit_config() {
            return;
        }

        if let Some(args) = self.youtube_extractor_args_argument() {
            command.arg("--extractor-args").arg(args);
        }
    }

    fn youtube_extractor_args_argument(&self) -> Option<String> {
        let mut args = Vec::new();
        if let Some(po_token) = normalized_youtube_subs_po_token(&self.youtube_subs_po_token) {
            args.push(format!("po_token={po_token}"));
        }
        if let Some(raw_args) = normalized_youtube_extractor_args(&self.youtube_extractor_args) {
            args.push(raw_args);
        }

        if args.is_empty() {
            None
        } else {
            Some(format!("youtube:{}", args.join(";")))
        }
    }

    fn cookie_file_path(&self) -> Result<Option<PathBuf>, String> {
        let trimmed = self.browser_cookie_file.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let path = resolve_support_path(trimmed);
        if path.is_file() {
            Ok(Some(path))
        } else {
            Err(format!(
                "Cookie file was not found: {}. Choose a Netscape cookies.txt again, or change the cookie source back to browser.",
                path.display()
            ))
        }
    }

    fn apply_common_yt_dlp_args(&self, command: &mut Command) -> Result<(), String> {
        let _storage_use = super::storage_paths::use_cache()?;
        let local;
        let config = if let Some(config) = &self.output_config {
            config.as_ref()
        } else {
            local = self.read_effective_config(false)?;
            &local
        };
        if let Some(path) = &config.path {
            command.arg("--config-location").arg(path);
        } else {
            command.arg("--ignore-config");
        }

        if let Some(proxy_url) = self.effective_proxy_url() {
            command.arg("--proxy").arg(proxy_url);
        }

        if self.no_check_certificates {
            command.arg("--no-check-certificates");
        }

        if !self.impersonate.is_empty() {
            command.arg("--impersonate").arg(&self.impersonate);
        }

        let ffmpeg_path = resolve_tool_path(&self.ffmpeg);
        if ffmpeg_path.is_file() {
            command.arg("--ffmpeg-location").arg(ffmpeg_path);
        }

        let deno_path = resolve_tool_path(&self.deno);
        if deno_path.is_file() {
            command
                .arg("--js-runtimes")
                .arg(format!("deno:{}", deno_path.display()));
        }

        if config.owns(&["--no-cache-dir"]) {
            command.arg("--no-cache-dir");
        } else {
            command
                .arg("--cache-dir")
                .arg(super::storage_paths::cache_directory("yt-dlp")?);
        }
        command.arg("-P").arg(format!(
            "temp:{}",
            super::storage_paths::cache_directory("yt-dlp-temp")?.display()
        ));
        Ok(())
    }

    fn explicit_config_path(&self) -> Option<PathBuf> {
        let trimmed = self.yt_dlp_config.trim();
        if trimmed.is_empty() {
            return None;
        }

        let path = resolve_support_path(trimmed);
        path.is_file().then_some(path)
    }

    fn portable_config_path(&self) -> Option<PathBuf> {
        let tool_path = resolve_tool_path(&self.yt_dlp);
        let parent = tool_path.parent()?;
        let config_path = parent.join("yt-dlp.conf");
        config_path.is_file().then_some(config_path)
    }
}

impl ToolPaths {
    fn run_analyze_command(
        &self,
        tool_path: &Path,
        url: &str,
        use_cookies: bool,
        format_selector: Option<&str>,
        playlist_item_limit: Option<usize>,
        playlist_index: Option<usize>,
    ) -> Result<AnalyzeOutput, AnalyzeError> {
        self.run_analyze_command_cancellable(
            tool_path,
            url,
            use_cookies,
            format_selector,
            playlist_item_limit,
            playlist_index,
            &AtomicBool::new(false),
        )
    }

    pub(crate) fn run_analyze_command_cancellable(
        &self,
        tool_path: &Path,
        url: &str,
        use_cookies: bool,
        format_selector: Option<&str>,
        playlist_item_limit: Option<usize>,
        playlist_index: Option<usize>,
        cancel: &AtomicBool,
    ) -> Result<AnalyzeOutput, AnalyzeError> {
        let (tools, mut command) = self
            .prepare_metadata_command(tool_path)
            .map_err(|error| AnalyzeError::new(error, None))?;
        command.arg("--dump-single-json").arg("--no-warnings");

        apply_analysis_selection_args(&mut command, playlist_item_limit, format_selector);
        apply_download_playlist_index_arg(&mut command, playlist_index);

        tools
            .apply_cookie_args(&mut command, use_cookies, Some(url))
            .map_err(|error| AnalyzeError::new(error, None))?;
        tools.apply_youtube_extractor_args(&mut command);
        command.arg(url);

        let command_line = format_command_line(tool_path, &command);
        eprintln!("[analyze] command: {command_line}");

        let output = super::process_guard::run_tracked_command_output_cancellable(
            &mut command,
            "yt-dlp analyze",
            cancel,
        )
        .map_err(|error| {
            AnalyzeError::new(
                format!("Could not start yt-dlp: {error}"),
                Some(command_line.clone()),
            )
        })?;

        // Some extractors return a usable playlist JSON together with a
        // non-zero status when individual entries fail (Instagram carousels
        // are one example). Keep the successful entries for the queue path.
        let partial_playlist = serde_json::from_slice::<Value>(&output.stdout)
            .ok()
            .filter(yt_dlp_has_usable_playlist_entries);
        if !output.status.success() && partial_playlist.is_none() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            let detail = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                format!("exit code {:?}", output.status.code())
            };
            let detail = humanize_yt_dlp_error(&detail);
            return Err(AnalyzeError::new(
                format!("yt-dlp analysis failed: {detail}"),
                Some(command_line),
            ));
        }

        let json = partial_playlist
            .map_or_else(|| serde_json::from_slice::<Value>(&output.stdout), Ok)
            .map_err(|error| {
                AnalyzeError::new(
                    format!("Could not parse yt-dlp JSON: {error}"),
                    Some(command_line.clone()),
                )
            })?;
        Ok(AnalyzeOutput { json, command_line })
    }
}

fn yt_dlp_has_usable_playlist_entries(json: &Value) -> bool {
    json.get("_type").and_then(Value::as_str) == Some("playlist")
        && json
            .get("entries")
            .and_then(Value::as_array)
            .is_some_and(|entries| entries.iter().any(Value::is_object))
}

pub fn humanize_yt_dlp_error(detail: &str) -> String {
    if is_chromium_cookie_database_copy_error(detail) {
        return format!(
            r#"Could not read the Chromium/Chrome cookie database directly. This is usually because the browser locked the Network\Cookies database, not because the checkbox state is wrong. Close the browser and retry, or change Cookie source to Use file (cookies.txt) in Advanced. Original message: {detail}"#
        );
    }
    if is_youtube_subtitle_429_error(detail) {
        return format!(
            r#"YouTube rejected the subtitle request (HTTP 429 Too Many Requests). This often happens on the YouTube auto-translation timedtext endpoint. cookies.txt can provide login state, but may not satisfy PO Token / rate-limit requirements for that endpoint. The GUI keeps the native yt-dlp flow and diagnostic logs instead of switching to a custom downloader. Original message: {detail}"#
        );
    }
    detail.to_owned()
}

fn is_youtube_subtitle_429_error(detail: &str) -> bool {
    let normalized = detail.to_ascii_lowercase();
    normalized.contains("unable to download video subtitles")
        && normalized.contains("http error 429")
}

fn is_chromium_cookie_database_copy_error(detail: &str) -> bool {
    let normalized = detail.to_ascii_lowercase();
    normalized.contains("could not copy chrome cookie database")
        || (normalized.contains("could not copy") && normalized.contains("cookie database"))
}

pub fn configure_background_command(command: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

fn selector_uses_separate_streams(selector: &str) -> bool {
    selector.split('/').any(|part| part.contains('+'))
}

fn aria2_downloader_args(proxy_url: Option<&str>, no_check_certificates: bool) -> Option<String> {
    // Piped downloads still need periodic readouts for the shared GUI parser.
    let mut args = vec![
        "--summary-interval=1".to_owned(),
        "--show-console-readout=true".to_owned(),
        "--enable-color=false".to_owned(),
    ];
    if let Some(proxy_url) = proxy_url.map(str::trim).filter(|value| !value.is_empty()) {
        args.push(format!("--all-proxy={proxy_url}"));
    }
    if no_check_certificates {
        args.push("--check-certificate=false".to_owned());
    }
    (!args.is_empty()).then(|| format!("aria2c:{}", args.join(" ")))
}

fn apply_download_section_args(
    command: &mut Command,
    target_kind: DownloadTargetKind,
    download_section_args: &[String],
    config_owns_download_sections: bool,
) -> bool {
    let has_item_download_section =
        target_kind != DownloadTargetKind::Subtitle && !download_section_args.is_empty();
    if has_item_download_section && !config_owns_download_sections {
        for section in download_section_args {
            command.arg("--download-sections").arg(section.trim());
        }
    }
    has_item_download_section
}

fn apply_live_from_start_arg(
    command: &mut Command,
    target_kind: DownloadTargetKind,
    enabled: bool,
    config_owns_setting: bool,
) {
    if target_kind != DownloadTargetKind::Subtitle && enabled && !config_owns_setting {
        command.arg("--live-from-start");
    }
}

fn apply_merge_output_container_arg(
    command: &mut Command,
    target_kind: DownloadTargetKind,
    container: Option<&str>,
) {
    if target_kind != DownloadTargetKind::Normal {
        return;
    }
    if let Some(container) = container.and_then(normalized_download_merge_container) {
        command.arg("--merge-output-format").arg(container);
    }
}

fn apply_subtitle_download_args(
    command: &mut Command,
    request: &DownloadRequest,
    should_write_subtitles: bool,
) {
    if !should_write_subtitles {
        return;
    }
    let Some(subtitle_lang) = request.subtitle_lang.as_deref() else {
        return;
    };

    command.arg("--sub-langs").arg(subtitle_lang);
    if request.write_auto_subs {
        command.arg("--write-auto-subs");
        if request.subtitle_is_auto_translated {
            command.arg("--sleep-subtitles").arg("1");
        }
    } else if request.target_kind == DownloadTargetKind::Subtitle || !request.embed_subtitles {
        // yt-dlp keeps the sidecar when --write-subs and --embed-subs are
        // explicitly combined. Embed mode intentionally relies on --embed-subs
        // to request normal subtitles and remove its temporary sidecar.
        command.arg("--write-subs");
    }

    let subtitle_ext =
        normalized_extension(&request.subtitle_ext).unwrap_or_else(|| "srt".to_owned());
    if request.target_kind == DownloadTargetKind::Subtitle {
        command
            .arg("--sub-format")
            .arg(subtitle_format_preference(&subtitle_ext));
        if subtitle_conversion_is_supported(&subtitle_ext) {
            command.arg("--convert-subs").arg(&subtitle_ext);
        }
    } else if request.embed_subtitles {
        command.arg("--embed-subs");
    } else {
        command.arg("--convert-subs").arg("srt");
    }
}

fn auto_cookie_file_for_url(url: &str) -> Result<Option<PathBuf>, String> {
    let Some(host) = auto_cookie_host_from_url(url) else {
        return Ok(None);
    };

    let cookie_dir = portable_root_dir().join("data").join("cookies");
    let Some(index) = read_cookie_site_index(&cookie_dir)? else {
        return Ok(None);
    };

    for site in index.sites {
        let cookie_file = site.cookie_file.trim();
        if cookie_file.is_empty() {
            continue;
        }

        let matches = site
            .match_domains
            .iter()
            .map(String::as_str)
            .any(|domain| auto_cookie_domain_matches(&host, domain));
        if !matches {
            continue;
        }

        let candidate = PathBuf::from(cookie_file);
        let path = if candidate.is_absolute() {
            candidate
        } else {
            cookie_dir.join(candidate)
        };
        if path.is_file() {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

fn auto_cookie_host_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next()?.trim();
    if authority.is_empty() {
        return None;
    }
    let host = authority
        .rsplit('@')
        .next()
        .unwrap_or(authority)
        .split(':')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.is_empty() { None } else { Some(host) }
}

fn auto_cookie_domain_matches(host: &str, domain: &str) -> bool {
    let domain = domain
        .trim()
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if domain.is_empty() {
        return false;
    }
    host == domain || host.ends_with(&format!(".{domain}"))
}

fn cookie_source_argument(source: &str, profile: &str) -> String {
    let source = source.trim();
    if source.is_empty() {
        return String::new();
    }
    let profile = profile.trim();
    if profile.is_empty() {
        source.to_owned()
    } else {
        format!("{source}:{profile}")
    }
}

fn browser_cookie_source_candidates() -> [BrowserCookieSourceOption; 7] {
    [
        BrowserCookieSourceOption {
            value: "brave",
            label: "Brave",
        },
        BrowserCookieSourceOption {
            value: "chrome",
            label: "Chrome",
        },
        BrowserCookieSourceOption {
            value: "chromium",
            label: "Chromium",
        },
        BrowserCookieSourceOption {
            value: "edge",
            label: "Edge",
        },
        BrowserCookieSourceOption {
            value: "firefox",
            label: "Firefox",
        },
        BrowserCookieSourceOption {
            value: "opera",
            label: "Opera",
        },
        BrowserCookieSourceOption {
            value: "vivaldi",
            label: "Vivaldi",
        },
    ]
}

fn browser_cookie_source_exists(browser: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        let roaming = std::env::var_os("APPDATA").map(PathBuf::from);
        match browser {
            "brave" => local
                .map(|base| base.join(r"BraveSoftware\Brave-Browser\User Data"))
                .is_some_and(|path| path.exists()),
            "chrome" => local
                .map(|base| base.join(r"Google\Chrome\User Data"))
                .is_some_and(|path| path.exists()),
            "chromium" => local
                .map(|base| base.join(r"Chromium\User Data"))
                .is_some_and(|path| path.exists()),
            "edge" => local
                .map(|base| base.join(r"Microsoft\Edge\User Data"))
                .is_some_and(|path| path.exists()),
            "firefox" => roaming
                .map(|base| base.join(r"Mozilla\Firefox\Profiles"))
                .is_some_and(|path| path.exists()),
            "opera" => roaming
                .map(|base| base.join(r"Opera Software\Opera Stable"))
                .is_some_and(|path| path.exists()),
            "vivaldi" => local
                .map(|base| base.join(r"Vivaldi\User Data"))
                .is_some_and(|path| path.exists()),
            _ => false,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = browser;
        false
    }
}

fn browser_cookie_profiles(browser: &str) -> Vec<BrowserCookieProfileOption> {
    #[cfg(target_os = "windows")]
    {
        let Some(base_dir) = browser_cookie_base_dir(browser) else {
            return Vec::new();
        };
        let mut items = Vec::new();
        let chromium_profile_names = chromium_profile_display_names(&base_dir);

        let single_profile = matches!(browser, "opera");
        if single_profile {
            if base_dir.exists() {
                items.push(BrowserCookieProfileOption {
                    value: "Default".to_owned(),
                    label: chromium_profile_names
                        .get("Default")
                        .cloned()
                        .unwrap_or_else(|| "Default".to_owned()),
                });
            }
            return items;
        }

        let Ok(entries) = fs::read_dir(base_dir) else {
            return items;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if name == "Default" || name == "Guest Profile" || name.starts_with("Profile ") {
                let label = chromium_profile_names
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| name.to_owned());
                items.push(BrowserCookieProfileOption {
                    value: name.to_owned(),
                    label,
                });
            }
        }

        if browser == "firefox" {
            items.clear();
            let Ok(entries) = fs::read_dir(browser_cookie_base_dir(browser).unwrap()) else {
                return items;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                    continue;
                };
                items.push(BrowserCookieProfileOption {
                    value: name.to_owned(),
                    label: name.to_owned(),
                });
            }
        }

        items.sort_by(|left, right| left.label.cmp(&right.label));
        items
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = browser;
        Vec::new()
    }
}

#[cfg(target_os = "windows")]
fn browser_cookie_base_dir(browser: &str) -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let roaming = std::env::var_os("APPDATA").map(PathBuf::from);
    match browser {
        "brave" => local.map(|base| base.join(r"BraveSoftware\Brave-Browser\User Data")),
        "chrome" => local.map(|base| base.join(r"Google\Chrome\User Data")),
        "chromium" => local.map(|base| base.join(r"Chromium\User Data")),
        "edge" => local.map(|base| base.join(r"Microsoft\Edge\User Data")),
        "firefox" => roaming.map(|base| base.join(r"Mozilla\Firefox\Profiles")),
        "opera" => roaming.map(|base| base.join(r"Opera Software\Opera Stable")),
        "vivaldi" => local.map(|base| base.join(r"Vivaldi\User Data")),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn chromium_profile_display_names(base_dir: &Path) -> std::collections::HashMap<String, String> {
    let mut names = std::collections::HashMap::new();
    let local_state_path = base_dir.join("Local State");
    let Ok(content) = fs::read_to_string(local_state_path) else {
        return names;
    };
    let Ok(json) = serde_json::from_str::<Value>(&content) else {
        return names;
    };
    let Some(info_cache) = json
        .get("profile")
        .and_then(|profile| profile.get("info_cache"))
        .and_then(Value::as_object)
    else {
        return names;
    };

    for (folder, info) in info_cache {
        let display_name = info
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                info.get("shortcut_name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            });
        let user_name = info
            .get("user_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        let Some(display_name) = display_name else {
            continue;
        };
        let label = match user_name {
            Some(user_name) if !user_name.eq_ignore_ascii_case(&display_name) => {
                format!("{display_name} ({user_name})")
            }
            _ => display_name,
        };
        names.insert(folder.to_owned(), label);
    }

    names
}

pub fn youtube_url_has_video_and_playlist(url: &str) -> bool {
    looks_like_youtube_url(url) && youtube_video_id(url).is_some() && youtube_list_id(url).is_some()
}

pub fn youtube_url_force_single_video(url: &str) -> Option<String> {
    let video_id = youtube_video_id(url)?;
    Some(format!("https://www.youtube.com/watch?v={video_id}"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YoutubePlaylistKind {
    ChannelPopularOrGenerated,
    YoutubeMixRadio,
    YoutubeMusicAlbum,
    LikedVideos,
    FavoritesLegacy,
}

impl YoutubePlaylistKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::ChannelPopularOrGenerated => "YouTube generated channel playlist",
            Self::YoutubeMixRadio => "YouTube Mix / Radio",
            Self::YoutubeMusicAlbum => "YouTube Music album/collection",
            Self::LikedVideos => "Liked videos",
            Self::FavoritesLegacy => "Legacy favorites playlist",
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::ChannelPopularOrGenerated => "options.playlist_risk.kind.channel_generated",
            Self::YoutubeMixRadio => "options.playlist_risk.kind.youtube_mix_radio",
            Self::YoutubeMusicAlbum => "options.playlist_risk.kind.youtube_music_album",
            Self::LikedVideos => "options.playlist_risk.kind.liked_videos",
            Self::FavoritesLegacy => "options.playlist_risk.kind.favorites_legacy",
        }
    }

    pub fn note_key(self) -> &'static str {
        match self {
            Self::ChannelPopularOrGenerated => "options.playlist_risk.note.channel_generated",
            Self::YoutubeMixRadio => "options.playlist_risk.note.youtube_mix_radio",
            Self::YoutubeMusicAlbum => "options.playlist_risk.note.youtube_music_album",
            Self::LikedVideos => "options.playlist_risk.note.liked_videos",
            Self::FavoritesLegacy => "options.playlist_risk.note.favorites_legacy",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YoutubePlaylistRisk {
    pub kind: YoutubePlaylistKind,
    pub note: &'static str,
}

pub fn classify_youtube_playlist(url: &str) -> Option<YoutubePlaylistRisk> {
    if !looks_like_youtube_url(url) {
        return None;
    }

    let list_id = youtube_list_id(url)?;
    let uppercase = list_id.to_ascii_uppercase();

    let risk = if uppercase.starts_with("RDCLAK5UY") || uppercase.starts_with("RDCMUC") {
        YoutubePlaylistRisk {
            kind: YoutubePlaylistKind::YoutubeMixRadio,
            note: "This Mix / Radio playlist may contain many items and can change over time.",
        }
    } else if uppercase.starts_with("UULP") || uppercase.starts_with("UUSH") {
        YoutubePlaylistRisk {
            kind: YoutubePlaylistKind::ChannelPopularOrGenerated,
            note: "Treat this YouTube-generated channel playlist conservatively.",
        }
    } else if uppercase.starts_with("RD") {
        YoutubePlaylistRisk {
            kind: YoutubePlaylistKind::YoutubeMixRadio,
            note: "This Mix / Radio playlist may contain many items and can change over time.",
        }
    } else if uppercase == "LL" {
        YoutubePlaylistRisk {
            kind: YoutubePlaylistKind::LikedVideos,
            note: "Liked videos usually require login or cookies.",
        }
    } else if uppercase.starts_with("FL") {
        YoutubePlaylistRisk {
            kind: YoutubePlaylistKind::FavoritesLegacy,
            note: "This is a legacy favorites playlist style and may not be stable now.",
        }
    } else if uppercase.starts_with("OLAK5UY") {
        YoutubePlaylistRisk {
            kind: YoutubePlaylistKind::YoutubeMusicAlbum,
            note: "This is usually a YouTube Music album or collection.",
        }
    } else {
        return None;
    };

    Some(risk)
}

fn looks_like_youtube_url(url: &str) -> bool {
    let normalized = url.to_ascii_lowercase();
    normalized.contains("youtube.com") || normalized.contains("youtu.be")
}

fn youtube_list_id(url: &str) -> Option<&str> {
    let (_, tail) = url.split_once("list=")?;
    tail.split(['&', '#', '?'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn youtube_video_id(url: &str) -> Option<&str> {
    let normalized = url.to_ascii_lowercase();
    if normalized.contains("youtu.be/") {
        let (_, tail) = url.split_once("youtu.be/")?;
        return tail
            .split(['&', '#', '?', '/'])
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty());
    }

    let (_, tail) = url.split_once("v=")?;
    tail.split(['&', '#', '?', '/'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn playlist_entry_url(entry: &Value) -> Option<String> {
    entry
        .get("webpage_url")
        .or_else(|| entry.get("original_url"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            let raw_url = entry.get("url").and_then(Value::as_str)?;
            if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
                Some(raw_url.to_owned())
            } else {
                let id = entry.get("id").and_then(Value::as_str).unwrap_or(raw_url);
                Some(format!("https://www.youtube.com/watch?v={id}"))
            }
        })
}

fn build_output_plan(request: &DownloadRequest) -> Result<OutputPlan, String> {
    if let Some(output_path) = request
        .output_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        super::storage_paths::validate_output_destination(Path::new(output_path))?;
        return build_explicit_output_plan(request, output_path);
    }

    let output_dir = resolve_output_dir(&request.output_dir)?;
    super::storage_paths::validate_output_destination(&output_dir)?;
    let plan = build_directory_output_plan(request, &output_dir);
    super::storage_paths::validate_output_destination(&plan.final_output_path)?;
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("Could not create download folder: {error}"))?;
    Ok(plan)
}

fn build_directory_output_plan(request: &DownloadRequest, output_dir: &Path) -> OutputPlan {
    let expected_extension = determine_expected_container(request);
    let (base_name, legacy_forced_extension) = if request.target_kind == DownloadTargetKind::Normal
    {
        split_normal_download_file_name_parts(request.file_name.trim())
    } else {
        split_file_name_parts(request.file_name.trim())
    };
    let base_name = if base_name.is_empty() {
        "%(title)s".to_owned()
    } else {
        base_name
    };

    let has_explicit_merge_container = request
        .merge_output_container
        .as_deref()
        .and_then(normalized_download_merge_container)
        .is_some();
    let explicit_output_extension = if request.target_kind == DownloadTargetKind::Normal {
        request
            .output_extension_override
            .as_deref()
            .and_then(normalized_normal_output_extension)
    } else {
        legacy_forced_extension.clone()
    };
    let final_extension = explicit_output_extension
        .clone()
        .unwrap_or_else(|| expected_extension.clone());
    let final_output_path = output_dir.join(format!("{base_name}.{final_extension}"));
    let output_base_name =
        output_base_name_with_section_number(&base_name, request.download_section_args.len());

    if !has_explicit_merge_container
        && request.target_kind != DownloadTargetKind::Subtitle
        && explicit_output_extension
            .as_deref()
            .is_some_and(|ext| !ext.eq_ignore_ascii_case(&expected_extension))
    {
        OutputPlan {
            output_argument: output_dir
                .join(format!("{output_base_name}.%(ext)s"))
                .display()
                .to_string(),
            final_output_path,
            remux_extension: Some(final_extension),
            extract_audio: false,
            audio_format: None,
        }
    } else {
        OutputPlan {
            output_argument: output_argument_for_kind(
                request.target_kind,
                &output_dir.join(format!("{output_base_name}.{final_extension}")),
            ),
            final_output_path,
            remux_extension: None,
            extract_audio: false,
            audio_format: None,
        }
    }
}

fn normalized_youtube_subs_po_token(value: &str) -> Option<String> {
    let mut token = value.trim();
    if token.is_empty() {
        return None;
    }
    if let Some(rest) = token.strip_prefix("youtube:") {
        token = rest.trim();
    }
    if let Some(rest) = token.strip_prefix("po_token=") {
        token = rest.trim();
    }
    if token.is_empty() {
        return None;
    }

    if token.contains('+') {
        Some(token.to_owned())
    } else {
        Some(format!("web.subs+{token}"))
    }
}

fn normalized_youtube_extractor_args(value: &str) -> Option<String> {
    let mut args = value.trim();
    if args.is_empty() {
        return None;
    }
    if let Some(rest) = args.strip_prefix("--extractor-args") {
        args = rest.trim().trim_matches('"').trim_matches('\'');
    }
    if let Some(rest) = args.strip_prefix("youtube:") {
        args = rest.trim();
    }
    if args.is_empty() {
        None
    } else {
        Some(args.to_owned())
    }
}

fn subtitle_format_preference(extension: &str) -> String {
    if subtitle_conversion_is_supported(extension) {
        format!("{extension}/best")
    } else {
        extension.to_owned()
    }
}

fn subtitle_conversion_is_supported(extension: &str) -> bool {
    matches!(extension, "ass" | "lrc" | "srt" | "vtt")
}

fn determine_expected_container(request: &DownloadRequest) -> String {
    if request.target_kind == DownloadTargetKind::Normal {
        if let Some(container) = request
            .merge_output_container
            .as_deref()
            .and_then(normalized_download_merge_container)
        {
            return container;
        }
    }

    match request.target_kind {
        DownloadTargetKind::Audio => {
            return normalized_extension(&request.audio_ext).unwrap_or_else(|| "m4a".to_owned());
        }
        DownloadTargetKind::Video => {
            return normalized_extension(&request.video_ext).unwrap_or_else(|| "mkv".to_owned());
        }
        DownloadTargetKind::Subtitle => {
            return normalized_extension(&request.subtitle_ext).unwrap_or_else(|| "srt".to_owned());
        }
        DownloadTargetKind::Normal => {}
    }

    crate::domain::default_download_container_extension(
        None,
        request.is_muxed_video,
        &request.video_ext,
        &request.audio_ext,
    )
}

fn build_explicit_output_plan(
    request: &DownloadRequest,
    output_path: &str,
) -> Result<OutputPlan, String> {
    let requested_path = PathBuf::from(output_path);
    let parent = requested_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent)
        .map_err(|error| format!("Could not create download folder: {error}"))?;

    let expected_extension = determine_expected_container(request);
    let (base_name, forced_extension) = split_file_name_parts(
        requested_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default(),
    );
    let final_extension = forced_extension
        .clone()
        .unwrap_or_else(|| expected_extension.clone());
    let final_output_path = parent.join(format!("{base_name}.{final_extension}"));
    let output_base_name =
        output_base_name_with_section_number(&base_name, request.download_section_args.len());
    let output_path = parent.join(format!("{output_base_name}.{final_extension}"));

    match request.target_kind {
        DownloadTargetKind::Audio => Ok(OutputPlan {
            output_argument: parent.join(output_base_name).display().to_string(),
            final_output_path,
            remux_extension: None,
            extract_audio: true,
            audio_format: Some(final_extension),
        }),
        DownloadTargetKind::Video | DownloadTargetKind::Subtitle | DownloadTargetKind::Normal => {
            let remux_extension = forced_extension
                .as_deref()
                .filter(|_| request.target_kind != DownloadTargetKind::Subtitle)
                .filter(|ext| !ext.eq_ignore_ascii_case(&expected_extension))
                .map(ToOwned::to_owned);
            Ok(OutputPlan {
                output_argument: output_argument_for_kind(request.target_kind, &output_path),
                final_output_path,
                remux_extension,
                extract_audio: false,
                audio_format: None,
            })
        }
    }
}

fn output_base_name_with_section_number(base_name: &str, section_count: usize) -> String {
    if section_count > 1 {
        format!("{base_name} - %(autonumber)02d")
    } else {
        base_name.to_owned()
    }
}

fn apply_music_chapter_split_args(command: &mut Command, output_dir: Option<&Path>) {
    if let Some(output_dir) = output_dir {
        command
            .arg("--split-chapters")
            .arg("-P")
            .arg(format!("chapter:{}", output_dir.display()))
            .arg("--output")
            .arg("chapter:%(section_number)02d - %(section_title)s.%(ext)s");
    }
}

/// Keep final destinations while allowing yt-dlp's temporary path to take effect.
pub(crate) fn apply_output_path(command: &mut Command, output: &str) -> Result<(), String> {
    let (kind, output) = ["subtitle:", "chapter:", "thumbnail:"]
        .iter()
        .find_map(|prefix| {
            output
                .strip_prefix(prefix)
                .map(|rest| (prefix.trim_end_matches(':'), rest))
        })
        .unwrap_or(("home", output));
    let path = Path::new(output);
    let name = path.file_name().ok_or("Output template has no filename")?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        command
            .arg("-P")
            .arg(format!("{kind}:{}", parent.display()));
    }
    let mut template = std::ffi::OsString::new();
    if kind != "home" {
        template.push(format!("{kind}:"));
    }
    template.push(name);
    command.arg("--output").arg(template);
    Ok(())
}

pub(crate) fn validate_external_output_config(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Could not inspect yt-dlp output configuration: {e}"))?;
    validate_external_output_arguments(&config_options::interpret(&config_arguments(&content)?)?)
}
fn validate_external_output_arguments(options: &[ConfigOption]) -> Result<(), String> {
    output_preparation::validate_protected_options(options)?;
    for option in options {
        let paths = (option.name == "--paths").then(|| option.values[0].as_str());
        if let Some(paths) = paths {
            let (kind, directory) = paths
                .split_once(':')
                .filter(|(kind, _)| {
                    kind.len() > 1 && kind.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                })
                .unwrap_or(("home", paths));
            if !matches!(kind, "home" | "temp") {
                return Err(format!("Unsupported protected output path: {paths}"));
            }
            if kind != "temp" {
                if directory.starts_with('~') || contains_environment_reference(directory) {
                    return Err("Environment-dependent yt-dlp output directories cannot be checked for cache ownership.".to_owned());
                }
                // Pure config inspection is also used before startup in tests;
                // real command preparation always requires initialized storage.
                if super::storage_paths::require_ready().is_ok() {
                    super::storage_paths::validate_output_destination(&resolve_output_dir(
                        directory,
                    )?)?;
                }
            }
        }
        let output = (option.name == "--output").then(|| option.values[0].as_str());
        if let Some(output) = output {
            let output = output
                .split_once(':')
                .filter(|(kind, _)| {
                    kind.len() > 1 && kind.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                })
                .map(|(_, rest)| rest)
                .unwrap_or(output);
            let drive = output.as_bytes().get(1) == Some(&b':');
            let variable = contains_environment_reference(output);
            if Path::new(output).has_root()
                || output.starts_with('~')
                || output.starts_with(['/', '\\'])
                || drive
                || variable
                || Path::new(output)
                    .components()
                    .any(|part| part == std::path::Component::ParentDir)
            {
                return Err("Absolute or environment-dependent yt-dlp output templates bypass cache temporary paths. Use a relative template or the application's output selection.".to_owned());
            }
        }
    }
    Ok(())
}

fn contains_environment_reference(value: &str) -> bool {
    if value.contains('$') {
        return true;
    }
    let mut rest = value;
    while let Some((_, tail)) = rest.split_once('%') {
        let Some((name, tail)) = tail.split_once('%') else {
            break;
        };
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return true;
        }
        rest = tail;
    }
    false
}

/// The supported config subset uses shell-style quoting, escaping and comments.
fn config_arguments(content: &str) -> Result<Vec<String>, String> {
    let content = content
        .strip_prefix('\u{feff}')
        .unwrap_or(content)
        .replace("\r\n", "\n");
    let mut chars = content.chars().peekable();
    let mut quote = None;
    let mut token = String::new();
    let mut started = false;
    let mut result = Vec::new();
    while let Some(c) = chars.next() {
        if c == '\\' && quote != Some('\'') {
            started = true;
            if quote == Some('"') && !matches!(chars.peek(), Some('"' | '\\')) {
                token.push(c);
                continue;
            }
            let next = chars
                .next()
                .ok_or("Incomplete escape in yt-dlp configuration")?;
            if next != '\n' {
                token.push(next);
            }
        } else if quote == Some(c) {
            quote = None;
        } else if quote.is_none() && matches!(c, '\'' | '"') {
            quote = Some(c);
            started = true;
        } else if quote.is_none() && c == '#' {
            for next in chars.by_ref() {
                if next == '\n' {
                    break;
                }
            }
            if started {
                result.push(std::mem::take(&mut token));
                started = false;
            }
        } else if quote.is_none() && c.is_whitespace() {
            if started {
                result.push(std::mem::take(&mut token));
                started = false;
            }
        } else {
            token.push(c);
            started = true;
        }
    }
    if quote.is_some() {
        return Err("Unclosed quote in yt-dlp configuration".to_owned());
    }
    if started {
        result.push(token);
    }
    Ok(result)
}

#[cfg(test)]
mod config_semantics_tests {
    use super::*;

    #[test]
    fn consumed_option_values_do_not_own_output_or_trigger_protected_hooks() {
        let options = config_options::interpret(&config_arguments(
            "--match-title '-Pfoo' --user-agent '--exec' --format app --output '%(title)s.%(ext)s'",
        ).unwrap()).unwrap();
        validate_external_output_arguments(&options).unwrap();
        assert_eq!(
            output_preparation::external_output(&options).unwrap(),
            (Some("%(title)s.%(ext)s".into()), None)
        );
        let config = EffectiveYtDlpConfig {
            options,
            ..Default::default()
        };
        assert!(config.owns(&["--format", "--output"]));
        assert!(!config.owns(&["--paths", "--exec"]));
        let forbidden =
            config_options::interpret(&config_arguments("--exec actual-hook").unwrap()).unwrap();
        assert!(
            validate_external_output_arguments(&forbidden)
                .unwrap_err()
                .contains("--exec")
        );
    }

    #[test]
    fn quoted_empty_values_attached_options_aliases_and_short_clusters_keep_boundaries() {
        let options = config_options::interpret(
            &config_arguments("--match-t '' -qfapp --rate-limit=10K -Pfoo").unwrap(),
        )
        .unwrap();
        assert_eq!(options[0].name, "--match-title");
        assert_eq!(options[0].values, [""]);
        assert_eq!(options[1].name, "--quiet");
        assert_eq!(options[2].name, "--format");
        assert_eq!(options[2].values, ["app"]);
        assert_eq!(options[3].name, "--limit-rate");
        assert_eq!(options[3].values, ["10K"]);
        assert_eq!(options[4].name, "--paths");
        assert_eq!(options[4].values, ["foo"]);
        assert_eq!(options.len(), 5);
    }

    #[test]
    fn bom_hooks_and_terminators_cannot_hide_config_options() {
        for prefix in ["", "\u{feff}"] {
            let options = config_options::interpret(
                &config_arguments(&format!("{prefix}--exec harmless-hook")).unwrap(),
            )
            .unwrap();
            assert!(
                validate_external_output_arguments(&options)
                    .unwrap_err()
                    .contains("--exec")
            );
        }
        let options = config_options::interpret(
            &config_arguments("\u{feff}--user-agent 'a\u{feff}b' --match-title --").unwrap(),
        )
        .unwrap();
        validate_external_output_arguments(&options).unwrap();
        assert_eq!(options[0].values, ["a\u{feff}b"]);
        assert_eq!(options[1].values, ["--"]);
        assert!(
            config_options::interpret(
                &config_arguments("--ignore-errors -- --exec hidden").unwrap()
            )
            .unwrap_err()
            .contains("terminator: --")
        );
    }

    #[test]
    fn output_presets_are_rejected_through_supported_spellings() {
        for spelling in [
            "-tmp3",
            "--preset-alias=mp3",
            "--preset-a mp4",
            "-t aac",
            "-t mkv",
            "-t unknown",
        ] {
            let options = config_options::interpret(&config_arguments(spelling).unwrap()).unwrap();
            assert!(
                validate_external_output_arguments(&options)
                    .unwrap_err()
                    .contains("Unsupported preset"),
                "{spelling}"
            );
        }
        let options = config_options::interpret(
            &config_arguments("-t sleep --match-title '-t' -f best").unwrap(),
        )
        .unwrap();
        validate_external_output_arguments(&options).unwrap();
        assert_eq!(options[1].values, ["-t"]);
    }
}

fn output_argument_for_kind(kind: DownloadTargetKind, path: &Path) -> String {
    match kind {
        DownloadTargetKind::Subtitle => format!("subtitle:{}", path.display()),
        DownloadTargetKind::Normal | DownloadTargetKind::Video | DownloadTargetKind::Audio => {
            path.display().to_string()
        }
    }
}

fn split_file_name_parts(file_name: &str) -> (String, Option<String>) {
    if file_name.is_empty() {
        return (String::new(), None);
    }

    let path = Path::new(file_name);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .and_then(normalized_extension)
        .filter(|extension| is_known_output_extension(extension));

    if let Some(extension) = extension {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(file_name)
            .to_owned();
        (stem, Some(extension))
    } else {
        (file_name.to_owned(), None)
    }
}

fn split_normal_download_file_name_parts(file_name: &str) -> (String, Option<String>) {
    if file_name.is_empty() {
        return (String::new(), None);
    }

    let path = Path::new(file_name);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .and_then(normalized_normal_output_extension);
    if let Some(extension) = extension {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(file_name)
            .to_owned();
        (stem, Some(extension))
    } else {
        (file_name.to_owned(), None)
    }
}

fn is_known_output_extension(extension: &str) -> bool {
    matches!(
        extension,
        "mp4"
            | "mkv"
            | "webm"
            | "mov"
            | "flv"
            | "avi"
            | "mp3"
            | "m4a"
            | "flac"
            | "wav"
            | "opus"
            | "aac"
            | "vorbis"
            | "alac"
            | "srt"
            | "vtt"
            | "ass"
            | "ssa"
            | "lrc"
            | "ttml"
            | "dfxp"
            | "json3"
            | "srv3"
            | "srv2"
            | "srv1"
    )
}

fn music_audio_download_format_selector(target_format: &str) -> &'static str {
    match target_format
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
        .as_str()
    {
        "mp3" => "bestaudio[ext=mp3]/bestaudio[acodec^=mp3]/bestaudio/best[acodec!=none]",
        "m4a" | "aac" => {
            "bestaudio[ext=m4a]/bestaudio[acodec^=mp4a]/bestaudio[acodec^=aac]/bestaudio/best[acodec!=none]"
        }
        "opus" => {
            "bestaudio[acodec^=opus]/bestaudio[ext=opus]/bestaudio[ext=webm][acodec^=opus]/bestaudio/best[acodec!=none]"
        }
        "flac" => "bestaudio[ext=flac]/bestaudio[acodec^=flac]/bestaudio/best[acodec!=none]",
        "wav" => "bestaudio[ext=wav]/bestaudio[acodec^=pcm]/bestaudio/best[acodec!=none]",
        _ => MUSIC_STREAM_FORMAT_SELECTOR,
    }
}

fn music_audio_download_format_supports_thumbnail(format: &str) -> bool {
    matches!(
        format.trim().to_ascii_lowercase().as_str(),
        "mp3" | "m4a" | "flac"
    )
}

fn music_album_parse_metadata_args() -> [&'static str; 2] {
    [
        "%(playlist|)s:%(meta_album)s",
        "%(playlist_title|)s:%(meta_album)s",
    ]
}

fn normalized_extension(extension: &str) -> Option<String> {
    let trimmed = extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn normalized_download_merge_container(container: &str) -> Option<String> {
    normalized_extension(container)
        .filter(|container| matches!(container.as_str(), "mkv" | "mp4" | "webm"))
}

fn normalized_normal_output_extension(extension: &str) -> Option<String> {
    normalized_extension(extension)
        .filter(|extension| matches!(extension.as_str(), "mkv" | "mp4" | "webm" | "mov" | "flv"))
}

pub fn resolve_tool_path(path: &str) -> PathBuf {
    resolve_support_path(path)
}

fn resolve_support_path(path: &str) -> PathBuf {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return portable_root_dir();
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        return candidate;
    }

    for base in candidate_base_dirs() {
        let joined = base.join(&candidate);
        if joined.exists() {
            return joined;
        }
    }

    portable_root_dir().join(candidate)
}

fn candidate_base_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![portable_root_dir()];
    if let Some(exe_dir) = current_executable_dir() {
        if !dirs.iter().any(|dir| dir == &exe_dir) {
            dirs.push(exe_dir);
        }
    }

    dirs
}

fn portable_root_dir() -> PathBuf {
    super::runtime_paths::application_root()
}

fn current_executable_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

pub(super) fn format_command_line(program: &Path, command: &Command) -> String {
    let args = command
        .get_args()
        .map(|arg| quote_arg(&arg.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ");
    if args.is_empty() {
        quote_arg(&program.display().to_string())
    } else {
        format!("{} {}", quote_arg(&program.display().to_string()), args)
    }
}

fn quote_arg(value: &str) -> String {
    if value.contains([' ', '\t', '"']) {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_owned()
    }
}

pub fn resolve_output_dir(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Download folder cannot be empty.".to_owned());
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(special) = resolve_windows_special_dir(trimmed) {
            return Ok(special);
        }
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        Ok(candidate)
    } else {
        Ok(resolve_support_path(trimmed))
    }
}

pub fn display_output_dir(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Some(portable_display) = display_portable_relative_dir(trimmed) {
        return portable_display;
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(label) = windows_known_folder_display_name(trimmed) {
            return label;
        }

        if let Some(label) = match_windows_special_dir_path(trimmed) {
            return label;
        }
    }

    let resolved = resolve_support_path(trimmed);
    if resolved == portable_root_dir() {
        return "yt-dlp-gui".to_owned();
    }

    trimmed.to_owned()
}

fn display_portable_relative_dir(path: &str) -> Option<String> {
    let normalized = path.trim().replace('/', "\\");
    if normalized == "." || normalized == ".\\" {
        return Some("yt-dlp-gui".to_owned());
    }

    let suffix = normalized
        .strip_prefix(".\\")
        .or_else(|| normalized.strip_prefix('.'))?
        .trim_start_matches('\\');
    if suffix.is_empty() {
        Some("yt-dlp-gui".to_owned())
    } else {
        Some(format!("yt-dlp-gui\\{suffix}"))
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct WindowsKnownFolder {
    canonical_name: &'static str,
    display_name: String,
    path: PathBuf,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[cfg(target_os = "windows")]
const FOLDERID_DESKTOP: Guid = Guid {
    data1: 0xB4BFCC3A,
    data2: 0xDB2C,
    data3: 0x424C,
    data4: [0xB0, 0x29, 0x7F, 0xE9, 0x9A, 0x87, 0xC6, 0x41],
};
#[cfg(target_os = "windows")]
const FOLDERID_DOCUMENTS: Guid = Guid {
    data1: 0xFDD39AD0,
    data2: 0x238F,
    data3: 0x46AF,
    data4: [0xAD, 0xB4, 0x6C, 0x85, 0x48, 0x03, 0x69, 0xC7],
};
#[cfg(target_os = "windows")]
const FOLDERID_DOWNLOADS: Guid = Guid {
    data1: 0x374DE290,
    data2: 0x123F,
    data3: 0x4565,
    data4: [0x91, 0x64, 0x39, 0xC4, 0x92, 0x5E, 0x46, 0x7B],
};
#[cfg(target_os = "windows")]
const FOLDERID_MUSIC: Guid = Guid {
    data1: 0x4BD8D571,
    data2: 0x6D19,
    data3: 0x48D3,
    data4: [0xBE, 0x97, 0x42, 0x22, 0x20, 0x08, 0x04, 0xE3],
};
#[cfg(target_os = "windows")]
const FOLDERID_VIDEOS: Guid = Guid {
    data1: 0x18989B1D,
    data2: 0x99B5,
    data3: 0x455B,
    data4: [0x84, 0x1C, 0xAB, 0x7C, 0x74, 0xE4, 0xDD, 0xFC],
};

#[cfg(target_os = "windows")]
#[link(name = "shell32")]
unsafe extern "system" {
    fn SHGetKnownFolderPath(
        rfid: *const Guid,
        dw_flags: u32,
        h_token: *mut c_void,
        ppsz_path: *mut *mut u16,
    ) -> i32;

    fn SHGetFileInfoW(
        psz_path: *const u16,
        dw_file_attributes: u32,
        psfi: *mut ShFileInfoW,
        cb_file_info: u32,
        u_flags: u32,
    ) -> usize;
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct ShFileInfoW {
    h_icon: *mut c_void,
    i_icon: i32,
    dw_attributes: u32,
    sz_display_name: [u16; 260],
    sz_type_name: [u16; 80],
}

#[cfg(target_os = "windows")]
impl Default for ShFileInfoW {
    fn default() -> Self {
        Self {
            h_icon: std::ptr::null_mut(),
            i_icon: 0,
            dw_attributes: 0,
            sz_display_name: [0; 260],
            sz_type_name: [0; 80],
        }
    }
}

#[cfg(target_os = "windows")]
#[link(name = "ole32")]
unsafe extern "system" {
    fn CoTaskMemFree(pv: *mut c_void);
}

#[cfg(target_os = "windows")]
fn resolve_windows_special_dir(path: &str) -> Option<PathBuf> {
    windows_known_folders()
        .iter()
        .find(|folder| windows_known_folder_name_matches(folder, path))
        .map(|folder| folder.path.clone())
}

#[cfg(target_os = "windows")]
pub fn is_windows_known_folder_segment(segment: &str) -> bool {
    let trimmed = segment.trim();
    if trimmed == "yt-dlp-gui" {
        return true;
    }
    windows_known_folders()
        .iter()
        .any(|folder| windows_known_folder_name_matches(folder, trimmed))
}

#[cfg(not(target_os = "windows"))]
pub fn is_windows_known_folder_segment(segment: &str) -> bool {
    segment.trim() == "yt-dlp-gui"
}

#[cfg(target_os = "windows")]
fn windows_known_folder_display_name(path: &str) -> Option<String> {
    windows_known_folders()
        .iter()
        .find(|folder| windows_known_folder_name_matches(folder, path))
        .map(|folder| folder.display_name.clone())
}

#[cfg(target_os = "windows")]
fn match_windows_special_dir_path(path: &str) -> Option<String> {
    let candidate = PathBuf::from(path);
    if !candidate.is_absolute() {
        return None;
    }

    if candidate == portable_root_dir() {
        return Some("yt-dlp-gui".to_owned());
    }

    windows_known_folders()
        .iter()
        .find(|folder| same_windows_path(&candidate, &folder.path))
        .map(|folder| folder.display_name.clone())
}

#[cfg(target_os = "windows")]
static WINDOWS_KNOWN_FOLDERS: OnceLock<Vec<WindowsKnownFolder>> = OnceLock::new();

#[cfg(target_os = "windows")]
fn windows_known_folders() -> &'static [WindowsKnownFolder] {
    WINDOWS_KNOWN_FOLDERS
        .get_or_init(load_windows_known_folders)
        .as_slice()
}

#[cfg(target_os = "windows")]
fn load_windows_known_folders() -> Vec<WindowsKnownFolder> {
    [
        ("Desktop", FOLDERID_DESKTOP),
        ("Documents", FOLDERID_DOCUMENTS),
        ("Downloads", FOLDERID_DOWNLOADS),
        ("Music", FOLDERID_MUSIC),
        ("Videos", FOLDERID_VIDEOS),
    ]
    .into_iter()
    .filter_map(|(canonical_name, guid)| {
        let path = sh_get_known_folder_path(guid)?;
        let display_name = shell_display_name_for_path(&path)
            .or_else(|| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .filter(|name| !name.trim().is_empty())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| canonical_name.to_owned());
        Some(WindowsKnownFolder {
            canonical_name,
            display_name,
            path,
        })
    })
    .collect()
}

#[cfg(target_os = "windows")]
fn windows_known_folder_name_matches(folder: &WindowsKnownFolder, input: &str) -> bool {
    input.eq_ignore_ascii_case(folder.display_name.as_str())
        || input.eq_ignore_ascii_case(folder.canonical_name)
}

#[cfg(target_os = "windows")]
fn same_windows_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy()
        .eq_ignore_ascii_case(&b.to_string_lossy())
}

#[cfg(target_os = "windows")]
fn shell_display_name_for_path(path: &Path) -> Option<String> {
    let wide_path = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut info = ShFileInfoW::default();
    let result = unsafe {
        SHGetFileInfoW(
            wide_path.as_ptr(),
            0,
            &mut info as *mut ShFileInfoW,
            std::mem::size_of::<ShFileInfoW>() as u32,
            SHGFI_DISPLAYNAME,
        )
    };
    if result == 0 {
        return None;
    }

    let len = info
        .sz_display_name
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(info.sz_display_name.len());
    let display_name = OsString::from_wide(&info.sz_display_name[..len])
        .to_string_lossy()
        .trim()
        .to_owned();
    if display_name.is_empty() {
        None
    } else {
        Some(display_name)
    }
}

#[cfg(target_os = "windows")]
fn sh_get_known_folder_path(guid: Guid) -> Option<PathBuf> {
    unsafe {
        let mut raw_path: *mut u16 = std::ptr::null_mut();
        let hr = SHGetKnownFolderPath(
            &guid as *const Guid,
            0,
            std::ptr::null_mut(),
            &mut raw_path as *mut *mut u16,
        );
        if hr < 0 || raw_path.is_null() {
            return None;
        }

        let mut len = 0usize;
        while *raw_path.add(len) != 0 {
            len += 1;
        }
        let wide = std::slice::from_raw_parts(raw_path, len);
        let path = PathBuf::from(OsString::from_wide(wide));
        CoTaskMemFree(raw_path.cast::<c_void>());
        Some(path)
    }
}
