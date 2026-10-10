//! Projects canonical gallery metadata into stable v2 UI fields.
//!
//! Raw extractor metadata remains authoritative for download, archive, rename,
//! and embedding. Keyword-to-property decisions belong exclusively to
//! `gallery_canonical_metadata`; this module owns presentation-only values such
//! as site icons and first-media thumbnail selection.

use serde_json::Value;

use crate::domain::GalleryAnalysis;

use super::gallery_canonical_metadata::map_gallery_canonical_metadata_from_roots;
use super::site_icon::SiteIcon;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct GalleryUiMetadata {
    pub site_icon: Option<SiteIcon>,
    pub title: String,
    pub author: String,
    pub author_handle: String,
    pub date: String,
    pub description: String,
    pub thumbnail: String,
    /// Author avatar used only when the first media image cannot be loaded.
    pub thumbnail_fallback: String,
    pub media_count: usize,
    pub source: GalleryUiSource,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct GalleryUiSource {
    pub site: String,
    pub category: String,
    pub subcategory: String,
    pub url: String,
}

struct GalleryMetadataRule {
    site_icon: Option<SiteIcon>,
    avatar: &'static str,
}

const TWITTER_RULE: GalleryMetadataRule = GalleryMetadataRule {
    site_icon: Some(SiteIcon::X),
    avatar: "{author.profile_image}",
};

const INSTAGRAM_RULE: GalleryMetadataRule = GalleryMetadataRule {
    site_icon: Some(SiteIcon::Instagram),
    avatar: "{user.profile_pic_url}",
};

const PIXIV_ARTWORK_RULE: GalleryMetadataRule = GalleryMetadataRule {
    site_icon: Some(SiteIcon::Pixiv),
    avatar: "{user.profile_image_urls.medium}",
};

const GENERIC_RULE: GalleryMetadataRule = GalleryMetadataRule {
    site_icon: None,
    avatar: "{thumbnail}",
};

pub(crate) fn map_gallery_ui_metadata(analysis: &GalleryAnalysis) -> GalleryUiMetadata {
    let post = analysis.posts.first();
    let media = analysis.media.first();
    let roots = [
        post.map(|post| &post.source_metadata),
        media.map(|media| &media.source_metadata),
    ];
    let canonical_roots = roots.iter().flatten().copied().collect::<Vec<_>>();
    let canonical = if canonical_roots.iter().any(|root| !root.is_null()) {
        map_gallery_canonical_metadata_from_roots(&canonical_roots)
    } else {
        analysis.persisted_metadata.clone().unwrap_or_default()
    };
    let category = non_empty_owned(canonical.category.clone())
        .or_else(|| {
            post.map(|post| post.category.trim().to_owned())
                .filter(not_empty)
        })
        .unwrap_or_default();
    let subcategory = non_empty_owned(canonical.subcategory.clone())
        .or_else(|| {
            post.map(|post| post.subcategory.trim().to_owned())
                .filter(not_empty)
        })
        .unwrap_or_default();
    let rule = mapping_rule(&category);
    let site = source_site_name(&category, &analysis.source_url);
    let title = canonical.title.clone().unwrap_or_else(|| {
        gallery_display_title(
            &site,
            canonical.creator.as_deref(),
            canonical.creator_handle.as_deref(),
        )
    });
    let thumbnail = analysis
        .first_post_media()
        .find(|media| gallery_media_is_image(media))
        .map(|media| media.url.trim().to_owned())
        .filter(not_empty)
        .unwrap_or_default();
    let thumbnail_fallback = render_metadata_format(&roots, rule.avatar).unwrap_or_default();

    GalleryUiMetadata {
        site_icon: rule.site_icon,
        title,
        author: canonical.creator.unwrap_or_default(),
        author_handle: canonical.creator_handle.unwrap_or_default(),
        date: canonical
            .created_at
            .as_deref()
            .map(format_gallery_display_date)
            .unwrap_or_default(),
        description: canonical.description.unwrap_or_default(),
        thumbnail,
        thumbnail_fallback,
        media_count: analysis.first_post_media_count(),
        source: GalleryUiSource {
            site,
            category,
            subcategory,
            url: analysis.source_url.clone(),
        },
    }
}

fn gallery_display_title(site: &str, author: Option<&str>, handle: Option<&str>) -> String {
    let site = if site.trim().is_empty() {
        "gallery-dl"
    } else {
        site.trim()
    };
    if let Some(handle) = handle.filter(|value| !value.trim().is_empty()) {
        format!("{site} · @{}", handle.trim().trim_start_matches('@'))
    } else if let Some(author) = author.filter(|value| !value.trim().is_empty()) {
        format!("{site} · {}", author.trim())
    } else {
        site.to_owned()
    }
}

fn gallery_media_is_image(media: &crate::domain::GalleryMediaItem) -> bool {
    media.media_type.eq_ignore_ascii_case("image")
        || matches!(
            media.extension.trim().to_ascii_lowercase().as_str(),
            "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "avif"
        )
}

/// Keep the source timezone unchanged; an absent offset must not be guessed.
fn format_gallery_display_date(value: &str) -> String {
    let Some((date, time)) = value.split_once('T') else {
        return value.to_owned();
    };
    let Some(hour_minute) = time.get(0..5) else {
        return value.to_owned();
    };
    let suffix = time.get(8..).unwrap_or_default();
    let timezone = if suffix.ends_with('Z') {
        " UTC".to_owned()
    } else if let Some(index) = suffix.find(['+', '-']) {
        format!(" {}", &suffix[index..])
    } else {
        String::new()
    };
    format!("{date} {hour_minute}{timezone}")
}

fn mapping_rule(category: &str) -> &'static GalleryMetadataRule {
    match category.to_ascii_lowercase().as_str() {
        "twitter" | "x" | "nitter" => &TWITTER_RULE,
        "instagram" => &INSTAGRAM_RULE,
        "pixiv" => &PIXIV_ARTWORK_RULE,
        _ => &GENERIC_RULE,
    }
}

