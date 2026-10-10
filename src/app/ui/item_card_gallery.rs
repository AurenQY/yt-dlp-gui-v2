//! Gallery content card: shared queue/header controls with a gallery-specific image preview.
use eframe::egui::{self, Align2, Color32, Rect, Sense, TextStyle, Ui};
use egui_taffy::{Tui, TuiBuilderLogic as _};

use crate::app::gallery_metadata_mapping::map_gallery_ui_metadata;
use crate::app::state::{AppState, ThumbnailRenderSource};
use crate::domain::QueueBackend;

use super::item_card::{item_header_height, item_header_row, paint_duration_badge};
use super::item_card_template::item_detail_column_style;
use super::item_card_thumbnail::paint_thumbnail_image_layers;
use super::{semantic_ui_metrics, xaml_taffy_styles};

#[path = "item_card_gallery_fields.rs"]
pub(super) mod fields;

#[path = "gallery_item_feedback.rs"]
pub(super) mod feedback;

pub(super) enum GalleryCardAction {
    None,
    Remove(u64),
    Stop(u64),
    Download(u64),
}

pub(super) fn render_gallery_queue_item(
    tui: &mut Tui,
    state: &mut AppState,
    index: usize,
) -> GalleryCardAction {
    let item = &state.queue_items[index];
    let QueueBackend::GalleryDl(gallery) = &item.backend else {
        return GalleryCardAction::None;
    };
    let item_id = item.id;
    let ui_metadata = gallery.analysis.as_ref().map(map_gallery_ui_metadata);
    let count = ui_metadata
        .as_ref()
        .map(|metadata| metadata.media_count)
        .filter(|count| *count > 0);
    let heading = ui_metadata
        .as_ref()
        .filter(|_| !item.needs_analysis_refresh)
        .map(|metadata| metadata.title.clone())
        .unwrap_or_else(|| item.title.clone());
    let date = ui_metadata
        .as_ref()
        .map(|metadata| metadata.date.clone())
        .unwrap_or_default();
    let url = ui_metadata
        .as_ref()
        .map(|metadata| metadata.source.url.clone())
        .unwrap_or_else(|| item.source_url.clone());
    let thumbnail_url = ui_metadata
        .as_ref()
        .filter(|metadata| !metadata.thumbnail.is_empty())
        .map(|metadata| metadata.thumbnail.clone())
        .unwrap_or_else(|| item.thumbnail_url.clone());
    let thumbnail_fallback = ui_metadata
        .as_ref()
        .map(|metadata| metadata.thumbnail_fallback.clone())
        .unwrap_or_default();
    let feedback::GalleryFeedback {
        timeline,
        running,
        stopping,
        finished,
        has_download,
        completed,
        range_text,
        counter,
        folder,
        folder_text,
        latest_file,
    } = feedback::GalleryFeedback::from_state(state, index).expect("Gallery card backend");
    let item_locked = state.item_is_busy(index);
    let downloadable = state.can_start_gallery_item_download(index);
    let cancellable = state.item_has_cancellable_download_workflow(item_id);
    let title_state = state.item_title_visual_state(index);
    let mut remove = false;
    let mut cancel = false;
    let mut download = false;

    super::item_card_template::show_queue_card(
        tui,
        ("gallery-item-card", item_id),
        |tui, item_hovered| {
            super::item_card_template::show_card_thumbnail(
                tui,
                ui_metadata.as_ref().and_then(|metadata| metadata.site_icon),
                |ui| {
                    let thumbnail_source = state.queue_thumbnail_render_source(
                        ui.ctx(),
                        item_id,
                        &[thumbnail_url.clone(), thumbnail_fallback.clone()],
                        semantic_ui_metrics::item_card_thumbnail_size(),
                    );
                    gallery_thumbnail(ui, state, thumbnail_source, &thumbnail_url, count);
                },
            );
            tui.style(item_detail_column_style()).add(|tui| {
                let ui = tui.egui_ui();
                let header_height =
                    item_header_height(ui, &heading, false, None, ui.available_width());
                if item_header_row(
                    tui,
                    &heading,
                    &url,
                    title_state,
                    false,
                    None,
                    header_height,
                    state.item_can_remove(index),
                    item_hovered,
                    cancellable,
                    stopping,
                    state.ui_i18n_text_for_key(if stopping {
                        "action.stopping"
                    } else if cancellable {
                        "action.stop"
                    } else {
                        "action.clear"
                    }),
                ) {
                    if cancellable {
                        cancel = true;
                    } else {
                        remove = true;
                    }
                }
                if !date.is_empty() && !has_download {
                    tui.style(xaml_taffy_styles::xaml_auto_height_block_style())
                        .ui(|ui| {
                            ui.add(egui::Label::new(date.as_str()).truncate());
                        });
                }
                let actions = fields::range_row(
                    tui,
                    state.ui_i18n_text_for_key(if timeline {
                        "media.section"
                    } else {
                        "about.status"
                    }),
                    &range_text,
                    &counter,
                    timeline && !item_locked,
                    cancellable,
                    state.language(),
                    downloadable && !cancellable,
                );
                match actions {
                    fields::GalleryRangeAction::Select => state.open_gallery_range_picker(item_id),
                    fields::GalleryRangeAction::Download => download = true,
                    fields::GalleryRangeAction::None => {}
                }
                fields::folder_row(
                    tui,
                    state,
                    item_id,
                    folder.as_deref(),
                    &folder_text,
                    (running || finished) && !stopping,
                    completed,
                    latest_file.as_deref(),
                );
            });
        },
    );
    if cancel {
        GalleryCardAction::Stop(item_id)
    } else if remove {
        GalleryCardAction::Remove(item_id)
    } else if download {
        GalleryCardAction::Download(item_id)
    } else {
        GalleryCardAction::None
    }
}

