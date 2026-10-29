use eframe::egui;
use egui_taffy::Tui;

use crate::app::site_icon::{site_icon_for_url, site_icon_image};
use crate::app::state::{AppMode, AppState};
use crate::app::widgets::icon::{
    AppIcon, GALLERY_CAPABILITY_COLOR, VIDEO_CAPABILITY_COLOR, icon_image,
};
use crate::app::widgets::url_input::{AppTextBox, AppTextBoxSyntax};
use crate::infrastructure::extractor_index::YtSourceScope;

use super::common::UiText;
use super::xaml_taffy_styles;

#[derive(Debug, Clone, Copy)]
pub(super) struct UrlTextBox {
    cell: xaml_taffy_styles::XamlSingleLineRowCell,
    is_single_mode: bool,
    url_input_locked: bool,
}

#[derive(Debug, Clone)]
pub(super) struct PathTextBox {
    cell: xaml_taffy_styles::XamlSingleLineRowCell,
    display: String,
}

impl UrlTextBox {
    pub(super) fn resolve(
        row: xaml_taffy_styles::XamlSingleLineRowLayout,
        state: &AppState,
    ) -> Self {
        Self {
            cell: row.star_width_text_box_cell(),
            is_single_mode: state.app_mode() == AppMode::Origin,
            url_input_locked: state.url_input_locked(),
        }
    }

    pub(super) fn show(self, tui: &mut Tui, state: &mut AppState) {
        self.cell.show(tui, |ui| {
            let url_hint = state.ui_i18n_text_for_key(UiText::URL_HINT).to_owned();
            let language = state.language();
            let url_input_size = self.cell.measured_size_for_ui(ui);
            let icon_size = ui.text_style_height(&egui::TextStyle::Body) * 0.85;
            let leading_image = (!state.url_input.trim().is_empty()).then(|| {
                site_icon_for_url(&state.url_input).map_or_else(
                    || icon_image(AppIcon::Web, icon_size, ui.visuals().text_color()),
                    |icon| site_icon_image(icon, icon_size),
                )
            });
            let mut source_icons = Vec::new();
            if state.url_source_preview.scope != YtSourceScope::Unknown
                && state.url_source_preview.yt_working != Some(false)
            {
                source_icons.push((
                    match state.url_source_preview.scope {
                        YtSourceScope::Single => AppIcon::PlayBoxOutline,
                        YtSourceScope::Playlist => AppIcon::PlaylistPlay,
                        YtSourceScope::Any => AppIcon::Multimedia,
                        YtSourceScope::Unknown => {
                            unreachable!("unknown preview has no capability icon")
                        }
                    },
                    VIDEO_CAPABILITY_COLOR,
                ));
            }
            if state.url_source_preview.gallery_supported {
                source_icons.push((AppIcon::ImageMultipleOutline, GALLERY_CAPABILITY_COLOR));
            }
            let response = AppTextBox::new(&mut state.url_input)
                .hint_text(&url_hint)
                .leading_image(leading_image)
                .trailing_icons(source_icons)
                .language(language)
                .enabled(!self.url_input_locked)
                .syntax(AppTextBoxSyntax::Url)
                .desired_width(url_input_size.width)
                .desired_height(url_input_size.height)
                .min_rows(1)
                .max_rows(Some(1))
                .allow_newline(false)
                .ctrl_click_links(false)
                .ui(ui);
            if response.changed() {
                state.refresh_url_source_preview(ui.ctx());
            }
            let submit_requested = self.is_single_mode
                && response.has_focus()
                && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if submit_requested && !self.url_input_locked {
                state.run_primary_url_action();
            }
        });
    }
}

impl PathTextBox {
    pub(super) fn resolve(
        row: xaml_taffy_styles::XamlSingleLineRowLayout,
        display: String,
    ) -> Self {
        Self {
            cell: row.star_width_text_box_cell(),
            display,
        }
    }

    pub(super) fn show(self, tui: &mut Tui, _state: &mut AppState) {
        self.cell.show(tui, |ui| {
            let mut output_dir_display = self.display;
            let output_text_box_size = self.cell.measured_size_for_ui(ui);
            AppTextBox::new(&mut output_dir_display)
                .editable(false)
                .selectable(true)
                .syntax(AppTextBoxSyntax::Path)
                .desired_width(output_text_box_size.width)
                .desired_height(output_text_box_size.height)
                .min_rows(1)
                .max_rows(Some(1))
                .allow_newline(false)
                .ctrl_click_links(false)
                .ui(ui);
        });
    }
}
