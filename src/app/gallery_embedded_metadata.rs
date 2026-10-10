//! Maps gallery-dl's extractor-specific metadata into ordinary EXIF fields.
//!
//! The input schema is intentionally open-ended. Known extractors receive
//! precise path priority while generic fallbacks cover common gallery-dl keys.

use serde_json::Value;

use crate::infrastructure::gallery_exif::GalleryExifFields;

struct GalleryExifRule {
    title: &'static [&'static str],
    author: &'static [&'static str],
    description: &'static [&'static str],
    keywords: &'static [&'static str],
    date: &'static [&'static str],
    copyright: &'static [&'static str],
}

const TWITTER_RULE: GalleryExifRule = GalleryExifRule {
    title: &["title", "post_title"],
    author: &["author.nick", "author.name", "username"],
    description: &["content", "text", "description"],
    keywords: &["hashtags", "tags", "keywords"],
    date: &["date", "created_at", "timestamp"],
    copyright: &["copyright"],
};

const INSTAGRAM_RULE: GalleryExifRule = GalleryExifRule {
    title: &["title", "post_title"],
    author: &["fullname", "owner.full_name", "username", "owner.username"],
    description: &["description", "caption", "content", "text"],
    keywords: &["tags", "hashtags", "keywords"],
    date: &["post_date", "date", "taken_at", "created_at"],
    copyright: &["copyright"],
};

const PIXIV_RULE: GalleryExifRule = GalleryExifRule {
    title: &["title", "post_title"],
    author: &["user.name", "user.account", "author", "username"],
    description: &["caption", "description", "content"],
    keywords: &["tags", "keywords"],
    date: &["date", "create_date", "created_at"],
    copyright: &["copyright"],
};

const GENERIC_RULE: GalleryExifRule = GalleryExifRule {
    title: &["title", "post_title", "gallery_title", "name"],
    author: &[
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
    description: &["description", "content", "caption", "text", "comment"],
    keywords: &["tags", "hashtags", "keywords"],
    date: &[
        "post_date",
        "date",
        "created_at",
        "create_date",
        "taken_at",
        "timestamp",
    ],
    copyright: &["copyright", "rights"],
};

const COMMENT_FIELDS: &[(&str, &str)] = &[
    ("post_url", "Post URL"),
    ("source_url", "Source URL"),
    ("page_url", "Page URL"),
    ("webpage_url", "Webpage URL"),
    ("url", "URL"),
    ("post_id", "Post ID"),
    ("tweet_id", "Tweet ID"),
    ("media_id", "Media ID"),
    ("image_id", "Image ID"),
    ("gallery_id", "Gallery ID"),
    ("post_shortcode", "Post Shortcode"),
    ("shortcode", "Shortcode"),
    ("num", "Media Index"),
    ("category", "Site"),
    ("subcategory", "Type"),
    ("location_name", "Location"),
    ("location_id", "Location ID"),
    ("location_url", "Location URL"),
    ("audio_title", "Audio Title"),
    ("audio_artist", "Audio Artist"),
    ("audio_duration", "Audio Duration"),
];

// Fields already mapped to dedicated EXIF properties, plus transient/file-level
// values that add noise rather than durable human-readable metadata. Unknown
// top-level scalar fields that are not in this list fall back to XPComment.
const COMMENT_SKIP_FIELDS: &[&str] = &[
    "title",
    "post_title",
    "gallery_title",
    "name",
    "fullname",
    "author",
    "username",
    "description",
    "content",
    "caption",
    "text",
    "comment",
    "tags",
    "hashtags",
    "keywords",
    "post_date",
    "date",
    "created_at",
    "create_date",
    "taken_at",
    "timestamp",
    "copyright",
    "rights",
    "filename",
    "extension",
    "width",
    "height",
    "width_original",
    "height_original",
    "display_url",
    "video_url",
    "image_url",
    "thumbnail",
    "thumbnail_url",
    "profile_pic_url",
];

pub(crate) fn map_gallery_embedded_metadata(metadata: &Value) -> GalleryExifFields {
    let category = metadata_path_text(metadata, "category").unwrap_or_default();
    let rule = mapping_rule(&category);

    let title = gallery_title(metadata, &category, rule);
    let author = first_text_with_fallback(metadata, rule.author, GENERIC_RULE.author);
    let description =
        first_text_with_fallback(metadata, rule.description, GENERIC_RULE.description);
    let date_time_original = first_text_with_fallback(metadata, rule.date, GENERIC_RULE.date)
        .and_then(|value| normalize_exif_datetime(&value));
    let copyright =
        first_text_with_fallback(metadata, rule.copyright, GENERIC_RULE.copyright);
    let keywords = first_keywords_with_fallback(metadata, rule.keywords, GENERIC_RULE.keywords);
    let subject = gallery_subject(metadata);
    let comment = gallery_comment(metadata);

    GalleryExifFields {
        title,
        author,
        description,
        keywords,
        subject,
        date_time_original,
        copyright,
        comment,
    }
}

fn gallery_title(
    metadata: &Value,
    category: &str,
    rule: &GalleryExifRule,
) -> Option<String> {
    if let Some(title) = first_text_with_fallback(metadata, rule.title, GENERIC_RULE.title) {
        return Some(title);
    }

    match category.to_ascii_lowercase().as_str() {
        "instagram" => joined_identity(metadata, "fullname", "username"),
        "twitter" | "x" | "nitter" => joined_identity(metadata, "author.nick", "author.name"),
        "pixiv" => joined_identity(metadata, "user.name", "user.account"),
        _ => None,
    }
}

fn joined_identity(metadata: &Value, name_path: &str, handle_path: &str) -> Option<String> {
    let name = metadata_path_text(metadata, name_path);
    let handle = metadata_path_text(metadata, handle_path)
        .map(|value| value.trim_start_matches('@').to_owned())
        .filter(|value| !value.is_empty());
    match (name, handle) {
        (Some(name), Some(handle)) => Some(format!("{name} (@{handle})")),
        (Some(name), None) => Some(name),
        (None, Some(handle)) => Some(format!("@{handle}")),
        (None, None) => None,
    }
}

fn mapping_rule(category: &str) -> &'static GalleryExifRule {
    match category.to_ascii_lowercase().as_str() {
        "twitter" | "x" | "nitter" => &TWITTER_RULE,
        "instagram" => &INSTAGRAM_RULE,
        "pixiv" => &PIXIV_RULE,
        _ => &GENERIC_RULE,
    }
}

