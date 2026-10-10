//! Native filesystem adapter; property mapping and byte encoding are shared with WASM.
use super::{gallery_xmp_packet::encode_gallery_xmp, xmp::ImageFormat};
use crate::domain::GalleryCanonicalMetadata;
use std::{fs, path::Path};
#[path = "gallery_xmp_file.rs"]
mod file;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GalleryXmpWriteOutcome {
    Written,
    AlreadyCurrent,
    Unsupported,
}

pub(crate) fn write_gallery_xmp(
    path: &Path,
    archive_key: &str,
    fields: &GalleryCanonicalMetadata,
) -> Result<GalleryXmpWriteOutcome, String> {
    let mut diagnostic =
        super::operation_diagnostics::OperationSpan::start("gallery.xmp", serde_json::json!({}));
    let Some(format) = image_format(path) else {
        diagnostic.finish("unsupported", serde_json::json!({}));
        return Ok(GalleryXmpWriteOutcome::Unsupported);
    };
    let original = fs::read(path)
        .map_err(|error| format!("Could not read image XMP {}: {error}", path.display()))?;
    let Some(output) = encode_gallery_xmp(format, &original, archive_key, fields)? else {
        diagnostic.finish("already-current", serde_json::json!({}));
        return Ok(GalleryXmpWriteOutcome::AlreadyCurrent);
    };
    file::replace_image(path, &original, &output)?;
    diagnostic.finish(
        "written",
        serde_json::json!({"originalBytes": original.len(), "outputBytes": output.len()}),
    );
    Ok(GalleryXmpWriteOutcome::Written)
}

fn image_format(path: &Path) -> Option<ImageFormat> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "png" => Some(ImageFormat::Png),
        "gif" => Some(ImageFormat::Gif),
        _ => None,
    }
}
