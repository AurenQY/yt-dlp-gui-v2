use eframe::egui;
use egui_taffy::Tui;

use crate::app::state::AppState;
use crate::infrastructure::render_backend::RenderBackend;
use crate::infrastructure::wgpu_api::WgpuApi;
use crate::infrastructure::{ThemeAccentColor, ThemeMode};

use super::settings_components::{
    SettingsFormLayout, SettingsSelectOption, settings_flexible_form_row, settings_section,
    settings_select_row, settings_toggle_row,
};

fn theme_mode_label_key(mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::System => "options.theme_mode.system",
        ThemeMode::Light => "options.theme_mode.light",
        ThemeMode::Dark => "options.theme_mode.dark",
    }
}

fn theme_accent_color_label_key(color: ThemeAccentColor) -> &'static str {
    match color {
        ThemeAccentColor::Off => "options.theme_color.off",
        ThemeAccentColor::System => "options.theme_color.blue",
        ThemeAccentColor::Blue => "options.theme_color.soft_blue",
        ThemeAccentColor::Purple => "options.theme_color.purple",
        ThemeAccentColor::Pink => "options.theme_color.pink",
        ThemeAccentColor::Green => "options.theme_color.green",
        ThemeAccentColor::Orange => "options.theme_color.orange",
        ThemeAccentColor::Slate => "options.theme_color.slate",
    }
}

pub(super) fn render_window_group(tui: &mut Tui, state: &mut AppState, form: SettingsFormLayout) {
    settings_section(
        tui,
        state.ui_i18n_text_for_key("options.appearance_window"),
        |tui| {
            if let Some(enabled) = settings_toggle_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.notifications"),
                state.config.windows_toast_enabled,
                state.ui_i18n_text_for_key("options.enable"),
            ) {
                state.set_windows_toast_enabled(enabled);
            }

            let theme_mode = state.config.theme_mode;
            let backend = state.config.render_backend;
            let backend_options = [RenderBackend::Glow, RenderBackend::Wgpu]
                .map(|backend| SettingsSelectOption::new(backend, backend.label()));
            if let Some(backend) = settings_select_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.render_backend"),
                "render-backend",
                backend,
                backend.label(),
                &backend_options,
            ) {
                state.set_render_backend(backend);
            }

            if state.config.render_backend == RenderBackend::Wgpu {
                let api = state.config.wgpu_api;
                let label = |api: WgpuApi| {
                    if api == WgpuApi::Auto {
                        state.ui_i18n_text_for_key("options.auto_detect")
                    } else {
                        api.label()
                    }
                };
                let options = [
                    WgpuApi::Auto,
                    WgpuApi::Dx12,
                    WgpuApi::Vulkan,
                    WgpuApi::OpenGl,
                ]
                .map(|api| SettingsSelectOption::new(api, label(api)));
                let selected = wgpu_api_selected_text(
                    api,
                    label(api),
                    state.ui_i18n_text_for_key("options.current"),
                    state.runtime_renderer.as_ref(),
                );
                if let Some(api) = settings_select_row(
                    tui,
                    form,
                    state.ui_i18n_text_for_key("options.wgpu_api"),
                    "wgpu-api",
                    api,
                    &selected,
                    &options,
                ) {
                    state.set_wgpu_api(api);
                }
            }
            if let Some(active) = &state.runtime_renderer {
                settings_flexible_form_row(
                    tui,
                    form,
                    state.ui_i18n_text_for_key("options.current"),
                    |ui| {
                        let text = if active.is_wgpu {
                            format!("Wgpu / {}", active.api)
                        } else {
                            active.api.clone()
                        };
                        ui.label(text).on_hover_text(&active.gpu);
                    },
                );
            }

            let theme_options = ThemeMode::variants().map(|mode| {
                SettingsSelectOption::new(
                    mode,
                    state.ui_i18n_text_for_key(theme_mode_label_key(mode)),
                )
            });
            if let Some(mode) = settings_select_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.theme"),
                "theme-mode",
                theme_mode,
                state.ui_i18n_text_for_key(theme_mode_label_key(theme_mode)),
                &theme_options,
            ) {
                state.set_theme_mode(mode);
            }

            let accent_color = state.config.theme_accent_color;
            let accent_options = ThemeAccentColor::variants().map(|color| {
                SettingsSelectOption::new(
                    color,
                    state.ui_i18n_text_for_key(theme_accent_color_label_key(color)),
                )
            });
            if let Some(color) = settings_select_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.theme_color"),
                "theme-accent-color",
                accent_color,
                state.ui_i18n_text_for_key(theme_accent_color_label_key(accent_color)),
                &accent_options,
            ) {
                state.set_theme_accent_color(color);
            }

            settings_flexible_form_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.ui_scale"),
                |ui| {
                    let mut pending = state.pending_ui_scale_percent();
                    if ui
                        .add(
                            egui::DragValue::new(&mut pending)
                                .range(80..=200)
                                .speed(1.0)
                                .suffix("%"),
                        )
                        .changed()
                    {
                        state.set_pending_ui_scale_percent(pending);
                    }

                    let has_pending_change = state.ui_scale_has_pending_change();
                    ui.add_enabled_ui(has_pending_change, |ui| {
                        if ui
                            .button(state.ui_i18n_text_for_key("options.apply"))
                            .clicked()
                        {
                            state.apply_pending_ui_scale_percent();
                        }
                    });
                    ui.label(format!(
                        "{} {}%",
                        state.ui_i18n_text_for_key("options.current"),
                        state.config.ui_scale_percent
                    ));
                },
            );
            if let Some(enabled) = settings_toggle_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.always_on_top"),
                state.config.keep_window_on_top,
                state.ui_i18n_text_for_key("options.enable"),
            ) {
                state.set_keep_window_on_top(enabled);
            }
            if let Some(enabled) = settings_toggle_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.window_position"),
                state.config.remember_window_position,
                state.ui_i18n_text_for_key("options.remember"),
            ) {
                state.set_remember_window_position(enabled);
            }
            if let Some(enabled) = settings_toggle_row(
                tui,
                form,
                state.ui_i18n_text_for_key("options.window_size"),
                state.config.remember_window_size,
                state.ui_i18n_text_for_key("options.remember"),
            ) {
                state.set_remember_window_size(enabled);
            }
        },
    );
}

fn wgpu_api_selected_text(
    requested: WgpuApi,
    label: &str,
    current_label: &str,
    active: Option<&crate::infrastructure::wgpu_api::RuntimeRendererInfo>,
) -> String {
    match active {
        Some(active) if requested == WgpuApi::Auto && active.is_wgpu => {
            format!("{label} ({current_label}: {})", active.api)
        }
        _ => label.to_owned(),
    }
}
