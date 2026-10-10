//! Classic Gallery feedback/folder in the format slot; bottom controls own Download/Stop.
use crate::app::state::AppState;
use crate::domain::QueueBackend;
use egui_taffy::Tui;

use super::item_card_gallery::{feedback::GalleryFeedback, fields};

pub(super) fn row_count(state: &AppState) -> usize {
    if state
        .queue_items
        .first()
        .is_some_and(|item| {
            !item.needs_analysis_refresh
                && matches!(&item.backend, QueueBackend::GalleryDl(gallery) if gallery.analysis.is_some())
        })
    {
        2
    } else {
        0
    }
}

pub(super) fn render(tui: &mut Tui, state: &mut AppState) {
    if row_count(state) == 0 {
        return;
    }
    let Some(feedback) = GalleryFeedback::from_state(state, 0) else {
        return;
    };
    let item_id = state.queue_items[0].id;
    let selectable = feedback.timeline && !state.item_is_busy(0);
    if fields::classic_range_row(
        tui,
        if feedback.timeline {
            state.ui_i18n_text_for_key("media.section")
        } else {
            state.ui_i18n_text_for_key("about.status")
        },
        &feedback.range_text,
        &feedback.counter,
        selectable,
        state.language(),
    ) {
        state.open_gallery_range_picker(item_id);
    }
    fields::folder_row(
        tui,
        state,
        item_id,
        feedback.folder.as_deref(),
        &feedback.folder_text,
        (feedback.running || feedback.finished) && !feedback.stopping,
        feedback.completed,
        feedback.latest_file.as_deref(),
    );
}