/// The thumbnail slot belongs to the existing item-card Taffy template.
/// Only its image contents are aspect-fitted; sibling positions stay unchanged.
fn gallery_thumbnail(
    ui: &mut Ui,
    state: &AppState,
    source: ThumbnailRenderSource,
    thumbnail_url: &str,
    count: Option<usize>,
) {
    let slot_size = semantic_ui_metrics::item_card_thumbnail_size();
    let (slot, _) = ui.allocate_exact_size(slot_size, Sense::hover());
    let visuals = &ui.style().visuals.widgets.noninteractive;
    ui.painter()
        .rect_filled(slot, 0.0, ui.visuals().faint_bg_color);

    if !paint_thumbnail_image_layers(ui, slot, &source, thumbnail_url) {
        match source {
            ThumbnailRenderSource::Loading => {
                gallery_thumbnail_placeholder(
                    ui,
                    slot,
                    state.ui_i18n_text_for_key("item.loading_thumbnail"),
                    visuals.fg_stroke.color,
                );
            }
            ThumbnailRenderSource::Failed(_)
            | ThumbnailRenderSource::None
            | ThumbnailRenderSource::DirectUrl
            | ThumbnailRenderSource::Texture(_) => {
                gallery_thumbnail_placeholder(
                    ui,
                    slot,
                    state.ui_i18n_text_for_key("item.thumbnail"),
                    visuals.fg_stroke.color,
                );
            }
        }
    }
    if let Some(count) = count {
        paint_duration_badge(
            ui,
            slot,
            &crate::i18n::format_text(
                state.language(),
                "gallery.media_count",
                &[("{count}", &count.to_string())],
            ),
        );
    }
    ui.painter()
        .rect_stroke(slot, 0.0, visuals.bg_stroke, egui::StrokeKind::Outside);
}

fn gallery_thumbnail_placeholder(ui: &Ui, slot: Rect, text: &str, color: Color32) {
    ui.painter().text(
        slot.center(),
        Align2::CENTER_CENTER,
        text,
        TextStyle::Body.resolve(ui.style()),
        color,
    );
}
