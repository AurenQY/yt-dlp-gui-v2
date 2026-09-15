use egui_taffy::Tui;

use crate::{
    app::state::AppState, infrastructure::browser_native_messaging::browser_integration_status,
};

use super::settings_components::{SettingsFormLayout, settings_section, settings_toggle_row};

pub(super) fn render_browser_integration_section(
    tui: &mut Tui,
    state: &mut AppState,
    form: SettingsFormLayout,
) {
    settings_section(
        tui,
        state.ui_i18n_text_for_key("advance.browser_integration"),
        |tui| match browser_integration_status() {
            Ok(status) => {
                let registered = [
                    status.chrome_registered.then_some("Chrome"),
                    status.brave_registered.then_some("Brave"),
                    status.edge_registered.then_some("Edge"),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" / ");
                let detail = if status.enabled {
                    state.ui_i18n_text_with_replacements(
                        "advance.browser_connection_enabled",
                        &[("{browsers}", registered.as_str())],
                    )
                } else {
                    state
                        .ui_i18n_text_for_key("advance.browser_connection_disabled")
                        .to_owned()
                };
                if let Some(enabled) = settings_toggle_row(
                    tui,
                    form,
                    state.ui_i18n_text_for_key("advance.browser_connection"),
                    state.config.browser_integration_enabled && status.enabled,
                    &detail,
                ) {
                    state.set_browser_integration_enabled(enabled);
                }
            }
            Err(error) => {
                super::settings_components::settings_flexible_form_row(
                    tui,
                    form,
                    state.ui_i18n_text_for_key("advance.browser_connection"),
                    |ui| {
                        ui.label(error);
                    },
                );
            }
        },
    );
}
