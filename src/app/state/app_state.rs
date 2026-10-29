use super::*;

pub struct AppState {
    pub active_tab: AppTab,
    pub url_input: String,
    pub(crate) url_source_preview: super::url_source_preview::UrlSourcePreview,
    pub batch_input: String,
    pub batch_enabled: bool,
    pub monitor_clipboard: bool,
    pub(super) last_clipboard_text: String,
    pub(super) last_clipboard_check: Option<Instant>,
    pub(super) clipboard_monitor_baseline_ready: bool,
    pub empty_item_preview: VideoMetadata,
    pub queue_items: Vec<QueueItem>,
    pub(super) queue_persistence: super::queue_persistence_actions::QueuePersistenceState,
    pub(super) queue_refresh_rx: Receiver<super::queue_reanalysis_actions::QueueRefreshResult>,
    pub(super) queue_refresh_tx: Sender<super::queue_reanalysis_actions::QueueRefreshResult>,
    pub(super) queue_display_mode: QueueDisplayMode,
    pub(super) app_mode: AppMode,
    pub item_defaults: DownloadOptions,
    pub config: AppConfig,
    pub output_draft: crate::infrastructure::output_policy::OutputPreferences,
    pub output_settings_feedback: String,
    pub runtime_renderer: Option<crate::infrastructure::wgpu_api::RuntimeRendererInfo>,
    pub(super) pending_ui_scale_percent: u16,
    pub options_detail_page: Option<OptionsDetailPage>,
    pub prepare_detail_page: Option<PrepareDetailPage>,
    pub advance_detail_page: Option<AdvanceDetailPage>,
    pub about_detail_target: AboutDetailTarget,
    pub tool_paths: ToolPaths,
    #[cfg(test)]
    pub(super) last_dispatched_download: Option<DownloadRequest>,
    #[cfg(test)]
    pub(super) review_capture_stream_preparation: bool,
    #[cfg(test)]
    pub(super) review_prepared_stream: Option<ResolvedMusicStream>,
    pub prepare_report: PrepareReport,
    pub(super) gallery_readiness: super::gallery_readiness_actions::GalleryReadinessState,
    pub(super) prepare_tab_snoozed: bool,
    pub last_action: String,
    pub(super) logs: LogState,
    pub format_picker: FormatPickerState,
    pub gallery_range_picker: GalleryRangePicker,
    pub(super) pending_queue_removals: Vec<QueueItemId>,
    pub is_adding_batch: bool,
    pub is_cancelling_batch_add: bool,
    pub youtube_playlist_prompt: Option<YoutubePlaylistPrompt>,
    // URL matcher runs before downloader choice; selected analyzer then runs.
    pub(super) gallery_route_pending: bool,
    pub(super) gallery_downloader_choice_prompt:
        Option<super::gallery_queue_actions::GalleryDownloaderChoicePrompt>,
    // Audio mode preflights only yt-dlp rules; no gallery-dl process is ever launched.
    pub(super) audio_url_preflight_pending: bool,
    pub(super) audio_url_preflight_rx:
        Receiver<super::music_batch_actions::AudioSourcePreflightResult>,
    pub(super) audio_url_preflight_tx:
        Sender<super::music_batch_actions::AudioSourcePreflightResult>,
    pub(super) gallery_route_prompt: Option<super::gallery_queue_actions::GalleryRoutePrompt>,
    pub(super) gallery_route_rx: Receiver<super::gallery_queue_actions::GalleryRouteResult>,
    pub(super) gallery_route_tx: Sender<super::gallery_queue_actions::GalleryRouteResult>,
    pub(super) cookie_rescue: CookieRescueState,
    pub(super) analyze_result_rx: Receiver<AnalyzeResult>,
    pub(super) analyze_result_tx: Sender<AnalyzeResult>,
    pub(super) batch_add_result_rx: Option<Receiver<BatchAddEvent>>,
    pub(super) batch_add_child: Option<Arc<Mutex<Option<Child>>>>,
    pub(super) batch_add_cancel_requested: Option<Arc<AtomicBool>>,
    pub(super) batch_add_music_compact: bool,
    pub(super) download_result_rx: Receiver<DownloadEvent>,
    pub(super) download_result_tx: Sender<DownloadEvent>,
    pub(super) post_process_result_rx: Receiver<PostProcessEvent>,
    pub(super) post_process_result_tx: Sender<PostProcessEvent>,
    pub(super) component_update_result_rx: Receiver<ComponentUpdateEvent>,
    pub(super) component_update_result_tx: Sender<ComponentUpdateEvent>,
    pub(super) thumbnail_result_rx: Receiver<ThumbnailFetchEvent>,
    pub(super) thumbnail_result_tx: Sender<ThumbnailFetchEvent>,
    pub(super) music_stream_result_rx: Receiver<MusicStreamResolveEvent>,
    pub(super) music_stream_result_tx: Sender<MusicStreamResolveEvent>,
    pub(super) music_playback_event_rx: Receiver<MusicPlaybackEvent>,
    pub(super) music_playback_event_tx: Sender<MusicPlaybackEvent>,
    pub(super) music_download_event_rx: Receiver<MusicDownloadEvent>,
    pub(super) music_download_event_tx: Sender<MusicDownloadEvent>,
    pub(super) music: MusicState,
    pub(super) thumbnail_cache: HashMap<String, ThumbnailCacheEntry>,
    pub(super) queue_thumbnail_disk_cache:
        Option<Arc<crate::infrastructure::queue_thumbnail_cache::QueueThumbnailCache>>,
    pub(super) queue_thumbnail_saved_owners: std::collections::BTreeSet<String>,
    pub component_update_snapshot: ComponentUpdateSnapshot,
    pub(super) app_instance_guard: Option<AppInstanceGuard>,
    pub(super) font_content_revision: u64,
    pub(super) active_workflows: HashMap<WorkflowRunId, ActiveWorkflow>,
    pub(super) next_queue_item_id: QueueItemId,
    pub(super) next_workflow_run_id: WorkflowRunId,
}

