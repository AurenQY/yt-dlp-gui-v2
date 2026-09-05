use eframe::egui::{self, Sense, TextStyle, Ui};

use crate::app::state::AppState;
use crate::app::widgets::icon::standard_icon_color;
use crate::domain::QueueItemId;
use crate::infrastructure::{
    DownloadTargetKind, OutputFileActionMode, open_output_file, open_output_folder,
    output_file_exists, output_parent_folder_exists,
};

use super::semantic_ui_metrics;
use super::xaml_layout_contracts::LayoutLength;

pub(super) fn open_export_dialog(
    state: &mut AppState,
    item_id: QueueItemId,
    kind: DownloadTargetKind,
) {
    let Some(item_index) = state.queue_items.iter().position(|item| item.id == item_id) else {
        return;
    };
    if !state.item_can_export(item_index, kind) {
        return;
    }

    let mut dialog = rfd::FileDialog::new();
    let preferred_extension = state.item_export_default_extension(item_index, kind);
    if let Some(directory) = state.item_export_initial_directory(item_index) {
        dialog = dialog.set_directory(directory);
    }
    if let Some(file_name) = state.item_export_default_name(item_index, kind) {
        dialog = dialog.set_file_name(&file_name);
    }
    dialog = match kind {
        DownloadTargetKind::Video => dialog
            .add_filter(".mp4", &["mp4"])
            .add_filter(".mkv", &["mkv"])
            .add_filter(".webm", &["webm"])
            .add_filter(".mov", &["mov"])
            .add_filter(".flv", &["flv"]),
        DownloadTargetKind::Audio => {
            add_audio_export_filters(dialog, preferred_extension.as_deref())
        }
        DownloadTargetKind::Subtitle => dialog
            .add_filter(".srt", &["srt"])
            .add_filter(".vtt", &["vtt"])
            .add_filter(".ass", &["ass"])
            .add_filter(".ssa", &["ssa"])
            .add_filter(".lrc", &["lrc"])
            .add_filter(".ttml", &["ttml"])
            .add_filter(".dfxp", &["dfxp"])
            .add_filter(".json3", &["json3"])
            .add_filter(".srv3", &["srv3"])
            .add_filter(".srv2", &["srv2"])
            .add_filter(".srv1", &["srv1"]),
        DownloadTargetKind::Normal => dialog,
    };

    if let Some(path) = dialog.save_file() {
        if let Err(error) = state.start_item_export(item_id, kind, path.display().to_string()) {
            state.set_last_action_message(error);
        }
    }
}

const AUDIO_EXPORT_EXTENSIONS: [&str; 8] =
    ["mp3", "m4a", "flac", "wav", "opus", "aac", "vorbis", "alac"];

fn ordered_audio_export_extensions(preferred_extension: Option<&str>) -> Vec<&'static str> {
    let preferred = preferred_extension
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut extensions = Vec::with_capacity(AUDIO_EXPORT_EXTENSIONS.len());
    if let Some(preferred) = preferred {
        if let Some(extension) = AUDIO_EXPORT_EXTENSIONS
            .iter()
            .copied()
            .find(|extension| extension.eq_ignore_ascii_case(preferred))
        {
            extensions.push(extension);
        }
    }
    for extension in AUDIO_EXPORT_EXTENSIONS {
        if !extensions
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(extension))
        {
            extensions.push(extension);
        }
    }
    extensions
}

fn add_audio_export_filters(
    mut dialog: rfd::FileDialog,
    preferred_extension: Option<&str>,
) -> rfd::FileDialog {
    for extension in ordered_audio_export_extensions(preferred_extension) {
        dialog = dialog.add_filter(format!(".{extension}"), &[extension]);
    }
    dialog
}

