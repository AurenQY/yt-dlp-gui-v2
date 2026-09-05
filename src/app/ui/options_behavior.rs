use eframe::egui;
use egui_taffy::Tui;

use crate::app::state::AppState;
use crate::infrastructure::{OutputFileActionMode, YoutubeVideoPlaylistMode};

use super::settings_components::{
    SettingsFormLayout, SettingsSelectOption, settings_flexible_form_row, settings_section,
    settings_select_row, settings_toggle_row,
};

pub(super) fn render_behavior_group(tui: &mut Tui, state: &mut AppState, form: SettingsFormLayout) {
    settings_section(tui, state.ui_i18n_text_for_key("options.behavior"), |tui| {
        if let Some(enabled) = settings_toggle_row(
            tui,
            form,
            state.ui_i18n_text_for_key("options.add_action"),
            state.config.direct_download_on_add,
            state.ui_i18n_text_for_key("options.download_directly"),
        ) {
            state.set_direct_download_on_add(enabled);
        }
        if let Some(enabled) = settings_toggle_row(
            tui,
            form,
            state.ui_i18n_text_for_key("options.clipboard_change"),
            state.config.clipboard_auto_add,
            state.ui_i18n_text_for_key("options.run_immediately"),
        ) {
            state.set_clipboard_auto_add(enabled);
        }
    });
}

pub(super) fn render_tabs_group(tui: &mut Tui, state: &mut AppState, form: SettingsFormLayout) {
    settings_section(tui, state.ui_i18n_text_for_key("options.tabs"), |tui| {
        if let Some(enabled) = settings_toggle_row(
            tui,
            form,
            state.ui_i18n_text_for_key("options.log_tab"),
            state.config.show_log_tab,
            state.ui_i18n_text_for_key("options.show_log_tab"),
        ) {
            state.set_show_log_tab(enabled);
        }
    });
}

pub(super) fn render_playlist_group(tui: &mut Tui, state: &mut AppState, form: SettingsFormLayout) {
    settings_section(
        tui,
        state.ui_i18n_text_for_key("options.playlist_2"),
        |tui| {
            let playlist_mode = state.config.youtube_video_playlist_mode;
            let playlist_options = [
                SettingsSelectOption::new(
                    YoutubeVideoPlaylistMode::Ask,
                    state.ui_i18n_text_for_key("options.ask"),
                ),
                SettingsSelectOption::new(
                    YoutubeVideoPlaylistMode::Video,
                    state.ui_i18n_text_for_key("options.single_video"),
                ),
                SettingsSelectOption::new(
                    YoutubeVideoPlaylistMode::Ignore,
                    state.ui_i18n_text_for_key("options.full_playlist"),
                ),
            ];
            let selected_text = match playlist_mode {
                YoutubeVideoPlaylistMode::Ask => state.ui_i18n_text_for_key("options.ask"),
                YoutubeVideoPlaylistMode::Video => {
                    state.ui_i18n_text_for_key("options.single_video")
                }
                YoutubeVideoPlaylistMode::Ignore => {
                    state.ui_i18n_text_for_key("options.full_playlist")
                }
            };
            if let Some(mode) = settings_select_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.with_playlist"),
                "youtube-video-playlist-mode",
                playlist_mode,
                selected_text,
                &playlist_options,
            ) {
                state.set_youtube_video_playlist_mode(mode);
            }
            if let Some(enabled) = settings_toggle_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.high_risk_prompt"),
                state.config.youtube_high_risk_playlist_prompt,
                state.ui_i18n_text_for_key("options.on"),
            ) {
                state.set_youtube_high_risk_playlist_prompt(enabled);
            }
            settings_flexible_form_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.playlist_count"),
                |ui| {
                    let mut enabled = state.config.batch_limit_enabled;
                    if ui
                        .checkbox(&mut enabled, state.ui_i18n_text_for_key("options.limit"))
                        .changed()
                    {
                        state.set_batch_limit_enabled(enabled);
                    }
                    let mut count = state.config.batch_limit_count;
                    if ui
                        .add(
                            egui::DragValue::new(&mut count)
                                .range(1..=9999)
                                .prefix(state.ui_i18n_text_for_key("options.max"))
                                .suffix(state.ui_i18n_text_for_key("options.items")),
                        )
                        .changed()
                    {
                        state.set_batch_limit_count(count);
                    }
                },
            );
        },
    );
}

pub(super) fn render_file_action_group(
    tui: &mut Tui,
    state: &mut AppState,
    form: SettingsFormLayout,
) {
    settings_section(
        tui,
        state.ui_i18n_text_for_key("options.file_actions"),
        |tui| {
            let current = state.config.output_file_action_mode;
            let options = OutputFileActionMode::variants().map(|mode| {
                SettingsSelectOption::new(
                    mode,
                    state.ui_i18n_text_for_key(output_file_action_mode_label_key(mode)),
                )
            });
            let selected_text =
                state.ui_i18n_text_for_key(output_file_action_mode_label_key(current));
            if let Some(mode) = settings_select_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.action_button"),
                "output-file-action-mode",
                current,
                selected_text,
                &options,
            ) {
                state.set_output_file_action_mode(mode);
            }
        },
    );
}

fn output_file_action_mode_label_key(mode: OutputFileActionMode) -> &'static str {
    match mode {
        OutputFileActionMode::Menu => "options.file_action.show_menu",
        OutputFileActionMode::OpenFolder => "item.open_folder",
        OutputFileActionMode::OpenFile => "item.open_file",
    }
}
