//! Native forms of Pegasus scene records serialized inside RW4 arenas.
//!
//! These layouts come from the game's PDB. Pointer and object-reference
//! semantics come from the corresponding `Fixup` routines in the reference
//! executable; no live Xbox pointer values are exposed by this module.

use std::fmt;

#[path = "pegasus_grab.rs"]
mod grab;
pub use grab::{GRAB_DATA_TYPE_ID, Grab, GrabData};
#[path = "pegasus_visual_indicator.rs"]
mod visual_indicator;
pub use visual_indicator::{VISUAL_INDICATOR_DATA_TYPE_ID, VisualIndicator, VisualIndicatorData};

pub const C_MATERIAL_DATA_TYPE_ID: u32 = 0x00eb_0006;
pub const SPLINE_DATA_TYPE_ID: u32 = 0x00eb_0004;
pub const LOCATION_DESC_DATA_TYPE_ID: u32 = 0x00eb_0009;
pub const C_MODEL_DATA_TYPE_ID: u32 = 0x00eb_000a;
pub const TABLE_OF_CONTENTS_TYPE_ID: u32 = 0x00eb_000b;
pub const INSTANCE_DATA_TYPE_ID: u32 = 0x00eb_000d;
pub const TRIGGER_INSTANCE_DATA_TYPE_ID: u32 = 0x00eb_0019;
pub const EMBEDDED_DATA_TYPE_ID: u32 = 0x00eb_001b;
pub const DMO_DATA_TYPE_ID: u32 = 0x00eb_001d;
pub const TOC_SPLINE_SUBREFERENCE_TYPE_ID: u32 = 0x00eb_0064;
pub const TOC_LOCATION_DESC_SUBREFERENCE_TYPE_ID: u32 = 0x00eb_0068;
pub const TOC_INSTANCE_SUBREFERENCE_TYPE_ID: u32 = 0x00eb_0069;
pub const TOC_TRIGGER_INSTANCE_SUBREFERENCE_TYPE_ID: u32 = 0x00eb_006b;

// Runtime-initialized by hashing the corresponding `Instance_*` strings with
// `rw::RwHash32String`; see the initializers at 0x82f76880..0x82f76a40.
pub const INSTANCE_MARQUEE_ASSET_TYPE_ID: u32 = 0x01ea_4aad;
pub const INSTANCE_DMO_ASSET_TYPE_ID: u32 = 0x06d0_51d1;
pub const INSTANCE_PROP_ASSET_TYPE_ID: u32 = 0x28b6_31ac;
pub const INSTANCE_TRIGGER_ASSET_TYPE_ID: u32 = 0x46db_86e5;
pub const INSTANCE_SPATIAL_ASSET_TYPE_ID: u32 = 0x7369_d9b5;
pub const INSTANCE_CREATE_A_CHARACTER_ASSET_TYPE_ID: u32 = 0x8456_31ad;
pub const INSTANCE_VISUAL_INDICATOR_ASSET_TYPE_ID: u32 = 0x9fcf_53fa;
pub const INSTANCE_ENVIRONMENT_ASSET_TYPE_ID: u32 = 0xacb3_1c9a;
pub const INSTANCE_LIVING_WORLD_ASSET_TYPE_ID: u32 = 0xe7f4_e308;

const INSTANCE_DATA_SIZE: usize = 20;
const INSTANCE_SIZE: usize = 160;
const C_MODEL_DATA_SIZE: usize = 12;
const C_MODEL_MESH_SIZE: usize = 8;
const TABLE_OF_CONTENTS_SIZE: usize = 20;
const TOC_ENTRY_SIZE: usize = 24;
const TYPE_MAP_SIZE: usize = 8;
const C_MATERIAL_DATA_SIZE: usize = 28;
const AUDIO_MATERIAL_SIZE: usize = 8;
const PHYSICS_MATERIAL_SIZE: usize = 8;
const C_MATERIAL_SIZE: usize = 12;
const SPLINE_DATA_SIZE: usize = 16;
const SPLINE_HEADER_SIZE: usize = 32;
const SPLINE_SEGMENT_SIZE: usize = 144;
const LOCATION_DESC_DATA_SIZE: usize = 24;
const LOCATION_DESC_SIZE: usize = 128;
const SUB_LOCATION_DESC_SIZE: usize = 112;
const TRIGGER_INSTANCE_DATA_SIZE: usize = 20;
const TRIGGER_INSTANCE_SIZE: usize = 240;
const EMBEDDED_DATA_SIZE: usize = 36;
const EMBEDDED_REFERENCE_SIZE: usize = 8;
const DMO_DATA_SIZE: usize = 20;
const DMO_SIZE: usize = 128;

/// Packed 16-bit Pegasus collision surface ID. Reference gameplay code reads
/// audio from bits 0..6, physics from bits 7..11, and pattern from bits 12..15.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SurfaceId(pub u16);

impl SurfaceId {
    #[must_use]
    pub const fn audio_surface_raw(self) -> u8 {
        (self.0 & 0x007f) as u8
    }

    #[must_use]
    pub const fn physics_surface_raw(self) -> u8 {
        ((self.0 >> 7) & 0x001f) as u8
    }

    #[must_use]
    pub const fn physics_surface(self) -> Option<PhysicsSurface> {
        PhysicsSurface::from_raw(self.physics_surface_raw())
    }

    #[must_use]
    pub const fn pattern_raw(self) -> u8 {
        ((self.0 >> 12) & 0x000f) as u8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PhysicsSurface {
    Undefined = 0,
    Smooth = 1,
    Rough = 2,
    Slow = 3,
    Slippery = 4,
    VerySlow = 5,
    Unrideable = 6,
    DoNotAlign = 7,
    Stair = 8,
    Wipeout = 9,
}

impl PhysicsSurface {
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Undefined,
            1 => Self::Smooth,
            2 => Self::Rough,
            3 => Self::Slow,
            4 => Self::Slippery,
            5 => Self::VerySlow,
            6 => Self::Unrideable,
            7 => Self::DoNotAlign,
            8 => Self::Stair,
            9 => Self::Wipeout,
            _ => return None,
        })
    }
}

/// Serialized arena object ID. `Arena::IdToObject` splits the value this way
/// before indexing a 24-byte arena dictionary entry.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ObjectId(pub u32);

impl ObjectId {
    #[must_use]
    pub const fn section_index(self) -> u16 {
        (self.0 >> 22) as u16
    }

    #[must_use]
    pub const fn entry_index(self) -> u32 {
        self.0 & 0x003f_ffff
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstanceData {
    pub type_id: u32,
    pub string_count: u32,
    pub instances_offset: u32,
    pub string_list_offset: u32,
    pub instances: Vec<Instance>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    pub transform: [f32; 16],
    pub bounding_box_min: [f32; 4],
    pub bounding_box_max: [f32; 4],
    pub guid: u64,
    pub local_guid: u64,
    pub class_key: u64,
    pub instance_key: u64,
    pub render_model: Option<ObjectId>,
    pub collision_model: Option<ObjectId>,
    pub embedded_data: Option<ObjectId>,
    pub name: String,
    pub description: String,
    pub component: String,
    pub category: String,
}

/// Borrowed, fixed-stride `tInstance` array used by processors that forward
/// the native record address instead of materializing the complete value.
///
/// This view deliberately validates only the 20-byte container header and the
/// counted 160-byte record range. In particular, it does not inspect string
/// offsets that `ProcessInstanceList` does not read on its DMO branch.
pub struct InstanceRecordTable<'a> {
    type_id: u32,
    instances_offset: u32,
    records: &'a [u8],
}

/// One owner-borrowed serialized `tInstance` record.
#[derive(Clone, Copy)]
pub struct InstanceRecordRef<'a> {
    bytes: &'a [u8],
    object_offset: u32,
    guid: u64,
    local_guid: u64,
}

