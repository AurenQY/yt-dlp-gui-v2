//! JPEG APP1 XMP + Extended XMP; all other markers and entropy bytes stay intact.
use super::{MAX_PACKET_BYTES, Packet, md5, packet::NOTE};
use std::ops::Range;
const STANDARD: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const EXTENDED: &[u8] = b"http://ns.adobe.com/xmp/extension/\0";
const STANDARD_LIMIT: usize = 65_533 - STANDARD.len();

struct Segment {
    range: Range<usize>,
    marker: u8,
    data: Range<usize>,
}
fn segments(bytes: &[u8]) -> Result<Vec<Segment>, String> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Err("Invalid JPEG signature".into());
    }
    let mut result = Vec::new();
    let mut pos = 2;
    let mut scan = false;
    while pos < bytes.len() {
        if scan {
            while pos < bytes.len() {
                if bytes[pos] != 0xff {
                    pos += 1;
                    continue;
                }
                let mut next = pos + 1;
                while bytes.get(next) == Some(&0xff) {
                    next += 1;
                }
                let marker = *bytes.get(next).ok_or("Truncated JPEG entropy")?;
                if marker == 0 || (0xd0..=0xd7).contains(&marker) {
                    pos = next + 1;
                    continue;
                }
                break;
            }
            scan = false;
        }
        let start = pos;
        if bytes.get(pos) != Some(&0xff) {
            return Err("Invalid JPEG marker".into());
        }
        while bytes.get(pos) == Some(&0xff) {
            pos += 1;
        }
        let marker = *bytes.get(pos).ok_or("Truncated JPEG marker")?;
        pos += 1;
        if marker == 0xd9 {
            result.push(Segment {
                range: start..pos,
                marker,
                data: pos..pos,
            });
            return Ok(result);
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            result.push(Segment {
                range: start..pos,
                marker,
                data: pos..pos,
            });
            continue;
        }
        if marker == 0 || marker == 0xd8 {
            return Err("Invalid nested JPEG marker".into());
        }
        let length = bytes.get(pos..pos + 2).ok_or("Truncated JPEG length")?;
        let length = u16::from_be_bytes(length.try_into().unwrap()) as usize;
        if length < 2 {
            return Err("Invalid JPEG segment length".into());
        }
        let end = pos
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or("Truncated JPEG segment")?;
        result.push(Segment {
            range: start..end,
            marker,
            data: pos + 2..end,
        });
        pos = end;
        if marker == 0xda {
            scan = true;
        }
    }
    Err("JPEG has no end marker".into())
}
fn is_xmp(segment: &Segment, bytes: &[u8]) -> bool {
    segment.marker == 0xe1
        && (bytes[segment.data.clone()].starts_with(STANDARD)
            || bytes[segment.data.clone()].starts_with(EXTENDED))
}

pub(super) fn read(bytes: &[u8]) -> Result<Option<Packet>, String> {
    let parts = segments(bytes)?;
    let main: Vec<_> = parts
        .iter()
        .filter(|s| s.marker == 0xe1 && bytes[s.data.clone()].starts_with(STANDARD))
        .collect();
    if main.len() > 1 {
        return Err("Multiple primary JPEG XMP packets".into());
    }
    let Some(main) = main.first() else {
        if parts.iter().any(|s| is_xmp(s, bytes)) {
            return Err("Orphaned JPEG Extended XMP".into());
        }
        return Ok(None);
    };
    let mut packet = Packet::parse(&bytes[main.data.start + STANDARD.len()..main.data.end])?;
    if let Some(guid) = packet.simple(NOTE, "HasExtendedXMP") {
        if guid.len() != 32 || !guid.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid JPEG XMP digest".into());
        }
        let mut chunks = Vec::new();
        let mut total = None;
        for part in &parts {
            if part.marker != 0xe1 {
                continue;
            }
            let data = &bytes[part.data.clone()];
            let Some(data) = data.strip_prefix(EXTENDED) else {
                continue;
            };
            if data.len() < 40 {
                return Err("Truncated Extended XMP header".into());
            }
            if !data[..32].eq_ignore_ascii_case(guid.as_bytes()) {
                return Err("Unreferenced Extended XMP packet".into());
            }
            let size = u32::from_be_bytes(data[32..36].try_into().unwrap()) as usize;
            let offset = u32::from_be_bytes(data[36..40].try_into().unwrap()) as usize;
            if size == 0 || size > MAX_PACKET_BYTES || total.is_some_and(|old| old != size) {
                return Err("Invalid Extended XMP total length".into());
            }
            total = Some(size);
            chunks.push((offset, &data[40..]));
        }
        let total = total.ok_or("Missing JPEG Extended XMP")?;
        chunks.sort_by_key(|(offset, _)| *offset);
        let mut extended = Vec::with_capacity(total);
        for (offset, chunk) in chunks {
            if offset != extended.len() || extended.len() + chunk.len() > total {
                return Err("Overlapping/incomplete Extended XMP".into());
            }
            extended.extend_from_slice(chunk);
        }
        if extended.len() != total || !md5::hex(&extended).eq_ignore_ascii_case(&guid) {
            return Err("JPEG Extended XMP digest/length mismatch".into());
        }
        packet.merge(Packet::parse(&extended)?)?;
        packet.delete(NOTE, "HasExtendedXMP");
    } else if parts
        .iter()
        .any(|s| s.marker == 0xe1 && bytes[s.data.clone()].starts_with(EXTENDED))
    {
        return Err("Unreferenced Extended XMP segments".into());
    }
    Ok(Some(packet))
}

