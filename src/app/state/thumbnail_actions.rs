//! Queue thumbnail ownership and UI-size requests; all pixel/disk work stays off the UI thread.
use super::*;
use crate::app::thumbnail_pixels::thumbnail_pixel_bounds;
use crate::infrastructure::queue_thumbnail_cache::{
    QueueThumbnailCache, QueueThumbnailCacheRequest,
};
use std::collections::BTreeSet;

pub(super) fn queue_item_thumbnail_owner(item: &QueueItem) -> String {
    item.persistent_id.as_str().to_owned()
}

fn normalized_candidates(urls: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for url in urls
        .iter()
        .map(|url| url.trim())
        .filter(|url| !url.is_empty())
    {
        if !result.iter().any(|entry| entry == url) {
            result.push(url.to_owned());
        }
    }
    result
}

/// Queue presentation belongs to the item, not its refreshed URL or pixel bounds.
/// Origin previews keep their existing source-scoped presentation lifetime.
fn thumbnail_presentation_prefix(key: &str) -> String {
    if let Some(rest) = key.strip_prefix("queue:")
        && let Some((owner, _)) = rest.split_once(':')
    {
        return format!("queue:{owner}:");
    }
    key.rsplit_once(':')
        .map(|(prefix, _)| format!("{prefix}:"))
        .unwrap_or_else(|| key.to_owned())
}

impl AppState {
    pub fn poll_thumbnail_work(&mut self, ctx: &eframe::egui::Context) {
        while let Ok(event) = self.thumbnail_result_rx.try_recv() {
            if !self.thumbnail_cache.contains_key(&event.key) {
                continue;
            }
            match event.stage {
                ThumbnailFetchStage::Foreground => {
                    let entry = match event.result {
                        Ok(image) => {
                            let prefix = thumbnail_presentation_prefix(&event.key);
                            let previous = self.thumbnail_cache.iter().find_map(|(key, entry)| {
                                if key.starts_with(&prefix)
                                    && let ThumbnailCacheEntry::Ready(ready) = entry
                                {
                                    Some(ready.clone())
                                } else {
                                    None
                                }
                            });
                            self.thumbnail_cache.retain(|other, _| {
                                other == &event.key || !other.starts_with(&prefix)
                            });
                            let texture = ctx.load_texture(
                                thumbnail_texture_id(&event.key),
                                image,
                                eframe::egui::TextureOptions::LINEAR,
                            );
                            ThumbnailCacheEntry::Ready(ThumbnailReadyImage {
                                foreground: texture,
                                // Keep an already-visible image's fade lifetime across replacement.
                                blurred_background: previous
                                    .as_ref()
                                    .and_then(|ready| ready.blurred_background.clone()),
                                foreground_ready_at: previous
                                    .as_ref()
                                    .map(|ready| ready.foreground_ready_at)
                                    .unwrap_or_else(Instant::now),
                                background_ready_at: previous
                                    .and_then(|ready| ready.background_ready_at),
                                attempt: event.attempt,
                            })
                        }
                        Err(error) => ThumbnailCacheEntry::Failed {
                            error,
                            attempt: event.attempt,
                            retry_at: event
                                .retryable
                                .then(|| {
                                    crate::app::thumbnail_worker::thumbnail_retry_delay(
                                        event.attempt,
                                    )
                                })
                                .flatten()
                                .map(|delay| Instant::now() + delay),
                        },
                    };
                    self.thumbnail_cache.insert(event.key, entry);
                }
                ThumbnailFetchStage::BlurredBackground => {
                    if let Ok(image) = event.result {
                        if let Some(ThumbnailCacheEntry::Ready(ready)) =
                            self.thumbnail_cache.get_mut(&event.key)
                        {
                            if ready.attempt == event.attempt {
                                ready.blurred_background = Some(ctx.load_texture(
                                    format!("{}-blur", thumbnail_texture_id(&event.key)),
                                    image,
                                    eframe::egui::TextureOptions::LINEAR,
                                ));
                                ready.background_ready_at.get_or_insert_with(|| {
                                    Instant::now()
                                        .max(ready.foreground_ready_at + Duration::from_millis(110))
                                });
                            }
                        }
                    }
                }
            }
            ctx.request_repaint();
        }
    }

    pub fn has_loading_thumbnails(&self) -> bool {
        self.thumbnail_cache
            .values()
            .any(|entry| matches!(entry, ThumbnailCacheEntry::Loading))
    }

