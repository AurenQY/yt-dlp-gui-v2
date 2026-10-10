//! Relocate MP Index entries after primary JPEG metadata edits, without touching
//! secondary JPEGs (including HDR gain maps). Offsets are relative to MP endian
//! header, except the primary image's mandated zero offset (CIPA DC-007).
use std::ops::Range;

struct Tiff<'a> {
    bytes: &'a [u8],
    little: bool,
}
impl Tiff<'_> {
    fn u16(&self, at: usize) -> Result<u16, String> {
        let end = at.checked_add(2).ok_or("MPF SHORT offset overflow")?;
        let raw = self.bytes.get(at..end).ok_or("Truncated MPF SHORT")?;
        let raw = raw.try_into().unwrap();
        Ok(if self.little {
            u16::from_le_bytes(raw)
        } else {
            u16::from_be_bytes(raw)
        })
    }
    fn u32(&self, at: usize) -> Result<u32, String> {
        let end = at.checked_add(4).ok_or("MPF LONG offset overflow")?;
        let raw = self.bytes.get(at..end).ok_or("Truncated MPF LONG")?;
        let raw = raw.try_into().unwrap();
        Ok(if self.little {
            u32::from_le_bytes(raw)
        } else {
            u32::from_be_bytes(raw)
        })
    }
}
fn put(bytes: &mut [u8], at: usize, value: usize, little: bool) -> Result<(), String> {
    let value = u32::try_from(value).map_err(|_| "Relocated MPF exceeds 32-bit limit")?;
    let raw = if little {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    };
    bytes
        .get_mut(at..at + 4)
        .ok_or("Truncated relocated MPF")?
        .copy_from_slice(&raw);
    Ok(())
}

pub(super) fn relocate(
    original: &[u8],
    output: &mut [u8],
    old_data: Range<usize>,
    new_data: Range<usize>,
    old_primary_end: usize,
    new_primary_end: usize,
) -> Result<(), String> {
    let old_base = old_data.start + 4;
    let new_base = new_data.start + 4;
    let data = original
        .get(old_base..old_data.end)
        .ok_or("Truncated MPF header")?;
    let little = match data.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return Err("Invalid MPF byte order".into()),
    };
    let tiff = Tiff {
        bytes: data,
        little,
    };
    if tiff.u16(2)? != 42 || old_data.len() != new_data.len() {
        return Err("Invalid MPF TIFF header".into());
    }
    let ifd = tiff.u32(4)? as usize;
    if ifd < 8 {
        return Err("Invalid MPF IFD offset".into());
    }
    let count = tiff.u16(ifd)? as usize;
    let table_end = ifd
        .checked_add(2 + count * 12 + 4)
        .filter(|end| *end <= data.len())
        .ok_or("Truncated MPF IFD")?;
    let mut number = None;
    let mut entries = None;
    for i in 0..count {
        let at = ifd + 2 + i * 12;
        match tiff.u16(at)? {
            0xb001 => {
                if number.is_some() || tiff.u16(at + 2)? != 4 || tiff.u32(at + 4)? != 1 {
                    return Err("Invalid MPF NumberOfImages".into());
                }
                number = Some(tiff.u32(at + 8)? as usize);
            }
            0xb002 => {
                if entries.is_some() || tiff.u16(at + 2)? != 7 {
                    return Err("Invalid MPF MPEntry".into());
                }
                entries = Some((tiff.u32(at + 8)? as usize, tiff.u32(at + 4)? as usize));
            }
            _ => {}
        }
    }
    let number = number
        .filter(|n| *n > 0)
        .ok_or("Missing MPF NumberOfImages")?;
    let (entry_start, length) = entries.ok_or("Missing MPF MPEntry")?;
    if number.checked_mul(16) != Some(length)
        || entry_start < table_end
        || entry_start
            .checked_add(length)
            .is_none_or(|end| end > data.len())
    {
        return Err("Invalid MPF entry table bounds".into());
    }
    // All edits are in the primary JPEG; its tail and every secondary byte stay
    // identical. Both the primary length and the MP-header-relative offsets move.
    if original[old_primary_end..] != output[new_primary_end..] {
        return Err("MPF secondary payload was changed".into());
    }
    let mut ranges = Vec::with_capacity(number);
    for i in 0..number {
        let at = entry_start + i * 16;
        let size = tiff.u32(at + 4)? as usize;
        let offset = tiff.u32(at + 8)? as usize;
        let start = if i == 0 {
            if offset != 0 || size != old_primary_end {
                return Err("Invalid MPF primary image bounds".into());
            }
            0
        } else {
            let start = old_base
                .checked_add(offset)
                .ok_or("MPF image offset overflow")?;
            if offset == 0 || start < old_primary_end {
                return Err("Invalid MPF secondary image offset".into());
            }
            start
        };
        let end = start
            .checked_add(size)
            .filter(|end| *end <= original.len())
            .ok_or("MPF image exceeds file bounds")?;
        if size < 4
            || original.get(start..start + 2) != Some(&[0xff, 0xd8])
            || original.get(end - 2..end) != Some(&[0xff, 0xd9])
        {
            return Err("Invalid MPF JPEG image boundaries".into());
        }
        ranges.push(start..end);
        if i == 0 {
            put(output, new_base + at + 4, new_primary_end, little)?;
        } else {
            let new_start = new_primary_end
                .checked_add(start - old_primary_end)
                .ok_or("Relocated MPF offset overflow")?;
            let relative = new_start
                .checked_sub(new_base)
                .ok_or("Invalid relocated MPF base")?;
            put(output, new_base + at + 8, relative, little)?;
        }
    }
    ranges.sort_by_key(|range| range.start);
    if ranges.windows(2).any(|pair| pair[0].end > pair[1].start) {
        return Err("Overlapping MPF images; original left unchanged".into());
    }
    Ok(())
}
