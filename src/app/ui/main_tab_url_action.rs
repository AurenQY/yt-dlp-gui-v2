use eframe::egui::{self, RichText, Spinner, TextStyle, Ui, WidgetText};
use egui_taffy::Tui;

use crate::app::state::{AppMode, AppState};
use crate::app::widgets::icon::AppIcon;

use super::common::icon_text_button;
use super::main_tab_dependency_notice::{
    missing_tool_icon_text_button, missing_tool_plain_button, show_missing_yt_dlp_callout,
};
use super::xaml_rect_template::{show_rect_template, show_ui_at_rect};
use super::xaml_ui_nodes::{TemplateAxis, block, cols, fill, fixed_px, gap};
use super::{semantic_ui_metrics, xaml_taffy_styles};

type MainUrlActionButtonTemplate = StartButton;

#[derive(Debug, Clone, Copy)]
pub(super) struct StartButton {
    cell: xaml_taffy_styles::XamlSingleLineRowCell,
    show_spinner: bool,
    analysis_running: bool,
    url_input_locked: bool,
    spinner_size: f32,
    spinner_gap: f32,
    spinner_label_width: f32,
}

impl StartButton {
    pub(super) fn resolve(
        row: xaml_taffy_styles::XamlSingleLineRowLayout,
        ui: &Ui,
        state: &AppState,
    ) -> Self {
        let show_spinner = state.is_adding_batch && !state.is_cancelling_batch_add;
        // Keep the existing single-button spinner visible while the two
        // Standard-mode backends are analyzing the URL.
        let analysis_running = state.single_mode_analysis_running()
            || state.gallery_backend_probe_running()
            || state.audio_url_preflight_running();
        let url_input_locked = state.url_input_locked();
        let spinner_size = semantic_ui_metrics::standard_icon_size_from_current_control_metrics(ui);
        let spinner_gap = semantic_ui_metrics::main_url_action_spinner_to_text_horizontal_spacing();
        let checking = gallery_checking_blocks_start(state);
        let action_text = state.ui_i18n_text_for_key(if checking {
            "about.status.checking"
        } else {
            state.primary_url_action_label_key()
        });
        let spinner_active = analysis_running || show_spinner;
        let spinner_label_width = WidgetText::from(RichText::new(action_text).size(spinner_size))
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                TextStyle::Button,
            )
            .size()
            .x;
        let action_element = if spinner_active {
            semantic_ui_metrics::xaml_spinner_text_button_ui_element_for_measured_label(
                ui,
                spinner_label_width,
                spinner_size,
                spinner_gap,
            )
        } else if primary_url_action_uses_icon(state) && !checking {
            semantic_ui_metrics::xaml_icon_text_button_ui_element_for_visible_text(ui, action_text)
        } else {
            semantic_ui_metrics::xaml_button_ui_element_for_visible_text(ui, action_text)
        };
        let action_button_size = row.measure_auto_width_element(action_element).size;

        Self {
            cell: row.fixed_width_stretch_cell(action_button_size.width),
            show_spinner,
            analysis_running,
            url_input_locked,
            spinner_size,
            spinner_gap,
            spinner_label_width,
        }
    }

    pub(super) fn show(self, tui: &mut Tui, state: &mut AppState) {
        self.cell.show(tui, |ui| {
            if self.analysis_running || self.show_spinner {
                render_spinner_action_button(ui, state, &self);
                return;
            }

            let action_button_size = self.cell.measured_size_for_ui(ui);
            let action_button_vec2 =
                egui::vec2(action_button_size.width, action_button_size.height);

            if state.is_adding_batch && state.is_cancelling_batch_add {
                let button = primary_url_action_button(ui, state);
                ui.add_enabled(
                    false,
                    button
                        .min_size(action_button_vec2)
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
                return;
            }

            // Standard and Origin may analyze gallery-only URLs without yt-dlp.
            let gallery_eligible = matches!(state.app_mode(), AppMode::Standard | AppMode::Origin)
                && !state.queue_display_mode_is_audio()
                && !state.config.direct_download_on_add;
            let gallery_available = gallery_eligible && state.gallery_is_ready_for_display();
            if gallery_checking_blocks_start(state) {
                ui.add_enabled(
                    false,
                    egui::Button::new(state.ui_i18n_text_for_key("about.status.checking"))
                        .min_size(action_button_vec2),
                )
                .on_disabled_hover_text(state.gallery_readiness_detail());
                return;
            }
            let missing_yt_dlp = !gallery_available && state.required_dependency_notice().is_some();
            let button = primary_url_action_button_for_state(ui, state, missing_yt_dlp)
                .min_size(action_button_vec2)
                .wrap_mode(egui::TextWrapMode::Extend);
            let response = if missing_yt_dlp {
                ui.add(button)
            } else {
                ui.add_enabled(!self.url_input_locked, button)
            };
            if missing_yt_dlp {
                show_missing_yt_dlp_callout(ui, response.rect, "url-action", state);
                if response.clicked() {
                    state.open_dependency_component(crate::infrastructure::DependencyTool::YtDlp);
                }
            } else if response.clicked() {
                state.run_primary_url_action();
            }
        });
    }
}

