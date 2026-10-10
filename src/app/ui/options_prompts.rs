use eframe::egui::{self, Color32, RichText, Ui};

use crate::app::state::{
    AppState, GalleryDownloaderChoice, MusicDownloadFormat, MusicDownloadMode,
    MusicDownloadSaveMode, MusicOriginalPreference, YoutubePlaylistPromptKind,
};
use crate::app::widgets::icon::{AppIcon, icon_image};
use crate::app::widgets::url_input::{accent_green_for_ui, accent_red_for_ui};

use super::semantic_ui_metrics;

#[path = "gallery_downloader_choice_actions.rs"]
mod gallery_downloader_choice_actions;

pub(super) fn render_music_download_prompt(ctx: &egui::Context, state: &mut AppState) {
    if !state.music_download_prompt_open() {
        return;
    }

    egui::Window::new(state.ui_i18n_text_for_key("options.music_download_format"))
        .id(egui::Id::new("music-download-format-prompt-window-v4"))
        .collapsible(false)
        .resizable(false)
        .anchor(
            egui::Align2::CENTER_CENTER,
            semantic_ui_metrics::modal_prompt_center_anchor_vector(),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing =
                semantic_ui_metrics::music_download_prompt_item_spacing();
            let prompt_width = music_download_prompt_content_width(ctx, ui, state);
            ui.set_width(prompt_width);
            ui.set_max_width(prompt_width);

            render_music_download_preference_panel(ui, state, prompt_width);

            ui.add_space(
                semantic_ui_metrics::music_download_prompt_action_to_panel_vertical_spacing(),
            );
            render_music_download_prompt_actions(ui, state, prompt_width);
        });
}

fn render_music_download_preference_panel(ui: &mut Ui, state: &mut AppState, prompt_width: f32) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y =
            semantic_ui_metrics::music_download_prompt_preference_panel_vertical_spacing();
        ui.label(
            RichText::new(state.ui_i18n_text_for_key("options.music_download_audio_label"))
                .strong()
                .size(semantic_ui_metrics::playlist_prompt_body_text_size()),
        );
        render_music_download_preference_chips(ui, state, prompt_width);
        ui.label(
            RichText::new(state.ui_i18n_text_for_key("options.music_download_save_label"))
                .strong()
                .size(semantic_ui_metrics::playlist_prompt_body_text_size()),
        );
        render_music_download_save_chips(ui, state, prompt_width);
    });
}

fn render_music_download_preference_chips(ui: &mut Ui, state: &mut AppState, prompt_width: f32) {
    let choice = state.music_download_prompt_choice();
    let chip_items = MusicDownloadPromptOption::ALL
        .into_iter()
        .map(|option| {
            let label = option.label(state).to_owned();
            let width =
                semantic_ui_metrics::music_download_prompt_choice_chip_width_for_visible_label(
                    ui, &label,
                );
            (option, label, width)
        })
        .collect::<Vec<_>>();
    let chip_spacing = semantic_ui_metrics::music_download_prompt_choice_chip_horizontal_spacing();
    let total_width = chip_items.iter().map(|(_, _, width)| *width).sum::<f32>()
        + chip_spacing * chip_items.len().saturating_sub(1) as f32;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = chip_spacing;
        ui.add_space(
            semantic_ui_metrics::remaining_width_before_centered_prompt_content(
                prompt_width,
                total_width,
            ),
        );
        for (option, label, width) in chip_items {
            if music_prompt_choice_chip(ui, &label, option.is_selected(choice), width).clicked() {
                option.apply(state);
                state.set_music_download_embed_cover(true);
                state.set_music_download_write_tags(true);
            }
        }
    });
}

fn render_music_download_save_chips(ui: &mut Ui, state: &mut AppState, prompt_width: f32) {
    let choice = state.music_download_prompt_choice();
    let chip_items = MusicDownloadSaveMode::ALL
        .into_iter()
        .map(|mode| {
            let label = music_download_save_mode_label(state, mode).to_owned();
            let width =
                semantic_ui_metrics::music_download_prompt_save_chip_width_for_visible_label(
                    ui, &label,
                );
            (mode, label, width)
        })
        .collect::<Vec<_>>();
    let chip_spacing = semantic_ui_metrics::music_download_prompt_choice_chip_horizontal_spacing();
    let total_width = chip_items.iter().map(|(_, _, width)| *width).sum::<f32>()
        + chip_spacing * chip_items.len().saturating_sub(1) as f32;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = chip_spacing;
        ui.add_space(
            semantic_ui_metrics::remaining_width_before_centered_prompt_content(
                prompt_width,
                total_width,
            ),
        );
        for (mode, label, width) in chip_items {
            if music_prompt_choice_chip(ui, &label, choice.save_mode == mode, width).clicked() {
                state.set_music_download_save_mode(mode);
            }
        }
    });
}

