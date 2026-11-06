//! Product-level Settings controls backed by the XAML UiElement row contract.
//!
//! Each row is a real auto-height Taffy flex row. The page shares label-column
//! width and spacing, while every row derives its height from its own rendered
//! label and content cells.

use std::collections::BTreeMap;

use eframe::egui::{self, Align, Layout, Ui};

use crate::app::state::{AppState, DeferredSetting};
use crate::app::widgets::url_input::{AppTextBox, AppTextBoxSyntax};
use crate::i18n::Language;
use egui_taffy::Tui;

use super::common::settings_taffy_section;
use super::semantic_ui_metrics;
use super::xaml_layout_contracts::{
    HorizontalAlignment, LayoutLength, LayoutSize, UiElement, VerticalAlignment,
};
use super::xaml_taffy_styles::{XamlAutoHeightRowLayout, XamlTaffyElement};

pub(super) struct SettingsSelectOption<T> {
    pub(super) value: T,
    pub(super) label: String,
}

#[derive(Clone, Copy)]
pub(super) struct SettingsFormLayout {
    row: XamlAutoHeightRowLayout,
    label_width: f32,
    vertical_gap: f32,
}

impl SettingsFormLayout {
    pub(super) fn resolve(ui: &Ui, label_width: f32) -> Self {
        let column_gap = ui.spacing().item_spacing.x;
        Self {
            row: XamlAutoHeightRowLayout::new().with_column_gap(column_gap),
            label_width: label_width.max(0.0),
            vertical_gap:
                semantic_ui_metrics::settings_form_row_vertical_spacing_from_current_text_metrics(
                    ui,
                ),
        }
    }
}

impl<T> SettingsSelectOption<T> {
    pub(super) fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
        }
    }
}

pub(super) fn settings_section(tui: &mut Tui, title: &str, add_contents: impl FnOnce(&mut Tui)) {
    settings_taffy_section(tui, title, add_contents);
}

pub(super) fn settings_flexible_form_row(
    tui: &mut Tui,
    layout: SettingsFormLayout,
    label: &str,
    add_contents: impl FnOnce(&mut Ui),
) {
    let row = layout.row;
    XamlTaffyElement::fixed_height_spacer(layout.vertical_gap).show_empty(tui);
    row.show(tui, |tui| {
        let label_element = UiElement::label(LayoutSize::new(layout.label_width, 0.0))
            .with_height(LayoutLength::Auto)
            .with_content_alignment(HorizontalAlignment::Right, VerticalAlignment::Center);
        row.fixed_width_element_cell(label_element, layout.label_width)
            .show(tui, |ui| {
                show_settings_cell_content(ui, label_element, |ui| {
                    ui.label(label);
                });
            });

        let content_element = UiElement::auto_content_panel()
            .with_content_alignment(HorizontalAlignment::Left, VerticalAlignment::Center);
        row.star_width_element_cell(content_element)
            .show(tui, |ui| {
                show_settings_cell_content(ui, content_element, add_contents);
            });
    });
    XamlTaffyElement::fixed_height_spacer(layout.vertical_gap).show_empty(tui);
}

pub(super) fn settings_toggle_row(
    tui: &mut Tui,
    layout: SettingsFormLayout,
    label: &str,
    current: bool,
    control_label: &str,
) -> Option<bool> {
    let mut next = current;
    let mut changed = false;
    settings_flexible_form_row(tui, layout, label, |ui| {
        changed = ui.checkbox(&mut next, control_label).changed();
    });
    changed.then_some(next)
}

pub(super) fn settings_select_row<T: Clone + Eq>(
    tui: &mut Tui,
    layout: SettingsFormLayout,
    label: &str,
    id_salt: &'static str,
    current: T,
    selected_text: &str,
    options: &[SettingsSelectOption<T>],
) -> Option<T> {
    let mut selected = None;
    settings_flexible_form_row(tui, layout, label, |ui| {
        egui::ComboBox::from_id_salt(id_salt)
            .wrap()
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for option in options {
                    if ui
                        .selectable_label(current == option.value, option.label.as_str())
                        .clicked()
                    {
                        selected = Some(option.value.clone());
                    }
                }
            });
    });
    selected.filter(|value| *value != current)
}

pub(super) fn settings_navigation_row(
    tui: &mut Tui,
    layout: SettingsFormLayout,
    label: &str,
    button_label: &str,
) -> bool {
    let mut clicked = false;
    settings_flexible_form_row(tui, layout, label, |ui| {
        clicked = ui.button(format!("{button_label}  ›")).clicked();
    });
    clicked
}

