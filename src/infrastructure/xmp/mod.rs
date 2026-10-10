//! Dependency-free XMP bytes core. Filesystem persistence belongs to the caller.
mod gif;
mod jpeg;
mod jpeg_mpf;
mod md5;
mod packet;
mod packet_transfer;
mod png;
mod xml;
mod zlib;

pub(crate) use packet::{DC, EXIF, IPTC, Packet, RIGHTS, XMP};
pub(super) const MAX_PACKET_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ImageFormat {
    Jpeg,
    Png,
    Gif,
}

pub(crate) fn read_packet(format: ImageFormat, bytes: &[u8]) -> Result<Option<Packet>, String> {
    match format {
        ImageFormat::Jpeg => jpeg::read(bytes),
        ImageFormat::Png => png::read(bytes),
        ImageFormat::Gif => gif::read(bytes),
    }
}

/// None removes all XMP for this image; Some replaces only its XMP blocks.
pub(crate) fn write_packet(
    format: ImageFormat,
    bytes: &[u8],
    packet: Option<&Packet>,
) -> Result<Vec<u8>, String> {
    let serialized = packet.map(Packet::serialize).transpose()?;
    match format {
        ImageFormat::Jpeg => jpeg::write(bytes, serialized.as_deref()),
        ImageFormat::Png => png::write(bytes, serialized.as_deref()),
        ImageFormat::Gif => gif::write(bytes, serialized.as_deref()),
    }
}