fn music_download_save_mode_label(state: &AppState, mode: MusicDownloadSaveMode) -> &str {
    match mode {
        MusicDownloadSaveMode::FullAudio => {
            state.ui_i18n_text_for_key("options.music_download_full_audio")
        }
        MusicDownloadSaveMode::SplitChapters => {
            state.ui_i18n_text_for_key("options.music_download_split_chapters")
        }
    }
}

fn render_music_download_prompt_actions(ui: &mut Ui, state: &mut AppState, prompt_width: f32) {
    let cancel_label = state.ui_i18n_text_for_key("options.cancel").to_owned();
    let download_label = state.ui_i18n_text_for_key("action.download").to_owned();
    let cancel_width = semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(
        ui,
        &cancel_label,
    );
    let download_width = semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(
        ui,
        &download_label,
    );
    let action_spacing =
        semantic_ui_metrics::music_download_prompt_action_button_horizontal_spacing();
    let total_width = cancel_width + download_width + action_spacing;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = action_spacing;
        ui.add_space(
            semantic_ui_metrics::remaining_width_before_right_aligned_prompt_actions(
                prompt_width,
                total_width,
            ),
        );

        let cancel_button = prompt_action_button(
            ui,
            AppIcon::WindowClose,
            accent_red_for_ui(ui),
            &cancel_label,
            cancel_width,
            semantic_ui_metrics::prompt_action_button_height_for_playlist_decision(),
        );
        if ui.add(cancel_button).clicked() {
            state.cancel_music_download_prompt();
        }

        let download_button = prompt_action_button(
            ui,
            AppIcon::Download,
            accent_green_for_ui(ui),
            &download_label,
            download_width,
            semantic_ui_metrics::prompt_action_button_height_for_playlist_decision(),
        );
        if ui.add(download_button).clicked() {
            state.confirm_music_download_choice();
        }
    });
}

