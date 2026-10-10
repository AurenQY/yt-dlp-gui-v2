//! Classic-mode direct downloader choice; Standard keeps its independent multi-select prompt.
use eframe::egui::{self, Ui};
use egui_taffy::{TuiBuilderLogic as _, tui};

use crate::app::state::{AppState, GalleryDownloaderChoice};

use super::super::{semantic_ui_metrics as metrics, xaml_taffy_styles as styles};

pub(super) fn render(ui: &mut Ui, state: &mut AppState) {
    let cancel_text = state.ui_i18n_text_for_key("options.cancel").to_owned();
    let row = metrics::xaml_single_line_control_row_contract_from_height(
        metrics::prompt_action_button_height_for_playlist_decision(),
    );
    let layout = styles::XamlSingleLineRowLayout::new(row)
        .with_column_gap(metrics::playlist_prompt_action_button_horizontal_spacing());
    let width = ui.available_width();
    let mut choice = None;
    let mut cancel = false;
    tui(ui, "classic-downloader-choice-actions")
        .reserve_width(width)
        .reserve_height(row.height)
        .style(layout.style())
        .show(|tui| {
            for (label, backend) in [
                ("yt-dlp", Some(GalleryDownloaderChoice::YtDlp)),
                ("gallery-dl", Some(GalleryDownloaderChoice::GalleryDl)),
                (cancel_text.as_str(), None),
            ] {
                tui.style(layout.star_width_cell_style(0.0, 1.0)).ui(|ui| {
                    if ui
                        .centered_and_justified(|ui| ui.add(egui::Button::new(label)))
                        .inner
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        match backend {
                            Some(backend) => choice = Some(backend),
                            None => cancel = true,
                        }
                    }
                });
            }
        });
    // Dispatch once after layout; reuse the existing selection/analysis route.
    if cancel {
        state.cancel_gallery_downloader_choice();
    } else if let Some(choice) = choice {
        state.set_gallery_downloader_checks(
            choice == GalleryDownloaderChoice::YtDlp,
            choice == GalleryDownloaderChoice::GalleryDl,
        );
        state.confirm_gallery_downloader_choice();
    }
}
