//! One data/queue.yaml owner for downloads and audio, including parked mode queues.
use super::*;
use crate::domain::queue_snapshot::QueueSnapshot;
use crate::infrastructure::queue_store::{
    queue_snapshot_path, read_queue_snapshot, write_queue_snapshot,
};

#[derive(Default)]
pub(super) struct QueuePersistenceState {
    last_saved: Option<QueueSnapshot>,
    last_error: Option<String>,
    retry_after: Option<Instant>,
}

impl AppState {
    pub(super) fn restore_saved_queues(&mut self) {
        let path = queue_snapshot_path();
        let snapshot = match read_queue_snapshot::<QueueSnapshot>(&path) {
            Ok(None) => return,
            Ok(Some(snapshot)) => snapshot,
            Err(error) => {
                // Queue state is replaceable: a bad document must not disable future saves.
                eprintln!("[queue] {error}");
                self.last_action = error;
                return;
            }
        };
        self.queue_thumbnail_saved_owners = snapshot.thumbnail_ids().map(str::to_owned).collect();
        let mut items = Vec::with_capacity(snapshot.downloads.len());
        for entry in snapshot.downloads {
            let id = self.alloc_queue_item_id();
            let workflow_id = self.alloc_workflow_run_id();
            if let Some(item) = entry.restore(id, workflow_id) {
                items.push(item);
            }
        }
        let audio = snapshot
            .audio
            .into_iter()
            .filter_map(|entry| self.queue_item_from_audio_playlist_snapshot(entry))
            .collect();
        if self.queue_display_mode == QueueDisplayMode::Audio {
            self.music.non_audio_queue_items = items;
            self.queue_items = audio;
        } else {
            self.queue_items = items;
            self.music.audio_queue_items = audio;
        }
        self.rebuild_batch_input_from_queue();
        self.mark_font_content_changed();
        self.sync_queue_thumbnail_cache();
        // Restored pending work is idle. No worker is registered or started here.
    }

    pub fn save_queue_if_changed(&mut self) {
        self.sync_queue_thumbnail_cache();
        if self
            .queue_persistence
            .retry_after
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return;
        }
        let snapshot = QueueSnapshot::capture(
            self.queue_display_mode == QueueDisplayMode::Audio,
            &self.queue_items,
            &self.music.non_audio_queue_items,
            &self.music.audio_queue_items,
            &self.pending_queue_removals,
            super::download_container_actions::download_queue_display,
        );
        // Progress-only redraws compare the safe projection without YAML encoding or I/O.
        if self.queue_persistence.last_saved.as_ref() == Some(&snapshot) {
            return;
        }
        match write_queue_snapshot(&queue_snapshot_path(), &snapshot) {
            Ok(()) => {
                self.queue_thumbnail_saved_owners =
                    snapshot.thumbnail_ids().map(str::to_owned).collect();
                self.sync_queue_thumbnail_cache();
                self.queue_persistence.last_saved = Some(snapshot);
                self.queue_persistence.last_error = None;
                self.queue_persistence.retry_after = None;
            }
            Err(error) => {
                self.queue_persistence.retry_after = Some(Instant::now() + Duration::from_secs(5));
                if self.queue_persistence.last_error.as_ref() != Some(&error) {
                    self.last_action = error.clone();
                    eprintln!("[queue] {error}");
                    self.queue_persistence.last_error = Some(error);
                }
            }
        }
    }

    pub(super) fn flush_queue_persistence_before_exit(&mut self) {
        self.queue_persistence.retry_after = None;
        self.save_queue_if_changed();
        if self.queue_persistence.retry_after.is_none() {
            self.finish_queue_thumbnail_cleanup();
        }
    }
}
