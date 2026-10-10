use std::path::PathBuf;

use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GalleryAnalysis {
    pub source_url: String,
    /// Root URL extractor, not the child post extractor emitted by the bounded probe.
    #[serde(default)]
    pub root_extractor: String,
    /// Safe canonical projection retained by queue snapshots; never raw extractor JSON.
    #[serde(default)]
    pub persisted_metadata: Option<super::GalleryCanonicalMetadata>,
    /// Display-only first-post count restored from a queue summary, never a timeline total.
    #[serde(skip)]
    pub preview_media_count: Option<usize>,
    pub posts: Vec<GalleryPost>,
    pub media: Vec<GalleryMediaItem>,
    #[serde(skip)]
    pub queued_urls: Vec<GalleryQueuedUrl>,
}

impl GalleryAnalysis {
    /// Native post-range counters restart for each delegated extractor. A probe
    /// that expands several child URLs cannot promise one global post limit.
    pub fn supports_single_feed_post_range(&self) -> bool {
        self.queued_urls.len() <= 1
    }

    /// The bounded probe may contain a second post. It is not part of the card preview.
    pub fn first_post_media(&self) -> impl Iterator<Item = &GalleryMediaItem> {
        let first = self.media.first();
        self.media.iter().filter(move |media| {
            first.is_some_and(|first| {
                media.post_index == first.post_index
                    && (first.post_id.is_empty() || media.post_id == first.post_id)
            })
        })
    }

    pub fn first_post_media_count(&self) -> usize {
        self.preview_media_count
            .unwrap_or_else(|| self.first_post_media().count())
    }

    pub fn primary_title(&self) -> Option<&str> {
        self.posts
            .iter()
            .find_map(|post| (!post.title.is_empty()).then_some(post.title.as_str()))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GalleryPost {
    pub id: String,
    pub title: String,
    pub author: String,
    pub author_nick: String,
    pub author_name: String,
    pub author_id: String,
    pub category: String,
    pub subcategory: String,
    pub date: String,
    pub content: String,
    #[serde(skip)]
    pub thumbnail_url: String,
    /// Opaque extractor metadata retained for future rename and embedding policies.
    #[serde(skip)]
    pub source_metadata: Value,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GalleryMediaItem {
    pub post_index: Option<usize>,
    #[serde(skip)]
    pub url: String,
    pub source_page_url: String,
    pub post_id: String,
    pub media_id: String,
    pub sequence: Option<u64>,
    pub media_type: String,
    pub filename: String,
    pub extension: String,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub archive_key: String,
    pub output_path: Option<PathBuf>,
    /// Browser probe only: gallery-dl's formatted path relative to its destination.
    #[serde(skip)]
    pub browser_relative_path: Option<String>,
    /// Extractor-owned alternate media URLs, only for the live browser album.
    /// Never persist signed CDN URLs in desktop queue snapshots.
    #[serde(skip)]
    pub browser_fallback_urls: Vec<String>,
    /// Opaque extractor metadata retained without making UI depend on its schema.
    #[serde(skip)]
    pub source_metadata: Value,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GalleryQueuedUrl {
    pub url: String,
    pub category: String,
    pub subcategory: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryDownloadArtifact {
    /// Canonical creation date, absent when extractor metadata cannot identify it.
    pub date: Option<String>,
    pub path: PathBuf,
    pub archive_key: String,
    pub media_url: String,
    pub category: String,
    pub subcategory: String,
    pub sequence: Option<u64>,
    pub media_type: String,
    pub filename: String,
    pub extension: String,
}
