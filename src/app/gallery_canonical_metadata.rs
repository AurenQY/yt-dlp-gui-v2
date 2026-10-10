//! Resolves gallery-dl extractor keywords into one canonical metadata model.
//!
//! This is the only keyword-to-property mapping boundary for gallery content.
//! UI projection and embedded XMP both consume the same resolved values.

use serde_json::Value;

use crate::domain::{GalleryCanonicalIdentifier, GalleryCanonicalMetadata};

struct GalleryCanonicalRule {
    title: &'static [&'static str],
    creator: &'static [&'static str],
    creator_handle: &'static [&'static str],
    description: &'static [&'static str],
    keywords: &'static [&'static str],
    created_at: &'static [&'static str],
}

const TWITTER_RULE: GalleryCanonicalRule = GalleryCanonicalRule {
    title: &["title", "post_title"],
    creator: &["author.nick", "author.name", "username"],
    creator_handle: &["author.name", "username"],
    description: &["content", "text", "description"],
    keywords: &["hashtags", "tags", "keywords"],
    created_at: &["date", "created_at"],
};

const INSTAGRAM_RULE: GalleryCanonicalRule = GalleryCanonicalRule {
    title: &["title", "post_title"],
    creator: &["fullname", "owner.full_name", "username", "owner.username"],
    creator_handle: &["username", "owner.username"],
    description: &["description", "caption", "content", "text"],
    keywords: &["tags", "hashtags", "keywords"],
    created_at: &["post_date", "date", "created_at"],
};

const PIXIV_RULE: GalleryCanonicalRule = GalleryCanonicalRule {
    title: &["title", "post_title"],
    creator: &["user.name", "user.account", "author", "username"],
    creator_handle: &["user.account", "username"],
    description: &["caption", "description", "content"],
    keywords: &["tags", "keywords"],
    created_at: &["date", "create_date", "created_at"],
};

const GENERIC_RULE: GalleryCanonicalRule = GalleryCanonicalRule {
    title: &["title", "post_title"],
    creator: &[
        "fullname",
        "author.nick",
        "author.name",
        "author",
        "user.name",
        "owner.full_name",
        "username",
        "user.account",
        "owner.username",
    ],
    creator_handle: &["username", "user.account", "owner.username", "author.name"],
    description: &["description", "content", "caption", "text"],
    keywords: &["tags", "hashtags", "keywords", "categories"],
    created_at: &["post_date", "created_at", "create_date"],
};

const SOURCE_URL_PATHS: &[&str] = &["post_url", "page_url", "webpage_url"];
const RIGHTS_PATHS: &[&str] = &["copyright", "rights"];
const LANGUAGE_PATHS: &[&str] = &["language", "lang"];

pub(crate) fn map_gallery_canonical_metadata(metadata: &Value) -> GalleryCanonicalMetadata {
    map_gallery_canonical_metadata_from_roots(&[metadata])
}

/// Download events have an authoritative output path, including conversions.
/// Keep this override here so persistence and UI share the same type vocabulary.
pub(crate) fn map_gallery_artifact_metadata(
    metadata: &Value,
    path: &std::path::Path,
) -> GalleryCanonicalMetadata {
    let mut fields = map_gallery_canonical_metadata(metadata);
    fields.content_types = path
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(content_type_from_extension)
        .map(|value| vec![value.to_owned()])
        .unwrap_or_default();
    fields
}

