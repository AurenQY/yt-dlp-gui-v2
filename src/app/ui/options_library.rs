//! Options › Library: download history, subscriptions and per-site folders.
use eframe::egui::{self, Align, Layout, RichText, ScrollArea, Ui};
use egui_taffy::Tui;

use crate::app::state::{AppState, OptionsDetailPage};
use crate::app::widgets::url_input::{AppTextBox, AppTextBoxSyntax};

use super::common::settings_section;
use super::semantic_ui_metrics;
use super::settings_components::{
    SettingsFormLayout, settings_flexible_form_row, settings_section as settings_taffy_section,
};
use super::settings_detail_template::{
    SettingsDetailNode, render_settings_detail_header, render_settings_detail_page,
};
use super::xaml_template_renderer::show_template_ui_block;

const LIST_MAX_HEIGHT: f32 = 420.0;
const URL_FIELD_WIDTH: f32 = 320.0;
const DOMAIN_FIELD_WIDTH: f32 = 160.0;

pub(super) fn render_library_group(tui: &mut Tui, state: &mut AppState, form: SettingsFormLayout) {
    settings_taffy_section(tui, state.ui_i18n_text_for_key("options.library"), |tui| {
        settings_flexible_form_row(
            tui,
            form,
            state.ui_i18n_text_for_key("options.history"),
            |ui| {
                let mut enabled = state.config.history_enabled;
                if ui
                    .checkbox(
                        &mut enabled,
                        state.ui_i18n_text_for_key("options.record_history"),
                    )
                    .changed()
                {
                    state.set_history_enabled(enabled);
                }
                let label = state.ui_i18n_text_with_replacements(
                    "options.history_entries",
                    &[("{count}", &state.history().len().to_string())],
                );
                if ui.button(format!("{label}  ›")).clicked() {
                    state.open_options_detail_page(OptionsDetailPage::History);
                }
            },
        );
        settings_flexible_form_row(
            tui,
            form,
            state.ui_i18n_text_for_key("options.subscriptions"),
            |ui| {
                let label = state.ui_i18n_text_with_replacements(
                    "options.subscriptions_count",
                    &[("{count}", &state.subscriptions().len().to_string())],
                );
                if ui.button(format!("{label}  ›")).clicked() {
                    state.open_options_detail_page(OptionsDetailPage::Subscriptions);
                }
            },
        );
        settings_flexible_form_row(
            tui,
            form,
            state.ui_i18n_text_for_key("options.site_rules"),
            |ui| {
                let label = state.ui_i18n_text_with_replacements(
                    "options.site_rules_count",
                    &[("{count}", &state.config.site_rules.len().to_string())],
                );
                if ui.button(format!("{label}  ›")).clicked() {
                    state.open_options_detail_page(OptionsDetailPage::SiteRules);
                }
            },
        );
    });
}

pub(super) fn render_playlist_filter_row(
    tui: &mut Tui,
    state: &mut AppState,
    form: SettingsFormLayout,
) {
    settings_flexible_form_row(
        tui,
        form,
        state.ui_i18n_text_for_key("options.playlist_filter"),
        |ui| {
            let current = state.config.playlist_match_filter.clone();
            if let Some(filter) = super::settings_components::deferred_text_box(
                ui,
                "playlist-match-filter",
                &current,
                state.ui_i18n_text_for_key("options.playlist_filter_hint"),
                state.language(),
                semantic_ui_metrics::advance_form_standard_text_field_width(),
                true,
            ) {
                state.set_playlist_match_filter(&filter);
            }
        },
    );
}

fn render_detail_page(
    ui: &mut Ui,
    state: &mut AppState,
    scroll_id: &'static str,
    taffy_id: &'static str,
    title_key: &'static str,
    body: fn(&mut Ui, &mut AppState),
) {
    // Taffy settles auto-height rows over passes; repaint once when a page
    // first appears so it never stays on its first-pass layout while idle.
    let seen_id = egui::Id::new((scroll_id, "last-frame"));
    let frame = ui.ctx().cumulative_frame_nr();
    let previous = ui.ctx().data(|data| data.get_temp::<u64>(seen_id));
    ui.ctx().data_mut(|data| data.insert_temp(seen_id, frame));
    if previous.is_none_or(|last| last + 1 < frame) {
        ui.ctx().request_repaint();
    }
    let mut show_block = |slot, node, tui: &mut Tui| match node {
        SettingsDetailNode::Header => show_template_ui_block(slot, tui, |ui| {
            render_settings_detail_header(
                ui,
                state,
                title_key,
                AppState::close_options_detail_page,
            );
        }),
        SettingsDetailNode::Body => show_template_ui_block(slot, tui, |ui| body(ui, state)),
    };
    render_settings_detail_page(
        ui,
        scroll_id,
        taffy_id,
        semantic_ui_metrics::options_language_detail_header_to_body_vertical_spacing(),
        &mut show_block,
    );
}

// ---- history -----------------------------------------------------------------

