use super::*;

impl AppState {
    pub fn set_post_download_conversion_enabled(&mut self, enabled: bool) {
        self.config.post_download_conversion_enabled = enabled;
        let _ = self.config.save();
    }

    pub fn set_enable_builtin_transcode_after_download(&mut self, enabled: bool) {
        self.set_post_download_conversion_enabled(enabled);
    }

    pub fn set_chapter_compatibility_mode(&mut self, enabled: bool) {
        self.tool_paths.chapter_compatibility_mode = enabled;
        self.config.chapter_compatibility_mode = enabled;
        let _ = self.config.save();
    }

    pub fn set_always_show_download_range(&mut self, enabled: bool) {
        self.config.always_show_download_range = enabled;
        let _ = self.config.save();
    }

    pub fn set_quality_preset(&mut self, preset: QualityPreset) {
        self.item_defaults.quality = preset;
        for item in &mut self.queue_items {
            item.selection.quality = preset;
        }
    }

    pub fn cache_location_display(&self) -> String {
        self.app_cache_root_path().display().to_string()
    }

    pub fn pending_cache_location_display(&self) -> Option<String> {
        match crate::infrastructure::storage_paths::pending_root(
            self.saved_cache_location_mode(),
            &self.config.cache_dir,
        ) {
            Ok(root) if root == self.app_cache_root_path() => None,
            Ok(root) => Some(root.display().to_string()),
            Err(_)
                if self.saved_cache_location_mode()
                    == crate::infrastructure::storage_paths::paths().active_mode =>
            {
                None
            }
            Err(error) => Some(error),
        }
    }

    pub fn cache_storage_error(&self) -> Option<&str> {
        crate::infrastructure::storage_paths::error()
            .or(self.config.cache_settings_error.as_deref())
    }

    pub fn refresh_cache_management_summary_if_stale(&mut self) {
        if self
            .music
            .cache_management_summary_refreshed_at
            .is_some_and(|last| last.elapsed() < Duration::from_secs(2))
        {
            return;
        }
        self.refresh_cache_management_summary();
    }

    pub fn refresh_cache_management_summary(&mut self) {
        let root = self.app_cache_root_path();
        self.music.cache_management_summary = calculate_cache_management_summary(&root);
        self.music.cache_management_summary_refreshed_at = Some(Instant::now());
    }

    pub fn cache_management_usage_display(&self) -> String {
        let total = format_byte_size(self.music.cache_management_summary.total_bytes);
        let audio = format_byte_size(self.music.cache_management_summary.music_bytes);
        let expired = format_byte_size(self.music.cache_management_summary.expired_music_bytes);
        self.ui_i18n_text_with_replacements(
            "options.cache_usage_detail",
            &[
                ("{total}", total.as_str()),
                ("{audio}", audio.as_str()),
                ("{expired}", expired.as_str()),
            ],
        )
    }

    pub fn clear_expired_music_cache(&mut self) {
        let _guard = match crate::infrastructure::storage_paths::exclusive_cleanup() {
            Ok(guard) => guard,
            Err(error) => {
                self.last_action = error;
                return;
            }
        };
        let root = self.music_stream_cache_root();
        match remove_expired_music_cache_dirs(&root) {
            Ok(summary) => {
                self.refresh_cache_management_summary();
                let count = summary.entries.to_string();
                let size = format_byte_size(summary.bytes);
                self.last_action = i18n::format_fixed_english(
                    "Cleared {count} expired cache entries ({size}).",
                    &[("{count}", count.as_str()), ("{size}", size.as_str())],
                );
            }
            Err(error) => {
                let error = error.to_string();
                self.last_action = i18n::format_fixed_english(
                    "Cache cleanup failed: {error}",
                    &[("{error}", error.as_str())],
                );
            }
        }
    }

    pub fn clear_music_stream_cache(&mut self) {
        let _guard = match crate::infrastructure::storage_paths::exclusive_cleanup() {
            Ok(guard) => guard,
            Err(error) => {
                self.last_action = error;
                return;
            }
        };
        let root = self.music_stream_cache_root();
        match remove_path_contents_or_dir(&root) {
            Ok(summary) => {
                self.refresh_cache_management_summary();
                let count = summary.entries.to_string();
                let size = format_byte_size(summary.bytes);
                self.last_action = i18n::format_fixed_english(
                    "Cleared audio cache: {count} entries ({size}).",
                    &[("{count}", count.as_str()), ("{size}", size.as_str())],
                );
            }
            Err(error) => {
                let error = error.to_string();
                self.last_action = i18n::format_fixed_english(
                    "Cache cleanup failed: {error}",
                    &[("{error}", error.as_str())],
                );
            }
        }
    }

    pub fn clear_app_cache(&mut self) {
        let _guard = match crate::infrastructure::storage_paths::exclusive_cleanup() {
            Ok(guard) => guard,
            Err(error) => {
                self.last_action = error;
                return;
            }
        };
        let root = self.app_cache_root_path();
        match remove_safe_app_cache_contents(&root) {
            Ok(summary) => {
                self.refresh_cache_management_summary();
                let count = summary.entries.to_string();
                let size = format_byte_size(summary.bytes);
                self.last_action = i18n::format_fixed_english(
                    "Cleared app cache: {count} entries ({size}).",
                    &[("{count}", count.as_str()), ("{size}", size.as_str())],
                );
                if !crate::infrastructure::storage_paths::protected_update_paths().is_empty() {
                    self.last_action
                        .push_str(" Pending update files were kept.");
                }
            }
            Err(error) => {
                let error = error.to_string();
                self.last_action = i18n::format_fixed_english(
                    "Cache cleanup failed: {error}",
                    &[("{error}", error.as_str())],
                );
            }
        }
    }

    pub(super) fn app_cache_root_path(&self) -> PathBuf {
        crate::infrastructure::storage_paths::paths()
            .cache_root
            .clone()
    }
}