fn music_download_prompt_content_width(ctx: &egui::Context, ui: &Ui, state: &AppState) -> f32 {
    let audio_label_text = state.ui_i18n_text_for_key("options.music_download_audio_label");
    let save_label_text = state.ui_i18n_text_for_key("options.music_download_save_label");
    let title_width = [audio_label_text, save_label_text]
        .into_iter()
        .map(|label| semantic_ui_metrics::music_download_prompt_section_label_width(ui, label))
        .fold(0.0_f32, f32::max);
    let preference_labels = MusicDownloadPromptOption::ALL
        .into_iter()
        .map(|option| option.label(state).to_owned())
        .collect::<Vec<_>>();
    let chip_spacing = semantic_ui_metrics::music_download_prompt_choice_chip_horizontal_spacing();
    let chips_width = preference_labels
        .iter()
        .map(|label| {
            semantic_ui_metrics::music_download_prompt_choice_chip_width_for_visible_label(
                ui, label,
            )
        })
        .sum::<f32>()
        + chip_spacing * preference_labels.len().saturating_sub(1) as f32;
    let save_chips_width = MusicDownloadSaveMode::ALL
        .into_iter()
        .map(|mode| {
            semantic_ui_metrics::music_download_prompt_save_chip_width_for_visible_label(
                ui,
                music_download_save_mode_label(state, mode),
            )
        })
        .sum::<f32>()
        + chip_spacing * MusicDownloadSaveMode::ALL.len().saturating_sub(1) as f32;
    let cancel_text = state.ui_i18n_text_for_key("options.cancel");
    let download_text = state.ui_i18n_text_for_key("action.download");
    let action_width =
        semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(ui, cancel_text)
            + semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(
                ui,
                download_text,
            )
            + semantic_ui_metrics::music_download_prompt_action_button_horizontal_spacing();
    let content_width = title_width
        .max(chips_width)
        .max(save_chips_width)
        .max(action_width);
    let max_prompt_width = semantic_ui_metrics::music_download_prompt_maximum_width_for_viewport(
        ctx.content_rect().width(),
    );
    semantic_ui_metrics::music_download_prompt_width_from_content_width(
        content_width,
        max_prompt_width,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MusicDownloadPromptOption {
    OriginalBest,
    Unified(MusicDownloadFormat),
}

impl MusicDownloadPromptOption {
    const ALL: [Self; 4] = [
        Self::OriginalBest,
        Self::Unified(MusicDownloadFormat::Opus),
        Self::Unified(MusicDownloadFormat::M4aAac),
        Self::Unified(MusicDownloadFormat::Mp3),
    ];

    fn label(self, state: &AppState) -> &str {
        match self {
            Self::OriginalBest => {
                state.ui_i18n_text_for_key("options.music_download_preference_best")
            }
            Self::Unified(format) => format.label(),
        }
    }

    fn is_selected(self, choice: crate::app::state::MusicDownloadChoice) -> bool {
        match self {
            Self::OriginalBest => choice.mode == MusicDownloadMode::Original,
            Self::Unified(format) => {
                choice.mode == MusicDownloadMode::Unified && choice.unified_format == format
            }
        }
    }

    fn apply(self, state: &mut AppState) {
        match self {
            Self::OriginalBest => {
                state.set_music_download_prompt_mode(MusicDownloadMode::Original);
                state.set_music_download_original_preference(MusicOriginalPreference::Auto);
            }
            Self::Unified(format) => {
                state.set_music_download_prompt_mode(MusicDownloadMode::Unified);
                state.set_music_download_unified_format(format);
            }
        }
    }
}

fn music_prompt_choice_chip(
    ui: &mut Ui,
    label: &str,
    selected: bool,
    width: f32,
) -> egui::Response {
    let desired_size = egui::vec2(
        width,
        semantic_ui_metrics::music_download_prompt_choice_chip_height(),
    );
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.visuals();
        let fill = if selected {
            visuals.selection.bg_fill
        } else if response.hovered() {
            visuals.widgets.hovered.weak_bg_fill
        } else {
            Color32::TRANSPARENT
        };
        let stroke = if selected {
            egui::Stroke::new(
                semantic_ui_metrics::playlist_prompt_choice_selected_stroke_width(),
                visuals.selection.stroke.color,
            )
        } else if response.hovered() {
            visuals.widgets.hovered.bg_stroke
        } else {
            visuals.widgets.noninteractive.bg_stroke
        };
        let text_color = if selected {
            visuals.selection.stroke.color
        } else {
            visuals.text_color()
        };
        ui.painter().rect_filled(
            rect,
            semantic_ui_metrics::playlist_prompt_choice_corner_radius(),
            fill,
        );
        ui.painter().rect_stroke(
            rect,
            semantic_ui_metrics::playlist_prompt_choice_corner_radius(),
            stroke,
            egui::StrokeKind::Outside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::TextStyle::Button.resolve(ui.style()),
            text_color,
        );
    }

    response
}

pub(super) fn render_youtube_playlist_prompt(ctx: &egui::Context, state: &mut AppState) {
    let Some(prompt) = state.youtube_playlist_prompt.as_ref() else {
        return;
    };
    let prompt_kind = prompt.kind;
    let prompt_risk = prompt.risk;

    match prompt_kind {
        YoutubePlaylistPromptKind::VideoAndPlaylist => {
            let title = state
                .ui_i18n_text_for_key("options.this_url_contains_both_a_video_and_a_playlis")
                .to_owned();
            render_playlist_prompt_window(
                ctx,
                state,
                title,
                None,
                YoutubePlaylistPromptKind::VideoAndPlaylist,
            );
        }
        YoutubePlaylistPromptKind::GenericPlaylist => {
            render_playlist_prompt_window(
                ctx,
                state,
                state.ui_i18n_text_for_key("options.playlist").to_owned(),
                None,
                YoutubePlaylistPromptKind::GenericPlaylist,
            );
        }
        YoutubePlaylistPromptKind::HighRiskPlaylist => {
            let risk = prompt_risk.expect("high risk prompt should include risk");
            let title = format!(
                "{}{}",
                state.ui_i18n_text_for_key("options.detected"),
                state.ui_i18n_text_for_key(risk.kind.label_key())
            );
            render_playlist_prompt_window(
                ctx,
                state,
                title,
                Some(risk.kind.note_key()),
                YoutubePlaylistPromptKind::HighRiskPlaylist,
            );
        }
    }
}

fn render_playlist_prompt_window(
    ctx: &egui::Context,
    state: &mut AppState,
    title: String,
    risk_note_key: Option<&'static str>,
    prompt_kind: YoutubePlaylistPromptKind,
) {
    let caption = state
        .ui_i18n_text_for_key("options.playlist_prompt")
        .to_owned();
    render_shared_choice_window(ctx, state, &caption, |ui, state| {
        render_playlist_prompt_body(ui, state, &title, risk_note_key, prompt_kind);
        ui.add_space(semantic_ui_metrics::playlist_prompt_actions_to_body_vertical_spacing());
        render_playlist_prompt_actions(ui, state, prompt_kind);
    });
}

// Both the existing video/playlist choice and video/images choice use the
// same dialog shell, size, alignment, spacing and action-button components.
fn render_shared_choice_window(
    ctx: &egui::Context,
    state: &mut AppState,
    caption: &str,
    content: impl FnOnce(&mut Ui, &mut AppState),
) {
    egui::Window::new(caption)
        .id(egui::Id::new("youtube-playlist-prompt-window-fit-v5"))
        .collapsible(false)
        .resizable(false)
        .anchor(
            egui::Align2::CENTER_CENTER,
            semantic_ui_metrics::modal_prompt_center_anchor_vector(),
        )
        .show(ctx, |ui| {
            let prompt_width = semantic_ui_metrics::playlist_prompt_content_width();
            ui.set_width(prompt_width);
            ui.set_max_width(prompt_width);
            ui.spacing_mut().item_spacing = semantic_ui_metrics::playlist_prompt_item_spacing();
            ui.spacing_mut().button_padding = semantic_ui_metrics::playlist_prompt_button_padding();

            content(ui, state);
        });
}

pub(super) fn render_gallery_downloader_choice_prompt(ctx: &egui::Context, state: &mut AppState) {
    if !state.gallery_downloader_choice_open() {
        return;
    }
    render_shared_choice_window(
        ctx,
        state,
        crate::i18n::text(state.language(), "gallery.choice.downloader"),
        |ui, state| {
            if !state.gallery_downloader_multiple_allowed() {
                gallery_downloader_choice_actions::render(ui, state);
                return;
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y =
                    semantic_ui_metrics::playlist_prompt_body_vertical_spacing();
                ui.label(
                    RichText::new(crate::i18n::text(
                        state.language(),
                        "gallery.choice.dual_hint",
                    ))
                    .strong()
                    .size(semantic_ui_metrics::playlist_prompt_title_text_size()),
                );
                let selected = state.gallery_downloader_choice();
                let mut yt_dlp = matches!(
                    selected,
                    Some(GalleryDownloaderChoice::YtDlp | GalleryDownloaderChoice::Both)
                );
                let mut gallery_dl = matches!(
                    selected,
                    Some(GalleryDownloaderChoice::GalleryDl | GalleryDownloaderChoice::Both)
                );
                if ui.checkbox(&mut yt_dlp, "yt-dlp").changed() {
                    state.set_gallery_downloader_checks(yt_dlp, gallery_dl);
                }
                if ui.checkbox(&mut gallery_dl, "gallery-dl").changed() {
                    state.set_gallery_downloader_checks(yt_dlp, gallery_dl);
                }
            });
            ui.add_space(semantic_ui_metrics::playlist_prompt_actions_to_body_vertical_spacing());
            let confirm_text = state
                .ui_i18n_text_for_key(state.gallery_downloader_choice_action_label_key())
                .to_owned();
            let cancel_text = state.ui_i18n_text_for_key("options.cancel").to_owned();
            let button_height =
                semantic_ui_metrics::prompt_action_button_height_for_playlist_decision();
            let spacing = semantic_ui_metrics::playlist_prompt_action_button_horizontal_spacing();
            let confirm_width =
                semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(
                    ui,
                    &confirm_text,
                );
            let cancel_width =
                semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(
                    ui,
                    &cancel_text,
                );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = spacing;
                ui.add_space(
                    semantic_ui_metrics::remaining_width_before_right_aligned_prompt_actions(
                        semantic_ui_metrics::playlist_prompt_content_width(),
                        confirm_width + cancel_width + spacing,
                    ),
                );
                let cancel = prompt_action_button(
                    ui,
                    AppIcon::WindowClose,
                    accent_red_for_ui(ui),
                    &cancel_text,
                    cancel_width,
                    button_height,
                );
                if ui.add(cancel).clicked() {
                    state.cancel_gallery_downloader_choice();
                }
                let confirm = prompt_action_button(
                    ui,
                    AppIcon::Import,
                    accent_green_for_ui(ui),
                    &confirm_text,
                    confirm_width,
                    button_height,
                );
                if ui
                    .add_enabled(state.gallery_downloader_choice().is_some(), confirm)
                    .clicked()
                {
                    state.confirm_gallery_downloader_choice();
                }
            });
        },
    );
}

