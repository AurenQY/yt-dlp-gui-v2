//! Sole canonical Gallery property mapping, shared by desktop and WASM.
use super::xmp::{self, DC, EXIF, IPTC, ImageFormat, Packet, RIGHTS, XMP};
use crate::domain::GalleryCanonicalMetadata;
const ARCHIVE_SCHEME: &str = "gallery-dl-archive";

/// None means the existing bytes already contain the requested metadata.
pub(crate) fn encode_gallery_xmp(
    format: ImageFormat,
    original: &[u8],
    archive_key: &str,
    fields: &GalleryCanonicalMetadata,
) -> Result<Option<Vec<u8>>, String> {
    let archive_key = non_empty(Some(archive_key))
        .ok_or("gallery-dl metadata did not contain an archive key.")?;
    let mut packet = xmp::read_packet(format, original)?.unwrap_or_else(Packet::new);
    if gallery_xmp_matches(&packet, archive_key, fields) {
        return Ok(None);
    }
    apply_gallery_xmp(&mut packet, archive_key, fields);
    let output = xmp::write_packet(format, original, Some(&packet))?;
    let readback = xmp::read_packet(format, &output)?.ok_or("Encoded image has no XMP")?;
    if !gallery_xmp_matches(&readback, archive_key, fields) {
        return Err(
            "Encoded image XMP did not preserve the requested properties; original left unchanged."
                .into(),
        );
    }
    Ok(Some(output))
}

fn simple_fields(
    fields: &GalleryCanonicalMetadata,
) -> [(&'static str, &'static str, Option<&str>); 3] {
    [
        (XMP, "CreateDate", fields.created_at.as_deref()),
        (DC, "source", fields.source_url.as_deref()),
        (IPTC, "Location", fields.location.as_deref()),
    ]
}
fn localized_fields(
    fields: &GalleryCanonicalMetadata,
) -> [(&'static str, &'static str, Option<&str>); 5] {
    [
        (DC, "title", fields.title.as_deref()),
        (DC, "description", fields.description.as_deref()),
        (DC, "rights", fields.rights.as_deref()),
        (RIGHTS, "UsageTerms", fields.usage_terms.as_deref()),
        (EXIF, "UserComment", fields.user_comment.as_deref()),
    ]
}
fn apply_gallery_xmp(packet: &mut Packet, archive_key: &str, fields: &GalleryCanonicalMetadata) {
    for (ns, key, value) in simple_fields(fields) {
        if let Some(value) = non_empty(value) {
            packet.set_simple(ns, key, value);
        }
    }
    for (ns, key, value) in localized_fields(fields) {
        if let Some(value) = non_empty(value) {
            packet.set_localized(ns, key, value);
        }
    }
    for (key, values, ordered) in [
        (
            "creator",
            fields.creator.iter().cloned().collect::<Vec<_>>(),
            true,
        ),
        ("subject", fields.keywords.clone(), false),
        ("type", fields.content_types.clone(), false),
        ("language", fields.languages.clone(), false),
    ] {
        let values: Vec<_> = values
            .iter()
            .filter_map(|value| non_empty(Some(value)))
            .collect();
        if !values.is_empty() {
            packet.set_array(DC, key, &values, ordered);
        }
    }
    packet.ensure_identifier(archive_key, ARCHIVE_SCHEME);
    if let Some(identifier) = &fields.resource_identifier {
        if let Some(value) = non_empty(Some(&identifier.value)) {
            packet.ensure_identifier(value, &identifier.scheme);
        }
    }
}
fn gallery_xmp_matches(
    packet: &Packet,
    archive_key: &str,
    fields: &GalleryCanonicalMetadata,
) -> bool {
    packet.has_identifier(archive_key, ARCHIVE_SCHEME)
        && simple_fields(fields).iter().all(|(ns, key, value)| {
            non_empty(*value).is_none_or(|value| packet.simple(ns, key).as_deref() == Some(value))
        })
        && localized_fields(fields).iter().all(|(ns, key, value)| {
            non_empty(*value)
                .is_none_or(|value| packet.localized(ns, key).as_deref() == Some(value))
        })
        && [
            (
                "creator",
                fields.creator.iter().cloned().collect::<Vec<_>>(),
            ),
            ("subject", fields.keywords.clone()),
            ("type", fields.content_types.clone()),
            ("language", fields.languages.clone()),
        ]
        .iter()
        .all(|(key, values)| {
            let expected: Vec<_> = values
                .iter()
                .filter_map(|value| non_empty(Some(value)))
                .collect();
            expected.is_empty()
                || packet
                    .array(DC, key)
                    .iter()
                    .map(String::as_str)
                    .eq(expected)
        })
        && fields
            .resource_identifier
            .as_ref()
            .is_none_or(|identifier| {
                non_empty(Some(&identifier.value))
                    .is_none_or(|value| packet.has_identifier(value, &identifier.scheme))
            })
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
