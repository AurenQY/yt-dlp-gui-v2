use super::*;

impl AppState {
    pub fn item_supports_mp4_download_container(&self, item_index: usize) -> bool {
        if self.item_uses_muxed_video(item_index) {
            return self.selected_muxed_container_is(item_index, "mp4");
        }
        self.selected_download_codecs(item_index)
            .is_some_and(|(video, audio)| codecs_support_mp4_container(video, audio))
    }

    pub fn item_supports_webm_download_container(&self, item_index: usize) -> bool {
        if self.item_uses_muxed_video(item_index) {
            return self.selected_muxed_container_is(item_index, "webm");
        }
        self.selected_download_codecs(item_index)
            .is_some_and(|(video, audio)| codecs_support_webm_container(video, audio))
    }

    pub fn item_download_container_is_selectable(&self, item_index: usize) -> bool {
        !self.item_is_busy(item_index)
            && self
                .queue_items
                .get(item_index)
                .and_then(|item| item.metadata())
                .is_some_and(|metadata| {
                    metadata.formats.iter().any(|format| {
                        format.id == self.queue_items[item_index].selection.video_selector
                    })
                })
    }

    pub fn resolved_item_download_container_extension(&self, item_index: usize) -> Option<String> {
        let item = self.queue_items.get(item_index)?;
        if item.needs_analysis_refresh {
            return item
                .selection
                .container_preference
                .extension()
                .map(str::to_owned)
                .or_else(|| {
                    (!item.saved_download_display.extension.is_empty())
                        .then(|| item.saved_download_display.extension.clone())
                });
        }
        resolved_download_item_extension(item)
    }

    pub fn item_download_container_override(&self, item_index: usize) -> Option<&'static str> {
        let item = self.queue_items.get(item_index)?;
        if let Some(extension) = item.selection.container_preference.extension() {
            let mergeable = matches!(
                item.selection.container_preference,
                DownloadContainerPreference::Mkv
                    | DownloadContainerPreference::Mp4
                    | DownloadContainerPreference::Webm
            ) && !self.item_uses_muxed_video(item_index)
                && self
                    .selected_download_codecs(item_index)
                    .is_some_and(|(_, audio)| !audio.is_empty())
                && self.item_supports_download_container(
                    item_index,
                    item.selection.container_preference,
                );
            return mergeable.then_some(extension);
        }
        (item.selection.embed_thumbnail
            && automatic_download_item_extension(item).as_deref() == Some("webm")
            && !self.item_uses_muxed_video(item_index)
            && !item.selection.audio_selector.is_empty())
        .then_some("mkv")
    }

    pub fn item_download_output_extension_override(
        &self,
        item_index: usize,
    ) -> Option<&'static str> {
        let item = self.queue_items.get(item_index)?;
        if !item.metadata_loaded() {
            return None;
        }
        item.selection.container_preference.extension().or_else(|| {
            (item.selection.embed_thumbnail
                && automatic_download_item_extension(item).as_deref() == Some("webm"))
            .then_some("mkv")
        })
    }

    pub fn set_item_download_container_preference(
        &mut self,
        item_index: usize,
        preference: DownloadContainerPreference,
    ) {
        if !self.item_supports_download_container(item_index, preference) {
            return;
        }
        let Some(item) = self.queue_items.get_mut(item_index) else {
            return;
        };
        apply_download_container_preference(&mut item.selection, preference);
    }

    pub(super) fn set_item_download_container_from_save_as(
        &mut self,
        item_index: usize,
        extension: &str,
    ) -> Result<(), String> {
        let preference = DownloadContainerPreference::from_extension(extension)
            .ok_or_else(|| format!("Unsupported output container: {extension}"))?;
        let Some(item) = self.queue_items.get_mut(item_index) else {
            return Err("The selected download item no longer exists.".to_owned());
        };
        apply_download_container_preference(&mut item.selection, preference);
        Ok(())
    }

    pub fn normalize_item_download_base_name(&mut self, item_index: usize) {
        let Some(item) = self.queue_items.get_mut(item_index) else {
            return;
        };
        item.selection.name_origin = Some(crate::domain::OutputNameOrigin::Explicit);
        item.selection.file_name =
            normal_download_base_name(&sanitize_file_name_for_windows(&item.selection.file_name));
    }

    pub(super) fn reconcile_item_download_container(&mut self, item_index: usize) {
        let Some(preference) = self
            .queue_items
            .get(item_index)
            .map(|item| item.selection.container_preference)
        else {
            return;
        };
        if !self.item_supports_download_container(item_index, preference) {
            if let Some(item) = self.queue_items.get_mut(item_index) {
                item.selection.container_preference = DownloadContainerPreference::Auto;
            }
        }
    }

    fn selected_download_codecs(&self, item_index: usize) -> Option<(&str, &str)> {
        let item = self.queue_items.get(item_index)?;
        let metadata = item.metadata()?;
        if self.item_uses_muxed_video(item_index) {
            return None;
        }
        let video_codec = metadata
            .formats
            .iter()
            .find(|format| format.id == item.selection.video_selector)?
            .codec
            .as_str();
        let audio_codec = if item.selection.audio_selector.is_empty() {
            ""
        } else {
            metadata
                .formats
                .iter()
                .find(|format| format.id == item.selection.audio_selector)?
                .codec
                .as_str()
        };
        if !item.selection.audio_selector.is_empty() && audio_codec.is_empty() {
            return None;
        }
        Some((video_codec, audio_codec))
    }

    fn selected_muxed_container_is(&self, item_index: usize, extension: &str) -> bool {
        self.queue_items
            .get(item_index)
            .and_then(|item| {
                item.metadata().and_then(|metadata| {
                    metadata
                        .formats
                        .iter()
                        .find(|format| format.id == item.selection.video_selector)
                })
            })
            .is_some_and(|format| format.ext.eq_ignore_ascii_case(extension))
    }

    fn item_supports_download_container(
        &self,
        item_index: usize,
        preference: DownloadContainerPreference,
    ) -> bool {
        match preference {
            DownloadContainerPreference::Auto => true,
            DownloadContainerPreference::Mkv => self
                .queue_items
                .get(item_index)
                .is_some_and(|item| item.metadata_loaded()),
            DownloadContainerPreference::Mp4 => {
                self.item_supports_mp4_download_container(item_index)
            }
            DownloadContainerPreference::Webm => {
                self.item_supports_webm_download_container(item_index)
            }
            DownloadContainerPreference::Mov | DownloadContainerPreference::Flv => self
                .queue_items
                .get(item_index)
                .is_some_and(|item| item.metadata_loaded()),
        }
    }
}

