//! Decoded stock samples. Playback/interpolation and reference composition are
//! separate core operations; this loader never repairs missing bone channels.
use crate::animation_metadata::AnimationMetadata;
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};
mod native;

/// Scale xyz, quaternion xyzw, translation xyz; each component is an f32 word.
pub type SampleWords = [u32; 10];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipFrames {
    pub name: String,
    pub source_offset: u64,
    pub fps_bits: u32,
    pub loop_translation_bits: [u32; 3],
    pub loop_rotation_bits: [u32; 4],
    pub channel_animation: bool,
    pub channel_weights: Vec<u32>,
    pub frames: Vec<Vec<SampleWords>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferencePose {
    pub name: String,
    pub source_offset: u64,
    pub samples: Vec<SampleWords>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    version: u32,
    source_sha256: String,
    decoder_sha256: String,
    decoder_hardware_verified: bool,
    parents: Vec<i32>,
    bone_names: Vec<String>,
    mirror_indices: Vec<i32>,
    has_trajectory: bool,
    clips: Vec<ClipFrames>,
    reference_poses: Vec<ReferencePose>,
}

pub struct AnimationFrames {
    pub source_sha256: String,
    pub decoder_sha256: String,
    pub decoder_hardware_verified: bool,
    pub parents: Vec<i32>,
    pub bone_names: Vec<String>,
    pub mirror_indices: Vec<i32>,
    pub has_trajectory: bool,
    clips: BTreeMap<String, ClipFrames>,
    reference_poses: BTreeMap<u64, ReferencePose>,
    native: Option<native::NativeFrames>,
}

impl AnimationFrames {
    pub fn from_banks(banks: &crate::animation_banks::AnimationBanks) -> Result<Self, String> {
        let hierarchy = banks.banks[0]
            .hierarchy()
            .ok_or("Missing animation hierarchy")?;
        Ok(Self {
            source_sha256: banks.identities[0].clone(),
            decoder_sha256: String::new(),
            decoder_hardware_verified: false,
            parents: hierarchy.parents.clone(),
            bone_names: hierarchy.bone_names.clone(),
            mirror_indices: hierarchy.mirrors.clone(),
            has_trajectory: hierarchy.has_trajectory,
            clips: BTreeMap::new(),
            reference_poses: BTreeMap::new(),
            native: Some(native::NativeFrames::new(banks)?),
        })
    }
    pub fn load(path: &Path, metadata: &AnimationMetadata) -> Result<Self, String> {
        let text = crate::asset_io::read_to_string(path)
            .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
        Self::parse(&text, metadata).map_err(|error| format!("{}: {error}", path.display()))
    }

    pub fn parse(text: &str, metadata: &AnimationMetadata) -> Result<Self, String> {
        let file: File = serde_json::from_str(text).map_err(|error| error.to_string())?;
        if file.version != 3 || file.source_sha256 != metadata.source_sha256 {
            return Err(
                "Animation sample format or stock bank identity differs from metadata".into(),
            );
        }
        if file.decoder_sha256.len() != 64
            || !file.decoder_sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("Animation sample decoder identity is invalid".into());
        }
        let bones = file.parents.len();
        if bones == 0 || bones > 255 {
            return Err("Animation sample hierarchy has invalid bone count".into());
        }
        for (index, &parent) in file.parents.iter().enumerate() {
            if parent < -1 || parent >= index as i32 {
                return Err(format!("Animation bone{index} has invalid parent{parent}"));
            }
        }
        if file.bone_names.len() != bones || file.mirror_indices.len() != bones {
            return Err("Animation hierarchy names/mirrors are incomplete".into());
        }
        let mut names = std::collections::BTreeSet::new();
        for (i, (&partner, name)) in file.mirror_indices.iter().zip(&file.bone_names).enumerate() {
            if name.is_empty()
                || !names.insert(name)
                || partner < -1
                || partner >= bones as i32
                || (partner >= 0 && file.mirror_indices[partner as usize] != i as i32)
            {
                return Err(format!(
                    "Animation bone{i} has an invalid name/mirror mapping"
                ));
            }
        }
        let mut clips = BTreeMap::new();
        for clip in file.clips {
            let source = metadata.clip(&clip.name)?;
            if source.source_offset != clip.source_offset
                || source.fps_bits != clip.fps_bits
                || f32::from_bits(source.frames_bits) != clip.frames.len() as f32
                || clip.channel_animation != (source.flags_word & (1 << 29) != 0)
            {
                return Err(format!(
                    "{}: decoded samples disagree with stock clip header",
                    clip.name
                ));
            }
            if clip.channel_weights.len() != bones
                || clip
                    .channel_weights
                    .iter()
                    .any(|&word| !f32::from_bits(word).is_finite())
                || clip
                    .loop_translation_bits
                    .iter()
                    .chain(&clip.loop_rotation_bits)
                    .any(|&word| !f32::from_bits(word).is_finite())
            {
                return Err(format!(
                    "{}: invalid loop transform or missing channel weights",
                    clip.name
                ));
            }
            for frame in &clip.frames {
                validate_samples(frame, bones)?;
            }
            let name = clip.name.clone();
            if clips.insert(name.clone(), clip).is_some() {
                return Err(format!("Duplicate decoded clip{name}"));
            }
        }
        let mut reference_poses = BTreeMap::new();
        for pose in file.reference_poses {
            validate_samples(&pose.samples, bones)?;
            if !matches!(
                pose.name.as_str(),
                "RIG_TPOSE" | "BOARD_BACKWARDS" | "BOARD_BACKWARDS_IK"
            ) || reference_poses.insert(pose.source_offset, pose).is_some()
            {
                return Err("Invalid or duplicated reference-pose source record".into());
            }
        }
        Ok(Self {
            source_sha256: file.source_sha256,
            decoder_sha256: file.decoder_sha256,
            decoder_hardware_verified: file.decoder_hardware_verified,
            parents: file.parents,
            bone_names: file.bone_names,
            mirror_indices: file.mirror_indices,
            has_trajectory: file.has_trajectory,
            clips,
            reference_poses,
            native: None,
        })
    }

    pub fn clip(&self, name: &str) -> Result<&ClipFrames, String> {
        if let Some(native) = &self.native {
            return native.clip(name);
        }
        self.clips
            .get(&name.to_ascii_uppercase())
            .ok_or_else(|| format!("Required decoded stock clip is absent: {name}"))
    }

    /// Explicit source-record lookup, useful when auditing original bank data.
    pub fn reference_pose(&self, source_offset: u64) -> Result<&ReferencePose, String> {
        if let Some(native) = &self.native {
            return native.reference_pose(0, source_offset);
        }
        self.reference_poses
            .get(&source_offset)
            .ok_or_else(|| format!("Required reference-pose record{source_offset:#x} is absent"))
    }

    /// SetDBContent82D1B45C assigns into the selected database's name map,
    /// replacing earlier same-name records in that bank.
    pub fn named_pose(&self, name: &str) -> Result<&ReferencePose, String> {
        self.named_pose_in(0, name)
    }
    pub fn named_pose_in(&self, bank: usize, name: &str) -> Result<&ReferencePose, String> {
        if let Some(native) = &self.native {
            return native.named_pose(bank, name);
        }
        if bank != 0 {
            return Err("Converted animation data contains only one bank".into());
        }
        self.reference_poses
            .values()
            .rev()
            .find(|pose| pose.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("Required stock reference pose is absent: {name}"))
    }

    pub fn clip_count(&self) -> usize {
        if let Some(native) = &self.native {
            return native.clip_count();
        }
        self.clips.len()
    }

    pub fn clip_names(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        if let Some(native) = &self.native {
            return Box::new(native.clip_names());
        }
        Box::new(self.clips.keys().map(String::as_str))
    }
    pub fn clip_bank(&self, name: &str) -> Result<usize, String> {
        if let Some(native) = &self.native {
            return native.clip_bank(name);
        }
        self.clip(name).map(|_| 0)
    }
}

fn validate_samples(samples: &[SampleWords], bones: usize) -> Result<(), String> {
    if samples.len() != bones
        || samples
            .iter()
            .flatten()
            .any(|&word| !f32::from_bits(word).is_finite())
    {
        return Err("Decoded stock frame is incomplete or nonfinite".into());
    }
    Ok(())
}
