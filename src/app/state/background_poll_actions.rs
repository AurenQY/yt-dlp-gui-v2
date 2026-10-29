use super::*;

impl AppState {
    pub fn poll_background_work(&mut self) {
        self.poll_gallery_readiness();
        self.poll_media_session_commands();
        self.poll_youtube_login_rescue();
        self.poll_standard_backend_probe();
        self.poll_queue_reanalysis();
        self.poll_audio_url_preflight();

        loop {
            match self.analyze_result_rx.try_recv() {
                Ok(message) => {
                    let analyze_succeeded = message.result.is_ok();
                    let analyze_error_detail = message.result.as_ref().err().cloned();
                    if let (Some(action_id), Some(command_line)) =
                        (message.tool_log_action_id, message.command_line.as_ref())
                    {
                        self.push_tool_log_step_internal(
                            action_id,
                            if analyze_succeeded {
                                ToolLogStatus::Success
                            } else {
                                ToolLogStatus::Failed
                            },
                            "yt-dlp",
                            "analyze",
                            command_line.clone(),
                            analyze_error_detail,
                            true,
                        );
                    }

                    match message.result {
                        Ok(json) => {
                            if let Some(item_id) = message.target_item_id {
                                if let Some(item) = self.queue_item_mut_by_id(item_id) {
                                    item.cookie_policy = if message.used_cookies {
                                        CookiePolicy::Required
                                    } else {
                                        CookiePolicy::NotNeeded
                                    };
                                }
                            }
                            self.apply_analysis_json(
                                json,
                                Some(message.source),
                                message.target_item_id,
                                message.workflow_id,
                            );
                        }
                        Err(error) => {
                            let should_retry_with_cookies = message
                                .target_item_id
                                .and_then(|item_id| self.queue_item_by_id(item_id))
                                .is_some_and(|item| item.selection.use_cookies)
                                && !message.used_cookies
                                && message
                                    .target_item_id
                                    .and_then(|item_id| self.queue_item_by_id(item_id))
                                    .is_some_and(|item| {
                                        item.cookie_policy == CookiePolicy::Unknown
                                    })
                                && should_retry_analyze_with_cookies(&error);

                            if should_retry_with_cookies {
                                if let Some(item_id) = message.target_item_id {
                                    if let Some(item) = self.queue_item_mut_by_id(item_id) {
                                        item.cookie_policy = CookiePolicy::Required;
                                    }
                                }
                                self.last_action = i18n::format_fixed_english(
                                    "Retrying analysis with cookies: {source}",
                                    &[("{source}", message.source.as_str())],
                                );
                                self.spawn_analyze_worker(
                                    message.source,
                                    message.target_item_id,
                                    message.workflow_id,
                                    true,
                                );
                                continue;
                            }
                            eprintln!("[analyze] {error}");
                            if let Some(item_id) = message.target_item_id {
                                if let Some(item) = self.queue_item_mut_by_id(item_id) {
                                    item.metadata_state = MetadataState::Failed(error.clone());
                                    item.last_error = Some(error.clone());
                                    if let Some(workflow_id) = message.workflow_id {
                                        if let Some(run) = item
                                            .workflows
                                            .iter_mut()
                                            .find(|run| run.id == workflow_id)
                                        {
                                            run.state = WorkflowState::Failed;
                                            run.error = Some(error.clone());
                                        }
                                        self.unregister_active_workflow(workflow_id);
                                    }
                                }
                            }
                            self.last_action = error;
                        }
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        loop {
            match self.music_stream_result_rx.try_recv() {
                Ok(message) => self.apply_music_stream_result(message),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        loop {
            match self.music_playback_event_rx.try_recv() {
                Ok(event) => self.apply_music_playback_event(event),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        for _ in 0..BACKGROUND_MUSIC_DOWNLOAD_EVENT_BUDGET_PER_POLL {
            match self.music_download_event_rx.try_recv() {
                Ok(event) => self.apply_music_download_event(event),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        self.maybe_prefetch_next_music_item();
        self.poll_music_chorus_flow();

        if let Some(rx) = self.batch_add_result_rx.take() {
            let mut keep_rx = true;
            loop {
                match rx.try_recv() {
                    Ok(BatchAddEvent::ToolCommandFinished {
                        action_id,
                        command_line,
                        success,
                        partial_success,
                        detail,
                    }) => {
                        self.push_tool_log_step_internal(
                            action_id,
                            if partial_success {
                                ToolLogStatus::Recovered
                            } else {
                                self.tool_log_status_for_batch_step(success)
                            },
                            "yt-dlp",
                            "batch import",
                            command_line,
                            detail,
                            true,
                        );
                    }
                    Ok(BatchAddEvent::ItemAdded { source, seed }) => {
                        let cancel_requested = self
                            .batch_add_cancel_requested
                            .as_ref()
                            .is_some_and(|flag| flag.load(Ordering::Relaxed));
                        if !cancel_requested {
                            if self.batch_add_music_compact {
                                self.append_music_compact_seed(seed);
                            } else {
                                self.append_batch_seed(&source, seed);
                            }
                        }
                    }
                    Ok(BatchAddEvent::Finished {
                        source,
                        added,
                        stopped_by_limit,
                        partial_success,
                    }) => {
                        self.is_adding_batch = false;
                        self.is_cancelling_batch_add = false;
                        self.batch_add_child = None;
                        self.batch_add_cancel_requested = None;
                        self.batch_add_music_compact = false;
                        keep_rx = false;
                        if added == 0 {
                            self.last_action = "No new items were found in the batch.".to_owned();
                        } else if partial_success {
                            self.last_action =
                                format!("Added {added} batch items; some yt-dlp entries failed.");
                        } else if stopped_by_limit {
                            self.last_action = i18n::format_fixed_english(
                                "Added {count} batch items from the playlist (limit applied).",
                                &[("{count}", &added.to_string())],
                            );
                        } else if added == 1 {
                            let fallback_title =
                                infer_title(&source, "Untitled task", "Imported {tail}");
                            self.last_action = i18n::format_fixed_english(
                                "Added to batch: {title}",
                                &[("{title}", fallback_title.as_str())],
                            );
                        } else {
                            self.last_action = i18n::format_fixed_english(
                                "Added {count} batch items from the playlist.",
                                &[("{count}", &added.to_string())],
                            );
                        }
                        self.url_input.clear();
                        break;
                    }
                    Ok(BatchAddEvent::Failed { error }) => {
                        self.is_adding_batch = false;
                        self.is_cancelling_batch_add = false;
                        self.batch_add_child = None;
                        self.batch_add_cancel_requested = None;
                        self.batch_add_music_compact = false;
                        keep_rx = false;
                        self.last_action = error;
                        break;
                    }
                    Ok(BatchAddEvent::Cancelled { added }) => {
                        self.is_adding_batch = false;
                        self.is_cancelling_batch_add = false;
                        self.batch_add_child = None;
                        self.batch_add_cancel_requested = None;
                        self.batch_add_music_compact = false;
                        keep_rx = false;
                        self.last_action = if added == 0 {
                            "Batch add cancelled.".to_owned()
                        } else {
                            i18n::format_fixed_english(
                                "Batch add cancelled; {count} items were added.",
                                &[("{count}", &added.to_string())],
                            )
                        };
                        self.url_input.clear();
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        self.is_adding_batch = false;
                        self.is_cancelling_batch_add = false;
                        self.batch_add_child = None;
                        self.batch_add_cancel_requested = None;
                        self.batch_add_music_compact = false;
                        keep_rx = false;
                        self.last_action = "Batch add was interrupted.".to_owned();
                        break;
                    }
                }
            }
            if keep_rx {
                self.batch_add_result_rx = Some(rx);
            }
        }

        loop {
            match self.component_update_result_rx.try_recv() {
                Ok(ComponentUpdateEvent::Snapshot(snapshot)) => {
                    self.component_update_snapshot = snapshot;
                    self.last_action = self.component_update_snapshot.message.clone();
                }
                Ok(ComponentUpdateEvent::Finished(snapshot)) => {
                    self.component_update_snapshot = snapshot;
                    self.last_action = self.component_update_snapshot.message.clone();
                    self.request_gallery_readiness(true);
                    self.sync_available_managed_tool_paths_from_update_snapshot();
                    self.refresh_prepare_report();
                    if !self.should_show_prepare_tab() && self.active_tab == AppTab::Prepare {
                        self.active_tab = AppTab::Main;
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        for _ in 0..BACKGROUND_DOWNLOAD_EVENT_BUDGET_PER_POLL {
            match self.download_result_rx.try_recv() {
                Ok(DownloadEvent::OutputStatus {
                    item_id,
                    workflow_id,
                    detail,
                }) => {
                    if let Some(item) = self.queue_item_mut_by_id(item_id) {
                        if let Some(run) =
                            item.workflows.iter_mut().find(|run| run.id == workflow_id)
                        {
                            run.detail = detail;
                        }
                    }
                }
                Ok(DownloadEvent::Metadata { item_id, json }) => {
                    self.apply_analysis_json(json, None, Some(item_id), None);
                }
                Ok(DownloadEvent::GalleryPostCompleted {
                    item_id,
                    workflow_id,
                }) => {
                    if !self.is_current_download_progress_event(item_id, workflow_id) {
                        continue;
                    }
                    if let Some(item) = self.queue_item_mut_by_id(item_id) {
                        if let QueueBackend::GalleryDl(gallery) = &mut item.backend {
                            gallery.processed_posts += 1;
                            if gallery.analysis.as_ref().is_some_and(|analysis| {
                                crate::domain::gallery_source_scope(&analysis.root_extractor)
                                    == crate::domain::GallerySourceScope::Timeline
                            }) && let crate::domain::GalleryDownloadRange::Posts { count } =
                                gallery.download_range
                            {
                                if let Some(run) =
                                    item.workflows.iter_mut().find(|run| run.id == workflow_id)
                                {
                                    if count > 0 {
                                        run.progress = monotonic_progress(
                                            run.progress,
                                            (gallery.processed_posts as f32 / count as f32 * 100.0)
                                                .min(99.0),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(DownloadEvent::GalleryProcessExited {
                    item_id,
                    workflow_id,
                }) => {
                    if !self.is_current_download_progress_event(item_id, workflow_id) {
                        continue;
                    }
                    if let Some(item) = self.queue_item_mut_by_id(item_id) {
                        if let Some(run) =
                            item.workflows.iter_mut().find(|run| run.id == workflow_id)
                        {
                            run.detail = "gallery-finishing".to_owned();
                        }
                    }
                }
                Ok(DownloadEvent::GalleryArtifact {
                    item_id,
                    workflow_id,
                    artifact,
                }) => {
                    if !self.is_current_download_progress_event(item_id, workflow_id) {
                        continue;
                    }
                    if let Some(item) = self.queue_item_mut_by_id(item_id) {
                        let QueueBackend::GalleryDl(gallery) = &mut item.backend else {
                            continue;
                        };
                        if gallery
                            .download_artifacts
                            .iter()
                            .any(|known| known.archive_key == artifact.archive_key)
                        {
                            continue;
                        }
                        if let Some(analysis) = gallery.analysis.as_mut() {
                            let matching_index = analysis
                                .media
                                .iter()
                                .position(|media| {
                                    !artifact.media_url.is_empty()
                                        && media.url == artifact.media_url
                                })
                                .or_else(|| {
                                    analysis.media.iter().position(|media| {
                                        media.sequence == artifact.sequence
                                            && media.filename == artifact.filename
                                    })
                                });
                            if let Some(media) =
                                matching_index.and_then(|index| analysis.media.get_mut(index))
                            {
                                media.archive_key = artifact.archive_key.clone();
                                media.output_path = Some(artifact.path.clone());
                            }
                        }
                        gallery.download_artifacts.push(artifact);
                        let completed = gallery.download_artifacts.len();
                        let exact_total = gallery.analysis.as_ref().and_then(|analysis| {
                            (crate::domain::gallery_source_scope(&analysis.root_extractor)
                                == crate::domain::GallerySourceScope::SinglePost)
                                .then_some(analysis.media.len())
                        });
                        if let Some(run) =
                            item.workflows.iter_mut().find(|run| run.id == workflow_id)
                        {
                            if let Some(total) = exact_total.filter(|total| *total > 0) {
                                let percent = (completed as f32 / total as f32 * 100.0).min(99.0);
                                run.progress = monotonic_progress(run.progress, percent);
                                if run.detail != "gallery-finishing" {
                                    run.detail = format!("{completed} / {total}");
                                }
                            } else {
                                // Full/source analysis intentionally probes only the first two
                                // posts, so its media count is not a real total. Report only
                                // work v2 has actually completed and reserve 100% for Finished.
                                if run.detail != "gallery-finishing" {
                                    run.detail = format!("{completed} / ?");
                                }
                            }
                        }
                    }
                }
                Ok(DownloadEvent::ToolCommandFinished {
                    item_id,
                    workflow_id,
                    target_kind,
                    command_line,
                    success,
                    detail,
                }) => {
                    let tool_name = if self
                        .queue_item_by_id(item_id)
                        .is_some_and(|item| matches!(&item.backend, QueueBackend::GalleryDl(_)))
                    {
                        "gallery-dl"
                    } else {
                        "yt-dlp"
                    };
                    let action_id = self.workflow_tool_log_action(
                        workflow_id,
                        "origin",
                        download_target_log_action(target_kind),
                    );
                    self.push_tool_log_step_with_detail_without_failure_reveal(
                        action_id,
                        self.tool_log_status_for_workflow_step(workflow_id, success),
                        tool_name,
                        "download",
                        command_line,
                        detail,
                    );
                }
                Ok(DownloadEvent::RecoveryStep {
                    item_id: _,
                    workflow_id,
                    target_kind,
                    action,
                    detail,
                    recover_previous_failure,
                    resolved_success,
                }) => {
                    let action_id = self.workflow_tool_log_action(
                        workflow_id,
                        "origin",
                        download_target_log_action(target_kind),
                    );
                    if recover_previous_failure {
                        self.mark_last_failed_tool_log_step_as_recoverable(action_id);
                    }
                    let status = if resolved_success {
                        ToolLogStatus::Success
                    } else {
                        ToolLogStatus::Skipped
                    };
                    self.push_tool_log_step(action_id, status, "v2", action, detail);
                }
                Ok(DownloadEvent::Progress {
                    item_id,
                    workflow_id,
                    slot,
                    percent,
                    detail,
                }) => {
                    if !self.is_current_download_progress_event(item_id, workflow_id) {
                        continue;
                    }
                    let language = self.language();
                    if let Some(item) = self.queue_item_mut_by_id(item_id) {
                        let display_percent = percent.clamp(0.0, 100.0);
                        if let Some(run) =
                            item.workflows.iter_mut().find(|run| run.id == workflow_id)
                        {
                            run.progress = monotonic_progress(run.progress, display_percent);
                            if let Some(detail) = detail.as_ref() {
                                run.detail = format_download_progress_detail(language, detail);
                            }
                        }
                        match slot {
                            DownloadProgressSlot::Video => {
                                item.progress.video =
                                    monotonic_progress(item.progress.video, display_percent);
                            }
                            DownloadProgressSlot::Audio => {
                                item.progress.audio =
                                    monotonic_progress(item.progress.audio, display_percent);
                            }
                            DownloadProgressSlot::Subtitle => {
                                item.progress.subtitle =
                                    monotonic_progress(item.progress.subtitle, display_percent);
                            }
                            DownloadProgressSlot::Both => {
                                item.progress.video =
                                    monotonic_progress(item.progress.video, display_percent);
                                item.progress.audio =
                                    monotonic_progress(item.progress.audio, display_percent);
                            }
                        }
                    }
                }
                Ok(DownloadEvent::Finished(mut message)) => {
                    // Honor a stop/remove click racing with the worker's final
                    // message, before starting any follow-up post-processing.
                    if self.active_workflow_cancel_requested(message.workflow_id)
                        && !message.processed_before_publication
                        && message.result.is_ok()
                    {
                        message.result = Err(DOWNLOAD_CANCELLED_MESSAGE.to_owned());
                    }
                    let finished_item_id = message.item_id;
                    let notification_title = self
                        .queue_item_by_id(message.item_id)
                        .map(|item| item.title.trim().to_owned())
                        .filter(|title| !title.is_empty())
                        .unwrap_or_else(|| "Download item".to_owned());
                    let notification_result = message.result.clone();
                    let should_send_windows_toast =
                        message.workflow_kind == WorkflowKind::DownloadMedia;
                    self.unregister_active_workflow(message.workflow_id);
                    self.finish_workflow_tool_log(message.workflow_id);
                    let mut pending_post_process_input = None;
                    if let Some(item) = self.queue_item_mut_by_id(message.item_id) {
                        if let Some(run) = item
                            .workflows
                            .iter_mut()
                            .find(|run| run.id == message.workflow_id)
                        {
                            match &message.result {
                                Ok(output_path) => {
                                    run.state = WorkflowState::Finished;
                                    run.progress = 100.0;
                                    run.output_path = Some(output_path.clone());
                                    if let QueueBackend::GalleryDl(gallery) = &mut item.backend {
                                        if let crate::domain::GalleryDownloadRange::Recent {
                                            since,
                                            until,
                                            ..
                                        } = &mut gallery.download_range
                                        {
                                            // Successful recurring runs start a new time window next time.
                                            // Cancelled runs retain both boundaries for a safe rescan.
                                            *since = None;
                                            *until = None;
                                        }
                                    }
                                    match message.target_kind {
                                        DownloadTargetKind::Normal
                                            if matches!(
                                                &item.backend,
                                                QueueBackend::GalleryDl(_)
                                            ) =>
                                        {
                                            // Gallery downloads do not have yt-dlp video/audio selections.
                                        }
                                        DownloadTargetKind::Normal => {
                                            item.progress.video = 100.0;
                                            item.progress.audio = 100.0;
                                            if message.disposition == crate::infrastructure::output_policy::OutputDisposition::Published { item.completed_selection = Some(CompletedSelection::from_selection(&item.selection)); }
                                        }
                                        DownloadTargetKind::Video => item.progress.video = 100.0,
                                        DownloadTargetKind::Audio => item.progress.audio = 100.0,
                                        DownloadTargetKind::Subtitle => {
                                            item.progress.subtitle = 100.0
                                        }
                                    }
                                    if message.disposition == crate::infrastructure::output_policy::OutputDisposition::SkippedExisting { run.detail = format!("Skipped existing destination: {output_path}"); } else if message.processed_before_publication { run.detail = format!("Saved as {output_path}"); }
                                    item.last_output_path = Some(output_path.clone());
                                    item.last_error = None;
                                    if matches!(&item.backend, QueueBackend::YtDlp)
                                        && message.workflow_kind == WorkflowKind::DownloadMedia
                                        && message.target_kind == DownloadTargetKind::Normal
                                    {
                                        if !message.processed_before_publication {
                                            pending_post_process_input = Some(output_path.clone());
                                        }
                                    }
                                }
                                Err(error) if error == DOWNLOAD_CANCELLED_MESSAGE => {
                                    run.state = WorkflowState::Cancelled;
                                    run.error = None;
                                    item.last_error = None;
                                }
                                Err(error) => {
                                    run.state = WorkflowState::Failed;
                                    run.error = Some(error.clone());
                                    item.last_error = Some(error.clone());
                                }
                            }
                        }
                    }

                    let post_process_started =
                        pending_post_process_input
                            .as_deref()
                            .is_some_and(|output_path| {
                                self.maybe_start_builtin_transcode_post_process(
                                    message.item_id,
                                    output_path,
                                )
                            });

                    match message.result {
                        Ok(output_path) => {
                            if message.disposition == crate::infrastructure::output_policy::OutputDisposition::SkippedExisting { self.push_runtime_log(format!("Skipped existing destination: {output_path}")); self.last_action = format!("Skipped existing destination: {output_path}"); } else { self.push_runtime_log(format!("Download finished: {output_path}")); }
                            if !post_process_started && message.disposition != crate::infrastructure::output_policy::OutputDisposition::SkippedExisting {
                                self.last_action.clear();
                            }
                        }
                        Err(error) if error == DOWNLOAD_CANCELLED_MESSAGE => {
                            self.push_runtime_log("Download cancelled".to_owned());
                            self.last_action = "Download stopped.".to_owned();
                        }
                        Err(error) => {
                            self.push_runtime_log(format!("Download failed: {error}"));
                            eprintln!("[download] {error}");
                            self.reveal_log_tab_for_tool_failure();
                            self.last_action = error;
                        }
                    }

                    self.finish_pending_queue_removal(finished_item_id);
                    if should_send_windows_toast && !post_process_started {
                        if message.disposition != crate::infrastructure::output_policy::OutputDisposition::SkippedExisting {
                        self.send_download_result_windows_toast(
                            notification_title,
                            notification_result,
                        );
                        }
                        self.start_next_queued_download_after(finished_item_id);
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        loop {
            match self.post_process_result_rx.try_recv() {
                Ok(PostProcessEvent::Progress {
                    item_id,
                    workflow_id,
                    percent,
                }) => {
                    if let Some(item) = self.queue_item_mut_by_id(item_id) {
                        item.progress.post_process = percent;
                        if let Some(run) =
                            item.workflows.iter_mut().find(|run| run.id == workflow_id)
                        {
                            run.progress = percent;
                        }
                    }
                }
                Ok(PostProcessEvent::ToolCommandFinished {
                    item_id: _,
                    workflow_id,
                    tool,
                    action,
                    command_line,
                    success,
                    detail,
                }) => {
                    let action_id =
                        self.workflow_tool_log_action(workflow_id, "origin", "post-process");
                    self.push_tool_log_step_with_detail_without_failure_reveal(
                        action_id,
                        self.tool_log_status_for_workflow_step(workflow_id, success),
                        tool,
                        action,
                        command_line,
                        detail,
                    );
                }
                Ok(PostProcessEvent::RecoveryStep {
                    item_id: _,
                    workflow_id,
                    action,
                    detail,
                    recover_previous_failure,
                    resolved_success,
                }) => {
                    let action_id =
                        self.workflow_tool_log_action(workflow_id, "origin", "post-process");
                    if recover_previous_failure {
                        self.mark_last_failed_tool_log_step_as_recoverable(action_id);
                    }
                    let status = if resolved_success {
                        ToolLogStatus::Success
                    } else {
                        ToolLogStatus::Skipped
                    };
                    self.push_tool_log_step(action_id, status, "v2", action, detail);
                }
                Ok(PostProcessEvent::Finished(message)) => {
                    let finished_item_id = message.item_id;
                    let notification_title = self
                        .queue_item_by_id(message.item_id)
                        .map(|item| item.title.trim().to_owned())
                        .filter(|title| !title.is_empty())
                        .unwrap_or_else(|| "Download item".to_owned());
                    let notification_result = message.result.clone();
                    self.unregister_active_workflow(message.workflow_id);

                    if let Some(item) = self.queue_item_mut_by_id(message.item_id) {
                        if let Some(run) = item
                            .workflows
                            .iter_mut()
                            .find(|run| run.id == message.workflow_id)
                        {
                            match &message.result {
                                Ok(output_path) => {
                                    run.state = WorkflowState::Finished;
                                    run.progress = 100.0;
                                    run.output_path = Some(output_path.clone());
                                    item.progress.post_process = 100.0;
                                    item.last_output_path = Some(output_path.clone());
                                    if message.disposition == crate::infrastructure::output_policy::OutputDisposition::Published { item.completed_selection = Some(CompletedSelection::from_selection(&item.selection)); }
                                    run.detail = if message.disposition == crate::infrastructure::output_policy::OutputDisposition::SkippedExisting { format!("Skipped existing destination: {output_path}") } else { format!("Saved as {output_path}") };
                                    item.last_error = None;
                                }
                                Err(error) if error == POST_PROCESS_CANCELLED_MESSAGE => {
                                    run.state = WorkflowState::Cancelled;
                                    run.error = None;
                                    item.last_error = None;
                                }
                                Err(error) => {
                                    run.state = WorkflowState::Failed;
                                    run.error = Some(error.clone());
                                    item.last_error = Some(error.clone());
                                    item.completed_selection = None;
                                }
                            }
                        }
                    }

                    match message.result {
                        Ok(output_path) => {
                            self.push_runtime_log(format!("Post-process finished: {output_path}"));
                            self.last_action.clear();
                        }
                        Err(error) if error == POST_PROCESS_CANCELLED_MESSAGE => {
                            self.push_runtime_log("Post-process cancelled".to_owned());
                            self.last_action = "Download stopped.".to_owned();
                        }
                        Err(error) => {
                            self.push_runtime_log(format!("Post-process failed: {error}"));
                            eprintln!("[post-process] {error}");
                            self.reveal_log_tab_for_tool_failure();
                            self.last_action = error;
                        }
                    }

                    if message.disposition
                        != crate::infrastructure::output_policy::OutputDisposition::SkippedExisting
                    {
                        self.send_download_result_windows_toast(
                            notification_title,
                            notification_result,
                        );
                    }
                    self.finish_pending_queue_removal(finished_item_id);
                    self.start_next_queued_download_after(finished_item_id);
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        self.sync_media_session();
    }

    pub fn queue_batch(&mut self) {
        if self.queue_structure_mutation_blocked("rebuilding the queue") {
            return;
        }

        let urls = self.parsed_batch_urls();
        let count = urls.len();
        if count == 0 {
            self.last_action = "There is no URL to add to the batch.".to_owned();
            return;
        }

        self.queue_items = urls
            .iter()
            .map(|url| self.build_queue_item_from_url(url))
            .collect();
        if self.queue_display_mode == QueueDisplayMode::Audio {
            self.prepare_queue_items_for_audio_mode();
            self.save_active_audio_playlist_if_needed();
        }
        self.last_action = i18n::format_fixed_english(
            "Added {count} queued items from batch input.",
            &[("{count}", &count.to_string())],
        );
    }

    pub fn app_mode(&self) -> AppMode {
        self.app_mode
    }

    pub fn set_app_mode(&mut self, mode: AppMode) {
        if self.app_mode == mode {
            return;
        }
        if self.queue_structure_mutation_blocked("switching modes") {
            return;
        }
        self.app_mode = mode;
        match mode {
            AppMode::Audio => {
                if self.queue_display_mode != QueueDisplayMode::Audio {
                    self.enter_audio_queue_context();
                    self.queue_display_mode = QueueDisplayMode::Audio;
                    self.config.queue_display_mode =
                        QueueDisplayMode::Audio.config_value().to_owned();
                }
            }
            AppMode::Origin | AppMode::Standard => {
                if self.queue_display_mode == QueueDisplayMode::Audio {
                    self.leave_audio_queue_context();
                    self.queue_display_mode = QueueDisplayMode::Normal;
                    self.config.queue_display_mode =
                        QueueDisplayMode::Normal.config_value().to_owned();
                }
            }
        }
        self.config.app_mode = mode.config_value().to_owned();
        let _ = self.config.save();
        self.last_action = self.ui_i18n_text_for_key(mode.label_key()).to_owned();
    }

    pub fn queue_display_mode(&self) -> QueueDisplayMode {
        self.queue_display_mode
    }

    pub fn set_queue_display_mode(&mut self, mode: QueueDisplayMode) {
        if self.queue_display_mode == mode {
            return;
        }
        if self.queue_structure_mutation_blocked("switching list modes") {
            return;
        }
        if mode == QueueDisplayMode::Audio {
            self.enter_audio_queue_context();
            self.app_mode = AppMode::Audio;
        } else {
            self.leave_audio_queue_context();
            if self.app_mode == AppMode::Audio {
                self.app_mode = AppMode::Standard;
            }
        }
        self.queue_display_mode = mode;
        self.config.queue_display_mode = mode.config_value().to_owned();
        self.config.app_mode = self.app_mode.config_value().to_owned();
        let _ = self.config.save();
        let mode_label_key = match mode {
            QueueDisplayMode::Normal => "Standard",
            QueueDisplayMode::Audio => "Audio",
        };
        self.last_action =
            i18n::format_fixed_english("List mode: {mode}", &[("{mode}", mode_label_key)]);
    }

    pub fn start_single_download(&mut self) {
        if self.app_mode == AppMode::Origin {
            self.request_origin_download();
            return;
        }
        let url = self.primary_candidate_url();
        if self.queue_items.is_empty() && url.is_none() && self.parsed_batch_urls().is_empty() {
            self.last_action = "There is no URL to download.".to_owned();
            return;
        }

        if self.queue_items.is_empty() && !self.parsed_batch_urls().is_empty() {
            self.queue_batch();
        }

        if self.queue_items.is_empty() {
            let url = url.expect("url checked above");
            if let Err(error) = self.ensure_yt_dlp_ready() {
                self.last_action = error;
                return;
            }
            self.queue_items = vec![self.build_queue_item_from_url(&url)];
        }

        if self.has_running_download_workflow() {
            self.last_action =
                "A download is already running. Please wait for it to finish.".to_owned();
            return;
        }

        self.enqueue_download_ready_items();

        let Some(item_id) = self
            .queue_items
            .iter()
            .find(|item| item_latest_download_state(item).is_some_and(is_pending_download_state))
            .map(|item| item.id)
        else {
            self.last_action = "There are no runnable batch items.".to_owned();
            return;
        };

        let emit_json = self
            .queue_item_by_id(item_id)
            .is_some_and(|item| !item.metadata_loaded());
        let _ = self.start_download_task_at(item_id, emit_json);
    }
}
