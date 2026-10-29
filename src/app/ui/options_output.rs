use super::common::{settings_taffy_form_row, settings_taffy_section};
use crate::{
    app::state::AppState,
    infrastructure::output_policy::{ExistingFilePolicy, OutputNameContext, OutputNamingMode},
};
use eframe::egui;
use egui_taffy::Tui;

pub(super) fn render_output_group(tui: &mut Tui, state: &mut AppState, width: f32) {
    settings_taffy_section(tui, state.ui_i18n_text_for_key("options.output"), |tui| {
        settings_taffy_form_row(
            tui,
            width,
            state.ui_i18n_text_for_key("options.output.naming"),
            |ui| {
                egui::ComboBox::from_id_salt("output-naming")
                    .selected_text(
                        state.ui_i18n_text_for_key(state.output_draft.naming.label_key()),
                    )
                    .show_ui(ui, |ui| {
                        for mode in OutputNamingMode::ALL {
                            let label = state.ui_i18n_text_for_key(mode.label_key());
                            ui.selectable_value(&mut state.output_draft.naming, mode, label);
                        }
                    });
            },
        );
        settings_taffy_form_row(
            tui,
            width,
            state.ui_i18n_text_for_key("options.output.existing"),
            |ui| {
                egui::ComboBox::from_id_salt("output-existing")
                    .selected_text(state.ui_i18n_text_for_key(
                        if state.output_draft.existing == ExistingFilePolicy::KeepBoth {
                            "options.output.keep_both"
                        } else {
                            "options.output.skip"
                        },
                    ))
                    .show_ui(ui, |ui| {
                        for (policy, key) in [
                            (ExistingFilePolicy::KeepBoth, "options.output.keep_both"),
                            (ExistingFilePolicy::Skip, "options.output.skip"),
                        ] {
                            let label = state.ui_i18n_text_for_key(key);
                            ui.selectable_value(&mut state.output_draft.existing, policy, label);
                        }
                    });
            },
        );
        if state.output_draft.naming == OutputNamingMode::Custom {
            settings_taffy_form_row(
                tui,
                width,
                state.ui_i18n_text_for_key("options.output.template"),
                |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.output_draft.template)
                            .desired_width(ui.available_width()),
                    );
                    ui.add(
                        egui::Label::new(
                            state.ui_i18n_text_for_key("options.output.template_help"),
                        )
                        .wrap(),
                    );
                },
            );
        }
        let selected = state.output_preview_context();
        let (context, extension, explicit) = selected.clone().unwrap_or_else(|| {
            (
                OutputNameContext {
                    resolved: true,
                    title: "Example video".into(),
                    id: "a1b2c3".into(),
                    uploader: "Example uploader".into(),
                    upload_date: "20261027".into(),
                    source: "preview".into(),
                },
                "mp4".into(),
                false,
            )
        });
        let preview = state.output_draft.basename(&context);
        let validation = state.output_draft.validate();
        settings_taffy_form_row(
            tui,
            width,
            state.ui_i18n_text_for_key("options.output.preview"),
            |ui| {
                match &preview {
                    Ok(name) => {
                        ui.add(
                            egui::Label::new(format!("{name}.{extension}"))
                                .wrap()
                                .selectable(true),
                        );
                    }
                    Err(error) => {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(error).color(ui.visuals().error_fg_color),
                            )
                            .wrap(),
                        );
                    }
                }
                if selected.is_none() {
                    ui.label("Sample preview");
                }
                if explicit {
                    ui.add(egui::Label::new("This item keeps its explicit filename.").wrap());
                }
                if !explicit && state.tool_paths.effective_config_owns_output() {
                    ui.add(egui::Label::new("Filename controlled by yt-dlp configuration").wrap());
                }
                ui.add(
                    egui::Label::new(state.ui_i18n_text_for_key("options.output.preview_help"))
                        .wrap(),
                );
            },
        );
        settings_taffy_form_row(tui, width, "", |ui| {
            let dirty = state.output_draft != state.config.output_preferences();
            if dirty
                && state.output_settings_feedback
                    == state.ui_i18n_text_for_key("options.output.saved")
            {
                state.output_settings_feedback.clear();
            }
            if dirty {
                ui.add(
                    egui::Label::new(state.ui_i18n_text_for_key("options.output.unsaved")).wrap(),
                );
            }
            if state.output_draft.naming != OutputNamingMode::Custom {
                if let Err(error) = &validation {
                    ui.add(egui::Label::new(error).wrap());
                }
            }
            if ui
                .add_enabled(
                    dirty && preview.is_ok() && validation.is_ok(),
                    egui::Button::new(state.ui_i18n_text_for_key("options.output.save")),
                )
                .clicked()
            {
                match state.config.with_saved_output(&state.output_draft) {
                    Ok(saved) => {
                        state.config = saved;
                        state.output_settings_feedback =
                            state.ui_i18n_text_for_key("options.output.saved").into();
                    }
                    Err(error) => {
                        state.output_settings_feedback =
                            format!("Could not save output settings: {error}");
                    }
                }
            }
            ui.add(egui::Label::new(state.ui_i18n_text_for_key("options.output.help")).wrap());
            if !state.output_settings_feedback.is_empty() {
                ui.add(
                    egui::Label::new(&state.output_settings_feedback)
                        .wrap()
                        .selectable(true),
                );
            }
        });
    });
}
