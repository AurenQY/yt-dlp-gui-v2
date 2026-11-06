use eframe::egui;
use egui_taffy::Tui;

use crate::app::state::{AdvanceDetailPage, AppState};
use crate::app::widgets::icon::AppIcon;
use crate::infrastructure::{PostProcessMode, SponsorBlockMode};

use super::common::{settings_taffy_form_row, settings_taffy_section, text_trailing_icon_button};

pub(super) fn render_post_processing_section(
    tui: &mut Tui,
    state: &mut AppState,
    label_width: f32,
) {
    settings_taffy_section(
        tui,
        state.ui_i18n_text_for_key("advance.post_processing"),
        |tui| {
            settings_taffy_form_row(
                tui,
                label_width,
                state.ui_i18n_text_for_key("advance.thumbnail"),
                |ui| {
                    post_process_mode_selector(
                        ui,
                        state,
                        state.config.thumbnail_mode,
                        AppState::set_thumbnail_post_process_mode,
                    );
                },
            );
            settings_taffy_form_row(
                tui,
                label_width,
                state.ui_i18n_text_for_key("advance.subtitles"),
                |ui| {
                    post_process_mode_selector(
                        ui,
                        state,
                        state.config.subtitle_mode,
                        AppState::set_subtitle_post_process_mode,
                    );
                },
            );
            settings_taffy_form_row(
                tui,
                label_width,
                state.ui_i18n_text_for_key("advance.chapters"),
                |ui| {
                    post_process_mode_selector(
                        ui,
                        state,
                        state.config.chapter_mode,
                        AppState::set_chapter_post_process_mode,
                    );
                },
            );
            settings_taffy_form_row(
                tui,
                label_width,
                state.ui_i18n_text_for_key("advance.sponsorblock"),
                |ui| render_sponsorblock_controls(ui, state),
            );
            settings_taffy_form_row(
                tui,
                label_width,
                state.ui_i18n_text_for_key("advance.metadata"),
                |ui| {
                    let mut enabled = state.config.embed_metadata;
                    if ui
                        .checkbox(
                            &mut enabled,
                            state.ui_i18n_text_for_key("advance.embed_metadata"),
                        )
                        .changed()
                    {
                        state.set_embed_metadata(enabled);
                    }
                },
            );
            settings_taffy_form_row(
                tui,
                label_width,
                state.ui_i18n_text_for_key("advance.download_conversion"),
                |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let mut enabled = state.config.post_download_conversion_enabled;
                        if ui
                            .checkbox(&mut enabled, state.ui_i18n_text_for_key("advance.enable"))
                            .changed()
                        {
                            state.set_enable_builtin_transcode_after_download(enabled);
                        }

                        if ui
                            .add(text_trailing_icon_button(
                                ui,
                                state.ui_i18n_text_for_key("advance.settings"),
                                AppIcon::MenuRight,
                            ))
                            .clicked()
                        {
                            state.open_advance_detail_page(AdvanceDetailPage::Transcode);
                        }
                    });
                },
            );
        },
    );
}

fn post_process_mode_selector(
    ui: &mut egui::Ui,
    state: &mut AppState,
    current: PostProcessMode,
    set_mode: fn(&mut AppState, PostProcessMode),
) {
    ui.horizontal_wrapped(|ui| {
        for mode in PostProcessMode::variants() {
            let response = ui.selectable_label(
                current == mode,
                state.ui_i18n_text_for_key(mode.label_key()),
            );
            if response.clicked() && current != mode {
                set_mode(state, mode);
            }
        }
    });
}

fn render_sponsorblock_controls(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.horizontal_wrapped(|ui| {
            let current = state.config.sponsorblock_mode;
            for mode in SponsorBlockMode::variants() {
                let response = ui.selectable_label(
                    current == mode,
                    state.ui_i18n_text_for_key(mode.label_key()),
                );
                if response.clicked() && current != mode {
                    state.set_sponsorblock_mode(mode);
                }
            }
        });
        if state.config.sponsorblock_mode != SponsorBlockMode::Off {
            ui.horizontal_wrapped(|ui| {
                ui.label(state.ui_i18n_text_for_key("advance.sponsorblock_categories"));
                let current = state.config.sponsorblock_categories.clone();
                if let Some(categories) = super::settings_components::deferred_text_box(
                    ui,
                    crate::app::state::DeferredSetting::SponsorBlockCategories,
                    &current,
                    crate::infrastructure::DEFAULT_SPONSORBLOCK_CATEGORIES,
                    state.language(),
                    super::semantic_ui_metrics::advance_form_standard_text_field_width(),
                    true,
                ) {
                    state.set_sponsorblock_categories(&categories);
                }
            });
        }
    });
}
