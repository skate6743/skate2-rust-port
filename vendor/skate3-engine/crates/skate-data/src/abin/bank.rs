//! Ordered bank indexing and lossless header decoding. Codec math is separate.
use super::*;
use std::{collections::BTreeMap, path::Path, sync::Arc};

pub struct Bank {
    bytes: Arc<[u8]>,
    pub wrapper_bytes: usize,
    pub trailer: Range<usize>,
    records: Vec<Record>,
    offsets: BTreeMap<usize, usize>,
    animations: BTreeMap<String, usize>,
    clips: BTreeMap<String, usize>,
    poses: BTreeMap<String, usize>,
    hierarchy: Option<usize>,
}
impl Bank {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = crate::asset_io::read(path)
            .map_err(|e| Error::new(0, format!("{}: {e}", path.display())))?;
        Self::parse(bytes)
    }
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        let reader = Reader::new(&bytes);
        let end = reader.u32(0)? as usize;
        if reader.u32(4)? != 1 || end < 48 || end > bytes.len() {
            return Err(Error::new(0, "invalid type1 bank wrapper"));
        }
        let mut records = Vec::new();
        let mut offsets = BTreeMap::new();
        let mut animations = BTreeMap::new();
        let mut clips = BTreeMap::new();
        let mut poses = BTreeMap::new();
        let mut hierarchy = None;
        let mut at = 48;
        while at < end {
            let size = reader.u32(at)? as usize;
            if size < 48 || size % 16 != 0 || size > end - at {
                return Err(Error::new(at, "invalid record size/alignment"));
            }
            let r = reader.bounded(at..at + size)?;
            let header = RecordHeader {
                name: r.name(at + 16, 6)?,
                name_words: r.words(at + 16)?,
                offset: at,
                size,
                payload_offset: (at + 55) & !15,
                type_id: r.u32(at + 4)?,
                codec_word: r.u32(at + 8)?,
                guid: r.u32(at + 12)?,
            };
            let data = match header.type_id {
                2 => RecordData::Clip(read_clip(r, &header)?),
                3 => RecordData::Pose(Pose {
                    parts: read_parts(
                        r,
                        header.payload_offset,
                        4,
                        r.u32(header.payload_offset)? as usize,
                        false,
                    )?,
                }),
                4 => RecordData::Hierarchy(read_hierarchy(r, header.payload_offset)?),
                5 => {
                    let n = r.u32(header.payload_offset)?;
                    let size = (n as usize)
                        .checked_mul(112)
                        .ok_or_else(|| Error::new(at, "physics bone size overflow"))?;
                    RecordData::PhysicsPose(PhysicsPose {
                        bone_count: n,
                        records: r.relative(header.payload_offset, 16, size)?,
                    })
                }
                _ => RecordData::Opaque,
            };
            let index = records.len();
            // SetDBContent82D1B428..45C overwrites map entries in file order.
            // Clips also participate in the animation-tree map, poses do not.
            match header.type_id {
                2 => {
                    clips.insert(header.name.clone(), index);
                    animations.insert(header.name.clone(), index);
                }
                3 => {
                    poses.insert(header.name.clone(), index);
                }
                4 => {
                    hierarchy = Some(index);
                }
                6..=11 => {
                    animations.insert(header.name.clone(), index);
                }
                _ => {}
            }
            offsets.insert(at, index);
            records.push(Record { header, data });
            at += size;
        }
        let file_size = bytes.len();
        Ok(Self {
            bytes: bytes.into(),
            wrapper_bytes: end,
            trailer: end..file_size,
            records,
            offsets,
            animations,
            clips,
            poses,
            hierarchy,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn records(&self) -> &[Record] {
        &self.records
    }
    pub fn record_at(&self, offset: usize) -> Option<&Record> {
        self.offsets.get(&offset).map(|&i| &self.records[i])
    }
    pub fn animation(&self, name: &str) -> Option<&Record> {
        self.animations
            .get(&name.to_ascii_uppercase())
            .map(|&i| &self.records[i])
    }
    pub fn clip(&self, name: &str) -> Option<(&RecordHeader, &Clip)> {
        let record = &self.records[*self.clips.get(&name.to_ascii_uppercase())?];
        if let RecordData::Clip(clip) = &record.data {
            Some((&record.header, clip))
        } else {
            None
        }
    }
    pub fn pose(&self, name: &str) -> Option<(&RecordHeader, &Pose)> {
        let record = &self.records[*self.poses.get(&name.to_ascii_uppercase())?];
        if let RecordData::Pose(pose) = &record.data {
            Some((&record.header, pose))
        } else {
            None
        }
    }
    pub fn hierarchy(&self) -> Option<&Hierarchy> {
        match &self.records[self.hierarchy?].data {
            RecordData::Hierarchy(v) => Some(v),
            _ => None,
        }
    }
    ///The stock OnBoard/OffBoard VBR banks carry all eight parts in hierarchy
    ///order. Their encoded ids are all zero; source82D20608/82D20924 advances
    ///its equal-id cursor one entry at a time, preserving this ordinal mapping.
    ///Call before using positional part association; partial banks may require
    ///the original named/id association and are not silently treated as full.
    pub fn validate_positional_parts(&self) -> Result<()> {
        let hierarchy = self
            .hierarchy()
            .ok_or_else(|| Error::new(0, "missing hierarchy"))?;
        for record in &self.records {
            let parts = match &record.data {
                RecordData::Clip(v) => &v.parts,
                RecordData::Pose(v) => &v.parts,
                _ => continue,
            };
            if parts.len() != hierarchy.parts.len() {
                return Err(Error::new(
                    record.header.offset,
                    "part table differs from full hierarchy",
                ));
            }
            for (entry, layout) in parts.iter().zip(&hierarchy.parts) {
                if let Some(part) = &entry.part {
                    if part.channel_count as u32 != layout.bone_count {
                        return Err(Error::new(
                            part.offset,
                            "part channel count differs from hierarchy",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

fn read_clip(r: Reader<'_>, h: &RecordHeader) -> Result<Clip> {
    let p = h.payload_offset;
    r.take(p, 56)?;
    let flags = r.u32(p + 52)?;
    let attribute_count = flags as u8;
    let attribute_offset = r.relative(p, r.u32(p + 44)?, 0)?.start;
    Ok(Clip {
        fps_bits: r.u32(p + 32)?,
        frame_count_bits: r.u32(p + 36)?,
        base_speed_bits: r.u32(p + 40)?,
        flags,
        loop_translation_words: r.words(p)?,
        loop_rotation_words: r.words(p + 16)?,
        attribute_offset,
        attribute_count,
        parts: read_parts(
            r,
            p,
            56,
            ((flags >> 8) & 31) as usize,
            flags & 0x2000_0000 != 0,
        )?,
    })
}
fn read_parts(
    r: Reader<'_>,
    base: usize,
    table: usize,
    count: usize,
    channel: bool,
) -> Result<Vec<PartEntry>> {
    let size = count
        .checked_mul(4)
        .ok_or_else(|| Error::new(base, "part table size overflow"))?;
    r.take(base + table, size)?;
    let mut parts = Vec::with_capacity(count);
    for i in 0..count {
        let table_offset = base + table + 4 * i;
        let word = r.u32(table_offset)?;
        let relative = word & 0x00ff_ffff;
        let part = if relative == 0 {
            None
        } else {
            let offset = r.relative(base, relative, 16)?.start;
            let header_word = r.u32(offset)?;
            let compressed_size = r.u32(offset + 4)?;
            let compression_header_relative = r.u32(offset + 8)?;
            let compressed_data_relative = r.u32(offset + 12)?;
            let channel_count = (((header_word >> 6) & 63) + 1) as u16;
            let compression_header = r.relative(
                offset,
                compression_header_relative,
                (header_word >> 12) as usize,
            )?;
            let compressed_data =
                r.relative(offset, compressed_data_relative, compressed_size as usize)?;
            let channel_weights = if channel {
                Some(r.relative(offset, 16, 4 * channel_count as usize)?)
            } else {
                None
            };
            Some(AnimationPart {
                offset,
                header_word,
                compressed_size,
                compression_header_relative,
                compressed_data_relative,
                channel_count,
                compression_header,
                compressed_data,
                channel_weights,
            })
        };
        parts.push(PartEntry {
            table_index: i,
            table_offset,
            raw_word: word,
            encoded_id: (word >> 24) as u8,
            part,
        });
    }
    Ok(parts)
}
fn read_hierarchy(r: Reader<'_>, p: usize) -> Result<Hierarchy> {
    let bone_count = r.u16(p)?;
    let n = bone_count as usize;
    let count = r.u32(p + 4)? as usize;
    let part_bytes = count
        .checked_mul(36)
        .ok_or_else(|| Error::new(p, "hierarchy part count overflow"))?;
    let part_base = p + 8 + 8 * n;
    r.take(p + 8, 8 * n)?;
    r.take(part_base, part_bytes)?;
    let name_base = part_base + part_bytes;
    r.take(name_base, 20 * n)?;
    let mut parents = Vec::with_capacity(n);
    let mut mirrors = Vec::with_capacity(n);
    let mut bone_names = Vec::with_capacity(n);
    for i in 0..n {
        let parent = r.i32(p + 8 + 4 * i)?;
        let mirror = r.i32(p + 8 + 4 * n + 4 * i)?;
        if parent < -1 || parent >= i as i32 || mirror < -1 || mirror >= n as i32 {
            return Err(Error::new(p + 8 + 4 * i, "invalid parent or mirror index"));
        }
        parents.push(parent);
        mirrors.push(mirror);
        bone_names.push(r.name(name_base + 20 * i, 5)?);
    }
    let mut parts = Vec::with_capacity(count);
    for i in 0..count {
        let at = part_base + i * 36;
        let bone_count = r.u32(at + 24)?;
        let sqt_offset = r.i32(at + 28)?;
        if sqt_offset < 0 || sqt_offset as usize + bone_count as usize > n {
            return Err(Error::new(at, "part extends outside skeleton output"));
        }
        parts.push(HierarchyPart {
            name: r.name(at, 6)?,
            bone_count,
            sqt_offset,
            flags: r.u16(at + 32)?,
            encoded_id: r.u16(at + 34)?,
        });
    }
    Ok(Hierarchy {
        offset: p,
        bone_count,
        has_trajectory: r.u16(p + 2)? != 0,
        parents,
        mirrors,
        bone_names,
        parts,
    })
}
