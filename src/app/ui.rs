// Shared shell and layout helpers.
mod common;
mod measure;
mod semantic_ui_metrics;
mod settings_components;
mod settings_detail_template;
mod titlebar;

// Generic views and reusable page widgets.
mod about_tab;
mod about_tab_controls;
mod about_tab_template;
mod about_tool_version;
mod batch_list_template;
mod compact_row;
mod format_picker;
mod format_picker_content;
mod format_picker_filters;
mod format_picker_header;
mod format_picker_section;
mod format_picker_selection;
mod format_picker_subtitle;
mod format_picker_table;
mod format_picker_template;
mod format_picker_time_range;
mod gallery_range_filter;
mod gallery_range_picker;
mod gallery_range_text;
mod item_card;
mod item_card_compact;
mod item_card_gallery;
mod item_card_output_actions;
mod item_card_template;
mod item_card_thumbnail;
mod prepare_tab;
mod prepare_tab_template;
mod single_mode;
mod single_mode_format_rows;
mod single_mode_gallery;
mod single_mode_preview;
mod single_mode_template;

// Processing / log viewer.
mod processing_command_viewer;
mod processing_conversion;
mod processing_conversion_template;
mod processing_log_table;
mod processing_log_viewer;
mod processing_tab;
mod processing_tab_template;

// Main tab template and role controls.
mod main_tab;
mod main_tab_controls;
mod main_tab_dependency_notice;
mod main_tab_monitor_control;
mod main_tab_music_aura;
mod main_tab_music_aura_dynamics;
mod main_tab_music_controls;
mod main_tab_music_lyrics;
mod main_tab_music_panel;
mod main_tab_music_stage_controls;
mod main_tab_output_actions;
mod main_tab_template;
mod main_tab_text_fields;
mod main_tab_url_action;

// Options tab sections.
mod options_behavior;
mod options_cache;
mod options_formats;
mod options_language;
mod options_layout;
mod options_prompts;
mod options_tab;
mod options_tab_template;
mod options_window;

// Advance tab sections.
mod advance_command_previews;
mod advance_conversion;
mod advance_conversion_template;
mod advance_cookie_manager;
mod advance_cookie_manager_template;
mod advance_cookie_rescue;
mod advance_download_controls;
mod advance_network;
mod advance_post_processing;
mod advance_source;
mod advance_tab;
mod advance_tab_template;
mod options_browser_integration;

// XAML-like template layer.
mod xaml_layout_contracts;
mod xaml_rect_template;
mod xaml_taffy_styles;
mod xaml_template_renderer;
mod xaml_ui_nodes;

use std::cell::Cell;

use eframe::egui::{self, CentralPanel, Ui};

use crate::app::state::{AppState, AppTab};

pub(crate) struct UiRenderResources {
    music_player_aura: Option<main_tab_music_aura::MusicPlayerAuraRenderer>,
    music_aura_presentation: Cell<main_tab_music_aura::MusicAuraPresentation>,
    music_window_material_progress: Cell<f32>,
}

impl UiRenderResources {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let music_player_aura = main_tab_music_aura::MusicPlayerAuraRenderer::new(cc)
            .map_err(|error| {
                crate::infrastructure::record_startup_error("music-aura", &error);
                eprintln!("[music-aura] GPU renderer unavailable: {error}");
            })
            .ok();
        crate::infrastructure::record_startup_event(
            "music-aura",
            format!("available={}", music_player_aura.is_some()),
        );
        Self {
            music_player_aura,
            music_aura_presentation: Cell::new(Default::default()),
            music_window_material_progress: Cell::new(0.0),
        }
    }

    fn music_player_aura(&self) -> Option<main_tab_music_aura::MusicPlayerAuraRenderer> {
        self.music_player_aura.clone()
    }

    fn toggle_music_aura_presentation(&self) {
        let presentation = self.music_aura_presentation.get().toggled();
        self.music_aura_presentation.set(presentation);
        eprintln!("[music-aura] presentation: {presentation:?}");
    }

    fn music_window_ambient_enabled(&self) -> bool {
        self.music_player_aura.is_some()
            && self.music_aura_presentation.get().window_layer_enabled()
    }

    fn music_aura_presentation(&self) -> main_tab_music_aura::MusicAuraPresentation {
        self.music_aura_presentation.get()
    }

    fn music_window_material_progress(&self) -> f32 {
        self.music_window_material_progress.get()
    }

    pub(crate) fn destroy(&mut self, gl: Option<&eframe::glow::Context>) {
        if let (Some(renderer), Some(gl)) = (self.music_player_aura.take(), gl) {
            renderer.destroy(gl);
        }
    }
}