pub(super) fn render_gallery_backend_prompt(ctx: &egui::Context, state: &mut AppState) {
    if !state.gallery_backend_prompt_open() {
        return;
    }
    let has_playlist_scope = state.gallery_route_prompt_has_playlist_scope();
    let is_origin = state.gallery_route_prompt_is_origin();
    let title = if is_origin {
        crate::i18n::text(state.language(), "gallery.choice.content")
    } else {
        crate::i18n::text(state.language(), "gallery.choice.add_title")
    };
    render_shared_choice_window(ctx, state, title, |ui, state| {
        if is_origin {
            render_origin_media_choices(ui, state);
        } else if has_playlist_scope {
            render_playlist_prompt_body(
                ui,
                state,
                "",
                None,
                YoutubePlaylistPromptKind::VideoAndPlaylist,
            );
            render_gallery_image_choice(ui, state);
        } else {
            render_gallery_media_choices(ui, state);
        }
        ui.add_space(semantic_ui_metrics::playlist_prompt_actions_to_body_vertical_spacing());
        if has_playlist_scope && !is_origin {
            render_gallery_playlist_actions(ui, state);
        } else {
            render_gallery_media_actions(ui, state);
        }
    });
}

fn render_origin_media_choices(ui: &mut Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y =
            semantic_ui_metrics::playlist_prompt_body_vertical_spacing();
        ui.label(
            RichText::new(crate::i18n::text(
                state.language(),
                "gallery.choice.origin_hint",
            ))
            .strong()
            .size(semantic_ui_metrics::playlist_prompt_title_text_size()),
        );
        let selected_video = state.gallery_route_include_video();
        if ui
            .radio(
                selected_video,
                crate::i18n::text(state.language(), "media.video"),
            )
            .clicked()
        {
            state.set_gallery_route_include_video(true);
        }
        let label = crate::i18n::format_text(
            state.language(),
            "gallery.media_count",
            &[("{count}", &state.gallery_route_image_count().to_string())],
        );
        if ui.radio(!selected_video, label).clicked() {
            state.set_gallery_route_include_images(true);
        }
    });
}