impl AppState {
    pub fn new() -> Self {
        let (config, tool_paths) = AppConfig::load_runtime();
        Self::from_runtime(config, tool_paths)
    }

    pub fn from_runtime(mut config: AppConfig, tool_paths: ToolPaths) -> Self {
        let (analyze_result_tx, analyze_result_rx) = mpsc::channel();
        let (queue_refresh_tx, queue_refresh_rx) = mpsc::channel();
        let (gallery_route_tx, gallery_route_rx) = mpsc::channel();
        let (audio_url_preflight_tx, audio_url_preflight_rx) = mpsc::channel();
        let (download_result_tx, download_result_rx) = mpsc::channel();
        let (post_process_result_tx, post_process_result_rx) = mpsc::channel();
        let (component_update_result_tx, component_update_result_rx) = mpsc::channel();
        let (thumbnail_result_tx, thumbnail_result_rx) = mpsc::channel();
        let (music_stream_result_tx, music_stream_result_rx) = mpsc::channel();
        let (music_playback_event_tx, music_playback_event_rx) = mpsc::channel();
        let (music_download_event_tx, music_download_event_rx) = mpsc::channel();
        let pending_ui_scale_percent = config.ui_scale_percent;
        let music_volume = config.music_volume.clamp(0.0, 1.0);
        let music_playback_mode = MusicPlaybackMode::from_config_value(&config.music_playback_mode);
        let app_mode = AppMode::from_config_value(&config.app_mode);
        let queue_display_mode = QueueDisplayMode::from_app_mode(app_mode);
        config.app_mode = app_mode.config_value().to_owned();
        config.queue_display_mode = queue_display_mode.config_value().to_owned();
        let mut state = Self {
            #[cfg(test)]
            last_dispatched_download: None,
            #[cfg(test)]
            review_capture_stream_preparation: false,
            #[cfg(test)]
            review_prepared_stream: None,
            runtime_renderer: None,
            active_tab: AppTab::Main,
            url_input: String::new(),
            url_source_preview: Default::default(),
            batch_input: String::new(),
            batch_enabled: true,
            monitor_clipboard: config.auto_paste_clipboard,
            last_clipboard_text: String::new(),
            last_clipboard_check: config.auto_paste_clipboard.then(Instant::now),
            clipboard_monitor_baseline_ready: false,
            empty_item_preview: VideoMetadata::empty_preview(),
            queue_items: Vec::new(),
            queue_persistence: Default::default(),
            queue_refresh_rx,
            queue_refresh_tx,
            queue_display_mode,
            app_mode,
            item_defaults: {
                let mut defaults = DownloadOptions::default();
                defaults.output_dir = config.download_dir.clone();
                defaults.use_cookies = config.use_browser_cookies;
                defaults.use_aria2 = config.use_aria2;
                defaults.write_thumbnail = config.thumbnail_mode.writes();
                defaults.embed_thumbnail = config.thumbnail_mode.embeds();
                defaults.write_subtitles = config.subtitle_mode.writes();
                defaults.embed_subtitles = config.subtitle_mode.embeds();
                defaults.write_chapters = config.chapter_mode.writes();
                defaults.embed_chapters = config.chapter_mode.embeds();
                defaults
            },
            output_draft: config.output_preferences(),
            output_settings_feedback: config.output_settings_error.clone().unwrap_or_default(),
            config,
            pending_ui_scale_percent,
            options_detail_page: None,
            prepare_detail_page: None,
            advance_detail_page: None,
            about_detail_target: AboutDetailTarget::App,
            tool_paths,
            prepare_report: PrepareReport::default(),
            gallery_readiness: Default::default(),
            prepare_tab_snoozed: false,
            last_action: String::new(),
            logs: LogState::default(),
            format_picker: FormatPickerState::default(),
            gallery_range_picker: GalleryRangePicker::default(),
            pending_queue_removals: Vec::new(),
            is_adding_batch: false,
            is_cancelling_batch_add: false,
            youtube_playlist_prompt: None,
            gallery_route_pending: false,
            gallery_downloader_choice_prompt: None,
            audio_url_preflight_pending: false,
            audio_url_preflight_rx,
            audio_url_preflight_tx,
            gallery_route_prompt: None,
            gallery_route_rx,
            gallery_route_tx,
            cookie_rescue: CookieRescueState::default(),
            analyze_result_rx,
            analyze_result_tx,
            batch_add_result_rx: None,
            batch_add_child: None,
            batch_add_cancel_requested: None,
            batch_add_music_compact: false,
            download_result_rx,
            download_result_tx,
            post_process_result_rx,
            post_process_result_tx,
            component_update_result_rx,
            component_update_result_tx,
            thumbnail_result_rx,
            thumbnail_result_tx,
            music_stream_result_rx,
            music_stream_result_tx,
            music_playback_event_rx,
            music_playback_event_tx,
            music_download_event_rx,
            music_download_event_tx,
            music: MusicState::new(music_volume, music_playback_mode),
            thumbnail_cache: HashMap::new(),
            queue_thumbnail_disk_cache: None,
            queue_thumbnail_saved_owners: Default::default(),
            component_update_snapshot: component_update_startup_snapshot(),
            app_instance_guard: register_app_instance(),
            font_content_revision: 1,
            active_workflows: HashMap::new(),
            next_queue_item_id: 1,
            next_workflow_run_id: 1,
        };

        if let Some(error) = state.cache_storage_error() {
            state.last_action =
                format!("Cache storage: {error}. Choose a valid location in Options and restart.");
        }
        cleanup_applied_update();
        state.restore_saved_queues();
        state.request_gallery_readiness(false);
        state.prepare_report = collect_dependency_presence_report(&state.tool_paths, None);
        state.sanitize_startup_prepare_component_update_snapshot();
        schedule_startup_transient_cleanup();
        if state.should_open_component_pool_on_startup() {
            if let Some(tool) = state.first_missing_core_dependency() {
                state.open_dependency_component(tool);
            }
        } else if state.should_show_prepare_tab() {
            // Non-tool environment failures still use the dedicated
            // diagnostics page until they receive a component-pool item.
            state.active_tab = AppTab::Prepare;
        }
        state
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        self.flush_queue_persistence_before_exit();
        self.force_stop_owned_external_tools();
    }
}

impl AppState {
    fn force_stop_owned_external_tools(&mut self) {
        if let Some(cancel_requested) = self.batch_add_cancel_requested.take() {
            cancel_requested.store(true, Ordering::Relaxed);
        }
        if let Some(child_handle) = self.batch_add_child.take() {
            Self::terminate_child_handle(&child_handle);
        }

        for workflow in self.active_workflows.values_mut() {
            if let Some(cancel_requested) = workflow.cancel_requested.as_ref() {
                cancel_requested.store(true, Ordering::Relaxed);
            }
            if let Some(child_handle) = workflow.download_child.as_ref() {
                Self::terminate_child_handle(child_handle);
            }
        }
        self.active_workflows.clear();

        crate::infrastructure::force_cleanup_tracked_processes();
    }

    fn terminate_child_handle(child_handle: &Arc<Mutex<Option<Child>>>) {
        let Ok(mut guard) = child_handle.lock() else {
            return;
        };
        let Some(mut child) = guard.take() else {
            return;
        };
        terminate_child_process(&mut child);
        let _ = child.wait();
    }
}
