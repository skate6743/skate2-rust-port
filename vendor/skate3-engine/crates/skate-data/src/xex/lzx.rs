//! LZX decompression as used by XEX2 "normal" compression: one continuous
//! stream, 32 KiB frames, no reset interval, no E8 translation.
//! Follows the classic (non-DELTA) LZX decoder of libmspack's lzxd.c.

const FRAME: usize = 32768;
const MIN_MATCH: usize = 2;
const NUM_CHARS: usize = 256;
const PRETREE: usize = 20;
const LENGTH_TREE: usize = 249;
const ALIGNED_TREE: usize = 8;
const MAX_BITS: usize = 16;

/// 16-bit little-endian words, consumed most-significant bit first.
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    buffer: u32,
    left: u32,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            buffer: 0,
            left: 0,
        }
    }
    fn byte(&mut self) -> u8 {
        // Past the end the stream reads as zeros, like libmspack's padding.
        let b = self.data.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b
    }
    fn ensure(&mut self, n: u32) {
        while self.left < n {
            let lo = u32::from(self.byte());
            let hi = u32::from(self.byte());
            self.buffer |= ((hi << 8) | lo) << (16 - self.left);
            self.left += 16;
        }
    }
    fn peek(&mut self, n: u32) -> u32 {
        self.ensure(n);
        self.buffer >> (32 - n)
    }
    fn remove(&mut self, n: u32) {
        self.buffer = if n >= 32 { 0 } else { self.buffer << n };
        self.left -= n;
    }
    fn read(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        if n > 16 {
            let hi = self.read(n - 16);
            return (hi << 16) | self.read(16);
        }
        let v = self.peek(n);
        self.remove(n);
        v
    }
    /// libmspack's start-of-uncompressed-block realignment.
    fn realign_to_bytes(&mut self) {
        self.ensure(16);
        if self.left > 16 {
            self.pos -= 2;
        }
        self.left = 0;
        self.buffer = 0;
    }
    /// End-of-frame realignment to a 16-bit boundary.
    fn realign_frame(&mut self) {
        if self.left > 0 {
            self.ensure(16);
        }
        if self.left & 15 != 0 {
            let n = self.left & 15;
            self.remove(n);
        }
    }
    fn le32(&mut self) -> u32 {
        u32::from_le_bytes([self.byte(), self.byte(), self.byte(), self.byte()])
    }
}

/// Canonical Huffman decoder built from code lengths (MSB-first codes).
struct Tree {
    /// Symbols sorted by (length, symbol).
    symbols: Vec<u16>,
    /// Per length: number of codes.
    counts: [u16; MAX_BITS + 1],
    empty: bool,
}

impl Tree {
    fn new(lengths: &[u8]) -> Result<Self, String> {
        let mut counts = [0u16; MAX_BITS + 1];
        for &l in lengths {
            if usize::from(l) > MAX_BITS {
                return Err("LZX code length exceeds 16".into());
            }
            counts[usize::from(l)] += 1;
        }
        counts[0] = 0;
        let mut symbols: Vec<u16> = (0..lengths.len() as u16)
            .filter(|&s| lengths[s as usize] != 0)
            .collect();
        symbols.sort_by_key(|&s| (lengths[s as usize], s));
        // Over-subscribed trees are invalid; incomplete ones are tolerated
        // only when empty (libmspack accepts an all-zero tree).
        let mut left: i32 = 1;
        for len in 1..=MAX_BITS {
            left = (left << 1) - i32::from(counts[len]);
            if left < 0 {
                return Err("LZX Huffman tree is over-subscribed".into());
            }
        }
        let empty = symbols.is_empty();
        if !empty && left != 0 {
            return Err("LZX Huffman tree is incomplete".into());
        }
        Ok(Self {
            symbols,
            counts,
            empty,
        })
    }

