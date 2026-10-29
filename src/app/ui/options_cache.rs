use eframe::egui;
use egui_taffy::Tui;

use crate::app::state::AppState;
use crate::infrastructure::CacheLocationMode;

use super::common::{settings_taffy_form_row, settings_taffy_section};

fn cache_location_mode_label(state: &AppState, mode: CacheLocationMode) -> &'static str {
    match mode {
        CacheLocationMode::V2Cache => state.ui_i18n_text_for_key("options.cache_location.portable"),
        CacheLocationMode::WindowsTemp => {
            state.ui_i18n_text_for_key("options.cache_location.system_temp")
        }
    }
}

pub(super) fn render_cache_group(tui: &mut Tui, state: &mut AppState, label_width: f32) {
    settings_taffy_section(tui, state.ui_i18n_text_for_key("options.cache"), |tui| {
        settings_taffy_form_row(
            tui,
            label_width,
            state.ui_i18n_text_for_key("options.cache_location"),
            |ui| {
                egui::ComboBox::from_id_salt("cache-location-mode")
                    .selected_text(cache_location_mode_label(
                        state,
                        state.saved_cache_location_mode(),
                    ))
                    .show_ui(ui, |ui| {
                        for mode in state.available_cache_location_modes() {
                            let ui_text = cache_location_mode_label(state, mode);
                            let response = ui.selectable_label(
                                state.saved_cache_location_mode() == mode,
                                ui_text,
                            );
                            if response.clicked() {
                                state.set_cache_location_mode(mode);
                            }
                        }
                    });
            },
        );

        settings_taffy_form_row(
            tui,
            label_width,
            state.ui_i18n_text_for_key("options.cache_current_folder"),
            |ui| {
                ui.add(
                    egui::Label::new(state.cache_location_display())
                        .wrap()
                        .selectable(true),
                );
            },
        );
        if let Some(path) = state.pending_cache_location_display() {
            settings_taffy_form_row(
                tui,
                label_width,
                state.ui_i18n_text_for_key("options.cache_after_restart"),
                |ui| {
                    ui.vertical(|ui| {
                        ui.strong(state.ui_i18n_text_for_key("options.cache_restart_required"));
                        ui.add(egui::Label::new(path).wrap().selectable(true));
                    });
                },
            );
        }
        settings_taffy_form_row(tui, label_width, "", |ui| {
            ui.label(state.ui_i18n_text_for_key("options.cache_location_help"));
            if state.config.legacy_cache_notice {
                ui.label(state.ui_i18n_text_for_key("options.cache_legacy_notice"));
            }
            if let Some(error) = state.cache_storage_error() {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        });

        state.refresh_cache_management_summary_if_stale();

        settings_taffy_form_row(
            tui,
            label_width,
            state.ui_i18n_text_for_key("options.cache_usage"),
            |ui| {
                ui.label(state.cache_management_usage_display());
            },
        );

        settings_taffy_form_row(
            tui,
            label_width,
            state.ui_i18n_text_for_key("options.cache_cleanup"),
            |ui| {
                ui.add_enabled_ui(
                    crate::infrastructure::storage_paths::error().is_none(),
                    |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .button(state.ui_i18n_text_for_key("options.cache_refresh"))
                                .clicked()
                            {
                                state.refresh_cache_management_summary();
                            }
                            if ui
                                .button(state.ui_i18n_text_for_key("options.cache_clear_expired"))
                                .clicked()
                            {
                                state.clear_expired_music_cache();
                            }
                            if ui
                                .button(state.ui_i18n_text_for_key("options.cache_clear_audio"))
                                .clicked()
                            {
                                state.clear_music_stream_cache();
                            }
                            if ui
                                .button(state.ui_i18n_text_for_key("options.cache_clear_all"))
                                .clicked()
                            {
                                state.clear_app_cache();
                            }
                        });
                    },
                );
            },
        );
        if !state.last_action.is_empty() {
            // The global footer follows the full-height settings scroll area.
            // Keep existing operation feedback visible beside these controls.
            settings_taffy_form_row(tui, label_width, "", |ui| {
                ui.add(egui::Label::new(state.localize_message(&state.last_action)).wrap());
            });
        }
    });
}
