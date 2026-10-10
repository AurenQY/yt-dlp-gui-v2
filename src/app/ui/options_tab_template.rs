use eframe::egui::{ScrollArea, Ui};
use egui_taffy::Tui;

use crate::app::state::AppState;

use super::common::settings_taffy_scroll_content;
use super::options_layout::OptionsLayoutMetrics;
use super::xaml_template_renderer::show_auto_height_tui_template;
use super::xaml_ui_nodes::{TemplateNode, auto_block_rows};

#[derive(Clone, Copy)]
enum OptionsRootNode {
    Language,
    BrowserIntegration,
    Behavior,
    FormatList,
    Tabs,
    Playlist,
    FileAction,
    Cache,
    Window,
}

type OptionsRootTemplate = TemplateNode<OptionsRootNode>;

pub(super) fn render_options_root_page(ui: &mut Ui, state: &mut AppState) {
    let restored_offset = state.options_root_scroll_offset();
    let output = ScrollArea::vertical()
        .id_salt("options-tab-scroll")
        .vertical_scroll_offset(restored_offset)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let metrics = OptionsLayoutMetrics::new(ui, state);
            settings_taffy_scroll_content(ui, "options-root-settings-taffy", |tui| {
                show_options_root_template(options_root_template(), tui, state, metrics);
            });
        });
    state.remember_options_root_scroll_offset(output.state.offset.y);
}

fn options_root_template() -> OptionsRootTemplate {
    auto_block_rows([
        OptionsRootNode::Language,
        OptionsRootNode::BrowserIntegration,
        OptionsRootNode::Behavior,
        OptionsRootNode::FormatList,
        OptionsRootNode::Tabs,
        OptionsRootNode::Playlist,
        OptionsRootNode::FileAction,
        OptionsRootNode::Cache,
        OptionsRootNode::Window,
    ])
}

fn show_options_root_template(
    template: OptionsRootTemplate,
    tui: &mut Tui,
    state: &mut AppState,
    metrics: OptionsLayoutMetrics,
) {
    let mut show_block = |node, tui: &mut Tui| {
        show_options_root_block(node, tui, state, metrics);
    };
    show_auto_height_tui_template(template, tui, &mut show_block);
}

fn show_options_root_block(
    node: OptionsRootNode,
    tui: &mut Tui,
    state: &mut AppState,
    metrics: OptionsLayoutMetrics,
) {
    let label_width = metrics.label_width;
    let form = metrics.form;
    match node {
        OptionsRootNode::BrowserIntegration => {
            super::options_browser_integration::render_browser_integration_section(tui, state, form)
        }
        OptionsRootNode::Language => {
            super::options_language::render_language_group(tui, state, form)
        }
        OptionsRootNode::Behavior => {
            super::options_behavior::render_behavior_group(tui, state, form)
        }
        OptionsRootNode::FormatList => {
            super::options_formats::render_format_list_group(tui, state, form)
        }
        OptionsRootNode::Tabs => super::options_behavior::render_tabs_group(tui, state, form),
        OptionsRootNode::Playlist => {
            super::options_behavior::render_playlist_group(tui, state, form)
        }
        OptionsRootNode::FileAction => {
            super::options_behavior::render_file_action_group(tui, state, form)
        }
        OptionsRootNode::Cache => super::options_cache::render_cache_group(tui, state, label_width),
        OptionsRootNode::Window => super::options_window::render_window_group(tui, state, form),
    }
}
