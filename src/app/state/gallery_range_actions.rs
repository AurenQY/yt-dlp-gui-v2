//! Timeline range editor and per-item cancellation presentation.
use super::*;
use crate::domain::GalleryDownloadRange;

#[derive(Default)]
pub struct GalleryRangePicker {
    pub item_id: Option<QueueItemId>,
    pub section: Option<GalleryRangeSection>,
    pub custom: String,
    pub error: Option<String>,
    pub selected: Option<GalleryDownloadRange>,
    pub custom_selected: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GalleryRangeSection {
    Time,
    Posts,
}

impl AppState {
    pub fn gallery_post_range_available(&self) -> bool {
        self.gallery_range_picker
            .item_id
            .and_then(|id| self.queue_item_by_id(id))
            .is_some_and(|item| match &item.backend {
                QueueBackend::GalleryDl(gallery) => gallery
                    .analysis
                    .as_ref()
                    .is_some_and(|analysis| analysis.supports_single_feed_post_range()),
                _ => false,
            })
    }

    pub fn open_gallery_range_picker(&mut self, item_id: QueueItemId) {
        let Some(index) = self.queue_item_index_by_id(item_id) else {
            return;
        };
        if self.item_is_busy(index) {
            return;
        }
        let range = match &self.queue_items[index].backend {
            QueueBackend::GalleryDl(gallery) => gallery.download_range.clone(),
            _ => return,
        };
        let section = match &range {
            GalleryDownloadRange::Posts { .. } => Some(GalleryRangeSection::Posts),
            GalleryDownloadRange::Recent { .. } | GalleryDownloadRange::Since { .. } => {
                Some(GalleryRangeSection::Time)
            }
            GalleryDownloadRange::All => None,
        };
        let (custom_selected, custom) = match &range {
            GalleryDownloadRange::Since { date } => (true, date.clone()),
            GalleryDownloadRange::Posts { count } if ![10, 50, 100].contains(count) => {
                (true, count.to_string())
            }
            _ => (false, String::new()),
        };
        self.gallery_range_picker = GalleryRangePicker {
            item_id: Some(item_id),
            section,
            selected: Some(range),
            custom_selected,
            custom,
            ..Default::default()
        };
    }

    pub fn apply_gallery_range(&mut self, range: GalleryDownloadRange) {
        let Some(id) = self.gallery_range_picker.item_id else {
            return;
        };
        let Some(index) = self.queue_item_index_by_id(id) else {
            self.gallery_range_picker = Default::default();
            return;
        };
        if self.item_is_busy(index) {
            return;
        }
        let mut validated = range.clone();
        validated.freeze(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
        if validated.post_filter().is_err() {
            self.gallery_range_picker.error =
                Some(crate::i18n::text(self.language(), "gallery.range.invalid").to_owned());
            return;
        }
        if let QueueBackend::GalleryDl(gallery) = &mut self.queue_items[index].backend {
            gallery.download_range = range;
        }
        self.gallery_range_picker = Default::default();
        self.save_active_audio_playlist_if_needed();
    }
}
