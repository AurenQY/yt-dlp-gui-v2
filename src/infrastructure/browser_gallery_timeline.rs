//! Validated timeline intent and bounded album windows. No UI/sample inference.
use crate::domain::{GalleryDownloadRange, GallerySourceScope, gallery_source_scope};
use serde_json::Value;

pub(super) fn range(request: &Value, url: &str) -> Result<GalleryDownloadRange, String> {
    let root = super::extractor_index::matching_gallery_extractor(url)?.unwrap_or_default();
    if gallery_source_scope(&root) != GallerySourceScope::Timeline {
        return Err("This URL is not a recognized timeline.".into());
    }
    let mut range: GalleryDownloadRange = serde_json::from_value(request["timelineRange"].clone())
        .map_err(|_| "Invalid timeline range.".to_owned())?;
    range.freeze(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "System clock is invalid.".to_owned())?
            .as_secs(),
    );
    range.post_filter()?;
    Ok(range)
}

pub(super) struct TimelineWindow {
    pub start: u32,
    pub end: u32,
    pub page_size: u32,
}

pub(super) fn window(
    request: &Value,
    range: &GalleryDownloadRange,
) -> Result<TimelineWindow, String> {
    let page_size = match request.get("postCount") {
        None => 10,
        Some(value) => value
            .as_u64()
            .filter(|n| (1..=10).contains(n))
            .ok_or("Invalid timeline page size.")? as u32,
    };
    let start = request["postStart"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= u32::MAX as u64 - 11)
        .ok_or_else(|| "Invalid timeline continuation.".to_owned())? as u32;
    // Complete posts plus one lookahead; downloads keep their range cap while
    // viewport-driven album browsing requests All with a one-post page.
    let end = match range {
        GalleryDownloadRange::Posts { count } => (start + page_size).min(*count),
        _ => start + page_size,
    };
    if end < start {
        return Err("Timeline range has already ended.".into());
    }
    super::operation_diagnostics::record(
        "timeline.window",
        serde_json::json!({"postStart": start, "postEnd": end, "pageSize": page_size}),
    );
    Ok(TimelineWindow {
        start,
        end,
        page_size,
    })
}
