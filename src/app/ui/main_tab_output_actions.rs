use eframe::egui::{self, Ui};
use egui_taffy::Tui;

use crate::app::state::{AppMode, AppState};
use crate::app::widgets::icon::AppIcon;
use crate::domain::{DownloadContainerPreference, QueueBackend, QueueItemId};

use super::common::{UiText, icon_text_button};
use super::item_card::draw_item_output_container_picker;
use super::main_tab_dependency_notice::missing_tool_icon_text_button;
use super::{semantic_ui_metrics, xaml_layout_contracts, xaml_taffy_styles};

#[derive(Debug, Clone, Copy)]
pub(super) struct TargetSelectButton {
    cell: xaml_taffy_styles::XamlSingleLineRowCell,
    button_size: xaml_layout_contracts::LayoutSize,
    mode: TargetSelectMode,
    enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetSelectMode {
    Folder,
    OriginSaveAs,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DownloadButton {
    cell: xaml_taffy_styles::XamlSingleLineRowCell,
    button_size: xaml_layout_contracts::LayoutSize,
}

#[derive(Debug, Clone)]
pub(super) struct DownloadContainerPicker {
    cell: xaml_taffy_styles::XamlSingleLineRowCell,
    item_id: QueueItemId,
    preference: DownloadContainerPreference,
    resolved_extension: Option<String>,
    enabled: bool,
    supports_mp4: bool,
    supports_webm: bool,
}

impl DownloadContainerPicker {
    pub(super) fn resolve(
        row: xaml_taffy_styles::XamlSingleLineRowLayout,
        state: &AppState,
    ) -> Option<Self> {
        if state.app_mode() != AppMode::Origin
            || state.queue_items.is_empty()
            || matches!(&state.queue_items[0].backend, QueueBackend::GalleryDl(_))
        {
            return None;
        }
        Some(Self {
            cell: row.fixed_width_stretch_cell(
                semantic_ui_metrics::item_card_output_container_picker_width(),
            ),
            item_id: state.queue_items[0].id,
            preference: state.queue_items[0].selection.container_preference,
            resolved_extension: state.resolved_item_download_container_extension(0),
            enabled: state.item_download_container_is_selectable(0),
            supports_mp4: state.item_supports_mp4_download_container(0),
            supports_webm: state.item_supports_webm_download_container(0),
        })
    }

    pub(super) fn show(self, tui: &mut Tui, state: &mut AppState) {
        self.cell.show(tui, |ui| {
            if let Some(container) = draw_item_output_container_picker(
                ui,
                self.item_id,
                self.preference,
                self.resolved_extension.as_deref(),
                self.enabled,
                self.supports_mp4,
                self.supports_webm,
            ) {
                state.set_item_download_container_preference(0, container);
            }
        });
    }
}

impl TargetSelectButton {
    pub(super) fn resolve(
        row: xaml_taffy_styles::XamlSingleLineRowLayout,
        ui: &Ui,
        state: &AppState,
    ) -> Self {
        let target_text = state.ui_i18n_text_for_key(UiText::TARGET_DIR);
        let element =
            semantic_ui_metrics::xaml_icon_text_button_ui_element_for_visible_text(ui, target_text);
        let button_size = row.measure_auto_width_element(element).size;
        let item_metadata_loaded = state
            .queue_items
            .first()
            .is_some_and(|item| item.metadata_loaded());
        let item_busy = state
            .queue_items
            .first()
            .is_some_and(|_| state.item_is_busy(0));
        let is_gallery = state
            .queue_items
            .first()
            .is_some_and(|item| matches!(&item.backend, QueueBackend::GalleryDl(_)));
        let mode = target_select_mode(
            state.app_mode(),
            item_metadata_loaded && !is_gallery,
            item_busy,
        );
        Self {
            cell: row.fixed_width_element_cell(element, button_size.width),
            button_size,
            mode,
            enabled: target_select_enabled(
                state.app_mode(),
                item_busy,
                state.output_dir_locked_by_config(),
            ),
        }
    }

    pub(super) fn show(self, tui: &mut Tui, state: &mut AppState) {
        self.cell.show(tui, |ui| {
            let target_text = state.ui_i18n_text_for_key(UiText::TARGET_DIR);
            let response = ui.add_enabled(
                self.enabled,
                icon_text_button(ui, AppIcon::FolderMoveOutline, target_text)
                    .min_size(egui::vec2(self.button_size.width, self.button_size.height)),
            );
            if response.clicked() {
                match self.mode {
                    TargetSelectMode::Folder => select_output_folder(state),
                    TargetSelectMode::OriginSaveAs => select_origin_output_file(state),
                }
            }
        });
    }
}

fn target_select_mode(
    app_mode: AppMode,
    item_metadata_loaded: bool,
    item_busy: bool,
) -> TargetSelectMode {
    if app_mode == AppMode::Origin && item_metadata_loaded && !item_busy {
        TargetSelectMode::OriginSaveAs
    } else {
        TargetSelectMode::Folder
    }
}

fn target_select_enabled(app_mode: AppMode, item_busy: bool, output_locked: bool) -> bool {
    !output_locked && !(app_mode == AppMode::Origin && item_busy)
}

fn output_dialog_with_initial_directory(state: &AppState) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new();
    if let Ok(current_dir) =
        crate::infrastructure::resolve_output_dir(&state.item_defaults.output_dir)
    {
        dialog = dialog.set_directory(current_dir);
    }
    dialog
}

fn select_output_folder(state: &mut AppState) {
    if let Some(folder) = output_dialog_with_initial_directory(state).pick_folder() {
        state.set_output_dir(folder.display().to_string());
    }
}

fn select_origin_output_file(state: &mut AppState) {
    let Some(default_extension) = state.item_main_download_default_extension(0) else {
        return;
    };
    let mut dialog = output_dialog_with_initial_directory(state);
    if let Some(default_name) =
        state.item_export_default_name(0, crate::infrastructure::DownloadTargetKind::Normal)
    {
        dialog = dialog.set_file_name(default_name);
    }
    for extension in origin_save_extensions(&default_extension) {
        dialog = dialog.add_filter(format!(".{extension}"), &[extension]);
    }
    if let Some(path) = dialog.save_file() {
        if let Err(error) = state.set_origin_save_as_target(path, &default_extension) {
            state.set_last_action_message(error);
        }
    }
}

fn origin_save_extensions(default_extension: &str) -> Vec<&str> {
    let mut extensions = vec![default_extension];
    for extension in ["mkv", "mp4", "webm", "mov", "flv"] {
        if !extensions
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(extension))
        {
            extensions.push(extension);
        }
    }
    extensions
}