impl<'a> InstanceRecordTable<'a> {
    #[must_use]
    pub const fn type_id(&self) -> u32 {
        self.type_id
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.records.len() / INSTANCE_SIZE
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    #[must_use]
    pub fn record(&self, index: usize) -> Option<InstanceRecordRef<'a>> {
        let start = index.checked_mul(INSTANCE_SIZE)?;
        let end = start.checked_add(INSTANCE_SIZE)?;
        let bytes = self.records.get(start..end)?;
        Some(InstanceRecordRef {
            bytes,
            object_offset: element_object_offset(index, self.instances_offset, INSTANCE_SIZE)?,
            guid: be_u64(bytes, 96).ok()?,
            local_guid: be_u64(bytes, 104).ok()?,
        })
    }
}

impl<'a> InstanceRecordRef<'a> {
    #[must_use]
    pub const fn object_offset(self) -> u32 {
        self.object_offset
    }

    #[must_use]
    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }

    #[must_use]
    pub const fn guid(self) -> u64 {
        self.guid
    }

    #[must_use]
    pub const fn local_guid(self) -> u64 {
        self.local_guid
    }
}

impl InstanceData {
    /// Borrows the exact inline record storage used by
    /// `ProcessInstanceList` (`0x82d5ec38`).
    ///
    /// Unlike [`Self::parse`], this processor-facing view does not validate or
    /// decode unrelated strings. The returned record slices therefore remain
    /// tied to the caller's arena storage and can support pointer-equivalent
    /// owner/offset identity.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] if the container header or counted record
    /// array is outside `bytes`.
    pub fn record_table(bytes: &[u8]) -> Result<InstanceRecordTable<'_>, PegasusError> {
        require(bytes, 0, INSTANCE_DATA_SIZE)?;
        let instance_count = be_u32(bytes, 4)?;
        let instances_offset = be_u32(bytes, 12)?;
        let records = counted_range(
            bytes,
            as_usize(instances_offset)?,
            instance_count,
            INSTANCE_SIZE,
        )?;
        Ok(InstanceRecordTable {
            type_id: be_u32(bytes, 0)?,
            instances_offset,
            records,
        })
    }

    /// Parses `pegasus::tInstanceData` and its inline 160-byte instance array.
    /// String pointers are offsets from the containing object, exactly as the
    /// reference fixup treats them.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] for invalid ranges, strings, or integer sizes.
    pub fn parse(bytes: &[u8]) -> Result<Self, PegasusError> {
        require(bytes, 0, INSTANCE_DATA_SIZE)?;
        let instance_count = be_u32(bytes, 4)?;
        let string_count = be_u32(bytes, 8)?;
        let instances_offset = be_u32(bytes, 12)?;
        let string_list_offset = be_u32(bytes, 16)?;
        let instances_start = as_usize(instances_offset)?;
        let instances_bytes = counted_range(bytes, instances_start, instance_count, INSTANCE_SIZE)?;
        require(bytes, as_usize(string_list_offset)?, 0)?;

        let mut instances = Vec::with_capacity(as_usize(instance_count)?);
        for index in 0..as_usize(instance_count)? {
            let start = instances_start + index * INSTANCE_SIZE;
            let record = &instances_bytes[index * INSTANCE_SIZE..(index + 1) * INSTANCE_SIZE];
            instances.push(Instance {
                transform: be_f32_array::<16>(record, 0)?,
                bounding_box_min: be_f32_array::<4>(record, 64)?,
                bounding_box_max: be_f32_array::<4>(record, 80)?,
                guid: be_u64(record, 96)?,
                local_guid: be_u64(record, 104)?,
                class_key: be_u64(record, 112)?,
                instance_key: be_u64(record, 120)?,
                render_model: optional_object_id(record, 128)?,
                collision_model: optional_object_id(record, 132)?,
                embedded_data: optional_object_id(record, 136)?,
                name: object_string(bytes, be_u32(record, 140)?, start + 140)?,
                description: object_string(bytes, be_u32(record, 144)?, start + 144)?,
                component: object_string(bytes, be_u32(record, 148)?, start + 148)?,
                category: object_string(bytes, be_u32(record, 152)?, start + 152)?,
            });
        }

        Ok(Self {
            type_id: be_u32(bytes, 0)?,
            string_count,
            instances_offset,
            string_list_offset,
            instances,
        })
    }

    /// Resolves a subreference byte offset to one of the inline instances.
    #[must_use]
    pub fn instance_at_object_offset(&self, offset: u32) -> Option<&Instance> {
        let relative = offset.checked_sub(self.instances_offset)?;
        let stride = u32::try_from(INSTANCE_SIZE).ok()?;
        if relative % stride != 0 {
            return None;
        }
        self.instances.get(usize::try_from(relative / stride).ok()?)
    }

    /// Returns an inline instance's object-relative byte offset.
    #[must_use]
    pub fn instance_object_offset(&self, index: usize) -> Option<u32> {
        let index = u32::try_from(index).ok()?;
        let stride = u32::try_from(INSTANCE_SIZE).ok()?;
        self.instances_offset
            .checked_add(index.checked_mul(stride)?)
    }
}

/// One entry in a `tEmbeddedData` pointer array. Fixup preserves the first
/// word and resolves the second word through `Arena::IdToObject`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmbeddedReference {
    pub key: u32,
    pub object: ObjectId,
}

/// Lossless native form of the 36-byte `pegasus::tEmbeddedData` header. Group
/// names and order come directly from the full PDB type.
#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddedData {
    pub type_id: u32,
    pub reference_groups: [Vec<EmbeddedReference>; 4],
}

impl EmbeddedData {
    /// Parses the four count/offset arrays recovered from `tEmbeddedData::Fixup`
    /// at `0x829c6738`.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] if the header or any eight-byte reference
    /// array extends beyond its arena allocation.
    pub fn parse(bytes: &[u8]) -> Result<Self, PegasusError> {
        require(bytes, 0, EMBEDDED_DATA_SIZE)?;
        let mut reference_groups: [Vec<EmbeddedReference>; 4] = Default::default();
        for (group_index, group) in reference_groups.iter_mut().enumerate() {
            let count = be_u32(bytes, 4 + group_index * 4)?;
            let offset = be_u32(bytes, 20 + group_index * 4)?;
            let records = counted_range(bytes, as_usize(offset)?, count, EMBEDDED_REFERENCE_SIZE)?;
            for record in records.chunks_exact(EMBEDDED_REFERENCE_SIZE) {
                group.push(EmbeddedReference {
                    key: be_u32(record, 0)?,
                    object: ObjectId(be_u32(record, 4)?),
                });
            }
        }
        Ok(Self {
            type_id: be_u32(bytes, 0)?,
            reference_groups,
        })
    }

    #[must_use]
    pub fn grind_splines(&self) -> &[EmbeddedReference] {
        &self.reference_groups[1]
    }

    #[must_use]
    pub fn grab_splines(&self) -> &[EmbeddedReference] {
        &self.reference_groups[2]
    }

    #[must_use]
    pub fn hot_points(&self) -> &[EmbeddedReference] {
        &self.reference_groups[0]
    }

    #[must_use]
    pub fn visual_indicators(&self) -> &[EmbeddedReference] {
        &self.reference_groups[3]
    }
}

/// Native container for streamed living-world dynamic-object locators. The
/// header mirrors `tInstanceData`: a record count, string count, 128-byte
/// record array, and string-list offset. Field names and sizes come from the
/// full PDB; `tDMOData::Fixup` at `0x829c5670` proves the pointer semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct DmoData {
    pub type_id: u32,
    pub string_count: u32,
    pub dmos_offset: u32,
    pub string_list_offset: u32,
    pub dmos: Vec<Dmo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dmo {
    pub transform: [f32; 16],
    pub bounding_box_min: [f32; 4],
    pub bounding_box_max: [f32; 4],
    pub guid: u64,
    pub local_guid: u64,
    pub instance_guid: u64,
    /// Serialized contents of `tInstancePtr`. The reference fixup clears this
    /// field to null; model registration fills it later at runtime.
    pub serialized_instance_word: u32,
    pub name_offset: u32,
    pub name: String,
}