/// Shared live resolver for card rendering and persistence, including parked queues.
fn resolved_download_item_extension(item: &QueueItem) -> Option<String> {
    if let Some(extension) = item.selection.container_preference.extension() {
        return item.metadata_loaded().then(|| extension.to_owned());
    }
    let automatic = automatic_download_item_extension(item)?;
    Some(if item.selection.embed_thumbnail && automatic == "webm" {
        "mkv".to_owned()
    } else {
        automatic
    })
}

fn automatic_download_item_extension(item: &QueueItem) -> Option<String> {
    let metadata = item.metadata()?;
    let video = metadata
        .formats
        .iter()
        .find(|f| f.id == item.selection.video_selector);
    let audio = metadata
        .formats
        .iter()
        .find(|f| f.id == item.selection.audio_selector);
    if video.is_none() && audio.is_none() {
        return None;
    }
    let muxed = video.is_some_and(|f| f.kind == MediaKind::Muxed);
    Some(normal_download_default_extension(
        None,
        muxed,
        video.map(|f| f.ext.as_str()).unwrap_or_default(),
        audio.map(|f| f.ext.as_str()).unwrap_or_default(),
    ))
}

pub(super) fn download_queue_display(
    item: &QueueItem,
) -> crate::domain::queue_preview::DownloadQueueDisplay {
    if !matches!(item.backend, QueueBackend::YtDlp) {
        return Default::default();
    }
    if item.needs_analysis_refresh {
        return item.saved_download_display.clone();
    }
    let label = |selector: &str| {
        item.metadata()
            .and_then(|m| m.formats.iter().find(|f| f.id == selector))
            .map(|f| f.label.clone())
            .unwrap_or_default()
    };
    crate::domain::queue_preview::DownloadQueueDisplay {
        video: label(&item.selection.video_selector),
        audio: label(&item.selection.audio_selector),
        extension: resolved_download_item_extension(item).unwrap_or_default(),
    }
}

fn apply_download_container_preference(
    selection: &mut crate::domain::DownloadSelection,
    preference: DownloadContainerPreference,
) {
    if preference == DownloadContainerPreference::Webm && selection.embed_thumbnail {
        selection.write_thumbnail = true;
        selection.embed_thumbnail = false;
    }
    selection.container_preference = preference;
}
