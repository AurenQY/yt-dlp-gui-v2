use eframe::egui::{ScrollArea, Ui};
use egui_taffy::Tui;

use crate::app::state::AppState;

use super::common::settings_taffy_scroll_content;
use super::semantic_ui_metrics;
use super::settings_components::SettingsFormLayout;
use super::xaml_template_renderer::show_auto_height_tui_template;
use super::xaml_ui_nodes::{TemplateNode, auto_block_rows};

#[derive(Clone, Copy)]
enum AdvanceRootNode {
    ConfigSource,
    Network,
    PostProcessing,
    DownloadProcessing,
    Aria2,
}

type AdvanceRootTemplate = TemplateNode<AdvanceRootNode>;

#[derive(Clone, Copy)]
struct AdvanceLayoutMetrics {
    label_width: f32,
    form: SettingsFormLayout,
}

pub(super) fn render_advance_root_page(ui: &mut Ui, state: &mut AppState) {
    let restored_offset = state.advance_root_scroll_offset();
    let output = ScrollArea::vertical()
        .id_salt("advance-tab-scroll")
        .vertical_scroll_offset(restored_offset)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let metrics = advance_layout_metrics(ui, state);
            settings_taffy_scroll_content(ui, "advance-root-settings-taffy", |tui| {
                show_advance_root_template(advance_root_template(), tui, state, metrics);
            });
        });
    state.remember_advance_root_scroll_offset(output.state.offset.y);
}

fn advance_root_template() -> AdvanceRootTemplate {
    auto_block_rows([
        AdvanceRootNode::ConfigSource,
        AdvanceRootNode::Network,
        AdvanceRootNode::PostProcessing,
        AdvanceRootNode::DownloadProcessing,
        AdvanceRootNode::Aria2,
    ])
}

fn show_advance_root_template(
    template: AdvanceRootTemplate,
    tui: &mut Tui,
    state: &mut AppState,
    metrics: AdvanceLayoutMetrics,
) {
    let mut show_block = |node, tui: &mut Tui| {
        show_advance_root_block(node, tui, state, metrics);
    };
    show_auto_height_tui_template(template, tui, &mut show_block);
}

fn show_advance_root_block(
    node: AdvanceRootNode,
    tui: &mut Tui,
    state: &mut AppState,
    metrics: AdvanceLayoutMetrics,
) {
    let label_width = metrics.label_width;
    let form = metrics.form;
    match node {
        AdvanceRootNode::ConfigSource => {
            super::advance_source::render_config_source_section(tui, state, form)
        }
        AdvanceRootNode::Network => {
            super::advance_network::render_network_section(tui, state, label_width)
        }
        AdvanceRootNode::PostProcessing => {
            super::advance_post_processing::render_post_processing_section(tui, state, label_width)
        }
        AdvanceRootNode::DownloadProcessing => {
            super::advance_download_controls::render_download_processing_section(tui, state, form)
        }
        AdvanceRootNode::Aria2 => {
            super::advance_download_controls::render_aria2_section(tui, state, form)
        }
    }
}

fn advance_layout_metrics(ui: &Ui, state: &AppState) -> AdvanceLayoutMetrics {
    let label_width = advance_label_width(ui, state);
    AdvanceLayoutMetrics {
        label_width,
        form: SettingsFormLayout::resolve(ui, label_width),
    }
}

fn advance_label_width(ui: &Ui, state: &AppState) -> f32 {
    semantic_ui_metrics::settings_form_label_column_width_for_translated_label_keys(
        ui,
        state,
        &[
            "advance.config",
            "advance.proxy",
            "advance.certificate",
            "advance.cookie",
            "advance.cookie_file_source",
            "advance.cookie_manager_row",
            "advance.file",
            "advance.browser",
            "advance.external_downloader",
            "advance.concurrent_fragments",
            "advance.rate_limit",
            "advance.chapters",
            "advance.file_time",
            "advance.thumbnail",
            "advance.subtitles",
            "advance.download_conversion",
        ],
    )
}