/// Renders the deliberately small v2 mapping syntax used by the constants in
/// this file. Every placeholder must resolve, otherwise the mapped UI field is
/// absent. Extractor rules remain explicit and never expand into guessed field
/// candidates.
fn render_metadata_format(roots: &[Option<&Value>], format: &str) -> Option<String> {
    let mut rendered = String::with_capacity(format.len());
    let mut remaining = format;
    while let Some(open) = remaining.find('{') {
        rendered.push_str(&remaining[..open]);
        let after_open = &remaining[open + 1..];
        let close = after_open.find('}')?;
        let path = after_open[..close].trim();
        if path.is_empty() {
            return None;
        }
        rendered.push_str(&first_text(roots, &[path])?);
        remaining = &after_open[close + 1..];
    }
    if remaining.contains('}') {
        return None;
    }
    rendered.push_str(remaining);
    let rendered = rendered.trim();
    (!rendered.is_empty()).then(|| rendered.to_owned())
}

fn first_text(roots: &[Option<&Value>], paths: &[&str]) -> Option<String> {
    let roots = roots.iter().flatten().copied().collect::<Vec<_>>();
    super::gallery_canonical_metadata::first_text(&roots, paths)
}

fn source_site_name(category: &str, source_url: &str) -> String {
    match category.to_ascii_lowercase().as_str() {
        "twitter" | "x" | "nitter" => "X".to_owned(),
        "instagram" => "Instagram".to_owned(),
        "pixiv" => "Pixiv".to_owned(),
        "generic" | "" => source_host(source_url).unwrap_or_else(|| category.to_owned()),
        _ => category.to_owned(),
    }
}

fn source_host(url: &str) -> Option<String> {
    let rest = url
        .trim()
        .strip_prefix("https://")
        .or_else(|| url.trim().strip_prefix("http://"))?;
    let host = rest
        .split(['/', '?', '#'])
        .next()?
        .rsplit('@')
        .next()?
        .split(':')
        .next()?
        .trim()
        .trim_end_matches('.');
    (!host.is_empty()).then(|| host.to_owned())
}

fn not_empty(value: &String) -> bool {
    !value.trim().is_empty()
}

fn non_empty_owned(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}
