//! One Gallery feedback projection for Classic and Standard; templates remain independent.
use super::fields;
use crate::app::state::AppState;
use crate::domain::{QueueBackend, WorkflowKind, WorkflowState};
use std::path::PathBuf;

pub(in crate::app::ui) struct GalleryFeedback {
    pub timeline: bool,
    pub running: bool,
    pub stopping: bool,
    pub finished: bool,
    pub has_download: bool,
    pub completed: usize,
    pub range_text: String,
    pub counter: String,
    pub folder: Option<PathBuf>,
    pub folder_text: String,
    pub latest_file: Option<String>,
}

impl GalleryFeedback {
    pub fn from_state(state: &AppState, index: usize) -> Option<Self> {
        let item = state.queue_items.get(index)?;
        let QueueBackend::GalleryDl(gallery) = &item.backend else {
            return None;
        };
        let item_id = item.id;
        let error = item.last_error.clone();
        let output_path = item.last_output_path.clone();
        let active = item
            .workflows
            .iter()
            .rev()
            .find(|run| run.kind == WorkflowKind::DownloadMedia);
        let running = active.is_some_and(|run| run.state == WorkflowState::Running);
        let queued = active.is_some_and(|run| run.state == WorkflowState::Queued);
        let analyzed = gallery.analysis.is_some();
        let stopping = state.item_download_stopping(item_id);
        let timeline = gallery.analysis.as_ref().is_some_and(|analysis| {
            crate::domain::gallery_source_scope(&analysis.root_extractor)
                == crate::domain::GallerySourceScope::Timeline
        });
        let range_summary = if timeline
            && matches!(
                gallery.download_range,
                crate::domain::GalleryDownloadRange::Posts { .. }
            )
            && gallery
                .analysis
                .as_ref()
                .is_some_and(|analysis| !analysis.supports_single_feed_post_range())
        {
            crate::i18n::text(state.language(), "gallery.range.time_or_all").to_owned()
        } else {
            super::super::gallery_range_text::summary(&gallery.download_range, state.language())
        };
        let completed = gallery.download_artifacts.len();
        let posts = gallery.processed_posts;
        let known_post_limit = match gallery.download_range {
            crate::domain::GalleryDownloadRange::Posts { count } => Some(count),
            _ => None,
        };
        let current_date = gallery
            .download_artifacts
            .last()
            .and_then(|artifact| artifact.date.as_deref())
            .map(|date| date.chars().take(10).collect::<String>());
        let stopped = item
            .workflows
            .iter()
            .rev()
            .find(|run| run.kind == WorkflowKind::DownloadMedia)
            .is_some_and(|run| run.state == WorkflowState::Cancelled);
        let status_key = if stopping {
            "action.stopping"
        } else if running && active.is_some_and(|run| run.detail == "gallery-finishing") {
            "gallery.status.finishing"
        } else if running {
            "about.status.downloading"
        } else if queued {
            "item.status.queued"
        } else if stopped {
            "item.status.cancelled"
        } else if error.is_some() {
            "item.status.failed"
        } else if active.is_some_and(|run| run.state == WorkflowState::Finished) {
            "item.status.finished"
        } else if analyzed {
            "gallery.status.waiting"
        } else {
            "gallery.status.analyzing"
        };
        let finished = status_key == "item.status.finished";
        let status = crate::i18n::text(state.language(), status_key);
        let has_download = item
            .workflows
            .iter()
            .any(|run| run.kind == WorkflowKind::DownloadMedia);
        let range_text = if has_download || error.is_some() {
            format!("{status} · {}", current_date.as_deref().unwrap_or("—"))
        } else if timeline {
            range_summary.clone()
        } else {
            status.to_owned()
        };
        let counter = if !has_download {
            String::new()
        } else if timeline {
            match known_post_limit.filter(|_| !finished) {
                Some(limit) => format!("{posts}/{limit}"),
                None => crate::i18n::format_text(
                    state.language(),
                    "gallery.posts_count",
                    &[("{count}", &posts.to_string())],
                ),
            }
        } else {
            format!(
                "{completed}/{}",
                gallery
                    .analysis
                    .as_ref()
                    .map(|a| a.media.len())
                    .unwrap_or(0)
            )
        };
        let folder = fields::output_folder(gallery, output_path.as_deref());
        let folder_text = fields::folder_display(
            folder.as_deref(),
            &item.selection.output_dir,
            state.language(),
        );
        let latest_file = gallery
            .download_artifacts
            .last()
            .and_then(|a| a.path.file_name())
            .map(|name| name.to_string_lossy().into_owned());

        Some(Self {
            timeline,
            running,
            stopping,
            finished,
            has_download,
            completed,
            range_text,
            counter,
            folder,
            folder_text,
            latest_file,
        })
    }
}
