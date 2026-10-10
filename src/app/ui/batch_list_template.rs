//! Standard download queue layout. All nodes descend from the Main page Tui.
use super::item_card;
use super::xaml_template_renderer::{
    TemplateBlockSlot, show_template, show_template_tui_block, show_vertical_scroll_template,
};
use super::xaml_ui_nodes::{TemplateNode, auto, block, fill, gap, rows};
use super::{semantic_ui_metrics, xaml_taffy_styles};
use crate::app::state::AppState;
use crate::domain::QueueBackend;
use crate::infrastructure::DownloadTargetKind;
use egui_taffy::{Tui, TuiBuilderLogic as _};

#[derive(Default)]
pub(super) struct QueueCardActions {
    pub remove: Option<u64>,
    pub cancel: Option<u64>,
    pub gallery_download: Option<u64>,
    pub export: Option<(u64, DownloadTargetKind)>,
}

enum QueueNode {
    Toolbar,
    Pool(Box<TemplateNode<QueueNode>>),
    Item { index: usize, id: u64 },
    Empty,
}

pub(super) fn show_batch_list(
    tui: &mut Tui,
    state: &mut AppState,
    bottom_safe_area: f32,
    music_window_material_progress: f32,
) {
    let spacing = tui.egui_ui().spacing().item_spacing.y;
    let mut items = Vec::new();
    if state.queue_items.is_empty() {
        items.push(auto(block(QueueNode::Empty)));
    } else {
        for (index, item) in state.queue_items.iter().enumerate() {
            items.push(auto(block(QueueNode::Item { index, id: item.id })));
            items.push(gap(spacing));
        }
    }
    items.push(gap(bottom_safe_area.max(0.0)));
    let template = rows(vec![
        auto(block(QueueNode::Toolbar)),
        gap(spacing),
        fill(block(QueueNode::Pool(Box::new(rows(items))))),
    ]);
    let mut actions = QueueCardActions::default();
    show_template(template, tui, &mut |slot, node, tui| {
        show_queue_node(
            slot,
            node,
            tui,
            state,
            music_window_material_progress,
            &mut actions,
        );
    });
    if let Some(id) = actions.cancel {
        state.cancel_item_download(id);
    }
    if let Some(id) = actions.remove {
        state.remove_queue_item(id);
    }
    if let Some(id) = actions.gallery_download {
        if let Err(error) = state.start_gallery_download(id) {
            state.set_last_action_message(error);
        }
    }
    if let Some((id, kind)) = actions.export {
        item_card::open_export_dialog(state, id, kind);
    }
}

fn show_queue_node(
    slot: TemplateBlockSlot,
    node: QueueNode,
    tui: &mut Tui,
    state: &mut AppState,
    music_progress: f32,
    actions: &mut QueueCardActions,
) {
    match node {
        QueueNode::Toolbar => show_template_tui_block(slot, tui, |tui| show_toolbar(tui, state)),
        QueueNode::Pool(tree) => {
            show_vertical_scroll_template(slot, tui, *tree, &mut |slot, node, tui| {
                show_queue_node(slot, node, tui, state, music_progress, actions);
            })
        }
        QueueNode::Item { index, id } => {
            // Keep node identity stable when a preceding item is removed.
            let mut style = match slot {
                TemplateBlockSlot::Child { style, .. } => style,
                TemplateBlockSlot::Root => xaml_taffy_styles::xaml_auto_height_block_style(),
            };
            style.flex_shrink = 0.0;
            tui.id(tui.current_id().with(("queue-item", id)))
                .style(style)
                .add(|tui| {
                    // Toolbar actions may change the queue before these nodes are painted.
                    if state
                        .queue_items
                        .get(index)
                        .is_none_or(|item| item.id != id)
                    {
                        return;
                    }
                    if matches!(state.queue_items[index].backend, QueueBackend::GalleryDl(_)) {
                        use super::item_card_gallery::{
                            GalleryCardAction, render_gallery_queue_item,
                        };
                        match render_gallery_queue_item(tui, state, index) {
                            GalleryCardAction::None => {}
                            GalleryCardAction::Remove(id) => actions.remove = Some(id),
                            GalleryCardAction::Stop(id) => actions.cancel = Some(id),
                            GalleryCardAction::Download(id) => actions.gallery_download = Some(id),
                        }
                    } else if state.queue_display_mode_is_audio() {
                        // The compact row is one custom-painted widget, not another Tui root.
                        tui.style(xaml_taffy_styles::xaml_auto_height_block_style())
                            .ui(|ui| {
                                if let Some(id) =
                                    super::item_card_compact::render_music_queue_item_row(
                                        ui,
                                        state,
                                        index,
                                        music_progress,
                                    )
                                {
                                    actions.remove = Some(id);
                                }
                            });
                    } else {
                        let next = item_card::render_video_queue_item(tui, state, index);
                        if next.remove.is_some() {
                            actions.remove = next.remove;
                        }
                        if next.cancel.is_some() {
                            actions.cancel = next.cancel;
                        }
                        if next.export.is_some() {
                            actions.export = next.export;
                        }
                    }
                });
        }
        QueueNode::Empty => show_template_tui_block(slot, tui, |tui| {
            if state.queue_display_mode_is_audio() {
                tui.style(xaml_taffy_styles::xaml_auto_height_block_style())
                    .ui(|ui| {
                        super::item_card_compact::render_empty_music_compact_item(
                            ui,
                            state,
                            music_progress,
                        );
                    });
            } else {
                item_card::render_empty_batch_item_card(tui, state, &state.empty_item_preview);
            }
        }),
    }
}

fn show_toolbar(tui: &mut Tui, state: &mut AppState) {
    let ui = tui.egui_ui();
    let row = xaml_taffy_styles::XamlSingleLineRowLayout::new(
        semantic_ui_metrics::xaml_single_line_control_row_contract_from_height(
            ui.spacing().interact_size.y,
        ),
    )
    .with_column_gap(ui.spacing().item_spacing.x);
    let button = semantic_ui_metrics::xaml_icon_text_button_ui_element_for_visible_text(
        ui,
        state.ui_i18n_text_for_key("item.clear_all"),
    );
    let size = row.measure_auto_width_element(button).size;
    row.show(tui, |tui| {
        row.star_width_element_cell(
            super::xaml_layout_contracts::UiElement::stretch_width_stretch_height(
                super::xaml_layout_contracts::LayoutSize::new(0.0, row.height()),
            )
            .with_content_alignment(
                super::xaml_layout_contracts::HorizontalAlignment::Left,
                super::xaml_layout_contracts::VerticalAlignment::Center,
            ),
            0.0,
            1.0,
        )
        .show(tui, |ui| item_card::render_queue_summary(ui, state));
        row.fixed_width_element_cell(button, size.width)
            .show(tui, |ui| item_card::render_clear_queue_button(ui, state));
    });
}
