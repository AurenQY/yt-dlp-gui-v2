//! Gallery-only single-line range/status and native output-folder controls.
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{self, TextStyle};
use egui_taffy::Tui;

use crate::app::state::AppState;
use crate::app::ui::{common, measure, semantic_ui_metrics, xaml_taffy_styles};
use crate::app::widgets::icon::{AppIcon, icon_image, standard_icon_color};
use crate::domain::GalleryQueueItem;
use crate::infrastructure::{open_output_folder, resolve_output_dir};

/// Native resolved media parents only; never reconstruct extractor directory templates.
pub(in crate::app::ui) fn output_folder(
    gallery: &GalleryQueueItem,
    output: Option<&str>,
) -> Option<PathBuf> {
    let mut parents = gallery
        .download_artifacts
        .iter()
        .filter_map(|a| a.path.parent());
    let mut common = parents.next().map(Path::to_path_buf);
    for parent in parents {
        while common
            .as_ref()
            .is_some_and(|folder| !parent.starts_with(folder))
        {
            common = common.and_then(|folder| folder.parent().map(Path::to_path_buf));
        }
    }
    common.or_else(|| {
        output.map(Path::new).and_then(|path| {
            if path.is_dir() {
                Some(path.to_path_buf())
            } else {
                path.parent().map(Path::to_path_buf)
            }
        })
    })
}

pub(in crate::app::ui) fn folder_display(
    folder: Option<&Path>,
    destination: &str,
    language: crate::i18n::Language,
) -> String {
    let Some(folder) = folder else {
        return crate::i18n::text(language, "gallery.folder_pending").to_owned();
    };
    let root = resolve_output_dir(destination).ok();
    let relative = root
        .as_deref()
        .and_then(|root| folder.strip_prefix(root).ok());
    match relative {
        Some(path) if path.as_os_str().is_empty() => {
            crate::i18n::text(language, "main.target_folder").to_owned()
        }
        Some(path) => format!(
            "{}{}",
            path.components().collect::<PathBuf>().display(),
            std::path::MAIN_SEPARATOR
        ),
        None => folder.display().to_string(),
    }
}

#[derive(Default)]
pub(super) enum GalleryRangeAction {
    #[default]
    None,
    Select,
    Download,
}

pub(super) fn range_row(
    tui: &mut Tui,
    label: &str,
    text: &str,
    counter: &str,
    selectable: bool,
    cancellable: bool,
    language: crate::i18n::Language,
    downloadable: bool,
) -> GalleryRangeAction {
    let mut select = false;
    let mut action = GalleryRangeAction::None;
    field_row(
        tui,
        label,
        language,
        |ui| {
            let response = summary_field(ui, text, counter, selectable);
            select = response.clicked();
        },
        |ui| {
            // Stop belongs exclusively to the card header while downloading.
            if !cancellable {
                if icon_action_button(ui, AppIcon::Download, downloadable).clicked() {
                    action = GalleryRangeAction::Download;
                }
            }
        },
    );
    if select {
        GalleryRangeAction::Select
    } else {
        action
    }
}

/// Latest-only filename pulse. This is disposable UI state, not queue history.
pub(in crate::app::ui) fn folder_row(
    tui: &mut Tui,
    state: &mut AppState,
    item_id: u64,
    folder: Option<&Path>,
    directory_text: &str,
    running: bool,
    completed: usize,
    filename: Option<&str>,
) {
    let ctx = tui.egui_ui().ctx().clone();
    let key = egui::Id::new(("gallery-completed-filename", item_id));
    let now = Instant::now();
    let (seen, changed) = ctx.data_mut(|data| {
        let previous = data.get_temp::<(usize, Instant)>(key);
        let value = match previous {
            Some((count, time)) if count == completed => (count, time),
            _ => (completed, now),
        };
        data.insert_temp(key, value);
        value
    });
    let duration = Duration::from_millis(1500);
    let flash = running && seen > 0 && now.duration_since(changed) < duration;
    if flash {
        ctx.request_repaint_after(duration.saturating_sub(now.duration_since(changed)));
    }
    let text = if flash {
        filename.unwrap_or(directory_text)
    } else {
        directory_text
    };
    let exists = folder.is_some_and(Path::is_dir);
    let mut open = false;
    field_row(
        tui,
        crate::i18n::text(state.language(), "main.target_folder"),
        state.language(),
        |ui| {
            summary_field(ui, text, "", false);
        },
        |ui| {
            ui.centered_and_justified(|ui| {
                open = super::super::item_card_output_actions::draw_output_action_arrow_button(
                    ui,
                    semantic_ui_metrics::item_card_field_row_height(),
                    exists,
                )
                .on_hover_cursor(if exists {
                    egui::CursorIcon::PointingHand
                } else {
                    egui::CursorIcon::Default
                })
                .clicked();
            });
        },
    );
    if open && let Some(folder) = folder.filter(|p| p.is_dir()) {
        match open_output_folder(&folder.display().to_string()) {
            Ok(()) => state.set_last_action_message("gallery.folder_opened"),
            Err(error) => state.set_last_action_message(error),
        }
    }
}

