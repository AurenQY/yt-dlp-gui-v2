//! On-demand analysis for compact restored queues; no startup network or duplicate insertion.
use super::*;
use crate::domain::queue_reanalysis::{select_reanalyzed_video, validate_saved_selection};
use crate::infrastructure::gallery_dl::GalleryDlBackend;

#[derive(Clone, Copy)]
pub(super) enum QueueRefreshAction {
    Download,
    Picker(FormatPickerKind),
}
pub(super) struct QueueRefreshResult {
    item_id: QueueItemId,
    workflow_id: WorkflowRunId,
    action: QueueRefreshAction,
    result: Result<QueueRefreshOutput, String>,
    command_line: Option<String>,
}
enum QueueRefreshOutput {
    Video(Value),
    Gallery(crate::domain::GalleryAnalysis),
}

impl AppState {
    pub(super) fn refresh_restored_queue_item(
        &mut self,
        item_id: QueueItemId,
        action: QueueRefreshAction,
    ) -> bool {
        let Some(item) = self.queue_item_by_id(item_id) else {
            return false;
        };
        if !item.needs_analysis_refresh {
            return false;
        }
        if item.workflows.iter().any(|run| {
            run.kind == WorkflowKind::AnalyzeMetadata
                && matches!(run.state, WorkflowState::Queued | WorkflowState::Running)
        }) {
            return true;
        }
        let source = item.source_url.clone();
        let use_cookies = item.selection.use_cookies;
        let index = item.playlist_index;
        let media_id = item.source_media_id.clone();
        let gallery = matches!(item.backend, QueueBackend::GalleryDl(_));
        let tool = if gallery {
            ToolKind::Other("gallery-dl".into())
        } else {
            ToolKind::YtDlp
        };
        let workflow_id = self.alloc_workflow_run_id();
        self.register_active_workflow(
            item_id,
            workflow_id,
            WorkflowKind::AnalyzeMetadata,
            tool.clone(),
        );
        if let Some(item) = self.queue_item_mut_by_id(item_id) {
            item.last_error = None;
            item.workflows.push(WorkflowRun::new(
                workflow_id,
                WorkflowKind::AnalyzeMetadata,
                tool,
                WorkflowState::Running,
            ));
        }
        let paths = self.tool_paths.clone();
        let tx = self.queue_refresh_tx.clone();
        self.last_action = format!("Refreshing saved item: {source}");
        thread::spawn(move || {
            let mut command_line = None;
            let result = if gallery {
                GalleryDlBackend::from_tool_paths(&paths)
                    .analyze_url(&source, &paths, use_cookies)
                    .and_then(|output| {
                        command_line = Some(output.command_line);
                        if output.analysis.media.is_empty() {
                            Err("gallery-dl found no media for the saved source.".into())
                        } else {
                            Ok(QueueRefreshOutput::Gallery(output.analysis))
                        }
                    })
            } else {
                match paths.analyze_queue_item_detailed(&source, use_cookies, index) {
                    Ok(output) => {
                        command_line = Some(output.command_line);
                        select_reanalyzed_video(output.json, index, media_id.as_deref())
                            .map(QueueRefreshOutput::Video)
                    }
                    Err(error) => {
                        command_line = error.command_line;
                        Err(error.message)
                    }
                }
            };
            let _ = tx.send(QueueRefreshResult {
                item_id,
                workflow_id,
                action,
                result,
                command_line,
            });
        });
        true
    }