    fn decode(&self, bits: &mut Bits<'_>) -> Result<usize, String> {
        if self.empty {
            return Err("LZX decode from an empty Huffman tree".into());
        }
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: i32 = 0;
        for len in 1..=MAX_BITS {
            code |= bits.read(1) as i32;
            let count = i32::from(self.counts[len]);
            if code - first < count {
                return Ok(usize::from(self.symbols[(index + code - first) as usize]));
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err("LZX Huffman code not found".into())
    }
}

fn read_lengths(
    bits: &mut Bits<'_>,
    lengths: &mut [u8],
    first: usize,
    last: usize,
) -> Result<(), String> {
    let mut pre = [0u8; PRETREE];
    for l in &mut pre {
        *l = bits.read(4) as u8;
    }
    let pretree = Tree::new(&pre)?;
    let mut x = first;
    while x < last {
        let z = pretree.decode(bits)?;
        match z {
            17 => {
                let run = bits.read(4) as usize + 4;
                fill(lengths, &mut x, last, run, 0)?;
            }
            18 => {
                let run = bits.read(5) as usize + 20;
                fill(lengths, &mut x, last, run, 0)?;
            }
            19 => {
                let run = bits.read(1) as usize + 4;
                let z = pretree.decode(bits)?;
                let v = (i32::from(lengths[x]) - z as i32).rem_euclid(17) as u8;
                fill(lengths, &mut x, last, run, v)?;
            }
            _ => {
                lengths[x] = (i32::from(lengths[x]) - z as i32).rem_euclid(17) as u8;
                x += 1;
            }
        }
    }
    Ok(())
}

fn fill(
    lengths: &mut [u8],
    x: &mut usize,
    last: usize,
    run: usize,
    value: u8,
) -> Result<(), String> {
    if *x + run > last {
        return Err("LZX length run overflows the tree".into());
    }
    lengths[*x..*x + run].fill(value);
    *x += run;
    Ok(())
}

/// Decompresses `output_len` bytes with a window of `window_size` bytes.
pub(crate) fn decompress(
    input: &[u8],
    window_size: usize,
    output_len: usize,
) -> Result<Vec<u8>, String> {
    let window_bits = window_size.trailing_zeros();
    if !window_size.is_power_of_two() || !(15..=21).contains(&window_bits) {
        return Err(format!("Unsupported LZX window size {window_size}"));
    }
    let position_slots = match window_bits {
        20 => 42,
        21 => 50,
        b => (b * 2) as usize,
    };
    let mut extra_bits = [0u32; 51];
    let mut position_base = [0usize; 51];
    let mut j = 0;
    let mut i = 0;
    while i < 51 {
        extra_bits[i] = j;
        if i + 1 < 51 {
            extra_bits[i + 1] = j;
        }
        if i != 0 && j < 17 {
            j += 1;
        }
        i += 2;
    }
    let mut base = 0;
    for i in 0..51 {
        position_base[i] = base;
        base += 1 << extra_bits[i];
    }

    let main_size = NUM_CHARS + position_slots * 8;
    let mut main_lengths = vec![0u8; main_size];
    let mut length_lengths = vec![0u8; LENGTH_TREE];
    let mut bits = Bits::new(input);
    let mut out = Vec::with_capacity(output_len);
    let mut r = [1usize; 3];

    // Header: E8 translation flag (+ 32-bit file size when set).
    if bits.read(1) != 0 {
        return Err("LZX E8 translation is not supported for XEX images".into());
    }

    let mut block_type = 0u32;
    let mut block_remaining = 0usize;
    let mut block_length = 0usize;
    let mut main_tree: Option<Tree> = None;
    let mut length_tree: Option<Tree> = None;
    let mut aligned_tree: Option<Tree> = None;

    while out.len() < output_len {
        let frame_end = (out.len() + FRAME).min(output_len);
        while out.len() < frame_end {
            if block_remaining == 0 {
                if block_type == 3 && block_length & 1 == 1 {
                    bits.byte();
                }
                block_type = bits.read(3);
                let hi = bits.read(16) as usize;
                let lo = bits.read(8) as usize;
                block_length = (hi << 8) | lo;
                block_remaining = block_length;
                match block_type {
                    2 | 1 => {
                        if block_type == 2 {
                            let mut aligned = [0u8; ALIGNED_TREE];
                            for l in &mut aligned {
                                *l = bits.read(3) as u8;
                            }
                            aligned_tree = Some(Tree::new(&aligned)?);
                        }
                        read_lengths(&mut bits, &mut main_lengths, 0, NUM_CHARS)?;
                        read_lengths(&mut bits, &mut main_lengths, NUM_CHARS, main_size)?;
                        main_tree = Some(Tree::new(&main_lengths)?);
                        read_lengths(&mut bits, &mut length_lengths, 0, LENGTH_TREE)?;
                        length_tree = Some(Tree::new(&length_lengths)?);
                    }
                    3 => {
                        bits.realign_to_bytes();
                        for slot in &mut r {
                            *slot = bits.le32() as usize;
                        }
                    }
                    _ => return Err(format!("Invalid LZX block type {block_type}")),
                }
                if block_remaining == 0 {
                    return Err("Empty LZX block".into());
                }
            }
            let this_run = block_remaining.min(frame_end - out.len());
            let target = out.len() + this_run;
            if block_type == 3 {
                for _ in 0..this_run {
                    let b = bits.byte();
                    out.push(b);
                }
                block_remaining -= this_run;
                continue;
            }
            let main = main_tree.as_ref().ok_or("LZX main tree missing")?;
            let start = out.len();
            while out.len() < target {
                let symbol = main.decode(&mut bits)?;
                if symbol < NUM_CHARS {
                    out.push(symbol as u8);
                    continue;
                }
                let symbol = symbol - NUM_CHARS;
                let mut length = symbol & 7;
                if length == 7 {
                    length += length_tree
                        .as_ref()
                        .ok_or("LZX length tree missing")?
                        .decode(&mut bits)?;
                }
                length += MIN_MATCH;
                let slot = symbol >> 3;
                let offset = if slot > 2 {
                    let extra = extra_bits[slot];
                    let offset = if block_type == 1 {
                        if slot == 3 {
                            1
                        } else {
                            position_base[slot] - 2 + bits.read(extra) as usize
                        }
                    } else {
                        let aligned = aligned_tree.as_ref().ok_or("LZX aligned tree missing")?;
                        let mut offset = position_base[slot] - 2;
                        if extra > 3 {
                            offset += (bits.read(extra - 3) as usize) << 3;
                            offset += aligned.decode(&mut bits)?;
                        } else if extra == 3 {
                            offset += aligned.decode(&mut bits)?;
                        } else if extra > 0 {
                            offset += bits.read(extra) as usize;
                        } else {
                            offset = 1;
                        }
                        offset
                    };
                    r[2] = r[1];
                    r[1] = r[0];
                    r[0] = offset;
                    offset
                } else if slot == 0 {
                    r[0]
                } else if slot == 1 {
                    r.swap(0, 1);
                    r[0]
                } else {
                    r.swap(0, 2);
                    r[0]
                };
                if offset == 0 || offset > out.len() || offset > window_size {
                    return Err(format!(
                        "LZX match offset {offset} outside the window at {}",
                        out.len()
                    ));
                }
                for _ in 0..length {
                    let b = out[out.len() - offset];
                    out.push(b);
                }
            }
            let decoded = out.len() - start;
            if decoded > block_remaining {
                return Err("LZX match overran its block".into());
            }
            block_remaining -= decoded;
            if out.len() > frame_end {
                return Err("LZX match overran its frame".into());
            }
        }
        bits.realign_frame();
    }
    out.truncate(output_len);
    Ok(out)
}
