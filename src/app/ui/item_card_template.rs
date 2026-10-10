use egui_taffy::{Tui, TuiBuilderLogic as _, taffy};

use super::{semantic_ui_metrics, xaml_taffy_styles};
use crate::app::site_icon::{SiteIcon, site_icon_image};

/// Thumbnail image and branding share a layout-owned layer host.
pub(super) fn show_card_thumbnail(
    tui: &mut Tui,
    site_icon: Option<SiteIcon>,
    show: impl FnOnce(&mut eframe::egui::Ui),
) {
    tui.style(item_thumbnail_column_style()).add(|tui| {
        tui.style(xaml_taffy_styles::xaml_overlay_content_style())
            .ui(show);
        if let Some(icon) = site_icon {
            let size = semantic_ui_metrics::item_card_title_site_icon_size();
            let inset = semantic_ui_metrics::item_card_title_site_icon_to_text_gap();
            tui.style(taffy::Style {
                position: taffy::Position::Absolute,
                inset: taffy::Rect {
                    left: taffy::style_helpers::length(inset),
                    top: taffy::style_helpers::length(inset),
                    right: taffy::style_helpers::auto(),
                    bottom: taffy::style_helpers::auto(),
                },
                size: taffy::Size {
                    width: taffy::style_helpers::length(size),
                    height: taffy::style_helpers::length(size),
                },
                ..Default::default()
            })
            .ui(|ui| {
                ui.add(site_icon_image(icon, size));
            });
        }
    });
}

/// Queue cards share their parent's tree. Theme padding participates in measurement.
pub(super) fn show_queue_card(
    tui: &mut Tui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    show: impl FnOnce(&mut Tui, bool),
) {
    let padding = eframe::egui::Frame::group(tui.egui_ui().style()).inner_margin;
    let id = tui.current_id().with(id_salt);
    tui.id(id)
        .style(xaml_taffy_styles::xaml_padded_style(
            item_card_root_style(),
            padding,
        ))
        .add_with_border(|tui| {
            let hovered = tui
                .egui_ui()
                .rect_contains_pointer(tui.taffy_container().full_container());
            show(tui, hovered);
        });
}

pub(super) fn item_card_root_style() -> taffy::Style {
    xaml_taffy_styles::xaml_horizontal_auto_stack_style(
        semantic_ui_metrics::item_card_column_gap(),
        taffy::AlignItems::FlexStart,
    )
}

pub(super) fn item_thumbnail_column_style() -> taffy::Style {
    xaml_taffy_styles::xaml_fixed_size_flex_cell_style(
        semantic_ui_metrics::item_card_thumbnail_width(),
        semantic_ui_metrics::item_card_thumbnail_height(),
    )
}

pub(super) fn item_detail_column_style() -> taffy::Style {
    xaml_taffy_styles::xaml_weighted_width_auto_height_vertical_stack_style(
        semantic_ui_metrics::item_card_detail_row_gap(),
    )
}
