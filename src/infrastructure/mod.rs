pub(crate) mod app_identity;
pub(crate) mod aria2_progress;
pub(crate) mod async_http;
mod browser_cookie_persistence;
mod browser_gallery_media;
mod browser_gallery_path;
mod browser_gallery_timeline;
mod browser_native_cookies;
pub(crate) mod browser_native_messaging;
mod component_update;
mod config;
pub(crate) mod cookie_site_index;
mod cookie_site_scope;
pub(crate) mod edition;
pub(crate) mod extractor_index;
mod gallery_browser_stream;
pub(crate) mod gallery_dl;
pub(crate) mod gallery_xmp;
pub(crate) mod gallery_xmp_packet;
mod media_session;
mod notification;
pub(crate) mod operation_diagnostics;
mod output_actions;
mod portable_cleanup;
mod prepare_check;
mod process_guard;
pub(crate) mod queue_store;
pub(crate) mod queue_thumbnail_cache;
pub(crate) mod render_backend;
pub(crate) mod runtime_paths;
mod sha256;
pub(crate) mod startup_diagnostics;
mod tool_install;
mod tools;
pub(crate) mod wgpu_api;
pub(crate) mod xmp;
pub(crate) mod yaml_store;
mod youtube_login_rescue;
pub(crate) mod yt_dlp_chapters;
mod yt_dlp_playlist_analysis;

pub use component_update::{
    AppInstanceGuard, ComponentOwnership, ComponentUpdateAction, ComponentUpdateEntry,
    ComponentUpdateEvent, ComponentUpdateSnapshot, ComponentUpdateStatus, ManagedComponentId,
    apply_update_args_requested, cleanup_applied_update, component_update_startup_snapshot,
    launch_pending_app_update, parse_apply_update_args, probe_dependency_tool_version,
    register_app_instance, resume_pending_app_update_on_launch, run_apply_update,
    run_component_update_action,
};
pub use config::{
    AppConfig, AudioPolicy, CompatibilityTarget, ConfigFileOption, ContainerPolicy,
    FormatPickerViewPreference, FrameRatePolicy, OutputFileActionMode, PostProcessMode,
    ResolutionPolicy, SerializableCacheLocationMode, SubtitlePolicy, ThemeAccentColor, ThemeMode,
    TranscodeIntentMode, TranscodeIntentSettings, TranscodeSettingKey, UnknownFilesizeFormatPolicy,
    VideoCodecPolicy, WindowPosition, WindowSize, YoutubeVideoPlaylistMode,
    available_yt_dlp_config_files, normalize_ui_scale_percent, yt_dlp_configs_dir_display,
};
pub use edition::{
    load_current_runtime_edition, parse_edition_command, run_edition_command, runtime_window_title,
    set_runtime_edition,
};
pub use media_session::{
    MediaSession, MediaSessionCommand, MediaSessionPlaybackStatus, MediaSessionTimeline,
    MediaSessionTrack,
};
pub use notification::{send_download_failed_windows_toast, send_download_finished_windows_toast};

pub use output_actions::{
    open_external_url, open_output_file, open_output_folder, output_file_exists,
    output_parent_folder_exists,
};
pub use portable_cleanup::schedule_startup_transient_cleanup;
pub use prepare_check::{
    PrepareReport, PrepareRequirement, PrepareStatus, collect_dependency_presence_report,
    collect_prepare_report,
};
pub use process_guard::{
    force_cleanup_tracked_processes, install_process_cleanup_panic_hook,
    run_tracked_command_output, track_child_process,
};
pub use startup_diagnostics::{
    enable_startup_diagnostics_if_requested, record_startup_checkpoint, record_startup_error,
    record_startup_event,
};
pub use tool_install::{
    DependencyTool, dependency_tool_exists, dependency_tool_is_available,
    dependency_tool_path_is_managed, detect_dependency_tool,
};
pub use tools::{
    AnalyzeError, AnalyzeOutput, BrowserCookieProfileOption, BrowserCookieSourceOption,
    CacheLocationMode, DownloadRequest, DownloadTargetKind, FINAL_OUTPUT_PATH_PREFIX, FileTimeMode,
    PreparedDownload, ToolPaths, YoutubePlaylistRisk, classify_youtube_playlist,
    configure_background_command, display_output_dir, humanize_yt_dlp_error,
    is_windows_known_folder_segment, playlist_entry_url, resolve_output_dir, resolve_tool_path,
    youtube_url_force_single_video, youtube_url_has_video_and_playlist,
};
pub(crate) mod yt_dlp_source_analysis;
pub use youtube_login_rescue::{
    YoutubeLoginRescueBrowserInfo, YoutubeLoginRescueCookieExport, YoutubeLoginRescueEvent,
    detect_default_youtube_login_rescue_browser, normalize_cookie_rescue_target_url,
    run_youtube_login_rescue_cookie_export,
};
