use eframe::egui::Ui;
use egui_taffy::Tui;

use crate::app::state::AppState;

use super::main_tab_monitor_control::MonitorToggleButton;
use super::main_tab_output_actions::{DownloadButton, DownloadContainerPicker, TargetSelectButton};
use super::main_tab_text_fields::{PathTextBox, UrlTextBox};
use super::main_tab_url_action::StartButton;
use super::xaml_taffy_styles;

pub(super) enum MainTabTextBoxRole {
    Url(UrlTextBox),
    Path(PathTextBox),
}

pub(super) enum MainTabButtonRole {
    MonitorToggle(MonitorToggleButton),
    Start(StartButton),
    TargetSelect(TargetSelectButton),
    Download(DownloadButton),
}

pub(super) struct MainTabControls {
    pub(super) url: MainTabTextBoxRole,
    pub(super) monitor_toggle: MainTabButtonRole,
    pub(super) start: MainTabButtonRole,
    pub(super) target_select: MainTabButtonRole,
    pub(super) path: MainTabTextBoxRole,
    pub(super) download_container: Option<DownloadContainerPicker>,
    pub(super) download: MainTabButtonRole,
}

impl MainTabTextBoxRole {
    pub(super) fn show(self, tui: &mut Tui, state: &mut AppState) {
        match self {
            Self::Url(url_text_box) => url_text_box.show(tui, state),
            Self::Path(path_text_box) => path_text_box.show(tui, state),
        }
    }
}

impl MainTabButtonRole {
    pub(super) fn show(self, tui: &mut Tui, state: &mut AppState) {
        match self {
            Self::MonitorToggle(monitor_toggle_button) => monitor_toggle_button.show(tui, state),
            Self::Start(start_button) => start_button.show(tui, state),
            Self::TargetSelect(target_select_button) => target_select_button.show(tui, state),
            Self::Download(download_button) => download_button.show(tui, state),
        }
    }
}

impl MainTabControls {
    pub(super) fn resolve(
        ui: &Ui,
        state: &AppState,
        row: xaml_taffy_styles::XamlSingleLineRowLayout,
    ) -> Self {
        #[cfg(any(test, cache_review_gui))]
        let config_before = crate::infrastructure::output_config_observation_counts(
            &crate::infrastructure::resolve_tool_path(&state.tool_paths.yt_dlp_config),
        );
        let output_locked = state.output_dir_locked_by_config();
        let output_display = state.output_dir_display_for_config_lock(output_locked);
        #[cfg(any(test, cache_review_gui))]
        ui.ctx().data_mut(|data| {
            let key = eframe::egui::Id::new("review-main-output-observations");
            let mut observations = data
                .get_temp::<Vec<serde_json::Value>>(key)
                .unwrap_or_default();
            let after = crate::infrastructure::output_config_observation_counts(
                &crate::infrastructure::resolve_tool_path(&state.tool_paths.yt_dlp_config),
            );
            observations.push(serde_json::json!({"before":config_before,"after":after,
                "locked":output_locked,"display":output_display}));
            data.insert_temp(key, observations);
        });
        Self {
            url: MainTabTextBoxRole::Url(UrlTextBox::resolve(row, state)),
            monitor_toggle: MainTabButtonRole::MonitorToggle(MonitorToggleButton::resolve(row)),
            start: MainTabButtonRole::Start(StartButton::resolve(row, ui, state)),
            target_select: MainTabButtonRole::TargetSelect(TargetSelectButton::resolve(
                row,
                ui,
                state,
                output_locked,
            )),
            path: MainTabTextBoxRole::Path(PathTextBox::resolve(row, output_display)),
            download_container: DownloadContainerPicker::resolve(row, state),
            download: MainTabButtonRole::Download(DownloadButton::resolve(row, ui, state)),
        }
    }
}
