//! Bank-backed lazy animation samples. Each requested clip is decoded once;
//! registration indexes every record without allocating every decoded frame.
use super::{ClipFrames, ReferencePose, SampleWords, validate_samples};
use crate::{
    abin::{Bank, Codec, PartEntry, Reader, RecordData},
    animation_banks::AnimationBanks,
};
use skate_core::animation::vbr::VbrDecoder;
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

struct Entry<T> {
    bank: usize,
    offset: usize,
    decoded: OnceLock<Result<T, String>>,
}
impl<T> Entry<T> {
    fn new(bank: usize, offset: usize) -> Self {
        Self {
            bank,
            offset,
            decoded: OnceLock::new(),
        }
    }
}
pub(super) struct NativeFrames {
    banks: Vec<Arc<Bank>>,
    clips: BTreeMap<String, Entry<ClipFrames>>,
    poses: BTreeMap<(usize, u64), Entry<ReferencePose>>,
    pose_names: BTreeMap<(usize, String), u64>,
}
impl NativeFrames {
    pub(super) fn new(banks: &AnimationBanks) -> Result<Self, String> {
        let mut clips: BTreeMap<String, Entry<ClipFrames>> = BTreeMap::new();
        let mut poses = BTreeMap::new();
        let mut pose_names = BTreeMap::new();
        for (bank_index, bank) in banks.banks.iter().enumerate() {
            for record in bank.records() {
                let name = &record.header.name;
                let at = record.header.offset;
                match &record.data {
                    RecordData::Clip(_) => {
                        if clips
                            .get(name)
                            .is_some_and(|entry| entry.bank != bank_index)
                        {
                            return Err(format!(
                                "Ambiguous clip {name} in multiple animation banks"
                            ));
                        }
                        clips.insert(name.clone(), Entry::new(bank_index, at));
                    }
                    RecordData::Pose(_) => {
                        poses.insert((bank_index, at as u64), Entry::new(bank_index, at));
                        pose_names.insert((bank_index, name.clone()), at as u64);
                    }
                    _ => {}
                }
            }
        }
        Ok(Self {
            banks: banks.banks.clone(),
            clips,
            poses,
            pose_names,
        })
    }
    pub(super) fn clip_count(&self) -> usize {
        self.clips.len()
    }
    pub(super) fn clip_names(&self) -> impl Iterator<Item = &str> {
        self.clips.keys().map(String::as_str)
    }
    pub(super) fn clip_bank(&self, name: &str) -> Result<usize, String> {
        self.clips
            .get(&name.to_ascii_uppercase())
            .map(|e| e.bank)
            .ok_or_else(|| format!("Missing stock animation clip {name}"))
    }
    pub(super) fn clip(&self, name: &str) -> Result<&ClipFrames, String> {
        let entry = self
            .clips
            .get(&name.to_ascii_uppercase())
            .ok_or_else(|| format!("Missing stock animation clip {name}"))?;
        entry
            .decoded
            .get_or_init(|| decode_clip(&self.banks[entry.bank], entry.offset))
            .as_ref()
            .map_err(Clone::clone)
    }
    pub(super) fn named_pose(&self, bank: usize, name: &str) -> Result<&ReferencePose, String> {
        let at = self
            .pose_names
            .get(&(bank, name.to_ascii_uppercase()))
            .ok_or_else(|| format!("Missing reference pose {name} in bank {bank}"))?;
        self.reference_pose(bank, *at)
    }
    pub(super) fn reference_pose(
        &self,
        bank: usize,
        offset: u64,
    ) -> Result<&ReferencePose, String> {
        let entry = self
            .poses
            .get(&(bank, offset))
            .ok_or_else(|| format!("Missing reference pose {bank}:{offset:#x}"))?;
        entry
            .decoded
            .get_or_init(|| decode_pose(&self.banks[bank], entry.offset))
            .as_ref()
            .map_err(Clone::clone)
    }
}

