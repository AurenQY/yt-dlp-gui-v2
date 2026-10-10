//! EXIF persistence for completed gallery-dl image artifacts.
//!
//! gallery-dl remains responsible for extraction/download identity. v2 writes
//! that identity and readable mapped metadata into the image after the file is
//! available, without sidecar metadata files.

use std::path::Path;

use little_exif::{exif_tag::ExifTag, ifd::ExifTagGroup, metadata::Metadata};

const XP_TITLE: u16 = 0x9C9B;
const XP_COMMENT: u16 = 0x9C9C;
const XP_AUTHOR: u16 = 0x9C9D;
const XP_KEYWORDS: u16 = 0x9C9E;
const XP_SUBJECT: u16 = 0x9C9F;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct GalleryExifFields {
    pub title: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub subject: Option<String>,
    pub date_time_original: Option<String>,
    pub copyright: Option<String>,
    pub comment: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GalleryExifWriteOutcome {
    Written,
    AlreadyCurrent,
    Unsupported,
}

pub(crate) fn write_gallery_exif(
    path: &Path,
    archive_key: &str,
    fields: &GalleryExifFields,
) -> Result<GalleryExifWriteOutcome, String> {
    if !supported_image_extension(path) {
        return Ok(GalleryExifWriteOutcome::Unsupported);
    }
    if archive_key.trim().is_empty() {
        return Err("gallery-dl metadata did not contain an archive key.".to_owned());
    }

    let mut metadata = match Metadata::new_from_path(path) {
        Ok(metadata) => metadata,
        Err(error) if is_missing_exif_error(&error.to_string()) => Metadata::new(),
        Err(error) => {
            return Err(format!(
                "Could not read image EXIF {}: {error}",
                path.display()
            ));
        }
    };
    if stored_archive_key(&metadata).as_deref() == Some(archive_key) {
        return Ok(GalleryExifWriteOutcome::AlreadyCurrent);
    }

    if let Some(description) = non_empty(fields.description.as_deref()) {
        metadata.set_tag(ExifTag::ImageDescription(description.to_owned()));
    }
    if let Some(author) = non_empty(fields.author.as_deref()) {
        metadata.set_tag(ExifTag::Artist(author.to_owned()));
        metadata.set_tag(windows_xp_tag(XP_AUTHOR, author));
    }
    if let Some(title) = non_empty(fields.title.as_deref()) {
        metadata.set_tag(windows_xp_tag(XP_TITLE, title));
    }
    if !fields.keywords.is_empty() {
        metadata.set_tag(windows_xp_tag(XP_KEYWORDS, &fields.keywords.join("; ")));
    }
    if let Some(subject) = non_empty(fields.subject.as_deref()) {
        metadata.set_tag(windows_xp_tag(XP_SUBJECT, subject));
    }
    if let Some(date) = non_empty(fields.date_time_original.as_deref()) {
        metadata.set_tag(ExifTag::DateTimeOriginal(date.to_owned()));
        metadata.set_tag(ExifTag::CreateDate(date.to_owned()));
    }
    if let Some(copyright) = non_empty(fields.copyright.as_deref()) {
        metadata.set_tag(ExifTag::Copyright(copyright.to_owned()));
    }

    let windows_comment = combined_comment(archive_key, fields);
    metadata.set_tag(windows_xp_tag(XP_COMMENT, &windows_comment));

    metadata
        .write_to_file(path)
        .map_err(|error| format!("Could not write image EXIF {}: {error}", path.display()))?;
    Ok(GalleryExifWriteOutcome::Written)
}


fn is_missing_exif_error(message: &str) -> bool {
    message
        .to_ascii_lowercase()
        .contains("no exif data found")
}

fn supported_image_extension(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "jxl" | "heif" | "heic" | "hif" | "avif" | "tif" | "tiff"
    )
}

fn windows_xp_tag(tag: u16, value: &str) -> ExifTag {
    ExifTag::UnknownINT8U(utf16le_null_terminated(value), tag, ExifTagGroup::GENERIC)
}

fn utf16le_null_terminated(value: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((value.encode_utf16().count() + 1) * 2);
    for unit in value.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes
}

fn combined_comment(archive_key: &str, fields: &GalleryExifFields) -> String {
    let mut sections = vec![format!("Archive ID: {archive_key}")];
    if let Some(description) = non_empty(fields.description.as_deref()) {
        sections.push(description.to_owned());
    }
    if let Some(comment) = non_empty(fields.comment.as_deref()) {
        sections.push(comment.to_owned());
    }
    sections.join("\n\n")
}

fn stored_archive_key(metadata: &Metadata) -> Option<String> {
    let tag = metadata
        .get_tag_by_hex(XP_COMMENT, Some(ExifTagGroup::GENERIC))
        .next()?;
    let ExifTag::UnknownINT8U(bytes, tag_id, ExifTagGroup::GENERIC) = tag else {
        return None;
    };
    if *tag_id != XP_COMMENT {
        return None;
    }
    let comment = decode_utf16le_null_terminated(bytes)?;
    comment.lines().find_map(|line| {
        line.strip_prefix("Archive ID:")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn decode_utf16le_null_terminated(bytes: &[u8]) -> Option<String> {
    if bytes.len() % 2 != 0 {
        return None;
    }
    let mut units = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let unit = u16::from_le_bytes([pair[0], pair[1]]);
        if unit == 0 {
            break;
        }
        units.push(unit);
    }
    String::from_utf16(&units).ok()
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

