//! Andale animation database records and serialized hierarchy data.
//!
//! The hierarchy format is reconstructed from the symbolized
//! `Andale::DataBase::SetDBContent`, `HierarchyData::GetBone`, and
//! `HierarchyData::GetPartIndex` routines. Andale stores names as chains of
//! six-character base-38 words rather than as text.

use std::{fmt, ops::Range};

#[path = "andale_raw.rs"]
mod raw;
pub use raw::AndaleRawError;

#[path = "andale_vbr_numeric.rs"]
mod vbr_numeric;

#[cfg(test)]
#[path = "andale_vbr_zero_count_tests.rs"]
mod vbr_zero_count_tests;

const DATA_HEADER_SIZE: usize = 40;
const RECORD_ALIGNMENT: usize = 16;
const FILE_HEADER_TYPE: u32 = 1;
const CLIP_TYPE: u32 = 2;
const POSE_TYPE: u32 = 3;
const HIERARCHY_TYPE: u32 = 4;
const PHYSICS_PARAMETERS_TYPE: u32 = 5;
const BLEND_SPACE_TYPE: u32 = 6;
const PHASE_BLEND_TYPE: u32 = 7;
const CONSTRUCTION_SELECTOR_TYPE: u32 = 8;
const SEQUENCE_TYPE: u32 = 9;
const SELECTOR_TYPE: u32 = 10;
const RAW_DISCRETIZE_CODEC: u32 = 0x0000_4452;
const VBR_CODEC: u32 = 0x0052_4256;
const HIERARCHY_PART_SIZE: usize = 36;
const FAST_STRING_RADIX: u32 = 38;
const FAST_STRING_WORD_CHARS: usize = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndaleDataHeader {
    pub size: u32,
    pub data_type: u32,
    pub codec: u32,
    pub guid: u32,
    pub name: PackedFastString<6>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndaleRecord {
    pub offset: usize,
    pub header: AndaleDataHeader,
    pub data_range: Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndaleDatabase {
    pub file_header: AndaleDataHeader,
    pub records: Vec<AndaleRecord>,
    pub hierarchy: AndaleHierarchy,
    pub clips: Vec<AndaleClip>,
    pub poses: Vec<AndalePose>,
    pub physics_parameters: Vec<AndalePhysicsParameters>,
    pub controllers: Vec<AndaleAnimationController>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndalePhysicsParameters {
    pub record_offset: usize,
    pub name: String,
    pub guid: u32,
    pub bones: Vec<AndalePhysicsBone>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndalePhysicsBone {
    pub parent_orientation: [f32; 4],
    pub joint_orientation: [f32; 4],
    pub size: [f32; 4],
    pub orientation: [f32; 4],
    pub position: [f32; 4],
    pub twist: f32,
    pub swing: f32,
    pub mass: f32,
    pub name: String,
    /// Original five-word identity used by `PhysicsParamData::GetBone`
    /// (`0x82b56620`) and `SkeletonData::Init` (`0x82b56778`).
    pub packed_name: PackedFastString<5>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AndaleAnimationController {
    BlendSpace(AndaleBlendSpace),
    PhaseBlend(AndalePhaseBlend),
    ConstructionSelector(AndaleConstructionSelector),
    Sequence(AndaleSequence),
    Selector(AndaleSelector),
}

impl AndaleAnimationController {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::BlendSpace(value) => &value.name,
            Self::PhaseBlend(value) => &value.name,
            Self::ConstructionSelector(value) => &value.name,
            Self::Sequence(value) => &value.name,
            Self::Selector(value) => &value.name,
        }
    }

    #[must_use]
    pub fn child_names(&self) -> Vec<&str> {
        match self {
            Self::BlendSpace(value) => value.animation_names.iter().map(String::as_str).collect(),
            Self::PhaseBlend(value) => value.children.iter().map(String::as_str).collect(),
            Self::ConstructionSelector(value) => value
                .options
                .iter()
                .map(|option| option.child.as_str())
                .collect(),
            Self::Sequence(value) => value
                .children
                .iter()
                .map(|child| child.name.as_str())
                .collect(),
            Self::Selector(value) => value
                .children
                .iter()
                .map(|child| child.name.as_str())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndaleBlendSpace {
    pub record_offset: usize,
    pub name: String,
    pub guid: u32,
    pub maximum_dimensions: u32,
    pub animation_names: Vec<String>,
    pub volume_sets: Vec<AndaleBlendSpaceVolumeSet>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndaleBlendSpaceVolumeSet {
    pub name: String,
    pub dimensions: u32,
    pub dimension_names: Vec<String>,
    /// Original packed identities used by `BlendSpace.SetAttributes`.
    pub packed_dimension_names: Vec<PackedFastString<5>>,
    /// Native volume-set `mSize`, retained for animation job submission.
    pub serialized_size: u32,
    pub volumes: Vec<AndaleBlendSpaceVolume>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndaleBlendSpaceVolume {
    pub animation_indexes_offset: u32,
    pub heights_offset: u32,
    pub normals_offset: u32,
    pub matrix_coordinates: Vec<f32>,
    /// The complete serialized volume is retained while the symbol-backed
    /// meanings of its offset tables are still being reconstructed.
    pub serialized_data: Vec<u8>,
    /// Actual adjacent word from the same serialized controller payload.
    /// Native optimized normal loads can cross mSize by four bytes; None
    /// preserves an unavailable tail instead of supplying padding.
    pub serialized_following_word: Option<[u8; 4]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndaleAttribute {
    pub begin: f32,
    pub end: f32,
    pub name: String,
    pub attribute_type: u8,
    pub value: AndaleAttributeValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AndaleAttributeValue {
    Float(f32),
    Vector([f32; 4]),
    Bytes24([u8; 24]),
    Unsupported { attribute_type: u8, bytes: Vec<u8> },
}

/// Typed view of the 24-byte attribute payload consumed by skater contact
/// processing: a packed `FastString<30>` bone name followed by a scalar.
#[derive(Clone, Debug, PartialEq)]
pub struct AndaleContactAttributeValue {
    pub bone_name: String,
    pub value: f32,
}

impl AndaleAttributeValue {
    /// Decodes a type-3 value as the contact payload used by
    /// `Skeleton::ProcessAnimAttributes`.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] if the packed bone name is not canonical.
    pub fn contact_value(&self) -> Result<Option<AndaleContactAttributeValue>, AndaleError> {
        let Self::Bytes24(bytes) = self else {
            return Ok(None);
        };
        Ok(Some(AndaleContactAttributeValue {
            bone_name: parse_fast_string::<5>(bytes, 0)?.decode()?,
            value: read_be_f32(bytes, 20)?,
        }))
    }
}

pub const ANDALE_ACTIVE_ATTRIBUTE_FILTER: u8 = 7;
const ANDALE_ALL_ATTRIBUTE_FILTER: u8 = 31;
const ATTRIBUTE_STATUS_STATIC: u8 = 6;
const ATTRIBUTE_STATUS_ACTIVE: u8 = 5;
const ATTRIBUTE_STATUS_ENDED: u8 = 9;
const ATTRIBUTE_STATUS_INACTIVE: u8 = 17;

/// Runtime form emitted by `Clip::GetAttributes`. Begin/end values are in
/// seconds, `status` retains the reference transition bits, and source order
/// is preserved for the physics consumer.
#[derive(Clone, Debug, PartialEq)]
pub struct AndaleRuntimeAttribute {
    /// Native Attribute+0x38; -1 until a `SequenceNode` publishes an authored index.
    pub sequence_id: i32,
    pub begin_seconds: f32,
    pub end_seconds: f32,
    pub name: String,
    pub status: u8,
    pub value: AndaleAttributeValue,
}

impl AndaleRuntimeAttribute {
    #[must_use]
    pub const fn scalar_value(&self) -> Option<f32> {
        match &self.value {
            AndaleAttributeValue::Float(value) => Some(*value),
            AndaleAttributeValue::Vector(_)
            | AndaleAttributeValue::Bytes24(_)
            | AndaleAttributeValue::Unsupported { .. } => None,
        }
    }

    /// Decodes this runtime value when it is the reference's type-3 contact
    /// payload.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] if the packed bone name is not canonical.
    pub fn contact_value(&self) -> Result<Option<AndaleContactAttributeValue>, AndaleError> {
        self.value.contact_value()
    }
}

impl AndaleAttribute {
    #[must_use]
    pub const fn scalar_value(&self) -> Option<f32> {
        match &self.value {
            AndaleAttributeValue::Float(value) => Some(*value),
            AndaleAttributeValue::Vector(_)
            | AndaleAttributeValue::Bytes24(_)
            | AndaleAttributeValue::Unsupported { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndalePhaseBlend {
    pub record_offset: usize,
    pub name: String,
    pub guid: u32,
    pub attributes: Vec<AndaleAttribute>,
    pub blend_attribute: String,
    pub children: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndaleConstructionSelector {
    pub record_offset: usize,
    pub name: String,
    pub guid: u32,
    pub construction_attribute: String,
    pub options: Vec<AndaleConstructionOption>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndaleConstructionOption {
    pub value: String,
    pub child: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndaleLocomotionStage {
    ContactToPassRight,
    PassToAirRight,
    AirRight,
    ContactToPassLeft,
    PassToAirLeft,
    AirLeft,
    Last,
    Unknown(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndaleSequenceChild {
    pub name: String,
    pub stage: AndaleLocomotionStage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndaleSequence {
    pub record_offset: usize,
    pub name: String,
    pub guid: u32,
    pub looping: bool,
    pub window: u8,
    pub children: Vec<AndaleSequenceChild>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndaleSelectorChild {
    pub name: String,
    pub attribute: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndaleSelector {
    pub record_offset: usize,
    pub name: String,
    pub guid: u32,
    pub children: Vec<AndaleSelectorChild>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndalePhaseBlendPoseSample {
    pub transforms: Vec<AndaleSqt>,
    pub duration_seconds: f32,
    pub weighted_clips: Vec<AndaleWeightedClip>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndaleWeightedClip {
    pub name: String,
    pub weight: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct AndalePhaseSelection<'a> {
    lower: &'a str,
    upper: &'a str,
    raw_coefficient: f32,
    pose_coefficient: f32,
}

#[derive(Clone, Copy, Debug)]
struct AndaleAttributeEvalContext<'a> {
    current_time: f32,
    previous_time: f32,
    looped: bool,
    inputs: &'a [(&'a str, f32)],
    blend_cull_threshold: f32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndaleHierarchy {
    pub has_trajectory: bool,
    pub parents: Vec<i32>,
    /// The second serialized per-bone table. It is present in the exact
    /// layout used by `GetBone`, but its runtime meaning is not yet named by
    /// the available PDB type (the flexible member exposes only `mParents`).
    pub bone_metadata: Vec<i32>,
    pub parts: Vec<AndaleHierarchyPart>,
    pub bone_names: Vec<String>,
    pub packed_bone_names: Vec<PackedFastString<5>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndaleHierarchyPart {
    pub name: String,
    pub packed_name: PackedFastString<6>,
    pub bone_count: u32,
    pub bone_offset: u32,
    pub is_animated: bool,
    pub name_index: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndalePose {
    pub record_offset: usize,
    pub parts: Vec<AndaleAnimationPart>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndaleClip {
    pub record_offset: usize,
    pub name: String,
    pub guid: u32,
    pub codec: u32,
    pub fixed_words: [u32; 8],
    pub frames_per_second: f32,
    pub frame_count: f32,
    pub base_speed: f32,
    pub unknown_2c: u32,
    pub unknown_30: u32,
    pub flags: u32,
    pub attributes: Vec<AndaleAttribute>,
    pub encoded_part_offsets: Vec<u32>,
    pub parts: Vec<AndaleClipPart>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndaleClipPart {
    pub header: u32,
    pub encoded_offset: u32,
    pub part_name_index: u8,
    pub compressed_data_size: u32,
    pub compression_header_offset: u32,
    pub compressed_data_offset: u32,
    pub compression_header_size: usize,
    /// Bytes after compressed data and before the next part/record boundary.
    /// The last part may be followed by clip-local auxiliary data.
    pub trailing_bytes_size: usize,
    pub compression: AndaleCompressionHeader,
    pub compression_header: Vec<u8>,
    pub compressed_data: Vec<u8>,
    pub trailing_data: Vec<u8>,
    pub vbr_prelude: Option<AndaleVbrPrelude>,
}

impl AndaleClip {
    #[must_use]
    pub fn looping(&self) -> bool {
        self.flags & 0x1000_0000 != 0
    }

    #[must_use]
    pub fn duration_seconds(&self) -> f32 {
        (self.frame_count - 1.0) / self.frames_per_second
    }

    /// Evaluates the attribute status window recovered from
    /// `Clip::GetAttributeStatus` and materializes the attributes accepted by
    /// the caller's status filter.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] for invalid normalized times or an inconsistent
    /// loop transition.
    pub fn runtime_attributes(
        &self,
        current_normalized_time: f32,
        previous_normalized_time: f32,
        looped: bool,
        status_filter: u8,
    ) -> Result<Vec<AndaleRuntimeAttribute>, AndaleError> {
        validate_normalized_time(current_normalized_time)?;
        validate_normalized_time(previous_normalized_time)?;
        if looped && current_normalized_time >= previous_normalized_time {
            return Err(AndaleError::InvalidAnimationTimeTransition {
                previous: previous_normalized_time.to_bits(),
                current: current_normalized_time.to_bits(),
            });
        }
        let duration = self.duration_seconds();
        Ok(self
            .attributes
            .iter()
            .filter_map(|attribute| {
                runtime_attribute(
                    attribute,
                    current_normalized_time * duration,
                    previous_normalized_time * duration,
                    duration,
                    looped,
                    status_filter,
                )
            })
            .collect())
    }

    /// Samples the clip at a normalized zero-through-one time. The frame pair
    /// follows `Clip::Eval`: normalized time maps across frames zero through
    /// `frame_count - 1`.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] for a non-VBR clip, inconsistent part frame
    /// counts, invalid normalized time, or a VBR decode failure.
    pub fn sample_vbr_pose_sqt_at_normalized_time(
        &self,
        hierarchy: &AndaleHierarchy,
        normalized_time: f32,
    ) -> Result<Vec<AndaleSqt>, AndaleError> {
        if !normalized_time.is_finite() || !(0.0..=1.0).contains(&normalized_time) {
            return Err(AndaleError::InvalidNormalizedAnimationTime(
                normalized_time.to_bits(),
            ));
        }
        let mut frame_count = None;
        for part in &self.parts {
            let AndaleCompressionHeader::Vbr(header) = &part.compression else {
                return Err(AndaleError::NotVbrCompression);
            };
            if frame_count
                .replace(header.frame_count)
                .is_some_and(|expected| expected != header.frame_count)
            {
                return Err(AndaleError::ClipPartFrameCountMismatch {
                    clip: self.name.clone(),
                });
            }
        }
        let frame_count = frame_count.ok_or(AndaleError::NotVbrCompression)?;
        if frame_count == 0 {
            return Err(AndaleError::ClipPartFrameCountMismatch {
                clip: self.name.clone(),
            });
        }
        let last_frame = frame_count - 1;
        let frame_position = normalized_time * f32::from(last_frame);
        let mut frame0 = 0_u16;
        while frame0 < last_frame && f32::from(frame0 + 1) <= frame_position {
            frame0 += 1;
        }
        let frame1 = frame0.saturating_add(1).min(last_frame);
        let blend = frame_position - f32::from(frame0);
        self.sample_vbr_pose_sqt_interpolated(
            hierarchy,
            usize::from(frame0),
            usize::from(frame1),
            blend,
        )
    }

    /// Samples every VBR part into the database hierarchy's bone order.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] if clip parts do not match the hierarchy or an
    /// individual part cannot sample the requested frames.
    pub fn sample_vbr_pose_sqt_interpolated(
        &self,
        hierarchy: &AndaleHierarchy,
        frame0: usize,
        frame1: usize,
        blend: f32,
    ) -> Result<Vec<AndaleSqt>, AndaleError> {
        if self.parts.len() != hierarchy.parts.len() {
            return Err(AndaleError::ClipHierarchyPartCountMismatch {
                clip: self.parts.len(),
                hierarchy: hierarchy.parts.len(),
            });
        }
        let mut pose = vec![identity_sqt(); hierarchy.bone_names.len()];
        for (index, (clip_part, hierarchy_part)) in
            self.parts.iter().zip(&hierarchy.parts).enumerate()
        {
            let part_pose = clip_part.sample_vbr_sqt_interpolated(frame0, frame1, blend)?;
            let expected = usize::try_from(hierarchy_part.bone_count)
                .map_err(|_| AndaleError::IntegerOverflow)?;
            if part_pose.len() != expected {
                return Err(AndaleError::ClipHierarchyPartMismatch {
                    index,
                    clip_bones: part_pose.len(),
                    hierarchy_bones: expected,
                });
            }
            let start = usize::try_from(hierarchy_part.bone_offset)
                .map_err(|_| AndaleError::IntegerOverflow)?;
            let end = start
                .checked_add(part_pose.len())
                .ok_or(AndaleError::IntegerOverflow)?;
            let hierarchy_bones = hierarchy.bone_names.len();
            let target = pose
                .get_mut(start..end)
                .ok_or(AndaleError::ClipPartOutsideHierarchy {
                    index,
                    offset: start,
                    bones: part_pose.len(),
                    hierarchy_bones,
                })?;
            target.copy_from_slice(&part_pose);
        }
        Ok(pose)
    }
}

impl AndaleClipPart {
    #[must_use]
    pub fn bone_count(&self) -> usize {
        usize::try_from((self.header >> 6 & 0x3f) + 1).unwrap_or_default()
    }

    /// Samples one integer frame from a VBR-compressed animation part.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] for an unsupported channel layout, an invalid
    /// frame, or inconsistent packed channel data.
    pub fn sample_vbr_sqt(&self, frame: usize) -> Result<Vec<AndaleSqt>, AndaleError> {
        self.sample_vbr_sqt_cached(frame, &mut None)
    }

    /// Decode the two keys of a consecutive frame pair, sharing immutable block
    /// coefficients when possible. A pair across an eight-frame block decodes the
    /// second frame for both keys ([`vbr_pair_first_frame`]). The caller retains its
    /// own interpolation/normalization policy.
    ///
    /// # Errors
    /// Returns the same errors, in the same order, as two `sample_vbr_sqt` calls.
    pub fn sample_vbr_sqt_pair(
        &self,
        first: usize,
        second: usize,
    ) -> Result<[Vec<AndaleSqt>; 2], AndaleError> {
        let mut block = None;
        Ok([
            self.sample_vbr_sqt_cached(vbr_pair_first_frame(first, second), &mut block)?,
            self.sample_vbr_sqt_cached(second, &mut block)?,
        ])
    }

    #[allow(clippy::unnecessary_lazy_evaluations,
        reason = "ARM64 otherwise calls AndaleError drop glue on successful reads")]
    fn sample_vbr_sqt_cached<'a>(
        &'a self,
        frame: usize,
        block: &mut Option<VbrSqtBlock<'a>>,
    ) -> Result<Vec<AndaleSqt>, AndaleError> {
        let AndaleCompressionHeader::Vbr(header) = &self.compression else {
            return Err(AndaleError::NotVbrCompression);
        };
        let prelude = self
            .vbr_prelude
            .as_ref()
            .ok_or_else(|| AndaleError::MissingVbrPrelude)?;
        sample_vbr_sqt(
            header,
            prelude,
            &self.compressed_data,
            self.bone_count(),
            frame,
            block,
        )
    }

    /// Samples and blends two integer VBR frames using the reference path's
    /// linear scale/translation blend and shortest-path normalized quaternion
    /// blend. A pair across an eight-frame block uses the second frame for both
    /// keys ([`vbr_pair_first_frame`]).
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] for invalid frames, a blend outside zero
    /// through one, inconsistent transforms, or a zero-length quaternion.
    pub fn sample_vbr_sqt_interpolated(
        &self,
        frame0: usize,
        frame1: usize,
        blend: f32,
    ) -> Result<Vec<AndaleSqt>, AndaleError> {
        if !blend.is_finite() || !(0.0..=1.0).contains(&blend) {
            return Err(AndaleError::InvalidVbrBlend(blend.to_bits()));
        }
        // Both samples belong to this immutable clip part. Adjacent frames
        // usually read the same eight-frame block; retain its decoded input
        // only for this call, preserving frame order and all reconstruction.
        let mut block = None;
        let frame0 = self.sample_vbr_sqt_cached(vbr_pair_first_frame(frame0, frame1), &mut block)?;
        let frame1 = self.sample_vbr_sqt_cached(frame1, &mut block)?;
        if frame0.len() != frame1.len() {
            return Err(AndaleError::VbrInterpolatedFrameSizeMismatch {
                frame0: frame0.len(),
                frame1: frame1.len(),
            });
        }
        frame0
            .into_iter()
            .zip(frame1)
            .map(|(frame0, frame1)| {
                Ok(AndaleSqt {
                    scale: lerp4(frame0.scale, frame1.scale, blend),
                    rotation: normalized_quaternion_lerp(frame0.rotation, frame1.rotation, blend)?,
                    translation: lerp4(frame0.translation, frame1.translation, blend),
                })
            })
            .collect()
    }
}

/// The frame `DecompressTwoConsecFrames` decodes for the first key of a consecutive pair.
/// Across an eight-frame VBR block (7, 8) it reads the second frame's block and in-block
/// index for both keys (82edc958..9b4, indices used at 82edce40..58), and
/// `Vector_ExtractPackedNVBR` 82d71564..78 selects that same block, so the pose holds the
/// second frame for the whole interval.
#[must_use]
pub fn vbr_pair_first_frame(first: usize, second: usize) -> usize {
    // SKATE2_DIAGNOSTIC_VBR_PAIR=linear decodes both keys as named (A/B tests).
    static LINEAR: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let linear = *LINEAR.get_or_init(|| std::env::var("SKATE2_DIAGNOSTIC_VBR_PAIR").is_ok_and(|v| v == "linear"));
    if !linear && first / 8 != second / 8 && (first % 8 == 7 || second % 8 == 0) {
        second
    } else {
        first
    }
}

fn lerp4(from: [f32; 4], to: [f32; 4], blend: f32) -> [f32; 4] {
    std::array::from_fn(|index| from[index] + (to[index] - from[index]) * blend)
}

fn normalized_quaternion_lerp(
    from: [f32; 4],
    mut to: [f32; 4],
    blend: f32,
) -> Result<[f32; 4], AndaleError> {
    let dot = from.iter().zip(to).map(|(from, to)| from * to).sum::<f32>();
    if dot < 0.0 {
        to = to.map(|component| -component);
    }
    let blended = lerp4(from, to, blend);
    let length_squared = blended
        .iter()
        .map(|component| component * component)
        .sum::<f32>();
    if !length_squared.is_normal() {
        return Err(AndaleError::ZeroLengthVbrQuaternion);
    }
    let reciprocal_length = length_squared.sqrt().recip();
    Ok(blended.map(|component| component * reciprocal_length))
}

fn blend_sqt(from: AndaleSqt, to: AndaleSqt, blend: f32) -> Result<AndaleSqt, AndaleError> {
    Ok(AndaleSqt {
        scale: lerp4(from.scale, to.scale, blend),
        rotation: normalized_quaternion_lerp(from.rotation, to.rotation, blend)?,
        translation: lerp4(from.translation, to.translation, blend),
    })
}

fn combine_weighted_clips(
    lower: Vec<AndaleWeightedClip>,
    upper: Vec<AndaleWeightedClip>,
    blend: f32,
) -> Vec<AndaleWeightedClip> {
    let mut combined = Vec::<AndaleWeightedClip>::new();
    for (clips, multiplier) in [(lower, 1.0 - blend), (upper, blend)] {
        for clip in clips {
            let weight = clip.weight * multiplier;
            if weight.to_bits().trailing_zeros() >= 31 {
                continue;
            }
            if let Some(existing) = combined.iter_mut().find(|value| value.name == clip.name) {
                existing.weight += weight;
            } else {
                combined.push(AndaleWeightedClip {
                    name: clip.name,
                    weight,
                });
            }
        }
    }
    combined
}

fn phase_pose_coefficient(
    raw_coefficient: f32,
    blend_cull_threshold: f32,
    lower_is_first: bool,
    upper_is_last: bool,
) -> f32 {
    if raw_coefficient <= blend_cull_threshold {
        return 0.0;
    }
    if raw_coefficient >= 1.0 - blend_cull_threshold {
        return 1.0;
    }
    let culled = raw_coefficient.clamp(blend_cull_threshold, 1.0 - blend_cull_threshold);
    let remapped = (culled - blend_cull_threshold) / (1.0 - blend_cull_threshold * 2.0);
    if remapped > 0.5 && upper_is_last {
        ((8.0 - remapped * 4.0) * remapped * remapped) - remapped * 4.0 + 1.0
    } else if remapped <= 0.5 && lower_is_first {
        -(remapped * 4.0 - 4.0) * remapped * remapped
    } else {
        raw_coefficient
    }
}

fn validate_normalized_time(time: f32) -> Result<(), AndaleError> {
    if time.is_finite() && (0.0..=1.0).contains(&time) {
        Ok(())
    } else {
        Err(AndaleError::InvalidNormalizedAnimationTime(time.to_bits()))
    }
}

fn runtime_attribute(
    attribute: &AndaleAttribute,
    current_time: f32,
    previous_time: f32,
    duration: f32,
    looped: bool,
    status_filter: u8,
) -> Option<AndaleRuntimeAttribute> {
    let is_static = attribute.begin.to_bits() == (-1.0_f32).to_bits();
    let status = if is_static {
        ATTRIBUTE_STATUS_STATIC
    } else {
        attribute_status(attribute, current_time, previous_time, duration, looped)
    };
    if !is_static && ((status_filter & 0x1c) & status == 0 || (status_filter & 3) & status == 0) {
        return None;
    }
    Some(AndaleRuntimeAttribute {
        sequence_id: -1,
        begin_seconds: if is_static {
            -1.0
        } else {
            attribute.begin * duration
        },
        end_seconds: if is_static {
            -1.0
        } else {
            attribute.end * duration
        },
        name: attribute.name.clone(),
        status,
        value: attribute.value.clone(),
    })
}

fn attribute_status(
    attribute: &AndaleAttribute,
    current_time: f32,
    previous_time: f32,
    duration: f32,
    looped: bool,
) -> u8 {
    let begin = attribute.begin * duration;
    let end = attribute.end * duration;
    let crossed_window = if looped {
        end > previous_time || begin <= current_time
    } else {
        end > previous_time && begin <= current_time
    };
    if !crossed_window {
        ATTRIBUTE_STATUS_INACTIVE
    } else if current_time < begin || current_time > end {
        ATTRIBUTE_STATUS_ENDED
    } else {
        ATTRIBUTE_STATUS_ACTIVE
    }
}

fn blend_runtime_attributes(
    lower: Vec<AndaleRuntimeAttribute>,
    upper: &[AndaleRuntimeAttribute],
    coefficient: f32,
) -> Result<Vec<AndaleRuntimeAttribute>, AndaleError> {
    let mut upper_start = 0;
    let mut blended = Vec::new();
    for mut attribute in lower {
        let Some(relative_index) = upper[upper_start..]
            .iter()
            .position(|candidate| candidate.name == attribute.name)
        else {
            continue;
        };
        upper_start += relative_index;
        blend_runtime_attribute(&mut attribute, &upper[upper_start], coefficient)?;
        blended.push(attribute);
    }
    Ok(blended)
}

fn blend_runtime_attribute(
    lower: &mut AndaleRuntimeAttribute,
    upper: &AndaleRuntimeAttribute,
    coefficient: f32,
) -> Result<(), AndaleError> {
    if lower.status & 2 == 0 {
        lower.begin_seconds = lerp(lower.begin_seconds, upper.begin_seconds, coefficient);
        lower.end_seconds = lerp(lower.end_seconds, upper.end_seconds, coefficient);
    }
    lower.value = match (&lower.value, &upper.value) {
        (AndaleAttributeValue::Float(lower), AndaleAttributeValue::Float(upper)) => {
            AndaleAttributeValue::Float(lerp(*lower, *upper, coefficient))
        }
        (AndaleAttributeValue::Vector(lower), AndaleAttributeValue::Vector(upper)) => {
            AndaleAttributeValue::Vector(std::array::from_fn(|index| {
                lerp(lower[index], upper[index], coefficient)
            }))
        }
        (AndaleAttributeValue::Bytes24(lower), AndaleAttributeValue::Bytes24(upper)) => {
            let mut value = *lower;
            let lower_scalar = f32::from_bits(u32::from_be_bytes([
                lower[20], lower[21], lower[22], lower[23],
            ]));
            let upper_scalar = f32::from_bits(u32::from_be_bytes([
                upper[20], upper[21], upper[22], upper[23],
            ]));
            value[20..24].copy_from_slice(
                &lerp(lower_scalar, upper_scalar, coefficient)
                    .to_bits()
                    .to_be_bytes(),
            );
            AndaleAttributeValue::Bytes24(value)
        }
        _ => {
            return Err(AndaleError::RuntimeAttributeTypeMismatch(
                lower.name.clone(),
            ));
        }
    };
    Ok(())
}

fn lerp(from: f32, to: f32, coefficient: f32) -> f32 {
    from * (1.0 - coefficient) + to * coefficient
}

#[derive(Clone, Debug, PartialEq)]
pub struct AndaleAnimationPart {
    /// Packed channel-count/format flags consumed by `Vector_ExtractPackedN`.
    pub header: u32,
    pub encoded_offset: u32,
    pub part_name_index: u8,
    pub compressed_data_size: u32,
    pub compression_header_offset: u32,
    pub compressed_data_offset: u32,
    pub compression: AndaleCompressionHeader,
    pub compression_header: Vec<u8>,
    pub compressed_data: Vec<u8>,
    pub padding: Vec<u8>,
    pub vbr_prelude: Option<AndaleVbrPrelude>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndaleRawCompressionHeader {
    pub channel_masks: [u32; 6],
    pub frame_stride: u16,
    pub last_bone_index: u8,
    pub format_flags: [u8; 2],
    pub padding: [u8; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub enum AndaleCompressionHeader {
    RawDiscretize(AndaleRawCompressionHeader),
    Vbr(AndaleVbrCompressionHeader),
    Unknown { codec: u32, bytes: Vec<u8> },
}

/// Symbol-backed fixed header used by the VBR decompressor for one animation
/// part. The compressed bitstream itself remains separate.
#[derive(Clone, Debug, PartialEq)]
pub struct AndaleVbrCompressionHeader {
    /// Number of alternating run lengths in the compressed constant-channel map.
    pub constant_channel_map_run_count: u16,
    pub frame_count: u16,
    pub dct_minimum: f32,
    pub dct_maximum: f32,
    pub channel_blocks: Vec<AndaleVbrChannelBlock>,
    pub constant_palette: Vec<f32>,
    /// One compressed byte count for each group of up to eight frames.
    pub frame_block_sizes: Vec<u16>,
    pub reserved: [u8; 2],
    pub alignment_padding: Vec<u8>,
    pub trailing_bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AndaleVbrChannelBlock {
    pub animated_entry_count: u16,
    pub constant_entry_count: u16,
    /// Constant palette values are expanded as
    /// `minimum + normalized * (maximum - minimum)`.
    pub minimum: f32,
    pub maximum: f32,
    /// Number of scalar components in each channel entry.
    pub component_count: u8,
    pub reserved: [u8; 3],
}

/// Values and channel classification stored before a VBR part's frame blocks.
#[derive(Clone, Debug, PartialEq)]
pub struct AndaleVbrPrelude {
    /// Dequantized constants, one flat component array per channel block.
    pub constant_values: Vec<Vec<f32>>,
    /// One flag per channel entry, in channel-block order.
    pub constant_entries: Vec<bool>,
    /// Four-byte work-table area reserved for each animated scalar channel.
    pub used_table: Vec<u8>,
    pub frame_data_offset: usize,
    /// Exact byte range for each independently compressed group of eight frames.
    pub frame_blocks: Vec<Range<usize>>,
    pub trailing_bytes: Vec<u8>,
}

/// Control prefix and coefficient payload for one independently compressed
/// group of up to eight VBR frames.
#[derive(Clone, Debug, PartialEq)]
pub struct AndaleVbrFrameBlock {
    /// Actual serialized byte, retained even when the native streams overlap.
    pub declared_significance_bit_count: u8,
    /// Selects one of the codec's 256 quantization rows for each channel type.
    pub quantization_table_indices: Vec<u8>,
    /// LSB-first flags indicating which configured, nonzero-width coefficients
    /// are present in this block.
    pub significance_bits: Vec<u8>,
    /// Packed signed coefficient values following the significance bitset.
    pub coefficient_payload: Vec<u8>,
}

/// `Vector_UnPackFrameBlockAndDecompressOneFrame82edbd18` reads the retained
/// compressed bytes directly. Runtime sampling does not need the owned copies
/// exposed by the public frame-block inspection API.
#[derive(Clone, Copy)]
struct BorrowedVbrFrameBlock<'a> {
    declared_significance_bit_count: u8,
    quantization_table_indices: &'a [u8],
    significance_bits: &'a [u8],
    coefficient_payload: &'a [u8],
}

impl AndaleVbrFrameBlock {
    fn borrowed(&self) -> BorrowedVbrFrameBlock<'_> {
        BorrowedVbrFrameBlock {
            declared_significance_bit_count: self.declared_significance_bit_count,
            quantization_table_indices: &self.quantization_table_indices,
            significance_bits: &self.significance_bits,
            coefficient_payload: &self.coefficient_payload,
        }
    }
}

/// Native scale/quaternion/translation value produced by Andale pose sampling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AndaleSqt {
    pub scale: [f32; 4],
    pub rotation: [f32; 4],
    pub translation: [f32; 4],
}

impl AndaleVbrCompressionHeader {
    /// Decodes the constant palette indices and alternating RLE channel map
    /// that precede VBR frame-block data.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] for truncated data, invalid palette indices, or
    /// a channel-map length inconsistent with the channel descriptors.
    pub fn decode_prelude(&self, bytes: &[u8]) -> Result<AndaleVbrPrelude, AndaleError> {
        let mut cursor = 0_usize;
        let mut constant_values = Vec::with_capacity(self.channel_blocks.len());
        for block in &self.channel_blocks {
            let scalar_count = usize::from(block.constant_entry_count)
                .checked_mul(usize::from(block.component_count))
                .ok_or(AndaleError::IntegerOverflow)?;
            require_range(bytes, cursor, scalar_count)?;
            let mut values = Vec::with_capacity(scalar_count);
            for &palette_index in &bytes[cursor..cursor + scalar_count] {
                let normalized = self
                    .constant_palette
                    .get(usize::from(palette_index))
                    .ok_or(AndaleError::VbrConstantPaletteIndexOutsidePalette {
                        index: palette_index,
                        palette_size: self.constant_palette.len(),
                    })?;
                values.push(block.minimum + normalized * (block.maximum - block.minimum));
            }
            cursor += scalar_count;
            constant_values.push(values);
        }

        let expected_entries = self
            .channel_blocks
            .iter()
            .try_fold(0_usize, |total, block| {
                total
                    .checked_add(usize::from(block.animated_entry_count))
                    .and_then(|total| total.checked_add(usize::from(block.constant_entry_count)))
                    .ok_or(AndaleError::IntegerOverflow)
            })?;
        let run_count = usize::from(self.constant_channel_map_run_count);
        require_range(bytes, cursor, run_count)?;
        let mut constant_entries = Vec::with_capacity(expected_entries);
        if run_count == 0 {
            constant_entries.resize(expected_entries, false);
        } else {
            let mut is_constant = false;
            for &run_length in &bytes[cursor..cursor + run_count] {
                let new_length = constant_entries
                    .len()
                    .checked_add(usize::from(run_length))
                    .ok_or(AndaleError::IntegerOverflow)?;
                constant_entries.resize(new_length, is_constant);
                is_constant = !is_constant;
            }
        }
        cursor += run_count;
        if constant_entries.len() != expected_entries {
            return Err(AndaleError::VbrConstantMapLengthMismatch {
                expected: expected_entries,
                actual: constant_entries.len(),
            });
        }

        let animated_scalar_count =
            self.channel_blocks
                .iter()
                .try_fold(0_usize, |total, block| {
                    usize::from(block.animated_entry_count)
                        .checked_mul(usize::from(block.component_count))
                        .and_then(|count| total.checked_add(count))
                        .ok_or(AndaleError::IntegerOverflow)
                })?;
        let used_table_size = animated_scalar_count
            .checked_mul(4)
            .ok_or(AndaleError::IntegerOverflow)?;
        require_range(bytes, cursor, used_table_size)?;
        let frame_data_offset = cursor
            .checked_add(used_table_size)
            .ok_or(AndaleError::IntegerOverflow)?;
        let mut frame_cursor = frame_data_offset;
        let mut frame_blocks = Vec::with_capacity(self.frame_block_sizes.len());
        for &size in &self.frame_block_sizes {
            let end = frame_cursor
                .checked_add(usize::from(size))
                .ok_or(AndaleError::IntegerOverflow)?;
            require_range(bytes, frame_cursor, usize::from(size))?;
            frame_blocks.push(frame_cursor..end);
            frame_cursor = end;
        }
        Ok(AndaleVbrPrelude {
            constant_values,
            constant_entries,
            used_table: bytes[cursor..frame_data_offset].to_vec(),
            frame_data_offset,
            frame_blocks,
            trailing_bytes: bytes[frame_cursor..].to_vec(),
        })
    }

    /// Splits one sized frame block into its quantization selectors,
    /// significance bitset, and packed coefficient payload.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] if the selected block is absent or truncated,
    /// or if its significance-bit count disagrees with the animation's packed
    /// coefficient-width table outside the source-backed zero/256 overlap.
    pub fn decode_frame_block(
        &self,
        prelude: &AndaleVbrPrelude,
        compressed_data: &[u8],
        block_index: usize,
    ) -> Result<AndaleVbrFrameBlock, AndaleError> {
        let block = self.borrow_frame_block(prelude, compressed_data, block_index)?;
        Ok(AndaleVbrFrameBlock {
            declared_significance_bit_count: block.declared_significance_bit_count,
            quantization_table_indices: block.quantization_table_indices.to_vec(),
            significance_bits: block.significance_bits.to_vec(),
            coefficient_payload: block.coefficient_payload.to_vec(),
        })
    }

    fn borrow_frame_block<'a>(
        &self,
        prelude: &AndaleVbrPrelude,
        compressed_data: &'a [u8],
        block_index: usize,
    ) -> Result<BorrowedVbrFrameBlock<'a>, AndaleError> {
        let range = prelude.frame_blocks.get(block_index).ok_or_else(||
            AndaleError::VbrFrameBlockIndexOutsideTable {
                index: block_index,
                block_count: prelude.frame_blocks.len(),
            },
        )?;
        require_range(compressed_data, range.start, range.len())?;
        let block = &compressed_data[range.clone()];
        let channel_count = self.channel_blocks.len();
        require_range(block, 0, channel_count + 1)?;

        let significance_bit_count = usize::from(block[channel_count]);
        let expected_significance_bit_count = prelude
            .used_table
            .iter()
            .map(|byte| usize::from(byte & 0x0f != 0) + usize::from(byte >> 4 != 0))
            .sum::<usize>();
        // 82edbdf8 skips the stream advance for an actual zero byte. The
        // width loop still reads flags: the observed 256-flag case overlaps
        // significance and coefficients instead of interpreting zero as256.
        let overlapping = significance_bit_count == 0 && expected_significance_bit_count == 256;
        if significance_bit_count != expected_significance_bit_count && !overlapping {
            return Err(AndaleError::VbrSignificanceBitCountMismatch {
                expected: expected_significance_bit_count,
                actual: significance_bit_count,
            });
        }

        let significance_byte_count = expected_significance_bit_count.div_ceil(8);
        let significance_start = channel_count + 1;
        require_range(block, significance_start, significance_byte_count)?;
        let coefficient_start = significance_start + significance_bit_count.div_ceil(8);
        Ok(BorrowedVbrFrameBlock {
            declared_significance_bit_count: block[channel_count],
            quantization_table_indices: &block[..channel_count],
            significance_bits: &block
                [significance_start..significance_start + significance_byte_count],
            coefficient_payload: &block[coefficient_start..],
        })
    }

    /// Expands the frame block's sign-magnitude coefficient stream into one
    /// eight-value row per animated scalar channel.
    /// This integer diagnostic view cannot retain negative zero; runtime
    /// reconstruction uses the private native float view instead.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] if either bitstream ends early or if the packed
    /// coefficient payload has a different byte size than its width table and
    /// significance flags require in an ordinary non-overlapping block.
    pub fn unpack_frame_block_coefficients(
        &self,
        prelude: &AndaleVbrPrelude,
        block: &AndaleVbrFrameBlock,
    ) -> Result<Vec<[i16; 8]>, AndaleError> {
        Self::unpack_coefficient_rows(prelude, block.borrowed(), |positive, magnitude| {
            if positive { magnitude } else { -magnitude }
        })
    }

    // 82edbf98/bfac converts magnitude then ORs the sign. Preserve negative
    // zero in the actual runtime path; the public integer view is diagnostic.
    fn unpack_float_coefficient_rows(
        prelude: &AndaleVbrPrelude,
        block: BorrowedVbrFrameBlock<'_>,
    ) -> Result<Vec<[f32; 8]>, AndaleError> {
        Self::unpack_coefficient_rows(prelude, block, |positive, magnitude| {
            let magnitude = f32::from(magnitude);
            if positive { magnitude } else { -magnitude }
        })
    }

    fn unpack_coefficient_rows<T: Copy + Default>(
        prelude: &AndaleVbrPrelude,
        block: BorrowedVbrFrameBlock<'_>,
        signed: impl Fn(bool, i16) -> T,
    ) -> Result<Vec<[T; 8]>, AndaleError> {
        if prelude.used_table.len() % 4 != 0 {
            return Err(AndaleError::VbrCoefficientWidthTableMisaligned(
                prelude.used_table.len(),
            ));
        }

        let mut significance_offset = 0_usize;
        let mut coefficient_offset = 0_usize;
        let mut rows = Vec::with_capacity(prelude.used_table.len() / 4);
        for packed_widths in prelude.used_table.chunks_exact(4) {
            let mut widths = u32::from_be_bytes([
                packed_widths[0],
                packed_widths[1],
                packed_widths[2],
                packed_widths[3],
            ]);
            let mut row = [T::default(); 8];
            for coefficient in &mut row {
                let width =
                    usize::try_from(widths & 0x0f).map_err(|_| AndaleError::IntegerOverflow)?;
                widths >>= 4;
                if width == 0 {
                    continue;
                }
                let present =
                    read_lsb_bits(block.significance_bits, &mut significance_offset, 1)? != 0;
                if !present {
                    continue;
                }
                let positive =
                    read_lsb_bits(block.coefficient_payload, &mut coefficient_offset, 1)? != 0;
                let magnitude =
                    read_lsb_bits(block.coefficient_payload, &mut coefficient_offset, width)?;
                let magnitude =
                    i16::try_from(magnitude).map_err(|_| AndaleError::IntegerOverflow)?;
                *coefficient = signed(positive, magnitude);
            }
            rows.push(row);
        }

        let expected_payload_size = coefficient_offset.div_ceil(8);
        // The native overlap path does not consume the suffix of the original
        // block. Keep that suffix in coefficient_payload; bounded bit reads
        // above still reject truncation. Other layouts retain the exact-size
        // admission check rather than accepting arbitrary count mismatches.
        let overlapping = block.declared_significance_bit_count == 0 && significance_offset == 256;
        if !overlapping && block.coefficient_payload.len() != expected_payload_size {
            return Err(AndaleError::VbrCoefficientPayloadSizeMismatch {
                expected: expected_payload_size,
                actual: block.coefficient_payload.len(),
            });
        }
        Ok(rows)
    }
}

/// Expands the raw-discretize codec's three stored quaternion components.
/// The reference codec reconstructs the non-negative fourth component.
#[must_use]
pub fn decompress_raw_quaternion3(encoded: [u8; 6]) -> [f32; 4] {
    let xyz = decode_normalized_i16x3(encoded);
    let squared_length = xyz
        .iter()
        .map(|component| component * component)
        .sum::<f32>();
    [
        xyz[0],
        xyz[1],
        xyz[2],
        (1.0 - squared_length).max(0.0).sqrt(),
    ]
}

/// Expands and normalizes the raw-discretize codec's four signed components.
#[must_use]
pub fn decompress_raw_quaternion4(encoded: [u8; 8]) -> [f32; 4] {
    let mut quaternion = [0.0_f32; 4];
    for (index, bytes) in encoded.chunks_exact(2).enumerate() {
        quaternion[index] = f32::from(i16::from_be_bytes([bytes[0], bytes[1]])) / 32767.0;
    }
    let reciprocal_length = quaternion
        .iter()
        .map(|component| component * component)
        .sum::<f32>()
        .sqrt()
        .recip();
    quaternion.map(|component| component * reciprocal_length)
}

/// Expands the raw-discretize codec's signed translation and homogeneous lane.
#[must_use]
pub fn decompress_raw_translation(encoded: [u8; 6], range: f32) -> [f32; 4] {
    let xyz = decode_normalized_i16x3(encoded).map(|component| component * range);
    [xyz[0], xyz[1], xyz[2], 1.0]
}

/// Expands the raw-discretize codec's spherical 32-bit quaternion.
///
/// The top 11 bits encode an azimuth over `[-pi, pi]`, the next 10 encode an
/// elevation over `[-pi/2, pi/2]`, and the low 11 encode the vector-part
/// magnitude. The scalar is reconstructed as the non-negative square root.
/// `QuatCompress` flips the vector lanes when the original scalar is negative,
/// so this canonical representation preserves rotation.
#[must_use]
pub fn decompress_raw_packed_quaternion(encoded: u32) -> [f32; 4] {
    let azimuth_code = u16::try_from(encoded >> 21).unwrap_or_default();
    let elevation_code = u16::try_from((encoded >> 11) & 0x03ff).unwrap_or_default();
    let vector_length_code = u16::try_from(encoded & 0x07ff).unwrap_or_default();
    let azimuth =
        f32::from(azimuth_code).mul_add(std::f32::consts::TAU / 2047.0, -std::f32::consts::PI);
    let elevation = f32::from(elevation_code)
        .mul_add(std::f32::consts::PI / 1023.0, -std::f32::consts::FRAC_PI_2);
    let vector_length = f32::from(vector_length_code) / 2047.0;
    let scalar = (1.0 - vector_length * vector_length).max(0.0).sqrt();
    let elevation_cosine = elevation.cos();
    [
        elevation_cosine * azimuth.cos() * vector_length,
        elevation_cosine * azimuth.sin() * vector_length,
        elevation.sin() * vector_length,
        scalar,
    ]
}

/// Reconstructs one eight-frame VBR channel block from its DCT coefficients.
///
/// Reference: `ANIM_CODEC::InitIDctTablesInv` at `0x82edb3d0` uses the native
/// cosine polynomial and precomputed frequency constants. Each pair of dot4
/// reductions uses the documented host policy in `vbr_numeric`.
#[must_use]
pub fn vbr_inverse_dct8(coefficients: [f32; 8]) -> [f32; 8] {
    vbr_numeric::inverse_dct_table()
        .map(|weights| vbr_numeric::reconstruct_sample(&coefficients, &weights))
}

/// Builds the eight coefficient scales selected by one frame block's table
/// index and the animation-wide DCT range.
#[must_use]
pub fn vbr_quantization_row(table_index: u8, dct_minimum: f32, dct_maximum: f32) -> [f32; 8] {
    let range = dct_minimum.abs().max(dct_maximum.abs());
    let range_squared = range * range;
    let minimum_scale = range * f32::from_bits(0x3800_0000);
    vbr_numeric::initial_quantization_row(table_index)
        .map(|initial| range_squared.mul_add(initial, minimum_scale))
}

/// Dequantizes and inverse-transforms one scalar channel's eight DCT values.
#[must_use]
pub fn vbr_reconstruct_scalar_samples(
    coefficients: [i16; 8],
    table_index: u8,
    dct_minimum: f32,
    dct_maximum: f32,
    channel_minimum: f32,
    channel_maximum: f32,
) -> [f32; 8] {
    reconstruct_float_scalar_samples(
        coefficients.map(f32::from),
        table_index,
        dct_minimum,
        dct_maximum,
        channel_minimum,
        channel_maximum,
    )
}

fn reconstruct_float_scalar_samples(
    coefficients: [f32; 8],
    table_index: u8,
    dct_minimum: f32,
    dct_maximum: f32,
    channel_minimum: f32,
    channel_maximum: f32,
) -> [f32; 8] {
    let quantization = vbr_quantization_row(table_index, dct_minimum, dct_maximum);
    let normalized = vbr_numeric::inverse_dct_table().map(|weights| {
        // 82edba88/ba90 pre-scale weights before multiplying coefficients.
        let scaled = std::array::from_fn(|index| quantization[index] * weights[index]);
        vbr_numeric::reconstruct_sample(&coefficients, &scaled)
    });
    // DecompressFrameBlock82edbb74..90 adds vhalf before the fused range
    // mapping. Its initializer82fabf10 converts integer1 with binary scale1.
    normalized
        .map(|value| (value + 0.5).mul_add(channel_maximum - channel_minimum, channel_minimum))
}

/// Host reuse of Vector_UnPackFrameBlockAndDecompressOneFrame82edbd18's
/// deterministic block output. DecompressFrameBlock82edb790 then reads these
/// same coefficients with a different IDCT row for each requested frame.
/// This is scoped to a single clip-part sampling call, not a global cache.
struct VbrSqtBlock<'a> {
    index: usize,
    block: BorrowedVbrFrameBlock<'a>,
    coefficient_rows: Vec<[f32; 8]>,
}

#[allow(clippy::unnecessary_lazy_evaluations,
    reason = "ARM64 otherwise calls AndaleError drop glue on successful reads")]
fn sample_vbr_sqt<'a>(
    header: &AndaleVbrCompressionHeader,
    prelude: &AndaleVbrPrelude,
    compressed_data: &'a [u8],
    bone_count: usize,
    frame: usize,
    cached: &mut Option<VbrSqtBlock<'a>>,
) -> Result<Vec<AndaleSqt>, AndaleError> {
    if frame >= usize::from(header.frame_count) {
        return Err(AndaleError::VbrFrameOutsideAnimation {
            frame,
            frame_count: usize::from(header.frame_count),
        });
    }
    let component_counts = header
        .channel_blocks
        .iter()
        .map(|channel| channel.component_count);
    if !component_counts.clone().eq([3, 4, 3]) {
        return Err(AndaleError::VbrSqtChannelLayout(component_counts.collect()));
    }

    if cached.as_ref().is_none_or(|block| block.index != frame / 8) {
        let block = header.borrow_frame_block(prelude, compressed_data, frame / 8)?;
        let coefficient_rows =
            AndaleVbrCompressionHeader::unpack_float_coefficient_rows(prelude, block)?;
        *cached = Some(VbrSqtBlock { index: frame / 8, block, coefficient_rows });
    }
    let decoded = cached.as_ref().expect("decoded VBR frame block");
    let block = &decoded.block;
    let coefficient_rows = &decoded.coefficient_rows;
    let expected_rows = header.channel_blocks.iter().try_fold(
        0_usize,
        |total, channel| -> Result<usize, AndaleError> {
            total
                .checked_add(
                    usize::from(channel.animated_entry_count)
                        .checked_mul(usize::from(channel.component_count))
                        .ok_or_else(|| AndaleError::IntegerOverflow)?,
                )
                .ok_or_else(|| AndaleError::IntegerOverflow)
        },
    )?;
    if coefficient_rows.len() != expected_rows {
        return Err(AndaleError::VbrCoefficientRowCountMismatch {
            expected: expected_rows,
            actual: coefficient_rows.len(),
        });
    }

    let mut sqt = vec![
        AndaleSqt {
            scale: [0.0, 0.0, 0.0, 1.0],
            rotation: [0.0; 4],
            translation: [0.0, 0.0, 0.0, 1.0],
        };
        bone_count
    ];
    let mut entry_offset = 0_usize;
    let mut coefficient_offset = 0_usize;
    // The packed part header admits at most 64 bones; the largest SQT channel
    // has four components. DecompressFrameBlock82edb790 likewise writes into
    // scratch storage. Preserve channel/scalar evaluation order without a
    // fresh heap allocation for each of the three channels in every sample.
    let mut animated_values = [0.0; 64 * 4];
    for (channel_index, channel) in header.channel_blocks.iter().enumerate() {
        let channel_entry_count = usize::from(channel.animated_entry_count)
            .checked_add(usize::from(channel.constant_entry_count))
            .ok_or_else(|| AndaleError::IntegerOverflow)?;
        if channel_entry_count != bone_count {
            return Err(AndaleError::VbrSqtBoneCountMismatch {
                channel: channel_index,
                expected: bone_count,
                actual: channel_entry_count,
            });
        }
        let component_count = usize::from(channel.component_count);
        let animated_scalar_count = usize::from(channel.animated_entry_count)
            .checked_mul(component_count)
            .ok_or_else(|| AndaleError::IntegerOverflow)?;
        let table_index = block
            .quantization_table_indices
            .get(channel_index)
            .copied()
            .ok_or_else(|| AndaleError::VbrSqtChannelDataTruncated(channel_index))?;
        let coefficient_end = coefficient_offset
            .checked_add(animated_scalar_count)
            .ok_or_else(|| AndaleError::IntegerOverflow)?;
        let channel_coefficients = coefficient_rows
            .get(coefficient_offset..coefficient_end)
            .ok_or_else(|| AndaleError::VbrSqtChannelDataTruncated(channel_index))?;
        let animated_values = &mut animated_values[..animated_scalar_count];
        reconstruct_vbr_channel_values(
            header,
            channel,
            table_index,
            channel_coefficients,
            frame,
            animated_values,
        );
        coefficient_offset += animated_scalar_count;

        write_vbr_sqt_channel(
            &mut sqt,
            prelude,
            channel_index,
            component_count,
            entry_offset,
            animated_values,
        )?;
        entry_offset += channel_entry_count;
    }
    Ok(sqt)
}

// Eager ok_or constructs a large drop-bearing enum even on success. The
// Android release compiler emitted three drop calls per bone here, plus
// three per present compressed coefficient in read_lsb_bits. Keep the same
// checks and errors, constructing the error only on the failing branch.
#[allow(clippy::unnecessary_lazy_evaluations,
    reason = "ARM64 otherwise calls AndaleError drop glue on successful reads")]
fn write_vbr_sqt_channel(
    sqt: &mut [AndaleSqt],
    prelude: &AndaleVbrPrelude,
    channel_index: usize,
    component_count: usize,
    entry_offset: usize,
    animated_values: &[f32],
) -> Result<(), AndaleError> {
    let constant_values = prelude
        .constant_values
        .get(channel_index)
        .ok_or_else(|| AndaleError::VbrSqtChannelDataTruncated(channel_index))?;
    let mut constant_offset = 0_usize;
    let mut animated_offset = 0_usize;
    for (bone, transform) in sqt.iter_mut().enumerate() {
        let is_constant = prelude
            .constant_entries
            .get(entry_offset + bone)
            .copied()
            .ok_or_else(|| AndaleError::VbrSqtChannelDataTruncated(channel_index))?;
        let (source, offset) = if is_constant {
            (constant_values.as_slice(), &mut constant_offset)
        } else {
            (animated_values, &mut animated_offset)
        };
        let end = offset
            .checked_add(component_count)
            .ok_or_else(|| AndaleError::IntegerOverflow)?;
        let components = source
            .get(*offset..end)
            .ok_or_else(|| AndaleError::VbrSqtChannelDataTruncated(channel_index))?;
        match channel_index {
            0 => transform.scale[..3].copy_from_slice(components),
            1 => transform.rotation.copy_from_slice(components),
            2 => transform.translation[..3].copy_from_slice(components),
            _ => return Err(AndaleError::VbrSqtChannelLayout(Vec::new())),
        }
        *offset = end;
    }
    Ok(())
}

fn reconstruct_vbr_channel_values(
    header: &AndaleVbrCompressionHeader,
    channel: &AndaleVbrChannelBlock,
    table_index: u8,
    coefficient_rows: &[[f32; 8]],
    frame: usize,
    output: &mut [f32],
) {
    // DecompressFrameBlock82edb838 selects the requested IDCT row, then
    // 82edba88..90 scales it once for this channel before the scalar loop.
    // Other frame rows have no contribution to this frame's result.
    let quantization = vbr_quantization_row(table_index, header.dct_minimum, header.dct_maximum);
    let weights = &vbr_numeric::inverse_dct_table()[frame & 7];
    let scaled = std::array::from_fn(|index| quantization[index] * weights[index]);
    let range = channel.maximum - channel.minimum;
    for (value, coefficients) in output.iter_mut().zip(coefficient_rows) {
        let normalized = vbr_numeric::reconstruct_sample(coefficients, &scaled);
        *value = (normalized + 0.5).mul_add(range, channel.minimum);
    }
}

impl AndaleAnimationPart {
    #[must_use]
    pub fn bone_count(&self) -> usize {
        usize::try_from((self.header >> 6 & 0x3f) + 1).unwrap_or_default()
    }

    #[must_use]
    pub fn pose_part_index(&self) -> usize {
        usize::try_from((self.header & 0x3f) / 2).unwrap_or_default()
    }

    #[must_use]
    pub fn packed_header_size(&self) -> usize {
        usize::try_from(self.header >> 12).unwrap_or_default()
    }

    /// Decodes one authored raw-discretize pose part.
    ///
    /// Each bone stores selected float3 scale, packed or float4 quaternion, then
    /// float3 translation, using the same extraction as integer RAW clip frames.
    /// Omitted channels retain SQT identity. Other format selectors remain
    /// explicit errors; authored scale channels are used by vehicle bind poses.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] for a non-raw part, a raw channel layout not
    /// supported by the shared extractor, or an inconsistent byte count.
    pub fn sample_raw_pose_sqt(&self) -> Result<Vec<AndaleSqt>, AndaleError> {
        let AndaleCompressionHeader::RawDiscretize(header) = &self.compression else {
            return Err(AndaleError::NotRawDiscretizeCompression);
        };
        if header.channel_masks[1] != 0
            || header.channel_masks[3] != 0
            || header.channel_masks[5] != 0
            || !matches!(header.format_flags, [0, 0] | [0, 1])
        {
            return Err(AndaleError::UnsupportedRawPoseLayout {
                channel_masks: header.channel_masks,
                format_flags: header.format_flags,
            });
        }

        let bone_count = self.bone_count();
        // Parsed databases already validate these bounds. Keep this public
        // part sampler safe for independently constructed parts as well.
        if bone_count > 32
            || header
                .channel_masks
                .iter()
                .any(|mask| *mask & !(u32::MAX >> (32 - bone_count.min(32))) != 0)
        {
            return Err(AndaleError::UnsupportedRawPoseLayout {
                channel_masks: header.channel_masks,
                format_flags: header.format_flags,
            });
        }
        let scale_mask = header.channel_masks[0];
        let scale_size = usize::try_from(scale_mask.count_ones())
            .map_err(|_| AndaleError::IntegerOverflow)?
            .checked_mul(12)
            .ok_or(AndaleError::IntegerOverflow)?;
        let rotation_mask = header.channel_masks[2];
        let translation_mask = header.channel_masks[4];
        let rotation_size = usize::try_from(rotation_mask.count_ones())
            .map_err(|_| AndaleError::IntegerOverflow)?
            .checked_mul(if header.format_flags[1] == 0 { 16 } else { 4 })
            .ok_or(AndaleError::IntegerOverflow)?;
        let translation_size = usize::try_from(translation_mask.count_ones())
            .map_err(|_| AndaleError::IntegerOverflow)?
            .checked_mul(12)
            .ok_or(AndaleError::IntegerOverflow)?;
        let expected_size = scale_size
            .checked_add(rotation_size)
            .and_then(|size| size.checked_add(translation_size))
            .ok_or(AndaleError::IntegerOverflow)?;
        if self.compressed_data.len() != expected_size {
            return Err(AndaleError::RawPoseDataSizeMismatch {
                expected: expected_size,
                actual: self.compressed_data.len(),
            });
        }

        raw::decode_raw_channels(
            &self.compressed_data,
            0,
            bone_count,
            [scale_mask, rotation_mask, translation_mask],
            header.format_flags[1] != 0,
        )
    }

    /// Materializes a VBR part that contains only constant SQT entries.
    /// Animated/DCT-backed parts return `None` until their frame sampler runs.
    #[must_use]
    pub fn constant_vbr_sqt(&self) -> Option<Vec<AndaleSqt>> {
        let AndaleCompressionHeader::Vbr(header) = &self.compression else {
            return None;
        };
        let prelude = self.vbr_prelude.as_ref()?;
        let bone_count = self.bone_count();
        if header.channel_blocks.len() != 3
            || header
                .channel_blocks
                .iter()
                .map(|block| block.component_count)
                .ne([3, 4, 3])
            || header.channel_blocks.iter().any(|block| {
                block.animated_entry_count != 0
                    || usize::from(block.constant_entry_count) != bone_count
            })
            || prelude.constant_entries.len() != bone_count * 3
            || prelude
                .constant_entries
                .iter()
                .any(|is_constant| !is_constant)
        {
            return None;
        }
        let scale = &prelude.constant_values[0];
        let rotation = &prelude.constant_values[1];
        let translation = &prelude.constant_values[2];
        Some(
            (0..bone_count)
                .map(|bone| AndaleSqt {
                    scale: [
                        scale[bone * 3],
                        scale[bone * 3 + 1],
                        scale[bone * 3 + 2],
                        1.0,
                    ],
                    rotation: [
                        rotation[bone * 4],
                        rotation[bone * 4 + 1],
                        rotation[bone * 4 + 2],
                        rotation[bone * 4 + 3],
                    ],
                    translation: [
                        translation[bone * 3],
                        translation[bone * 3 + 1],
                        translation[bone * 3 + 2],
                        1.0,
                    ],
                })
                .collect(),
        )
    }
}

impl AndalePose {
    /// Materializes a complete raw-discretize pose in hierarchy bone order.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] if the pose and hierarchy disagree or a part
    /// uses an unsupported raw channel layout.
    pub fn sample_raw_sqt(
        &self,
        hierarchy: &AndaleHierarchy,
    ) -> Result<Vec<AndaleSqt>, AndaleError> {
        self.validate_against(hierarchy)?;
        let mut pose = vec![identity_sqt(); hierarchy.bone_names.len()];
        for (animation_part, hierarchy_part) in self.parts.iter().zip(&hierarchy.parts) {
            let part_pose = animation_part.sample_raw_pose_sqt()?;
            let start = usize::try_from(hierarchy_part.bone_offset)
                .map_err(|_| AndaleError::IntegerOverflow)?;
            let end = start
                .checked_add(part_pose.len())
                .ok_or(AndaleError::IntegerOverflow)?;
            let hierarchy_bones = hierarchy.bone_names.len();
            let target =
                pose.get_mut(start..end)
                    .ok_or(AndaleError::RawPosePartOutsideHierarchy {
                        offset: start,
                        bones: part_pose.len(),
                        hierarchy_bones,
                    })?;
            target.copy_from_slice(&part_pose);
        }
        Ok(pose)
    }
}

const fn identity_sqt() -> AndaleSqt {
    AndaleSqt {
        scale: [1.0, 1.0, 1.0, 1.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [0.0, 0.0, 0.0, 1.0],
    }
}

/// Serialized `Util::FastString<N * 6>` storage.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PackedFastString<const WORDS: usize> {
    pub words: [u32; WORDS],
}

impl<const WORDS: usize> PackedFastString<WORDS> {
    /// Encodes the lossless alphabet used by the reference `FastString<6>::Assign`.
    /// Letter case is intentionally discarded by the original routine.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] if the input exceeds the fixed capacity or
    /// contains a byte outside digits, ASCII letters, and underscore.
    #[allow(clippy::unnecessary_lazy_evaluations, reason = "avoid ARM64 AndaleError drop glue after each successful character")]
    pub fn encode(value: &str) -> Result<Self, AndaleError> {
        let bytes = value.as_bytes();
        let capacity = WORDS
            .checked_mul(FAST_STRING_WORD_CHARS)
            .ok_or_else(|| AndaleError::IntegerOverflow)?;
        if bytes.len() > capacity {
            return Err(AndaleError::FastStringTooLong {
                length: bytes.len(),
                capacity,
            });
        }

        let mut words = [0_u32; WORDS];
        for (word_index, chunk) in bytes.chunks(FAST_STRING_WORD_CHARS).enumerate() {
            let mut multiplier = FAST_STRING_RADIX.pow(5);
            for &byte in chunk {
                let digit =
                    fast_string_digit(byte).ok_or_else(|| AndaleError::InvalidFastStringCharacter(byte))?;
                words[word_index] = words[word_index].wrapping_add(digit.wrapping_mul(multiplier));
                multiplier /= FAST_STRING_RADIX;
            }
        }
        Ok(Self { words })
    }

    /// Decodes the canonical lowercase representation of the packed name.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] if a word is outside the six-digit base-38
    /// domain or contains nonzero digits after its terminator.
    pub fn decode(&self) -> Result<String, AndaleError> {
        let mut decoded = String::with_capacity(WORDS * FAST_STRING_WORD_CHARS);
        let mut terminated = false;
        for &word in &self.words {
            let digits = decode_fast_string_word(word)?;
            for digit in digits {
                if digit == 0 {
                    terminated = true;
                } else if terminated {
                    return Err(AndaleError::NonCanonicalFastString(self.words.to_vec()));
                } else {
                    decoded.push(fast_string_character(digit));
                }
            }
        }
        Ok(decoded)
    }
}

impl AndaleDatabase {
    /// Evaluates a `PhaseBlend` tree into one native SQT pose at normalized
    /// animation time. Child coordinates come from serialized clip attributes;
    /// neighboring child poses use the coefficient recovered from
    /// `PhaseBlend::Eval`.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] for invalid time/cull inputs, absent controller
    /// attributes, unsupported node kinds, cycles, or incompatible child poses.
    pub fn sample_phase_blend_pose_sqt(
        &self,
        root: &str,
        hierarchy: &AndaleHierarchy,
        normalized_time: f32,
        attributes: &[(&str, f32)],
        blend_cull_threshold: f32,
    ) -> Result<AndalePhaseBlendPoseSample, AndaleError> {
        if !normalized_time.is_finite() || !(0.0..=1.0).contains(&normalized_time) {
            return Err(AndaleError::InvalidNormalizedAnimationTime(
                normalized_time.to_bits(),
            ));
        }
        if !blend_cull_threshold.is_finite() || !(0.0..0.5).contains(&blend_cull_threshold) {
            return Err(AndaleError::InvalidPhaseBlendCullThreshold(
                blend_cull_threshold.to_bits(),
            ));
        }
        self.sample_phase_blend_node(
            root,
            hierarchy,
            normalized_time,
            attributes,
            blend_cull_threshold,
            &mut Vec::new(),
        )
    }

    /// Evaluates the ordered runtime attributes emitted by the same recursive
    /// `PhaseBlend` path as the sampled pose. The default active filter keeps
    /// static and currently active timed attributes, matching the physics
    /// animation handoff.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] for invalid times, unsupported controller nodes,
    /// cycles, missing blend inputs, or incompatible matched attribute types.
    pub fn sample_phase_blend_attributes(
        &self,
        root: &str,
        current_normalized_time: f32,
        previous_normalized_time: f32,
        looped: bool,
        attributes: &[(&str, f32)],
        blend_cull_threshold: f32,
    ) -> Result<Vec<AndaleRuntimeAttribute>, AndaleError> {
        validate_normalized_time(current_normalized_time)?;
        validate_normalized_time(previous_normalized_time)?;
        if !blend_cull_threshold.is_finite() || !(0.0..0.5).contains(&blend_cull_threshold) {
            return Err(AndaleError::InvalidPhaseBlendCullThreshold(
                blend_cull_threshold.to_bits(),
            ));
        }
        let context = AndaleAttributeEvalContext {
            current_time: current_normalized_time,
            previous_time: previous_normalized_time,
            looped,
            inputs: attributes,
            blend_cull_threshold,
        };
        self.sample_phase_blend_attribute_node(
            root,
            ANDALE_ACTIVE_ATTRIBUTE_FILTER,
            context,
            &mut Vec::new(),
        )
    }

    fn sample_phase_blend_attribute_node(
        &self,
        name: &str,
        status_filter: u8,
        context: AndaleAttributeEvalContext<'_>,
        visiting: &mut Vec<String>,
    ) -> Result<Vec<AndaleRuntimeAttribute>, AndaleError> {
        if visiting.iter().any(|visited| visited == name) {
            return Err(AndaleError::ControllerCycle(name.to_owned()));
        }
        if let Some(clip) = self.clips.iter().find(|clip| clip.name == name) {
            return clip.runtime_attributes(
                context.current_time,
                context.previous_time,
                context.looped,
                status_filter,
            );
        }
        let controller = self
            .controllers
            .iter()
            .find(|controller| controller.name() == name)
            .ok_or_else(|| AndaleError::ControllerNotFound(name.to_owned()))?;
        let AndaleAnimationController::PhaseBlend(phase_blend) = controller else {
            return Err(AndaleError::ControllerIsNotPhaseBlend(name.to_owned()));
        };
        visiting.push(name.to_owned());
        let selection =
            self.phase_selection(phase_blend, context.inputs, context.blend_cull_threshold)?;
        let result = if selection.raw_coefficient < context.blend_cull_threshold {
            self.sample_phase_blend_attribute_node(
                selection.lower,
                status_filter,
                context,
                visiting,
            )
        } else if selection.raw_coefficient > 1.0 - context.blend_cull_threshold {
            self.sample_phase_blend_attribute_node(
                selection.upper,
                status_filter,
                context,
                visiting,
            )
        } else {
            let lower = self.sample_phase_blend_attribute_node(
                selection.lower,
                status_filter,
                context,
                visiting,
            )?;
            let upper = self.sample_phase_blend_attribute_node(
                selection.upper,
                ANDALE_ALL_ATTRIBUTE_FILTER,
                context,
                visiting,
            )?;
            blend_runtime_attributes(lower, &upper, selection.raw_coefficient)
        };
        visiting.pop();
        result
    }

    fn sample_phase_blend_node(
        &self,
        name: &str,
        hierarchy: &AndaleHierarchy,
        normalized_time: f32,
        attributes: &[(&str, f32)],
        blend_cull_threshold: f32,
        visiting: &mut Vec<String>,
    ) -> Result<AndalePhaseBlendPoseSample, AndaleError> {
        if visiting.iter().any(|visited| visited == name) {
            return Err(AndaleError::ControllerCycle(name.to_owned()));
        }
        if let Some(clip) = self.clips.iter().find(|clip| clip.name == name) {
            return Ok(AndalePhaseBlendPoseSample {
                transforms: clip
                    .sample_vbr_pose_sqt_at_normalized_time(hierarchy, normalized_time)?,
                duration_seconds: clip.duration_seconds(),
                weighted_clips: vec![AndaleWeightedClip {
                    name: clip.name.clone(),
                    weight: 1.0,
                }],
            });
        }
        let controller = self
            .controllers
            .iter()
            .find(|controller| controller.name() == name)
            .ok_or_else(|| AndaleError::ControllerNotFound(name.to_owned()))?;
        let AndaleAnimationController::PhaseBlend(phase_blend) = controller else {
            return Err(AndaleError::ControllerIsNotPhaseBlend(name.to_owned()));
        };
        visiting.push(name.to_owned());
        let selection = self.phase_selection(phase_blend, attributes, blend_cull_threshold)?;
        let lower = self.sample_phase_blend_node(
            selection.lower,
            hierarchy,
            normalized_time,
            attributes,
            blend_cull_threshold,
            visiting,
        )?;
        let upper = self.sample_phase_blend_node(
            selection.upper,
            hierarchy,
            normalized_time,
            attributes,
            blend_cull_threshold,
            visiting,
        )?;
        visiting.pop();
        if lower.transforms.len() != upper.transforms.len() {
            return Err(AndaleError::PhaseBlendPoseSizeMismatch {
                lower: lower.transforms.len(),
                upper: upper.transforms.len(),
            });
        }
        let transforms = lower
            .transforms
            .into_iter()
            .zip(upper.transforms)
            .map(|(lower, upper)| blend_sqt(lower, upper, selection.pose_coefficient))
            .collect::<Result<Vec<_>, _>>()?;
        let duration_seconds = lower.duration_seconds
            + (upper.duration_seconds - lower.duration_seconds) * selection.raw_coefficient;
        let weighted_clips = combine_weighted_clips(
            lower.weighted_clips,
            upper.weighted_clips,
            selection.pose_coefficient,
        );
        Ok(AndalePhaseBlendPoseSample {
            transforms,
            duration_seconds,
            weighted_clips,
        })
    }

    fn phase_selection<'a>(
        &'a self,
        phase_blend: &'a AndalePhaseBlend,
        attributes: &[(&str, f32)],
        blend_cull_threshold: f32,
    ) -> Result<AndalePhaseSelection<'a>, AndaleError> {
        let input = attributes
            .iter()
            .find_map(|(name, value)| (*name == phase_blend.blend_attribute).then_some(*value))
            .ok_or_else(|| AndaleError::MissingPhaseBlendAttribute {
                controller: phase_blend.name.clone(),
                attribute: phase_blend.blend_attribute.clone(),
            })?;
        let mut coordinates = phase_blend
            .children
            .iter()
            .map(|child| {
                Ok((
                    child.as_str(),
                    self.evaluate_scalar_attribute(
                        child,
                        &phase_blend.blend_attribute,
                        attributes,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, AndaleError>>()?;
        if coordinates.len() < 2 {
            return Err(AndaleError::PhaseBlendNeedsTwoChildren {
                controller: phase_blend.name.clone(),
                children: coordinates.len(),
            });
        }
        coordinates.sort_by(|left, right| left.1.total_cmp(&right.1));
        let mut upper_index = coordinates.partition_point(|coordinate| coordinate.1 < input);
        if upper_index == coordinates.len() {
            upper_index -= 1;
        }
        if upper_index == 0 {
            upper_index = 1;
        }
        let lower_index = upper_index - 1;
        let lower_value = coordinates[lower_index].1;
        let upper_value = coordinates[upper_index].1;
        let range = upper_value - lower_value;
        let raw_coefficient = if range.to_bits().trailing_zeros() >= 31 {
            0.0
        } else {
            ((input - lower_value) / range).clamp(0.0, 1.0)
        };
        let pose_coefficient = phase_pose_coefficient(
            raw_coefficient,
            blend_cull_threshold,
            lower_index == 0,
            upper_index + 1 == coordinates.len(),
        );
        Ok(AndalePhaseSelection {
            lower: coordinates[lower_index].0,
            upper: coordinates[upper_index].0,
            raw_coefficient,
            pose_coefficient,
        })
    }

    fn evaluate_scalar_attribute(
        &self,
        name: &str,
        attribute: &str,
        inputs: &[(&str, f32)],
    ) -> Result<f32, AndaleError> {
        self.evaluate_scalar_attribute_inner(name, attribute, inputs, &mut Vec::new())
    }

    fn evaluate_scalar_attribute_inner(
        &self,
        name: &str,
        attribute: &str,
        inputs: &[(&str, f32)],
        visiting: &mut Vec<String>,
    ) -> Result<f32, AndaleError> {
        if visiting.iter().any(|visited| visited == name) {
            return Err(AndaleError::ControllerCycle(name.to_owned()));
        }
        if let Some(clip) = self.clips.iter().find(|clip| clip.name == name) {
            return clip
                .attributes
                .iter()
                .find(|value| {
                    value.name == attribute && value.begin.to_bits() == (-1.0_f32).to_bits()
                })
                .and_then(AndaleAttribute::scalar_value)
                .ok_or_else(|| AndaleError::ClipScalarAttributeNotFound {
                    clip: name.to_owned(),
                    attribute: attribute.to_owned(),
                });
        }
        let controller = self
            .controllers
            .iter()
            .find(|controller| controller.name() == name)
            .ok_or_else(|| AndaleError::ControllerNotFound(name.to_owned()))?;
        let AndaleAnimationController::PhaseBlend(phase_blend) = controller else {
            return Err(AndaleError::ControllerIsNotPhaseBlend(name.to_owned()));
        };
        visiting.push(name.to_owned());
        let selection = self.phase_selection(phase_blend, inputs, 0.0)?;
        let lower =
            self.evaluate_scalar_attribute_inner(selection.lower, attribute, inputs, visiting)?;
        let upper =
            self.evaluate_scalar_attribute_inner(selection.upper, attribute, inputs, visiting)?;
        visiting.pop();
        Ok(lower + (upper - lower) * selection.raw_coefficient)
    }

    /// Returns a scalar clip attribute when every leaf below `name` exposes
    /// the same always-active value. This is how a parent `PhaseBlend`
    /// obtains each child's coordinate in the reference `SetAttributes` path.
    #[must_use]
    pub fn static_scalar_attribute(&self, name: &str, attribute: &str) -> Option<f32> {
        self.static_scalar_attribute_inner(name, attribute, &mut Vec::new())
    }

    /// Follows exact-coordinate `PhaseBlend` choices to one serialized clip.
    /// This deliberately handles only endpoint selection; interpolation
    /// between two children produces a blended pose rather than one clip.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] for missing input attributes, non-phase nodes,
    /// cycles, or values without an exact child coordinate.
    pub fn resolve_exact_phase_blend_clip(
        &self,
        root: &str,
        attributes: &[(&str, f32)],
    ) -> Result<&AndaleClip, AndaleError> {
        let mut current = root.to_owned();
        let mut visited = Vec::new();
        loop {
            if let Some(clip) = self.clips.iter().find(|clip| clip.name == current) {
                return Ok(clip);
            }
            if visited.contains(&current) {
                return Err(AndaleError::ControllerCycle(current));
            }
            visited.push(current.clone());
            let controller = self
                .controllers
                .iter()
                .find(|controller| controller.name() == current)
                .ok_or_else(|| AndaleError::ControllerNotFound(current.clone()))?;
            let AndaleAnimationController::PhaseBlend(phase_blend) = controller else {
                return Err(AndaleError::ControllerIsNotPhaseBlend(current));
            };
            let value = attributes
                .iter()
                .find_map(|(name, value)| (*name == phase_blend.blend_attribute).then_some(*value))
                .ok_or_else(|| AndaleError::MissingPhaseBlendAttribute {
                    controller: current.clone(),
                    attribute: phase_blend.blend_attribute.clone(),
                })?;
            current = phase_blend
                .children
                .iter()
                .find(|child| {
                    self.static_scalar_attribute(child, &phase_blend.blend_attribute)
                        .is_some_and(|coordinate| coordinate.to_bits() == value.to_bits())
                })
                .cloned()
                .ok_or_else(|| AndaleError::NoExactPhaseBlendCoordinate {
                    controller: current.clone(),
                    attribute: phase_blend.blend_attribute.clone(),
                    value: value.to_bits(),
                })?;
        }
    }

    fn static_scalar_attribute_inner(
        &self,
        name: &str,
        attribute: &str,
        visiting: &mut Vec<String>,
    ) -> Option<f32> {
        if visiting.iter().any(|visited| visited == name) {
            return None;
        }
        if let Some(clip) = self.clips.iter().find(|clip| clip.name == name) {
            let mut values = clip
                .attributes
                .iter()
                .filter(|value| {
                    value.name == attribute && value.begin.to_bits() == (-1.0_f32).to_bits()
                })
                .filter_map(AndaleAttribute::scalar_value);
            let first = values.next()?;
            return values
                .all(|value| value.to_bits() == first.to_bits())
                .then_some(first);
        }
        let controller = self
            .controllers
            .iter()
            .find(|controller| controller.name() == name)?;
        visiting.push(name.to_owned());
        let child_values = controller
            .child_names()
            .into_iter()
            .map(|child| self.static_scalar_attribute_inner(child, attribute, visiting))
            .collect::<Option<Vec<_>>>();
        let mut values = child_values?.into_iter();
        let first = values.next();
        let consistent =
            first.filter(|first| values.all(|value| value.to_bits() == first.to_bits()));
        visiting.pop();
        consistent
    }

    /// Resolves an animation clip by walking serialized controller children.
    /// Branch indices are supplied by the caller; evaluating runtime
    /// attributes and blend weights is a separate animation-system concern.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] if a controller or branch is absent, or if the
    /// final serialized name is not a clip.
    pub fn resolve_controller_branch_indices(
        &self,
        root: &str,
        branch_indices: &[usize],
    ) -> Result<&AndaleClip, AndaleError> {
        let mut current = root;
        for &branch_index in branch_indices {
            let controller = self
                .controllers
                .iter()
                .find(|controller| controller.name() == current)
                .ok_or_else(|| AndaleError::ControllerNotFound(current.to_owned()))?;
            let children = controller.child_names();
            current = children.get(branch_index).copied().ok_or_else(|| {
                AndaleError::ControllerBranchOutsideChildren {
                    controller: current.to_owned(),
                    branch: branch_index,
                    children: children.len(),
                }
            })?;
        }
        self.clips
            .iter()
            .find(|clip| clip.name == current)
            .ok_or_else(|| AndaleError::ControllerTerminalIsNotClip(current.to_owned()))
    }

    /// Parses one complete `.abin` database and its hierarchy record.
    ///
    /// # Errors
    ///
    /// Returns [`AndaleError`] for truncated headers, invalid record sizes,
    /// malformed packed names, or a missing/duplicate hierarchy.
    pub fn parse(bytes: &[u8]) -> Result<Self, AndaleError> {
        let (file_header, declared_size) = parse_and_validate_file_header(bytes)?;

        let mut cursor = align_up(DATA_HEADER_SIZE, RECORD_ALIGNMENT)?;
        let mut records = Vec::new();
        let mut hierarchy = None;
        let mut clips = Vec::new();
        let mut poses = Vec::new();
        let mut physics_parameters = Vec::new();
        let mut controllers = Vec::new();
        while cursor < declared_size {
            let size = read_be_u32(bytes, cursor)?;
            if size == 0 {
                break;
            }
            let header = parse_data_header(bytes, cursor)?;
            let record_size =
                usize::try_from(header.size).map_err(|_| AndaleError::IntegerOverflow)?;
            if record_size < DATA_HEADER_SIZE || record_size % RECORD_ALIGNMENT != 0 {
                return Err(AndaleError::InvalidRecordSize {
                    offset: cursor,
                    size: record_size,
                });
            }
            let end = cursor
                .checked_add(record_size)
                .ok_or(AndaleError::IntegerOverflow)?;
            if end > declared_size {
                return Err(AndaleError::RecordOutsideFile {
                    offset: cursor,
                    end,
                    file_size: declared_size,
                });
            }
            let data_start = align_up(
                cursor
                    .checked_add(DATA_HEADER_SIZE)
                    .ok_or(AndaleError::IntegerOverflow)?,
                RECORD_ALIGNMENT,
            )?;
            let data_range = data_start..end;
            if header.data_type == HIERARCHY_TYPE {
                let parsed = AndaleHierarchy::parse(&bytes[data_range.clone()])?;
                if hierarchy.replace(parsed).is_some() {
                    return Err(AndaleError::DuplicateHierarchy);
                }
            } else if header.data_type == CLIP_TYPE {
                clips.push(AndaleClip::parse(
                    cursor,
                    &header,
                    &bytes[data_range.clone()],
                )?);
            } else if header.data_type == POSE_TYPE {
                poses.push(AndalePose::parse(
                    cursor,
                    header.codec,
                    &bytes[data_range.clone()],
                )?);
            } else if header.data_type == PHYSICS_PARAMETERS_TYPE {
                physics_parameters.push(parse_physics_parameters(
                    cursor,
                    &header,
                    &bytes[data_range.clone()],
                )?);
            } else if (BLEND_SPACE_TYPE..=SELECTOR_TYPE).contains(&header.data_type) {
                controllers.push(parse_animation_controller(
                    cursor,
                    &header,
                    &bytes[data_range.clone()],
                )?);
            }
            records.push(AndaleRecord {
                offset: cursor,
                header,
                data_range,
            });
            cursor = end;
        }

        let hierarchy = hierarchy.ok_or(AndaleError::MissingHierarchy)?;
        for pose in &poses {
            pose.validate_against(&hierarchy)?;
        }
        Ok(Self {
            file_header,
            records,
            hierarchy,
            clips,
            poses,
            physics_parameters,
            controllers,
        })
    }
}

fn parse_and_validate_file_header(bytes: &[u8]) -> Result<(AndaleDataHeader, usize), AndaleError> {
    let file_header = parse_data_header(bytes, 0)?;
    if file_header.data_type != FILE_HEADER_TYPE {
        return Err(AndaleError::UnexpectedFileHeaderType(file_header.data_type));
    }
    let declared_size =
        usize::try_from(file_header.size).map_err(|_| AndaleError::IntegerOverflow)?;
    if declared_size > bytes.len() {
        return Err(AndaleError::FileSizeMismatch {
            declared: declared_size,
            actual: bytes.len(),
        });
    }
    if bytes[declared_size..]
        .iter()
        .any(|byte| !matches!(*byte, 0 | 0xcd))
    {
        return Err(AndaleError::NonzeroTrailingBytes {
            declared: declared_size,
            actual: bytes.len(),
        });
    }
    Ok((file_header, declared_size))
}

impl AndaleClip {
    fn parse(
        record_offset: usize,
        header: &AndaleDataHeader,
        bytes: &[u8],
    ) -> Result<Self, AndaleError> {
        require_range(bytes, 0, 56)?;
        let mut fixed_words = [0_u32; 8];
        for (index, word) in fixed_words.iter_mut().enumerate() {
            *word = read_be_u32(bytes, index * 4)?;
        }
        let frames_per_second = read_be_f32(bytes, 32)?;
        let frame_count = read_be_f32(bytes, 36)?;
        let base_speed = read_be_f32(bytes, 40)?;
        if !frames_per_second.is_finite()
            || frames_per_second <= 0.0
            || !frame_count.is_finite()
            || frame_count < 1.0
            || !base_speed.is_finite()
            || base_speed <= 0.0
        {
            return Err(AndaleError::InvalidClipTiming {
                record_offset,
                frames_per_second: frames_per_second.to_bits(),
                frame_count: frame_count.to_bits(),
                base_speed: base_speed.to_bits(),
            });
        }
        let flags = read_be_u32(bytes, 52)?;
        let attribute_count = usize::from(bytes[55]);
        let attribute_offset = u32_to_usize(read_be_u32(bytes, 44)?)?;
        let attributes = parse_attributes(bytes, attribute_offset, attribute_count)?;
        let part_count =
            usize::try_from(flags >> 8 & 0x1f).map_err(|_| AndaleError::IntegerOverflow)?;
        let table_end = 56_usize
            .checked_add(
                part_count
                    .checked_mul(4)
                    .ok_or(AndaleError::IntegerOverflow)?,
            )
            .ok_or(AndaleError::IntegerOverflow)?;
        require_range(bytes, 0, table_end)?;
        let encoded_part_offsets = (0..part_count)
            .map(|index| read_be_u32(bytes, 56 + index * 4))
            .collect::<Result<Vec<_>, _>>()?;
        let offsets = encoded_part_offsets
            .iter()
            .map(|encoded| u32_to_usize(encoded & 0x00ff_ffff))
            .collect::<Result<Vec<_>, _>>()?;
        if offsets.first().is_some_and(|offset| *offset < table_end)
            || offsets.windows(2).any(|pair| pair[0] >= pair[1])
            || offsets.last().is_some_and(|offset| *offset >= bytes.len())
        {
            return Err(AndaleError::InvalidClipPartOffsets { record_offset });
        }
        let mut parts = Vec::with_capacity(part_count);
        for index in 0..part_count {
            let start = offsets[index];
            let end = offsets.get(index + 1).copied().unwrap_or({
                if attribute_count != 0 && attribute_offset > start {
                    attribute_offset
                } else {
                    bytes.len()
                }
            });
            parts.push(parse_clip_part(
                bytes,
                record_offset,
                index,
                header.codec,
                encoded_part_offsets[index],
                start..end,
            )?);
        }
        Ok(Self {
            record_offset,
            name: header.name.decode()?,
            guid: header.guid,
            codec: header.codec,
            fixed_words,
            frames_per_second,
            frame_count,
            base_speed,
            unknown_2c: read_be_u32(bytes, 44)?,
            unknown_30: read_be_u32(bytes, 48)?,
            flags,
            attributes,
            encoded_part_offsets,
            parts,
        })
    }
}

impl AndalePose {
    fn parse(record_offset: usize, codec: u32, bytes: &[u8]) -> Result<Self, AndaleError> {
        let part_count =
            usize::try_from(read_be_u32(bytes, 0)?).map_err(|_| AndaleError::IntegerOverflow)?;
        let offsets_end = 4_usize
            .checked_add(
                part_count
                    .checked_mul(4)
                    .ok_or(AndaleError::IntegerOverflow)?,
            )
            .ok_or(AndaleError::IntegerOverflow)?;
        require_range(bytes, 0, offsets_end)?;
        let encoded_offsets = (0..part_count)
            .map(|index| read_be_u32(bytes, 4 + index * 4))
            .collect::<Result<Vec<_>, _>>()?;
        let offsets = encoded_offsets
            .iter()
            .map(|encoded| u32_to_usize(encoded & 0x00ff_ffff))
            .collect::<Result<Vec<_>, _>>()?;
        if offsets.first().is_some_and(|offset| *offset < offsets_end) {
            return Err(AndaleError::PosePartOffsetsOverlapTable);
        }
        if offsets.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(AndaleError::PosePartOffsetsNotIncreasing);
        }

        let mut parts = Vec::with_capacity(part_count);
        for index in 0..part_count {
            let start = offsets[index];
            let end = offsets.get(index + 1).copied().unwrap_or(bytes.len());
            require_range(bytes, start, 16)?;
            if end > bytes.len() || end < start + 16 {
                return Err(AndaleError::PosePartOutsideRecord {
                    index,
                    start,
                    end,
                    record_size: bytes.len(),
                });
            }
            let compressed_data_size = read_be_u32(bytes, start + 4)?;
            let compression_header_offset = read_be_u32(bytes, start + 8)?;
            let compressed_data_offset = read_be_u32(bytes, start + 12)?;
            let compression_start = start
                .checked_add(u32_to_usize(compression_header_offset)?)
                .ok_or(AndaleError::IntegerOverflow)?;
            let data_start = start
                .checked_add(u32_to_usize(compressed_data_offset)?)
                .ok_or(AndaleError::IntegerOverflow)?;
            let data_end = data_start
                .checked_add(u32_to_usize(compressed_data_size)?)
                .ok_or(AndaleError::IntegerOverflow)?;
            if compression_start < start + 16
                || compression_start > data_start
                || data_end > end
                || bytes[data_end..end].iter().any(|byte| *byte != 0xcd)
            {
                return Err(AndaleError::InvalidAnimationPartLayout {
                    index,
                    start,
                    end,
                    compression_start,
                    data_start,
                    data_end,
                });
            }
            let compression_header = &bytes[compression_start..data_start];
            let compression = parse_compression_header(codec, compression_header)?;
            let compressed_data = &bytes[data_start..data_end];
            let vbr_prelude = match &compression {
                AndaleCompressionHeader::Vbr(header) => {
                    Some(header.decode_prelude(compressed_data)?)
                }
                AndaleCompressionHeader::RawDiscretize(_)
                | AndaleCompressionHeader::Unknown { .. } => None,
            };
            parts.push(AndaleAnimationPart {
                header: read_be_u32(bytes, start)?,
                encoded_offset: encoded_offsets[index],
                part_name_index: (encoded_offsets[index] >> 24) as u8,
                compressed_data_size,
                compression_header_offset,
                compressed_data_offset,
                compression,
                compression_header: compression_header.to_vec(),
                compressed_data: compressed_data.to_vec(),
                padding: bytes[data_end..end].to_vec(),
                vbr_prelude,
            });
        }
        Ok(Self {
            record_offset,
            parts,
        })
    }

    fn validate_against(&self, hierarchy: &AndaleHierarchy) -> Result<(), AndaleError> {
        if self.parts.len() != hierarchy.parts.len() {
            return Err(AndaleError::PoseHierarchyPartCountMismatch {
                pose: self.parts.len(),
                hierarchy: hierarchy.parts.len(),
            });
        }
        for (index, (pose, hierarchy)) in self.parts.iter().zip(&hierarchy.parts).enumerate() {
            let expected_bones =
                usize::try_from(hierarchy.bone_count).map_err(|_| AndaleError::IntegerOverflow)?;
            if pose.pose_part_index() != index
                || pose.bone_count() != expected_bones
                || pose.packed_header_size() != pose.compression_header.len()
            {
                return Err(AndaleError::PoseHierarchyPartMismatch {
                    index,
                    pose_index: pose.pose_part_index(),
                    pose_bones: pose.bone_count(),
                    hierarchy_bones: expected_bones,
                });
            }
            if expected_bones == 0 || expected_bones > 32 {
                return Err(AndaleError::PoseMaskBoneCountUnsupported {
                    index,
                    bones: expected_bones,
                });
            }
            if let AndaleCompressionHeader::RawDiscretize(raw) = &pose.compression {
                if u32::from(raw.frame_stride) != pose.compressed_data_size
                    || usize::from(raw.last_bone_index) + 1 != expected_bones
                {
                    return Err(AndaleError::PoseRawHeaderMismatch { index });
                }
                let valid_mask = if expected_bones == 32 {
                    u32::MAX
                } else {
                    (1_u32 << expected_bones) - 1
                };
                if raw.channel_masks.iter().any(|mask| mask & !valid_mask != 0) {
                    return Err(AndaleError::PoseChannelMaskOutsidePart { index });
                }
            }
        }
        Ok(())
    }
}

impl AndaleHierarchy {
    fn parse(bytes: &[u8]) -> Result<Self, AndaleError> {
        let bone_count = usize::from(read_be_u16(bytes, 0)?);
        let has_trajectory = read_be_u16(bytes, 2)? != 0;
        let part_count =
            usize::try_from(read_be_u32(bytes, 4)?).map_err(|_| AndaleError::IntegerOverflow)?;
        let bone_tables_size = bone_count
            .checked_mul(8)
            .ok_or(AndaleError::IntegerOverflow)?;
        let parts_start = 8_usize
            .checked_add(bone_tables_size)
            .ok_or(AndaleError::IntegerOverflow)?;
        let parts_size = part_count
            .checked_mul(HIERARCHY_PART_SIZE)
            .ok_or(AndaleError::IntegerOverflow)?;
        let names_start = parts_start
            .checked_add(parts_size)
            .ok_or(AndaleError::IntegerOverflow)?;
        let names_size = bone_count
            .checked_mul(5 * size_of::<u32>())
            .ok_or(AndaleError::IntegerOverflow)?;
        let expected_size = names_start
            .checked_add(names_size)
            .ok_or(AndaleError::IntegerOverflow)?;
        if expected_size > bytes.len() {
            return Err(AndaleError::HierarchyOutsideRecord {
                expected: expected_size,
                actual: bytes.len(),
            });
        }

        let mut parents = Vec::with_capacity(bone_count);
        let mut bone_metadata = Vec::with_capacity(bone_count);
        for index in 0..bone_count {
            parents.push(read_be_i32(bytes, 8 + index * 4)?);
            bone_metadata.push(read_be_i32(bytes, 8 + bone_count * 4 + index * 4)?);
        }

        let mut parts = Vec::with_capacity(part_count);
        for index in 0..part_count {
            let offset = parts_start + index * HIERARCHY_PART_SIZE;
            let packed_name = parse_fast_string::<6>(bytes, offset)?;
            parts.push(AndaleHierarchyPart {
                name: packed_name.decode()?,
                packed_name,
                bone_count: read_be_u32(bytes, offset + 24)?,
                bone_offset: read_be_u32(bytes, offset + 28)?,
                is_animated: read_be_u16(bytes, offset + 32)? != 0,
                name_index: read_be_u16(bytes, offset + 34)?,
            });
        }

        let mut bone_names = Vec::with_capacity(bone_count);
        let mut packed_bone_names = Vec::with_capacity(bone_count);
        for index in 0..bone_count {
            let packed = parse_fast_string::<5>(bytes, names_start + index * 20)?;
            bone_names.push(packed.decode()?);
            packed_bone_names.push(packed);
        }

        Ok(Self {
            has_trajectory,
            parents,
            bone_metadata,
            parts,
            bone_names,
            packed_bone_names,
        })
    }
}

fn parse_physics_parameters(
    record_offset: usize,
    header: &AndaleDataHeader,
    bytes: &[u8],
) -> Result<AndalePhysicsParameters, AndaleError> {
    require_range(bytes, 0, 16)?;
    let bone_count = u32_to_usize(read_be_u32(bytes, 0)?)?;
    let bones_size = bone_count
        .checked_mul(112)
        .ok_or(AndaleError::IntegerOverflow)?;
    require_range(bytes, 16, bones_size)?;
    let mut bones = Vec::with_capacity(bone_count);
    for index in 0..bone_count {
        let offset = 16 + index * 112;
        let packed_name = parse_fast_string::<5>(bytes, offset + 92)?;
        bones.push(AndalePhysicsBone {
            parent_orientation: parse_float4(bytes, offset)?,
            joint_orientation: parse_float4(bytes, offset + 16)?,
            size: parse_float4(bytes, offset + 32)?,
            orientation: parse_float4(bytes, offset + 48)?,
            position: parse_float4(bytes, offset + 64)?,
            twist: read_be_f32(bytes, offset + 80)?,
            swing: read_be_f32(bytes, offset + 84)?,
            mass: read_be_f32(bytes, offset + 88)?,
            name: packed_name.decode()?,
            packed_name,
        });
    }
    Ok(AndalePhysicsParameters {
        record_offset,
        name: header.name.decode()?,
        guid: header.guid,
        bones,
    })
}

fn parse_animation_controller(
    record_offset: usize,
    header: &AndaleDataHeader,
    bytes: &[u8],
) -> Result<AndaleAnimationController, AndaleError> {
    match header.data_type {
        BLEND_SPACE_TYPE => Ok(AndaleAnimationController::BlendSpace(parse_blend_space(
            record_offset,
            header,
            bytes,
        )?)),
        PHASE_BLEND_TYPE => Ok(AndaleAnimationController::PhaseBlend(parse_phase_blend(
            record_offset,
            header,
            bytes,
        )?)),
        CONSTRUCTION_SELECTOR_TYPE => Ok(AndaleAnimationController::ConstructionSelector(
            parse_construction_selector(record_offset, header, bytes)?,
        )),
        SEQUENCE_TYPE => Ok(AndaleAnimationController::Sequence(parse_sequence(
            record_offset,
            header,
            bytes,
        )?)),
        SELECTOR_TYPE => Ok(AndaleAnimationController::Selector(parse_selector(
            record_offset,
            header,
            bytes,
        )?)),
        data_type => Err(AndaleError::UnexpectedControllerType(data_type)),
    }
}

fn parse_blend_space(
    record_offset: usize,
    header: &AndaleDataHeader,
    bytes: &[u8],
) -> Result<AndaleBlendSpace, AndaleError> {
    require_range(bytes, 0, 20)?;
    let animation_count = u32_to_usize(read_be_u32(bytes, 0)?)?;
    let animation_names_offset = u32_to_usize(read_be_u32(bytes, 4)?)?;
    let volume_set_count = u32_to_usize(read_be_u32(bytes, 8)?)?;
    let volume_sets_offset = u32_to_usize(read_be_u32(bytes, 12)?)?;
    let animation_names =
        parse_fast_string_array::<6>(bytes, animation_names_offset, animation_count)?;

    let mut volume_sets = Vec::with_capacity(volume_set_count);
    let mut volume_set_offset = volume_sets_offset;
    for _ in 0..volume_set_count {
        require_range(bytes, volume_set_offset, 44)?;
        let size = u32_to_usize(read_be_u32(bytes, volume_set_offset)?)?;
        if size < 44 {
            return Err(AndaleError::InvalidBlendSpaceElementSize(size));
        }
        require_range(bytes, volume_set_offset, size)?;
        let volume_set = &bytes[volume_set_offset..volume_set_offset + size];
        let dimensions = read_be_u32(volume_set, 28)?;
        let dimension_names_offset = u32_to_usize(read_be_u32(volume_set, 32)?)?;
        let dimension_count = u32_to_usize(dimensions)?;
        let dimension_bytes = dimension_count
            .checked_mul(20)
            .ok_or(AndaleError::IntegerOverflow)?;
        require_range(volume_set, dimension_names_offset, dimension_bytes)?;
        let packed_dimension_names = (0..dimension_count)
            .map(|index| parse_fast_string::<5>(volume_set, dimension_names_offset + index * 20))
            .collect::<Result<Vec<_>, _>>()?;
        let dimension_names = packed_dimension_names
            .iter()
            .map(PackedFastString::decode)
            .collect::<Result<Vec<_>, _>>()?;
        let volume_count = u32_to_usize(read_be_u32(volume_set, 36)?)?;
        let mut volume_offset = u32_to_usize(read_be_u32(volume_set, 40)?)?;
        let mut volumes = Vec::with_capacity(volume_count);
        for _ in 0..volume_count {
            require_range(volume_set, volume_offset, 20)?;
            let volume_size = u32_to_usize(read_be_u32(volume_set, volume_offset)?)?;
            if volume_size < 20 {
                return Err(AndaleError::InvalidBlendSpaceElementSize(volume_size));
            }
            require_range(volume_set, volume_offset, volume_size)?;
            let volume = &volume_set[volume_offset..volume_offset + volume_size];
            let coordinate_count = u32_to_usize(read_be_u32(volume, 16)?)?;
            let coordinate_bytes = coordinate_count
                .checked_mul(4)
                .ok_or(AndaleError::IntegerOverflow)?;
            require_range(volume, 20, coordinate_bytes)?;
            let matrix_coordinates = (0..coordinate_count)
                .map(|index| read_be_f32(volume, 20 + index * 4))
                .collect::<Result<Vec<_>, _>>()?;
            volumes.push(AndaleBlendSpaceVolume {
                animation_indexes_offset: read_be_u32(volume, 4)?,
                heights_offset: read_be_u32(volume, 8)?,
                normals_offset: read_be_u32(volume, 12)?,
                matrix_coordinates,
                serialized_data: volume.to_vec(),
                serialized_following_word: bytes
                    .get(volume_set_offset + volume_offset + volume_size..)
                    .and_then(|tail| tail.get(..4))
                    .and_then(|word| word.try_into().ok()),
            });
            volume_offset = volume_offset
                .checked_add(volume_size)
                .ok_or(AndaleError::IntegerOverflow)?;
        }
        volume_sets.push(AndaleBlendSpaceVolumeSet {
            name: parse_fast_string::<6>(volume_set, 4)?.decode()?,
            dimensions,
            dimension_names,
            packed_dimension_names,
            serialized_size: read_be_u32(volume_set, 0)?,
            volumes,
        });
        volume_set_offset = volume_set_offset
            .checked_add(size)
            .ok_or(AndaleError::IntegerOverflow)?;
    }

    Ok(AndaleBlendSpace {
        record_offset,
        name: header.name.decode()?,
        guid: header.guid,
        maximum_dimensions: read_be_u32(bytes, 16)?,
        animation_names,
        volume_sets,
    })
}

fn parse_phase_blend(
    record_offset: usize,
    header: &AndaleDataHeader,
    bytes: &[u8],
) -> Result<AndalePhaseBlend, AndaleError> {
    require_range(bytes, 0, 36)?;
    let attribute_count = u32_to_usize(read_be_u32(bytes, 0)?)?;
    let mut attribute_offset = u32_to_usize(read_be_u32(bytes, 4)?)?;
    let mut attributes = Vec::with_capacity(attribute_count);
    for _ in 0..attribute_count {
        let (attribute, next) = parse_attribute(bytes, attribute_offset)?;
        attributes.push(attribute);
        attribute_offset = next;
    }
    let child_count = u32_to_usize(read_be_u32(bytes, 8)?)?;
    Ok(AndalePhaseBlend {
        record_offset,
        name: header.name.decode()?,
        guid: header.guid,
        attributes,
        blend_attribute: parse_fast_string::<5>(bytes, 16)?.decode()?,
        children: parse_fast_string_array::<6>(bytes, 36, child_count)?,
    })
}

fn parse_attributes(
    bytes: &[u8],
    mut offset: usize,
    count: usize,
) -> Result<Vec<AndaleAttribute>, AndaleError> {
    let mut attributes = Vec::with_capacity(count);
    for _ in 0..count {
        let (attribute, next) = parse_attribute(bytes, offset)?;
        attributes.push(attribute);
        offset = next;
    }
    Ok(attributes)
}

fn parse_attribute(bytes: &[u8], offset: usize) -> Result<(AndaleAttribute, usize), AndaleError> {
    require_range(bytes, offset, 36)?;
    let size = u32_to_usize(read_be_u32(bytes, offset)?)?;
    if size < 36 {
        return Err(AndaleError::InvalidAttributeSize(size));
    }
    require_range(bytes, offset, size)?;
    let end = offset
        .checked_add(size)
        .ok_or(AndaleError::IntegerOverflow)?;
    let value_offset = align_up(
        offset.checked_add(36).ok_or(AndaleError::IntegerOverflow)?,
        16,
    )?;
    if value_offset > end {
        return Err(AndaleError::InvalidAttributeSize(size));
    }
    let attribute_type = bytes[offset + 32];
    let value = match attribute_type {
        0 => {
            require_range_before(bytes, value_offset, 4, end)?;
            AndaleAttributeValue::Float(read_be_f32(bytes, value_offset)?)
        }
        1 => {
            require_range_before(bytes, value_offset, 16, end)?;
            AndaleAttributeValue::Vector(parse_float4(bytes, value_offset)?)
        }
        3 => {
            require_range_before(bytes, value_offset, 24, end)?;
            let mut value = [0_u8; 24];
            value.copy_from_slice(&bytes[value_offset..value_offset + 24]);
            AndaleAttributeValue::Bytes24(value)
        }
        attribute_type => AndaleAttributeValue::Unsupported {
            attribute_type,
            bytes: bytes[value_offset..end].to_vec(),
        },
    };
    Ok((
        AndaleAttribute {
            begin: read_be_f32(bytes, offset + 4)?,
            end: read_be_f32(bytes, offset + 8)?,
            name: parse_fast_string::<5>(bytes, offset + 12)?.decode()?,
            attribute_type,
            value,
        },
        end,
    ))
}

fn parse_construction_selector(
    record_offset: usize,
    header: &AndaleDataHeader,
    bytes: &[u8],
) -> Result<AndaleConstructionSelector, AndaleError> {
    require_range(bytes, 0, 32)?;
    let child_names_offset = u32_to_usize(read_be_u32(bytes, 20)?)?;
    let option_values_offset = u32_to_usize(read_be_u32(bytes, 24)?)?;
    let option_count = u32_to_usize(read_be_u32(bytes, 28)?)?;
    let children = parse_fast_string_array::<6>(bytes, child_names_offset, option_count)?;
    let values = parse_fast_string_array::<5>(bytes, option_values_offset, option_count)?;
    Ok(AndaleConstructionSelector {
        record_offset,
        name: header.name.decode()?,
        guid: header.guid,
        construction_attribute: parse_fast_string::<5>(bytes, 0)?.decode()?,
        options: values
            .into_iter()
            .zip(children)
            .map(|(value, child)| AndaleConstructionOption { value, child })
            .collect(),
    })
}

fn parse_sequence(
    record_offset: usize,
    header: &AndaleDataHeader,
    bytes: &[u8],
) -> Result<AndaleSequence, AndaleError> {
    require_range(bytes, 0, 4)?;
    let child_count = usize::from(read_be_u16(bytes, 0)?);
    let child_bytes = child_count
        .checked_mul(28)
        .ok_or(AndaleError::IntegerOverflow)?;
    require_range(bytes, 4, child_bytes)?;
    let mut children = Vec::with_capacity(child_count);
    for index in 0..child_count {
        let offset = 4 + index * 28;
        children.push(AndaleSequenceChild {
            name: parse_fast_string::<6>(bytes, offset)?.decode()?,
            stage: parse_locomotion_stage(read_be_u32(bytes, offset + 24)?),
        });
    }
    Ok(AndaleSequence {
        record_offset,
        name: header.name.decode()?,
        guid: header.guid,
        looping: bytes[2] != 0,
        window: bytes[3],
        children,
    })
}

fn parse_selector(
    record_offset: usize,
    header: &AndaleDataHeader,
    bytes: &[u8],
) -> Result<AndaleSelector, AndaleError> {
    require_range(bytes, 0, 4)?;
    let child_count = u32_to_usize(read_be_u32(bytes, 0)?)?;
    let child_bytes = child_count
        .checked_mul(44)
        .ok_or(AndaleError::IntegerOverflow)?;
    require_range(bytes, 4, child_bytes)?;
    let mut children = Vec::with_capacity(child_count);
    for index in 0..child_count {
        let offset = 4 + index * 44;
        children.push(AndaleSelectorChild {
            name: parse_fast_string::<6>(bytes, offset)?.decode()?,
            attribute: parse_fast_string::<5>(bytes, offset + 24)?.decode()?,
        });
    }
    Ok(AndaleSelector {
        record_offset,
        name: header.name.decode()?,
        guid: header.guid,
        children,
    })
}

const fn parse_locomotion_stage(value: u32) -> AndaleLocomotionStage {
    match value {
        0 => AndaleLocomotionStage::ContactToPassRight,
        1 => AndaleLocomotionStage::PassToAirRight,
        2 => AndaleLocomotionStage::AirRight,
        3 => AndaleLocomotionStage::ContactToPassLeft,
        4 => AndaleLocomotionStage::PassToAirLeft,
        5 => AndaleLocomotionStage::AirLeft,
        0x1000_0000 => AndaleLocomotionStage::Last,
        value => AndaleLocomotionStage::Unknown(value),
    }
}

fn parse_float4(bytes: &[u8], offset: usize) -> Result<[f32; 4], AndaleError> {
    Ok([
        read_be_f32(bytes, offset)?,
        read_be_f32(bytes, offset + 4)?,
        read_be_f32(bytes, offset + 8)?,
        read_be_f32(bytes, offset + 12)?,
    ])
}

fn parse_fast_string_array<const WORDS: usize>(
    bytes: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<String>, AndaleError> {
    let stride = WORDS
        .checked_mul(size_of::<u32>())
        .ok_or(AndaleError::IntegerOverflow)?;
    let size = count
        .checked_mul(stride)
        .ok_or(AndaleError::IntegerOverflow)?;
    require_range(bytes, offset, size)?;
    (0..count)
        .map(|index| parse_fast_string::<WORDS>(bytes, offset + index * stride)?.decode())
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AndaleError {
    Truncated {
        offset: usize,
        size: usize,
    },
    IntegerOverflow,
    UnexpectedFileHeaderType(u32),
    FileSizeMismatch {
        declared: usize,
        actual: usize,
    },
    NonzeroTrailingBytes {
        declared: usize,
        actual: usize,
    },
    InvalidRecordSize {
        offset: usize,
        size: usize,
    },
    RecordOutsideFile {
        offset: usize,
        end: usize,
        file_size: usize,
    },
    MissingHierarchy,
    DuplicateHierarchy,
    UnexpectedControllerType(u32),
    InvalidBlendSpaceElementSize(usize),
    InvalidAttributeSize(usize),
    ControllerNotFound(String),
    ControllerBranchOutsideChildren {
        controller: String,
        branch: usize,
        children: usize,
    },
    ControllerTerminalIsNotClip(String),
    ControllerCycle(String),
    ControllerIsNotPhaseBlend(String),
    MissingPhaseBlendAttribute {
        controller: String,
        attribute: String,
    },
    NoExactPhaseBlendCoordinate {
        controller: String,
        attribute: String,
        value: u32,
    },
    InvalidNormalizedAnimationTime(u32),
    InvalidAnimationTimeTransition {
        previous: u32,
        current: u32,
    },
    InvalidPhaseBlendCullThreshold(u32),
    RuntimeAttributeTypeMismatch(String),
    ClipPartFrameCountMismatch {
        clip: String,
    },
    PhaseBlendNeedsTwoChildren {
        controller: String,
        children: usize,
    },
    ClipScalarAttributeNotFound {
        clip: String,
        attribute: String,
    },
    PhaseBlendPoseSizeMismatch {
        lower: usize,
        upper: usize,
    },
    HierarchyOutsideRecord {
        expected: usize,
        actual: usize,
    },
    PosePartOffsetsOverlapTable,
    PosePartOffsetsNotIncreasing,
    PosePartOutsideRecord {
        index: usize,
        start: usize,
        end: usize,
        record_size: usize,
    },
    InvalidAnimationPartLayout {
        index: usize,
        start: usize,
        end: usize,
        compression_start: usize,
        data_start: usize,
        data_end: usize,
    },
    UnexpectedRawCompressionHeaderSize(usize),
    NotRawDiscretizeCompression,
    UnsupportedRawPoseLayout {
        channel_masks: [u32; 6],
        format_flags: [u8; 2],
    },
    RawPoseDataSizeMismatch {
        expected: usize,
        actual: usize,
    },
    RawPosePartOutsideHierarchy {
        offset: usize,
        bones: usize,
        hierarchy_bones: usize,
    },
    VbrConstantPaletteIndexOutsidePalette {
        index: u8,
        palette_size: usize,
    },
    VbrConstantMapLengthMismatch {
        expected: usize,
        actual: usize,
    },
    VbrFrameBlockIndexOutsideTable {
        index: usize,
        block_count: usize,
    },
    VbrSignificanceBitCountMismatch {
        expected: usize,
        actual: usize,
    },
    VbrCoefficientWidthTableMisaligned(usize),
    VbrCoefficientPayloadSizeMismatch {
        expected: usize,
        actual: usize,
    },
    NotVbrCompression,
    MissingVbrPrelude,
    VbrFrameOutsideAnimation {
        frame: usize,
        frame_count: usize,
    },
    VbrSqtChannelLayout(Vec<u8>),
    VbrSqtBoneCountMismatch {
        channel: usize,
        expected: usize,
        actual: usize,
    },
    VbrCoefficientRowCountMismatch {
        expected: usize,
        actual: usize,
    },
    VbrSqtChannelDataTruncated(usize),
    InvalidVbrBlend(u32),
    VbrInterpolatedFrameSizeMismatch {
        frame0: usize,
        frame1: usize,
    },
    ZeroLengthVbrQuaternion,
    InvalidClipTiming {
        record_offset: usize,
        frames_per_second: u32,
        frame_count: u32,
        base_speed: u32,
    },
    InvalidClipPartOffsets {
        record_offset: usize,
    },
    InvalidClipPartLayout {
        record_offset: usize,
        index: usize,
    },
    ClipHierarchyPartCountMismatch {
        clip: usize,
        hierarchy: usize,
    },
    ClipHierarchyPartMismatch {
        index: usize,
        clip_bones: usize,
        hierarchy_bones: usize,
    },
    ClipPartOutsideHierarchy {
        index: usize,
        offset: usize,
        bones: usize,
        hierarchy_bones: usize,
    },
    PoseHierarchyPartCountMismatch {
        pose: usize,
        hierarchy: usize,
    },
    PoseHierarchyPartMismatch {
        index: usize,
        pose_index: usize,
        pose_bones: usize,
        hierarchy_bones: usize,
    },
    PoseChannelMaskOutsidePart {
        index: usize,
    },
    PoseRawHeaderMismatch {
        index: usize,
    },
    PoseMaskBoneCountUnsupported {
        index: usize,
        bones: usize,
    },
    FastStringTooLong {
        length: usize,
        capacity: usize,
    },
    InvalidFastStringCharacter(u8),
    InvalidFastStringWord(u32),
    NonCanonicalFastString(Vec<u32>),
}

impl fmt::Display for AndaleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Andale database: {self:?}")
    }
}

impl std::error::Error for AndaleError {}

fn parse_clip_part(
    bytes: &[u8],
    record_offset: usize,
    index: usize,
    codec: u32,
    encoded_offset: u32,
    part_range: Range<usize>,
) -> Result<AndaleClipPart, AndaleError> {
    let start = part_range.start;
    let end = part_range.end;
    require_range(bytes, start, 16)?;
    let compressed_data_size = read_be_u32(bytes, start + 4)?;
    let compression_header_offset = read_be_u32(bytes, start + 8)?;
    let compressed_data_offset = read_be_u32(bytes, start + 12)?;
    let compression_start = start
        .checked_add(u32_to_usize(compression_header_offset)?)
        .ok_or(AndaleError::IntegerOverflow)?;
    let data_start = start
        .checked_add(u32_to_usize(compressed_data_offset)?)
        .ok_or(AndaleError::IntegerOverflow)?;
    let data_end = data_start
        .checked_add(u32_to_usize(compressed_data_size)?)
        .ok_or(AndaleError::IntegerOverflow)?;
    if end > bytes.len()
        || compression_start < start + 16
        || compression_start > data_start
        || data_end > end
    {
        return Err(AndaleError::InvalidClipPartLayout {
            record_offset,
            index,
        });
    }
    let compression_header = &bytes[compression_start..data_start];
    let compression = parse_compression_header(codec, compression_header)?;
    let compressed_data = &bytes[data_start..data_end];
    let vbr_prelude = match &compression {
        AndaleCompressionHeader::Vbr(header) => Some(header.decode_prelude(compressed_data)?),
        AndaleCompressionHeader::RawDiscretize(_) | AndaleCompressionHeader::Unknown { .. } => None,
    };
    Ok(AndaleClipPart {
        header: read_be_u32(bytes, start)?,
        encoded_offset,
        part_name_index: (encoded_offset >> 24) as u8,
        compressed_data_size,
        compression_header_offset,
        compressed_data_offset,
        compression_header_size: data_start - compression_start,
        trailing_bytes_size: end - data_end,
        compression,
        compression_header: compression_header.to_vec(),
        compressed_data: compressed_data.to_vec(),
        trailing_data: bytes[data_end..end].to_vec(),
        vbr_prelude,
    })
}

fn parse_data_header(bytes: &[u8], offset: usize) -> Result<AndaleDataHeader, AndaleError> {
    Ok(AndaleDataHeader {
        size: read_be_u32(bytes, offset)?,
        data_type: read_be_u32(bytes, offset + 4)?,
        codec: read_be_u32(bytes, offset + 8)?,
        guid: read_be_u32(bytes, offset + 12)?,
        name: parse_fast_string(bytes, offset + 16)?,
    })
}

fn parse_fast_string<const WORDS: usize>(
    bytes: &[u8],
    offset: usize,
) -> Result<PackedFastString<WORDS>, AndaleError> {
    let mut words = [0_u32; WORDS];
    for (index, word) in words.iter_mut().enumerate() {
        *word = read_be_u32(bytes, offset + index * 4)?;
    }
    Ok(PackedFastString { words })
}

fn parse_raw_compression_header(bytes: &[u8]) -> Result<AndaleRawCompressionHeader, AndaleError> {
    if bytes.len() != 32 {
        return Err(AndaleError::UnexpectedRawCompressionHeaderSize(bytes.len()));
    }
    let mut channel_masks = [0_u32; 6];
    for (index, mask) in channel_masks.iter_mut().enumerate() {
        *mask = read_be_u32(bytes, index * 4)?;
    }
    Ok(AndaleRawCompressionHeader {
        channel_masks,
        frame_stride: read_be_u16(bytes, 24)?,
        last_bone_index: bytes[26],
        format_flags: [bytes[27], bytes[28]],
        padding: [bytes[29], bytes[30], bytes[31]],
    })
}

fn parse_compression_header(
    codec: u32,
    bytes: &[u8],
) -> Result<AndaleCompressionHeader, AndaleError> {
    match codec {
        RAW_DISCRETIZE_CODEC => Ok(AndaleCompressionHeader::RawDiscretize(
            parse_raw_compression_header(bytes)?,
        )),
        VBR_CODEC => Ok(AndaleCompressionHeader::Vbr(parse_vbr_compression_header(
            bytes,
        )?)),
        codec => Ok(AndaleCompressionHeader::Unknown {
            codec,
            bytes: bytes.to_vec(),
        }),
    }
}

fn parse_vbr_compression_header(bytes: &[u8]) -> Result<AndaleVbrCompressionHeader, AndaleError> {
    require_range(bytes, 0, 16)?;
    let frame_count = read_be_u16(bytes, 2)?;
    let channel_block_count = usize::from(bytes[12]);
    let constant_palette_size = usize::from(bytes[13]);
    let palette_end = 16_usize
        .checked_add(
            constant_palette_size
                .checked_mul(4)
                .ok_or(AndaleError::IntegerOverflow)?,
        )
        .ok_or(AndaleError::IntegerOverflow)?;
    let frame_block_count = usize::from(frame_count).div_ceil(8);
    let frame_blocks_end = palette_end
        .checked_add(
            frame_block_count
                .checked_mul(2)
                .ok_or(AndaleError::IntegerOverflow)?,
        )
        .ok_or(AndaleError::IntegerOverflow)?;
    let channel_blocks_start = align_up(frame_blocks_end, 16)?;
    let channel_blocks_end = channel_blocks_start
        .checked_add(
            channel_block_count
                .checked_mul(16)
                .ok_or(AndaleError::IntegerOverflow)?,
        )
        .ok_or(AndaleError::IntegerOverflow)?;
    require_range(bytes, 0, channel_blocks_end)?;

    let constant_palette = (0..constant_palette_size)
        .map(|index| read_be_f32(bytes, 16 + index * 4))
        .collect::<Result<Vec<_>, _>>()?;
    let frame_block_sizes = (0..frame_block_count)
        .map(|index| read_be_u16(bytes, palette_end + index * 2))
        .collect::<Result<Vec<_>, _>>()?;
    let channel_blocks = (0..channel_block_count)
        .map(|index| {
            let offset = channel_blocks_start + index * 16;
            Ok(AndaleVbrChannelBlock {
                animated_entry_count: read_be_u16(bytes, offset)?,
                constant_entry_count: read_be_u16(bytes, offset + 2)?,
                minimum: read_be_f32(bytes, offset + 4)?,
                maximum: read_be_f32(bytes, offset + 8)?,
                component_count: bytes[offset + 12],
                reserved: [bytes[offset + 13], bytes[offset + 14], bytes[offset + 15]],
            })
        })
        .collect::<Result<Vec<_>, AndaleError>>()?;

    Ok(AndaleVbrCompressionHeader {
        constant_channel_map_run_count: read_be_u16(bytes, 0)?,
        frame_count,
        dct_minimum: read_be_f32(bytes, 4)?,
        dct_maximum: read_be_f32(bytes, 8)?,
        channel_blocks,
        constant_palette,
        frame_block_sizes,
        reserved: [bytes[14], bytes[15]],
        alignment_padding: bytes[frame_blocks_end..channel_blocks_start].to_vec(),
        trailing_bytes: bytes[channel_blocks_end..].to_vec(),
    })
}

fn decode_normalized_i16x3(encoded: [u8; 6]) -> [f32; 3] {
    let mut decoded = [0.0_f32; 3];
    for (index, bytes) in encoded.chunks_exact(2).enumerate() {
        decoded[index] = f32::from(i16::from_be_bytes([bytes[0], bytes[1]])) / 32767.0;
    }
    decoded
}

fn decode_fast_string_word(mut word: u32) -> Result<[u32; 6], AndaleError> {
    if word >= FAST_STRING_RADIX.pow(6) {
        return Err(AndaleError::InvalidFastStringWord(word));
    }
    let mut digits = [0_u32; 6];
    for (index, multiplier) in [
        FAST_STRING_RADIX.pow(5),
        FAST_STRING_RADIX.pow(4),
        FAST_STRING_RADIX.pow(3),
        FAST_STRING_RADIX.pow(2),
        FAST_STRING_RADIX,
        1,
    ]
    .into_iter()
    .enumerate()
    {
        digits[index] = word / multiplier;
        word %= multiplier;
    }
    Ok(digits)
}

fn fast_string_digit(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0') + 1),
        b'A'..=b'Z' => Some(u32::from(byte - b'A') + 11),
        b'a'..=b'z' => Some(u32::from(byte - b'a') + 11),
        b'_' => Some(37),
        _ => None,
    }
}

fn fast_string_character(digit: u32) -> char {
    match digit {
        1..=10 => char::from_u32(u32::from(b'0') + digit - 1).unwrap(),
        11..=36 => char::from_u32(u32::from(b'a') + digit - 11).unwrap(),
        37 => '_',
        _ => unreachable!("base-38 digit is validated before conversion"),
    }
}

fn align_up(value: usize, alignment: usize) -> Result<usize, AndaleError> {
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or(AndaleError::IntegerOverflow)
}

fn read_be_u16(bytes: &[u8], offset: usize) -> Result<u16, AndaleError> {
    let field = bytes
        .get(offset..offset + 2)
        .ok_or(AndaleError::Truncated { offset, size: 2 })?;
    Ok(u16::from_be_bytes(field.try_into().unwrap()))
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32, AndaleError> {
    let field = bytes
        .get(offset..offset + 4)
        .ok_or(AndaleError::Truncated { offset, size: 4 })?;
    Ok(u32::from_be_bytes(field.try_into().unwrap()))
}

fn read_be_i32(bytes: &[u8], offset: usize) -> Result<i32, AndaleError> {
    Ok(i32::from_be_bytes(
        read_be_u32(bytes, offset)?.to_be_bytes(),
    ))
}

fn read_be_f32(bytes: &[u8], offset: usize) -> Result<f32, AndaleError> {
    Ok(f32::from_bits(read_be_u32(bytes, offset)?))
}

#[allow(clippy::unnecessary_lazy_evaluations,
    reason = "ARM64 otherwise calls AndaleError drop glue on successful reads")]
fn require_range(bytes: &[u8], offset: usize, size: usize) -> Result<(), AndaleError> {
    let end = offset
        .checked_add(size)
        .ok_or_else(|| AndaleError::IntegerOverflow)?;
    if end > bytes.len() {
        return Err(AndaleError::Truncated { offset, size });
    }
    Ok(())
}

fn require_range_before(
    bytes: &[u8],
    offset: usize,
    size: usize,
    limit: usize,
) -> Result<(), AndaleError> {
    require_range(bytes, offset, size)?;
    let end = offset
        .checked_add(size)
        .ok_or(AndaleError::IntegerOverflow)?;
    if end > limit {
        return Err(AndaleError::Truncated { offset, size });
    }
    Ok(())
}

#[inline]
#[allow(clippy::unnecessary_lazy_evaluations,
    reason = "ARM64 otherwise calls AndaleError drop glue on successful reads")]
fn read_lsb_bits(bytes: &[u8], offset: &mut usize, count: usize) -> Result<u32, AndaleError> {
    let end = offset
        .checked_add(count)
        .ok_or_else(|| AndaleError::IntegerOverflow)?;
    if end > bytes.len() * 8 || count > 32 {
        return Err(AndaleError::Truncated {
            offset: *offset / 8,
            size: count.div_ceil(8),
        });
    }
    // The native VBR unpacker82edbde8/82edc018 reads little-endian
    // byte groups and shifts the requested field. Assemble at most five
    // bounded bytes here rather than visiting every bit individually.
    // All bounds checks and error offsets above retain their original order.
    let first_byte = *offset / 8;
    let shift = *offset & 7;
    let byte_count = (shift + count).div_ceil(8);
    let mut packed = 0_u64;
    if count != 0 {
        for (index, &byte) in bytes[first_byte..first_byte + byte_count].iter().enumerate() {
            packed |= u64::from(byte) << (8 * index);
        }
    }
    let value = ((packed >> shift) & ((1_u64 << count) - 1)) as u32;
    *offset = end;
    Ok(value)
}

fn u32_to_usize(value: u32) -> Result<usize, AndaleError> {
    usize::try_from(value).map_err(|_| AndaleError::IntegerOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vbr_pair_across_a_block_decodes_the_second_frame_for_both_keys() {
        assert_eq!(vbr_pair_first_frame(6, 7), 6);
        assert_eq!(vbr_pair_first_frame(7, 8), 8);
        assert_eq!(vbr_pair_first_frame(8, 9), 8);
        assert_eq!(vbr_pair_first_frame(15, 16), 16);
        assert_eq!(vbr_pair_first_frame(16, 16), 16);
        assert_eq!(vbr_pair_first_frame(0, 1), 0);
    }

    fn write_header(
        bytes: &mut [u8],
        offset: usize,
        size: u32,
        data_type: u32,
        codec: u32,
        name: &str,
    ) {
        bytes[offset..offset + 4].copy_from_slice(&size.to_be_bytes());
        bytes[offset + 4..offset + 8].copy_from_slice(&data_type.to_be_bytes());
        bytes[offset + 8..offset + 12].copy_from_slice(&codec.to_be_bytes());
        let packed = PackedFastString::<6>::encode(name).unwrap();
        for (index, word) in packed.words.into_iter().enumerate() {
            bytes[offset + 16 + index * 4..offset + 20 + index * 4]
                .copy_from_slice(&word.to_be_bytes());
        }
    }

    fn write_fast_string<const WORDS: usize>(bytes: &mut [u8], offset: usize, value: &str) {
        let packed = PackedFastString::<WORDS>::encode(value).unwrap();
        for (index, word) in packed.words.into_iter().enumerate() {
            bytes[offset + index * 4..offset + index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
    }

    fn assert_components_close<const N: usize>(actual: [f32; N], expected: [f32; N]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn fast_string_matches_reference_words() {
        let packed = PackedFastString::<5>::encode("Trajectory").unwrap();
        assert_eq!(packed.words, [0x9133_81df, 0x90e2_9cdc, 0, 0, 0]);
        assert_eq!(packed.decode().unwrap(), "trajectory");
        assert_eq!(
            PackedFastString::<5>::encode("SPINE3")
                .unwrap()
                .decode()
                .unwrap(),
            "spine3"
        );
    }

    #[test]
    fn parses_physics_parameter_bones() {
        let header = AndaleDataHeader {
            size: 176,
            data_type: PHYSICS_PARAMETERS_TYPE,
            codec: 0,
            guid: 7,
            name: PackedFastString::<6>::encode("physics").unwrap(),
        };
        let mut bytes = vec![0_u8; 128];
        bytes[0..4].copy_from_slice(&1_u32.to_be_bytes());
        for (index, value) in (1_u32..=20).enumerate() {
            bytes[16 + index * 4..20 + index * 4].copy_from_slice(&value.to_be_bytes());
        }
        bytes[96..100].copy_from_slice(&2.5_f32.to_bits().to_be_bytes());
        bytes[100..104].copy_from_slice(&3.5_f32.to_bits().to_be_bytes());
        bytes[104..108].copy_from_slice(&4.5_f32.to_bits().to_be_bytes());
        write_fast_string::<5>(&mut bytes, 108, "hips");

        let physics = parse_physics_parameters(48, &header, &bytes).unwrap();

        assert_eq!(physics.name, "physics");
        assert_eq!(physics.guid, 7);
        assert_eq!(physics.bones.len(), 1);
        assert_eq!(physics.bones[0].name, "hips");
        assert_eq!(
            physics.bones[0].packed_name.words,
            std::array::from_fn(|word| read_be_u32(&bytes, 108 + word * 4).unwrap())
        );
        assert!((physics.bones[0].mass - 4.5).abs() < f32::EPSILON);
    }

    #[test]
    fn parses_blend_space_structure() {
        let header = controller_header(BLEND_SPACE_TYPE, "locomotion");
        let mut bytes = vec![0_u8; 132];
        bytes[0..4].copy_from_slice(&1_u32.to_be_bytes());
        bytes[4..8].copy_from_slice(&20_u32.to_be_bytes());
        bytes[8..12].copy_from_slice(&1_u32.to_be_bytes());
        bytes[12..16].copy_from_slice(&44_u32.to_be_bytes());
        bytes[16..20].copy_from_slice(&2_u32.to_be_bytes());
        write_fast_string::<6>(&mut bytes, 20, "idle");
        bytes[44..48].copy_from_slice(&88_u32.to_be_bytes());
        write_fast_string::<6>(&mut bytes, 48, "ground");
        bytes[72..76].copy_from_slice(&1_u32.to_be_bytes());
        bytes[76..80].copy_from_slice(&44_u32.to_be_bytes());
        bytes[80..84].copy_from_slice(&1_u32.to_be_bytes());
        bytes[84..88].copy_from_slice(&64_u32.to_be_bytes());
        write_fast_string::<5>(&mut bytes, 88, "speed");
        bytes[108..112].copy_from_slice(&24_u32.to_be_bytes());
        bytes[124..128].copy_from_slice(&1_u32.to_be_bytes());
        bytes[128..132].copy_from_slice(&0.25_f32.to_bits().to_be_bytes());

        let controller = parse_animation_controller(48, &header, &bytes).unwrap();
        let AndaleAnimationController::BlendSpace(blend_space) = controller else {
            panic!("expected blend space");
        };
        assert_eq!(blend_space.animation_names, ["idle"]);
        assert_eq!(blend_space.maximum_dimensions, 2);
        assert_eq!(blend_space.volume_sets[0].name, "ground");
        assert_eq!(blend_space.volume_sets[0].dimension_names, ["speed"]);
        assert_eq!(blend_space.volume_sets[0].volumes.len(), 1);
        assert_eq!(
            blend_space.volume_sets[0].volumes[0].serialized_following_word,
            None
        );
        let word = 0x7fc1_2345_u32.to_be_bytes();
        bytes.extend_from_slice(&word);
        let AndaleAnimationController::BlendSpace(with_tail) =
            parse_animation_controller(48, &header, &bytes).unwrap()
        else {
            panic!("expected blend space");
        };
        assert_eq!(
            with_tail.volume_sets[0].volumes[0].serialized_data.len(),
            24
        );
        assert_eq!(
            with_tail.volume_sets[0].volumes[0].serialized_following_word,
            Some(word)
        );
    }

    #[test]
    fn parses_phase_blend_attributes_and_children() {
        let header = controller_header(PHASE_BLEND_TYPE, "phase");
        let mut bytes = vec![0_u8; 100];
        bytes[0..4].copy_from_slice(&1_u32.to_be_bytes());
        bytes[4..8].copy_from_slice(&60_u32.to_be_bytes());
        bytes[8..12].copy_from_slice(&1_u32.to_be_bytes());
        write_fast_string::<5>(&mut bytes, 16, "phaseattr");
        write_fast_string::<6>(&mut bytes, 36, "push");
        bytes[60..64].copy_from_slice(&40_u32.to_be_bytes());
        bytes[64..68].copy_from_slice(&0.0_f32.to_bits().to_be_bytes());
        bytes[68..72].copy_from_slice(&1.0_f32.to_bits().to_be_bytes());
        write_fast_string::<5>(&mut bytes, 72, "speed");
        bytes[92] = 0;
        bytes[96..100].copy_from_slice(&0.5_f32.to_bits().to_be_bytes());

        let controller = parse_animation_controller(48, &header, &bytes).unwrap();
        let AndaleAnimationController::PhaseBlend(phase) = controller else {
            panic!("expected phase blend");
        };
        assert_eq!(phase.blend_attribute, "phaseattr");
        assert_eq!(phase.children, ["push"]);
        assert_eq!(phase.attributes[0].name, "speed");
        assert_eq!(phase.attributes[0].attribute_type, 0);
        assert_eq!(phase.attributes[0].scalar_value(), Some(0.5));
    }

    #[test]
    fn parses_construction_selector_options() {
        let header = controller_header(CONSTRUCTION_SELECTOR_TYPE, "stance");
        let mut bytes = vec![0_u8; 76];
        write_fast_string::<5>(&mut bytes, 0, "stance");
        bytes[20..24].copy_from_slice(&32_u32.to_be_bytes());
        bytes[24..28].copy_from_slice(&56_u32.to_be_bytes());
        bytes[28..32].copy_from_slice(&1_u32.to_be_bytes());
        write_fast_string::<6>(&mut bytes, 32, "regular");
        write_fast_string::<5>(&mut bytes, 56, "left");

        let controller = parse_animation_controller(48, &header, &bytes).unwrap();
        let AndaleAnimationController::ConstructionSelector(selector) = controller else {
            panic!("expected construction selector");
        };
        assert_eq!(selector.construction_attribute, "stance");
        assert_eq!(selector.options[0].value, "left");
        assert_eq!(selector.options[0].child, "regular");
    }

    #[test]
    fn parses_sequence_stages() {
        let header = controller_header(SEQUENCE_TYPE, "runout");
        let mut bytes = vec![0_u8; 32];
        bytes[0..2].copy_from_slice(&1_u16.to_be_bytes());
        bytes[2] = 1;
        bytes[3] = 4;
        write_fast_string::<6>(&mut bytes, 4, "air");
        bytes[28..32].copy_from_slice(&2_u32.to_be_bytes());

        let controller = parse_animation_controller(48, &header, &bytes).unwrap();
        let AndaleAnimationController::Sequence(sequence) = controller else {
            panic!("expected sequence");
        };
        assert!(sequence.looping);
        assert_eq!(sequence.window, 4);
        assert_eq!(sequence.children[0].stage, AndaleLocomotionStage::AirRight);
    }

    #[test]
    fn parses_selector_children() {
        let header = controller_header(SELECTOR_TYPE, "grounded");
        let mut bytes = vec![0_u8; 48];
        bytes[0..4].copy_from_slice(&1_u32.to_be_bytes());
        write_fast_string::<6>(&mut bytes, 4, "idle");
        write_fast_string::<5>(&mut bytes, 28, "speed");

        let controller = parse_animation_controller(48, &header, &bytes).unwrap();
        let AndaleAnimationController::Selector(selector) = controller else {
            panic!("expected selector");
        };
        assert_eq!(selector.children[0].name, "idle");
        assert_eq!(selector.children[0].attribute, "speed");
    }

    fn controller_header(data_type: u32, name: &str) -> AndaleDataHeader {
        AndaleDataHeader {
            size: 0,
            data_type,
            codec: 0,
            guid: 11,
            name: PackedFastString::<6>::encode(name).unwrap(),
        }
    }

    #[test]
    fn parses_hierarchy_record_and_names() {
        let mut bytes = vec![0_u8; 272];
        write_header(&mut bytes, 0, 256, FILE_HEADER_TYPE, 0, "fullskeleton");
        write_header(&mut bytes, 48, 208, HIERARCHY_TYPE, 1, "rig_tpose");
        let data = 96;
        bytes[data..data + 2].copy_from_slice(&2_u16.to_be_bytes());
        bytes[data + 2..data + 4].copy_from_slice(&1_u16.to_be_bytes());
        bytes[data + 4..data + 8].copy_from_slice(&1_u32.to_be_bytes());
        bytes[data + 8..data + 12].copy_from_slice(&(-1_i32).to_be_bytes());
        bytes[data + 12..data + 16].copy_from_slice(&1_i32.to_be_bytes());
        bytes[data + 16..data + 20].copy_from_slice(&(-1_i32).to_be_bytes());
        bytes[data + 20..data + 24].copy_from_slice(&(-1_i32).to_be_bytes());
        let part = data + 24;
        let part_name = PackedFastString::<6>::encode("body").unwrap();
        for (index, word) in part_name.words.into_iter().enumerate() {
            bytes[part + index * 4..part + index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        bytes[part + 24..part + 28].copy_from_slice(&2_u32.to_be_bytes());
        bytes[part + 32..part + 34].copy_from_slice(&1_u16.to_be_bytes());
        let names = part + HIERARCHY_PART_SIZE;
        for (bone_index, name) in ["Hips", "Spine1"].into_iter().enumerate() {
            let packed = PackedFastString::<5>::encode(name).unwrap();
            for (word_index, word) in packed.words.into_iter().enumerate() {
                let offset = names + bone_index * 20 + word_index * 4;
                bytes[offset..offset + 4].copy_from_slice(&word.to_be_bytes());
            }
        }

        let database = AndaleDatabase::parse(&bytes).unwrap();

        assert_eq!(database.file_header.name.decode().unwrap(), "fullskeleton");
        assert_eq!(database.records.len(), 1);
        assert_eq!(database.hierarchy.parents, [-1, 1]);
        assert_eq!(database.hierarchy.parts[0].name, "body");
        assert_eq!(database.hierarchy.bone_names, ["hips", "spine1"]);
        assert!(database.poses.is_empty());
    }

    #[test]
    fn parses_pose_part_ranges() {
        let mut bytes = vec![0_u8; 352];
        write_header(&mut bytes, 0, 336, FILE_HEADER_TYPE, 0, "fullskeleton");
        write_header(&mut bytes, 48, 144, HIERARCHY_TYPE, 1, "rig_tpose");
        let hierarchy = 96;
        bytes[hierarchy..hierarchy + 2].copy_from_slice(&1_u16.to_be_bytes());
        bytes[hierarchy + 4..hierarchy + 8].copy_from_slice(&1_u32.to_be_bytes());
        bytes[hierarchy + 8..hierarchy + 12].copy_from_slice(&(-1_i32).to_be_bytes());
        bytes[hierarchy + 12..hierarchy + 16].copy_from_slice(&(-1_i32).to_be_bytes());
        let hierarchy_part = hierarchy + 16;
        let part_name = PackedFastString::<6>::encode("body").unwrap();
        for (index, word) in part_name.words.into_iter().enumerate() {
            bytes[hierarchy_part + index * 4..hierarchy_part + index * 4 + 4]
                .copy_from_slice(&word.to_be_bytes());
        }
        bytes[hierarchy_part + 24..hierarchy_part + 28].copy_from_slice(&1_u32.to_be_bytes());
        let bone_name = PackedFastString::<5>::encode("hips").unwrap();
        for (index, word) in bone_name.words.into_iter().enumerate() {
            bytes[hierarchy_part + 36 + index * 4..hierarchy_part + 40 + index * 4]
                .copy_from_slice(&word.to_be_bytes());
        }
        write_header(&mut bytes, 192, 144, POSE_TYPE, 0x4452, "rig_tpose");
        let pose = 240;
        bytes[pose..pose + 4].copy_from_slice(&1_u32.to_be_bytes());
        bytes[pose + 4..pose + 8].copy_from_slice(&16_u32.to_be_bytes());
        let part = pose + 16;
        bytes[part..part + 4].copy_from_slice(&0x0002_0000_u32.to_be_bytes());
        bytes[part + 4..part + 8].copy_from_slice(&32_u32.to_be_bytes());
        bytes[part + 8..part + 12].copy_from_slice(&16_u32.to_be_bytes());
        bytes[part + 12..part + 16].copy_from_slice(&48_u32.to_be_bytes());
        bytes[part + 40..part + 42].copy_from_slice(&32_u16.to_be_bytes());

        let database = AndaleDatabase::parse(&bytes).unwrap();

        assert_eq!(database.poses.len(), 1);
        let part = &database.poses[0].parts[0];
        assert_eq!(part.header, 0x0002_0000);
        assert_eq!(part.compression_header.len(), 32);
        assert_eq!(part.compressed_data.len(), 32);
    }

    #[test]
    fn parses_clip_timing_and_part_layout() {
        let header = AndaleDataHeader {
            size: 160,
            data_type: CLIP_TYPE,
            codec: VBR_CODEC,
            guid: 0x1234_5678,
            name: PackedFastString::<6>::encode("kickflip").unwrap(),
        };
        let mut bytes = vec![0_u8; 176];
        bytes[32..36].copy_from_slice(&30.0_f32.to_bits().to_be_bytes());
        bytes[36..40].copy_from_slice(&61.0_f32.to_bits().to_be_bytes());
        bytes[40..44].copy_from_slice(&1.0_f32.to_bits().to_be_bytes());
        bytes[44..48].copy_from_slice(&112_u32.to_be_bytes());
        bytes[52..56].copy_from_slice(&(0x1000_0000_u32 | 1 << 8 | 1).to_be_bytes());
        bytes[56..60].copy_from_slice(&(3_u32 << 24 | 64).to_be_bytes());

        let part = 64;
        bytes[part..part + 4].copy_from_slice(&0x0002_0000_u32.to_be_bytes());
        bytes[part + 4..part + 8].copy_from_slice(&16_u32.to_be_bytes());
        bytes[part + 8..part + 12].copy_from_slice(&16_u32.to_be_bytes());
        bytes[part + 12..part + 16].copy_from_slice(&32_u32.to_be_bytes());
        bytes[112..116].copy_from_slice(&52_u32.to_be_bytes());
        bytes[116..120].copy_from_slice(&(-1.0_f32).to_bits().to_be_bytes());
        bytes[120..124].copy_from_slice(&(-1.0_f32).to_bits().to_be_bytes());
        write_fast_string::<5>(&mut bytes, 124, "angle");
        bytes[144] = 0;
        bytes[160..164].copy_from_slice(&(-1.0_f32).to_bits().to_be_bytes());

        let clip = AndaleClip::parse(0x200, &header, &bytes).unwrap();

        assert_eq!(clip.name, "kickflip");
        assert_eq!(clip.guid, 0x1234_5678);
        assert!((clip.duration_seconds() - 2.0).abs() < f32::EPSILON);
        assert!(clip.looping());
        assert_eq!(clip.parts.len(), 1);
        assert_eq!(clip.parts[0].part_name_index, 3);
        assert_eq!(clip.parts[0].compression_header_size, 16);
        assert_eq!(clip.parts[0].compressed_data_size, 16);
        assert_eq!(clip.parts[0].trailing_bytes_size, 0);
        assert_eq!(clip.attributes.len(), 1);
        assert_eq!(clip.attributes[0].name, "angle");
        assert!((clip.attributes[0].scalar_value().unwrap() + 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn parses_symbol_backed_vbr_header_sections() {
        let mut bytes = vec![0xcd_u8; 80];
        bytes[0..2].copy_from_slice(&2_u16.to_be_bytes());
        bytes[2..4].copy_from_slice(&1_u16.to_be_bytes());
        bytes[4..8].copy_from_slice(&f32::MAX.to_bits().to_be_bytes());
        bytes[8..12].copy_from_slice(&f32::MIN.to_bits().to_be_bytes());
        bytes[12] = 3;
        bytes[13] = 2;
        bytes[16..20].copy_from_slice(&0.0_f32.to_bits().to_be_bytes());
        bytes[20..24].copy_from_slice(&1.0_f32.to_bits().to_be_bytes());
        bytes[24..26].copy_from_slice(&4_u16.to_be_bytes());
        for (index, component_count) in [3_u8, 4, 3].into_iter().enumerate() {
            let offset = 32 + index * 16;
            bytes[offset..offset + 2].copy_from_slice(&0_u16.to_be_bytes());
            bytes[offset + 2..offset + 4].copy_from_slice(&1_u16.to_be_bytes());
            bytes[offset + 4..offset + 8].copy_from_slice(&0.0_f32.to_bits().to_be_bytes());
            bytes[offset + 8..offset + 12].copy_from_slice(&1.0_f32.to_bits().to_be_bytes());
            bytes[offset + 12] = component_count;
        }

        let header = parse_vbr_compression_header(&bytes).unwrap();

        assert_eq!(header.constant_channel_map_run_count, 2);
        assert_eq!(header.frame_count, 1);
        assert_eq!(header.constant_palette, [0.0, 1.0]);
        assert_eq!(header.frame_block_sizes, [4]);
        assert_eq!(header.alignment_padding.len(), 6);
        assert!(header.trailing_bytes.is_empty());
        assert_eq!(header.channel_blocks.len(), 3);
        assert_eq!(header.channel_blocks[0].constant_entry_count, 1);
        assert_eq!(header.channel_blocks[0].component_count, 3);

        let mut prelude_bytes = vec![0_u8; 16];
        prelude_bytes[1] = 1;
        prelude_bytes[3] = 1;
        prelude_bytes[6] = 1;
        prelude_bytes[10..12].copy_from_slice(&[0, 3]);
        let prelude = header.decode_prelude(&prelude_bytes).unwrap();
        assert_eq!(
            prelude
                .constant_values
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            [3, 4, 3]
        );
        assert_eq!(prelude.constant_entries, [true, true, true]);
        assert_eq!(prelude.frame_data_offset, 12);
        assert!(prelude.used_table.is_empty());
        assert_eq!(prelude.frame_blocks.len(), 1);
        assert_eq!(prelude.frame_blocks[0], 12..16);
        assert!(prelude.trailing_bytes.is_empty());
        let frame_block = header
            .decode_frame_block(&prelude, &prelude_bytes, 0)
            .unwrap();
        assert_eq!(frame_block.quantization_table_indices, [0, 0, 0]);
        assert!(frame_block.significance_bits.is_empty());
        assert!(frame_block.coefficient_payload.is_empty());
        assert!(
            header
                .unpack_frame_block_coefficients(&prelude, &frame_block)
                .unwrap()
                .is_empty()
        );

        let animated_prelude = AndaleVbrPrelude {
            constant_values: Vec::new(),
            constant_entries: Vec::new(),
            used_table: vec![0, 0, 0, 0x12],
            frame_data_offset: 0,
            frame_blocks: std::iter::once(0..6).collect(),
            trailing_bytes: Vec::new(),
        };
        let animated_bytes = [0, 0, 0, 2, 0b11, 0b1_0111];
        let animated_block = header
            .decode_frame_block(&animated_prelude, &animated_bytes, 0)
            .unwrap();
        let coefficients = header
            .unpack_frame_block_coefficients(&animated_prelude, &animated_block)
            .unwrap();
        assert_eq!(coefficients, [[3, -1, 0, 0, 0, 0, 0, 0]]);

        let part = AndaleAnimationPart {
            header: 80 << 12,
            encoded_offset: 0,
            part_name_index: 0,
            compressed_data_size: 16,
            compression_header_offset: 16,
            compressed_data_offset: 96,
            compression: AndaleCompressionHeader::Vbr(header),
            compression_header: bytes,
            compressed_data: prelude_bytes,
            padding: Vec::new(),
            vbr_prelude: Some(prelude),
        };
        let sqt = part.constant_vbr_sqt().unwrap();
        assert_eq!(sqt.len(), 1);
        assert_components_close(sqt[0].scale, [0.0, 1.0, 0.0, 1.0]);
        assert_components_close(sqt[0].rotation, [1.0, 0.0, 0.0, 1.0]);
        assert_components_close(sqt[0].translation, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn expands_raw_discretize_quaternions_and_translations() {
        let identity = decompress_raw_quaternion3([0; 6]);
        assert!(
            identity[..3]
                .iter()
                .all(|component| component.abs() < f32::EPSILON)
        );
        assert!((identity[3] - 1.0).abs() < f32::EPSILON);

        let mut encoded_identity = [0_u8; 8];
        encoded_identity[6..8].copy_from_slice(&i16::MAX.to_be_bytes());
        let normalized = decompress_raw_quaternion4(encoded_identity);
        assert!(
            normalized[..3]
                .iter()
                .all(|component| component.abs() < f32::EPSILON)
        );
        assert!((normalized[3] - 1.0).abs() < f32::EPSILON);

        let mut encoded_translation = [0_u8; 6];
        encoded_translation[..2].copy_from_slice(&i16::MAX.to_be_bytes());
        encoded_translation[2..4].copy_from_slice(&(-i16::MAX).to_be_bytes());
        let translation = decompress_raw_translation(encoded_translation, 2.0);
        for (actual, expected) in translation.into_iter().zip([2.0, -2.0, 0.0, 1.0]) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }

        let packed_identity = decompress_raw_packed_quaternion(0);
        assert!(
            packed_identity[..3]
                .iter()
                .all(|component| component.abs() < f32::EPSILON)
        );
        assert!((packed_identity[3] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn expands_interleaved_raw_pose_channels() {
        let mut compressed_data = Vec::new();
        compressed_data.extend_from_slice(&0_u32.to_be_bytes());
        for component in [1.0_f32, 2.0, 3.0, -1.0, -2.0, -3.0] {
            compressed_data.extend_from_slice(&component.to_be_bytes());
        }
        let part = AndaleAnimationPart {
            header: 1 << 6,
            encoded_offset: 0,
            part_name_index: 0,
            compressed_data_size: 28,
            compression_header_offset: 16,
            compressed_data_offset: 48,
            compression: AndaleCompressionHeader::RawDiscretize(AndaleRawCompressionHeader {
                channel_masks: [0, 0, 1, 0, 3, 0],
                frame_stride: 28,
                last_bone_index: 1,
                format_flags: [0, 1],
                padding: [0; 3],
            }),
            compression_header: vec![0; 32],
            compressed_data,
            padding: Vec::new(),
            vbr_prelude: None,
        };
        let pose = part.sample_raw_pose_sqt().unwrap();
        assert_eq!(pose.len(), 2);
        assert_components_close(pose[0].rotation, [0.0, 0.0, 0.0, 1.0]);
        assert_components_close(pose[0].translation, [1.0, 2.0, 3.0, 1.0]);
        assert_components_close(pose[1].translation, [-1.0, -2.0, -3.0, 1.0]);
    }

    #[test]
    fn reconstructs_vbr_inverse_dct_basis_vectors() {
        let dc = vbr_inverse_dct8([8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        for sample in dc {
            assert!((sample - 1.0).abs() < 1.0e-6);
        }

        let first_frequency = vbr_inverse_dct8([0.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let expected = [
            0.980_785_25,
            0.831_469_6,
            0.555_570_24,
            0.195_090_32,
            -0.195_090_32,
            -0.555_570_24,
            -0.831_469_6,
            -0.980_785_25,
        ];
        for (actual, expected) in first_frequency.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
    }

    #[test]
    fn dequantizes_and_interpolates_vbr_channels() {
        let quantization = vbr_quantization_row(0, -1.0, 0.5);
        let expected_dc = (1.0 + 2.0_f32.ln()) / 32768.0;
        assert!((quantization[0] - expected_dc).abs() < 1.0e-9);

        let samples = vbr_reconstruct_scalar_samples([0; 8], 0, -1.0, 0.5, -2.0, 3.0);
        for sample in samples {
            assert_eq!(sample.to_bits(), 0.5_f32.to_bits());
        }

        let rotation =
            normalized_quaternion_lerp([0.0, 0.0, 0.0, 2.0], [0.0, 0.0, 0.0, -4.0], 0.5).unwrap();
        assert_components_close(rotation, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn shapes_phase_blend_weights_at_outer_edges() {
        let lower_edge = phase_pose_coefficient(0.25, 0.0, true, false);
        let upper_edge = phase_pose_coefficient(0.75, 0.0, false, true);
        let interior = phase_pose_coefficient(0.25, 0.0, false, false);
        let culled = phase_pose_coefficient(0.1, 0.1, true, false);
        let upper_culled = phase_pose_coefficient(0.9, 0.1, false, true);

        assert!((lower_edge - 0.1875).abs() < f32::EPSILON);
        assert!((upper_edge - 0.8125).abs() < f32::EPSILON);
        assert!((interior - 0.25).abs() < f32::EPSILON);
        assert!(culled.abs() < f32::EPSILON);
        assert!((upper_culled - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn evaluates_reference_attribute_status_windows() {
        let timed = AndaleAttribute {
            begin: 0.2,
            end: 0.4,
            name: "PrepareJump".to_owned(),
            attribute_type: 0,
            value: AndaleAttributeValue::Float(1.0),
        };
        let active = runtime_attribute(&timed, 3.0, 2.5, 10.0, false, 7).unwrap();
        assert_eq!(active.status, ATTRIBUTE_STATUS_ACTIVE);
        assert!((active.begin_seconds - 2.0).abs() < f32::EPSILON);
        assert!((active.end_seconds - 4.0).abs() < f32::EPSILON);
        assert!(runtime_attribute(&timed, 5.0, 4.5, 10.0, false, 7).is_none());
        let ended = runtime_attribute(&timed, 5.0, 3.5, 10.0, false, 9).unwrap();
        assert_eq!(ended.status, ATTRIBUTE_STATUS_ENDED);

        let static_attribute = AndaleAttribute {
            begin: -1.0,
            end: -1.0,
            name: "angle".to_owned(),
            attribute_type: 0,
            value: AndaleAttributeValue::Float(-1.0),
        };
        let static_runtime = runtime_attribute(&static_attribute, 0.0, 0.0, 1.0, false, 0).unwrap();
        assert_eq!(static_runtime.status, ATTRIBUTE_STATUS_STATIC);
    }

    #[test]
    fn blends_only_matching_runtime_attributes_in_left_order() {
        let runtime = |name: &str, value: f32| AndaleRuntimeAttribute {
            sequence_id: -1,
            begin_seconds: -1.0,
            end_seconds: -1.0,
            name: name.to_owned(),
            status: ATTRIBUTE_STATUS_STATIC,
            value: AndaleAttributeValue::Float(value),
        };
        let lower = vec![runtime("angle", -1.0), runtime("compression", 1.0)];
        let upper = vec![runtime("angle", 1.0), runtime("unused", 2.0)];
        let blended = blend_runtime_attributes(lower, &upper, 0.25).unwrap();
        assert_eq!(blended.len(), 1);
        assert_eq!(blended[0].name, "angle");
        assert!((blended[0].scalar_value().unwrap() + 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn decodes_and_blends_contact_attribute_payloads() {
        let contact = |bone_name: &str, value: f32| {
            let mut bytes = [0_u8; 24];
            write_fast_string::<5>(&mut bytes, 0, bone_name);
            bytes[20..24].copy_from_slice(&value.to_bits().to_be_bytes());
            AndaleRuntimeAttribute {
                sequence_id: -1,
                begin_seconds: 0.1,
                end_seconds: 0.3,
                name: "push_contact".to_owned(),
                status: ATTRIBUTE_STATUS_ACTIVE,
                value: AndaleAttributeValue::Bytes24(bytes),
            }
        };
        let mut lower = contact("leftfoot", -1.0);
        let upper = contact("rightfoot", 1.0);
        blend_runtime_attribute(&mut lower, &upper, 0.75).unwrap();

        let decoded = lower.contact_value().unwrap().unwrap();
        assert_eq!(decoded.bone_name, "leftfoot");
        assert!((decoded.value - 0.5).abs() < f32::EPSILON);
        assert!((lower.begin_seconds - 0.1).abs() < f32::EPSILON);
        assert!((lower.end_seconds - 0.3).abs() < f32::EPSILON);
    }
}
