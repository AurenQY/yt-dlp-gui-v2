//! Single source of truth for persisted paths, ordering and English comments.
//! AppConfig serde remains an internal value snapshot, not the on-disk format.
pub(super) struct Field {
    pub source: &'static str,
    pub path: &'static str,
    pub comment: &'static str,
}

pub(super) const FIELDS: &[Field] = &[
    Field {
        source: "output_naming",
        path: "storage.output.naming",
        comment: "Automatic filename rule: title | title_id | uploader_title_id | custom.",
    },
    Field {
        source: "output_existing_file",
        path: "storage.output.existing_file",
        comment: "Existing output policy: keep_both | skip. Existing files are never silently replaced.",
    },
    Field {
        source: "output_template",
        path: "storage.output.template",
        comment: "Custom basename fields: {title}, {id}, {uploader}, {upload_date}; {{ and }} escape braces. No directory or extension.",
    },
    Field {
        source: "download_dir",
        path: "storage.download_dir",
        comment: "Download directory. Relative paths use the application data root.",
    },
    Field {
        source: "cache_location_mode",
        path: "storage.cache.mode",
        comment: "Application cache location: v2_cache | windows_temp. Changes take effect after restart; legacy yt_dlp_default maps to v2_cache.",
    },
    Field {
        source: "cache_dir",
        path: "storage.cache.dir",
        comment: "Portable cache directory, used by v2_cache. Relative paths use the application root; a missing directory is created on startup.",
    },
    Field {
        source: "yt_dlp_path",
        path: "tools.yt_dlp.path",
        comment: "Custom yt-dlp executable. Omit to detect automatically.",
    },
    Field {
        source: "gallery_dl_path",
        path: "tools.gallery_dl.path",
        comment: "Custom gallery-dl executable. Omit to detect automatically.",
    },
    Field {
        source: "yt_dlp_config_path",
        path: "tools.yt_dlp.config_file",
        comment: "Additional yt-dlp configuration file. Empty means none.",
    },
    Field {
        source: "ffmpeg_path",
        path: "tools.ffmpeg.path",
        comment: "Custom FFmpeg executable. Omit to detect automatically.",
    },
    Field {
        source: "aria2c_path",
        path: "tools.aria2c.path",
        comment: "Custom aria2c executable. Omit to detect automatically.",
    },
    Field {
        source: "deno_path",
        path: "tools.deno.path",
        comment: "Custom Deno executable. Omit to detect automatically.",
    },
    Field {
        source: "proxy_enabled",
        path: "network.proxy.enabled",
        comment: "Enable the configured proxy.",
    },
    Field {
        source: "proxy_url",
        path: "network.proxy.url",
        comment: "Proxy URL. Kept when the proxy is disabled.",
    },
    Field {
        source: "no_check_certificates",
        path: "network.no_check_certificates",
        comment: "Skip HTTPS certificate verification.",
    },
    Field {
        source: "impersonate",
        path: "network.impersonate",
        comment: "yt-dlp impersonation target, for example chrome. Empty disables it.",
    },
    Field {
        source: "limit_rate",
        path: "network.limit_rate",
        comment: "Download speed limit, for example 5M. Empty means unlimited.",
    },
    Field {
        source: "use_browser_cookies",
        path: "network.cookies.enabled",
        comment: "Enable cookies for website requests.",
    },
    Field {
        source: "browser_cookie_source",
        path: "network.cookies.source",
        comment: "Browser cookie source, for example chrome, firefox or auto.",
    },
    Field {
        source: "browser_cookie_profile",
        path: "network.cookies.profile",
        comment: "Browser profile. Empty uses the default profile.",
    },
    Field {
        source: "browser_cookie_file",
        path: "network.cookies.file",
        comment: "Cookie file path. Empty means no explicit file.",
    },
    Field {
        source: "youtube_subs_po_token",
        path: "youtube.subtitles.po_token",
        comment: "YouTube subtitle PO token. Empty means none.",
    },
    Field {
        source: "youtube_extractor_args",
        path: "youtube.extractor_args",
        comment: "Additional YouTube extractor arguments.",
    },
    Field {
        source: "youtube_video_playlist_mode",
        path: "youtube.playlist.video_mode",
        comment: "Combined video/playlist URL behavior: ask | video | ignore.",
    },
    Field {
        source: "youtube_high_risk_playlist_prompt",
        path: "youtube.playlist.high_risk_prompt",
        comment: "Ask before processing high-risk playlists.",
    },
    Field {
        source: "concurrent_fragments",
        path: "download.concurrent_fragments",
        comment: "Concurrent fragments per download: 1 | 2 | 4 | 8.",
    },
    Field {
        source: "live_from_start",
        path: "download.live_from_start",
        comment: "Download live streams from the beginning when supported.",
    },
    Field {
        source: "use_aria2",
        path: "download.use_aria2",
        comment: "Use aria2c for supported downloads.",
    },
    Field {
        source: "file_time_mode",
        path: "download.file_time_mode",
        comment: "File timestamp: none | use_upload_date | use_download_time.",
    },
    Field {
        source: "unknown_filesize_format_policy",
        path: "download.unknown_filesize_policy",
        comment: "Unknown file size handling: off | filter.",
    },
    Field {
        source: "download_sections",
        path: "download.range.sections",
        comment: "yt-dlp download sections. Empty downloads the full item.",
    },
    Field {
        source: "always_show_download_range",
        path: "download.range.always_show",
        comment: "Always show download range controls.",
    },
    Field {
        source: "chapter_compatibility_mode",
        path: "download.range.chapter_compatibility",
        comment: "Enable chapter range compatibility handling.",
    },
    Field {
        source: "thumbnail_mode",
        path: "post_process.thumbnail",
        comment: "Thumbnail handling: off | download | embed.",
    },
    Field {
        source: "subtitle_mode",
        path: "post_process.subtitle",
        comment: "Subtitle handling: off | download | embed.",
    },
    Field {
        source: "chapter_mode",
        path: "post_process.chapter",
        comment: "Chapter handling: off | download | embed.",
    },
    Field {
        source: "post_download_conversion_enabled",
        path: "post_process.conversion.enabled",
        comment: "Enable conversion after downloading.",
    },
    Field {
        source: "transcode_intent.video_codec_policy",
        path: "post_process.conversion.video_codec",
        comment: "Video codec: auto | h264 | hevc | av1.",
    },
    Field {
        source: "transcode_intent.container_policy",
        path: "post_process.conversion.container",
        comment: "Output container: auto | mp4 | mkv | mov.",
    },
    Field {
        source: "transcode_intent.audio_policy",
        path: "post_process.conversion.audio",
        comment: "Audio policy: auto | aac | opus | flac.",
    },
    Field {
        source: "transcode_intent.subtitle_policy",
        path: "post_process.conversion.subtitle",
        comment: "Subtitle policy: preserve | embed | burn.",
    },
    Field {
        source: "auto_paste_clipboard",
        path: "workflow.clipboard.monitor",
        comment: "Monitor the clipboard for supported URLs.",
    },
    Field {
        source: "clipboard_auto_add",
        path: "workflow.clipboard.auto_add",
        comment: "Automatically add detected clipboard URLs.",
    },
    Field {
        source: "auto_analyze",
        path: "workflow.auto_analyze",
        comment: "Automatically analyze added items.",
    },
    Field {
        source: "force_analysis",
        path: "workflow.force_analysis",
        comment: "Ignore the embedded URL index and try all installed analysis tools.",
    },
    Field {
        source: "analysis_choice_yt_dlp",
        path: "workflow.analysis_choice.yt_dlp",
        comment: "Last confirmed yt-dlp checkbox; restores the choice without confirming it.",
    },
    Field {
        source: "analysis_choice_gallery_dl",
        path: "workflow.analysis_choice.gallery_dl",
        comment: "Last confirmed gallery-dl checkbox; restores the choice without confirming it.",
    },
    Field {
        source: "direct_download_on_add",
        path: "workflow.direct_download_on_add",
        comment: "Start downloading directly when adding an item.",
    },
    Field {
        source: "output_file_action_mode",
        path: "workflow.output_action",
        comment: "Output action: menu | open_folder | open_file.",
    },
    Field {
        source: "enable_batch_panel",
        path: "workflow.batch.panel_enabled",
        comment: "Show the batch panel.",
    },
    Field {
        source: "batch_limit_enabled",
        path: "workflow.batch.limit.enabled",
        comment: "Limit the number of batch items.",
    },
    Field {
        source: "batch_limit_count",
        path: "workflow.batch.limit.count",
        comment: "Maximum batch item count when the limit is enabled.",
    },
    Field {
        source: "music_volume",
        path: "player.volume",
        comment: "Playback volume from 0.0 to 1.0.",
    },
    Field {
        source: "music_playback_mode",
        path: "player.playback_mode",
        comment: "Playback order: sequential | repeat_all | shuffle | repeat_one.",
    },
    Field {
        source: "app_mode",
        path: "interface.mode",
        comment: "Application mode: origin | standard | audio.",
    },
    Field {
        source: "language",
        path: "interface.language",
        comment: "Language: auto or a locale code such as zh-TW.",
    },
    Field {
        source: "show_log_tab",
        path: "interface.show_log_tab",
        comment: "Show the log tab.",
    },
    Field {
        source: "windows_toast_enabled",
        path: "interface.windows_toast_enabled",
        comment: "Enable Windows toast notifications.",
    },
    Field {
        source: "browser_integration_enabled",
        path: "integration.browser.enabled",
        comment: "Automatically register and repair the browser extension connection.",
    },
    Field {
        source: "theme_mode",
        path: "interface.appearance.theme",
        comment: "Theme: system | light | dark.",
    },
    Field {
        source: "theme_accent_color",
        path: "interface.appearance.accent",
        comment: "Accent: off | system | blue | purple | pink | green | orange | slate.",
    },
    Field {
        source: "ui_scale_percent",
        path: "interface.appearance.scale_percent",
        comment: "User interface scale in percent, from 80 to 200.",
    },
    Field {
        source: "render_backend",
        path: "interface.graphics.backend",
        comment: "Rendering backend: glow | wgpu. Applied on the next launch.",
    },
    Field {
        source: "wgpu_api",
        path: "interface.graphics.wgpu_api",
        comment: "Wgpu API: auto | dx12 | vulkan | open_gl. Applied on the next launch.",
    },
    Field {
        source: "keep_window_on_top",
        path: "window.always_on_top",
        comment: "Keep the window above other windows.",
    },
    Field {
        source: "remember_window_position",
        path: "window.remember_position",
        comment: "Remember the window position between launches.",
    },
    Field {
        source: "remember_window_size",
        path: "window.remember_size",
        comment: "Remember the window size between launches.",
    },
    Field {
        source: "prepare_skipped",
        path: "state.prepare_skipped",
        comment: "Whether the user skipped the preparation workflow.",
    },
    Field {
        source: "component_update_cache",
        path: "state.component_updates",
        comment: "Last checked component versions and time (informational; not update authorization).",
    },
    Field {
        source: "options_scroll_offset",
        path: "state.scroll.options",
        comment: "Saved options scroll offset in logical points.",
    },
    Field {
        source: "advance_scroll_offset",
        path: "state.scroll.advanced",
        comment: "Saved advanced settings scroll offset in logical points.",
    },
    Field {
        source: "format_picker_view_preference",
        path: "state.format_picker_view",
        comment: "Preferred format view: filter | table. Omit for automatic selection.",
    },
    Field {
        source: "window_position",
        path: "state.window.position",
        comment: "Saved window coordinates: x and y.",
    },
    Field {
        source: "window_size",
        path: "state.window.size",
        comment: "Saved window dimensions: width and height.",
    },
];

pub(super) const GROUPS: &[(&str, &str)] = &[
    ("storage", "Download and cache locations."),
    (
        "tools",
        "External tools. Standard bundled paths are omitted.",
    ),
    ("network", "Network and website authentication settings."),
    ("youtube", "YouTube-specific settings."),
    ("download", "Download behavior and format selection."),
    ("post_process", "Post-download processing."),
    ("workflow", "Item addition and batch workflows."),
    ("player", "Music playback preferences."),
    ("interface", "Interface and appearance preferences."),
    ("integration", "Connections to external applications."),
    ("integration.browser", "Browser extension connection."),
    (
        "interface.graphics",
        "Graphics settings. Changes take effect on the next launch.",
    ),
    ("window", "Window preferences."),
    ("state", "Automatically saved application state."),
];
