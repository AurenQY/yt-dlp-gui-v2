//! Thin byte ABI only. No filesystem, networking, decoder or second mapping.
#![allow(dead_code)]
#[path = "../../../src/domain/gallery_metadata.rs"]
mod gallery_metadata;
mod domain {
    pub use crate::gallery_metadata::*;
}
#[path = "../../../src/infrastructure/gallery_xmp_packet.rs"]
mod gallery_xmp_packet;
#[path = "../../../src/infrastructure/xmp/mod.rs"]
mod xmp;

#[derive(serde::Deserialize)]
struct Request {
    archive_key: String,
    fields: domain::GalleryCanonicalMetadata,
}

/// Host allocates and frees input blocks; xmp_write owns only its result block.
#[unsafe(no_mangle)]
pub extern "C" fn xmp_alloc(length: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; length].into_boxed_slice()) as *mut u8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmp_free(pointer: *mut u8, length: usize) {
    if !pointer.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                pointer, length,
            )))
        };
    }
}

/// Result: u32 LE payload length, u8 status (0=success/1=error), payload bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xmp_write(
    image_pointer: *const u8,
    image_length: usize,
    request_pointer: *const u8,
    request_length: usize,
) -> *mut u8 {
    let result = (|| {
        if image_pointer.is_null() || request_pointer.is_null() || request_length > 16 * 1024 * 1024
        {
            return Err("Invalid XMP input".to_owned());
        }
        let image = unsafe { std::slice::from_raw_parts(image_pointer, image_length) };
        let metadata = unsafe { std::slice::from_raw_parts(request_pointer, request_length) };
        let request: Request =
            serde_json::from_slice(metadata).map_err(|error| error.to_string())?;
        use xmp::ImageFormat;
        let format = if image.starts_with(&[0xff, 0xd8]) {
            ImageFormat::Jpeg
        } else if image.starts_with(b"\x89PNG\r\n\x1a\n") {
            ImageFormat::Png
        } else if image.starts_with(b"GIF87a") || image.starts_with(b"GIF89a") {
            ImageFormat::Gif
        } else {
            return Err("XMP embedding supports JPEG, PNG and GIF only".to_owned());
        };
        gallery_xmp_packet::encode_gallery_xmp(format, image, &request.archive_key, &request.fields)
            .map(|output| output.unwrap_or_else(|| image.to_vec()))
    })();
    let (status, payload) = match result {
        Ok(bytes) => (0u8, bytes),
        Err(message) => (1u8, message.into_bytes()),
    };
    let mut output = Vec::with_capacity(payload.len() + 5);
    output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    output.push(status);
    output.extend_from_slice(&payload);
    Box::into_raw(output.into_boxed_slice()) as *mut u8
}