pub fn render_app(root_ui: &mut Ui, state: &mut AppState, render_resources: &UiRenderResources) {
    handle_hidden_music_aura_shortcut(root_ui, render_resources);

    let prompt_open = state.youtube_playlist_prompt.is_some()
        || state.gallery_backend_prompt_open()
        || state.music_download_prompt_open()
        || state.youtube_login_rescue_dialog_visible();
    if state.active_tab == AppTab::Prepare && !state.should_show_prepare_tab() {
        state.active_tab = AppTab::Main;
    }

    let panel_fill = root_ui.visuals().panel_fill;
    let window_ambient_visible =
        music_window_ambient_visible(state, render_resources.music_window_ambient_enabled());
    let material_progress = main_tab_music_aura::music_window_material_progress(
        root_ui,
        window_ambient_visible
            && render_resources
                .music_aura_presentation()
                .background_glass_enabled(),
        state.music_player_is_playing(),
    );
    render_resources
        .music_window_material_progress
        .set(material_progress);

    CentralPanel::default()
        .frame(egui::Frame::NONE.fill(panel_fill))
        .show(root_ui, |ui| {
            if window_ambient_visible {
                render_music_window_ambient(ui, state, render_resources);
            }
            titlebar::render_titlebar(ui, state, window_ambient_visible);

            let content_frame = egui::Frame::NONE.inner_margin(egui::Margin {
                left: 8,
                right: 8,
                top: 4,
                bottom: 6,
            });
            let content_frame = if window_ambient_visible {
                content_frame
            } else {
                content_frame.fill(panel_fill)
            };
            content_frame.show(ui, |ui| {
                ui.add_enabled_ui(!prompt_open, |ui| {
                    if state.gallery_range_picker.item_id.is_some() {
                        gallery_range_picker::render(ui, state);
                        return;
                    }
                    if state.format_picker.open {
                        format_picker::render_format_picker_screen(ui, state);
                        return;
                    }

                    if state.should_show_prepare_tab() {
                        match state.active_tab {
                            AppTab::Options => options_tab::render_options_tab(ui, state),
                            AppTab::About => about_tab::render_about_tab(ui, state),
                            AppTab::Log if state.config.show_log_tab => {
                                processing_tab::render_log_tab(ui, state);
                            }
                            _ => {
                                state.active_tab = AppTab::Prepare;
                                prepare_tab::render_prepare_tab(ui, state);
                            }
                        }
                    } else {
                        match state.active_tab {
                            AppTab::Prepare => {
                                main_tab::render_main_tab(ui, state, render_resources)
                            }
                            AppTab::Main => main_tab::render_main_tab(ui, state, render_resources),
                            AppTab::Advance => advance_tab::render_advance_tab(ui, state),
                            AppTab::Options => options_tab::render_options_tab(ui, state),
                            AppTab::About => about_tab::render_about_tab(ui, state),
                            AppTab::Log => {
                                if state.config.show_log_tab {
                                    processing_tab::render_log_tab(ui, state);
                                } else {
                                    state.active_tab = AppTab::Options;
                                    options_tab::render_options_tab(ui, state);
                                }
                            }
                        }
                    }

                    if !state.last_action.is_empty()
                        && state.active_tab != AppTab::About
                        && !state.should_show_prepare_tab()
                    {
                        ui.separator();
                        ui.small(state.localize_message(&state.last_action));
                    }
                });
            });
        });

    options_prompts::render_youtube_playlist_prompt(root_ui.ctx(), state);
    options_prompts::render_gallery_downloader_choice_prompt(root_ui.ctx(), state);
    options_prompts::render_gallery_backend_prompt(root_ui.ctx(), state);
    options_prompts::render_music_download_prompt(root_ui.ctx(), state);
    advance_cookie_rescue::render_youtube_login_rescue_dialog(root_ui.ctx(), state);
}

fn handle_hidden_music_aura_shortcut(ui: &Ui, render_resources: &UiRenderResources) {
    let modifiers = egui::Modifiers {
        ctrl: true,
        shift: true,
        ..Default::default()
    };
    if ui
        .ctx()
        .input_mut(|input| input.consume_key(modifiers, egui::Key::F10))
    {
        render_resources.toggle_music_aura_presentation();
        ui.ctx().request_repaint();
    }
}

fn render_music_window_ambient(ui: &Ui, state: &AppState, render_resources: &UiRenderResources) {
    main_tab_music_aura::render_music_window_ambient_at(
        ui,
        ui.max_rect(),
        render_resources.music_player_aura().as_ref(),
        state.music_player_aura_display(),
        render_resources.music_window_material_progress(),
    );
}

fn music_window_ambient_visible(state: &AppState, presentation_enabled: bool) -> bool {
    presentation_enabled
        && !state.format_picker.open
        && !state.should_show_prepare_tab()
        && state.music_player_visible()
        && matches!(state.active_tab, AppTab::Prepare | AppTab::Main)
}
