//! Project extractor download instructions to browser-fetchable media URLs.
//! Native Gallery downloads keep their original downloader protocol and quality.

use crate::domain::GalleryMediaItem;

pub(super) fn browser_item(
    media: &GalleryMediaItem,
    post: Option<&crate::domain::GalleryPost>,
    analysis_url: &str,
    post_index: usize,
) -> Result<serde_json::Value, String> {
    let url = browser_gallery_media_url(media)?;
    let mut fallback_urls = Vec::new();
    for fallback in &media.browser_fallback_urls {
        if fallback != url
            && is_browser_media_url(fallback)
            && !fallback_urls.contains(&fallback.as_str())
        {
            fallback_urls.push(fallback.as_str());
        }
    }
    let thumbnail = browser_gallery_thumbnail_url(media, url);
    let title = post
        .map(|post| post.title.as_str())
        .filter(|title| !title.is_empty())
        .unwrap_or(&media.filename);
    let creator = post
        .map(|post| post.author_nick.as_str())
        .filter(|name| !name.is_empty())
        .or_else(|| post.map(|post| post.author_name.as_str()))
        .unwrap_or("");
    Ok(serde_json::json!({
        "backend":"g", "url":url, "thumbnail":thumbnail,
        "fallbackUrls":fallback_urls,
        "sourceScope":"single-post", "analysisSourceUrl":analysis_url,
        "analysisPostIndex":post_index, "sourcePageUrl":media.source_page_url,
        "title":title, "creator":creator, "description":post.map(|post| post.content.as_str()).unwrap_or(""),
        "postId":media.post_id, "mediaType":media.media_type, "filename":media.filename,
        "extension":media.extension, "relativePath":media.browser_relative_path,
        "width":media.width, "height":media.height, "date":post.map(|post|post.date.as_str()).unwrap_or(""),
        "archiveKey":media.archive_key,
        "canonicalMetadata":crate::app::gallery_canonical_metadata::map_gallery_canonical_metadata_from_roots(
            &[&media.source_metadata, &post.map(|post|post.source_metadata.clone()).unwrap_or(serde_json::Value::Null)])
    }))
}

/// Initial analysis has the same completed-post boundary as album reads.
/// A live probe is not proof that a feed with one observed post has ended.
pub(super) fn browser_analysis_item(
    media: &GalleryMediaItem,
    post: Option<&crate::domain::GalleryPost>,
    analysis_url: &str,
    source_scope: &str,
    completed_post_count: usize,
) -> Result<serde_json::Value, String> {
    let mut item = browser_item(media, post, analysis_url, media.post_index.unwrap_or(0))?;
    let timeline = source_scope == "timeline";
    item["sourceScope"] = serde_json::json!(source_scope);
    item["analysisPostIndex"] = serde_json::json!(if timeline { media.post_index } else { None });
    item["analysisProbePostCount"] = serde_json::json!(if timeline {
        Some(completed_post_count.min(2))
    } else {
        None
    });
    item["analysisProbeComplete"] = serde_json::json!(false);
    Ok(item)
}

pub(crate) fn browser_gallery_media_url(media: &GalleryMediaItem) -> Result<&str, String> {
    if is_browser_media_url(&media.url) {
        return Ok(&media.url);
    }
    // A ytdl: URL is an extractor instruction, NOT an HTTP media URL with a
    // removable prefix. Prefer the actual direct media supplied by the extractor.
    if media.url.starts_with("ytdl:") {
        for field in ["video_url", "audio_url"] {
            if let Some(url) = media
                .source_metadata
                .get(field)
                .and_then(|value| value.as_str())
            {
                if is_browser_media_url(url) {
                    return Ok(url);
                }
            }
        }
    }
    Err("gallery-dl did not provide a direct HTTP(S) media URL for browser viewing. Use native download for this item.".into())
}

pub(crate) fn browser_gallery_thumbnail_url<'a>(
    media: &'a GalleryMediaItem,
    direct_url: &'a str,
) -> &'a str {
    for field in ["display_url", "thumbnail", "preview", "cover"] {
        if let Some(url) = media
            .source_metadata
            .get(field)
            .and_then(|value| value.as_str())
        {
            if is_browser_media_url(url) {
                return url;
            }
        }
    }
    direct_url
}

fn is_browser_media_url(value: &str) -> bool {
    reqwest::Url::parse(value).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
    })
}