fn gallery_checking_blocks_start(state: &AppState) -> bool {
    matches!(state.app_mode(), AppMode::Standard | AppMode::Origin)
        && !state.queue_display_mode_is_audio()
        && !state.config.direct_download_on_add
        && state.gallery_readiness_pending()
        && state.required_dependency_notice().is_some()
}

#[derive(Clone, Copy)]
enum SpinnerButtonSlot {
    Spacer,
    Spinner,
    Label,
}

fn render_spinner_action_button(
    ui: &mut Ui,
    state: &mut AppState,
    action_button: &MainUrlActionButtonTemplate,
) {
    let analysis_button_size = action_button.cell.measured_size_for_ui(ui);
    let desired_size = egui::vec2(analysis_button_size.width, analysis_button_size.height);
    let can_cancel = action_button.show_spinner && !action_button.analysis_running;
    let sense = if can_cancel {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(desired_size, sense);
    let (button_bg_fill, button_bg_stroke, button_fg_color) = {
        let visuals = if can_cancel {
            ui.style().interact(&response)
        } else {
            &ui.visuals().widgets.inactive
        };
        (visuals.bg_fill, visuals.bg_stroke, visuals.text_color())
    };
    ui.painter().rect(
        rect,
        2.0,
        button_bg_fill,
        button_bg_stroke,
        egui::StrokeKind::Outside,
    );

    let label = state.ui_i18n_text_for_key(state.primary_url_action_label_key());
    let template = cols([
        fill(block(SpinnerButtonSlot::Spacer)),
        fixed_px(
            action_button.spinner_size,
            block(SpinnerButtonSlot::Spinner),
        ),
        gap(action_button.spinner_gap),
        fixed_px(
            action_button.spinner_label_width,
            block(SpinnerButtonSlot::Label),
        ),
        fill(block(SpinnerButtonSlot::Spacer)),
    ]);
    show_rect_template(
        rect,
        template,
        &mut |_, _: TemplateAxis, _| 0.0,
        &mut |slot, slot_rect| match slot {
            SpinnerButtonSlot::Spacer => {}
            SpinnerButtonSlot::Spinner => {
                show_ui_at_rect(ui, slot_rect, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.add(Spinner::new().size(action_button.spinner_size));
                    });
                });
            }
            SpinnerButtonSlot::Label => {
                show_ui_at_rect(ui, slot_rect, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new(label)
                                .size(action_button.spinner_size)
                                .color(button_fg_color),
                        );
                    });
                });
            }
        },
    );
    if can_cancel && response.clicked() {
        state.cancel_batch_add();
    }
}

fn primary_url_action_icon(state: &AppState) -> AppIcon {
    if state.app_mode() == AppMode::Origin {
        AppIcon::Magnify
    } else if state.config.direct_download_on_add && !state.queue_display_mode_is_audio() {
        AppIcon::Download
    } else {
        AppIcon::Import
    }
}

fn primary_url_action_uses_icon(state: &AppState) -> bool {
    if state.is_adding_batch {
        return state.app_mode() == AppMode::Origin
            || (state.config.direct_download_on_add && !state.queue_display_mode_is_audio());
    }

    state.app_mode() == AppMode::Origin
        || state.queue_display_mode_is_audio()
        || state.config.direct_download_on_add
        || state.app_mode() == AppMode::Standard
}

fn primary_url_action_button_for_state(
    ui: &Ui,
    state: &AppState,
    muted: bool,
) -> egui::Button<'static> {
    if primary_url_action_uses_icon(state) {
        if muted {
            missing_tool_icon_text_button(
                ui,
                primary_url_action_icon(state),
                state.ui_i18n_text_for_key(state.primary_url_action_label_key()),
            )
        } else {
            icon_text_button(
                ui,
                primary_url_action_icon(state),
                state.ui_i18n_text_for_key(state.primary_url_action_label_key()),
            )
        }
    } else if muted {
        missing_tool_plain_button(
            ui,
            state.ui_i18n_text_for_key(state.primary_url_action_label_key()),
        )
    } else {
        egui::Button::new(state.ui_i18n_text_for_key(state.primary_url_action_label_key()))
    }
}

fn primary_url_action_button(ui: &Ui, state: &AppState) -> egui::Button<'static> {
    primary_url_action_button_for_state(ui, state, false)
}
