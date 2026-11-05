use egui_taffy::Tui;

use crate::app::state::AppState;
use crate::app::widgets::url_input::{AppTextBox, AppTextBoxSyntax};
use crate::infrastructure::FileTimeMode;

use super::semantic_ui_metrics;
use super::settings_components::{
    SettingsFormLayout, SettingsSelectOption, deferred_text_box, settings_flexible_form_row,
    settings_section, settings_select_row, settings_toggle_row,
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
            let parallel = state.config.max_concurrent_downloads;
            let parallel_options = (1..=crate::infrastructure::MAX_CONCURRENT_DOWNLOADS)
                .map(|value| SettingsSelectOption::new(value, value.to_string()))
                .collect::<Vec<_>>();
            if let Some(value) = settings_select_row(
                tui,
                form,
                state.ui_i18n_text_for_key("advance.parallel_downloads"),
                "parallel-downloads",
                parallel,
                &parallel.to_string(),
                &parallel_options,
            ) {
                state.set_max_concurrent_downloads(value);
            }
            let retry = state.config.auto_retry_count;
            let retry_label = |state: &AppState, value: usize| {
                if value == 0 {
                    state
                        .ui_i18n_text_for_key("advance.auto_retry_off")
                        .to_owned()
                } else {
                    state.ui_i18n_text_with_replacements(
                        "advance.auto_retry_times",
                        &[("{count}", &value.to_string())],
                    )
                }
            };
            let retry_options = [
                0usize,
                1,
                2,
                3,
                5,
                crate::infrastructure::MAX_AUTO_RETRY_COUNT,
            ]
            .into_iter()
            .map(|value| SettingsSelectOption::new(value, retry_label(state, value)))
            .collect::<Vec<_>>();
            if let Some(value) = settings_select_row(
                tui,
                form,
                state.ui_i18n_text_for_key("advance.auto_retry"),
                "auto-retry-count",
                retry,
                &retry_label(state, retry),
                &retry_options,
            ) {
                state.set_auto_retry_count(value);
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
            settings_flexible_form_row(
                tui,
                form,
                state.ui_i18n_text_for_key("advance.scheduled_rate"),
                |ui| render_scheduled_rate_controls(ui, state),
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

fn render_scheduled_rate_controls(ui: &mut eframe::egui::Ui, state: &mut AppState) {
    let current = state.config.limit_rate_schedule.clone();
    let mut next = current.clone();
    let mut commit = false;
    let language = state.language();
    ui.horizontal_wrapped(|ui| {
        commit |= ui
            .checkbox(
                &mut next.enabled,
                state.ui_i18n_text_for_key("options.enable"),
            )
            .changed();
        let enabled = next.enabled;
        for (salt, value, hint) in [
            ("scheduled-rate-start", &mut next.start, "09:00"),
            ("scheduled-rate-end", &mut next.end, "18:00"),
        ] {
            if let Some(text) = deferred_text_box(
                ui,
                salt,
                value,
                hint,
                language,
                SCHEDULE_TIME_FIELD_WIDTH,
                enabled,
            ) {
                *value = text;
                commit = true;
            }
        }
        ui.label(state.ui_i18n_text_for_key("advance.rate_limit"));
        if let Some(text) = deferred_text_box(
            ui,
            "scheduled-rate-value",
            &current.rate,
            state.ui_i18n_text_for_key("advance.e_g_2m_800k_leave_empty_for_unlimited"),
            language,
            semantic_ui_metrics::advance_form_standard_text_field_width(),
            enabled,
        ) {
            next.rate = text;
            commit = true;
        }
        if enabled && state.active_scheduled_rate_limit().is_some() {
            ui.label(state.ui_i18n_text_for_key("advance.scheduled_rate_active"));
        }
    });
    ui.label(
        eframe::egui::RichText::new(state.ui_i18n_text_for_key("advance.scheduled_rate_note"))
            .weak()
            .small(),
    );
    if commit {
        state.set_limit_rate_schedule(next.normalized());
    }
}

const SCHEDULE_TIME_FIELD_WIDTH: f32 = 52.0;