fn decode_clip(bank: &Bank, offset: usize) -> Result<ClipFrames, String> {
    let record = bank.record_at(offset).ok_or("Missing clip source record")?;
    let RecordData::Clip(clip) = &record.data else {
        return Err("Source record is not a clip".into());
    };
    require_vbr(record.header.codec())?;
    let count = f32::from_bits(clip.frame_count_bits);
    if !count.is_finite() || count < 1.0 || count.fract() != 0.0 {
        return Err(format!("{}: invalid frame count", record.header.name));
    }
    let frames = decode_parts(bank, &clip.parts, count as usize)
        .map_err(|e| format!("{}: {e}", record.header.name))?;
    let bones = bank.hierarchy().ok_or("No bank hierarchy")?.bone_count as usize;
    let mut weights = vec![1.0f32.to_bits(); bones];
    if clip.channel_animation() {
        let reader = Reader::new(bank.bytes());
        for (entry, layout) in clip.parts.iter().zip(&bank.hierarchy().unwrap().parts) {
            let part = entry
                .part
                .as_ref()
                .ok_or("Channel animation part is absent")?;
            let range = part
                .channel_weights
                .as_ref()
                .ok_or("Channel animation weights are absent")?;
            for lane in 0..layout.bone_count as usize {
                let word = reader
                    .u32(range.start + lane * 4)
                    .map_err(|e| e.to_string())?;
                if !f32::from_bits(word).is_finite() {
                    return Err("Nonfinite channel weight".into());
                }
                weights[layout.sqt_offset as usize + lane] = word;
            }
        }
    }
    Ok(ClipFrames {
        name: record.header.name.clone(),
        source_offset: offset as u64,
        fps_bits: clip.fps_bits,
        loop_translation_bits: clip.loop_translation_words[..3].try_into().unwrap(),
        loop_rotation_bits: clip.loop_rotation_words,
        channel_animation: clip.channel_animation(),
        channel_weights: weights,
        frames,
    })
}
fn decode_pose(bank: &Bank, offset: usize) -> Result<ReferencePose, String> {
    let record = bank.record_at(offset).ok_or("Missing pose source record")?;
    let RecordData::Pose(pose) = &record.data else {
        return Err("Source record is not a pose".into());
    };
    require_vbr(record.header.codec())?;
    let mut frames =
        decode_parts(bank, &pose.parts, 1).map_err(|e| format!("{}: {e}", record.header.name))?;
    Ok(ReferencePose {
        name: record.header.name.clone(),
        source_offset: offset as u64,
        samples: frames.remove(0),
    })
}
fn require_vbr(codec: Codec) -> Result<(), String> {
    if codec != Codec::Vbr {
        return Err(format!("Unsupported native animation codec {codec:?}"));
    }
    Ok(())
}
fn decode_parts(
    bank: &Bank,
    parts: &[PartEntry],
    count: usize,
) -> Result<Vec<Vec<SampleWords>>, String> {
    let hierarchy = bank.hierarchy().ok_or("Missing stock hierarchy")?;
    let bones = hierarchy.bone_count as usize;
    if parts.len() != hierarchy.parts.len() {
        return Err("Animation part count differs from hierarchy".into());
    }
    let mut frames = vec![vec![[0; 10]; bones]; count];
    let mut assigned = vec![false; bones];
    for (entry, layout) in parts.iter().zip(&hierarchy.parts) {
        let part = entry
            .part
            .as_ref()
            .ok_or("Required stock animation part is absent")?;
        let mut decoder = VbrDecoder::new(
            bank.bytes(),
            part.offset,
            part.compression_header_relative as usize,
            part.compressed_data_relative as usize,
        )?;
        if decoder.frame_count() != count || decoder.channel_count() != layout.bone_count as usize {
            return Err(format!(
                "{}: codec frame/channel count disagrees with bank header",
                layout.name
            ));
        }
        let start =
            usize::try_from(layout.sqt_offset).map_err(|_| "Negative animation part offset")?;
        let end = start + layout.bone_count as usize;
        if end > bones || assigned[start..end].iter().any(|b| *b) {
            return Err("Overlapping animation parts".into());
        }
        assigned[start..end].fill(true);
        for (index, frame) in frames.iter_mut().enumerate() {
            frame[start..end].copy_from_slice(&decoder.decode_frame(index)?);
        }
    }
    if assigned.iter().any(|v| !*v) {
        return Err("Incomplete animation hierarchy channels".into());
    }
    for frame in &frames {
        validate_samples(frame, bones)?;
    }
    Ok(frames)
}
