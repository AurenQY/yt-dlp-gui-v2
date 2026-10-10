use eframe::egui::Ui;
use egui_taffy::{Tui, TuiBuilderLogic as _};

use crate::app::state::AppState;
use crate::infrastructure::{DependencyTool, ManagedComponentId};

use super::item_card_template::item_card_root_style;
use super::xaml_layout_contracts::{HorizontalAlignment, LayoutSize, UiElement, VerticalAlignment};
use super::{semantic_ui_metrics, xaml_taffy_styles};

#[derive(Debug, Clone, Copy)]
pub(super) struct AboutRowMetrics {
    card_padding: eframe::egui::Margin,
    row: xaml_taffy_styles::XamlSingleLineRowLayout,
    link_row: xaml_taffy_styles::XamlSingleLineRowLayout,
    header_row: xaml_taffy_styles::XamlSingleLineRowLayout,
    app_detail_row: xaml_taffy_styles::XamlSingleLineRowLayout,
    row_height: f32,
    action_width: f32,
    check_button_width: f32,
    update_all_button_width: f32,
    install_dependencies_button_width: f32,
    name_width: f32,
    status_widths: [(ManagedComponentId, f32); 6],
    browse_button_width: f32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AboutFooterRow {
    metrics: AboutRowMetrics,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AboutComponentCard {
    metrics: AboutRowMetrics,
    id: ManagedComponentId,
}

impl AboutRowMetrics {
    pub(super) fn resolve(ui: &Ui, state: &AppState, component_ids: &[ManagedComponentId]) -> Self {
        // Share the standard mode's single-line footer row contract.
        let row =
            super::main_tab_template::main_single_line_row_layout(ui, ui.spacing().interact_size.y);

        Self {
            card_padding: eframe::egui::Frame::group(ui.style()).inner_margin,
            row,
            link_row: xaml_taffy_styles::XamlSingleLineRowLayout::new(
                semantic_ui_metrics::xaml_single_line_control_row_contract_from_height(
                    semantic_ui_metrics::about_component_link_row_height(ui),
                ),
            ),
            header_row: xaml_taffy_styles::XamlSingleLineRowLayout::new(
                semantic_ui_metrics::xaml_single_line_control_row_contract_from_height(
                    semantic_ui_metrics::about_component_header_row_height(ui),
                ),
            ),
            app_detail_row: xaml_taffy_styles::XamlSingleLineRowLayout::new(
                semantic_ui_metrics::xaml_single_line_control_row_contract_from_height(
                    semantic_ui_metrics::about_app_detail_row_height(ui),
                ),
            ),
            row_height: row.height(),
            action_width: super::about_tab::about_action_slot_width(ui, state),
            name_width: component_ids
                .iter()
                .copied()
                .chain(std::iter::once(ManagedComponentId::App))
                .map(|id| {
                    super::measure::text_width(
                        ui,
                        super::about_tab::component_display_name(id),
                        eframe::egui::TextStyle::Body,
                    )
                })
                .fold(0.0, f32::max)
                + ui.spacing().item_spacing.x * 2.0,
            // Independent status width: an absent optional dependency must not
            // reserve the largest badge width of an unrelated Item.
            status_widths: ManagedComponentId::ALL.map(|id| {
                (
                    id,
                    super::about_tab::about_header_status_width(ui, state, id),
                )
            }),
            check_button_width: super::about_tab::about_check_button_width(ui, state),
            update_all_button_width: super::about_tab::about_update_all_button_width(ui, state),
            install_dependencies_button_width:
                super::about_tab::about_install_dependencies_button_width(ui, state),
            browse_button_width:
                semantic_ui_metrics::standard_icon_text_button_width_for_visible_text(
                    ui,
                    state.ui_i18n_text_for_key("advance.browse"),
                ),
        }
    }

    pub(super) fn footer_row(self) -> AboutFooterRow {
        AboutFooterRow { metrics: self }
    }

    pub(super) fn component_card(self, id: ManagedComponentId) -> AboutComponentCard {
        AboutComponentCard { metrics: self, id }
    }

    fn status_width_for(self, id: ManagedComponentId) -> f32 {
        self.status_widths
            .iter()
            .find_map(|(candidate, width)| (*candidate == id).then_some(*width))
            .unwrap_or(0.0)
    }
}

impl AboutFooterRow {
    pub(super) fn show(self, tui: &mut Tui, state: &mut AppState) {
        let metrics = self.metrics;
        metrics.row.show(tui, |tui| {
            show_header_star_cell(tui, metrics, |ui| {
                super::about_tab::render_about_footer_message(ui, state)
            });
            if state.has_missing_required_dependencies() {
                show_header_fixed_cell(
                    tui,
                    metrics,
                    metrics.install_dependencies_button_width,
                    UiElement::icon_text_button,
                    |ui| super::about_tab::render_install_dependencies_button(ui, state),
                );
            }
            show_header_fixed_cell(
                tui,
                metrics,
                metrics.check_button_width,
                UiElement::icon_text_button,
                |ui| super::about_tab::render_check_updates_button(ui, state),
            );
            show_header_fixed_cell(
                tui,
                metrics,
                metrics.update_all_button_width,
                UiElement::icon_text_button,
                |ui| super::about_tab::render_update_all_button(ui, state),
            );
        });
    }
}

// Every card stays in the page's existing layout tree, including scroll content.
impl AboutComponentCard {
    pub(super) fn show(self, tui: &mut Tui, state: &mut AppState) {
        if self.id == ManagedComponentId::App {
            // The App is borderless, but its icon/text column must start at the
            // same themed *content* edge as bordered dependency Items. Reserve
            // the group-frame border in the Taffy padding contract, not with a
            // painter offset or a hand-positioned icon.
            let group_frame = eframe::egui::Frame::group(tui.egui_ui().style());
            let total_margin = group_frame.total_margin();
            // Frame::total_margin() returns floating-point pixels, while the
            // egui::Margin consumed by the template padding contract uses i8.
            // Convert only at this type boundary; preserve the theme metrics.
            let app_padding = eframe::egui::Margin {
                left: total_margin.left.round() as i8,
                right: total_margin.right.round() as i8,
                top: self.metrics.card_padding.top,
                bottom: self.metrics.card_padding.bottom,
            };
            let layout = xaml_taffy_styles::xaml_padded_style(item_card_root_style(), app_padding);
            tui.style(layout).add(|tui| self.show_contents(tui, state));
        } else {
            let layout = xaml_taffy_styles::xaml_padded_style(
                item_card_root_style(),
                self.metrics.card_padding,
            );
            tui.style(layout)
                .add_with_border(|tui| self.show_contents(tui, state));
        }
    }

    fn show_contents(self, tui: &mut Tui, state: &mut AppState) {
        let id = self.id;
        // The application card owns a separate two-row layout. Dependency cards
        // keep the three-row template and the full-height path control.
        let thumbnail_side = if id == ManagedComponentId::App {
            semantic_ui_metrics::about_app_thumbnail_side()
        } else {
            semantic_ui_metrics::about_component_thumbnail_side()
        };
        tui.style(xaml_taffy_styles::xaml_fixed_size_flex_cell_style(
            thumbnail_side,
            thumbnail_side,
        ))
        .add(|tui| {
            let icon_size =
                semantic_ui_metrics::about_component_icon_size_from_item_height(thumbnail_side);
            tui.style(xaml_taffy_styles::xaml_centered_fixed_size_cell_style(
                icon_size, icon_size,
            ))
            .ui(|ui| super::about_tab::render_component_icon_cell(ui, id, icon_size));
        });
        tui.style(
            xaml_taffy_styles::xaml_weighted_width_auto_height_vertical_stack_style(
                semantic_ui_metrics::about_component_detail_row_gap(),
            ),
        )
        .add(|tui| {
            show_component_header(tui, self.metrics, state, id);
            if let Some(tool) = id.as_dependency_tool() {
                if let Some(description_key) = extension_tool_description_key(id) {
                    show_component_description(tui, self.metrics, state, description_key);
                }
                show_component_project_link(tui, self.metrics, id);
                show_tool_path_row(tui, self.metrics, state, tool);
            } else {
                show_app_info_row(tui, self.metrics, state);
            }
        });
    }
}

/// Extension Items alone get one localized explanatory row between the
/// title and project URL; core Items keep their existing three-row template.
fn extension_tool_description_key(id: ManagedComponentId) -> Option<&'static str> {
    match id {
        ManagedComponentId::GalleryDl => Some("about.gallery_dl_description"),
        ManagedComponentId::Aria2c => Some("about.aria2_description"),
        _ => None,
    }
}

fn show_component_description(
    tui: &mut Tui,
    metrics: AboutRowMetrics,
    state: &AppState,
    description_key: &'static str,
) {
    metrics.link_row.show(tui, |tui| {
        show_row_star_cell(tui, metrics.link_row, |ui| {
            let description = state.ui_i18n_text_for_key(description_key);
            ui.add(
                eframe::egui::Label::new(
                    eframe::egui::RichText::new(description)
                        .small()
                        .color(ui.visuals().weak_text_color()),
                )
                .truncate(),
            );
        });
    });
}

fn show_component_project_link(tui: &mut Tui, metrics: AboutRowMetrics, id: ManagedComponentId) {
    metrics.link_row.show(tui, |tui| {
        show_row_star_cell(tui, metrics.link_row, |ui| {
            super::about_tab::render_component_project_link(ui, id);
        });
    });
}

fn show_component_header(
    tui: &mut Tui,
    metrics: AboutRowMetrics,
    state: &mut AppState,
    id: ManagedComponentId,
) {
    metrics.header_row.show(tui, |tui| {
        show_row_fixed_cell(
            tui,
            metrics.header_row,
            metrics.name_width,
            HorizontalAlignment::Left,
            |ui| super::about_tab::render_component_title(ui, id),
        );
        show_row_star_cell(tui, metrics.header_row, |ui| {
            super::about_tab::render_component_current_version(ui, state, id);
        });
        let status_width = metrics.status_width_for(id);
        if status_width > 0.0 {
            show_row_fixed_cell(
                tui,
                metrics.header_row,
                status_width,
                HorizontalAlignment::Right,
                |ui| super::about_tab::render_about_header_status(ui, state, id),
            );
        }
    });
}

// App-only two-line layout: first row shows title and inline current → newer
// version, just like dependency Items; second row shows the homepage and the
// existing update/restart action. No duplicate latest-version label or badge.
fn show_app_info_row(tui: &mut Tui, metrics: AboutRowMetrics, state: &mut AppState) {
    let has_action = super::about_tab::about_app_has_action(state);
    let row = if has_action {
        metrics.row
    } else {
        metrics.app_detail_row
    };
    row.show(tui, |tui| {
        show_row_star_cell(tui, row, |ui| {
            super::about_tab::render_component_project_link(ui, ManagedComponentId::App);
        });
        if has_action {
            show_header_fixed_cell(
                tui,
                metrics,
                metrics.action_width,
                UiElement::icon_text_button,
                |ui| {
                    super::about_tab::render_component_action_cell(
                        ui,
                        state,
                        ManagedComponentId::App,
                    )
                },
            );
        }
    });
}

fn show_tool_path_row(
    tui: &mut Tui,
    metrics: AboutRowMetrics,
    state: &mut AppState,
    tool: DependencyTool,
) {
    metrics.row.show(tui, |tui| {
        let cell = metrics.row.star_width_text_box_cell();
        cell.show(tui, |ui| {
            let size = cell.measured_size_for_ui(ui);
            super::about_tab::render_tool_path_text_box(ui, state, tool, size);
        });
        if super::about_tab::about_tool_has_action(state, tool) {
            show_header_fixed_cell(
                tui,
                metrics,
                metrics.action_width,
                UiElement::icon_text_button,
                |ui| {
                    super::about_tab::render_component_action_cell(
                        ui,
                        state,
                        ManagedComponentId::for_dependency_tool(tool),
                    )
                },
            );
        }
        show_header_fixed_cell(
            tui,
            metrics,
            metrics.browse_button_width,
            UiElement::icon_text_button,
            |ui| super::about_tab::render_tool_browse_button(ui, state, tool),
        );
    });
}

fn show_row_star_cell(
    tui: &mut Tui,
    row: xaml_taffy_styles::XamlSingleLineRowLayout,
    add_contents: impl FnOnce(&mut Ui),
) {
    let element = UiElement::stretch_width_stretch_height(LayoutSize::new(0.0, row.height()))
        .with_content_alignment(HorizontalAlignment::Left, VerticalAlignment::Center);
    row.star_width_element_cell(element, 0.0, 1.0)
        .show_aligned(tui, add_contents);
}

fn show_row_fixed_cell(
    tui: &mut Tui,
    row: xaml_taffy_styles::XamlSingleLineRowLayout,
    width: f32,
    horizontal_alignment: HorizontalAlignment,
    add_contents: impl FnOnce(&mut Ui),
) {
    let width = width.max(0.0);
    let element = UiElement::stretch_width_stretch_height(LayoutSize::new(width, row.height()))
        .with_content_alignment(horizontal_alignment, VerticalAlignment::Center);
    row.fixed_width_element_cell(element, width)
        .show_aligned(tui, add_contents);
}

fn show_header_star_cell(
    tui: &mut Tui,
    metrics: AboutRowMetrics,
    add_contents: impl FnOnce(&mut Ui),
) {
    show_row_star_cell(tui, metrics.row, add_contents);
}

fn show_header_fixed_cell(
    tui: &mut Tui,
    metrics: AboutRowMetrics,
    width: f32,
    make_element: impl FnOnce(LayoutSize) -> UiElement,
    add_contents: impl FnOnce(&mut Ui),
) {
    let width = width.max(0.0);
    let element = make_element(LayoutSize::new(width, metrics.row_height))
        .with_content_alignment(HorizontalAlignment::Center, VerticalAlignment::Center);
    metrics
        .row
        .fixed_width_element_cell(element, width)
        .show_aligned(tui, add_contents);
}
