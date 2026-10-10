use egui_taffy::Tui;

use crate::app::state::AppState;
use crate::app::widgets::url_input::{AppTextBox, AppTextBoxSyntax};
use crate::infrastructure::FileTimeMode;

use super::semantic_ui_metrics;
use super::settings_components::{
    SettingsFormLayout, SettingsSelectOption, settings_flexible_form_row, settings_section,
    settings_select_row, settings_toggle_row,
};

pub(super) fn render_aria2_section(tui: &mut Tui, state: &mut AppState, form: SettingsFormLayout) {
    settings_section(tui, "Aria2", |tui| {
        if let Some(enabled) = settings_toggle_row(
            tui,
            form,
            state.ui_i18n_text_for_key("advance.external_downloader"),
            state.item_defaults.use_aria2,
            state.ui_i18n_text_for_key("advance.use_aria2_external_downloader"),
        ) {
            state.set_use_aria2(enabled);
        }
    });
}
pub(super) fn render_download_processing_section(
    tui: &mut Tui,
    state: &mut AppState,
    form: SettingsFormLayout,
) {
    settings_section(
        tui,
        state.ui_i18n_text_for_key("advance.download_control"),
        |tui| {
            let selected = state.tool_paths.concurrent_fragments;
            let fragment_options = state
                .available_concurrent_fragment_values()
                .into_iter()
                .map(|value| {
                    let label = if value == 1 {
                        state.ui_i18n_text_for_key("advance.1_default").to_owned()
                    } else {
                        value.to_string()
                    };
                    SettingsSelectOption::new(value, label)
                })
                .collect::<Vec<_>>();
            if let Some(value) = settings_select_row(
                tui,
                form,
                state.ui_i18n_text_for_key("advance.concurrent_fragments"),
                "concurrent-fragments",
                selected,
                &selected.max(1).to_string(),
                &fragment_options,
            ) {
                state.set_concurrent_fragments(value);
            }
            if let Some(enabled) = settings_toggle_row(
                tui,
                form,
                state.ui_i18n_text_for_key("advance.live_streams"),
                state.config.live_from_start,
                state.ui_i18n_text_for_key("advance.download_live_streams_from_start_experimental"),
            ) {
                state.set_live_from_start(enabled);
            }
            settings_flexible_form_row(
                tui,
                form,
                state.ui_i18n_text_for_key("advance.rate_limit"),
                |ui| {
                    let mut limit_rate = state.tool_paths.limit_rate.clone();
                    if AppTextBox::new(&mut limit_rate)
                .hint_text(state.ui_i18n_text_for_key("advance.e_g_2m_800k_leave_empty_for_unlimited"))
                .language(state.language())
                .syntax(AppTextBoxSyntax::Plain)
                .desired_width(semantic_ui_metrics::advance_form_standard_text_field_width())
                .ui(ui)
                .changed()
            {
                state.set_limit_rate(limit_rate);
            }
                },
            );
            if let Some(enabled) = settings_toggle_row(
                tui,
                form,
                state.ui_i18n_text_for_key("advance.chapters"),
                state.tool_paths.chapter_compatibility_mode,
                state.ui_i18n_text_for_key("advance.chapter_download_compatibility_mode"),
            ) {
                state.set_chapter_compatibility_mode(enabled);
            }
            if let Some(enabled) = settings_toggle_row(
                tui,
                form,
                state.ui_i18n_text_for_key("advance.download_range"),
                state.config.always_show_download_range,
                state.ui_i18n_text_for_key("advance.always_show_download_range"),
            ) {
                state.set_always_show_download_range(enabled);
            }

            let file_time = state.tool_paths.file_time_mode;
            let file_time_options = [
                FileTimeMode::None,
                FileTimeMode::UseUploadDate,
                FileTimeMode::UseDownloadTime,
            ]
            .map(|mode| {
                SettingsSelectOption::new(mode, state.ui_i18n_text_for_key(mode.label_key()))
            });
            if let Some(mode) = settings_select_row(
                tui,
                form,
                state.ui_i18n_text_for_key("advance.file_time"),
                "file-time-mode",
                file_time,
                state.ui_i18n_text_for_key(file_time.label_key()),
                &file_time_options,
            ) {
                state.set_file_time_mode(mode);
            }
        },
    );
}
