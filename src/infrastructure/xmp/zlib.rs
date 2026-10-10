//! Bounded RFC 1950/1951 reader used ONLY for existing compressed PNG XMP iTXt.
//! Image pixels are never inflated. New XMP is stored uncompressed.
use super::MAX_PACKET_BYTES;
struct Bits<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl Bits<'_> {
    fn read(&mut self, count: usize) -> Result<usize, String> {
        let mut value = 0;
        for bit in 0..count {
            let byte = *self
                .bytes
                .get(self.pos / 8)
                .ok_or("Truncated XMP deflate stream")?;
            value |= (((byte >> (self.pos % 8)) & 1) as usize) << bit;
            self.pos += 1;
        }
        Ok(value)
    }
    fn align(&mut self) {
        self.pos = (self.pos + 7) & !7;
    }
}
#[derive(Default)]
struct Branch {
    children: [Option<usize>; 2],
    symbol: Option<usize>,
}
struct Huffman {
    tree: Vec<Branch>,
}
impl Huffman {
    fn new(lengths: &[usize]) -> Result<Self, String> {
        let mut counts = [0usize; 16];
        for length in lengths {
            if *length > 15 {
                return Err("Invalid deflate code length".into());
            }
            if *length > 0 {
                counts[*length] += 1;
            }
        }
        let mut remaining = 1isize;
        for count in &counts[1..] {
            remaining = remaining * 2 - *count as isize;
            if remaining < 0 {
                return Err("Oversubscribed deflate tree".into());
            }
        }
        let mut next = [0usize; 16];
        let mut code = 0;
        for bits in 1..16 {
            code = (code + counts[bits - 1]) << 1;
            next[bits] = code;
        }
        let mut tree = vec![Branch::default()];
        for (symbol, length) in lengths.iter().copied().enumerate() {
            if length == 0 {
                continue;
            }
            let code = next[length];
            next[length] += 1;
            let mut node = 0;
            for bit in (0..length).rev() {
                if tree[node].symbol.is_some() {
                    return Err("Invalid deflate tree prefix".into());
                }
                let direction = (code >> bit) & 1;
                let child = if let Some(child) = tree[node].children[direction] {
                    child
                } else {
                    let child = tree.len();
                    tree.push(Branch::default());
                    tree[node].children[direction] = Some(child);
                    child
                };
                node = child;
            }
            tree[node].symbol = Some(symbol);
        }
        Ok(Self { tree })
    }
    fn symbol(&self, bits: &mut Bits<'_>) -> Result<usize, String> {
        let mut node = 0;
        for _ in 0..=15 {
            if let Some(symbol) = self.tree[node].symbol {
                return Ok(symbol);
            }
            node = self.tree[node].children[bits.read(1)?].ok_or("Invalid deflate symbol")?;
        }
        Err("Deflate code too long".into())
    }
}
fn dynamic(bits: &mut Bits<'_>) -> Result<(Huffman, Huffman), String> {
    let literal_count = bits.read(5)? + 257;
    let distance_count = bits.read(5)? + 1;
    let code_count = bits.read(4)? + 4;
    if literal_count > 286 {
        return Err("Invalid deflate literal count".into());
    }
    let order = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut lengths = [0; 19];
    for index in 0..code_count {
        lengths[order[index]] = bits.read(3)?;
    }
    let codes = Huffman::new(&lengths)?;
    let total = literal_count + distance_count;
    let mut lengths = Vec::with_capacity(total);
    while lengths.len() < total {
        match codes.symbol(bits)? {
            value @ 0..=15 => lengths.push(value),
            value @ 16..=18 => {
                let (repeat, length) = match value {
                    16 => (
                        bits.read(2)? + 3,
                        *lengths.last().ok_or("Deflate repeat without predecessor")?,
                    ),
                    17 => (bits.read(3)? + 3, 0),
                    _ => (bits.read(7)? + 11, 0),
                };
                if lengths.len() + repeat > total {
                    return Err("Deflate code repeat overflow".into());
                }
                lengths.extend(std::iter::repeat_n(length, repeat));
            }
            _ => return Err("Invalid deflate code length symbol".into()),
        }
    }
    if lengths[256] == 0 {
        return Err("Deflate tree has no end-of-block".into());
    }
    Ok((
        Huffman::new(&lengths[..literal_count])?,
        Huffman::new(&lengths[literal_count..])?,
    ))
}
fn compressed(
    bits: &mut Bits<'_>,
    output: &mut Vec<u8>,
    literal: &Huffman,
    distance: &Huffman,
    window: usize,
) -> Result<(), String> {
    const LENGTH_BASE: [usize; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LENGTH_BITS: [usize; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DISTANCE_BASE: [usize; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    loop {
        match literal.symbol(bits)? {
            value @ 0..=255 => {
                if output.len() >= MAX_PACKET_BYTES {
                    return Err("Inflated XMP exceeds processing limit".into());
                }
                output.push(value as u8);
            }
            256 => return Ok(()),
            value @ 257..=285 => {
                let index = value - 257;
                let length = LENGTH_BASE[index] + bits.read(LENGTH_BITS[index])?;
                let code = distance.symbol(bits)?;
                if code >= 30 {
                    return Err("Reserved deflate distance".into());
                }
                let distance =
                    DISTANCE_BASE[code] + bits.read(if code < 4 { 0 } else { code / 2 - 1 })?;
                if distance > output.len()
                    || distance > window
                    || output.len() + length > MAX_PACKET_BYTES
                {
                    return Err("Invalid deflate distance/output length".into());
                }
                for _ in 0..length {
                    output.push(output[output.len() - distance]);
                }
            }
            _ => return Err("Reserved deflate length".into()),
        }
    }
}
pub(super) fn inflate(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < 6
        || bytes[0] & 15 != 8
        || bytes[0] >> 4 > 7
        || u16::from_be_bytes([bytes[0], bytes[1]]) % 31 != 0
        || bytes[1] & 0x20 != 0
    {
        return Err("Invalid/unsupported XMP zlib header".into());
    }
    let window = 1usize << ((bytes[0] >> 4) + 8);
    let payload = &bytes[2..bytes.len() - 4];
    let mut bits = Bits {
        bytes: payload,
        pos: 0,
    };
    let mut output = Vec::new();
    loop {
        let final_block = bits.read(1)? != 0;
        match bits.read(2)? {
            0 => {
                bits.align();
                let length = bits.read(16)?;
                if bits.read(16)? != (length ^ 65535) {
                    return Err("Invalid deflate stored length".into());
                }
                if output.len() + length > MAX_PACKET_BYTES {
                    return Err("Inflated XMP exceeds processing limit".into());
                }
                for _ in 0..length {
                    output.push(bits.read(8)? as u8);
                }
            }
            1 => {
                let mut lengths = [8; 288];
                lengths[144..256].fill(9);
                lengths[256..280].fill(7);
                let literal = Huffman::new(&lengths)?;
                let distance = Huffman::new(&[5; 32])?;
                compressed(&mut bits, &mut output, &literal, &distance, window)?;
            }
            2 => {
                let (literal, distance) = dynamic(&mut bits)?;
                compressed(&mut bits, &mut output, &literal, &distance, window)?;
            }
            _ => return Err("Reserved deflate block".into()),
        }
        if final_block {
            break;
        }
    }
    if bits.pos.div_ceil(8) != payload.len() {
        return Err("Trailing XMP deflate bytes".into());
    }
    let mut a = 1u32;
    let mut b = 0u32;
    for byte in &output {
        a = (a + *byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    if (b << 16 | a) != u32::from_be_bytes(bytes[bytes.len() - 4..].try_into().unwrap()) {
        return Err("XMP zlib checksum mismatch".into());
    }
    Ok(output)
}