    pub(super) fn poll_queue_reanalysis(&mut self) {
        while let Ok(message) = self.queue_refresh_rx.try_recv() {
            if !self
                .active_workflows
                .get(&message.workflow_id)
                .is_some_and(|run| run.item_id == message.item_id)
            {
                continue;
            }
            let action_id =
                self.push_tool_log_action(self.app_mode.config_value(), "refresh saved item");
            if let Some(command) = message.command_line {
                let tool = self
                    .active_workflows
                    .get(&message.workflow_id)
                    .map(|run| workflow_tool_label(&run.tool))
                    .unwrap_or_default();
                self.push_tool_log_step_internal(
                    action_id,
                    if message.result.is_ok() {
                        ToolLogStatus::Success
                    } else {
                        ToolLogStatus::Failed
                    },
                    &tool,
                    "analyze",
                    command,
                    message.result.as_ref().err().cloned(),
                    true,
                );
            }
            match message.result {
                Err(error) => {
                    self.finish_queue_refresh_error(message.item_id, message.workflow_id, error);
                    continue;
                }
                Ok(QueueRefreshOutput::Video(json)) => {
                    let source = self
                        .queue_item_by_id(message.item_id)
                        .map(|item| item.source_url.clone());
                    self.apply_analysis_json(
                        json,
                        source,
                        Some(message.item_id),
                        Some(message.workflow_id),
                    );
                }
                Ok(QueueRefreshOutput::Gallery(mut analysis)) => {
                    let roots: Vec<_> = analysis
                        .posts
                        .first()
                        .map(|p| &p.source_metadata)
                        .into_iter()
                        .chain(analysis.media.first().map(|m| &m.source_metadata))
                        .collect();
                    analysis.persisted_metadata = Some(crate::app::gallery_canonical_metadata::map_gallery_canonical_metadata_from_roots(&roots));
                    let preview =
                        crate::app::gallery_metadata_mapping::map_gallery_ui_metadata(&analysis);
                    if let Some(item) = self.queue_item_mut_by_id(message.item_id) {
                        item.title = preview.title;
                        item.thumbnail_url = preview.thumbnail;
                        item.needs_analysis_refresh = false;
                        item.metadata_state = MetadataState::Idle;
                        if let QueueBackend::GalleryDl(gallery) = &mut item.backend {
                            gallery.analysis = Some(analysis);
                        }
                        if let Some(run) = item
                            .workflows
                            .iter_mut()
                            .find(|run| run.id == message.workflow_id)
                        {
                            run.state = WorkflowState::Finished;
                        }
                    }
                    self.unregister_active_workflow(message.workflow_id);
                    self.mark_font_content_changed();
                }
            }
            let validation = self.validate_queue_item_saved_selection(message.item_id);
            match message.action {
                QueueRefreshAction::Picker(kind) => {
                    if let Err(error) = validation {
                        self.last_action = error.clone();
                        if let Some(item) = self.queue_item_mut_by_id(message.item_id) {
                            item.last_error = Some(error);
                        }
                    }
                    if let Some(index) = self.queue_item_index_by_id(message.item_id) {
                        self.open_format_picker(index, kind);
                    }
                }
                QueueRefreshAction::Download => {
                    if let Err(error) = validation {
                        self.mark_download_preflight_failed(message.item_id, &error);
                        self.last_action = error;
                    } else {
                        let _ = self.start_download_task_at(message.item_id, false);
                    }
                }
            }
        }
    }

    fn finish_queue_refresh_error(
        &mut self,
        item_id: QueueItemId,
        workflow_id: WorkflowRunId,
        error: String,
    ) {
        if let Some(item) = self.queue_item_mut_by_id(item_id) {
            // Keep the original offline preview and saved selectors on failure.
            item.last_error = Some(error.clone());
            if let Some(run) = item.workflows.iter_mut().find(|run| run.id == workflow_id) {
                run.state = WorkflowState::Failed;
                run.error = Some(error.clone());
            }
        }
        self.unregister_active_workflow(workflow_id);
        self.mark_download_preflight_failed(item_id, &error);
        self.last_action = error;
    }

    pub(super) fn validate_queue_item_saved_selection(
        &self,
        item_id: QueueItemId,
    ) -> Result<(), String> {
        let item = self
            .queue_item_by_id(item_id)
            .ok_or("Saved item no longer exists.")?;
        if matches!(item.backend, QueueBackend::GalleryDl(_)) {
            return Ok(());
        }
        if item.restored_chapters_unresolved {
            return Err(
                "Saved chapter boundaries could not be resolved. Select the download range again."
                    .into(),
            );
        }
        let metadata = item.metadata().ok_or("Wait for item analysis to finish.")?;
        validate_saved_selection(&item.selection, metadata)?;
        let index = self
            .queue_item_index_by_id(item_id)
            .ok_or("Saved item no longer exists.")?;
        let compatible = match item.selection.container_preference {
            DownloadContainerPreference::Mp4 => self.item_supports_mp4_download_container(index),
            DownloadContainerPreference::Webm => self.item_supports_webm_download_container(index),
            DownloadContainerPreference::Mkv => true,
            _ => true,
        };
        if !compatible {
            return Err("The saved output container is incompatible with the current formats. Select a container again.".into());
        }
        Ok(())
    }
}
