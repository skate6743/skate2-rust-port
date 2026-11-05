//! Lossless original ABIN metadata layouts. No pose codec or numerical decoder.
//! Clip header827B8AB0, attribute82D164F8, tree lookup82D1B5B8.
use super::{
    AnimationMetadata, ClipAttribute, ClipMetadata, File, PhaseBlendMetadata, SelectorMetadata,
    UnsupportedTree,
};
use crate::abin::{Bank, Clip, Error, Reader, RecordData, RecordHeader, Result};

impl AnimationMetadata {
    /// Read the retained bytes without constructing or evaluating animation
    /// trees. Caller supplies the independently calculated bank SHA-256.
    pub fn from_bank(
        bank: &Bank,
        source_bank: String,
        source_sha256: String,
    ) -> std::result::Result<Self, String> {
        let mut file = File {
            version: 1,
            source_bank,
            source_sha256,
            source_bytes: bank.bytes().len() as u64,
            clips: Vec::new(),
            phase_blends: Vec::new(),
            blend_spaces: Vec::new(),
            selectors: Vec::new(),
            selection_spaces: Vec::new(),
            unsupported_trees: Vec::new(),
        };
        let reader = Reader::new(bank.bytes());
        for record in bank.records() {
            let h = &record.header;
            let r = reader.bounded(h.range()).map_err(|e| e.to_string())?;
            match &record.data {
                RecordData::Clip(clip) => file
                    .clips
                    .push(read_clip(r, h, clip).map_err(|e| e.to_string())?),
                RecordData::Pose(_) | RecordData::Hierarchy(_) | RecordData::PhysicsPose(_) => {}
                RecordData::Opaque => match h.type_id {
                    6 => file
                        .blend_spaces
                        .push(super::blend_space::read(r, h).map_err(|e| e.to_string())?),
                    7 => file
                        .phase_blends
                        .push(read_phase_blend(r, h).map_err(|e| e.to_string())?),
                    8 => file
                        .selectors
                        .push(read_selector(r, h).map_err(|e| e.to_string())?),
                    11 => file
                        .selection_spaces
                        .push(super::selection_space::read(r, h).map_err(|e| e.to_string())?),
                    kind => file.unsupported_trees.push(UnsupportedTree {
                        name: h.name.clone(),
                        source_offset: h.offset as u64,
                        type_id: kind,
                    }),
                },
            }
        }
        Self::from_file(file)
    }
}

fn read_clip(r: Reader<'_>, h: &RecordHeader, clip: &Clip) -> Result<ClipMetadata> {
    let mut at = clip.attribute_offset;
    let mut attributes = Vec::with_capacity(clip.attribute_count as usize);
    for _ in 0..clip.attribute_count {
        // Original GetAttribute82D25CA0 advances by the record's first word.
        let size = r.u32(at)? as usize;
        if size < 48 {
            return Err(Error::new(at, "attribute record smaller than header"));
        }
        let end = at
            .checked_add(size)
            .ok_or_else(|| Error::new(at, "attribute size overflow"))?;
        let a = r.bounded(at..end)?;
        let data = at
            .checked_add(51)
            .ok_or_else(|| Error::new(at, "attribute alignment overflow"))?
            & !15;
        let payload = a.take(
            data,
            end.checked_sub(data)
                .ok_or_else(|| Error::new(at, "attribute payload outside record"))?,
        )?;
        if payload.len() % 4 != 0 {
            return Err(Error::new(at, "unaligned attribute payload"));
        }
        attributes.push(ClipAttribute {
            name: a.name(at + 12, 5)?,
            type_id: a.u8(at + 32)?,
            begin_bits: a.u32(at + 4)?,
            end_bits: a.u32(at + 8)?,
            payload_words: payload
                .chunks_exact(4)
                .map(|word| u32::from_be_bytes(word.try_into().unwrap()))
                .collect(),
            source_offset: at as u64,
        });
        at = end;
    }
    Ok(ClipMetadata {
        name: h.name.clone(),
        source_offset: h.offset as u64,
        fps_bits: clip.fps_bits,
        frames_bits: clip.frame_count_bits,
        base_speed_bits: clip.base_speed_bits,
        flags_word: clip.flags,
        attributes,
    })
}

fn read_phase_blend(r: Reader<'_>, h: &RecordHeader) -> Result<PhaseBlendMetadata> {
    // Type7 construction82D1B5B8: count+8, parameter+16, child names+36.
    let p = h.payload_offset;
    let count = r.u32(p + 8)? as usize;
    let size = count
        .checked_mul(24)
        .ok_or_else(|| Error::new(p, "PhaseBlend child count overflow"))?;
    r.take(p + 36, size)?;
    let children = (0..count)
        .map(|i| r.name(p + 36 + 24 * i, 6))
        .collect::<Result<Vec<_>>>()?;
    Ok(PhaseBlendMetadata {
        name: h.name.clone(),
        source_offset: h.offset as u64,
        parameter: r.name(p + 16, 5)?,
        children,
    })
}

fn read_selector(r: Reader<'_>, h: &RecordHeader) -> Result<SelectorMetadata> {
    // Type8 construction82D1B5B8 keeps the first name as authored default.
    // Child/value order matters: runtime chooses the first matching value.
    let p = h.payload_offset;
    let count = r.u32(p + 28)? as usize;
    let names_size = count
        .max(1)
        .checked_mul(24)
        .ok_or_else(|| Error::new(p, "Selector child count overflow"))?;
    let keys_size = count
        .checked_mul(20)
        .ok_or_else(|| Error::new(p, "Selector key count overflow"))?;
    let names = r.relative(p, r.u32(p + 20)?, names_size)?.start;
    let keys = r.relative(p, r.u32(p + 24)?, keys_size)?.start;
    Ok(SelectorMetadata {
        name: h.name.clone(),
        source_offset: h.offset as u64,
        parameter: r.name(p, 5)?,
        default: r.name(names, 6)?,
        children: (0..count)
            .map(|i| r.name(names + 24 * i, 6))
            .collect::<Result<Vec<_>>>()?,
        values: (0..count)
            .map(|i| r.name(keys + 20 * i, 5))
            .collect::<Result<Vec<_>>>()?,
    })
}
