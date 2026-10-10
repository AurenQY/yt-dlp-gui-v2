//! Durable audio playlist fields; playback/stream handles never enter a snapshot.
use super::QueueItem;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AudioQueueItemSnapshot {
    #[serde(rename = "id")]
    pub persistent_id: super::PersistentItemId,
    pub source_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub album_title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub thumbnail_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub use_cookies: bool,
}
fn is_false(value: &bool) -> bool {
    !*value
}

impl AudioQueueItemSnapshot {
    pub(crate) fn from_item(item: &QueueItem) -> Self {
        Self {
            persistent_id: item.persistent_id.clone(),
            source_url: item.source_url.clone(),
            title: item.title.clone(),
            album_title: item.music_album_title.clone(),
            thumbnail_url: super::queue_snapshot::durable_thumbnail_url(&item.thumbnail_url),
            duration_millis: super::queue_preview::duration_millis(item),
            use_cookies: item.selection.use_cookies,
        }
    }
}