pub(super) fn row_output_action_button(
    ui: &mut Ui,
    state: &mut AppState,
    output_path: &str,
    mode: OutputFileActionMode,
    row_height: f32,
) {
    match mode {
        OutputFileActionMode::Menu => {
            let response = draw_output_action_arrow_button(ui, row_height, true);
            egui::Popup::menu(&response).show(|ui| {
                let file_exists = output_file_exists(output_path);
                let folder_exists = output_parent_folder_exists(output_path);

                if ui
                    .add_enabled(
                        file_exists,
                        egui::Button::new(state.ui_i18n_text_for_key("item.open_file")),
                    )
                    .clicked()
                {
                    perform_output_action(ui, state, output_path, OutputAction::OpenFile);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        folder_exists,
                        egui::Button::new(state.ui_i18n_text_for_key("item.open_folder")),
                    )
                    .clicked()
                {
                    perform_output_action(ui, state, output_path, OutputAction::OpenFolder);
                    ui.close();
                }
                if ui
                    .button(state.ui_i18n_text_for_key("item.copy_path"))
                    .clicked()
                {
                    perform_output_action(ui, state, output_path, OutputAction::CopyPath);
                    ui.close();
                }
            });
        }
        OutputFileActionMode::OpenFolder => {
            if draw_output_action_arrow_button(ui, row_height, true).clicked() {
                perform_output_action(ui, state, output_path, OutputAction::OpenFolder);
            }
        }
        OutputFileActionMode::OpenFile => {
            if draw_output_action_arrow_button(ui, row_height, true).clicked() {
                perform_output_action(ui, state, output_path, OutputAction::OpenFile);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum OutputAction {
    OpenFile,
    OpenFolder,
    CopyPath,
}

fn perform_output_action(
    ui: &mut Ui,
    state: &mut AppState,
    output_path: &str,
    action: OutputAction,
) {
    match action {
        OutputAction::OpenFile => match open_output_file(output_path) {
            Ok(()) => state.set_last_action_message("Opened output file."),
            Err(file_error) => match open_output_folder(output_path) {
                Ok(()) => state.set_last_action_message(
                    state.ui_i18n_text_for_key("item.file_not_found_opened_the_output_location"),
                ),
                Err(folder_error) => {
                    state.set_last_action_message(format!("{file_error}; {folder_error}"));
                }
            },
        },
        OutputAction::OpenFolder => match open_output_folder(output_path) {
            Ok(()) => state
                .set_last_action_message(state.ui_i18n_text_for_key("item.opened_output_location")),
            Err(error) => state.set_last_action_message(error),
        },
        OutputAction::CopyPath => {
            ui.ctx().copy_text(output_path.to_owned());
            state.set_last_action_message(state.ui_i18n_text_for_key("item.copied_output_path"));
        }
    }
}

pub(super) fn draw_output_action_arrow_button(
    ui: &mut Ui,
    row_height: f32,
    enabled: bool,
) -> egui::Response {
    let desired_size =
        output_action_button_size_for_available_width(row_height, ui.available_width());
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(desired_size, sense);
    let visuals = ui.style().interact(&response);
    let text_color = if enabled {
        standard_icon_color(ui)
    } else {
        ui.visuals().weak_text_color()
    };

    ui.painter().rect(
        rect,
        2.0,
        visuals.bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Outside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "→",
        TextStyle::Body.resolve(ui.style()),
        text_color,
    );

    response
}

fn output_action_button_size_for_available_width(
    row_height: f32,
    available_width: f32,
) -> egui::Vec2 {
    let row_contract =
        semantic_ui_metrics::xaml_single_line_control_row_contract_from_height(row_height);
    let size = row_contract.measure_stretch_width_ui_element(
        semantic_ui_metrics::xaml_icon_button_ui_element_from_row_contract(row_contract)
            .width(LayoutLength::Star(1.0)),
        available_width,
    );
    egui::vec2(size.width, size.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_export_filters_start_with_selected_source_extension() {
        let extensions = ordered_audio_export_extensions(Some("opus"));

        assert_eq!(extensions.first().copied(), Some("opus"));
        assert_eq!(extensions.len(), AUDIO_EXPORT_EXTENSIONS.len());
        assert_eq!(
            extensions
                .iter()
                .filter(|extension| extension.eq_ignore_ascii_case("opus"))
                .count(),
            1
        );
    }

    #[test]
    fn unknown_audio_extension_keeps_stable_filter_order() {
        assert_eq!(
            ordered_audio_export_extensions(Some("unknown")),
            AUDIO_EXPORT_EXTENSIONS
        );
    }
}
