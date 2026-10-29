//! Standard-mode backend selection and gallery-dl download lifecycle.
use super::gallery_analysis_log::{analysis_worker_panic, send_analysis_log};
use super::*;
use crate::app::download_worker::{DownloadResult, wait_download_child};
use crate::app::gallery_canonical_metadata::map_gallery_artifact_metadata;
use crate::app::gallery_metadata_mapping::map_gallery_ui_metadata;
use crate::domain::GalleryAnalysis;
use crate::infrastructure::extractor_index::{
    MatchedTools, gallery_replaces_broken_yt, match_embedded_url,
};
use crate::infrastructure::gallery_dl::{
    GalleryDlBackend, GalleryDlMetadataEvent, parse_gallery_dl_download_event,
};
use crate::infrastructure::gallery_xmp::write_gallery_xmp;

pub(super) struct GalleryRouteResult {
    pub(super) source: String,
    pub(super) video_source: String,
    pub(super) kind: GalleryRoutePromptKind,
    pub(super) outcome: GalleryRouteOutcome,
}

pub(super) enum GalleryRouteOutcome {
    Log {
        action_id: u64,
        tool: &'static str,
        status: ToolLogStatus,
        command: String,
        detail: String,
    },
    VideoReady(Value),
    GalleryReady(GalleryAnalysis),
    AnalyzerFailed(&'static str, String),
    Error(String),
    ChooseDownloader,
    YtPlaylist {
        gallery: Option<GalleryAnalysis>,
    },
    Analyzed {
        video: Result<Value, String>,
        gallery: Result<GalleryAnalysis, String>,
        tried_video: bool,
        tried_gallery: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GalleryRoutePromptKind {
    Media,
    VideoAndPlaylist,
    Origin,
    Immediate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GalleryDownloaderChoice {
    YtDlp,
    GalleryDl,
    Both,
}

impl GalleryDownloaderChoice {
    fn from_checks(yt_dlp: bool, gallery_dl: bool) -> Option<Self> {
        match (yt_dlp, gallery_dl) {
            (true, true) => Some(Self::Both),
            (true, false) => Some(Self::YtDlp),
            (false, true) => Some(Self::GalleryDl),
            (false, false) => None,
        }
    }
}

pub(super) struct GalleryDownloaderChoicePrompt {
    source: String,
    kind: GalleryRoutePromptKind,
    selected: Option<GalleryDownloaderChoice>,
}

pub(super) struct GalleryRoutePrompt {
    pub(super) source: String,
    pub(super) video_source: String,
    pub(super) kind: GalleryRoutePromptKind,
    pub(super) video: Value,
    pub(super) gallery: GalleryAnalysis,
    pub(super) include_video: bool,
    pub(super) include_images: bool,
}

/// Return a routing/preflight problem through the existing GUI result channel.
/// Neither downloader has been launched for this result.
fn send_gallery_route_error(
    tx: &Sender<GalleryRouteResult>,
    action_id: u64,
    source: String,
    video_source: String,
    kind: GalleryRoutePromptKind,
    error: String,
) {
    send_analysis_log(
        tx,
        &source,
        &video_source,
        kind,
        action_id,
        "v2",
        ToolLogStatus::Failed,
        String::new(),
        error.clone(),
    );
    let _ = tx.send(GalleryRouteResult {
        source,
        video_source,
        kind,
        outcome: GalleryRouteOutcome::Error(error),
    });
}

/// Keep already analyzed playable children instead of flattening away their
/// format metadata. A child without a stable yt-dlp playlist index falls back
/// to the existing batch importer rather than risking the wrong download.
fn analyzed_playlist_entries(video: &Value) -> Option<Vec<(usize, Value)>> {
    let entries = video.get("entries")?.as_array()?;
    let mut playable = Vec::new();
    for entry in entries.iter().filter(|entry| !entry.is_null()) {
        let index = entry
            .get("playlist_index")
            .and_then(Value::as_u64)
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index > 0)?;
        if !entry
            .get("formats")
            .and_then(Value::as_array)
            .is_some_and(|formats| !formats.is_empty())
        {
            return None;
        }
        playable.push((index, entry.clone()));
    }
    (!playable.is_empty()).then_some(playable)
}

impl AppState {
    pub(super) fn start_standard_backend_probe(&mut self, source: String) {
        self.start_standard_backend_probe_with_kind(source, GalleryRoutePromptKind::Media, None);
    }

    pub(super) fn start_origin_backend_probe(&mut self, source: String) {
        self.start_standard_backend_probe_with_kind(source, GalleryRoutePromptKind::Origin, None);
    }

    pub(super) fn start_standard_video_playlist_probe(&mut self, source: String) {
        self.start_standard_backend_probe_with_kind(
            source,
            GalleryRoutePromptKind::VideoAndPlaylist,
            None,
        );
    }

    pub(super) fn start_immediate_backend_probe(&mut self, source: String) {
        // Keep Download-now's existing YouTube mixed-URL single-video intent.
        let source = if youtube_url_has_video_and_playlist(&source) {
            youtube_url_force_single_video(&source).unwrap_or(source)
        } else {
            source
        };
        self.start_standard_backend_probe_with_kind(
            source,
            GalleryRoutePromptKind::Immediate,
            None,
        );
    }

    fn start_standard_backend_probe_with_kind(
        &mut self,
        source: String,
        kind: GalleryRoutePromptKind,
        selected: Option<GalleryDownloaderChoice>,
    ) {
        if selected == Some(GalleryDownloaderChoice::Both)
            && (self.app_mode != AppMode::Standard || kind == GalleryRoutePromptKind::Origin)
        {
            self.last_action = "gallery.choice.one_required".to_owned();
            return;
        }
        if self.gallery_route_pending
            || self.gallery_route_prompt.is_some()
            || self.gallery_downloader_choice_prompt.is_some()
        {
            self.last_action = "Wait for the current URL selection to finish.".to_owned();
            return;
        }
        let action_id = self.push_tool_log_action(self.app_mode.config_value(), "source analysis");
        let trace = crate::infrastructure::operation_diagnostics::new_trace();
        self.push_runtime_log(format!(
            "Source analysis traceId={trace}; diagnostics=data/diagnostics/operations.jsonl"
        ));
        self.record_analysis_tool_log(
            action_id,
            "v2",
            ToolLogStatus::Running,
            String::new(),
            format!("traceId={trace}; stage=routing"),
        );
        self.gallery_route_pending = true;
        self.last_action = format!("Checking URL extractor rules: {source}");
        let tool_paths = self.tool_paths.clone();
        let use_cookies = self.item_defaults.use_cookies;
        let force_analysis = self.config.force_analysis;
        let tx = self.gallery_route_tx.clone();
        thread::spawn(move || {
            let _trace = crate::infrastructure::operation_diagnostics::enter(&trace);
            let mut route_span = crate::infrastructure::operation_diagnostics::OperationSpan::start(
                "desktop.route",
                serde_json::json!({"cookiesRequested": use_cookies, "forceAnalysis": force_analysis}),
            );
            let video_source = if matches!(
                kind,
                GalleryRoutePromptKind::VideoAndPlaylist | GalleryRoutePromptKind::Origin
            ) {
                youtube_url_force_single_video(&source).unwrap_or_else(|| source.clone())
            } else {
                source.clone()
            };
            // The matcher is compiled once on this worker, not during GUI
            // startup or a UI event. No version-number check is performed.
            let matched = if force_analysis {
                Ok(MatchedTools {
                    yt_dlp: true,
                    gallery_dl: true,
                })
            } else {
                match_embedded_url(&source)
            };
            let matched = match matched {
                Ok(matched) => matched,
                Err(error) => {
                    send_gallery_route_error(&tx, action_id, source, video_source, kind, error);
                    return;
                }
            };
            if !matched.yt_dlp && !matched.gallery_dl {
                send_gallery_route_error(
                    &tx,
                    action_id,
                    source,
                    video_source,
                    kind,
                    "No matching yt-dlp or gallery-dl URL rule. Enable Force analysis in Advanced settings to try both tools.".to_owned(),
                );
                return;
            }
            let gallery_backend = GalleryDlBackend::from_tool_paths(&tool_paths);
            let gallery_available = gallery_backend.is_available();
            let yt_dlp_available = tool_paths.validate_yt_dlp_available().is_ok();
            let selected = if selected.is_none()
                && !force_analysis
                && gallery_replaces_broken_yt(&source, matched, gallery_available).unwrap_or(false)
            {
                Some(GalleryDownloaderChoice::GalleryDl)
            } else {
                selected
            };
            if matched.yt_dlp
                && matched.gallery_dl
                && yt_dlp_available
                && gallery_available
                && selected.is_none()
            {
                route_span.finish("awaiting-choice", serde_json::json!({}));
                send_analysis_log(
                    &tx,
                    &source,
                    &video_source,
                    kind,
                    action_id,
                    "v2",
                    ToolLogStatus::Skipped,
                    String::new(),
                    format!(
                        "traceId={trace}; reason=user-choice-required; ytAvailable={yt_dlp_available}; galleryAvailable={gallery_available}"
                    ),
                );
                let _ = tx.send(GalleryRouteResult {
                    source,
                    video_source,
                    kind,
                    outcome: GalleryRouteOutcome::ChooseDownloader,
                });
                return;
            }
            let missing_selected_yt_dlp = matched.yt_dlp
                && !yt_dlp_available
                && selected != Some(GalleryDownloaderChoice::GalleryDl);
            let matched = MatchedTools {
                yt_dlp: matched.yt_dlp
                    && yt_dlp_available
                    && selected != Some(GalleryDownloaderChoice::GalleryDl),
                gallery_dl: matched.gallery_dl && selected != Some(GalleryDownloaderChoice::YtDlp),
            };
            if !matched.yt_dlp && !matched.gallery_dl {
                send_gallery_route_error(
                    &tx,
                    action_id,
                    source,
                    video_source,
                    kind,
                    if missing_selected_yt_dlp {
                        "This URL matches yt-dlp, but yt-dlp is not installed.".to_owned()
                    } else {
                        "This URL matches gallery-dl, but gallery-dl is not installed.".to_owned()
                    },
                );
                return;
            }

            let tried_gallery = matched.gallery_dl && gallery_available;
            if !matched.yt_dlp && !tried_gallery {
                send_gallery_route_error(
                    &tx,
                    action_id,
                    source,
                    video_source,
                    kind,
                    "This URL matches gallery-dl, but gallery-dl is not installed.".to_owned(),
                );
                return;
            }

            crate::infrastructure::operation_diagnostics::record(
                "route.plan",
                serde_json::json!({"ytSelected": matched.yt_dlp, "gallerySelected": tried_gallery, "ytAvailable": yt_dlp_available, "galleryAvailable": gallery_available, "explicitChoice": selected.is_some()}),
            );
            route_span.finish(
                "selected",
                serde_json::json!({"ytSelected": matched.yt_dlp, "gallerySelected": tried_gallery}),
            );
            send_analysis_log(
                &tx,
                &source,
                &video_source,
                kind,
                action_id,
                "v2",
                ToolLogStatus::Success,
                String::new(),
                format!(
                    "traceId={trace}; ytSelected={}; gallerySelected={tried_gallery}; ytAvailable={yt_dlp_available}; galleryAvailable={gallery_available}; forceAnalysis={force_analysis}; cookiesRequested={use_cookies}",
                    matched.yt_dlp
                ),
            );
            // The selected, installed downloader alone analyzes the URL.
            let video_worker = matched.yt_dlp.then(|| {
                let video_paths = tool_paths.clone();
                let source = video_source.clone();
                let tx = tx.clone();
                let route_source = source.clone();
                let trace = trace.clone();
                thread::spawn(move || {
                    let _trace = crate::infrastructure::operation_diagnostics::enter(&trace);
                    let started = std::time::Instant::now();
                    send_analysis_log(&tx, &route_source, &source, kind, action_id, "yt-dlp", ToolLogStatus::Running, String::new(), format!("traceId={trace}; cookiesRequested={use_cookies}; stage=starting"));
                    let output = crate::infrastructure::yt_dlp_source_analysis::analyze_desktop_source(
                        &video_paths,
                        &source,
                        use_cookies,
                    );
                    let command = match &output {
                        Ok(crate::infrastructure::yt_dlp_source_analysis::DesktopSourceAnalysis::Single(value)) => value.command_line.clone(),
                        Err(error) => error.command_line.clone().unwrap_or_default(),
                        _ => String::new(),
                    };
                    let count = match &output {
                        Ok(crate::infrastructure::yt_dlp_source_analysis::DesktopSourceAnalysis::Single(_)) => 1,
                        _ => 0,
                    };
                    let detail = format!("traceId={trace}; elapsedMs={}; mediaCount={count}; playlist={}; error={}", started.elapsed().as_millis(), matches!(&output, Ok(crate::infrastructure::yt_dlp_source_analysis::DesktopSourceAnalysis::Playlist)), output.as_ref().err().map(|e| e.message.as_str()).unwrap_or("none"));
                    send_analysis_log(&tx, &route_source, &source, kind, action_id, "yt-dlp", if output.is_ok() { ToolLogStatus::Success } else { ToolLogStatus::Failed }, command, detail);
                    let result = output.map_err(|error| error.message);
                    if kind == GalleryRoutePromptKind::Media {
                        if let Err(error) = &result {
                            let _ = tx.send(GalleryRouteResult {
                                source: route_source.clone(), video_source: source.clone(), kind,
                                outcome: GalleryRouteOutcome::AnalyzerFailed("yt-dlp", error.clone()),
                            });
                        }
                        if let Ok(crate::infrastructure::yt_dlp_source_analysis::DesktopSourceAnalysis::Single(output)) = &result {
                            let _ = tx.send(GalleryRouteResult {
                                source: route_source, video_source: source, kind,
                                outcome: GalleryRouteOutcome::VideoReady(output.json.clone()),
                            });
                        }
                    }
                    result
                })
            });
            let gallery_worker = tried_gallery.then(|| {
                let gallery_paths = tool_paths.clone();
                let source = source.clone();
                let tx = tx.clone();
                let video_source = video_source.clone();
                let trace = trace.clone();
                thread::spawn(move || {
                    let _trace = crate::infrastructure::operation_diagnostics::enter(&trace);
                    let started = std::time::Instant::now();
                    send_analysis_log(
                        &tx,
                        &source,
                        &video_source,
                        kind,
                        action_id,
                        "gallery-dl",
                        ToolLogStatus::Running,
                        String::new(),
                        format!("traceId={trace}; cookiesRequested={use_cookies}; stage=starting"),
                    );
                    let output =
                        gallery_backend.analyze_url_detailed(&source, &gallery_paths, use_cookies);
                    let command = output
                        .as_ref()
                        .map(|value| value.command_line.clone())
                        .unwrap_or_else(|error| error.command_line.clone().unwrap_or_default());
                    let count = output
                        .as_ref()
                        .map(|value| value.analysis.media.len())
                        .unwrap_or(0);
                    let detail = format!(
                        "traceId={trace}; elapsedMs={}; mediaCount={count}; error={}",
                        started.elapsed().as_millis(),
                        output
                            .as_ref()
                            .err()
                            .map(|error| error.message.as_str())
                            .unwrap_or("none")
                    );
                    send_analysis_log(
                        &tx,
                        &source,
                        &video_source,
                        kind,
                        action_id,
                        "gallery-dl",
                        if output.is_ok() {
                            ToolLogStatus::Success
                        } else {
                            ToolLogStatus::Failed
                        },
                        command,
                        detail,
                    );
                    let result = output
                        .map(|output| output.analysis)
                        .map_err(|error| error.message);
                    if kind == GalleryRoutePromptKind::Media {
                        if let Err(error) = &result {
                            let _ = tx.send(GalleryRouteResult {
                                source: source.clone(),
                                video_source: video_source.clone(),
                                kind,
                                outcome: GalleryRouteOutcome::AnalyzerFailed(
                                    "gallery-dl",
                                    error.clone(),
                                ),
                            });
                        }
                        if let Ok(analysis) = &result {
                            if !analysis.media.is_empty() {
                                let _ = tx.send(GalleryRouteResult {
                                    source,
                                    video_source,
                                    kind,
                                    outcome: GalleryRouteOutcome::GalleryReady(analysis.clone()),
                                });
                            }
                        }
                    }
                    result
                })
            });
            let video = video_worker.map_or_else(
                || Err("yt-dlp was not selected by the URL rules.".to_owned()),
                |worker| {
                    worker.join().unwrap_or_else(|_| {
                        Err(analysis_worker_panic(
                            &tx,
                            &source,
                            &video_source,
                            kind,
                            action_id,
                            "yt-dlp",
                        ))
                    })
                },
            );
            let video = match video {
                Ok(
                    crate::infrastructure::yt_dlp_source_analysis::DesktopSourceAnalysis::Playlist,
                ) => {
                    let gallery = gallery_worker
                        .and_then(|worker| {
                            worker
                                .join()
                                .unwrap_or_else(|_| {
                                    Err(analysis_worker_panic(
                                        &tx,
                                        &source,
                                        &video_source,
                                        kind,
                                        action_id,
                                        "gallery-dl",
                                    ))
                                })
                                .ok()
                        })
                        .filter(|analysis| !analysis.media.is_empty());
                    // Do not silently import an entire creator feed or select its
                    // first item. The UI exposes the existing explicit Batch path.
                    let _ = tx.send(GalleryRouteResult {
                        source,
                        video_source,
                        kind,
                        outcome: GalleryRouteOutcome::YtPlaylist {
                            // Media result already delivered by GalleryReady.
                            gallery: if kind == GalleryRoutePromptKind::Media {
                                None
                            } else {
                                gallery
                            },
                        },
                    });
                    return;
                }
                Ok(
                    crate::infrastructure::yt_dlp_source_analysis::DesktopSourceAnalysis::Single(
                        output,
                    ),
                ) => Ok(output.json),
                Err(error) => Err(error),
            };
            let gallery = gallery_worker.map_or_else(
                || Err("gallery-dl was not selected or is not installed.".to_owned()),
                |worker| {
                    worker.join().unwrap_or_else(|_| {
                        Err(analysis_worker_panic(
                            &tx,
                            &source,
                            &video_source,
                            kind,
                            action_id,
                            "gallery-dl",
                        ))
                    })
                },
            );
            let _ = tx.send(GalleryRouteResult {
                source,
                video_source,
                kind,
                outcome: GalleryRouteOutcome::Analyzed {
                    video,
                    gallery,
                    tried_video: matched.yt_dlp,
                    tried_gallery,
                },
            });
        });
    }

    pub(super) fn poll_standard_backend_probe(&mut self) {
        while let Ok(result) = self.gallery_route_rx.try_recv() {
            if !self.gallery_route_pending {
                continue;
            }
            let GalleryRouteResult {
                source,
                video_source,
                kind,
                outcome,
            } = result;
            match outcome {
                GalleryRouteOutcome::Log {
                    action_id,
                    tool,
                    status,
                    command,
                    detail,
                } => {
                    self.record_analysis_tool_log(action_id, tool, status, command, detail);
                    continue;
                }
                GalleryRouteOutcome::AnalyzerFailed(tool, error) => {
                    self.last_action = format!("{tool} could not analyze this URL: {error}");
                    continue;
                }
                GalleryRouteOutcome::VideoReady(video) => {
                    self.add_video_probe_result(video_source, video);
                    continue;
                }
                GalleryRouteOutcome::GalleryReady(gallery) => {
                    self.add_gallery_analysis_to_queue(source, gallery);
                    continue;
                }
                _ => {}
            }
            self.gallery_route_pending = false;
            let (video, gallery, tried_video, tried_gallery) = match outcome {
                GalleryRouteOutcome::Log { .. }
                | GalleryRouteOutcome::VideoReady(_)
                | GalleryRouteOutcome::GalleryReady(_)
                | GalleryRouteOutcome::AnalyzerFailed(_, _) => {
                    unreachable!("progress handled above")
                }
                GalleryRouteOutcome::Error(error) => {
                    self.last_action = error;
                    continue;
                }
                GalleryRouteOutcome::ChooseDownloader => {
                    let selected = if self.app_mode == AppMode::Standard
                        && kind != GalleryRoutePromptKind::Origin
                    {
                        GalleryDownloaderChoice::from_checks(
                            self.config.analysis_choice_yt_dlp,
                            self.config.analysis_choice_gallery_dl,
                        )
                    } else {
                        None
                    };
                    self.gallery_downloader_choice_prompt = Some(GalleryDownloaderChoicePrompt {
                        source,
                        kind,
                        selected,
                    });
                    self.last_action = if self.gallery_downloader_multiple_allowed() {
                        "gallery.choice.tool_required"
                    } else {
                        "gallery.choice.one_required"
                    }
                    .to_owned();
                    continue;
                }
                GalleryRouteOutcome::YtPlaylist { gallery } => {
                    // Both-tools selection still retains a successful gallery result.
                    // No gallery download is started implicitly by this scope decision.
                    if kind == GalleryRoutePromptKind::Media {
                        if let Some(analysis) = gallery {
                            self.add_gallery_analysis_to_queue(source.clone(), analysis);
                        }
                    }
                    self.last_action = "This URL is a playlist. Use Standard Mode Batch import instead of single-video analysis or Download now.".to_owned();
                    if kind == GalleryRoutePromptKind::Media {
                        self.youtube_playlist_prompt = Some(YoutubePlaylistPrompt {
                            source,
                            kind: YoutubePlaylistPromptKind::GenericPlaylist,
                            risk: None,
                            music_compact: false,
                        });
                    }
                    continue;
                }
                GalleryRouteOutcome::Analyzed {
                    video,
                    gallery,
                    tried_video,
                    tried_gallery,
                } => (video, gallery, tried_video, tried_gallery),
            };
            let gallery = gallery.and_then(|analysis| {
                if analysis.media.is_empty() {
                    Err("gallery-dl found no media for this URL.".to_owned())
                } else {
                    Ok(analysis)
                }
            });
            // Media-mode successes were already presented as each worker finished.
            // Keep final failure aggregation, but never insert duplicate cards.
            if kind == GalleryRoutePromptKind::Media && (video.is_ok() || gallery.is_ok()) {
                continue;
            }
            match (video, gallery) {
                (Ok(video), Ok(gallery)) => {
                    if kind == GalleryRoutePromptKind::Media {
                        self.add_video_probe_result(video_source, video);
                        self.add_gallery_analysis_to_queue(source, gallery);
                        continue;
                    }
                    if kind == GalleryRoutePromptKind::Immediate {
                        let video_id = self.ensure_queue_item_for_url(&video_source);
                        self.apply_analysis_json(
                            video,
                            Some(video_source.clone()),
                            Some(video_id),
                            None,
                        );
                        let gallery_id = self.add_gallery_analysis_to_queue(source, gallery);
                        self.enqueue_download_ready_item(video_id);
                        if let Some(gallery_id) = gallery_id {
                            self.enqueue_download_ready_item(gallery_id);
                        }
                        let emit_json = self
                            .queue_item_by_id(video_id)
                            .is_some_and(|item| !item.metadata_loaded());
                        let _ = self.start_download_task_at(video_id, emit_json);
                        continue;
                    }
                    self.gallery_route_prompt = Some(GalleryRoutePrompt {
                        source,
                        video_source,
                        kind,
                        video,
                        gallery,
                        include_video: true,
                        include_images: kind != GalleryRoutePromptKind::Origin,
                    });
                    self.last_action = if kind == GalleryRoutePromptKind::Origin {
                        "gallery.choice.content_required".to_owned()
                    } else {
                        "gallery.choice.content_required".to_owned()
                    };
                }
                (Ok(video), Err(_)) => match kind {
                    GalleryRoutePromptKind::Origin => {
                        self.set_origin_video_probe_result(video_source, video)
                    }
                    GalleryRoutePromptKind::Media => {
                        self.add_video_probe_result(video_source, video)
                    }
                    GalleryRoutePromptKind::Immediate => {
                        self.download_analyzed_single_video(video_source, video)
                    }
                    GalleryRoutePromptKind::VideoAndPlaylist => {
                        self.open_video_playlist_prompt(source)
                    }
                },
                (Err(_), Ok(gallery)) => {
                    if kind == GalleryRoutePromptKind::Origin {
                        self.set_origin_gallery_probe_result(source, gallery);
                    } else if kind == GalleryRoutePromptKind::Immediate {
                        if let Some(item_id) = self.add_gallery_analysis_to_queue(source, gallery) {
                            let _ = self.start_gallery_download(item_id);
                        }
                    } else {
                        self.add_gallery_analysis_to_queue(source, gallery);
                    }
                }
                (Err(video_error), Err(gallery_error)) => {
                    self.last_action = match (tried_video, tried_gallery) {
                        (true, true) => format!(
                            "Neither downloader could analyze this URL. yt-dlp: {video_error}; gallery-dl: {gallery_error}"
                        ),
                        (true, false) => {
                            format!("yt-dlp could not analyze this URL: {video_error}")
                        }
                        (false, true) => {
                            format!("gallery-dl could not analyze this URL: {gallery_error}")
                        }
                        (false, false) => "No matching URL rules.".to_owned(),
                    };
                }
            }
        }
    }

    pub fn gallery_backend_prompt_open(&self) -> bool {
        self.gallery_route_prompt.is_some()
    }

    pub fn gallery_downloader_choice_open(&self) -> bool {
        self.gallery_downloader_choice_prompt.is_some()
    }

    pub fn gallery_downloader_choice(&self) -> Option<GalleryDownloaderChoice> {
        self.gallery_downloader_choice_prompt
            .as_ref()
            .and_then(|prompt| prompt.selected)
            .filter(|choice| {
                *choice != GalleryDownloaderChoice::Both
                    || self.gallery_downloader_multiple_allowed()
            })
    }

    pub fn gallery_downloader_multiple_allowed(&self) -> bool {
        self.app_mode == AppMode::Standard
            && self
                .gallery_downloader_choice_prompt
                .as_ref()
                .is_some_and(|prompt| prompt.kind != GalleryRoutePromptKind::Origin)
    }

    pub fn gallery_downloader_choice_action_label_key(&self) -> &'static str {
        match self
            .gallery_downloader_choice_prompt
            .as_ref()
            .map(|prompt| prompt.kind)
        {
            Some(GalleryRoutePromptKind::Origin) => "action.analyze",
            Some(GalleryRoutePromptKind::Immediate) => "action.download",
            _ => "action.add",
        }
    }

    pub fn set_gallery_downloader_checks(&mut self, yt_dlp: bool, gallery_dl: bool) {
        let selected = GalleryDownloaderChoice::from_checks(yt_dlp, gallery_dl).filter(|choice| {
            *choice != GalleryDownloaderChoice::Both || self.gallery_downloader_multiple_allowed()
        });
        if let Some(prompt) = self.gallery_downloader_choice_prompt.as_mut() {
            prompt.selected = selected;
        }
    }

    pub fn confirm_gallery_downloader_choice(&mut self) {
        let Some(selected) = self.gallery_downloader_choice() else {
            return;
        };
        // Checkbox changes stay local until confirmation. Single-choice button
        // prompts retain their existing behavior and do not replace this memory.
        if self.gallery_downloader_multiple_allowed() {
            self.config.analysis_choice_yt_dlp = matches!(
                selected,
                GalleryDownloaderChoice::YtDlp | GalleryDownloaderChoice::Both
            );
            self.config.analysis_choice_gallery_dl = matches!(
                selected,
                GalleryDownloaderChoice::GalleryDl | GalleryDownloaderChoice::Both
            );
            if let Err(error) = self.config.save() {
                self.push_runtime_log(format!("Could not save analysis tool selection: {error}"));
            }
        }
        let Some(prompt) = self.gallery_downloader_choice_prompt.take() else {
            return;
        };
        self.start_standard_backend_probe_with_kind(prompt.source, prompt.kind, Some(selected));
    }

    pub fn cancel_gallery_downloader_choice(&mut self) {
        self.gallery_downloader_choice_prompt = None;
        self.last_action = "gallery.choice.cancelled".to_owned();
    }

    pub fn gallery_backend_probe_running(&self) -> bool {
        self.gallery_route_pending
    }

    pub fn gallery_route_prompt_is_origin(&self) -> bool {
        self.gallery_route_prompt
            .as_ref()
            .is_some_and(|prompt| prompt.kind == GalleryRoutePromptKind::Origin)
    }

    pub fn gallery_route_prompt_has_playlist_scope(&self) -> bool {
        self.gallery_route_prompt
            .as_ref()
            .is_some_and(|prompt| prompt.kind == GalleryRoutePromptKind::VideoAndPlaylist)
    }

    pub fn gallery_route_image_count(&self) -> usize {
        self.gallery_route_prompt
            .as_ref()
            .map(|prompt| prompt.gallery.first_post_media_count())
            .unwrap_or_default()
    }

    pub fn gallery_route_include_video(&self) -> bool {
        self.gallery_route_prompt
            .as_ref()
            .is_some_and(|prompt| prompt.include_video)
    }

    pub fn gallery_route_include_images(&self) -> bool {
        self.gallery_route_prompt
            .as_ref()
            .is_some_and(|prompt| prompt.include_images)
    }

    pub fn set_gallery_route_include_video(&mut self, include: bool) {
        if let Some(prompt) = self.gallery_route_prompt.as_mut() {
            prompt.include_video = include;
            if prompt.kind == GalleryRoutePromptKind::Origin {
                prompt.include_images = !include;
            }
        }
    }

    pub fn set_gallery_route_include_images(&mut self, include: bool) {
        if let Some(prompt) = self.gallery_route_prompt.as_mut() {
            prompt.include_images = include;
            if prompt.kind == GalleryRoutePromptKind::Origin {
                prompt.include_video = !include;
            }
        }
    }

    pub fn confirm_gallery_route_media(&mut self) {
        let Some(prompt) = self.gallery_route_prompt.take() else {
            return;
        };
        if prompt.kind == GalleryRoutePromptKind::Origin {
            if prompt.include_video {
                self.set_origin_video_probe_result(prompt.video_source, prompt.video);
            } else {
                self.set_origin_gallery_probe_result(prompt.source, prompt.gallery);
            }
            return;
        }
        if !prompt.include_video && !prompt.include_images {
            self.gallery_route_prompt = Some(prompt);
            self.last_action = "gallery.choice.content_required".to_owned();
            return;
        }
        if prompt.include_video {
            self.add_video_probe_result(prompt.video_source, prompt.video);
        }
        if prompt.include_images {
            self.add_gallery_analysis_to_queue(prompt.source, prompt.gallery);
        }
    }

    pub fn confirm_gallery_route_as_video(&mut self) {
        let Some(prompt) = self.gallery_route_prompt.take() else {
            return;
        };
        self.add_video_probe_result(prompt.video_source, prompt.video);
        if prompt.include_images {
            self.add_gallery_analysis_to_queue(prompt.source, prompt.gallery);
        }
    }

    pub fn confirm_gallery_route_as_playlist(&mut self) {
        let Some(prompt) = self.gallery_route_prompt.take() else {
            return;
        };
        if prompt.include_images {
            self.add_gallery_analysis_to_queue(prompt.source.clone(), prompt.gallery);
        }
        self.begin_batch_add(prompt.source);
    }

    pub fn cancel_gallery_route_prompt(&mut self) {
        self.gallery_route_prompt = None;
        self.last_action = "gallery.choice.cancelled".to_owned();
    }

    fn continue_without_gallery_probe(&mut self, source: String, kind: GalleryRoutePromptKind) {
        match kind {
            GalleryRoutePromptKind::Media => self.add_single_url_to_batch(source),
            GalleryRoutePromptKind::Origin => self.start_origin_backend_probe(source),
            GalleryRoutePromptKind::VideoAndPlaylist => self.open_video_playlist_prompt(source),
            GalleryRoutePromptKind::Immediate => self.start_immediate_backend_probe(source),
        }
    }

    fn set_origin_video_probe_result(&mut self, source: String, video: Value) {
        let (playlist_index, video) = if video.get("entries").and_then(Value::as_array).is_some() {
            let Some(mut entries) = analyzed_playlist_entries(&video) else {
                self.last_action =
                    "Origin Mode could not identify a playable playlist item.".to_owned();
                return;
            };
            if entries.len() != 1 {
                self.last_action =
                    "Origin Mode supports only one video, not a playlist.".to_owned();
                return;
            }
            let (index, entry) = entries.remove(0);
            (Some(index), entry)
        } else {
            (None, video)
        };
        if !self.active_workflows.is_empty() {
            self.last_action = "Wait for the current Origin Mode item to finish first.".to_owned();
            return;
        }
        self.stop_music_playback();
        self.format_picker.open = false;
        self.queue_items.clear();
        self.batch_input.clear();
        let mut item = self.build_queue_item_from_url(&source);
        item.playlist_index = playlist_index;
        let id = item.id;
        self.queue_items.push(item);
        self.apply_analysis_json(video, Some(source), Some(id), None);
    }

    fn set_origin_gallery_probe_result(&mut self, source: String, gallery: GalleryAnalysis) {
        if !self.active_workflows.is_empty() {
            self.last_action = "Wait for the current Origin Mode item to finish first.".to_owned();
            return;
        }
        self.stop_music_playback();
        self.format_picker.open = false;
        self.queue_items.clear();
        self.batch_input.clear();
        self.add_gallery_analysis_to_queue(source, gallery);
    }

    fn add_video_probe_result(&mut self, source: String, video: Value) {
        if video.get("entries").and_then(Value::as_array).is_some() {
            let Some(entries) = analyzed_playlist_entries(&video) else {
                // Unresolved or flat playlists still use the guarded importer.
                self.begin_batch_add(source);
                return;
            };
            let source_key = canonical_queue_source_key(&source);
            let mut added = 0;
            for (playlist_index, entry) in entries {
                if self.queue_items.iter().any(|item| {
                    matches!(&item.backend, QueueBackend::YtDlp)
                        && canonical_queue_source_key(&item.source_url) == source_key
                        && item.playlist_index == Some(playlist_index)
                }) {
                    continue;
                }
                let mut item = self.build_queue_item_from_url(&source);
                item.playlist_index = Some(playlist_index);
                let item_id = item.id;
                self.queue_items.push(item);
                self.apply_analysis_json(entry, Some(source.clone()), Some(item_id), None);
                added += 1;
            }
            if added > 0 {
                self.batch_input_push_unique(&source);
                self.url_input.clear();
                self.last_action = format!("Added {added} analyzed yt-dlp video items.");
            } else {
                self.last_action = "These yt-dlp video items are already in the queue.".to_owned();
            }
            return;
        }
        let source_key = canonical_queue_source_key(&source);
        if self.queue_items.iter().any(|item| {
            matches!(&item.backend, QueueBackend::YtDlp)
                && canonical_queue_source_key(&item.source_url) == source_key
                && item.playlist_index.is_none()
        }) {
            self.last_action = "This video URL is already in the queue.".to_owned();
            return;
        }
        let item = self.build_queue_item_from_url(&source);
        let item_id = item.id;
        self.queue_items.push(item);
        self.batch_input_push_unique(&source);
        // Reuse the already returned yt-dlp JSON: do not run a second long analysis.
        self.apply_analysis_json(video, Some(source.clone()), Some(item_id), None);
        self.url_input.clear();
    }

    fn add_gallery_analysis_to_queue(
        &mut self,
        source: String,
        mut analysis: GalleryAnalysis,
    ) -> Option<QueueItemId> {
        let source_key = canonical_queue_source_key(&source);
        if self.queue_items.iter().any(|item| {
            matches!(&item.backend, QueueBackend::GalleryDl(_))
                && canonical_queue_source_key(&item.source_url) == source_key
        }) {
            self.last_action = "This image URL is already in the queue.".to_owned();
            return None;
        }
        let count = analysis.first_post_media_count();
        let roots: Vec<_> = analysis
            .posts
            .first()
            .map(|post| &post.source_metadata)
            .into_iter()
            .chain(analysis.media.first().map(|media| &media.source_metadata))
            .collect();
        analysis.persisted_metadata = Some(
            crate::app::gallery_canonical_metadata::map_gallery_canonical_metadata_from_roots(
                &roots,
            ),
        );
        let ui_metadata = map_gallery_ui_metadata(&analysis);
        let mut item = self.build_queue_item_from_url(&source);
        let item_id = item.id;
        item.backend = QueueBackend::GalleryDl(GalleryQueueItem {
            download_range: Default::default(),
            processed_posts: 0,
            analysis: Some(analysis),
            download_artifacts: Vec::new(),
        });
        item.title = ui_metadata.title;
        item.thumbnail_url = ui_metadata.thumbnail;
        item.metadata_state = MetadataState::Idle;
        self.queue_items.push(item);
        if self.app_mode != AppMode::Origin {
            self.url_input.clear();
        }
        self.mark_font_content_changed();
        self.last_action = format!("gallery-dl: {count} images added to the queue.");
        Some(item_id)
    }

    pub fn can_start_gallery_item_download(&self, index: usize) -> bool {
        !self.has_running_download_workflow()
            && !self.item_is_busy(index)
            && self.queue_items.get(index).is_some_and(|item| {
                matches!(&item.backend, QueueBackend::GalleryDl(_))
                    && item_can_enter_download_queue(item)
            })
    }

    pub fn start_gallery_download(&mut self, item_id: QueueItemId) -> Result<(), String> {
        if self.refresh_restored_queue_item(
            item_id,
            super::queue_reanalysis_actions::QueueRefreshAction::Download,
        ) {
            return Ok(());
        }
        if self.has_running_download_workflow() {
            return Err("A download is already running.".to_owned());
        }
        let Some(item) = self.queue_item_by_id(item_id) else {
            return Err("Image item not found.".to_owned());
        };
        let QueueBackend::GalleryDl(gallery) = &item.backend else {
            return Err("Not a gallery-dl item.".to_owned());
        };
        if gallery.analysis.is_none() {
            return Err("Wait for gallery-dl image analysis to finish.".to_owned());
        }
        let source = item.source_url.clone();
        let timeline = gallery.analysis.as_ref().is_some_and(|analysis| {
            crate::domain::gallery_source_scope(&analysis.root_extractor)
                == crate::domain::GallerySourceScope::Timeline
        });
        let mut range = gallery.download_range.clone();
        let single_feed = gallery
            .analysis
            .as_ref()
            .is_some_and(|analysis| analysis.supports_single_feed_post_range());
        range.freeze(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
        let use_cookies = item.selection.use_cookies;
        let destination = resolve_output_dir(&item.selection.output_dir);
        let prepared = destination.and_then(|destination| {
            GalleryDlBackend::from_tool_paths(&self.tool_paths).prepare_item_download(
                &source,
                &destination,
                &self.tool_paths,
                use_cookies,
            )
        });
        let prepared = prepared.and_then(|mut prepared| {
            if timeline {
                if matches!(range, crate::domain::GalleryDownloadRange::Posts { .. })
                    && !single_feed
                {
                    return Err(
                        "This source delegates multiple feeds; choose a time range or all instead of a total post limit."
                            .to_owned(),
                    );
                }
                crate::infrastructure::gallery_dl::apply_timeline_download_range(
                    &mut prepared,
                    &range,
                )?;
            }
            Ok(prepared)
        });
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                self.mark_download_preflight_failed(item_id, &error);
                self.last_action = error.clone();
                return Err(error);
            }
        };
        let workflow_id = match self.queue_item_by_id(item_id).and_then(|item| {
            item.workflows
                .iter()
                .rev()
                .find(|run| {
                    run.kind == WorkflowKind::DownloadMedia && run.state == WorkflowState::Queued
                })
                .map(|run| run.id)
        }) {
            Some(id) => id,
            None => self.alloc_workflow_run_id(),
        };
        self.register_active_workflow(
            item_id,
            workflow_id,
            WorkflowKind::DownloadMedia,
            ToolKind::Other("gallery-dl".to_owned()),
        );
        if let Some(item) = self.queue_item_mut_by_id(item_id) {
            item.last_error = None;
            item.last_output_path = None;
            item.completed_selection = None;
            if let QueueBackend::GalleryDl(gallery) = &mut item.backend {
                gallery.download_range = range;
                gallery.download_artifacts.clear();
                gallery.processed_posts = 0;
                if let Some(analysis) = gallery.analysis.as_mut() {
                    for media in &mut analysis.media {
                        media.archive_key.clear();
                        media.output_path = None;
                    }
                }
            }
            if let Some(run) = item.workflows.iter_mut().find(|run| run.id == workflow_id) {
                run.state = WorkflowState::Running;
                run.progress = 0.0;
                run.detail = prepared.destination.display().to_string();
            } else {
                let mut run = WorkflowRun::new(
                    workflow_id,
                    WorkflowKind::DownloadMedia,
                    ToolKind::Other("gallery-dl".to_owned()),
                    WorkflowState::Running,
                );
                run.detail = prepared.destination.display().to_string();
                item.workflows.push(run);
            }
        }
        let child_handle = Arc::new(Mutex::new(None));
        let cancel_requested = Arc::new(AtomicBool::new(false));
        self.attach_active_download_process(
            workflow_id,
            child_handle.clone(),
            cancel_requested.clone(),
        );
        self.last_action = format!("gallery-dl downloading: {source}");
        let tx = self.download_result_tx.clone();
        thread::spawn(move || {
            run_gallery_download(
                prepared,
                item_id,
                workflow_id,
                tx,
                child_handle,
                cancel_requested,
            );
        });
        Ok(())
    }
}

enum GalleryDownloadWork {
    Artifact(GalleryDlMetadataEvent),
    PostCompleted,
}

fn run_gallery_download(
    prepared: crate::infrastructure::gallery_dl::PreparedGalleryDlDownload,
    item_id: QueueItemId,
    workflow_id: WorkflowRunId,
    tx: Sender<DownloadEvent>,
    child_handle: Arc<Mutex<Option<Child>>>,
    cancel_requested: Arc<AtomicBool>,
) {
    let trace = crate::infrastructure::operation_diagnostics::new_trace();
    let _scope = crate::infrastructure::operation_diagnostics::enter(&trace);
    let mut diagnostic = crate::infrastructure::operation_diagnostics::OperationSpan::start(
        "desktop.gallery.download",
        serde_json::json!({"itemId": item_id, "workflowId": workflow_id, "command": crate::infrastructure::operation_diagnostics::command_context(&prepared.command)}),
    );
    let mut command = prepared.command;
    let _runtime_guard = prepared.runtime_guard;
    let _storage_work = prepared.storage_work;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let result = match command.spawn() {
        Err(error) => Err(format!("Could not start gallery-dl: {error}")),
        Ok(mut child) => {
            let _tracked = track_child_process(&child, "gallery-dl download");
            // Keep gallery-dl stdout draining independently from XMP writes.
            // The queue is intentionally bounded: metadata JSON is compacted
            // before enqueueing, while sustained post-processing pressure is
            // allowed to apply backpressure instead of growing memory forever.
            let (metadata_tx, metadata_rx) = mpsc::sync_channel::<GalleryDownloadWork>(16);
            let artifact_tx = tx.clone();
            let metadata_cancel = cancel_requested.clone();
            let metadata_trace = trace.clone();
            let metadata_worker = thread::spawn(move || {
                let _scope = crate::infrastructure::operation_diagnostics::enter(&metadata_trace);
                while let Ok(work) = metadata_rx.recv() {
                    if metadata_cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let GalleryDownloadWork::Artifact(mut event) = work else {
                        // FIFO ordering means every earlier image's XMP has finished.
                        let _ = artifact_tx.send(DownloadEvent::GalleryPostCompleted {
                            item_id,
                            workflow_id,
                        });
                        continue;
                    };
                    let fields =
                        map_gallery_artifact_metadata(&event.metadata, &event.artifact.path);
                    event.artifact.date = fields.created_at.clone();
                    if let Err(error) = write_gallery_xmp(
                        &event.artifact.path,
                        &event.artifact.archive_key,
                        &fields,
                    ) {
                        eprintln!(
                            "[gallery-dl] XMP post-processing warning for {}: {error}",
                            event.artifact.path.display()
                        );
                    }
                    let _ = artifact_tx.send(DownloadEvent::GalleryArtifact {
                        item_id,
                        workflow_id,
                        artifact: event.artifact,
                    });
                }
            });
            let stdout_reader = child.stdout.take().map(|stdout| {
                thread::spawn(move || -> Result<(), String> {
                    let mut first_error = None;
                    let mut reader = BufReader::new(stdout);
                    let mut raw_line = Vec::new();
                    loop {
                        raw_line.clear();
                        let read = reader.read_until(b'\n', &mut raw_line).map_err(|error| {
                            format!("Could not read gallery-dl download output: {error}")
                        })?;
                        if read == 0 {
                            break;
                        }
                        // gallery-dl is normally UTF-8 here, but a single invalid
                        // byte from a downloader/path must never terminate the reader:
                        // closing the Windows pipe early makes Python surface EINVAL
                        // (Errno 22) / exit code 120 while flushing stdout.
                        let line = String::from_utf8_lossy(&raw_line);
                        if line.trim()
                            == crate::infrastructure::gallery_dl::GALLERY_POST_COMPLETED_MARKER
                        {
                            let _ = metadata_tx.send(GalleryDownloadWork::PostCompleted);
                            continue;
                        }
                        let Some(event) = parse_gallery_dl_download_event(&line) else {
                            continue;
                        };
                        match event {
                            Ok(event) => {
                                if metadata_tx
                                    .send(GalleryDownloadWork::Artifact(event))
                                    .is_err()
                                    && first_error.is_none()
                                {
                                    first_error = Some(
                                        "gallery-dl metadata worker stopped unexpectedly."
                                            .to_owned(),
                                    );
                                }
                            }
                            Err(error) => {
                                if first_error.is_none() {
                                    first_error = Some(error);
                                }
                            }
                        }
                    }
                    first_error.map_or(Ok(()), Err)
                })
            });
            // Drain the pipe throughout the download: reading it only after
            // process exit could deadlock when gallery-dl emits many messages.
            // Retain a bounded tail for an actionable error when it fails.
            let stderr_reader = child.stderr.take().map(|mut stderr| {
                thread::spawn(move || {
                    const MAX_ERROR_BYTES: usize = 16 * 1024;
                    let mut tail = Vec::new();
                    let mut buffer = [0_u8; 4096];
                    loop {
                        match stderr.read(&mut buffer) {
                            Ok(0) | Err(_) => break,
                            Ok(read) => {
                                tail.extend_from_slice(&buffer[..read]);
                                if tail.len() > MAX_ERROR_BYTES {
                                    let overflow = tail.len() - MAX_ERROR_BYTES;
                                    tail.drain(..overflow);
                                }
                            }
                        }
                    }
                    String::from_utf8_lossy(&tail).trim().to_owned()
                })
            });
            if let Ok(mut guard) = child_handle.lock() {
                *guard = Some(child);
            }
            let status =
                wait_download_child(&child_handle, &cancel_requested).unwrap_or_else(|| {
                    Err(std::io::Error::other(
                        "gallery-dl process handle unavailable",
                    ))
                });
            let _ = tx.send(DownloadEvent::GalleryProcessExited {
                item_id,
                workflow_id,
            });
            let stderr_detail = stderr_reader
                .and_then(|reader| reader.join().ok())
                .unwrap_or_default();
            let stdout_result = stdout_reader
                .map(|reader| {
                    reader.join().unwrap_or_else(|_| {
                        Err("gallery-dl output reader stopped unexpectedly.".to_owned())
                    })
                })
                .unwrap_or(Ok(()));
            // Normal completion drains XMP. Cancellation drops pending work
            // after the current file closes safely, before Finished is emitted.
            let metadata_worker_finished = metadata_worker.join().is_ok();
            let stdout_result = if stdout_result.is_ok() && !metadata_worker_finished {
                Err("gallery-dl metadata worker stopped unexpectedly.".to_owned())
            } else {
                stdout_result
            };
            if cancel_requested.load(Ordering::Relaxed) {
                Err(DOWNLOAD_CANCELLED_MESSAGE.to_owned())
            } else {
                crate::infrastructure::operation_diagnostics::record(
                    "tool.exit",
                    serde_json::json!({"exitCode": status.as_ref().ok().and_then(|status| status.code()), "stderrBytes": stderr_detail.len()}),
                );
                match status {
                    Ok(status) if status.success() && stdout_result.is_err() => {
                        Err(stdout_result.expect_err("checked above"))
                    }
                    Ok(status) if status.success() => {
                        Ok(prepared.destination.display().to_string())
                    }
                    Ok(status) if stderr_detail.is_empty() => {
                        Err(format!("gallery-dl failed (exit status: {status})."))
                    }
                    Ok(status) => Err(format!(
                        "gallery-dl failed (exit status: {status}): {stderr_detail}"
                    )),
                    Err(error) => Err(format!("gallery-dl process failed: {error}")),
                }
            }
        }
    };
    let success = result.is_ok();
    diagnostic.finish(
        if cancel_requested.load(Ordering::Acquire) {
            "cancelled"
        } else if success {
            "success"
        } else {
            "failed"
        },
        serde_json::json!({}),
    );
    let _ = tx.send(DownloadEvent::ToolCommandFinished {
        item_id,
        workflow_id,
        target_kind: DownloadTargetKind::Normal,
        command_line: prepared.command_line,
        success,
        detail: result.as_ref().err().cloned(),
    });
    let _ = tx.send(DownloadEvent::Finished(DownloadResult {
        disposition: crate::infrastructure::output_policy::OutputDisposition::Published,
        processed_before_publication: false,
        item_id,
        workflow_id,
        workflow_kind: WorkflowKind::DownloadMedia,
        target_kind: DownloadTargetKind::Normal,
        result,
    }));
}