pub(crate) fn map_gallery_canonical_metadata_from_roots(
    roots: &[&Value],
) -> GalleryCanonicalMetadata {
    let category = first_text(roots, &["category"]).unwrap_or_default();
    let subcategory = first_text(roots, &["subcategory"]).unwrap_or_default();
    let rule = mapping_rule(&category);
    let title = first_text_with_fallback(roots, rule.title, GENERIC_RULE.title);
    let creator = first_text_with_fallback(roots, rule.creator, GENERIC_RULE.creator);
    let creator_handle =
        first_text_with_fallback(roots, rule.creator_handle, GENERIC_RULE.creator_handle)
            .map(|value| value.trim_start_matches('@').to_owned())
            .filter(|value| !value.is_empty());
    let description = first_text_with_fallback(roots, rule.description, GENERIC_RULE.description);
    let created_at = first_text_with_fallback(roots, rule.created_at, GENERIC_RULE.created_at)
        .and_then(|value| normalize_xmp_datetime(&value));
    let keywords = first_keywords_with_fallback(roots, rule.keywords, GENERIC_RULE.keywords);
    let source_url = source_page_url(roots, &category);
    let rights = first_text(roots, RIGHTS_PATHS);
    let usage_terms = first_text(roots, &["license"]);
    let languages = normalized_languages(roots);
    let user_comment = first_text(roots, &["user_comment"]);
    // Only the reviewed Instagram paths identify the post's location;
    // a generic `location` can instead describe the uploader's profile.
    let location = if category.eq_ignore_ascii_case("instagram") {
        first_text(roots, &["location_name", "location.name"])
    } else {
        None
    };
    let resource_identifier = resource_identifier(roots, &category);
    let content_types = first_text(roots, &["extension", "file_ext"])
        .and_then(|extension| content_type_from_extension(&extension))
        .map(|value| vec![value.to_owned()])
        .unwrap_or_default();

    GalleryCanonicalMetadata {
        title,
        creator,
        creator_handle,
        description,
        keywords,
        created_at,
        source_url,
        resource_identifier,
        content_types,
        languages,
        rights,
        usage_terms,
        user_comment,
        location,
        category,
        subcategory,
    }
}

fn mapping_rule(category: &str) -> &'static GalleryCanonicalRule {
    match category.to_ascii_lowercase().as_str() {
        "twitter" | "x" | "nitter" => &TWITTER_RULE,
        "instagram" => &INSTAGRAM_RULE,
        "pixiv" => &PIXIV_RULE,
        _ => &GENERIC_RULE,
    }
}

fn content_type_from_extension(extension: &str) -> Option<&'static str> {
    match extension
        .trim_start_matches('.')
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" | "tif" | "tiff" | "bmp" | "heic" | "jxl" => Some("StillImage"),
        // These containers can hold either a still or an animation.
        "png" | "gif" | "webp" | "avif" | "heif" => Some("Image"),
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "m4v" => Some("MovingImage"),
        "mp3" | "m4a" | "flac" | "wav" | "opus" | "aac" => Some("Sound"),
        "txt" | "html" | "htm" => Some("Text"),
        _ => None,
    }
}

fn source_page_url(roots: &[&Value], category: &str) -> Option<String> {
    // A direct media URL or HTTP referrer does not establish a source page.
    for path in SOURCE_URL_PATHS {
        for root in roots {
            if let Some(value) = metadata_path_text(root, path) {
                if let Ok(url) = reqwest::Url::parse(&value) {
                    if matches!(url.scheme(), "http" | "https")
                        && url.host_str().is_some()
                        && url.username().is_empty()
                        && url.password().is_none()
                    {
                        return Some(value);
                    }
                }
            }
        }
    }
    let (paths, prefix): (&[&str], &str) = match category.to_ascii_lowercase().as_str() {
        "twitter" | "x" | "nitter" => (&["tweet_id"], "https://x.com/i/web/status/"),
        "pixiv"
            if matches!(
                first_text(roots, &["type"]).as_deref(),
                Some("illust" | "manga" | "ugoira")
            ) =>
        {
            (&["id"], "https://www.pixiv.net/artworks/")
        }
        _ => return None,
    };
    let id = first_text(roots, paths)?;
    id.bytes()
        .all(|byte| byte.is_ascii_digit())
        .then(|| format!("{prefix}{id}"))
}

fn normalized_languages(roots: &[&Value]) -> Vec<String> {
    let mut languages = Vec::new();
    for language in first_keywords(roots, LANGUAGE_PATHS).unwrap_or_default() {
        let language = language.replace('_', "-");
        let mut parts = language.split('-');
        let primary = parts.next().unwrap_or_default();
        if !(2..=3).contains(&primary.len())
            || !primary.bytes().all(|byte| byte.is_ascii_alphabetic())
        {
            continue;
        }
        let mut normalized = primary.to_ascii_lowercase();
        let mut valid = true;
        for part in parts {
            if !(1..=8).contains(&part.len())
                || !part.bytes().all(|byte| byte.is_ascii_alphanumeric())
            {
                valid = false;
                break;
            }
            normalized.push('-');
            normalized.push_str(&part.to_ascii_lowercase());
        }
        if valid {
            push_unique(&mut languages, normalized);
        }
    }
    languages
}

