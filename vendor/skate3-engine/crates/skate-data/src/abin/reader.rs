//! Checked big-endian reads with absolute offsets retained through subrecords.
use super::{Error, Result};
use std::ops::Range;

#[derive(Clone, Copy)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    begin: usize,
    end: usize,
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            begin: 0,
            end: bytes.len(),
        }
    }
    pub fn bounded(self, range: Range<usize>) -> Result<Self> {
        self.take(
            range.start,
            range
                .end
                .checked_sub(range.start)
                .ok_or_else(|| Error::new(range.start, "reversed record range"))?,
        )?;
        Ok(Self {
            bytes: self.bytes,
            begin: range.start,
            end: range.end,
        })
    }
    pub fn take(self, offset: usize, size: usize) -> Result<&'a [u8]> {
        let end = offset
            .checked_add(size)
            .ok_or_else(|| Error::new(offset, "read overflow"))?;
        if offset < self.begin || end > self.end {
            return Err(Error::new(
                offset,
                format!(
                    "read size{size} exceeds record {:#x}..{:#x}",
                    self.begin, self.end
                ),
            ));
        }
        Ok(&self.bytes[offset..end])
    }
    pub fn u8(self, offset: usize) -> Result<u8> {
        Ok(self.take(offset, 1)?[0])
    }
    pub fn u16(self, offset: usize) -> Result<u16> {
        Ok(u16::from_be_bytes(
            self.take(offset, 2)?.try_into().unwrap(),
        ))
    }
    pub fn u32(self, offset: usize) -> Result<u32> {
        Ok(u32::from_be_bytes(
            self.take(offset, 4)?.try_into().unwrap(),
        ))
    }
    pub fn i32(self, offset: usize) -> Result<i32> {
        Ok(self.u32(offset)? as i32)
    }
    pub fn words<const N: usize>(self, offset: usize) -> Result<[u32; N]> {
        let raw = self.take(
            offset,
            N.checked_mul(4)
                .ok_or_else(|| Error::new(offset, "word count overflow"))?,
        )?;
        Ok(core::array::from_fn(|i| {
            u32::from_be_bytes(raw[4 * i..4 * i + 4].try_into().unwrap())
        }))
    }
    ///Five words encode FastString30, six encode FastString36. Names stay
    ///uppercase as authored; padding remains word-local, matching stock names.
    pub fn name(self, offset: usize, words: usize) -> Result<String> {
        let raw = self.take(
            offset,
            words
                .checked_mul(4)
                .ok_or_else(|| Error::new(offset, "name size overflow"))?,
        )?;
        const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_";
        let mut result = String::new();
        for raw_word in raw.chunks_exact(4) {
            let mut value = u32::from_be_bytes(raw_word.try_into().unwrap());
            let mut divisor = 38u32.pow(5);
            for _ in 0..6 {
                let digit = value / divisor;
                value %= divisor;
                if digit == 0 {
                    if value != 0 {
                        return Err(Error::new(offset, "invalid FastString padding"));
                    }
                    break;
                }
                let character = ALPHABET
                    .get(digit as usize - 1)
                    .ok_or_else(|| Error::new(offset, "invalid FastString digit"))?;
                result.push(*character as char);
                divisor /= 38;
            }
        }
        Ok(result)
    }
    pub(crate) fn relative(self, base: usize, offset: u32, size: usize) -> Result<Range<usize>> {
        let start = base
            .checked_add(offset as usize)
            .ok_or_else(|| Error::new(base, "relative offset overflow"))?;
        self.take(start, size)?;
        Ok(start..start + size)
    }
}