fn first_text_with_fallback(
    root: &Value,
    preferred: &[&str],
    fallback: &[&str],
) -> Option<String> {
    first_text(root, preferred).or_else(|| first_text(root, fallback))
}

fn first_text(root: &Value, paths: &[&str]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| metadata_path_text(root, path))
}

fn first_keywords_with_fallback(
    root: &Value,
    preferred: &[&str],
    fallback: &[&str],
) -> Vec<String> {
    preferred
        .iter()
        .find_map(|path| metadata_path_keywords(root, path))
        .or_else(|| {
            fallback
                .iter()
                .find_map(|path| metadata_path_keywords(root, path))
        })
        .unwrap_or_default()
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

fn gallery_subject(metadata: &Value) -> Option<String> {
    let category = metadata_path_text(metadata, "category");
    let subcategory = metadata_path_text(metadata, "subcategory");
    match (category, subcategory) {
        (Some(category), Some(subcategory)) if !subcategory.eq_ignore_ascii_case(&category) => {
            Some(format!("{category} · {subcategory}"))
        }
        (Some(category), _) => Some(category),
        (_, Some(subcategory)) => Some(subcategory),
        _ => None,
    }
}

fn gallery_comment(metadata: &Value) -> Option<String> {
    let mut lines = Vec::new();
    for (path, label) in COMMENT_FIELDS {
        let Some(value) = metadata_path_text(metadata, path) else {
            continue;
        };
        lines.push(format!("{label}: {value}"));
    }

    if let Some(object) = metadata.as_object() {
        for (key, value) in object {
            if comment_field_is_explicit(key) || COMMENT_SKIP_FIELDS.contains(&key.as_str()) {
                continue;
            }
            let Some(value) = readable_comment_value(value) else {
                continue;
            };
            lines.push(format!("{}: {value}", humanize_metadata_key(key)));
        }
    }

    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn comment_field_is_explicit(key: &str) -> bool {
    COMMENT_FIELDS.iter().any(|(path, _)| *path == key)
}

fn readable_comment_value(value: &Value) -> Option<String> {
    const MAX_VALUE_CHARS: usize = 2048;

    let text = match value {
        Value::String(value) => value.trim().to_owned(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Array(values) => {
            let mut parts = Vec::new();
            for value in values {
                let part = match value {
                    Value::String(value) => value.trim().to_owned(),
                    Value::Number(value) => value.to_string(),
                    Value::Bool(value) => value.to_string(),
                    _ => return None,
                };
                if !part.is_empty() {
                    parts.push(part);
                }
            }
            parts.join(", ")
        }
        Value::Null | Value::Object(_) => return None,
    };
    if text.is_empty() {
        return None;
    }
    Some(truncate_chars(&text, MAX_VALUE_CHARS))
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    truncated.push('…');
    truncated
}

fn humanize_metadata_key(key: &str) -> String {
    key.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = first.to_uppercase().collect::<String>();
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_exif_datetime(value: &str) -> Option<String> {
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

    let date = format!("{year}:{month}:{day}");
    let Some(time) = value.get(11..19) else {
        return Some(format!("{date} 00:00:00"));
    };
    let time_bytes = time.as_bytes();
    if time_bytes.get(2) == Some(&b':')
        && time_bytes.get(5) == Some(&b':')
        && time_bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 2 | 5) || byte.is_ascii_digit())
    {
        Some(format!("{date} {time}"))
    } else {
        Some(format!("{date} 00:00:00"))
    }
}

