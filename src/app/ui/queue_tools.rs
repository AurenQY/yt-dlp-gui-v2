//! Queue toolbar extras (search, status filter, queue-finished action,
//! scheduled start, import/countdown banners), keyboard shortcuts and
//! file drops for the Main tab.
use eframe::egui::{self, Align, Color32, Id, Layout, RichText, Ui};

use crate::app::state::{AfterQueueAction, AppMode, AppState, AppTab, QueueStatusFilter};
use crate::app::widgets::icon::AppIcon;
use crate::app::widgets::url_input::{AppTextBox, AppTextBoxSyntax};

use super::common::icon_text_button;

const SEARCH_FIELD_ID_KEY: &str = "queue-tools-search-field-id";
const SEARCH_FIELD_WIDTH: f32 = 180.0;
const TIME_FIELD_WIDTH: f32 = 52.0;

pub(super) fn render_queue_tools_row(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal_wrapped(|ui| {
        let search_hint = state.ui_i18n_text_for_key("queue.search_hint");
        let language = state.language();
        let response = AppTextBox::new(state.queue_filter_text_mut())
            .hint_text(search_hint)
            .language(language)
            .syntax(AppTextBoxSyntax::Plain)
            .desired_width(SEARCH_FIELD_WIDTH)
            .ui(ui);
        ui.ctx()
            .data_mut(|data| data.insert_temp(Id::new(SEARCH_FIELD_ID_KEY), response.id));
        // egui releases focus on Escape before this check runs.
        if (response.has_focus() || response.lost_focus())
            && ui.input(|input| input.key_pressed(egui::Key::Escape))
        {
            state.clear_queue_filter();
        }

        let current = state.queue_filter();
        egui::ComboBox::from_id_salt("queue-status-filter")
            .selected_text(state.ui_i18n_text_for_key(current.label_key()))
            .show_ui(ui, |ui| {
                for filter in QueueStatusFilter::ALL {
                    if ui
                        .selectable_label(
                            current == filter,
                            state.ui_i18n_text_for_key(filter.label_key()),
                        )
                        .clicked()
                    {
                        state.set_queue_filter(filter);
                    }
                }
            });
        if state.queue_filter_is_active()
            && ui
                .small_button(state.ui_i18n_text_for_key("action.clear"))
                .clicked()
        {
            state.clear_queue_filter();
        }

        ui.separator();
        render_after_queue_picker(ui, state);
        ui.separator();
        render_scheduled_start(ui, state);
    });
    render_queue_banners(ui, state);
}

fn render_after_queue_picker(ui: &mut Ui, state: &mut AppState) {
    ui.label(state.ui_i18n_text_for_key("queue.after_label"));
    let current = state.after_queue_action();
    egui::ComboBox::from_id_salt("queue-after-action")
        .selected_text(state.ui_i18n_text_for_key(current.label_key()))
        .show_ui(ui, |ui| {
            for action in AfterQueueAction::ALL {
                if ui
                    .selectable_label(
                        current == action,
                        state.ui_i18n_text_for_key(action.label_key()),
                    )
                    .clicked()
                {
                    state.set_after_queue_action(action);
                }
            }
        });
}

fn render_scheduled_start(ui: &mut Ui, state: &mut AppState) {
    if let Some(time) = state.scheduled_queue_start() {
        ui.label(
            state.ui_i18n_text_with_replacements("queue.start_scheduled", &[("{time}", &time)]),
        );
        if ui
            .small_button(state.ui_i18n_text_for_key("options.cancel"))
            .clicked()
        {
            state.cancel_scheduled_queue_start();
        }
        return;
    }
    let language = state.language();
    let response = AppTextBox::new(state.scheduled_start_draft_mut())
        .hint_text("HH:MM")
        .language(language)
        .syntax(AppTextBoxSyntax::Plain)
        .desired_width(TIME_FIELD_WIDTH)
        .ui(ui);
    let submitted = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if ui
        .button(state.ui_i18n_text_for_key("queue.start_at"))
        .clicked()
        || submitted
    {
        state.schedule_queue_start_from_draft();
    }
}

fn render_queue_banners(ui: &mut Ui, state: &mut AppState) {
    if let Some(seconds) = state.after_queue_countdown_remaining() {
        let action = state.ui_i18n_text_for_key(state.after_queue_action().label_key());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(state.ui_i18n_text_with_replacements(
                    "queue.after_countdown",
                    &[("{action}", action), ("{seconds}", &seconds.to_string())],
                ))
                .strong()
                .color(warning_color(ui)),
            );
            if ui
                .button(state.ui_i18n_text_for_key("options.cancel"))
                .clicked()
            {
                state.cancel_after_queue_action();
            }
        });
    }
    let pending = state.pending_import_count();
    if pending > 0 {
        ui.horizontal(|ui| {
            let key = if state.app_mode() == AppMode::Origin {
                "queue.import_waiting"
            } else {
                "queue.importing"
            };
            ui.label(
                state.ui_i18n_text_with_replacements(key, &[("{count}", &pending.to_string())]),
            );
            if ui
                .small_button(state.ui_i18n_text_for_key("options.cancel"))
                .clicked()
            {
                state.cancel_pending_imports();
            }
        });
    }
}