pub(super) fn render_history_detail_page(ui: &mut Ui, state: &mut AppState) {
    render_detail_page(
        ui,
        state,
        "options-history-page-scroll",
        "options-history-detail-template",
        "options.history",
        render_history_body,
    );
}

fn render_history_body(ui: &mut Ui, state: &mut AppState) {
    settings_section(ui, state.ui_i18n_text_for_key("options.history"), |ui| {
        ui.horizontal(|ui| {
            let hint = state.ui_i18n_text_for_key("history.search");
            let language = state.language();
            AppTextBox::new(state.history_search_mut())
                .hint_text(hint)
                .language(language)
                .syntax(AppTextBoxSyntax::Plain)
                .desired_width(URL_FIELD_WIDTH)
                .ui(ui);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !state.history().is_empty(),
                        egui::Button::new(state.ui_i18n_text_for_key("history.clear")),
                    )
                    .clicked()
                {
                    state.clear_history();
                }
            });
        });
        ui.add_space(6.0);
        let results = state.history_search_results();
        if results.is_empty() {
            ui.label(RichText::new(state.ui_i18n_text_for_key("history.empty")).weak());
            return;
        }
        let mut remove = None;
        let mut requeue = None;
        ScrollArea::vertical()
            .id_salt("options-history-list")
            .max_height(LIST_MAX_HEIGHT)
            .auto_shrink([false, true])
            .show_rows(ui, history_row_height(ui), results.len(), |ui, range| {
                for index in results[range].iter().copied() {
                    let Some(entry) = state.history().entries.get(index).cloned() else {
                        continue;
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&entry.downloaded_at).monospace().weak());
                        let title = if entry.title.trim().is_empty() {
                            entry.url.as_str()
                        } else {
                            entry.title.as_str()
                        };
                        ui.add(egui::Label::new(title).truncate())
                            .on_hover_text(format!("{}\n{}", entry.url, entry.output_path));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .small_button(state.ui_i18n_text_for_key("history.remove"))
                                .clicked()
                            {
                                remove = Some(index);
                            }
                            if ui
                                .small_button(state.ui_i18n_text_for_key("history.add_again"))
                                .clicked()
                            {
                                requeue = Some(index);
                            }
                            let exists =
                                crate::infrastructure::output_file_exists(&entry.output_path);
                            if ui
                                .add_enabled(
                                    exists,
                                    egui::Button::new(state.ui_i18n_text_for_key("item.open_file"))
                                        .small(),
                                )
                                .clicked()
                            {
                                if let Err(error) =
                                    crate::infrastructure::open_output_file(&entry.output_path)
                                {
                                    state.set_last_action_message(error);
                                }
                            }
                            if ui
                                .small_button(state.ui_i18n_text_for_key("item.open_folder"))
                                .clicked()
                            {
                                if let Err(error) =
                                    crate::infrastructure::open_output_folder(&entry.output_path)
                                {
                                    state.set_last_action_message(error);
                                }
                            }
                        });
                    });
                }
            });
        if let Some(index) = requeue {
            state.requeue_history_entry(index);
        }
        if let Some(index) = remove {
            state.remove_history_entry(index);
        }
    });
}

fn history_row_height(ui: &Ui) -> f32 {
    ui.spacing().interact_size.y + ui.spacing().item_spacing.y
}

// ---- subscriptions -------------------------------------------------------------

pub(super) fn render_subscriptions_detail_page(ui: &mut Ui, state: &mut AppState) {
    render_detail_page(
        ui,
        state,
        "options-subscriptions-page-scroll",
        "options-subscriptions-detail-template",
        "options.subscriptions",
        render_subscriptions_body,
    );
}

