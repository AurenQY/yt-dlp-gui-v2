use super::*;
use crate::domain::audio_queue_snapshot::AudioQueueItemSnapshot;

impl AppState {
    pub(super) fn music_playable_item_ids(&self) -> Vec<QueueItemId> {
        self.queue_items
            .iter()
            .filter(|item| self.music_item_can_play(item.id))
            .map(|item| item.id)
            .collect()
    }

    pub(super) fn complete_music_cache_hit_for_item(
        &self,
        item: &QueueItem,
    ) -> Option<CompleteMusicCacheHit> {
        let source_key = canonical_queue_source_key(&item.source_url);
        let mut best: Option<(u64, CompleteMusicCacheHit)> = None;
        let _use = crate::infrastructure::storage_paths::use_cache().ok()?;
        let root = crate::infrastructure::storage_paths::cache_directory("audio").ok()?;
        let entries = fs::read_dir(&root).ok()?;
        for entry in entries.filter_map(Result::ok) {
            let dir = entry.path();
            if crate::infrastructure::storage_paths::reject_links(&dir).is_err() || !dir.is_dir() {
                continue;
            }
            let manifest_path = dir.join("manifest.yaml");
            if crate::infrastructure::storage_paths::reject_links(&manifest_path).is_err() {
                continue;
            }
            let Some(manifest) = read_yaml_file::<AudioCacheManifestSnapshot>(&manifest_path)
            else {
                continue;
            };
            if !audio_cache_manifest_is_fresh(&manifest) {
                // Lookup only observes. Expiry is an exclusive cleanup action.
                continue;
            }
            if !manifest.complete {
                continue;
            }
            if !manifest.audio_only {
                continue;
            }
            let manifest_source = manifest.source_url.trim().to_owned();
            if manifest_source.is_empty() {
                continue;
            };
            if canonical_queue_source_key(&manifest_source) != source_key {
                continue;
            }
            let ext = manifest.ext.trim().to_owned();
            if ext.trim().is_empty() {
                continue;
            }
            let media_path = dir.join(format!("audio.{}", sanitize_music_cache_ext(&ext)));
            if crate::infrastructure::storage_paths::reject_links(&media_path).is_err() {
                continue;
            }
            let media_len = fs::metadata(&media_path)
                .map(|meta| meta.len())
                .unwrap_or(0);
            if media_len == 0 {
                continue;
            }
            let expected_bytes = manifest.expected_bytes;
            if expected_bytes.is_some_and(|expected| expected > media_len) {
                continue;
            }
            let cache_key = dir
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_owned();
            if cache_key.trim().is_empty() {
                continue;
            }
            let updated = manifest.updated_unix_seconds;
            let hit = CompleteMusicCacheHit {
                cache_key,
                source_url: manifest_source,
                title: manifest.title,
                album_title: manifest.album_title,
                thumbnail_url: manifest.thumbnail_url,
                duration_seconds: manifest.duration_seconds,
                ext,
                format_id: manifest.format_id,
                acodec: manifest.acodec,
                expected_bytes,
            };
            let replace_best = match best.as_ref() {
                Some((best_updated, _)) => updated >= *best_updated,
                None => true,
            };
            if replace_best {
                best = Some((updated, hit));
            }
        }
        best.map(|(_, hit)| hit)
    }

    pub(super) fn prepare_queue_items_for_audio_mode(&mut self) {
        let ids = self
            .queue_items
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();
        for item_id in ids {
            self.prepare_queue_item_for_audio_mode(item_id);
        }
    }

    pub(super) fn prepare_queue_item_for_audio_mode(&mut self, item_id: QueueItemId) {
        if let Some(item) = self.queue_item_mut_by_id(item_id) {
            if item.compact_music_state.is_none() {
                item.compact_music_state = Some(CompactMusicState::Ready);
            }
            if item.music_cache_key.trim().is_empty() && !item.source_url.trim().is_empty() {
                item.music_cache_key = music_cache_key(&item.source_url, "flat", "", "");
            }
            if item.music_duration_seconds.is_none() && !item.duration_text.trim().is_empty() {
                item.music_duration_seconds = duration_text_to_seconds(&item.duration_text);
            }
        }

        self.restore_music_compact_cache_hit_if_available(item_id);
        if let Some(item) = self.queue_item_by_id(item_id) {
            self.cache_music_cover_for_item(item);
        }
    }

    pub(super) fn restore_music_compact_cache_hit_if_available(
        &mut self,
        item_id: QueueItemId,
    ) -> bool {
        let hit = {
            self.queue_item_by_id(item_id)
                .and_then(|item| self.complete_music_cache_hit_for_item(item))
        };
        let Some(hit) = hit else {
            return false;
        };
        if let Some(item) = self.queue_item_mut_by_id(item_id) {
            restore_music_compact_item_from_cache_hit(item, &hit);
            self.mark_font_content_changed();
            return true;
        }
        false
    }

    pub fn music_item_has_complete_cache(&self, item_id: QueueItemId) -> bool {
        let Some(item) = self.queue_item_by_id(item_id) else {
            return false;
        };
        self.complete_music_cache_media_path(item).is_some()
    }

    pub(super) fn music_cached_progress_for_item(&self, item_id: QueueItemId) -> f32 {
        let Some(item) = self.queue_item_by_id(item_id) else {
            return 0.0;
        };
        music_cached_progress_for_item_in_root(item, &self.music_stream_cache_root())
    }

    pub(super) fn music_stream_cache_root(&self) -> PathBuf {
        self.audio_cache_root_path()
    }

    pub(super) fn audio_cache_root_path(&self) -> PathBuf {
        self.app_cache_root_path().join("audio")
    }

    pub(super) fn queue_item_from_audio_playlist_snapshot(
        &mut self,
        entry: AudioQueueItemSnapshot,
    ) -> Option<QueueItem> {
        let source = entry.source_url.trim().to_owned();
        if source.is_empty() {
            return None;
        }
        let mut item = QueueItem::with_persistent_id(
            self.alloc_queue_item_id(),
            entry.persistent_id,
            &source,
            infer_title(&source, "Untitled task", "Imported {tail}"),
        );
        self.apply_queue_item_defaults(&mut item);
        item.selection.name_origin = Some(crate::domain::OutputNameOrigin::Automatic);
        item.view_kind = QueueItemViewKind::MusicCompact;
        item.compact_music_state = Some(CompactMusicState::Ready);
        item.metadata_state = MetadataState::Idle;
        if !entry.title.trim().is_empty() {
            item.title = entry.title;
        }
        item.music_album_title = entry.album_title;
        item.thumbnail_url = entry.thumbnail_url;
        item.duration_text = crate::domain::queue_preview::duration_text(entry.duration_millis);
        item.music_duration_seconds = entry.duration_millis.map(|v| v as f64 / 1000.0);
        item.music_stream_url.clear();
        item.music_stream_headers.clear();
        item.music_cache_key = music_cache_key(&item.source_url, "flat", "", "");
        item.selection.use_cookies = entry.use_cookies;
        item.last_error = None;
        self.restore_music_compact_cache_hit_for_item(&mut item);
        Some(item)
    }

    /// Existing audio event hooks request the same complete snapshot as desktop redraws.
    pub(super) fn save_active_audio_playlist_if_needed(&mut self) {
        self.save_queue_if_changed();
    }

    pub(super) fn restore_music_compact_cache_hit_for_item(&self, item: &mut QueueItem) {
        if let Some(hit) = self.complete_music_cache_hit_for_item(item) {
            restore_music_compact_item_from_cache_hit(item, &hit);
        }
    }
}
