use super::*;

impl AppState {
    pub fn item_supports_mp4_download_container(&self, item_index: usize) -> bool {
        self.selected_download_codecs(item_index)
            .is_some_and(|(video, audio)| codecs_support_mp4_container(video, audio))
    }

    pub fn item_supports_webm_download_container(&self, item_index: usize) -> bool {
        self.selected_download_codecs(item_index)
            .is_some_and(|(video, audio)| codecs_support_webm_container(video, audio))
    }

    pub fn item_download_container_is_selectable(&self, item_index: usize) -> bool {
        !self.item_is_busy(item_index)
            && (self.item_supports_mp4_download_container(item_index)
                || self.item_supports_webm_download_container(item_index))
    }

    pub fn resolved_item_download_container_extension(&self, item_index: usize) -> Option<String> {
        let item = self.queue_items.get(item_index)?;
        if let Some(extension) = item.selection.container_preference.extension() {
            return item.metadata_loaded().then(|| extension.to_owned());
        }
        let metadata = item.metadata()?;
        let video_ext = metadata
            .formats
            .iter()
            .find(|format| format.id == item.selection.video_selector)
            .map(|format| format.ext.as_str())
            .unwrap_or_default();
        let audio_ext = metadata
            .formats
            .iter()
            .find(|format| format.id == item.selection.audio_selector)
            .map(|format| format.ext.as_str())
            .unwrap_or_default();

        Some(normal_download_default_extension(
            self.item_download_container_override(item_index),
            self.item_uses_muxed_video(item_index),
            video_ext,
            audio_ext,
        ))
    }

    pub fn item_download_container_override(&self, item_index: usize) -> Option<&'static str> {
        let item = self.queue_items.get(item_index)?;
        if let Some(extension) = item.selection.container_preference.extension() {
            let mergeable = matches!(
                item.selection.container_preference,
                DownloadContainerPreference::Mkv
                    | DownloadContainerPreference::Mp4
                    | DownloadContainerPreference::Webm
            ) && self
                .item_supports_download_container(item_index, item.selection.container_preference);
            return mergeable.then_some(extension);
        }
        (item.selection.embed_thumbnail && self.item_supports_webm_download_container(item_index))
            .then_some("mkv")
    }

    pub fn item_download_output_extension_override(
        &self,
        item_index: usize,
    ) -> Option<&'static str> {
        let item = self.queue_items.get(item_index)?;
        item.metadata_loaded()
            .then(|| item.selection.container_preference.extension())
            .flatten()
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
        let audio_codec = metadata
            .formats
            .iter()
            .find(|format| format.id == item.selection.audio_selector)?
            .codec
            .as_str();
        Some((video_codec, audio_codec))
    }

    fn item_supports_download_container(
        &self,
        item_index: usize,
        preference: DownloadContainerPreference,
    ) -> bool {
        match preference {
            DownloadContainerPreference::Auto => true,
            DownloadContainerPreference::Mkv => {
                self.queue_items
                    .get(item_index)
                    .is_some_and(|item| item.metadata_loaded())
                    && !self.item_uses_muxed_video(item_index)
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_container_extensions_are_stable() {
        assert_eq!(DownloadContainerPreference::Mkv.extension(), Some("mkv"));
        assert_eq!(DownloadContainerPreference::Mp4.extension(), Some("mp4"));
        assert_eq!(DownloadContainerPreference::Webm.extension(), Some("webm"));
        assert_eq!(DownloadContainerPreference::Mov.extension(), Some("mov"));
        assert_eq!(DownloadContainerPreference::Flv.extension(), Some("flv"));
        assert_eq!(DownloadContainerPreference::Auto.extension(), None);
    }

    #[test]
    fn save_as_extensions_map_to_explicit_container_preferences() {
        assert_eq!(
            DownloadContainerPreference::from_extension(".mov"),
            Some(DownloadContainerPreference::Mov)
        );
        assert_eq!(
            DownloadContainerPreference::from_extension("FLV"),
            Some(DownloadContainerPreference::Flv)
        );
        assert_eq!(DownloadContainerPreference::from_extension("mp3"), None);
    }

    #[test]
    fn choosing_webm_keeps_thumbnail_as_sidecar_instead_of_embedding() {
        let mut selection = crate::domain::DownloadSelection {
            write_thumbnail: true,
            embed_thumbnail: true,
            ..Default::default()
        };

        apply_download_container_preference(&mut selection, DownloadContainerPreference::Webm);

        assert_eq!(
            selection.container_preference,
            DownloadContainerPreference::Webm
        );
        assert!(selection.write_thumbnail);
        assert!(!selection.embed_thumbnail);
    }
}
