//! GIF XMP application extension; original LZW frames and other extensions stay intact.
use super::{MAX_PACKET_BYTES, Packet};
use std::ops::Range;
const ID: &[u8] = b"XMP DataXMP";
struct Layout {
    xmp: Vec<Range<usize>>,
    trailer: usize,
}
fn subblocks(bytes: &[u8], mut pos: usize) -> Result<usize, String> {
    loop {
        let size = *bytes.get(pos).ok_or("Truncated GIF subblock")? as usize;
        pos += 1;
        if size == 0 {
            return Ok(pos);
        }
        pos = pos
            .checked_add(size)
            .filter(|p| *p <= bytes.len())
            .ok_or("Truncated GIF subblock data")?;
    }
}
fn trailer() -> Vec<u8> {
    let mut value = vec![1];
    value.extend((0u8..=255).rev());
    value.push(0);
    value
}
fn layout(bytes: &[u8]) -> Result<Layout, String> {
    if !(bytes.starts_with(b"GIF89a") || bytes.starts_with(b"GIF87a")) || bytes.len() < 13 {
        return Err("Invalid GIF header".into());
    }
    let mut pos = 13;
    if bytes[10] & 0x80 != 0 {
        pos += 3 * (1usize << ((bytes[10] & 7) + 1));
    }
    let mut xmp = Vec::new();
    loop {
        let start = pos;
        match *bytes.get(pos).ok_or("Missing GIF trailer")? {
            0x3b => return Ok(Layout { xmp, trailer: pos }),
            0x2c => {
                let desc = bytes
                    .get(pos + 1..pos + 10)
                    .ok_or("Truncated GIF image descriptor")?;
                pos += 10;
                if desc[8] & 0x80 != 0 {
                    pos += 3 * (1usize << ((desc[8] & 7) + 1));
                }
                let code = *bytes.get(pos).ok_or("Missing GIF LZW code size")?;
                if !(2..=8).contains(&code) {
                    return Err("Invalid GIF LZW code size".into());
                }
                pos = subblocks(bytes, pos + 1)?;
            }
            0x21 => {
                let label = *bytes.get(pos + 1).ok_or("Truncated GIF extension")?;
                pos += 2;
                let is_xmp = label == 0xff
                    && bytes.get(pos) == Some(&11)
                    && bytes.get(pos + 1..pos + 12) == Some(ID);
                if is_xmp {
                    let payload = pos + 12;
                    // XMP is a raw UTF-8 packet, NOT length-prefixed GIF data subblocks.
                    // Its mandated 258-byte magic trailer also makes GIF subblock readers skip it.
                    let magic = trailer();
                    let end = bytes[payload..]
                        .windows(magic.len())
                        .position(|w| w == magic)
                        .ok_or("Missing GIF XMP magic trailer")?;
                    if end > MAX_PACKET_BYTES {
                        return Err("GIF XMP too large".into());
                    }
                    pos = payload + end + magic.len();
                    xmp.push(start..pos);
                } else {
                    pos = subblocks(bytes, pos)?;
                }
            }
            _ => return Err("Invalid GIF block".into()),
        }
    }
}
pub(super) fn read(bytes: &[u8]) -> Result<Option<Packet>, String> {
    let parsed = layout(bytes)?;
    if parsed.xmp.len() > 1 {
        return Err("Multiple GIF XMP packets".into());
    }
    parsed
        .xmp
        .first()
        .map(|range| Packet::parse(&bytes[range.start + 14..range.end - 258]))
        .transpose()
}
pub(super) fn write(bytes: &[u8], packet: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let parsed = layout(bytes)?;
    let mut output = Vec::with_capacity(bytes.len() + packet.map_or(0, |p| p.len() + 272));
    let mut copied = 0;
    for range in parsed.xmp {
        output.extend_from_slice(&bytes[copied..range.start]);
        copied = range.end;
    }
    output.extend_from_slice(&bytes[copied..parsed.trailer]);
    if let Some(packet) = packet {
        output[..6].copy_from_slice(b"GIF89a");
        output.extend_from_slice(&[0x21, 0xff, 11]);
        output.extend_from_slice(ID);
        output.extend_from_slice(packet);
        output.extend_from_slice(&trailer());
    }
    output.extend_from_slice(&bytes[parsed.trailer..]);
    Ok(output)
}