/// The template owns button size; image-only controls add no text padding or font cap.
fn icon_action_button(ui: &mut egui::Ui, icon: AppIcon, enabled: bool) -> egui::Response {
    let size = semantic_ui_metrics::item_card_download_icon_size_for_row_height(
        semantic_ui_metrics::item_card_field_row_height(),
    );
    let color = if enabled {
        standard_icon_color(ui)
    } else {
        ui.visuals().weak_text_color()
    };
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::Vec2::ZERO;
        ui.centered_and_justified(|ui| {
            ui.add_enabled(
                enabled,
                egui::Button::new(icon_image(icon, size, color))
                    .small()
                    .corner_radius(semantic_ui_metrics::item_card_icon_button_corner_radius()),
            )
        })
        .inner
    })
    .inner
    .on_hover_cursor(if enabled {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Default
    })
}

fn summary_field(ui: &mut egui::Ui, left: &str, right: &str, clickable: bool) -> egui::Response {
    let mut button = egui::Button::new("")
        .left_text(egui::RichText::new(left).text_style(TextStyle::Body))
        .right_text(egui::RichText::new(right).text_style(TextStyle::Body))
        .fill(ui.visuals().text_edit_bg_color())
        .corner_radius(semantic_ui_metrics::item_card_field_corner_radius())
        .truncate();
    if !clickable {
        button = button.sense(egui::Sense::hover());
    }
    let response = ui
        .centered_and_justified(|ui| {
            // Match the visible read-only border without overriding hover/active visuals.
            ui.visuals_mut().widgets.inactive.bg_stroke =
                ui.visuals().widgets.noninteractive.bg_stroke;
            ui.add(button)
        })
        .inner;
    if clickable {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

/// All sibling positioning belongs to the existing UiElement/Taffy row contract.
fn field_row(
    tui: &mut Tui,
    label_text: &str,
    language: crate::i18n::Language,
    field: impl FnOnce(&mut egui::Ui),
    action: impl FnOnce(&mut egui::Ui),
) {
    field_row_with_action(tui, label_text, language, field, Some(action));
}

/// Classic owns Download/Stop at the bottom, so this row has no empty action slot.
pub(in crate::app::ui) fn classic_range_row(
    tui: &mut Tui,
    label: &str,
    text: &str,
    counter: &str,
    selectable: bool,
    language: crate::i18n::Language,
) -> bool {
    let mut clicked = false;
    field_row_with_action(
        tui,
        label,
        language,
        |ui| clicked = summary_field(ui, text, counter, selectable).clicked(),
        None::<fn(&mut egui::Ui)>,
    );
    clicked
}

fn field_row_with_action(
    tui: &mut Tui,
    label_text: &str,
    language: crate::i18n::Language,
    field: impl FnOnce(&mut egui::Ui),
    action: Option<impl FnOnce(&mut egui::Ui)>,
) {
    let ui = tui.egui_ui();
    let row = semantic_ui_metrics::xaml_single_line_control_row_contract_from_height(
        semantic_ui_metrics::item_card_field_row_height(),
    );
    // Both rows reserve the same measured label/action widths.
    let label_width = ["media.section", "about.status", "main.target_folder"]
        .into_iter()
        .map(|key| measure::text_width(ui, crate::i18n::text(language, key), TextStyle::Body))
        .fold(0.0_f32, f32::max);
    let action_width =
        semantic_ui_metrics::item_card_action_column_width_for_row_height(row.height);
    let label =
        semantic_ui_metrics::xaml_label_ui_element_from_row_contract_and_width(row, label_width);
    let layout = xaml_taffy_styles::XamlSingleLineRowLayout::new(row)
        .with_column_gap(semantic_ui_metrics::item_card_detail_column_gap());
    layout.show(tui, |tui| {
        layout
            .fixed_width_element_cell(label, label_width)
            .show(tui, |ui| {
                common::cell_label_right(ui, label_text);
            });
        layout.star_width_text_box_cell().show(tui, field);
        if let Some(action) = action {
            layout
                .fixed_width_stretch_cell(action_width)
                .show(tui, action);
        }
    });
}
