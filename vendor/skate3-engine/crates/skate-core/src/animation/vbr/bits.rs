//! TU3 Vector_UnPackFrameBlock82E8A928, expressed as scalar bit readers.
//! The native reverse-byte permutation830CEAD0 (initializer82F96DF8) makes
//! both streams little endian. Width nibbles come from big-endian header words.

struct Bits<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl Bits<'_> {
    fn read(&mut self, count: u32) -> Result<u32, String> {
        let mut value = 0;
        for bit in 0..count {
            let byte = self
                .bytes
                .get(self.position / 8)
                .ok_or("VBR coefficient bit stream truncated")?;
            value |= (((byte >> (self.position % 8)) & 1) as u32) << bit;
            self.position += 1;
        }
        Ok(value)
    }
}

pub(super) fn unpack(block: &[u8], widths: &[u32]) -> Result<Vec<[f32; 8]>, String> {
    if widths.is_empty() {
        return Ok(Vec::new());
    }
    let prefix = *block.first().ok_or("VBR block has no selector length")? as usize;
    let selectors = block.get(1..).ok_or("VBR selector stream missing")?;
    let magnitudes = block
        .get(1 + prefix.div_ceil(8)..)
        .ok_or("VBR selector length exceeds block")?;
    let mut selected = Bits {
        bytes: selectors,
        position: 0,
    };
    let mut packed = Bits {
        bytes: magnitudes,
        position: 0,
    };
    let mut output = Vec::with_capacity(widths.len());
    for &descriptor in widths {
        let mut coefficients = [0.0; 8];
        for (i, coefficient) in coefficients.iter_mut().enumerate() {
            let width = (descriptor >> (4 * i)) & 15;
            // A zero width consumes neither a selector nor packed data.
            if width != 0 && selected.read(1)? != 0 {
                let positive = packed.read(1)? != 0;
                let magnitude = packed.read(width)? as f32;
                // The original complements the sign bit, then shifts it to
                // bit31 and ORs it into the converted unsigned magnitude.
                // Preserve negative zero; this is not two's complement.
                *coefficient =
                    f32::from_bits(magnitude.to_bits() | if positive { 0 } else { 0x8000_0000 });
            }
        }
        output.push(coefficients);
    }
    Ok(output)
}

#[cfg(test)]
#[path = "tests/bits.rs"]
mod tests;