impl DmoData {
    /// Parses the PDB-named container and locator fields whose pointer
    /// semantics are established by the reference fixup.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] when the header, DMO array, string-list base,
    /// or a locator name lies outside this RW4 object.
    pub fn parse(bytes: &[u8]) -> Result<Self, PegasusError> {
        require(bytes, 0, DMO_DATA_SIZE)?;
        let dmo_count = be_u32(bytes, 4)?;
        let string_count = be_u32(bytes, 8)?;
        let dmos_offset = be_u32(bytes, 12)?;
        let string_list_offset = be_u32(bytes, 16)?;
        let dmos_start = as_usize(dmos_offset)?;
        let dmos_bytes = counted_range(bytes, dmos_start, dmo_count, DMO_SIZE)?;
        require(bytes, as_usize(string_list_offset)?, 0)?;

        let mut dmos = Vec::with_capacity(as_usize(dmo_count)?);
        for index in 0..as_usize(dmo_count)? {
            let start = dmos_start + index * DMO_SIZE;
            let record = &dmos_bytes[index * DMO_SIZE..(index + 1) * DMO_SIZE];
            let name_offset = be_u32(record, 124)?;
            dmos.push(Dmo {
                transform: be_f32_array::<16>(record, 0)?,
                bounding_box_min: be_f32_array::<4>(record, 64)?,
                bounding_box_max: be_f32_array::<4>(record, 80)?,
                guid: be_u64(record, 96)?,
                local_guid: be_u64(record, 104)?,
                instance_guid: be_u64(record, 112)?,
                serialized_instance_word: be_u32(record, 120)?,
                name_offset,
                name: object_string(bytes, name_offset, start + 124)?,
            });
        }

        Ok(Self {
            type_id: be_u32(bytes, 0)?,
            string_count,
            dmos_offset,
            string_list_offset,
            dmos,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SplineData {
    pub splines_offset: u32,
    pub segments_offset: u32,
    pub splines: Vec<SplineHeader>,
    pub segments: Vec<SplineSegment>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplineHeader {
    pub guid: u64,
    pub local_guid: u64,
    pub instance: ObjectId,
    pub head_offset: Option<u32>,
    pub tail_offset: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplineSegment {
    pub basis_matrix: [f32; 16],
    pub inverse: [f32; 4],
    pub bounding_box_min: [f32; 4],
    pub bounding_box_max: [f32; 4],
    pub length: f32,
    pub distance: f32,
    pub spline_offset: Option<u32>,
    pub previous_offset: Option<u32>,
    pub next_offset: Option<u32>,
}

impl SplineData {
    /// Parses `tSplineData` plus its inline header and segment arrays.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] for truncated or invalid array ranges.
    pub fn parse(bytes: &[u8]) -> Result<Self, PegasusError> {
        require(bytes, 0, SPLINE_DATA_SIZE)?;
        let spline_count = be_u32(bytes, 0)?;
        let segment_count = be_u32(bytes, 4)?;
        let splines_offset = be_u32(bytes, 8)?;
        let segments_offset = be_u32(bytes, 12)?;
        let spline_bytes = counted_range(
            bytes,
            as_usize(splines_offset)?,
            spline_count,
            SPLINE_HEADER_SIZE,
        )?;
        let segment_bytes = counted_range(
            bytes,
            as_usize(segments_offset)?,
            segment_count,
            SPLINE_SEGMENT_SIZE,
        )?;
        let mut splines = Vec::with_capacity(as_usize(spline_count)?);
        for index in 0..as_usize(spline_count)? {
            let offset = index * SPLINE_HEADER_SIZE;
            splines.push(SplineHeader {
                guid: be_u64(spline_bytes, offset)?,
                local_guid: be_u64(spline_bytes, offset + 8)?,
                instance: ObjectId(be_u32(spline_bytes, offset + 16)?),
                head_offset: nonzero(be_u32(spline_bytes, offset + 20)?),
                tail_offset: nonzero(be_u32(spline_bytes, offset + 24)?),
            });
        }
        let mut segments = Vec::with_capacity(as_usize(segment_count)?);
        for index in 0..as_usize(segment_count)? {
            let offset = index * SPLINE_SEGMENT_SIZE;
            segments.push(SplineSegment {
                basis_matrix: be_f32_array::<16>(segment_bytes, offset)?,
                inverse: be_f32_array::<4>(segment_bytes, offset + 64)?,
                bounding_box_min: be_f32_array::<4>(segment_bytes, offset + 80)?,
                bounding_box_max: be_f32_array::<4>(segment_bytes, offset + 96)?,
                length: be_f32(segment_bytes, offset + 112)?,
                distance: be_f32(segment_bytes, offset + 116)?,
                spline_offset: nonzero(be_u32(segment_bytes, offset + 120)?),
                previous_offset: nonzero(be_u32(segment_bytes, offset + 124)?),
                next_offset: nonzero(be_u32(segment_bytes, offset + 128)?),
            });
        }
        Ok(Self {
            splines_offset,
            segments_offset,
            splines,
            segments,
        })
    }

    /// Resolves a section-2 byte offset to an inline spline header.
    #[must_use]
    pub fn spline_at_object_offset(&self, offset: u32) -> Option<&SplineHeader> {
        self.splines
            .get(self.spline_index_at_object_offset(offset)?)
    }

    /// Resolves a section-2 byte offset to an inline spline-header index.
    #[must_use]
    pub fn spline_index_at_object_offset(&self, offset: u32) -> Option<usize> {
        let relative = offset.checked_sub(self.splines_offset)?;
        let stride = u32::try_from(SPLINE_HEADER_SIZE).ok()?;
        if relative % stride != 0 {
            return None;
        }
        let index = usize::try_from(relative / stride).ok()?;
        (index < self.splines.len()).then_some(index)
    }

    /// Returns an inline spline header's object-relative byte offset.
    #[must_use]
    pub fn spline_object_offset(&self, index: usize) -> Option<u32> {
        let index = u32::try_from(index).ok()?;
        let stride = u32::try_from(SPLINE_HEADER_SIZE).ok()?;
        self.splines_offset.checked_add(index.checked_mul(stride)?)
    }

    /// Resolves an object-relative byte offset to an inline spline segment.
    #[must_use]
    pub fn segment_at_object_offset(&self, offset: u32) -> Option<&SplineSegment> {
        self.segments.get(self.segment_index_at_object_offset(offset)?)
    }

    /// Resolves an object-relative offset using the serialized segment stride.
    #[must_use]
    pub fn segment_index_at_object_offset(&self, offset: u32) -> Option<usize> {
        let relative = offset.checked_sub(self.segments_offset)?;
        let stride = u32::try_from(SPLINE_SEGMENT_SIZE).ok()?;
        if relative % stride != 0 {
            return None;
        }
        let index = usize::try_from(relative / stride).ok()?;
        (index < self.segments.len()).then_some(index)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisAlignedBox {
    pub minimum: [f32; 4],
    pub maximum: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocationDescData {
    pub string_count: u32,
    pub locations_offset: u32,
    pub sub_locations_offset: u32,
    pub string_list_offset: u32,
    pub locations: Vec<LocationDesc>,
    pub sub_locations: Vec<SubLocationDesc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocationDesc {
    pub transform: [f32; 16],
    pub bounding_box: AxisAlignedBox,
    pub sub_location_count: u32,
    pub sub_locations_offset: Option<u32>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub guid: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubLocationDesc {
    pub transform: [f32; 16],
    pub bounding_box: AxisAlignedBox,
    pub name: Option<String>,
    pub description: Option<String>,
}

impl LocationDescData {
    /// Parses `tLocationDescData` using the object-relative pointer contract
    /// proven by the reference fixup at `0x829c5998`.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] for truncated arrays, invalid sublocation
    /// spans, or malformed strings.
    pub fn parse(bytes: &[u8]) -> Result<Self, PegasusError> {
        require(bytes, 0, LOCATION_DESC_DATA_SIZE)?;
        let location_count = be_u32(bytes, 0)?;
        let sub_location_count = be_u32(bytes, 4)?;
        let string_count = be_u32(bytes, 8)?;
        let locations_offset = be_u32(bytes, 12)?;
        let sub_locations_offset = be_u32(bytes, 16)?;
        let string_list_offset = be_u32(bytes, 20)?;
        require(bytes, as_usize(string_list_offset)?, 0)?;
        let location_bytes = counted_range(
            bytes,
            as_usize(locations_offset)?,
            location_count,
            LOCATION_DESC_SIZE,
        )?;
        let sub_location_bytes = counted_range(
            bytes,
            as_usize(sub_locations_offset)?,
            sub_location_count,
            SUB_LOCATION_DESC_SIZE,
        )?;

        let mut locations = Vec::with_capacity(as_usize(location_count)?);
        for index in 0..as_usize(location_count)? {
            let offset = index * LOCATION_DESC_SIZE;
            let sub_count = be_u32(location_bytes, offset + 96)?;
            let sub_offset = nonzero(be_u32(location_bytes, offset + 100)?);
            if sub_count != 0 {
                let start = sub_offset.ok_or(PegasusError::MissingOffset {
                    field_offset: as_usize(locations_offset)? + offset + 100,
                })?;
                counted_range(bytes, as_usize(start)?, sub_count, SUB_LOCATION_DESC_SIZE)?;
            }
            locations.push(LocationDesc {
                transform: be_f32_array::<16>(location_bytes, offset)?,
                bounding_box: parse_aabb(location_bytes, offset + 64)?,
                sub_location_count: sub_count,
                sub_locations_offset: sub_offset,
                name: optional_object_string(
                    bytes,
                    be_u32(location_bytes, offset + 104)?,
                    as_usize(locations_offset)? + offset + 104,
                )?,
                description: optional_object_string(
                    bytes,
                    be_u32(location_bytes, offset + 108)?,
                    as_usize(locations_offset)? + offset + 108,
                )?,
                guid: be_u64(location_bytes, offset + 112)?,
            });
        }

        let mut sub_locations = Vec::with_capacity(as_usize(sub_location_count)?);
        for index in 0..as_usize(sub_location_count)? {
            let offset = index * SUB_LOCATION_DESC_SIZE;
            sub_locations.push(SubLocationDesc {
                transform: be_f32_array::<16>(sub_location_bytes, offset)?,
                bounding_box: parse_aabb(sub_location_bytes, offset + 64)?,
                name: optional_object_string(
                    bytes,
                    be_u32(sub_location_bytes, offset + 96)?,
                    as_usize(sub_locations_offset)? + offset + 96,
                )?,
                description: optional_object_string(
                    bytes,
                    be_u32(sub_location_bytes, offset + 100)?,
                    as_usize(sub_locations_offset)? + offset + 100,
                )?,
            });
        }

        Ok(Self {
            string_count,
            locations_offset,
            sub_locations_offset,
            string_list_offset,
            locations,
            sub_locations,
        })
    }

    #[must_use]
    pub fn location_at_object_offset(&self, offset: u32) -> Option<&LocationDesc> {
        element_at_offset(
            &self.locations,
            offset,
            self.locations_offset,
            LOCATION_DESC_SIZE,
        )
    }

    #[must_use]
    pub fn location_object_offset(&self, index: usize) -> Option<u32> {
        element_object_offset(index, self.locations_offset, LOCATION_DESC_SIZE)
    }

    #[must_use]
    pub fn sub_locations_for(&self, location: &LocationDesc) -> Option<&[SubLocationDesc]> {
        if location.sub_location_count == 0 {
            return Some(&[]);
        }
        let relative = location
            .sub_locations_offset?
            .checked_sub(self.sub_locations_offset)?;
        let stride = u32::try_from(SUB_LOCATION_DESC_SIZE).ok()?;
        if relative % stride != 0 {
            return None;
        }
        let start = usize::try_from(relative / stride).ok()?;
        let count = usize::try_from(location.sub_location_count).ok()?;
        self.sub_locations.get(start..start.checked_add(count)?)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TriggerInstanceType {
    Challenge = 0,
    Stairs = 1,
    Camera = 2,
    Crowd = 3,
}

impl TriggerInstanceType {
    #[must_use]
    pub const fn from_raw(value: u32) -> Option<Self> {
        Some(match value {
            0 => Self::Challenge,
            1 => Self::Stairs,
            2 => Self::Camera,
            3 => Self::Crowd,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TriggerInstanceData {
    pub type_id: u32,
    pub string_count: u32,
    pub instances_offset: u32,
    pub string_list_offset: u32,
    pub instances: Vec<TriggerInstance>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TriggerInstance {
    pub transform: [f32; 16],
    pub bounding_box: AxisAlignedBox,
    pub base_plane_normal: [f32; 4],
    pub base_plane_leading_edge: [[f32; 4]; 2],
    pub base_plane_trailing_edge: [[f32; 4]; 2],
    pub guid: u64,
    pub local_guid: u64,
    pub class_key: u64,
    pub instance_key: u64,
    pub trigger_data: u64,
    pub trigger_type_raw: u32,
    pub trigger_type: Option<TriggerInstanceType>,
    pub collision_model: Option<ObjectId>,
    pub name: Option<String>,
}

/// Borrowed, fixed-stride `tTriggerInstance` array used by
/// `ProcessTriggerInstanceList`.
pub struct TriggerInstanceRecordTable<'a> {
    instances_offset: u32,
    records: &'a [u8],
}

/// One owner-borrowed serialized 240-byte trigger record.
#[derive(Clone, Copy)]
pub struct TriggerInstanceRecordRef<'a> {
    bytes: &'a [u8],
    object_offset: u32,
    guid: u64,
    local_guid: u64,
}

impl<'a> TriggerInstanceRecordTable<'a> {
    #[must_use]
    pub const fn len(&self) -> usize {
        self.records.len() / TRIGGER_INSTANCE_SIZE
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    #[must_use]
    pub fn record(&self, index: usize) -> Option<TriggerInstanceRecordRef<'a>> {
        let start = index.checked_mul(TRIGGER_INSTANCE_SIZE)?;
        let end = start.checked_add(TRIGGER_INSTANCE_SIZE)?;
        let bytes = self.records.get(start..end)?;
        Some(TriggerInstanceRecordRef {
            bytes,
            object_offset: element_object_offset(
                index,
                self.instances_offset,
                TRIGGER_INSTANCE_SIZE,
            )?,
            guid: be_u64(bytes, 176).ok()?,
            local_guid: be_u64(bytes, 184).ok()?,
        })
    }
}

impl<'a> TriggerInstanceRecordRef<'a> {
    #[must_use]
    pub const fn object_offset(self) -> u32 {
        self.object_offset
    }

    #[must_use]
    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }

    #[must_use]
    pub const fn guid(self) -> u64 {
        self.guid
    }

    #[must_use]
    pub const fn local_guid(self) -> u64 {
        self.local_guid
    }
}

impl TriggerInstanceData {
    /// Borrows the exact inline record storage forwarded by
    /// `ProcessTriggerInstanceList` (`0x82d5f078`).
    ///
    /// The native processor reads the count, array pointer, and local GUID; it
    /// does not decode the optional name. This narrow view consequently does
    /// not impose [`Self::parse`]'s string validation on that processing path.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] if the container header or counted record
    /// array is outside `bytes`.
    pub fn record_table(bytes: &[u8]) -> Result<TriggerInstanceRecordTable<'_>, PegasusError> {
        require(bytes, 0, TRIGGER_INSTANCE_DATA_SIZE)?;
        let instance_count = be_u32(bytes, 4)?;
        let instances_offset = be_u32(bytes, 12)?;
        let records = counted_range(
            bytes,
            as_usize(instances_offset)?,
            instance_count,
            TRIGGER_INSTANCE_SIZE,
        )?;
        Ok(TriggerInstanceRecordTable {
            instances_offset,
            records,
        })
    }

    /// Parses `tTriggerInstanceData`; collision-model words are arena IDs and
    /// names are object-relative offsets per the fixup at `0x829c6b20`.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] for truncated instance arrays or strings.
    pub fn parse(bytes: &[u8]) -> Result<Self, PegasusError> {
        require(bytes, 0, TRIGGER_INSTANCE_DATA_SIZE)?;
        let type_id = be_u32(bytes, 0)?;
        let instance_count = be_u32(bytes, 4)?;
        let string_count = be_u32(bytes, 8)?;
        let instances_offset = be_u32(bytes, 12)?;
        let string_list_offset = be_u32(bytes, 16)?;
        let instances = counted_range(
            bytes,
            as_usize(instances_offset)?,
            instance_count,
            TRIGGER_INSTANCE_SIZE,
        )?;
        require(bytes, as_usize(string_list_offset)?, 0)?;
        let mut parsed = Vec::with_capacity(as_usize(instance_count)?);
        for index in 0..as_usize(instance_count)? {
            let offset = index * TRIGGER_INSTANCE_SIZE;
            let trigger_type_raw = be_u32(instances, offset + 216)?;
            parsed.push(TriggerInstance {
                transform: be_f32_array::<16>(instances, offset)?,
                bounding_box: parse_aabb(instances, offset + 64)?,
                base_plane_normal: be_f32_array::<4>(instances, offset + 96)?,
                base_plane_leading_edge: [
                    be_f32_array::<4>(instances, offset + 112)?,
                    be_f32_array::<4>(instances, offset + 128)?,
                ],
                base_plane_trailing_edge: [
                    be_f32_array::<4>(instances, offset + 144)?,
                    be_f32_array::<4>(instances, offset + 160)?,
                ],
                guid: be_u64(instances, offset + 176)?,
                local_guid: be_u64(instances, offset + 184)?,
                class_key: be_u64(instances, offset + 192)?,
                instance_key: be_u64(instances, offset + 200)?,
                trigger_data: be_u64(instances, offset + 208)?,
                trigger_type_raw,
                trigger_type: TriggerInstanceType::from_raw(trigger_type_raw),
                collision_model: optional_object_id(instances, offset + 220)?,
                name: optional_object_string(
                    bytes,
                    be_u32(instances, offset + 224)?,
                    as_usize(instances_offset)? + offset + 224,
                )?,
            });
        }
        Ok(Self {
            type_id,
            string_count,
            instances_offset,
            string_list_offset,
            instances: parsed,
        })
    }

    #[must_use]
    pub fn instance_at_object_offset(&self, offset: u32) -> Option<&TriggerInstance> {
        element_at_offset(
            &self.instances,
            offset,
            self.instances_offset,
            TRIGGER_INSTANCE_SIZE,
        )
    }

    #[must_use]
    pub fn instance_object_offset(&self, index: usize) -> Option<u32> {
        element_object_offset(index, self.instances_offset, TRIGGER_INSTANCE_SIZE)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollisionModelMeshes {
    /// The `m_iNumMeshes == -1` form is a direct arena ID for
    /// `rw::physics::Assembly`, not its definition.
    Assembly(ObjectId),
    Meshes(Vec<CollisionModelMesh>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollisionModelMesh {
    pub mesh: ObjectId,
    /// The reference fixup deliberately leaves this serialized value alone.
    /// Retain it until consumers establish whether it is an atom or an offset
    /// into an owning asset's string storage.
    pub bone_name: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollisionModelData {
    pub bounding_volume: Option<ObjectId>,
    pub meshes: CollisionModelMeshes,
}

impl CollisionModelData {
    /// Reads only `tCModelData::m_iNumMeshes` at `+0x04`, as used by the
    /// Environment hybrid filter in `ProcessInstanceList`
    /// (`0x82d5eea0..0x82d5eebc`).
    ///
    /// This deliberately does not validate the mesh table or assembly link;
    /// that branch observes only the raw count.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] when bytes through field `+0x04` are absent.
    pub fn raw_mesh_count(bytes: &[u8]) -> Result<u32, PegasusError> {
        be_u32(bytes, 4)
    }

    /// Parses a collision model and its inline mesh-reference table.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] for truncated headers or invalid table ranges.
    pub fn parse(bytes: &[u8]) -> Result<Self, PegasusError> {
        require(bytes, 0, C_MODEL_DATA_SIZE)?;
        let bounding_volume = optional_object_id(bytes, 0)?;
        let mesh_count = be_u32(bytes, 4)?;
        let mesh_word = be_u32(bytes, 8)?;
        let meshes = if mesh_count == u32::MAX {
            CollisionModelMeshes::Assembly(ObjectId(mesh_word))
        } else {
            let table = counted_range(bytes, as_usize(mesh_word)?, mesh_count, C_MODEL_MESH_SIZE)?;
            let mut meshes = Vec::with_capacity(as_usize(mesh_count)?);
            for index in 0..as_usize(mesh_count)? {
                let offset = index * C_MODEL_MESH_SIZE;
                meshes.push(CollisionModelMesh {
                    mesh: ObjectId(be_u32(table, offset)?),
                    bone_name: be_u32(table, offset + 4)?,
                });
            }
            CollisionModelMeshes::Meshes(meshes)
        };
        Ok(Self {
            bounding_volume,
            meshes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableOfContents {
    pub names_offset: u32,
    pub entries: Vec<TocEntry>,
    pub type_map: Vec<TypeMapEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TocEntry {
    pub name: String,
    pub guid: u64,
    pub object_type: u32,
    pub object: ObjectId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypeMapEntry {
    pub object_type: u32,
    pub first_index: u32,
}

impl TableOfContents {
    /// Parses TOC records, object-relative names, and type-map entries.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] for invalid table ranges or strings.
    pub fn parse(bytes: &[u8]) -> Result<Self, PegasusError> {
        require(bytes, 0, TABLE_OF_CONTENTS_SIZE)?;
        let item_count = be_u32(bytes, 0)?;
        let array_offset = be_u32(bytes, 4)?;
        let names_offset = be_u32(bytes, 8)?;
        let type_count = be_u32(bytes, 12)?;
        let type_map_offset = be_u32(bytes, 16)?;
        require(bytes, as_usize(names_offset)?, 0)?;
        let array = counted_range(bytes, as_usize(array_offset)?, item_count, TOC_ENTRY_SIZE)?;
        let mut entries = Vec::with_capacity(as_usize(item_count)?);
        for index in 0..as_usize(item_count)? {
            let offset = index * TOC_ENTRY_SIZE;
            entries.push(TocEntry {
                name: object_string(
                    bytes,
                    be_u32(array, offset)?,
                    as_usize(array_offset)? + offset,
                )?,
                guid: be_u64(array, offset + 8)?,
                object_type: be_u32(array, offset + 16)?,
                object: ObjectId(be_u32(array, offset + 20)?),
            });
        }
        let map = counted_range(bytes, as_usize(type_map_offset)?, type_count, TYPE_MAP_SIZE)?;
        let mut type_map = Vec::with_capacity(as_usize(type_count)?);
        for index in 0..as_usize(type_count)? {
            let offset = index * TYPE_MAP_SIZE;
            type_map.push(TypeMapEntry {
                object_type: be_u32(map, offset)?,
                first_index: be_u32(map, offset + 4)?,
            });
        }
        Ok(Self {
            names_offset,
            entries,
            type_map,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum AudioSurface {
    NewAsphalt = 0,
    OldAsphalt = 1,
    SideWalk = 2,
    Marble = 3,
    ConcretePark = 4,
    ConcretePavers = 5,
    LargeTilesExterior = 6,
    Brick = 7,
    CobbleStone = 8,
    Tile = 9,
    WoodContestRamp = 10,
    WoodPlywoodDiyRamp = 11,
    Dirt = 12,
    Metal = 13,
    Grass = 14,
    Glass = 15,
    WoodBoardWalk = 16,
    MetalGrate = 17,
    DeepSnow = 18,
    PackedSnow = 19,
    Ice = 20,
}

impl AudioSurface {
    #[must_use]
    pub const fn from_raw(value: u32) -> Option<Self> {
        Some(match value {
            0 => Self::NewAsphalt,
            1 => Self::OldAsphalt,
            2 => Self::SideWalk,
            3 => Self::Marble,
            4 => Self::ConcretePark,
            5 => Self::ConcretePavers,
            6 => Self::LargeTilesExterior,
            7 => Self::Brick,
            8 => Self::CobbleStone,
            9 => Self::Tile,
            10 => Self::WoodContestRamp,
            11 => Self::WoodPlywoodDiyRamp,
            12 => Self::Dirt,
            13 => Self::Metal,
            14 => Self::Grass,
            15 => Self::Glass,
            16 => Self::WoodBoardWalk,
            17 => Self::MetalGrate,
            18 => Self::DeepSnow,
            19 => Self::PackedSnow,
            20 => Self::Ice,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollisionMaterialData {
    pub string_list_offset: u32,
    pub audio_materials: Vec<AudioMaterial>,
    pub physics_materials: Vec<PhysicsMaterial>,
    pub collision_materials: Vec<CollisionMaterial>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioMaterial {
    pub surface_raw: u32,
    pub surface: Option<AudioSurface>,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicsMaterial {
    pub value: i32,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollisionMaterial {
    pub audio_material_offset: Option<u32>,
    pub physics_material_offset: Option<u32>,
    pub name: Option<String>,
}

impl CollisionMaterialData {
    /// Parses inline audio, physics, and collision-material tables.
    ///
    /// # Errors
    ///
    /// Returns [`PegasusError`] for invalid table ranges, links, or strings.
    pub fn parse(bytes: &[u8]) -> Result<Self, PegasusError> {
        require(bytes, 0, C_MATERIAL_DATA_SIZE)?;
        let audio_count = be_u32(bytes, 0)?;
        let physics_count = be_u32(bytes, 4)?;
        let collision_count = be_u32(bytes, 8)?;
        let audio_offset = be_u32(bytes, 12)?;
        let physics_offset = be_u32(bytes, 16)?;
        let collision_offset = be_u32(bytes, 20)?;
        let string_list_offset = be_u32(bytes, 24)?;
        require(bytes, as_usize(string_list_offset)?, 0)?;

        let audio = counted_range(
            bytes,
            as_usize(audio_offset)?,
            audio_count,
            AUDIO_MATERIAL_SIZE,
        )?;
        let mut audio_materials = Vec::with_capacity(as_usize(audio_count)?);
        for index in 0..as_usize(audio_count)? {
            let offset = index * AUDIO_MATERIAL_SIZE;
            let surface_raw = be_u32(audio, offset)?;
            audio_materials.push(AudioMaterial {
                surface_raw,
                surface: AudioSurface::from_raw(surface_raw),
                name: object_string(
                    bytes,
                    be_u32(audio, offset + 4)?,
                    as_usize(audio_offset)? + offset + 4,
                )?,
            });
        }

        let physics = counted_range(
            bytes,
            as_usize(physics_offset)?,
            physics_count,
            PHYSICS_MATERIAL_SIZE,
        )?;
        let mut physics_materials = Vec::with_capacity(as_usize(physics_count)?);
        for index in 0..as_usize(physics_count)? {
            let offset = index * PHYSICS_MATERIAL_SIZE;
            physics_materials.push(PhysicsMaterial {
                value: be_i32(physics, offset)?,
                name: object_string(
                    bytes,
                    be_u32(physics, offset + 4)?,
                    as_usize(physics_offset)? + offset + 4,
                )?,
            });
        }

        let collision = counted_range(
            bytes,
            as_usize(collision_offset)?,
            collision_count,
            C_MATERIAL_SIZE,
        )?;
        let mut collision_materials = Vec::with_capacity(as_usize(collision_count)?);
        for index in 0..as_usize(collision_count)? {
            let offset = index * C_MATERIAL_SIZE;
            let audio_material_offset = nonzero(be_u32(collision, offset)?);
            let physics_material_offset = nonzero(be_u32(collision, offset + 4)?);
            validate_optional_offset(bytes, audio_material_offset)?;
            validate_optional_offset(bytes, physics_material_offset)?;
            let name_offset = nonzero(be_u32(collision, offset + 8)?);
            let name = name_offset
                .map(|value| {
                    object_string(
                        bytes,
                        value,
                        as_usize(collision_offset).unwrap_or(0) + offset + 8,
                    )
                })
                .transpose()?;
            collision_materials.push(CollisionMaterial {
                audio_material_offset,
                physics_material_offset,
                name,
            });
        }

        Ok(Self {
            string_list_offset,
            audio_materials,
            physics_materials,
            collision_materials,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PegasusError {
    Truncated {
        offset: usize,
        required: usize,
        actual: usize,
    },
    IntegerOverflow,
    MissingOffset {
        field_offset: usize,
    },
    UnterminatedString {
        offset: usize,
    },
    InvalidUtf8 {
        offset: usize,
    },
}

impl fmt::Display for PegasusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Skate 2 Pegasus object: {self:?}")
    }
}

impl std::error::Error for PegasusError {}

fn optional_object_id(bytes: &[u8], offset: usize) -> Result<Option<ObjectId>, PegasusError> {
    Ok(nonzero(be_u32(bytes, offset)?).map(ObjectId))
}

fn optional_object_string(
    bytes: &[u8],
    offset: u32,
    field_offset: usize,
) -> Result<Option<String>, PegasusError> {
    nonzero(offset)
        .map(|value| object_string(bytes, value, field_offset))
        .transpose()
}

fn parse_aabb(bytes: &[u8], offset: usize) -> Result<AxisAlignedBox, PegasusError> {
    Ok(AxisAlignedBox {
        minimum: be_f32_array::<4>(bytes, offset)?,
        maximum: be_f32_array::<4>(bytes, offset + 16)?,
    })
}

fn element_at_offset<T>(
    elements: &[T],
    offset: u32,
    base_offset: u32,
    element_size: usize,
) -> Option<&T> {
    let relative = offset.checked_sub(base_offset)?;
    let stride = u32::try_from(element_size).ok()?;
    if relative % stride != 0 {
        return None;
    }
    elements.get(usize::try_from(relative / stride).ok()?)
}

fn element_object_offset(index: usize, base_offset: u32, element_size: usize) -> Option<u32> {
    let index = u32::try_from(index).ok()?;
    let stride = u32::try_from(element_size).ok()?;
    base_offset.checked_add(index.checked_mul(stride)?)
}

const fn nonzero(value: u32) -> Option<u32> {
    if value == 0 { None } else { Some(value) }
}

fn validate_optional_offset(bytes: &[u8], offset: Option<u32>) -> Result<(), PegasusError> {
    if let Some(offset) = offset {
        require(bytes, as_usize(offset)?, 0)?;
    }
    Ok(())
}

fn object_string(bytes: &[u8], offset: u32, field_offset: usize) -> Result<String, PegasusError> {
    let start = as_usize(offset)?;
    let tail = bytes.get(start..).ok_or(PegasusError::Truncated {
        offset: field_offset,
        required: start,
        actual: bytes.len(),
    })?;
    let length = tail
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(PegasusError::UnterminatedString { offset: start })?;
    std::str::from_utf8(&tail[..length])
        .map(str::to_owned)
        .map_err(|_| PegasusError::InvalidUtf8 { offset: start })
}

fn counted_range(
    bytes: &[u8],
    start: usize,
    count: u32,
    stride: usize,
) -> Result<&[u8], PegasusError> {
    let size = as_usize(count)?
        .checked_mul(stride)
        .ok_or(PegasusError::IntegerOverflow)?;
    require(bytes, start, size)
}

fn require(bytes: &[u8], offset: usize, size: usize) -> Result<&[u8], PegasusError> {
    let end = offset
        .checked_add(size)
        .ok_or(PegasusError::IntegerOverflow)?;
    bytes.get(offset..end).ok_or(PegasusError::Truncated {
        offset,
        required: end,
        actual: bytes.len(),
    })
}

fn as_usize(value: u32) -> Result<usize, PegasusError> {
    usize::try_from(value).map_err(|_| PegasusError::IntegerOverflow)
}

fn be_u32(bytes: &[u8], offset: usize) -> Result<u32, PegasusError> {
    require(bytes, offset, 4).map(|field| u32::from_be_bytes(field.try_into().unwrap()))
}

fn be_u64(bytes: &[u8], offset: usize) -> Result<u64, PegasusError> {
    require(bytes, offset, 8).map(|field| u64::from_be_bytes(field.try_into().unwrap()))
}

fn be_i32(bytes: &[u8], offset: usize) -> Result<i32, PegasusError> {
    require(bytes, offset, 4).map(|field| i32::from_be_bytes(field.try_into().unwrap()))
}

fn be_f32(bytes: &[u8], offset: usize) -> Result<f32, PegasusError> {
    be_u32(bytes, offset).map(f32::from_bits)
}

fn be_f32_array<const COUNT: usize>(
    bytes: &[u8],
    offset: usize,
) -> Result<[f32; COUNT], PegasusError> {
    let mut result = [0.0; COUNT];
    for (index, value) in result.iter_mut().enumerate() {
        *value = be_f32(bytes, offset + index * 4)?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    #[test]
    fn object_id_matches_reference_bit_split() {
        let id = ObjectId((7 << 22) | 0x12345);
        assert_eq!(id.section_index(), 7);
        assert_eq!(id.entry_index(), 0x12345);
    }

    #[test]
    fn unpacks_reference_surface_bit_fields() {
        let surface = SurfaceId((11 << 12) | (4 << 7) | 0x48);
        assert_eq!(surface.audio_surface_raw(), 72);
        assert_eq!(surface.physics_surface(), Some(PhysicsSurface::Slippery));
        assert_eq!(surface.pattern_raw(), 11);
    }

    #[test]
    fn parses_instance_links_and_object_relative_strings() {
        let mut bytes = vec![0_u8; 198];
        put_u32(&mut bytes, 0, 0x42);
        put_u32(&mut bytes, 4, 1);
        put_u32(&mut bytes, 8, 4);
        put_u32(&mut bytes, 12, 20);
        put_u32(&mut bytes, 16, 180);
        put_u32(&mut bytes, 20 + 132, 4);
        for (field, offset) in [(140, 180), (144, 185), (148, 190), (152, 194)] {
            put_u32(&mut bytes, 20 + field, offset);
        }
        bytes[180..198].copy_from_slice(b"rail\0desc\0cmp\0cat\0");

        let data = InstanceData::parse(&bytes).unwrap();
        assert_eq!(data.instances[0].collision_model, Some(ObjectId(4)));
        assert_eq!(data.instances[0].name, "rail");
        assert_eq!(data.instances[0].category, "cat");
    }

    #[test]
    fn borrows_instance_records_without_decoding_unread_strings() {
        let mut bytes = vec![0_u8; 20 + 2 * INSTANCE_SIZE];
        put_u32(&mut bytes, 0, INSTANCE_DMO_ASSET_TYPE_ID);
        put_u32(&mut bytes, 4, 2);
        put_u32(&mut bytes, 12, 20);
        put_u64(&mut bytes, 20 + 96, 0x1111_2222_3333_4444);
        put_u64(&mut bytes, 20 + 104, 0x5555_6666_7777_8888);
        put_u32(&mut bytes, 20 + 140, u32::MAX);
        put_u64(&mut bytes, 20 + INSTANCE_SIZE + 96, 0x9999_aaaa_bbbb_cccc);

        let records = InstanceData::record_table(&bytes).unwrap();
        assert_eq!(records.type_id(), INSTANCE_DMO_ASSET_TYPE_ID);
        assert_eq!(records.len(), 2);
        let first = records.record(0).unwrap();
        let second = records.record(1).unwrap();
        assert_eq!(first.object_offset(), 20);
        assert_eq!(first.guid(), 0x1111_2222_3333_4444);
        assert_eq!(first.local_guid(), 0x5555_6666_7777_8888);
        assert_eq!(second.object_offset(), 180);
        assert_eq!(second.guid(), 0x9999_aaaa_bbbb_cccc);
        assert_eq!(first.bytes().len(), INSTANCE_SIZE);
        assert!(records.record(2).is_none());
        assert!(InstanceData::parse(&bytes).is_err());
    }

    #[test]
    fn borrowed_instance_records_validate_header_count_and_empty_span() {
        assert!(InstanceData::record_table(&[0; 19]).is_err());

        let mut truncated = vec![0; 20];
        put_u32(&mut truncated, 4, 1);
        put_u32(&mut truncated, 12, 20);
        assert!(InstanceData::record_table(&truncated).is_err());

        put_u32(&mut truncated, 4, 0);
        let empty = InstanceData::record_table(&truncated).unwrap();
        assert!(empty.is_empty());
        assert!(empty.record(0).is_none());
    }

    #[test]
    fn parses_embedded_grind_and_grab_reference_arrays() {
        let mut bytes = vec![0_u8; 60];
        put_u32(&mut bytes, 0, EMBEDDED_DATA_TYPE_ID);
        put_u32(&mut bytes, 8, 2);
        put_u32(&mut bytes, 12, 1);
        put_u32(&mut bytes, 24, 36);
        put_u32(&mut bytes, 28, 52);
        put_u32(&mut bytes, 36, 10);
        put_u32(&mut bytes, 40, 0x0080_0001);
        put_u32(&mut bytes, 44, 11);
        put_u32(&mut bytes, 48, 0x0080_0002);
        put_u32(&mut bytes, 52, 20);
        put_u32(&mut bytes, 56, 0x0080_0003);

        let data = EmbeddedData::parse(&bytes).unwrap();
        assert_eq!(data.type_id, EMBEDDED_DATA_TYPE_ID);
        assert_eq!(
            data.grind_splines(),
            [
                EmbeddedReference {
                    key: 10,
                    object: ObjectId(0x0080_0001),
                },
                EmbeddedReference {
                    key: 11,
                    object: ObjectId(0x0080_0002),
                },
            ]
        );
        assert_eq!(
            data.grab_splines(),
            [EmbeddedReference {
                key: 20,
                object: ObjectId(0x0080_0003),
            }]
        );
    }

    #[test]
    fn parses_dmo_records_and_rebased_name_pointer() {
        let mut bytes = vec![0_u8; 153];
        put_u32(&mut bytes, 4, 1);
        put_u32(&mut bytes, 12, 20);
        put_u32(&mut bytes, 16, 148);
        put_u64(&mut bytes, 20 + 96, 0x1111_2222_3333_4444);
        put_u64(&mut bytes, 20 + 104, 0x5555_6666_7777_8888);
        put_u64(&mut bytes, 20 + 112, 0x9999_aaaa_bbbb_cccc);
        put_u32(&mut bytes, 20 + 120, 0xdead_beef);
        put_u32(&mut bytes, 20 + 124, 148);
        bytes[148..].copy_from_slice(b"rail\0");

        let data = DmoData::parse(&bytes).unwrap();
        assert_eq!(data.type_id, 0);
        assert_eq!(data.dmos_offset, 20);
        assert_eq!(data.dmos[0].guid, 0x1111_2222_3333_4444);
        assert_eq!(data.dmos[0].local_guid, 0x5555_6666_7777_8888);
        assert_eq!(data.dmos[0].instance_guid, 0x9999_aaaa_bbbb_cccc);
        assert_eq!(data.dmos[0].serialized_instance_word, 0xdead_beef);
        assert_eq!(data.dmos[0].name, "rail");
    }

    #[test]
    fn parses_collision_model_mesh_table() {
        let mut bytes = vec![0_u8; 20];
        put_u32(&mut bytes, 0, 2);
        put_u32(&mut bytes, 4, 1);
        put_u32(&mut bytes, 8, 12);
        put_u32(&mut bytes, 12, 3);
        put_u32(&mut bytes, 16, 0xfeed_beef);
        let model = CollisionModelData::parse(&bytes).unwrap();
        assert_eq!(model.bounding_volume, Some(ObjectId(2)));
        assert_eq!(
            model.meshes,
            CollisionModelMeshes::Meshes(vec![CollisionModelMesh {
                mesh: ObjectId(3),
                bone_name: 0xfeed_beef,
            }])
        );
        assert_eq!(CollisionModelData::raw_mesh_count(&bytes), Ok(1));
    }

    #[test]
    fn collision_model_raw_mesh_count_does_not_validate_unread_fields() {
        let mut bytes = vec![0_u8; 8];
        put_u32(&mut bytes, 4, u32::MAX);
        assert_eq!(CollisionModelData::raw_mesh_count(&bytes), Ok(u32::MAX));
        assert!(CollisionModelData::parse(&bytes).is_err());
    }

    #[test]
    fn parses_spline_arrays_and_internal_offsets() {
        let mut bytes = vec![0_u8; SPLINE_DATA_SIZE + SPLINE_HEADER_SIZE + SPLINE_SEGMENT_SIZE];
        put_u32(&mut bytes, 0, 1);
        put_u32(&mut bytes, 4, 1);
        put_u32(&mut bytes, 8, 16);
        put_u32(&mut bytes, 12, 48);
        put_u32(&mut bytes, 16 + 16, 0x0080_0000);
        put_u32(&mut bytes, 16 + 20, 48);
        put_u32(&mut bytes, 16 + 24, 48);
        put_u32(&mut bytes, 48 + 120, 16);

        let data = SplineData::parse(&bytes).unwrap();
        assert_eq!(data.splines.len(), 1);
        assert_eq!(data.segments.len(), 1);
        assert_eq!(data.spline_index_at_object_offset(16), Some(0));
        assert_eq!(data.spline_at_object_offset(16), data.splines.first());
        assert_eq!(data.segment_at_object_offset(48), data.segments.first());
        assert_eq!(data.segment_index_at_object_offset(48), Some(0));
        for invalid in [0, 47, 49, 48 + SPLINE_SEGMENT_SIZE as u32, u32::MAX] {
            assert_eq!(data.segment_index_at_object_offset(invalid), None);
            assert_eq!(data.segment_at_object_offset(invalid), None);
        }
        assert_eq!(data.splines[0].instance, ObjectId(0x0080_0000));
    }

    #[test]
    fn parses_location_and_sublocation_spans() {
        let mut bytes = vec![0_u8; 282];
        put_u32(&mut bytes, 0, 1);
        put_u32(&mut bytes, 4, 1);
        put_u32(&mut bytes, 8, 4);
        put_u32(&mut bytes, 12, 24);
        put_u32(&mut bytes, 16, 152);
        put_u32(&mut bytes, 20, 264);
        put_u32(&mut bytes, 24 + 96, 1);
        put_u32(&mut bytes, 24 + 100, 152);
        put_u32(&mut bytes, 24 + 104, 264);
        put_u32(&mut bytes, 24 + 108, 268);
        put_u64(&mut bytes, 24 + 112, 0x1234_5678_9abc_def0);
        put_u32(&mut bytes, 152 + 96, 273);
        put_u32(&mut bytes, 152 + 100, 277);
        bytes[264..].copy_from_slice(b"loc\0desc\0sub\0area\0");

        let data = LocationDescData::parse(&bytes).unwrap();
        let location = &data.locations[0];
        assert_eq!(location.name.as_deref(), Some("loc"));
        assert_eq!(location.guid, 0x1234_5678_9abc_def0);
        assert_eq!(data.location_at_object_offset(24), Some(location));
        let sub_locations = data.sub_locations_for(location).unwrap();
        assert_eq!(sub_locations[0].name.as_deref(), Some("sub"));
        assert_eq!(sub_locations[0].description.as_deref(), Some("area"));
    }

    #[test]
    fn parses_trigger_instance_links_and_type() {
        let mut bytes = vec![0_u8; 265];
        put_u32(&mut bytes, 0, TRIGGER_INSTANCE_DATA_TYPE_ID);
        put_u32(&mut bytes, 4, 1);
        put_u32(&mut bytes, 8, 1);
        put_u32(&mut bytes, 12, 20);
        put_u32(&mut bytes, 16, 260);
        put_u64(&mut bytes, 20 + 176, 0xfeed_beef);
        put_u32(&mut bytes, 20 + 216, TriggerInstanceType::Stairs as u32);
        put_u32(&mut bytes, 20 + 220, 7);
        put_u32(&mut bytes, 20 + 224, 260);
        bytes[260..].copy_from_slice(b"gate\0");

        let data = TriggerInstanceData::parse(&bytes).unwrap();
        let trigger = &data.instances[0];
        assert_eq!(trigger.guid, 0xfeed_beef);
        assert_eq!(trigger.trigger_type, Some(TriggerInstanceType::Stairs));
        assert_eq!(trigger.collision_model, Some(ObjectId(7)));
        assert_eq!(trigger.name.as_deref(), Some("gate"));
        assert_eq!(data.instance_at_object_offset(20), Some(trigger));
    }

    #[test]
    fn borrows_trigger_records_without_decoding_unread_names() {
        let mut bytes = vec![0_u8; 20 + TRIGGER_INSTANCE_SIZE];
        put_u32(&mut bytes, 0, 0xdead_beef);
        put_u32(&mut bytes, 4, 1);
        put_u32(&mut bytes, 12, 20);
        put_u64(&mut bytes, 20 + 176, 0x1111_2222_3333_4444);
        put_u64(&mut bytes, 20 + 184, 0xaaaa_bbbb_cccc_dddd);
        put_u32(&mut bytes, 20 + 224, u32::MAX);

        let records = TriggerInstanceData::record_table(&bytes).unwrap();
        assert_eq!(records.len(), 1);
        let record = records.record(0).unwrap();
        assert_eq!(record.object_offset(), 20);
        assert_eq!(record.guid(), 0x1111_2222_3333_4444);
        assert_eq!(record.local_guid(), 0xaaaa_bbbb_cccc_dddd);
        assert_eq!(record.bytes().len(), TRIGGER_INSTANCE_SIZE);
        assert!(records.record(1).is_none());
        assert!(TriggerInstanceData::parse(&bytes).is_err());
    }

    #[test]
    fn borrowed_trigger_records_validate_header_count_and_empty_span() {
        assert!(TriggerInstanceData::record_table(&[0; 19]).is_err());

        let mut truncated = vec![0; 20];
        put_u32(&mut truncated, 4, 1);
        put_u32(&mut truncated, 12, 20);
        assert!(TriggerInstanceData::record_table(&truncated).is_err());

        put_u32(&mut truncated, 4, 0);
        let empty = TriggerInstanceData::record_table(&truncated).unwrap();
        assert!(empty.is_empty());
        assert!(empty.record(0).is_none());
    }
}