fn warning_color(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(255, 196, 92)
    } else {
        Color32::from_rgb(176, 96, 0)
    }
}

pub(super) fn render_retry_failed_button(ui: &mut Ui, state: &mut AppState) {
    let response = ui.add_enabled(
        state.can_retry_failed_queue_items(),
        icon_text_button(
            ui,
            AppIcon::Repeat,
            state.ui_i18n_text_for_key("queue.retry_failed"),
        ),
    );
    if response.clicked() {
        state.retry_failed_queue_items();
    }
}

pub(super) fn render_import_button(ui: &mut Ui, state: &mut AppState) {
    let response = icon_text_button(
        ui,
        AppIcon::Import,
        state.ui_i18n_text_for_key("queue.import"),
    );
    if ui.add(response).clicked() {
        open_import_dialog(state);
    }
}

fn open_import_dialog(state: &mut AppState) {
    let files = rfd::FileDialog::new()
        .add_filter(
            "URL lists",
            &["txt", "url", "webloc", "csv", "md", "html", "htm"],
        )
        .add_filter("*", &["*"])
        .pick_files();
    if let Some(files) = files {
        state.import_urls_from_files(&files);
    }
}

pub(super) fn render_no_match_notice(ui: &mut Ui, state: &mut AppState) {
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.add_space(12.0);
        ui.label(RichText::new(state.ui_i18n_text_for_key("queue.no_match")).weak());
        if ui
            .button(state.ui_i18n_text_for_key("action.clear"))
            .clicked()
        {
            state.clear_queue_filter();
        }
    });
}

/// Main-tab keyboard shortcuts. While a text field has focus only Ctrl+F is
/// handled here; the field keeps every other key.
///
/// - Ctrl+V outside a text field: add the pasted URL(s)
/// - Ctrl+Enter: start downloading the queue
/// - Ctrl+F: focus the queue search box
/// - Ctrl+O: import URLs from files
/// - Ctrl+R: retry failed items
pub(super) fn handle_main_shortcuts(ctx: &egui::Context, state: &mut AppState, blocked: bool) {
    if blocked
        || state.active_tab != AppTab::Main
        || state.format_picker.open
        || state.gallery_range_picker.item_id.is_some()
        || state.should_show_prepare_tab()
    {
        return;
    }
    // Text fields keep their own Ctrl+V / Ctrl+Enter / Ctrl+R / Ctrl+O.
    // Ctrl+F stays global because it only moves focus to the queue search.
    let text_focused = ctx.text_edit_focused();
    let command = egui::Modifiers::COMMAND;
    let standard_queue = state.app_mode() != AppMode::Origin;
    if standard_queue && ctx.input_mut(|input| input.consume_key(command, egui::Key::F)) {
        if let Some(id) = ctx.data(|data| data.get_temp::<Id>(Id::new(SEARCH_FIELD_ID_KEY))) {
            ctx.memory_mut(|memory| memory.request_focus(id));
        }
    }
    if text_focused {
        return;
    }
    let pasted = ctx.input(|input| {
        input.events.iter().find_map(|event| match event {
            egui::Event::Paste(text) => Some(text.clone()),
            _ => None,
        })
    });
    if let Some(text) = pasted {
        state.paste_urls_from_text(&text);
    }
    if ctx.input_mut(|input| input.consume_key(command, egui::Key::Enter)) {
        state.start_queue_downloads_if_ready();
    }
    if standard_queue && ctx.input_mut(|input| input.consume_key(command, egui::Key::O)) {
        open_import_dialog(state);
    }
    if standard_queue && ctx.input_mut(|input| input.consume_key(command, egui::Key::R)) {
        state.retry_failed_queue_items();
    }
}

/// Import URLs from files dropped on the Main tab (`.txt`, `.url`, ...).
pub(super) fn handle_dropped_files(ctx: &egui::Context, state: &mut AppState, blocked: bool) {
    if blocked || state.active_tab != AppTab::Main || state.should_show_prepare_tab() {
        return;
    }
    let dropped = ctx.input(|input| {
        input
            .raw
            .dropped_files
            .iter()
            .map(|file| file.path().to_path_buf())
            .collect::<Vec<_>>()
    });
    if !dropped.is_empty() {
        state.import_urls_from_files(&dropped);
    }
}

pub(super) fn paint_drop_overlay(ctx: &egui::Context, state: &AppState) {
    if state.active_tab != AppTab::Main
        || state.should_show_prepare_tab()
        || ctx.input(|input| input.raw.hovered_files.is_empty())
    {
        return;
    }
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        Id::new("queue-drop-overlay"),
    ));
    let dark = ctx.global_style().visuals.dark_mode;
    let fill = if dark {
        Color32::from_black_alpha(170)
    } else {
        Color32::from_white_alpha(200)
    };
    painter.rect_filled(rect, 0.0, fill);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        state.ui_i18n_text_for_key("queue.drop_hint"),
        egui::FontId::proportional(18.0),
        ctx.global_style().visuals.strong_text_color(),
    );
}
