//! Original TU3 InitPerAnim82E88FA8 and UnPackHeaderBits82E8A780.
use std::ops::Range;

#[derive(Clone, Debug)]
pub(super) struct Channel {
    pub dynamic: usize,
    pub constant: usize,
    pub components: usize,
    pub min: f32,
    pub range: f32,
}

pub(super) struct Header {
    pub frames: usize,
    pub bones: usize,
    pub dct_scale: f32,
    pub channels: Vec<Channel>,
    pub constants: Vec<f32>,
    pub constant_flags: Vec<bool>,
    pub widths: Vec<u32>,
    pub blocks: Vec<Range<usize>>,
}

impl Header {
    pub fn read(
        bytes: &[u8],
        part: usize,
        header_relative: usize,
        data_relative: usize,
    ) -> Result<Self, String> {
        let header_word = u32_at(bytes, part)?;
        let bones = ((header_word >> 6) & 63) as usize + 1;
        let header = offset(part, header_relative)?;
        let header_end = offset(header, (header_word >> 12) as usize)?;
        let data = offset(part, data_relative)?;
        let data_end = offset(data, u32_at(bytes, offset(part, 4)?)? as usize)?;
        take(
            bytes,
            header,
            header_end.checked_sub(header).ok_or("VBR header range")?,
        )?;
        take(
            bytes,
            data,
            data_end.checked_sub(data).ok_or("VBR data range")?,
        )?;
        let frames = u16_at(bytes, header + 2)? as usize;
        let count = take(bytes, header + 12, 2)?;
        if count[0] != 3 || frames == 0 {
            return Err("VBR SQT requires three channel types and a nonempty frame range".into());
        }
        let dictionary = header + 16;
        let dictionary_count = count[1] as usize;
        let size_table = offset(dictionary, dictionary_count * 4)?;
        let block_count = frames.div_ceil(8);
        let types = offset(offset(size_table, block_count * 2)?, 15)? & !15;
        if offset(types, 48)? > header_end {
            return Err("VBR channel descriptions exceed compression header".into());
        }
        let mut channels = Vec::with_capacity(3);
        for (i, expected_components) in [3, 4, 3].into_iter().enumerate() {
            let at = types + 16 * i;
            let dynamic = u16_at(bytes, at)? as usize;
            let constant = u16_at(bytes, at + 2)? as usize;
            let components = take(bytes, at + 12, 1)?[0] as usize;
            let min = f32::from_bits(u32_at(bytes, at + 4)?);
            let max = f32::from_bits(u32_at(bytes, at + 8)?);
            if components != expected_components || dynamic + constant != bones {
                return Err(format!(
                    "VBR channel {i} does not match its SQT output descriptor"
                ));
            }
            channels.push(Channel {
                dynamic,
                constant,
                components,
                min,
                range: max - min,
            });
        }
        let mut cursor = data;
        let mut constants = Vec::new();
        for channel in &channels {
            for _ in 0..channel.constant * channel.components {
                let index = take(bytes, cursor, 1)?[0] as usize;
                cursor += 1;
                if index >= dictionary_count {
                    return Err("VBR constant dictionary index out of bounds".into());
                }
                let value = f32::from_bits(u32_at(bytes, dictionary + index * 4)?);
                // Original scalar fmadds82E8A804..878, including constants.
                constants.push(value.mul_add(channel.range, channel.min));
            }
        }
        let runs = u16_at(bytes, header)? as usize;
        let mut constant_flags = Vec::with_capacity(3 * bones);
        for run in 0..runs {
            let length = take(bytes, cursor, 1)?[0] as usize;
            cursor += 1;
            if constant_flags.len() + length > 3 * bones {
                return Err("VBR constant-channel run exceeds channel count".into());
            }
            constant_flags.extend(std::iter::repeat_n(run % 2 == 1, length));
        }
        if runs == 0 && constants.is_empty() {
            constant_flags.resize(3 * bones, false);
        }
        if constant_flags.len() != 3 * bones {
            return Err("VBR constant-channel runs do not cover all channels".into());
        }
        for (i, channel) in channels.iter().enumerate() {
            if constant_flags[i * bones..(i + 1) * bones]
                .iter()
                .filter(|v| **v)
                .count()
                != channel.constant
            {
                return Err(format!(
                    "VBR channel {i} constant count differs from its runs"
                ));
            }
        }
        let dynamic_count = channels
            .iter()
            .map(|c| c.dynamic * c.components)
            .sum::<usize>();
        let mut widths = Vec::with_capacity(dynamic_count);
        for _ in 0..dynamic_count {
            widths.push(u32_at(bytes, cursor)?);
            cursor += 4;
        }
        let block_start = cursor;
        let mut blocks = Vec::with_capacity(block_count);
        // Vector_ExtractPackedNVBR82D1E0E4..118 accumulates u16 block sizes.
        let mut displacement = 0u16;
        for block in 0..block_count {
            let size = u16_at(bytes, size_table + 2 * block)?;
            let start = offset(block_start, displacement as usize)?;
            let end = offset(start, size as usize)?;
            if end > data_end || end < start || size < 3 {
                return Err(format!("VBR block {block} exceeds compressed data"));
            }
            blocks.push(start..end);
            displacement = displacement.wrapping_add(size);
        }
        let low = f32::from_bits(u32_at(bytes, header + 4)?).abs();
        let high = f32::from_bits(u32_at(bytes, header + 8)?).abs();
        Ok(Self {
            frames,
            bones,
            dct_scale: low.max(high),
            channels,
            constants,
            constant_flags,
            widths,
            blocks,
        })
    }
}

pub(super) fn offset(base: usize, relative: usize) -> Result<usize, String> {
    base.checked_add(relative)
        .ok_or_else(|| "VBR offset overflow".into())
}
pub(super) fn take(bytes: &[u8], at: usize, count: usize) -> Result<&[u8], String> {
    bytes
        .get(at..offset(at, count)?)
        .ok_or_else(|| format!("VBR read {at:#x}+{count} out of bounds"))
}
pub(super) fn u16_at(bytes: &[u8], at: usize) -> Result<u16, String> {
    Ok(u16::from_be_bytes(take(bytes, at, 2)?.try_into().unwrap()))
}
pub(super) fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_be_bytes(take(bytes, at, 4)?.try_into().unwrap()))
}