fn resource_identifier(roots: &[&Value], category: &str) -> Option<GalleryCanonicalIdentifier> {
    // Do not infer identity scope from an ID-shaped key alone. Instagram's
    // media_id is reviewed as the individual carousel item; other extractors
    // retain the exact archive identifier until their media IDs are reviewed.
    let paths: &[(&str, &str)] = match category.to_ascii_lowercase().as_str() {
        "instagram" => &[("media_id", "media")],
        _ => &[],
    };
    paths.iter().find_map(|(path, kind)| {
        first_text(roots, &[*path]).map(|value| GalleryCanonicalIdentifier {
            scheme: match category.trim() {
                "" => format!("gallery-dl-{kind}"),
                category => format!("gallery-dl-{}-{kind}", category.to_ascii_lowercase()),
            },
            value,
        })
    })
}

fn first_text_with_fallback(
    roots: &[&Value],
    preferred: &[&str],
    fallback: &[&str],
) -> Option<String> {
    first_text(roots, preferred).or_else(|| first_text(roots, fallback))
}

pub(crate) fn first_text(roots: &[&Value], paths: &[&str]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| roots.iter().find_map(|root| metadata_path_text(root, path)))
}

fn first_keywords_with_fallback(
    roots: &[&Value],
    preferred: &[&str],
    fallback: &[&str],
) -> Vec<String> {
    first_keywords(roots, preferred)
        .or_else(|| first_keywords(roots, fallback))
        .unwrap_or_default()
}

fn first_keywords(roots: &[&Value], paths: &[&str]) -> Option<Vec<String>> {
    paths.iter().find_map(|path| {
        roots
            .iter()
            .find_map(|root| metadata_path_keywords(root, path))
    })
}

fn metadata_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut value = root;
    for segment in path.split('.') {
        value = value.as_object()?.get(segment)?;
    }
    Some(value)
}

fn metadata_path_text(root: &Value, path: &str) -> Option<String> {
    value_text(metadata_path(root, path)?)
}

fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_owned())
        }
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn metadata_path_keywords(root: &Value, path: &str) -> Option<Vec<String>> {
    let value = metadata_path(root, path)?;
    let mut keywords = Vec::new();
    match value {
        Value::Array(values) => {
            for value in values {
                if let Some(keyword) = keyword_text(value) {
                    push_unique(&mut keywords, keyword);
                }
            }
        }
        _ => {
            if let Some(keyword) = keyword_text(value) {
                push_unique(&mut keywords, keyword);
            }
        }
    }
    (!keywords.is_empty()).then_some(keywords)
}

fn keyword_text(value: &Value) -> Option<String> {
    if let Some(text) = value_text(value) {
        return Some(text);
    }
    let object = value.as_object()?;
    ["name", "tag", "value", "translated_name"]
        .iter()
        .find_map(|key| object.get(*key).and_then(value_text))
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|known| known == &value) {
        values.push(value);
    }
}

fn normalize_xmp_datetime(value: &str) -> Option<String> {
    let value = value.trim();
    let year = value.get(0..4)?;
    let month = value.get(5..7)?;
    let day = value.get(8..10)?;
    if value.as_bytes().get(4) != Some(&b'-')
        || value.as_bytes().get(7) != Some(&b'-')
        || ![year, month, day]
            .iter()
            .all(|part| part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }
    if value.len() == 10 {
        return Some(value.to_owned());
    }
    let separator = value.as_bytes().get(10).copied()?;
    if separator != b'T' && separator != b' ' {
        return None;
    }
    let time = value.get(11..19)?;
    if time.as_bytes().get(2) != Some(&b':')
        || time.as_bytes().get(5) != Some(&b':')
        || !time
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 2 | 5) || byte.is_ascii_digit())
    {
        return None;
    }
    let suffix = value.get(19..).unwrap_or_default();
    Some(format!("{year}-{month}-{day}T{time}{suffix}"))
}
