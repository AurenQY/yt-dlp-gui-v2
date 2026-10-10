//! Durable task intent and minimal display, not cached analysis or execution history.
use super::audio_queue_snapshot::AudioQueueItemSnapshot;
use super::queue_preview::{self, GalleryQueueSummary};
use serde::{Deserialize, Serialize};

use super::{
    DownloadSelection, GalleryQueueItem, GallerySourceScope, MetadataState, QueueBackend,
    QueueItem, QueueItemViewKind, WorkflowKind, WorkflowState, gallery_source_scope,
};

#[derive(PartialEq, Serialize, Deserialize)]
#[serde(try_from = "QueueSnapshotDocument")]
pub(crate) struct QueueSnapshot {
    pub downloads: Vec<DownloadQueueItemSnapshot>,
    pub audio: Vec<AudioQueueItemSnapshot>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueueSnapshotDocument {
    downloads: Vec<DownloadQueueItemSnapshot>,
    audio: Vec<AudioQueueItemSnapshot>,
}

impl TryFrom<QueueSnapshotDocument> for QueueSnapshot {
    type Error = &'static str;

    fn try_from(snapshot: QueueSnapshotDocument) -> Result<Self, Self::Error> {
        if snapshot.downloads.iter().any(|item| {
            item.source_url.trim().is_empty()
                || item.playlist_index == Some(0)
                || (item.backend == SavedBackend::YtDlp && item.gallery.is_some())
        }) {
            return Err("Invalid download queue item identity");
        }
        if snapshot
            .audio
            .iter()
            .any(|item| item.source_url.trim().is_empty())
        {
            return Err("Invalid audio queue item");
        }
        Ok(Self {
            downloads: snapshot.downloads,
            audio: snapshot.audio,
        })
    }
}

impl QueueSnapshot {
    pub(crate) fn thumbnail_ids(&self) -> impl Iterator<Item = &str> {
        self.downloads
            .iter()
            .map(|item| item.persistent_id.as_str())
            .chain(self.audio.iter().map(|item| item.persistent_id.as_str()))
    }

