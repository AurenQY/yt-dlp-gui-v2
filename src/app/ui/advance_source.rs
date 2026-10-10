use egui_taffy::Tui;

use crate::app::state::AppState;

use super::settings_components::{
    SettingsFormLayout, SettingsSelectOption, settings_section, settings_select_row,
    settings_toggle_row,
};

pub(super) fn render_config_source_section(
    tui: &mut Tui,
    state: &mut AppState,
    form: SettingsFormLayout,
) {
    settings_section(tui, state.ui_i18n_text_for_key("advance.source"), |tui| {
        let config_files = state.available_yt_dlp_config_files();
        let selected = state.tool_paths.yt_dlp_config.clone();
        let selected_label = if selected.trim().is_empty() {
            state.ui_i18n_text_for_key("advance.none").to_owned()
        } else {
            config_files
                .iter()
                .find(|option| option.path == selected)
                .map(|option| option.name.clone())
                .unwrap_or_else(|| selected.clone())
        };
        let mut options = vec![SettingsSelectOption::new(
            String::new(),
            state.ui_i18n_text_for_key("advance.none"),
        )];
        options.extend(
            config_files
                .into_iter()
                .map(|option| SettingsSelectOption::new(option.path, option.name)),
        );
        if let Some(path) = settings_select_row(
            tui,
            form,
            state.ui_i18n_text_for_key("advance.config"),
            "yt-dlp-config-file",
            selected,
            &selected_label,
            &options,
        ) {
            state.set_yt_dlp_config_path(path);
        }
        if let Some(enabled) = settings_toggle_row(
            tui,
            form,
            state.ui_i18n_text_for_key("advance.analysis"),
            state.config.force_analysis,
            state.ui_i18n_text_for_key("advance.force_analysis"),
        ) {
            state.set_force_analysis(enabled);
        }
    });
}