fn render_subscriptions_body(ui: &mut Ui, state: &mut AppState) {
    settings_section(
        ui,
        state.ui_i18n_text_for_key("options.subscriptions"),
        |ui| {
            ui.label(RichText::new(state.ui_i18n_text_for_key("subscriptions.help")).weak());
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                let hint = state.ui_i18n_text_for_key("subscriptions.url_hint");
                let language = state.language();
                let response = AppTextBox::new(state.subscription_url_draft_mut())
                    .hint_text(hint)
                    .language(language)
                    .syntax(AppTextBoxSyntax::Plain)
                    .desired_width(URL_FIELD_WIDTH)
                    .ui(ui);
                let submitted =
                    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if ui
                    .button(state.ui_i18n_text_for_key("action.add"))
                    .clicked()
                    || submitted
                {
                    state.add_subscription_from_draft();
                }
            });
            ui.horizontal_wrapped(|ui| {
                let mut auto_check = state.config.subscriptions_auto_check;
                if ui
                    .checkbox(
                        &mut auto_check,
                        state.ui_i18n_text_for_key("subscriptions.auto_check"),
                    )
                    .changed()
                {
                    state.set_subscriptions_auto_check(auto_check);
                }
                let mut hours = state.config.subscriptions_interval_hours;
                let response = ui.add_enabled(
                    state.config.subscriptions_auto_check,
                    egui::DragValue::new(&mut hours)
                        .range(1..=crate::infrastructure::MAX_SUBSCRIPTION_INTERVAL_HOURS)
                        .suffix(state.ui_i18n_text_for_key("subscriptions.hours_suffix")),
                );
                // Dragging updates the value live; the file is written once,
                // when the drag ends or a typed value is applied.
                if response.changed() {
                    state.preview_subscriptions_interval_hours(hours);
                }
                if response.drag_stopped() || (response.changed() && !response.dragged()) {
                    state.set_subscriptions_interval_hours(hours);
                }
                let running = state.subscription_check_running();
                let label = state.ui_i18n_text_for_key(if running {
                    "subscriptions.checking"
                } else {
                    "subscriptions.check_now"
                });
                if ui
                    .add_enabled(
                        !running && !state.subscriptions().is_empty(),
                        egui::Button::new(label),
                    )
                    .clicked()
                {
                    state.check_subscriptions_now();
                }
                if running {
                    ui.spinner();
                }
            });
            ui.add_space(6.0);
            if state.subscriptions().is_empty() {
                ui.label(RichText::new(state.ui_i18n_text_for_key("subscriptions.empty")).weak());
                return;
            }
            let mut remove = None;
            let mut toggle = None;
            let error_color = ui.visuals().error_fg_color;
            for (index, subscription) in state.subscriptions().to_vec().into_iter().enumerate() {
                ui.separator();
                ui.horizontal(|ui| {
                    let mut enabled = subscription.enabled;
                    if ui.checkbox(&mut enabled, "").changed() {
                        toggle = Some((index, enabled));
                    }
                    ui.vertical(|ui| {
                        ui.add(
                            egui::Label::new(RichText::new(subscription.display_name()).strong())
                                .truncate(),
                        )
                        .on_hover_text(&subscription.url);
                        let status = if subscription.last_checked.is_empty() {
                            state
                                .ui_i18n_text_for_key("subscriptions.never_checked")
                                .to_owned()
                        } else {
                            state.ui_i18n_text_with_replacements(
                                "subscriptions.last_checked",
                                &[("{time}", subscription.last_checked.as_str())],
                            )
                        };
                        ui.label(RichText::new(status).weak().small());
                        if !subscription.last_error.is_empty() {
                            ui.label(
                                RichText::new(&subscription.last_error)
                                    .small()
                                    .color(error_color),
                            );
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .small_button(state.ui_i18n_text_for_key("history.remove"))
                            .clicked()
                        {
                            remove = Some(index);
                        }
                    });
                });
            }
            if let Some((index, enabled)) = toggle {
                state.set_subscription_enabled(index, enabled);
            }
            if let Some(index) = remove {
                state.remove_subscription(index);
            }
        },
    );
}

// ---- site rules --------------------------------------------------------------------

pub(super) fn render_site_rules_detail_page(ui: &mut Ui, state: &mut AppState) {
    render_detail_page(
        ui,
        state,
        "options-site-rules-page-scroll",
        "options-site-rules-detail-template",
        "options.site_rules",
        render_site_rules_body,
    );
}

fn render_site_rules_body(ui: &mut Ui, state: &mut AppState) {
    settings_section(ui, state.ui_i18n_text_for_key("options.site_rules"), |ui| {
        ui.label(RichText::new(state.ui_i18n_text_for_key("site_rules.help")).weak());
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let language = state.language();
            let domain_hint = state.ui_i18n_text_for_key("site_rules.domain_hint");
            AppTextBox::new(state.site_rule_domain_draft_mut())
                .hint_text(domain_hint)
                .language(language)
                .syntax(AppTextBoxSyntax::Plain)
                .desired_width(DOMAIN_FIELD_WIDTH)
                .ui(ui);
            let folder_hint = state.ui_i18n_text_for_key("site_rules.folder_hint");
            AppTextBox::new(state.site_rule_dir_draft_mut())
                .hint_text(folder_hint)
                .language(language)
                .syntax(AppTextBoxSyntax::Plain)
                .desired_width(URL_FIELD_WIDTH)
                .ui(ui);
            if ui
                .button(state.ui_i18n_text_for_key("advance.browse"))
                .clicked()
            {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *state.site_rule_dir_draft_mut() = folder.display().to_string();
                }
            }
            if ui
                .button(state.ui_i18n_text_for_key("action.add"))
                .clicked()
            {
                state.add_site_rule_from_draft();
            }
        });
        ui.add_space(6.0);
        if state.config.site_rules.is_empty() {
            ui.label(RichText::new(state.ui_i18n_text_for_key("site_rules.empty")).weak());
            return;
        }
        let mut remove = None;
        for (index, rule) in state.config.site_rules.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&rule.domain).strong());
                ui.label("›");
                ui.add(egui::Label::new(&rule.output_dir).truncate());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .small_button(state.ui_i18n_text_for_key("history.remove"))
                        .clicked()
                    {
                        remove = Some(index);
                    }
                });
            });
        }
        if let Some(index) = remove {
            state.remove_site_rule(index);
        }
    });
}
