use egui_taffy::Tui;

use crate::app::state::AppState;
use crate::infrastructure::UnknownFilesizeFormatPolicy;

use super::settings_components::{
    SettingsFormLayout, SettingsSelectOption, settings_section, settings_select_row,
};

pub(super) fn render_format_list_group(
    tui: &mut Tui,
    state: &mut AppState,
    form: SettingsFormLayout,
) {
    settings_section(
        tui,
        state.ui_i18n_text_for_key("options.format_list"),
        |tui| {
            let policy = state.config.unknown_filesize_format_policy;
            let options = UnknownFilesizeFormatPolicy::variants().map(|candidate| {
                SettingsSelectOption::new(
                    candidate,
                    state.ui_i18n_text_for_key(candidate.label_key()),
                )
            });
            if let Some(candidate) = settings_select_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.unknown_filesize_formats"),
                "unknown-filesize-format-policy",
                policy,
                state.ui_i18n_text_for_key(policy.label_key()),
                &options,
            ) {
                state.set_unknown_filesize_format_policy(candidate);
            }
        },
    );
}