fn render_gallery_media_choices(ui: &mut Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y =
            semantic_ui_metrics::playlist_prompt_body_vertical_spacing();
        ui.label(
            RichText::new(crate::i18n::text(
                state.language(),
                "gallery.choice.add_hint",
            ))
            .strong()
            .size(semantic_ui_metrics::playlist_prompt_title_text_size()),
        );
        ui.label(
            RichText::new(crate::i18n::text(
                state.language(),
                "gallery.choice.dual_media",
            ))
            .size(semantic_ui_metrics::playlist_prompt_body_text_size()),
        );
        let mut include_video = state.gallery_route_include_video();
        if ui
            .checkbox(
                &mut include_video,
                crate::i18n::text(state.language(), "media.video"),
            )
            .changed()
        {
            state.set_gallery_route_include_video(include_video);
        }
        render_gallery_image_choice(ui, state);
    });
}

fn render_gallery_image_choice(ui: &mut Ui, state: &mut AppState) {
    let mut include_images = state.gallery_route_include_images();
    let label = crate::i18n::format_text(
        state.language(),
        "gallery.media_count",
        &[("{count}", &state.gallery_route_image_count().to_string())],
    );
    if ui.checkbox(&mut include_images, label).changed() {
        state.set_gallery_route_include_images(include_images);
    }
}

