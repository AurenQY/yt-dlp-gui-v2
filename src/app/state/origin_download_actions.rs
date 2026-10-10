//! Classic single-item download controls and Gallery feedback; never dispatch hidden queue items.
use super::*;

impl AppState {
    pub fn origin_download_cancellable(&self) -> bool {
        self.queue_items
            .first()
            .is_some_and(|item| self.item_has_cancellable_download_workflow(item.id))
    }

    pub fn origin_download_stopping(&self) -> bool {
        self.queue_items
            .first()
            .is_some_and(|item| self.item_download_stopping(item.id))
    }

    pub fn origin_download_can_start(&self) -> bool {
        !self.has_running_download_workflow()
            && self.queue_items.first().is_some_and(|item| {
                item_can_enter_download_queue(item)
                    && !matches!(
                        item.metadata_state,
                        MetadataState::Queued | MetadataState::Running
                    )
                    && !item.workflows.iter().any(|run| {
                        run.state == WorkflowState::Running
                            || (run.state == WorkflowState::Queued
                                && run.kind != WorkflowKind::DownloadMedia)
                    })
            })
    }

    pub fn request_origin_download(&mut self) {
        if self.app_mode != AppMode::Origin {
            return;
        }
        let Some(item_id) = self.queue_items.first().map(|item| item.id) else {
            return;
        };
        if self.origin_download_cancellable() {
            if !self.origin_download_stopping() {
                self.cancel_item_download(item_id);
            }
            return;
        }
        if !self.origin_download_can_start() {
            return;
        }
        if self
            .queue_items
            .first()
            .and_then(item_latest_download_state)
            != Some(WorkflowState::Queued)
        {
            self.enqueue_download_ready_item(item_id);
        }
        let emit_json = self
            .queue_item_by_id(item_id)
            .is_some_and(|item| !item.metadata_loaded());
        if let Err(error) = self.start_download_task_at(item_id, emit_json) {
            self.last_action = error;
        }
    }
}
