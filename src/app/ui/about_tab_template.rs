use eframe::egui::Ui;
use egui_taffy::Tui;

use super::about_tab_controls::{AboutComponentCard, AboutFooterRow, AboutRowMetrics};
use super::semantic_ui_metrics;
use super::xaml_template_renderer::{
    TemplateBlockSlot, show_full_height_page_template, show_template_tui_block,
    show_template_ui_block, show_vertical_scroll_template,
};
use super::xaml_ui_nodes::{self, auto, block, fill, gap, rows};
use crate::app::state::AppState;
use crate::infrastructure::ManagedComponentId;

// ABOUT TEMPLATE RULES — this file owns the TemplateTree shape only.
//
// Existing project pattern to follow before editing:
// - `main_tab_template.rs`: resolve frame/context/role objects, then dispatch blocks.
// - `options_tab_template.rs`: root page describes section slots; section modules render content.
// - `xaml_ui_nodes.rs`: rows/cols/auto/fill/fixed_px/gap/block are the layout vocabulary.
//
// Rules for this file:
// 1. Describe page structure with TemplateTree nodes only.
// 2. Do not hand-build egui alignment here. No `ui.with_layout`, no spacer math.
// 3. Row/cell UiElement details belong to `about_tab_controls.rs`.
// 4. Slot painting belongs to `about_tab.rs`.
// 5. Scroll content and cards remain descendants of the one page root.
//
// 每次處理中，如有發現不符合地方應強力修正；不要只修表面症狀。

type AboutTabTemplateTree = xaml_ui_nodes::TemplateNode<AboutTabNode>;

pub(super) fn render_about_tab_template(ui: &mut Ui, state: &mut AppState) {
    let template = AboutTabTemplate::resolve(ui, state);
    template.show(ui, state);
}

struct AboutTabTemplate {
    root: AboutTabTemplateTree,
}

#[derive(Clone)]
enum AboutTabNode {
    ComponentCard(AboutComponentCard),
    ComponentPool(Box<AboutTabTemplateTree>),
    SectionHeading(AboutSection),
    FooterRow(AboutFooterRow),
}

#[derive(Clone, Copy)]
enum AboutSection {
    CoreTools,
    ExtensionTools,
}

impl AboutSection {
    fn title_key(self) -> &'static str {
        match self {
            Self::CoreTools => "about.core_tools",
            Self::ExtensionTools => "about.extension_tools",
        }
    }
}

impl AboutTabTemplate {
    fn resolve(ui: &mut Ui, state: &mut AppState) -> Self {
        let component_ids = super::about_tab::about_component_ids();
        let row_metrics = AboutRowMetrics::resolve(ui, state, &component_ids);
        let root = about_tab_root_template(row_metrics, component_ids);

        Self { root }
    }

    fn show(self, ui: &mut Ui, state: &mut AppState) {
        let mut show_block = |slot, node, tui: &mut Tui| {
            show_about_tab_block(slot, node, tui, state);
        };
        show_full_height_page_template(ui, "about-tab-template", self.root, &mut show_block);
    }
}

fn about_tab_root_template(
    row_metrics: AboutRowMetrics,
    component_ids: Vec<crate::infrastructure::ManagedComponentId>,
) -> AboutTabTemplateTree {
    let mut items = vec![
        auto(block(AboutTabNode::SectionHeading(AboutSection::CoreTools))),
        gap(semantic_ui_metrics::main_section_vertical_spacing()),
    ];
    let mut component_ids = component_ids
        .into_iter()
        .filter(|id| *id != ManagedComponentId::App)
        .peekable();
    while let Some(id) = component_ids.next() {
        if id == ManagedComponentId::GalleryDl {
            items.push(auto(block(AboutTabNode::SectionHeading(
                AboutSection::ExtensionTools,
            ))));
            items.push(gap(semantic_ui_metrics::main_section_vertical_spacing()));
        }
        items.push(auto(block(AboutTabNode::ComponentCard(
            row_metrics.component_card(id),
        ))));
        if component_ids.peek().is_some() {
            items.push(gap(semantic_ui_metrics::about_component_card_vertical_gap()));
        }
    }
    rows(vec![
        auto(block(AboutTabNode::ComponentCard(
            row_metrics.component_card(ManagedComponentId::App),
        ))),
        gap(semantic_ui_metrics::about_component_card_vertical_gap()),
        fill(block(AboutTabNode::ComponentPool(Box::new(rows(items))))),
        gap(semantic_ui_metrics::main_section_vertical_spacing()),
        auto(block(AboutTabNode::FooterRow(row_metrics.footer_row()))),
        gap(semantic_ui_metrics::main_bottom_trailing_vertical_spacing()),
    ])
}

fn show_about_tab_block(
    slot: TemplateBlockSlot,
    node: AboutTabNode,
    tui: &mut Tui,
    state: &mut AppState,
) {
    match node {
        AboutTabNode::ComponentCard(card) => show_template_tui_block(slot, tui, |tui| {
            card.show(tui, state);
        }),
        AboutTabNode::ComponentPool(pool) => {
            show_vertical_scroll_template(slot, tui, *pool, &mut |slot, node, tui| {
                show_about_tab_block(slot, node, tui, state);
            });
        }
        AboutTabNode::SectionHeading(section) => show_template_ui_block(slot, tui, |ui| {
            super::about_tab::render_about_section_heading(ui, state, section.title_key());
        }),
        AboutTabNode::FooterRow(row) => show_template_tui_block(slot, tui, |tui| {
            row.show(tui, state);
        }),
    }
}