fn render_gallery_media_actions(ui: &mut Ui, state: &mut AppState) {
    let add_text = state
        .ui_i18n_text_for_key(if state.gallery_route_prompt_is_origin() {
            "action.analyze"
        } else {
            "action.add"
        })
        .to_owned();
    let cancel_text = state.ui_i18n_text_for_key("options.cancel").to_owned();
    let button_height = semantic_ui_metrics::prompt_action_button_height_for_playlist_decision();
    let spacing = semantic_ui_metrics::playlist_prompt_action_button_horizontal_spacing();
    let add_width =
        semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(ui, &add_text);
    let cancel_width =
        semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(ui, &cancel_text);
    let total_width = add_width + cancel_width + spacing;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = spacing;
        ui.add_space(
            semantic_ui_metrics::remaining_width_before_right_aligned_prompt_actions(
                semantic_ui_metrics::playlist_prompt_content_width(),
                total_width,
            ),
        );
        let cancel_button = prompt_action_button(
            ui,
            AppIcon::WindowClose,
            accent_red_for_ui(ui),
            &cancel_text,
            cancel_width,
            button_height,
        );
        if ui.add(cancel_button).clicked() {
            state.cancel_gallery_route_prompt();
        }
        let add_button = prompt_action_button(
            ui,
            AppIcon::Import,
            accent_green_for_ui(ui),
            &add_text,
            add_width,
            button_height,
        );
        let any_selected =
            state.gallery_route_include_video() || state.gallery_route_include_images();
        if ui.add_enabled(any_selected, add_button).clicked() {
            state.confirm_gallery_route_media();
        }
    });
}

fn render_gallery_playlist_actions(ui: &mut Ui, state: &mut AppState) {
    let video_text = state.ui_i18n_text_for_key("options.video").to_owned();
    let playlist_text = state.ui_i18n_text_for_key("options.playlist").to_owned();
    let cancel_text = state.ui_i18n_text_for_key("options.cancel").to_owned();
    let button_height = semantic_ui_metrics::prompt_action_button_height_for_playlist_decision();
    let spacing = semantic_ui_metrics::playlist_prompt_action_button_horizontal_spacing();
    let video_width =
        semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(ui, &video_text);
    let playlist_width = semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(
        ui,
        &playlist_text,
    );
    let cancel_width =
        semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(ui, &cancel_text);
    let total_width = video_width + playlist_width + cancel_width + spacing * 2.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = spacing;
        ui.add_space(
            semantic_ui_metrics::remaining_width_before_right_aligned_prompt_actions(
                semantic_ui_metrics::playlist_prompt_content_width(),
                total_width,
            ),
        );
        let video_button = prompt_action_button(
            ui,
            AppIcon::Video,
            accent_green_for_ui(ui),
            &video_text,
            video_width,
            button_height,
        );
        if ui.add(video_button).clicked() {
            state.confirm_gallery_route_as_video();
        }
        let playlist_button = prompt_action_button(
            ui,
            AppIcon::Import,
            accent_green_for_ui(ui),
            &playlist_text,
            playlist_width,
            button_height,
        );
        if ui.add(playlist_button).clicked() {
            state.confirm_gallery_route_as_playlist();
        }
        let cancel_button = prompt_action_button(
            ui,
            AppIcon::WindowClose,
            accent_red_for_ui(ui),
            &cancel_text,
            cancel_width,
            button_height,
        );
        if ui.add(cancel_button).clicked() {
            state.cancel_gallery_route_prompt();
        }
    });
}

fn render_playlist_prompt_body(
    ui: &mut Ui,
    state: &AppState,
    title: &str,
    risk_note_key: Option<&'static str>,
    prompt_kind: YoutubePlaylistPromptKind,
) {
    let (heading, body) = match prompt_kind {
        YoutubePlaylistPromptKind::VideoAndPlaylist => (
            state
                .ui_i18n_text_for_key("options.which_one_should_be_loaded")
                .to_owned(),
            state
                .ui_i18n_text_for_key("options.both_video_and_playlist_were_detected")
                .to_owned(),
        ),
        YoutubePlaylistPromptKind::HighRiskPlaylist
        | YoutubePlaylistPromptKind::GenericPlaylist => (
            title.to_owned(),
            risk_note_key
                .map(|key| state.ui_i18n_text_for_key(key))
                .unwrap_or(
                    state.ui_i18n_text_for_key("options.this_playlist_may_contain_many_items"),
                )
                .to_owned(),
        ),
    };

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y =
            semantic_ui_metrics::playlist_prompt_body_vertical_spacing();
        ui.label(
            RichText::new(heading)
                .strong()
                .size(semantic_ui_metrics::playlist_prompt_title_text_size()),
        );
        ui.label(RichText::new(body).size(semantic_ui_metrics::playlist_prompt_body_text_size()));
    });
}

