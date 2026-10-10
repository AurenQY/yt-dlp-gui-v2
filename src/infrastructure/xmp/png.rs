//! PNG/APNG XMP iTXt editing without touching image/animation chunks.
use super::{Packet, zlib};
use std::ops::Range;
const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const KEYWORD: &[u8] = b"XML:com.adobe.xmp\0";
struct Chunk {
    range: Range<usize>,
    kind: [u8; 4],
    data: Range<usize>,
}

fn chunks(bytes: &[u8]) -> Result<Vec<Chunk>, String> {
    if !bytes.starts_with(SIGNATURE) {
        return Err("Invalid PNG signature".into());
    }
    let mut result = Vec::new();
    let mut pos = 8;
    let mut has_data = false;
    while pos < bytes.len() {
        let header = bytes.get(pos..pos + 8).ok_or("Truncated PNG chunk")?;
        let length = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
        if length > 0x7fffffff {
            return Err("Invalid PNG chunk length".into());
        }
        let end = pos
            .checked_add(length)
            .and_then(|p| p.checked_add(12))
            .filter(|p| *p <= bytes.len())
            .ok_or("Truncated PNG payload")?;
        let kind: [u8; 4] = header[4..8].try_into().unwrap();
        if !kind.iter().all(u8::is_ascii_alphabetic) || kind[2].is_ascii_lowercase() {
            return Err("Invalid PNG chunk type".into());
        }
        if result.is_empty() && (kind != *b"IHDR" || length != 13) {
            return Err("Missing PNG IHDR".into());
        }
        if !result.is_empty() && kind == *b"IHDR" {
            return Err("Duplicate PNG IHDR".into());
        }
        let expected = u32::from_be_bytes(bytes[end - 4..end].try_into().unwrap());
        if crc32(&bytes[pos + 4..end - 4]) != expected {
            return Err("PNG chunk CRC mismatch".into());
        }
        has_data |= kind == *b"IDAT";
        result.push(Chunk {
            range: pos..end,
            kind,
            data: pos + 8..end - 4,
        });
        pos = end;
        if kind == *b"IEND" {
            if length != 0 || !has_data {
                return Err("Invalid PNG end/data".into());
            }
            return Ok(result);
        }
    }
    Err("Missing PNG IEND".into())
}
fn is_xmp(chunk: &Chunk, bytes: &[u8]) -> bool {
    chunk.kind == *b"iTXt" && bytes[chunk.data.clone()].starts_with(KEYWORD)
}
pub(super) fn read(bytes: &[u8]) -> Result<Option<Packet>, String> {
    let parts = chunks(bytes)?;
    let mut packet = None;
    for part in parts.iter().filter(|chunk| is_xmp(chunk, bytes)) {
        if packet.is_some() {
            return Err("Multiple PNG XMP packets".into());
        }
        let data = &bytes[part.data.start + KEYWORD.len()..part.data.end];
        let flag = *data.first().ok_or("Missing PNG XMP compression flag")?;
        if flag > 1 || data.get(1) != Some(&0) {
            return Err("Invalid PNG XMP compression".into());
        }
        let mut rest = &data[2..];
        for _ in 0..2 {
            let end = rest
                .iter()
                .position(|b| *b == 0)
                .ok_or("Truncated PNG iTXt header")?;
            rest = &rest[end + 1..];
        }
        packet = Some(if flag == 0 {
            Packet::parse(rest)?
        } else {
            Packet::parse(&zlib::inflate(rest)?)?
        });
    }
    Ok(packet)
}
pub(super) fn write(bytes: &[u8], packet: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let parts = chunks(bytes)?;
    let mut replacement = Vec::new();
    if let Some(packet) = packet {
        let mut data = KEYWORD.to_vec();
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(packet);
        let length = u32::try_from(data.len()).map_err(|_| "PNG XMP too large")?;
        replacement.extend_from_slice(&length.to_be_bytes());
        replacement.extend_from_slice(b"iTXt");
        replacement.extend_from_slice(&data);
        let crc = crc32(&replacement[4..]);
        replacement.extend_from_slice(&crc.to_be_bytes());
    }
    let mut output = Vec::with_capacity(bytes.len() + replacement.len());
    output.extend_from_slice(SIGNATURE);
    for chunk in &parts {
        if chunk.kind == *b"IEND" {
            output.extend_from_slice(&replacement);
        }
        if !is_xmp(chunk, bytes) {
            output.extend_from_slice(&bytes[chunk.range.clone()]);
        }
    }
    let end = parts.last().ok_or("PNG has no chunks")?.range.end;
    output.extend_from_slice(&bytes[end..]);
    Ok(output)
}
pub(super) fn crc32(bytes: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0; 256];
        let mut index = 0;
        while index < 256 {
            let mut crc = index as u32;
            let mut bit = 0;
            while bit < 8 {
                crc = if crc & 1 != 0 {
                    0xedb88320 ^ (crc >> 1)
                } else {
                    crc >> 1
                };
                bit += 1;
            }
            table[index] = crc;
            index += 1;
        }
        table
    };
    let mut crc = !0u32;
    for byte in bytes {
        crc = TABLE[((crc ^ *byte as u32) & 255) as usize] ^ (crc >> 8);
    }
    !crc
}
