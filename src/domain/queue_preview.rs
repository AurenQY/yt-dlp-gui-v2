//! Minimal offline queue display; never an extractor result or an XMP input.
use super::{GalleryAnalysis, GalleryCanonicalMetadata, QueueItem, VideoMetadata};
use serde::{Deserialize, Serialize};

/// Offline card text only; never format availability or download parameters.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct DownloadQueueDisplay {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub video: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub audio: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub extension: String,
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct GalleryQueueSummary {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub root_extractor: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub site: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// First-post card preview only, independent from source scope/download totals.
    pub media_count: Option<usize>,
}

impl GalleryQueueSummary {
    pub fn capture(analysis: &GalleryAnalysis) -> Self {
        let canonical = analysis.persisted_metadata.as_ref();
        let post = analysis.posts.first();
        Self {
            root_extractor: analysis.root_extractor.clone(),
            site: canonical
                .map(|m| m.category.clone())
                .unwrap_or_else(|| post.map(|p| p.category.clone()).unwrap_or_default()),
            date: canonical
                .and_then(|m| m.created_at.clone())
                .unwrap_or_else(|| post.map(|p| p.date.clone()).unwrap_or_default()),
            media_count: Some(analysis.first_post_media_count()).filter(|count| *count > 0),
        }
    }

    pub fn restore(self, source: &str) -> GalleryAnalysis {
        GalleryAnalysis {
            source_url: source.to_owned(),
            root_extractor: self.root_extractor,
            persisted_metadata: Some(GalleryCanonicalMetadata {
                category: self.site,
                created_at: (!self.date.is_empty()).then_some(self.date),
                ..Default::default()
            }),
            preview_media_count: self.media_count,
            ..Default::default()
        }
    }
}

pub(crate) fn duration_millis(item: &QueueItem) -> Option<u64> {
    item.metadata()
        .and_then(|m| m.duration_millis)
        .or_else(|| {
            item.music_duration_seconds
                .filter(|v| v.is_finite() && *v >= 0.0)
                .map(|v| (v * 1000.0).round() as u64)
        })
        .or_else(|| parse_duration(&item.duration_text))
}

fn parse_duration(text: &str) -> Option<u64> {
    if text.trim().is_empty() {
        return None;
    }
    let mut seconds = 0u64;
    for part in text.trim().split(':') {
        seconds = seconds
            .checked_mul(60)?
            .checked_add(part.parse::<u64>().ok()?)?;
    }
    seconds.checked_mul(1000)
}

pub(crate) fn duration_text(millis: Option<u64>) -> String {
    let Some(millis) = millis else {
        return String::new();
    };
    let s = millis / 1000;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

pub(crate) fn video_preview(
    item: &QueueItem,
    extractor: String,
    millis: Option<u64>,
) -> VideoMetadata {
    VideoMetadata {
        extractor_key: extractor,
        title: item.title.clone(),
        duration_millis: millis,
        duration_text: duration_text(millis),
        thumbnail_url: item.thumbnail_url.clone(),
        ..VideoMetadata::empty_preview()
    }
}