fn render_playlist_prompt_actions(
    ui: &mut Ui,
    state: &mut AppState,
    prompt_kind: YoutubePlaylistPromptKind,
) {
    let button_height = semantic_ui_metrics::prompt_action_button_height_for_playlist_decision();
    let spacing = semantic_ui_metrics::playlist_prompt_action_button_horizontal_spacing();

    match prompt_kind {
        YoutubePlaylistPromptKind::VideoAndPlaylist => {
            let video_text = state.ui_i18n_text_for_key("options.video").to_owned();
            let playlist_text = state.ui_i18n_text_for_key("options.playlist").to_owned();
            let cancel_text = state.ui_i18n_text_for_key("options.cancel").to_owned();
            let video_width =
                semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(
                    ui,
                    &video_text,
                );
            let playlist_width =
                semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(
                    ui,
                    &playlist_text,
                );
            let cancel_width =
                semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(
                    ui,
                    &cancel_text,
                );
            let total_width = video_width + playlist_width + cancel_width + spacing * 2.0;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = spacing;
                ui.add_space(
                    semantic_ui_metrics::remaining_width_before_right_aligned_prompt_actions(
                        semantic_ui_metrics::playlist_prompt_content_width(),
                        total_width,
                    ),
                );

                let video_button = prompt_action_button(
                    ui,
                    AppIcon::Video,
                    accent_green_for_ui(ui),
                    &video_text,
                    video_width,
                    button_height,
                );
                if ui.add(video_button).clicked() {
                    state.confirm_youtube_playlist_prompt_as_video();
                }

                let playlist_button = prompt_action_button(
                    ui,
                    AppIcon::Import,
                    accent_green_for_ui(ui),
                    &playlist_text,
                    playlist_width,
                    button_height,
                );
                if ui.add(playlist_button).clicked() {
                    state.confirm_youtube_playlist_prompt();
                }

                let cancel_button = prompt_action_button(
                    ui,
                    AppIcon::WindowClose,
                    accent_red_for_ui(ui),
                    &cancel_text,
                    cancel_width,
                    button_height,
                );
                if ui.add(cancel_button).clicked() {
                    state.cancel_youtube_playlist_prompt();
                }
            });
        }
        YoutubePlaylistPromptKind::HighRiskPlaylist
        | YoutubePlaylistPromptKind::GenericPlaylist => {
            let load_text = state.ui_i18n_text_for_key("options.load").to_owned();
            let cancel_text = state.ui_i18n_text_for_key("options.cancel").to_owned();
            let confirm_width =
                semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(
                    ui, &load_text,
                );
            let cancel_width =
                semantic_ui_metrics::prompt_action_button_width_for_icon_and_visible_text(
                    ui,
                    &cancel_text,
                );
            let total_width = confirm_width + cancel_width + spacing;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = spacing;
                ui.add_space(
                    semantic_ui_metrics::remaining_width_before_right_aligned_prompt_actions(
                        semantic_ui_metrics::playlist_prompt_content_width(),
                        total_width,
                    ),
                );

                let confirm_button = prompt_action_button(
                    ui,
                    AppIcon::Import,
                    accent_green_for_ui(ui),
                    &load_text,
                    confirm_width,
                    button_height,
                );
                if ui.add(confirm_button).clicked() {
                    state.confirm_youtube_playlist_prompt();
                }

                let cancel_button = prompt_action_button(
                    ui,
                    AppIcon::WindowClose,
                    accent_red_for_ui(ui),
                    &cancel_text,
                    cancel_width,
                    button_height,
                );
                if ui.add(cancel_button).clicked() {
                    state.cancel_youtube_playlist_prompt();
                }
            });
        }
    }
}

fn prompt_action_button<'a>(
    ui: &Ui,
    icon: AppIcon,
    icon_color: Color32,
    label: &'a str,
    width: f32,
    height: f32,
) -> egui::Button<'a> {
    let size = semantic_ui_metrics::standard_icon_size_from_current_control_metrics(ui);
    egui::Button::image_and_text(
        icon_image(icon, size, icon_color),
        RichText::new(label).size(size),
    )
    .min_size(egui::vec2(width, height))
}