impl DownloadButton {
    pub(super) fn resolve(
        row: xaml_taffy_styles::XamlSingleLineRowLayout,
        ui: &Ui,
        state: &AppState,
    ) -> Self {
        let download_text = state.ui_i18n_text_for_key(main_download_label_key(state));
        let element = semantic_ui_metrics::xaml_icon_text_button_ui_element_for_visible_text(
            ui,
            download_text,
        );
        let button_size = row.measure_auto_width_element(element).size;
        Self {
            cell: row.fixed_width_element_cell(element, button_size.width),
            button_size,
        }
    }

    pub(super) fn show(self, tui: &mut Tui, state: &mut AppState) {
        self.cell.show(tui, |ui| {
            if state.app_mode() == AppMode::Origin {
                let stop = state.origin_download_cancellable();
                let enabled = if stop {
                    !state.origin_download_stopping()
                } else {
                    state.origin_download_can_start()
                };
                let missing_yt_dlp = !stop
                    && state
                        .queue_items
                        .first()
                        .is_some_and(|item| matches!(&item.backend, QueueBackend::YtDlp))
                    && state.required_dependency_notice().is_some();
                let label = state.ui_i18n_text_for_key(main_download_label_key(state));
                let button = if missing_yt_dlp {
                    missing_tool_icon_text_button(ui, AppIcon::Download, label)
                } else {
                    icon_text_button(
                        ui,
                        if stop {
                            AppIcon::Stop
                        } else {
                            AppIcon::Download
                        },
                        label,
                    )
                }
                .min_size(egui::vec2(self.button_size.width, self.button_size.height));
                if ui.add_enabled(enabled, button).clicked() {
                    if missing_yt_dlp {
                        state.open_dependency_component(
                            crate::infrastructure::DependencyTool::YtDlp,
                        );
                    } else {
                        state.request_origin_download();
                    }
                }
                return;
            }
            let has_pending_downloads = state.has_pending_download_items();
            let missing_yt_dlp = has_pending_downloads
                && state.queue_items.iter().any(|item| {
                    matches!(&item.backend, crate::domain::QueueBackend::YtDlp)
                        && crate::app::queue_status::item_can_enter_download_queue(item)
                })
                && state.required_dependency_notice().is_some();
            let button = download_button_for_state(ui, state, missing_yt_dlp)
                .min_size(egui::vec2(self.button_size.width, self.button_size.height));
            let response = if has_pending_downloads {
                ui.add(button)
            } else {
                ui.add_enabled(false, button)
            };
            if response.clicked() && has_pending_downloads {
                if missing_yt_dlp {
                    state.open_dependency_component(crate::infrastructure::DependencyTool::YtDlp);
                } else {
                    state.request_main_download();
                }
            }
        });
    }
}

fn main_download_label_key(state: &AppState) -> &'static str {
    if state.app_mode() == AppMode::Origin && state.origin_download_cancellable() {
        if state.origin_download_stopping() {
            "action.stopping"
        } else {
            "action.stop"
        }
    } else {
        UiText::DOWNLOAD
    }
}

fn download_button_for_state(ui: &Ui, state: &AppState, muted: bool) -> egui::Button<'static> {
    let download_text = state.ui_i18n_text_for_key(UiText::DOWNLOAD);
    if muted {
        missing_tool_icon_text_button(ui, AppIcon::Download, download_text)
    } else {
        icon_text_button(ui, AppIcon::Download, download_text)
    }
}