    pub(super) fn sync_queue_thumbnail_cache(&mut self) {
        let root = self.app_cache_root_path();
        if self
            .queue_thumbnail_disk_cache
            .as_ref()
            .is_none_or(|cache| !cache.uses_root(&root))
        {
            self.thumbnail_cache
                .retain(|key, _| !key.starts_with("queue:"));
            self.queue_thumbnail_disk_cache = Some(QueueThumbnailCache::new(&root));
        }
        let current: BTreeSet<_> = self
            .queue_items
            .iter()
            .chain(&self.music.non_audio_queue_items)
            .chain(&self.music.audio_queue_items)
            .map(queue_item_thumbnail_owner)
            .collect();
        self.thumbnail_cache.retain(|key, _| {
            !key.starts_with("queue:")
                || key
                    .split(':')
                    .nth(1)
                    .is_some_and(|owner| current.contains(owner))
        });
        let mut live = current;
        live.extend(self.queue_thumbnail_saved_owners.iter().cloned());
        self.queue_thumbnail_disk_cache
            .as_ref()
            .unwrap()
            .sync_owners(live, true);
    }

    pub(super) fn finish_queue_thumbnail_cleanup(&mut self) {
        if let Some(cache) = self.queue_thumbnail_disk_cache.as_ref() {
            cache.sync_owners(self.queue_thumbnail_saved_owners.clone(), true);
            if let Err(error) = cache.prune() {
                eprintln!("[thumbnail-cache] {error}");
            }
        }
    }

    pub fn queue_thumbnail_render_source(
        &mut self,
        ctx: &eframe::egui::Context,
        item_id: QueueItemId,
        urls: &[String],
        logical_size: eframe::egui::Vec2,
    ) -> ThumbnailRenderSource {
        let Some(bounds) =
            thumbnail_pixel_bounds([logical_size.x, logical_size.y], ctx.pixels_per_point())
        else {
            return ThumbnailRenderSource::None;
        };
        let Some(item) = self.queue_item_by_id(item_id) else {
            return ThumbnailRenderSource::None;
        };
        let owner = queue_item_thumbnail_owner(item);
        let blur_background = item.view_kind != QueueItemViewKind::MusicCompact;
        let mut candidates = normalized_candidates(urls);
        if let Some(metadata) = item.metadata() {
            if candidates
                .first()
                .is_some_and(|url| *url == metadata.thumbnail_url)
            {
                candidates.extend(
                    metadata
                        .thumbnail_candidates
                        .iter()
                        .filter(|url| !url.is_empty())
                        .cloned(),
                );
                candidates = normalized_candidates(&candidates);
            }
        }
        if candidates
            .first()
            .is_some_and(|url| !thumbnail_needs_memory_loader(url))
        {
            return ThumbnailRenderSource::DirectUrl;
        }
        if self
            .queue_thumbnail_disk_cache
            .as_ref()
            .is_none_or(|cache| !cache.has_owner(&owner))
        {
            self.sync_queue_thumbnail_cache();
        }
        let disk_request = QueueThumbnailCacheRequest::new(
            Arc::clone(self.queue_thumbnail_disk_cache.as_ref().unwrap()),
            owner.clone(),
            candidates.first().map(String::as_str),
            bounds,
        );
        let proxy = self
            .tool_paths
            .effective_proxy_url()
            .unwrap_or_default()
            .to_owned();
        let source_key = thumbnail_texture_id(&thumbnail_cache_key(
            &candidates.join("\n"),
            &proxy,
            self.tool_paths.no_check_certificates,
        ));
        let prefix = format!("queue:{owner}:{source_key}:");
        let owner_prefix = format!("queue:{owner}:");
        // Retire obsolete requests, but keep the displayed texture until its replacement succeeds.
        self.thumbnail_cache.retain(|key, entry| {
            !key.starts_with(&owner_prefix)
                || key.starts_with(&prefix)
                || matches!(entry, ThumbnailCacheEntry::Ready(_))
        });
        self.thumbnail_request(
            ctx,
            candidates,
            proxy,
            prefix,
            ThumbnailFetchOptions {
                bounds,
                queue_cache: Some(disk_request),
                blur_background,
            },
        )
    }