    pub(crate) fn capture(
        audio_active: bool,
        active: &[QueueItem],
        parked_downloads: &[QueueItem],
        parked_audio: &[QueueItem],
        pending_removals: &[super::QueueItemId],
        display: impl Fn(&QueueItem) -> queue_preview::DownloadQueueDisplay,
    ) -> Self {
        let downloads = normal_queue_items(audio_active, active, parked_downloads);
        let audio = if audio_active { active } else { parked_audio };
        Self {
            downloads: downloads
                .iter()
                .filter(|item| retain_download_queue_item(item))
                .filter(|item| !pending_removals.contains(&item.id))
                .map(|item| {
                    let mut snapshot = DownloadQueueItemSnapshot::from_item(item);
                    snapshot.display = display(item);
                    snapshot
                })
                .collect(),
            audio: audio
                .iter()
                .filter(|item| item.view_kind == QueueItemViewKind::MusicCompact)
                .filter(|item| !item.source_url.trim().is_empty())
                .filter(|item| !pending_removals.contains(&item.id))
                .map(AudioQueueItemSnapshot::from_item)
                .collect(),
        }
    }
}

pub(crate) fn normal_queue_items<'a>(
    audio_active: bool,
    active: &'a [QueueItem],
    parked: &'a [QueueItem],
) -> &'a [QueueItem] {
    if audio_active { parked } else { active }
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DownloadQueueItemSnapshot {
    #[serde(rename = "id")]
    persistent_id: super::PersistentItemId,
    source_url: String,
    backend: SavedBackend,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    playlist_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    media_id: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    thumbnail_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    duration_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    extractor: String,
    #[serde(default, skip_serializing_if = "display_is_empty")]
    display: queue_preview::DownloadQueueDisplay,
    #[serde(default, skip_serializing_if = "selection_is_default")]
    selection: DownloadSelection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gallery: Option<GalleryQueueSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gallery_range: Option<super::GalleryDownloadRange>,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SavedBackend {
    YtDlp,
    GalleryDl,
}
fn selection_is_default(selection: &DownloadSelection) -> bool {
    *selection == DownloadSelection::default()
}
fn display_is_empty(display: &queue_preview::DownloadQueueDisplay) -> bool {
    *display == Default::default()
}

pub(crate) fn retain_download_queue_item(item: &QueueItem) -> bool {
    if item.view_kind == QueueItemViewKind::MusicCompact || item.source_url.trim().is_empty() {
        return false;
    }
    if let QueueBackend::GalleryDl(gallery) = &item.backend {
        // Unknown scope stays durable until it can be classified: never drop a
        // potentially recurring source merely because its bounded sample is small.
        if gallery.analysis.as_ref().is_none_or(|analysis| {
            gallery_source_scope(&analysis.root_extractor) != GallerySourceScope::SinglePost
        }) {
            return true;
        }
    }
    !download_is_complete(item)
}

fn download_is_complete(item: &QueueItem) -> bool {
    if item.last_error.is_some()
        || item.workflows.iter().any(|run| {
            matches!(
                run.kind,
                WorkflowKind::DownloadMedia | WorkflowKind::PostProcess | WorkflowKind::ExportMedia
            ) && matches!(run.state, WorkflowState::Queued | WorkflowState::Running)
        })
    {
        return false;
    }
    if !item
        .workflows
        .iter()
        .rev()
        .find(|run| run.kind == WorkflowKind::DownloadMedia)
        .is_some_and(|run| run.state == WorkflowState::Finished)
    {
        return false;
    }
    match &item.backend {
        QueueBackend::GalleryDl(_) => true,
        QueueBackend::YtDlp => item.completed_selection.as_ref().is_some_and(|completed| {
            super::models::CompletedSelection::from_selection(&item.selection) == *completed
        }),
    }
}

impl DownloadQueueItemSnapshot {
    pub(crate) fn from_item(item: &QueueItem) -> Self {
        Self {
            source_url: item.source_url.clone(),
            persistent_id: item.persistent_id.clone(),
            playlist_index: item.playlist_index,
            media_id: item.source_media_id.clone(),
            title: item.title.clone(),
            thumbnail_url: durable_thumbnail_url(&item.thumbnail_url),
            duration_millis: queue_preview::duration_millis(item),
            extractor: item
                .metadata()
                .map(|m| m.extractor_key.clone())
                .unwrap_or_default(),
            selection: persistent_selection(item),
            display: Default::default(),
            backend: match item.backend {
                QueueBackend::YtDlp => SavedBackend::YtDlp,
                QueueBackend::GalleryDl(_) => SavedBackend::GalleryDl,
            },
            gallery: match &item.backend {
                QueueBackend::YtDlp => None,
                QueueBackend::GalleryDl(gallery) => {
                    gallery.analysis.as_ref().map(GalleryQueueSummary::capture)
                }
            },
            gallery_range: match &item.backend {
                QueueBackend::GalleryDl(gallery) => Some(gallery.download_range.clone()),
                QueueBackend::YtDlp => None,
            },
        }
    }

    pub(crate) fn restore(self, id: u64, _workflow_id: u64) -> Option<QueueItem> {
        if self.source_url.trim().is_empty() || self.playlist_index == Some(0) {
            return None;
        }
        let mut item =
            QueueItem::with_persistent_id(id, self.persistent_id, self.source_url, self.title);
        item.playlist_index = self.playlist_index;
        item.source_media_id = self.media_id;
        item.thumbnail_url = self.thumbnail_url;
        item.duration_text = queue_preview::duration_text(self.duration_millis);
        item.selection = self.selection;
        item.restored_chapters_unresolved = !item
            .selection
            .download_range
            .selected_chapter_indices()
            .is_empty();
        item.needs_analysis_refresh = true;
        item.saved_download_display = self.display;
        item.metadata_state = MetadataState::Ready(queue_preview::video_preview(
            &item,
            self.extractor,
            self.duration_millis,
        ));
        if self.backend == SavedBackend::GalleryDl {
            item.metadata_state = MetadataState::Idle;
            item.backend = QueueBackend::GalleryDl(GalleryQueueItem {
                download_range: self.gallery_range.unwrap_or_default(),
                processed_posts: 0,
                analysis: Some(self.gallery.unwrap_or_default().restore(&item.source_url)),
                download_artifacts: Vec::new(),
            });
        }
        // No active workflow, stream URL, headers, child process or raw failure log survives.
        Some(item)
    }
}

/// Keep public preview references, not expiring/signature-bearing CDN URLs or credentials.
pub(crate) fn persistent_selection(item: &QueueItem) -> DownloadSelection {
    let mut selection = item.selection.clone();
    let Some(metadata) = item.metadata() else {
        return selection;
    };
    for (start, end) in item
        .selection
        .download_range
        .grouped_selected_chapter_spans(metadata.chapters.len())
    {
        let last = &metadata.chapters[end];
        let end_millis = last
            .end_millis
            .or_else(|| metadata.chapters.get(end + 1).map(|c| c.start_millis))
            .or(metadata.duration_millis);
        if let Some(range) = end_millis.and_then(|end| {
            super::DownloadTimeRange::new(metadata.chapters[start].start_millis, end)
        }) {
            for index in start..=end {
                selection.download_range.set_chapter_selected(index, false);
            }
            selection.download_range.add_custom_time_range(range);
        }
    }
    // Keep unresolved choices explicit rather than silently dropping them.
    selection
}

pub(crate) fn durable_thumbnail_url(value: &str) -> String {
    let Ok(url) = reqwest::Url::parse(value) else {
        return String::new();
    };
    if !url.username().is_empty() || url.password().is_some() {
        return String::new();
    }
    let sensitive_query = url.query_pairs().any(|(key, _)| {
        let key = key.to_ascii_lowercase();
        key.starts_with("x-amz-")
            || key.starts_with("x-goog-")
            || matches!(
                key.as_str(),
                "oh" | "oe"
                    | "token"
                    | "access_token"
                    | "auth"
                    | "authorization"
                    | "api_key"
                    | "signature"
                    | "sig"
                    | "policy"
                    | "key-pair-id"
                    | "__gda__"
                    | "expires"
                    | "expire"
                    | "credential"
            )
    });
    if sensitive_query {
        String::new()
    } else {
        value.to_owned()
    }
}

/// Decode ranges through their validated constructors, not private-field serde.
pub(crate) mod range_serde {
    use crate::domain::{DownloadRangeSelection, DownloadTimeRange};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Default, Serialize, Deserialize)]
    #[serde(default, deny_unknown_fields)]
    struct RangeSnapshot {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        chapters: Vec<usize>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        times: Vec<(u64, u64)>,
    }

    pub fn serialize<S: Serializer>(
        range: &DownloadRangeSelection,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        RangeSnapshot {
            chapters: range.selected_chapter_indices().to_vec(),
            times: range
                .custom_time_ranges()
                .iter()
                .map(|range| (range.start_millis(), range.end_millis()))
                .collect(),
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<DownloadRangeSelection, D::Error> {
        let snapshot = RangeSnapshot::deserialize(deserializer)?;
        let mut range = DownloadRangeSelection::default();
        for chapter in snapshot.chapters {
            range.set_chapter_selected(chapter, true);
        }
        for (start, end) in snapshot.times {
            let time = DownloadTimeRange::new(start, end)
                .ok_or_else(|| serde::de::Error::custom("download range end must exceed start"))?;
            range.add_custom_time_range(time);
        }
        Ok(range)
    }
}