fn show_settings_cell_content(ui: &mut Ui, element: UiElement, add_contents: impl FnOnce(&mut Ui)) {
    // `egui_taffy::ui` measures a finite leaf from the egui Ui's minimum
    // size. Calling `with_layout` directly here lets the child inherit the
    // complete Taffy max rect on the cross axis, so an auto-height row can
    // feed the remaining page height back into its own measurement.
    //
    // The Taffy cell already owns the width. Start the inner egui layout with
    // that slot width and no claimed height, then let its controls establish
    // the row's real intrinsic height.
    let slot_size = egui::vec2(ui.available_width().max(0.0), 0.0);

    match element.horizontal_content_alignment {
        HorizontalAlignment::Left | HorizontalAlignment::Stretch => {
            ui.allocate_ui_with_layout(
                slot_size,
                Layout::left_to_right(Align::Center),
                add_contents,
            );
        }
        HorizontalAlignment::Center => {
            ui.allocate_ui_with_layout(
                slot_size,
                Layout::centered_and_justified(egui::Direction::LeftToRight),
                add_contents,
            );
        }
        HorizontalAlignment::Right => {
            ui.allocate_ui_with_layout(
                slot_size,
                Layout::right_to_left(Align::Center),
                add_contents,
            );
        }
    }
}

/// Drafts of the deferred text boxes, with the frame each box last drew.
#[derive(Clone, Default)]
struct DeferredDrafts(BTreeMap<DeferredSetting, (String, u64)>);

fn deferred_drafts_id() -> egui::Id {
    egui::Id::new("deferred-setting-drafts")
}

/// A single-line text box whose edits stay a draft until editing ends:
/// focus moves elsewhere or Enter is pressed. Returns the new value once, at
/// that moment, when it differs from `current`. Use it for settings that are
/// saved to disk or used by running work, so neither sees half-typed input.
/// A draft whose box stops being drawn, for example because another page was
/// opened, is applied by [`commit_orphaned_setting_drafts`].
pub(super) fn deferred_text_box(
    ui: &mut Ui,
    setting: DeferredSetting,
    current: &str,
    hint: &str,
    language: Language,
    width: f32,
    enabled: bool,
) -> Option<String> {
    let frame = ui.ctx().cumulative_frame_nr();
    let mut text = ui
        .data(|data| {
            data.get_temp::<DeferredDrafts>(deferred_drafts_id())
                .and_then(|drafts| drafts.0.get(&setting).map(|(text, _)| text.clone()))
        })
        .unwrap_or_else(|| current.to_owned());
    let response = AppTextBox::new(&mut text)
        .hint_text(hint)
        .language(language)
        .syntax(AppTextBoxSyntax::Plain)
        .desired_width(width)
        .enabled(enabled)
        .ui(ui);
    let entered = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if entered {
        response.surrender_focus();
    }
    // Read focus before taking the data lock: `has_focus` locks the context too.
    let editing = response.has_focus() && !entered;
    ui.data_mut(|data| {
        let drafts = data.get_temp_mut_or_default::<DeferredDrafts>(deferred_drafts_id());
        if editing {
            drafts.0.insert(setting, (text.clone(), frame));
        } else {
            drafts.0.remove(&setting);
        }
    });
    if editing {
        return None;
    }
    ((entered || response.lost_focus()) && text != current).then_some(text)
}

/// Applies drafts whose text box was not drawn this frame, and every draft
/// when the window is closing. Call once per frame after the UI is drawn.
pub(super) fn commit_orphaned_setting_drafts(ctx: &egui::Context, state: &mut AppState) {
    let frame = ctx.cumulative_frame_nr();
    let closing = ctx.input(|input| input.viewport().close_requested());
    let orphaned = ctx.data_mut(|data| {
        let drafts = data.get_temp_mut_or_default::<DeferredDrafts>(deferred_drafts_id());
        let stale = drafts
            .0
            .iter()
            .filter(|(_, (_, drawn))| closing || *drawn < frame)
            .map(|(setting, (text, _))| (*setting, text.clone()))
            .collect::<Vec<_>>();
        for (setting, _) in &stale {
            drafts.0.remove(setting);
        }
        stale
    });
    for (setting, text) in orphaned {
        state.apply_deferred_setting(setting, &text);
    }
}