    pub fn single_thumbnail_render_source_for_url(
        &mut self,
        ctx: &eframe::egui::Context,
        url: &str,
        logical_size: eframe::egui::Vec2,
    ) -> ThumbnailRenderSource {
        let candidates = normalized_candidates(&[url.to_owned()]);
        if candidates.is_empty() {
            return ThumbnailRenderSource::None;
        }
        if !thumbnail_needs_memory_loader(&candidates[0]) {
            return ThumbnailRenderSource::DirectUrl;
        }
        let Some(bounds) =
            thumbnail_pixel_bounds([logical_size.x, logical_size.y], ctx.pixels_per_point())
        else {
            return ThumbnailRenderSource::None;
        };
        let proxy = self
            .tool_paths
            .effective_proxy_url()
            .unwrap_or_default()
            .to_owned();
        let source_key = thumbnail_texture_id(&thumbnail_cache_key(
            &candidates[0],
            &proxy,
            self.tool_paths.no_check_certificates,
        ));
        let prefix = format!("origin:{source_key}:");
        self.thumbnail_cache
            .retain(|key, _| !key.starts_with("origin:") || key.starts_with(&prefix));
        self.thumbnail_request(
            ctx,
            candidates,
            proxy,
            prefix,
            ThumbnailFetchOptions {
                bounds,
                queue_cache: None,
                blur_background: false,
            },
        )
    }

    fn thumbnail_request(
        &mut self,
        ctx: &eframe::egui::Context,
        candidates: Vec<String>,
        proxy_url: String,
        prefix: String,
        options: ThumbnailFetchOptions,
    ) -> ThumbnailRenderSource {
        self.poll_thumbnail_work(ctx);
        let key = format!("{prefix}{}x{}", options.bounds[0], options.bounds[1]);
        let presentation_prefix = thumbnail_presentation_prefix(&key);
        let fallback = self.thumbnail_cache.iter().find_map(|(other, entry)| {
            if !other.starts_with(&presentation_prefix) {
                return None;
            }
            if let ThumbnailCacheEntry::Ready(ready) = entry {
                Some(ready.clone())
            } else {
                None
            }
        });
        // During live Origin resize, let one in-flight request finish, then use the latest size.
        if self.thumbnail_cache.iter().any(|(other, entry)| {
            other != &key
                && other.starts_with(&prefix)
                && matches!(entry, ThumbnailCacheEntry::Loading)
        }) {
            ctx.request_repaint_after(Duration::from_millis(100));
            return fallback
                .map(ThumbnailRenderSource::Texture)
                .unwrap_or(ThumbnailRenderSource::Loading);
        }
        let now = Instant::now();
        let attempt = match self.thumbnail_cache.get(&key) {
            Some(ThumbnailCacheEntry::Failed {
                attempt,
                retry_at: Some(at),
                ..
            }) if now >= *at => attempt + 1,
            _ => 1,
        };
        if attempt > 1 {
            self.thumbnail_cache.remove(&key);
        }
        match self.thumbnail_cache.get(&key) {
            Some(ThumbnailCacheEntry::Ready(texture)) => {
                ThumbnailRenderSource::Texture(texture.clone())
            }
            Some(ThumbnailCacheEntry::Loading) => {
                ctx.request_repaint_after(Duration::from_millis(100));
                fallback
                    .map(ThumbnailRenderSource::Texture)
                    .unwrap_or(ThumbnailRenderSource::Loading)
            }
            Some(ThumbnailCacheEntry::Failed {
                error, retry_at, ..
            }) => {
                if let Some(at) = retry_at {
                    ctx.request_repaint_after(at.saturating_duration_since(now));
                }
                fallback
                    .map(ThumbnailRenderSource::Texture)
                    .unwrap_or_else(|| ThumbnailRenderSource::Failed(error.clone()))
            }
            None => {
                // Keep the previous ready entry through loading/failure. Successful foreground
                // delivery retires it after inheriting the presentation timestamps.
                self.thumbnail_cache
                    .insert(key.clone(), ThumbnailCacheEntry::Loading);
                run_thumbnail_fetch_worker(
                    key,
                    candidates,
                    proxy_url,
                    self.tool_paths.no_check_certificates,
                    attempt,
                    options,
                    self.thumbnail_result_tx.clone(),
                );
                ctx.request_repaint_after(Duration::from_millis(100));
                fallback
                    .map(ThumbnailRenderSource::Texture)
                    .unwrap_or(ThumbnailRenderSource::Loading)
            }
        }
    }
}
