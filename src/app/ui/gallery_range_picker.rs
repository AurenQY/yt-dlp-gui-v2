//! Gallery-only range page: measured Header, filter Content, draft Confirm/Back.
use super::gallery_range_filter;
use super::gallery_range_text;
use super::xaml_template_renderer::{
    show_full_height_page_template, show_template_ui_block, show_vertical_scroll_template,
};
use super::xaml_ui_nodes::{auto, block, fill, fixed_px, gap, rows};
use super::{semantic_ui_metrics as metrics, xaml_taffy_styles as styles};
use crate::app::state::{AppState, GalleryRangeSection};
use eframe::egui::{self, Ui};
use egui_taffy::TuiBuilderLogic as _;

#[derive(Clone, Copy)]
enum Node {
    Header,
    Separator,
    Content,
    Filter,
    Error,
}

pub(super) fn render(ui: &mut Ui, state: &mut AppState) {
    let row = metrics::xaml_format_picker_header_row_contract_from_current_control_metrics(ui);
    let back_text = state.ui_i18n_text_for_key("action.back");
    let confirm_text = state.ui_i18n_text_for_key("action.confirm");
    let filter_text = state.ui_i18n_text_for_key("picker.mode.filter");
    let button_style = |text| {
        styles::xaml_auto_width_cell_style(
            row,
            metrics::xaml_button_ui_element_for_visible_text(ui, text),
        )
        .1
    };
    let back_style = button_style(back_text);
    let confirm_style = button_style(confirm_text);
    let filter_style = button_style(filter_text);
    let help_style = button_style(crate::i18n::text(state.language(), "gallery.range.help"));
    let summary = state
        .gallery_range_picker
        .selected
        .as_ref()
        .map(|range| gallery_range_text::summary(range, state.language()));
    let summary_style = summary.as_deref().map(|text| {
        styles::xaml_shrinkable_auto_width_cell_style(
            row,
            super::xaml_layout_contracts::UiElement::label(
                super::xaml_layout_contracts::LayoutSize::new(
                    metrics::format_picker_header_summary_width_for_visible_text(ui, text),
                    row.height,
                ),
            ),
        )
        .1
    });
    let column_gap = ui.spacing().item_spacing.x;
    let vertical_gap = ui.spacing().item_spacing.y;
    let mut confirm = false;
    let mut back = false;
    show_full_height_page_template(
        ui,
        "gallery-range-picker",
        rows([
            fixed_px(row.height, block(Node::Header)),
            auto(block(Node::Separator)),
            fill(block(Node::Content)),
        ]),
        &mut |slot, node, tui| match node {
            Node::Header => {
                tui.style(styles::xaml_fixed_height_row_style(row, column_gap))
                    .add(|tui| {
                        tui.style(back_style.clone()).ui(|ui| {
                            ui.centered_and_justified(|ui| {
                                back = ui.button(back_text).clicked();
                            });
                        });
                        tui.style(filter_style.clone()).ui(|ui| {
                            ui.centered_and_justified(|ui| {
                                ui.add(egui::Button::new(filter_text).selected(true))
                                    .on_hover_text(crate::i18n::text(
                                        state.language(),
                                        "gallery.range.title",
                                    ));
                            });
                        });
                        tui.style(styles::xaml_flex_spacer_cell_style(row))
                            .ui(|_| {});
                        if let Some(style) = summary_style.clone() {
                            tui.style(style).ui(|ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(
                                                summary.as_deref().unwrap_or_default(),
                                            )
                                            .color(ui.visuals().weak_text_color()),
                                        )
                                        .truncate(),
                                    );
                                });
                            });
                        }
                        tui.style(help_style.clone()).ui(|ui| {
                            ui.centered_and_justified(|ui| {
                                ui.button(crate::i18n::text(
                                    state.language(),
                                    "gallery.range.help",
                                ))
                                .on_hover_ui(|ui| show_range_help(ui, state));
                            });
                        });
                        let allowed = state.gallery_range_picker.selected.is_some()
                            && (state.gallery_range_picker.section
                                != Some(GalleryRangeSection::Posts)
                                || state.gallery_post_range_available());
                        tui.style(confirm_style.clone()).ui(|ui| {
                            ui.centered_and_justified(|ui| {
                                confirm = ui
                                    .add_enabled(allowed, egui::Button::new(confirm_text))
                                    .clicked();
                            });
                        });
                    });
            }
            Node::Separator => show_template_ui_block(slot, tui, |ui| {
                ui.separator();
            }),
            Node::Content => {
                let mut content = vec![auto(block(Node::Filter))];
                if state.gallery_range_picker.error.is_some() {
                    content.push(gap(vertical_gap));
                    content.push(auto(block(Node::Error)));
                }
                show_vertical_scroll_template(slot, tui, rows(content), &mut |slot, node, tui| {
                    match node {
                        Node::Filter => gallery_range_filter::render(slot, tui, state),
                        _ => show_template_ui_block(slot, tui, |ui| {
                            render_custom_node(ui, state, node)
                        }),
                    }
                });
            }
            _ => unreachable!(),
        },
    );
    if back {
        state.gallery_range_picker = Default::default();
    } else if confirm {
        if let Some(range) = state.gallery_range_picker.selected.clone() {
            state.apply_gallery_range(range);
        }
    }
}

fn render_custom_node(ui: &mut Ui, state: &mut AppState, node: Node) {
    match node {
        Node::Error => {
            if let Some(error) = &state.gallery_range_picker.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        }
        _ => unreachable!(),
    }
}

fn show_range_help(ui: &mut Ui, state: &AppState) {
    ui.label(crate::i18n::text(state.language(), "gallery.range.title"));
    ui.label(crate::i18n::text(
        state.language(),
        "gallery.range.help_steps",
    ));
    if !state.gallery_post_range_available() {
        ui.label(crate::i18n::text(
            state.language(),
            "gallery.range.help_multiple",
        ));
    }
    ui.label(crate::i18n::text(
        state.language(),
        "gallery.range.help_order",
    ));
    ui.label(crate::i18n::text(
        state.language(),
        "gallery.range.help_time",
    ));
}