fn app1(data: &[u8], output: &mut Vec<u8>) -> Result<(), String> {
    let length = u16::try_from(data.len() + 2).map_err(|_| "JPEG APP1 length overflow")?;
    output.extend_from_slice(&[0xff, 0xe1]);
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(data);
    Ok(())
}
pub(super) fn write(bytes: &[u8], packet: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let parts = segments(bytes)?;
    let mut replacement = Vec::new();
    if let Some(packet) = packet {
        if packet.len() <= STANDARD_LIMIT {
            let mut data = STANDARD.to_vec();
            data.extend_from_slice(packet);
            app1(&data, &mut replacement)?;
        } else {
            let mut main = Packet::parse(packet)?;
            let mut extra = Packet::new();
            main.set_simple(NOTE, "HasExtendedXMP", &"0".repeat(32));
            while main.serialize()?.len() > STANDARD_LIMIT {
                if !main.move_largest_property_to(&mut extra, (NOTE, "HasExtendedXMP"))? {
                    return Err("JPEG primary XMP cannot fit; original left unchanged".into());
                }
            }
            let extended = extra.serialize()?;
            let guid = md5::hex(&extended);
            main.set_simple(NOTE, "HasExtendedXMP", &guid);
            let mut data = STANDARD.to_vec();
            data.extend_from_slice(&main.serialize()?);
            app1(&data, &mut replacement)?;
            let total = u32::try_from(extended.len()).map_err(|_| "Extended XMP too large")?;
            let limit = 65_533 - EXTENDED.len() - 40;
            for (index, chunk) in extended.chunks(limit).enumerate() {
                let mut data = EXTENDED.to_vec();
                data.extend_from_slice(guid.as_bytes());
                data.extend_from_slice(&total.to_be_bytes());
                data.extend_from_slice(&((index * limit) as u32).to_be_bytes());
                data.extend_from_slice(chunk);
                app1(&data, &mut replacement)?;
            }
        }
    }
    let insertion = parts
        .iter()
        .find(|s| s.marker == 0xda || s.marker == 0xd9)
        .ok_or("JPEG has no insertion boundary")?
        .range
        .start;
    let mut output = Vec::with_capacity(bytes.len() + replacement.len());
    let mut copied = 0;
    for part in parts.iter().filter(|s| is_xmp(s, bytes)) {
        if copied <= insertion && insertion < part.range.start {
            output.extend_from_slice(&bytes[copied..insertion]);
            output.extend_from_slice(&replacement);
            copied = insertion;
        }
        output.extend_from_slice(&bytes[copied..part.range.start]);
        copied = part.range.end;
    }
    if copied <= insertion {
        output.extend_from_slice(&bytes[copied..insertion]);
        output.extend_from_slice(&replacement);
        copied = insertion;
    }
    output.extend_from_slice(&bytes[copied..]);
    let old_mpf: Vec<_> = parts
        .iter()
        .filter(|s| s.marker == 0xe2 && bytes[s.data.clone()].starts_with(b"MPF\0"))
        .collect();
    if !old_mpf.is_empty() {
        if old_mpf.len() != 1 {
            return Err("Multiple primary JPEG MPF segments; original left unchanged".into());
        }
        let updated = segments(&output)?;
        let new_mpf = updated
            .iter()
            .find(|s| s.marker == 0xe2 && output[s.data.clone()].starts_with(b"MPF\0"))
            .ok_or("JPEG MPF was not preserved")?;
        super::jpeg_mpf::relocate(
            bytes,
            &mut output,
            old_mpf[0].data.clone(),
            new_mpf.data.clone(),
            parts.last().unwrap().range.end,
            updated.last().unwrap().range.end,
        )?;
    }
    Ok(output)
}
