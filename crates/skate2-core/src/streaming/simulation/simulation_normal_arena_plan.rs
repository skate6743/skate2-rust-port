//! Native-order normal Simulation actions expanded from one processed arena.
//!
//! `cProcessArenaAsset::ProcessArena` (`0x82d5fee0`) walks the first
//! used-dictionary TOC forward and invokes each registered processor
//! immediately. This plan merges the already reconstructed kind-6/kind-10
//! static actions with the eight processors whose construction and
//! `Simulation::RegArena` arms are recovered: pointer-backed kinds 11 and 16
//! plus ID-dispatched kinds 22, 23, 25, 27, 28, and 32.
//!
//! The plan deliberately stops at the dispatch boundary. It does not claim to
//! implement `WorldPainter`, `NavMesh`, RAIN, `AIPath`, waypoint, or `SpatialMap`
//! manager internals. Kind-11/16 operations retain an immutable raw arena owner
//! and exact record locator, but do not yet perform their manager effects. It
//! represents successful nominal processor allocation. The global
//! `PreventNavmeshLoadingHack` whole-arena bypass (live flag nonzero, nonempty
//! TOC, and first entry raw type `0x00eb0022`) and native allocation-failure
//! behavior are separate preconditions, not operations in this plan.
//!
//! `cProcessArenaAsset::ProcessArena` initializes `cArena::mCost` to zero.
//! `ProcessInstanceList` then wrapping-adds one only for each Environment
//! instance that survives the same collision-model/render-model exclusion
//! used to create [`SimulationNormalOperation::Environment`]. The retained
//! operation list can therefore reproduce that processed Presentation cost
//! without inventing serialized metadata.

use crate::simulation_static_arena_plan::{
    SimulationStaticArenaPlan, SimulationStaticArenaPlanError, SimulationStaticOperation,
};
use skate2_formats::{
    pegasus::{
        INSTANCE_DATA_TYPE_ID, INSTANCE_DMO_ASSET_TYPE_ID, InstanceData, ObjectId, PegasusError,
        TABLE_OF_CONTENTS_TYPE_ID, TRIGGER_INSTANCE_DATA_TYPE_ID, TableOfContents, TocEntry,
        TriggerInstanceData,
    },
    rw4::{Arena, DictionaryEntry},
};
use std::{error::Error, fmt, sync::Arc};

/// Raw processed object types registered with the recovered arena processors.
pub const WORLD_PAINTER_DATA_TYPE_ID_0F: u32 = 0x00eb_000f;
pub const WORLD_PAINTER_DATA_TYPE_ID_10: u32 = 0x00eb_0010;
pub const WORLD_PAINTER_DATA_TYPE_ID_11: u32 = 0x00eb_0011;
pub const RAIN_DATA_TYPE_ID: u32 = 0x00eb_0013;
pub const AI_PATH_DATA_TYPE_ID: u32 = 0x00eb_0014;
pub const WAYPOINT_DATA_TYPE_ID: u32 = 0x00eb_001a;
pub const SPATIAL_MAP_DATA_TYPE_ID: u32 = 0x00eb_0020;
pub const NAV_MESH_2_DATA_TYPE_ID: u32 = 0x00eb_0022;
/// Skate 3's `RWOBJECTTYPE_NAVPOWERDATA`, which its Sim tiles carry in place of NavMesh2: a
/// linked Skate 3 data folder's pedestrian navigation (see `living_world_nav_power`).
pub const NAV_POWER_DATA_TYPE_ID: u32 = 0x00eb_0027;

/// One owned arena and its currently reconstructed normal `RegArena` slice.
///
/// The retained static plan owns the complete decoded arena and every
/// pointer-backed static source. ID-dispatched operations retain exact source
/// locators as diagnostics, while their runtime boundary uses the full TOC
/// GUID just like the reference.
#[derive(Debug)]
pub struct SimulationNormalArenaPlan {
    static_plan: Arc<SimulationStaticArenaPlan>,
    operations: Vec<SimulationNormalOperation>,
    record_sources: Vec<Option<SimulationRecordLocator>>,
}

impl SimulationNormalArenaPlan {
    /// Builds the retained static plan and then merges every currently
    /// supported normal action into its authored position.
    ///
    /// # Errors
    ///
    /// Returns either the underlying static-plan error or a normal-plan merge
    /// error, before any runtime manager effect exists.
    pub fn from_collision_asset(
        asset: crate::simulation_collision_asset::SimulationCollisionAsset,
    ) -> Result<Self, SimulationNormalArenaPlanError> {
        let static_plan = Arc::new(SimulationStaticArenaPlan::from_collision_asset(asset)?);
        Self::from_static_plan(static_plan)
    }

    /// Merges the supported processed actions in exact authored TOC order.
    ///
    /// # Errors
    ///
    /// Returns a typed error if the first TOC or an ID-dispatched processor target
    /// lies outside retained storage, uses a nonlocal object ID, disagrees
    /// with its dictionary type, or if the supplied static plan cannot be
    /// merged monotonically into that same TOC.
    pub fn from_static_plan(
        static_plan: Arc<SimulationStaticArenaPlan>,
    ) -> Result<Self, SimulationNormalArenaPlanError> {
        let built = build_operations(&static_plan)?;
        Ok(Self {
            static_plan,
            operations: built.operations,
            record_sources: built.record_sources,
        })
    }

    #[must_use]
    pub fn static_plan(&self) -> &SimulationStaticArenaPlan {
        &self.static_plan
    }

    /// Clones the immutable backing owner shared by every pointer-equivalent
    /// record or object handle derived from this normal plan.
    #[must_use]
    pub fn static_plan_owner(&self) -> Arc<SimulationStaticArenaPlan> {
        Arc::clone(&self.static_plan)
    }

    #[must_use]
    pub fn operations(&self) -> &[SimulationNormalOperation] {
        &self.operations
    }

    /// Returns the processed `cArena::mCost` consumed by deferred Presentation
    /// registration.
    ///
    /// Native construction clears the field at `0x82d60328` (and the empty
    /// fallback at `0x82d60424`). `ProcessInstanceList` increments its local
    /// count only at `0x82d5ef38` for an admitted Environment instance, then
    /// wrapping-adds that count into `mCost` at `0x82d5ef50..0x82d5ef58`.
    /// Every other instance subtype adds zero at its corresponding exit.
    #[must_use]
    pub fn presentation_arena_cost(&self) -> u32 {
        self.operations.iter().fold(0_u32, |cost, operation| {
            cost.wrapping_add(u32::from(matches!(
                operation,
                SimulationNormalOperation::Environment { .. }
            )))
        })
    }

    /// Borrows the exact retained record backing a kind-11 or kind-16
    /// operation. ID-only and static operations return `None`.
    ///
    /// The borrow cannot outlive this plan. [`SimulationRecordSource::is_same_backing_record`]
    /// compares the immutable owner and byte locator, preserving native pointer
    /// aliases without exposing or persisting a host address.
    #[must_use]
    pub fn record_source(&self, operation_index: usize) -> Option<SimulationRecordSource<'_>> {
        let locator = self.record_sources.get(operation_index)?.as_ref()?;
        let bytes = self
            .static_plan
            .asset()
            .decoded
            .resources
            .resources
            .get(locator.resource_index)?
            .get(locator.record_start..locator.record_start.checked_add(locator.record_len)?)?;
        Some(SimulationRecordSource {
            owner: &self.static_plan,
            kind: locator.kind,
            bytes,
            locator,
        })
    }

    /// Clones an owning authority for one pointer-backed record.
    ///
    /// Unlike [`Self::record_source`], this handle may be retained by a
    /// manager. It keeps the immutable arena storage alive and compares
    /// identity by owner, resource, and record start rather than by value.
    #[must_use]
    pub fn record_handle(&self, operation_index: usize) -> Option<SimulationRecordHandle> {
        let locator = *self.record_sources.get(operation_index)?.as_ref()?;
        Some(SimulationRecordHandle {
            owner: Arc::clone(&self.static_plan),
            kind: locator.kind,
            locator,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_parts_with_record_sources_for_test(
        static_plan: Arc<SimulationStaticArenaPlan>,
        operations: Vec<SimulationNormalOperation>,
        record_sources: Vec<Option<(SimulationRecordSourceKind, usize, usize, usize)>>,
    ) -> Self {
        assert_eq!(
            operations.len(),
            record_sources.len(),
            "test operations and record sources remain positional"
        );
        Self {
            static_plan,
            operations,
            record_sources: record_sources
                .into_iter()
                .map(|source| {
                    source.map(|(kind, resource_index, record_start, record_len)| {
                        SimulationRecordLocator {
                            kind,
                            resource_index,
                            record_start,
                            record_len,
                        }
                    })
                })
                .collect(),
        }
    }
}

/// One action in the reconstructed normal `Simulation::RegArena` subsequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SimulationNormalOperation {
    /// Kind 6, addressed through the retained static plan.
    Grind { static_operation_index: usize },
    /// Kind 10, addressed through the retained static plan.
    Environment { static_operation_index: usize },
    /// One `Instance_DMO` child. The emitted kind-11 ID is the primary GUID;
    /// later DMO bookkeeping also observes `local_guid` and exact record
    /// identity.
    DmoInstance {
        asset_id: u64,
        local_guid: u64,
        source_toc_index: u32,
        source_dictionary_index: u32,
        source_inline_index: u32,
        source_record_offset: u32,
        raw_object_id: u32,
    },
    /// One trigger child. The emitted kind-16 ID is the local GUID, not the
    /// primary GUID retained here for diagnostics.
    TriggerInstance {
        asset_id: u64,
        primary_guid: u64,
        source_toc_index: u32,
        source_dictionary_index: u32,
        source_inline_index: u32,
        source_record_offset: u32,
        raw_object_id: u32,
    },
    /// One processor-created asset whose runtime manager receives only its
    /// full 64-bit asset ID.
    RegistryAsset {
        kind: SimulationRegistryAssetKind,
        asset_id: u64,
        source_toc_index: u32,
        source_dictionary_index: u32,
        raw_object_type: u32,
        raw_object_id: u32,
    },
}

impl SimulationNormalOperation {
    #[must_use]
    pub const fn simulation_kind(self) -> u32 {
        match self {
            Self::Grind { .. } => 6,
            Self::Environment { .. } => 10,
            Self::DmoInstance { .. } => 11,
            Self::TriggerInstance { .. } => 16,
            Self::RegistryAsset { kind, .. } => kind as u32,
        }
    }
}

/// Serialized record shape carried by an owner-bound source borrow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationRecordSourceKind {
    DmoInstance,
    TriggerInstance,
}

/// Short-lived view of one pointer-backed operation's exact retained bytes.
pub struct SimulationRecordSource<'a> {
    owner: &'a Arc<SimulationStaticArenaPlan>,
    kind: SimulationRecordSourceKind,
    bytes: &'a [u8],
    locator: &'a SimulationRecordLocator,
}

/// Cloneable owning authority for one exact pointer-backed record.
///
/// Cloning this value preserves one native-pointer-equivalent identity and
/// retains its immutable arena storage. It does not copy the serialized record.
#[derive(Clone)]
pub struct SimulationRecordHandle {
    owner: Arc<SimulationStaticArenaPlan>,
    kind: SimulationRecordSourceKind,
    locator: SimulationRecordLocator,
}

impl fmt::Debug for SimulationRecordHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationRecordHandle")
            .field("kind", &self.kind)
            .field("resource_index", &self.locator.resource_index)
            .field("record_start", &self.locator.record_start)
            .field("record_len", &self.locator.record_len)
            .finish_non_exhaustive()
    }
}

impl SimulationRecordHandle {
    /// TriggerVolumeUtils::IsPointInTriggerVolume82e216e0, with its existing
    /// arena collision projection resolving the serialized model reference.
    pub(crate) fn trigger_contains_point(&self, point: [f32; 4]) -> Result<bool, String> {
        let (transform, radius) = self.trigger_volume_geometry()?;
        let mut minimum = [0.0; 3];
        let mut maximum = [0.0; 3];
        for lane in 0..3 {
            let x = transform[lane] * radius[0];
            let y = transform[4 + lane] * radius[1];
            let z = transform[8 + lane] * radius[2];
            // Source subtracts/adds signed axes; it does not abs() them or
            // inverse-transform the point into an oriented box.
            minimum[lane] = ((transform[12 + lane] - x) - y) - z;
            maximum[lane] = ((transform[12 + lane] + x) + y) + z;
        }
        Ok(point[0] >= minimum[0] && point[1] >= minimum[1] && point[2] >= minimum[2]
            && maximum[0] >= point[0] && maximum[1] >= point[1] && maximum[2] >= point[2])
    }

    /// Shared ComputeTriggerVolumeTransform82e21628 and
    /// ComputerTriggerVolumeRadius82e21508, also used by pedestrian queries.
    pub(crate) fn trigger_volume_geometry(&self) -> Result<([f32; 16], [f32; 3]), String> {
        if self.kind != SimulationRecordSourceKind::TriggerInstance {
            return Err("trigger point query requires a trigger record".into());
        }
        let bytes = self.bytes();
        let word = |offset: usize| -> Result<u32, String> {
            Ok(u32::from_be_bytes(bytes.get(offset..offset + 4)
                .ok_or("truncated trigger record")?.try_into().unwrap()))
        };
        let id = skate2_formats::pegasus::ObjectId(word(220)?);
        // ComputeTriggerVolumeTransform82e21628 dereferences the bounding
        // volume: a missing model is not permission to substitute an AABB.
        if id.0 == 0 || id.section_index() != 0 {
            return Err(format!("trigger bounding model unavailable: {:#x}", id.0));
        }
        let collision = &self.owner.asset().collision;
        let model = collision.collision_models.iter()
            .find(|model| model.dictionary_index == id.entry_index())
            .ok_or("trigger collision model is not projected")?;
        let volume = &collision.aggregate_roots.get(model.bounding_root
            .ok_or("trigger model has no bounding volume")?)
            .ok_or("trigger bounding volume is not projected")?.volume;
        let mut transform = [0.0; 16];
        for (i, lane) in transform.iter_mut().enumerate() {
            *lane = f32::from_bits(word(i * 4)?);
        }
        let transform = crate::world::compose_collision_affine(transform, volume.transform);
        // ComputerTriggerVolumeRadius82e21508: Box reads XYZ half lengths;
        // every other volume broadcasts radius into XYZ (raw PPC verified).
        let radius = if volume.volume_type() == Some(skate2_formats::collision::VolumeType::Box) {
            volume.shape_words.map(f32::from_bits)
        } else { [volume.radius; 3] };
        Ok((transform, radius))
    }

    /// `TriggerVolumeUtils::IsColldingWithTriggerVolume`'s trigger operands:
    /// the instance's `m_pCModel` bounding volume and `m_TransformMatrix`.
    /// `None` when the instance has no collision model.
    pub(crate) fn trigger_bounding_volume(
        &self,
    ) -> Result<Option<(skate2_formats::collision::Volume, [f32; 16])>, String> {
        if self.kind != SimulationRecordSourceKind::TriggerInstance {
            return Err("trigger collision requires a trigger record".into());
        }
        let bytes = self.bytes();
        let word = |offset: usize| -> Result<u32, String> {
            Ok(u32::from_be_bytes(bytes.get(offset..offset + 4)
                .ok_or("truncated trigger record")?.try_into().unwrap()))
        };
        let id = skate2_formats::pegasus::ObjectId(word(220)?);
        if id.0 == 0 {
            return Ok(None);
        }
        if id.section_index() != 0 {
            return Err(format!("trigger collision model unavailable: {:#x}", id.0));
        }
        let collision = &self.owner.asset().collision;
        let model = collision.collision_models.iter()
            .find(|model| model.dictionary_index == id.entry_index())
            .ok_or("trigger collision model is not projected")?;
        let Some(root) = model.bounding_root else { return Ok(None) };
        let volume = collision.aggregate_roots.get(root)
            .ok_or("trigger bounding volume is not projected")?.volume.clone();
        let mut transform = [0.0; 16];
        for (i, lane) in transform.iter_mut().enumerate() {
            *lane = f32::from_bits(word(i * 4)?);
        }
        Ok(Some((volume, transform)))
    }

    /// `TriggerVolumeUtils::ComputeTriggerVolumeCenter` 0x82e21438: the bounding volume's
    /// translation through `m_TransformMatrix`, or without one the `m_BBox` centre.
    pub(crate) fn trigger_volume_center(&self) -> Result<[f32; 3], String> {
        if let Some((volume, transform)) = self.trigger_bounding_volume()? {
            let local = [volume.transform[12], volume.transform[13], volume.transform[14]];
            return Ok(std::array::from_fn(|lane| {
                let x = local[0].mul_add(transform[lane], transform[12 + lane]);
                let y = local[1].mul_add(transform[4 + lane], x);
                local[2].mul_add(transform[8 + lane], y)
            }));
        }
        let bytes = self.bytes();
        let lane = |offset: usize| -> Result<f32, String> {
            Ok(f32::from_bits(u32::from_be_bytes(bytes.get(offset..offset + 4)
                .ok_or("truncated trigger record")?.try_into().unwrap())))
        };
        // m_BBox (+0x40 minimum, +0x50 maximum) times vrefp(2) refined: exactly a half.
        let mut centre = [0.0; 3];
        for (index, value) in centre.iter_mut().enumerate() {
            *value = (lane(0x40 + index * 4)? + lane(0x50 + index * 4)?) * 0.5;
        }
        Ok(centre)
    }

    /// `tTriggerInstance::m_uiGuid` and `m_uiGuidLocal`.
    pub(crate) fn trigger_guids(&self) -> Result<(u64, u64), String> {
        let bytes = self.bytes();
        let read = |offset: usize| -> Result<u64, String> {
            Ok(u64::from_be_bytes(bytes.get(offset..offset + 8)
                .ok_or("truncated trigger record")?.try_into().unwrap()))
        };
        Ok((read(176)?, read(184)?))
    }

    #[must_use]
    pub const fn kind(&self) -> SimulationRecordSourceKind {
        self.kind
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.record_bytes()
    }

    /// Tests native-pointer-equivalent identity without exposing a host
    /// address. Equal record values at different starts or owners are not the
    /// same record.
    #[must_use]
    pub fn is_same_backing_record(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.locator.resource_index == other.locator.resource_index
            && self.locator.record_start == other.locator.record_start
    }

    fn record_bytes(&self) -> &[u8] {
        &self.owner.asset().decoded.resources.resources[self.locator.resource_index]
            [self.locator.record_start..self.locator.record_start + self.locator.record_len]
    }
}

impl SimulationRecordSource<'_> {
    #[must_use]
    pub const fn kind(&self) -> SimulationRecordSourceKind {
        self.kind
    }

    #[must_use]
    pub const fn bytes(&self) -> &[u8] {
        self.bytes
    }

    /// Tests native-pointer-equivalent identity: the same immutable arena
    /// owner, resource, and record start. Equal record values at different
    /// starts are intentionally not identical.
    #[must_use]
    pub fn is_same_backing_record(&self, other: &Self) -> bool {
        Arc::ptr_eq(self.owner, other.owner)
            && self.locator.resource_index == other.locator.resource_index
            && self.locator.record_start == other.locator.record_start
    }
}

#[derive(Clone, Copy, Debug)]
struct SimulationRecordLocator {
    kind: SimulationRecordSourceKind,
    resource_index: usize,
    record_start: usize,
    record_len: usize,
}

struct BuiltOperations {
    operations: Vec<SimulationNormalOperation>,
    record_sources: Vec<Option<SimulationRecordLocator>>,
}

/// ID-only manager route selected by `Simulation::RegArena`/`DRegArena`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SimulationRegistryAssetKind {
    WorldPainter = 22,
    NavMesh = 23,
    Rain = 25,
    AiPath = 27,
    Waypoint = 28,
    SpatialMap = 32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationNormalArenaPlanError {
    Static(SimulationStaticArenaPlanError),
    OperationStorageAllocation {
        requested: usize,
    },
    TocIndexOverflow(usize),
    InlineIndexOverflow(usize),
    NonLocalObjectReference {
        toc_index: u32,
        object_id: u32,
        section: u16,
    },
    MissingLocalObjectReference {
        toc_index: u32,
        object_id: u32,
        dictionary_index: u32,
    },
    WrongLocalObjectType {
        toc_index: u32,
        object_id: u32,
        expected: u32,
        actual: u32,
    },
    ObjectRangeOutsideResource {
        dictionary_index: u32,
        type_id: u32,
    },
    StaticOperationOutsideToc {
        static_operation_index: usize,
        source_toc_index: u32,
        toc_entry_count: usize,
    },
    StaticOperationOrder {
        static_operation_index: usize,
        previous_toc_index: u32,
        source_toc_index: u32,
    },
    Pegasus(PegasusError),
}

impl fmt::Display for SimulationNormalArenaPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(source) => write!(formatter, "static Simulation plan failed: {source}"),
            Self::OperationStorageAllocation { requested } => write!(
                formatter,
                "could not allocate {requested} normal Simulation arena operations"
            ),
            Self::TocIndexOverflow(index) => {
                write!(formatter, "TOC entry index {index} does not fit in u32")
            }
            Self::InlineIndexOverflow(index) => {
                write!(formatter, "inline record index {index} does not fit in u32")
            }
            Self::NonLocalObjectReference {
                toc_index,
                object_id,
                section,
            } => write!(
                formatter,
                "TOC entry {toc_index} uses unsupported nonlocal object ID {object_id:08x} in section {section}"
            ),
            Self::MissingLocalObjectReference {
                toc_index,
                object_id,
                dictionary_index,
            } => write!(
                formatter,
                "TOC entry {toc_index} object ID {object_id:08x} targets missing local entry {dictionary_index}"
            ),
            Self::WrongLocalObjectType {
                toc_index,
                object_id,
                expected,
                actual,
            } => write!(
                formatter,
                "TOC entry {toc_index} object ID {object_id:08x} has type {actual:08x}, expected {expected:08x}"
            ),
            Self::ObjectRangeOutsideResource {
                dictionary_index,
                type_id,
            } => write!(
                formatter,
                "dictionary entry {dictionary_index} type {type_id:08x} is outside its retained resource"
            ),
            Self::StaticOperationOutsideToc {
                static_operation_index,
                source_toc_index,
                toc_entry_count,
            } => write!(
                formatter,
                "static operation {static_operation_index} references TOC entry {source_toc_index}, but the TOC has {toc_entry_count} entries"
            ),
            Self::StaticOperationOrder {
                static_operation_index,
                previous_toc_index,
                source_toc_index,
            } => write!(
                formatter,
                "static operation {static_operation_index} moves backward from TOC entry {previous_toc_index} to {source_toc_index}"
            ),
            Self::Pegasus(source) => {
                write!(formatter, "processed Pegasus data is invalid: {source}")
            }
        }
    }
}

impl Error for SimulationNormalArenaPlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Static(source) => Some(source),
            Self::Pegasus(source) => Some(source),
            _ => None,
        }
    }
}

impl From<PegasusError> for SimulationNormalArenaPlanError {
    fn from(source: PegasusError) -> Self {
        Self::Pegasus(source)
    }
}

impl From<SimulationStaticArenaPlanError> for SimulationNormalArenaPlanError {
    fn from(source: SimulationStaticArenaPlanError) -> Self {
        Self::Static(source)
    }
}

#[allow(clippy::too_many_lines)] // Keep the single authored-order TOC merge visible.
fn build_operations(
    static_plan: &SimulationStaticArenaPlan,
) -> Result<BuiltOperations, SimulationNormalArenaPlanError> {
    validate_static_operation_order(static_plan.operations())?;
    let arena = &static_plan.asset().decoded.arena;
    let resources = &static_plan.asset().decoded.resources.resources;
    let Some((toc_dictionary_index, toc_entry)) = arena
        .dictionary
        .iter()
        .enumerate()
        .find(|(_, entry)| entry.type_id == TABLE_OF_CONTENTS_TYPE_ID)
    else {
        if let Some((static_operation_index, operation)) =
            static_plan.operations().iter().enumerate().next()
        {
            return Err(SimulationNormalArenaPlanError::StaticOperationOutsideToc {
                static_operation_index,
                source_toc_index: static_source_toc_index(operation),
                toc_entry_count: 0,
            });
        }
        return Ok(BuiltOperations {
            operations: Vec::new(),
            record_sources: Vec::new(),
        });
    };
    let toc = TableOfContents::parse(object_bytes(
        arena,
        resources,
        toc_dictionary_index,
        toc_entry,
    )?)?;
    let mut operations = Vec::new();
    let mut record_sources = Vec::new();
    let reserve = toc
        .entries
        .len()
        .saturating_add(static_plan.operations().len());
    operations.try_reserve(reserve).map_err(|_| {
        SimulationNormalArenaPlanError::OperationStorageAllocation { requested: reserve }
    })?;
    record_sources.try_reserve(reserve).map_err(|_| {
        SimulationNormalArenaPlanError::OperationStorageAllocation { requested: reserve }
    })?;
    let mut static_cursor = 0_usize;

    for (toc_index, toc_entry) in toc.entries.iter().enumerate() {
        let source_toc_index = u32::try_from(toc_index)
            .map_err(|_| SimulationNormalArenaPlanError::TocIndexOverflow(toc_index))?;
        while let Some(operation) = static_plan.operations().get(static_cursor) {
            if static_source_toc_index(operation) != source_toc_index {
                break;
            }
            let operation = match operation {
                SimulationStaticOperation::Grind { .. } => SimulationNormalOperation::Grind {
                    static_operation_index: static_cursor,
                },
                SimulationStaticOperation::Environment { .. } => {
                    SimulationNormalOperation::Environment {
                        static_operation_index: static_cursor,
                    }
                }
            };
            push_operation(&mut operations, &mut record_sources, operation, None)?;
            static_cursor += 1;
        }

        if toc_entry.object_type == INSTANCE_DATA_TYPE_ID {
            append_dmo_instance_operations(
                arena,
                resources,
                source_toc_index,
                toc_entry,
                &mut operations,
                &mut record_sources,
            )?;
        } else if toc_entry.object_type == TRIGGER_INSTANCE_DATA_TYPE_ID {
            append_trigger_instance_operations(
                arena,
                resources,
                source_toc_index,
                toc_entry,
                &mut operations,
                &mut record_sources,
            )?;
        } else if let Some(kind) = registry_kind(toc_entry.object_type) {
            let source_dictionary_index = validate_registry_source(
                arena,
                resources,
                source_toc_index,
                toc_entry.object,
                toc_entry.object_type,
            )?;
            push_operation(
                &mut operations,
                &mut record_sources,
                SimulationNormalOperation::RegistryAsset {
                    kind,
                    asset_id: toc_entry.guid,
                    source_toc_index,
                    source_dictionary_index,
                    raw_object_type: toc_entry.object_type,
                    raw_object_id: toc_entry.object.0,
                },
                None,
            )?;
        }
    }

    if let Some(operation) = static_plan.operations().get(static_cursor) {
        return Err(SimulationNormalArenaPlanError::StaticOperationOutsideToc {
            static_operation_index: static_cursor,
            source_toc_index: static_source_toc_index(operation),
            toc_entry_count: toc.entries.len(),
        });
    }
    Ok(BuiltOperations {
        operations,
        record_sources,
    })
}

fn append_dmo_instance_operations(
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    source_toc_index: u32,
    toc_entry: &TocEntry,
    operations: &mut Vec<SimulationNormalOperation>,
    record_sources: &mut Vec<Option<SimulationRecordLocator>>,
) -> Result<(), SimulationNormalArenaPlanError> {
    let source_dictionary_index = validate_registry_source(
        arena,
        resources,
        source_toc_index,
        toc_entry.object,
        INSTANCE_DATA_TYPE_ID,
    )?;
    let source_entry = &arena.dictionary[source_dictionary_index as usize];
    let bytes = object_bytes(
        arena,
        resources,
        source_dictionary_index as usize,
        source_entry,
    )?;
    let records = InstanceData::record_table(bytes)?;
    if records.type_id() != INSTANCE_DMO_ASSET_TYPE_ID {
        return Ok(());
    }
    for inline_index in 0..records.len() {
        let source_inline_index = u32::try_from(inline_index)
            .map_err(|_| SimulationNormalArenaPlanError::InlineIndexOverflow(inline_index))?;
        let record = records
            .record(inline_index)
            .ok_or(PegasusError::IntegerOverflow)?;
        let locator = record_locator(
            source_entry,
            record.object_offset(),
            record.bytes().len(),
            SimulationRecordSourceKind::DmoInstance,
            source_dictionary_index,
        )?;
        push_operation(
            operations,
            record_sources,
            SimulationNormalOperation::DmoInstance {
                asset_id: record.guid(),
                local_guid: record.local_guid(),
                source_toc_index,
                source_dictionary_index,
                source_inline_index,
                source_record_offset: record.object_offset(),
                raw_object_id: toc_entry.object.0,
            },
            Some(locator),
        )?;
    }
    Ok(())
}

fn append_trigger_instance_operations(
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    source_toc_index: u32,
    toc_entry: &TocEntry,
    operations: &mut Vec<SimulationNormalOperation>,
    record_sources: &mut Vec<Option<SimulationRecordLocator>>,
) -> Result<(), SimulationNormalArenaPlanError> {
    let source_dictionary_index = validate_registry_source(
        arena,
        resources,
        source_toc_index,
        toc_entry.object,
        TRIGGER_INSTANCE_DATA_TYPE_ID,
    )?;
    let source_entry = &arena.dictionary[source_dictionary_index as usize];
    let bytes = object_bytes(
        arena,
        resources,
        source_dictionary_index as usize,
        source_entry,
    )?;
    let records = TriggerInstanceData::record_table(bytes)?;
    for inline_index in 0..records.len() {
        let source_inline_index = u32::try_from(inline_index)
            .map_err(|_| SimulationNormalArenaPlanError::InlineIndexOverflow(inline_index))?;
        let record = records
            .record(inline_index)
            .ok_or(PegasusError::IntegerOverflow)?;
        let locator = record_locator(
            source_entry,
            record.object_offset(),
            record.bytes().len(),
            SimulationRecordSourceKind::TriggerInstance,
            source_dictionary_index,
        )?;
        push_operation(
            operations,
            record_sources,
            SimulationNormalOperation::TriggerInstance {
                asset_id: record.local_guid(),
                primary_guid: record.guid(),
                source_toc_index,
                source_dictionary_index,
                source_inline_index,
                source_record_offset: record.object_offset(),
                raw_object_id: toc_entry.object.0,
            },
            Some(locator),
        )?;
    }
    Ok(())
}

fn push_operation(
    operations: &mut Vec<SimulationNormalOperation>,
    record_sources: &mut Vec<Option<SimulationRecordLocator>>,
    operation: SimulationNormalOperation,
    record_source: Option<SimulationRecordLocator>,
) -> Result<(), SimulationNormalArenaPlanError> {
    let requested = operations.len().saturating_add(1);
    operations
        .try_reserve(1)
        .map_err(|_| SimulationNormalArenaPlanError::OperationStorageAllocation { requested })?;
    record_sources
        .try_reserve(1)
        .map_err(|_| SimulationNormalArenaPlanError::OperationStorageAllocation { requested })?;
    operations.push(operation);
    record_sources.push(record_source);
    Ok(())
}

fn record_locator(
    entry: &DictionaryEntry,
    object_offset: u32,
    record_len: usize,
    kind: SimulationRecordSourceKind,
    dictionary_index: u32,
) -> Result<SimulationRecordLocator, SimulationNormalArenaPlanError> {
    let record_start = entry
        .object_range
        .start
        .checked_add(object_offset as usize)
        .ok_or(SimulationNormalArenaPlanError::ObjectRangeOutsideResource {
            dictionary_index,
            type_id: entry.type_id,
        })?;
    let record_end = record_start.checked_add(record_len).ok_or(
        SimulationNormalArenaPlanError::ObjectRangeOutsideResource {
            dictionary_index,
            type_id: entry.type_id,
        },
    )?;
    if record_end > entry.object_range.end {
        return Err(SimulationNormalArenaPlanError::ObjectRangeOutsideResource {
            dictionary_index,
            type_id: entry.type_id,
        });
    }
    Ok(SimulationRecordLocator {
        kind,
        resource_index: entry.resource_index,
        record_start,
        record_len,
    })
}

fn validate_static_operation_order(
    operations: &[SimulationStaticOperation],
) -> Result<(), SimulationNormalArenaPlanError> {
    let mut previous = None;
    for (index, operation) in operations.iter().enumerate() {
        let current = static_source_toc_index(operation);
        if let Some(previous_toc_index) = previous
            && current < previous_toc_index
        {
            return Err(SimulationNormalArenaPlanError::StaticOperationOrder {
                static_operation_index: index,
                previous_toc_index,
                source_toc_index: current,
            });
        }
        previous = Some(current);
    }
    Ok(())
}

const fn static_source_toc_index(operation: &SimulationStaticOperation) -> u32 {
    match operation {
        SimulationStaticOperation::Grind {
            source_toc_index, ..
        }
        | SimulationStaticOperation::Environment {
            source_toc_index, ..
        } => *source_toc_index,
    }
}

const fn registry_kind(raw_object_type: u32) -> Option<SimulationRegistryAssetKind> {
    Some(match raw_object_type {
        WORLD_PAINTER_DATA_TYPE_ID_0F
        | WORLD_PAINTER_DATA_TYPE_ID_10
        | WORLD_PAINTER_DATA_TYPE_ID_11 => SimulationRegistryAssetKind::WorldPainter,
        // PC port: Skate 3's own RegArena hands a NavPower graph to its NavPower runtime; here it
        // joins the MeshStore like a NavMesh2 tile.
        NAV_MESH_2_DATA_TYPE_ID | NAV_POWER_DATA_TYPE_ID => SimulationRegistryAssetKind::NavMesh,
        RAIN_DATA_TYPE_ID => SimulationRegistryAssetKind::Rain,
        AI_PATH_DATA_TYPE_ID => SimulationRegistryAssetKind::AiPath,
        WAYPOINT_DATA_TYPE_ID => SimulationRegistryAssetKind::Waypoint,
        SPATIAL_MAP_DATA_TYPE_ID => SimulationRegistryAssetKind::SpatialMap,
        _ => return None,
    })
}

fn validate_registry_source(
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    toc_index: u32,
    object: ObjectId,
    expected: u32,
) -> Result<u32, SimulationNormalArenaPlanError> {
    let section = object.section_index();
    if section != 0 {
        return Err(SimulationNormalArenaPlanError::NonLocalObjectReference {
            toc_index,
            object_id: object.0,
            section,
        });
    }
    let dictionary_index = object.entry_index();
    let entry = arena.dictionary.get(dictionary_index as usize).ok_or(
        SimulationNormalArenaPlanError::MissingLocalObjectReference {
            toc_index,
            object_id: object.0,
            dictionary_index,
        },
    )?;
    if entry.type_id != expected {
        return Err(SimulationNormalArenaPlanError::WrongLocalObjectType {
            toc_index,
            object_id: object.0,
            expected,
            actual: entry.type_id,
        });
    }
    object_bytes(arena, resources, dictionary_index as usize, entry)?;
    Ok(dictionary_index)
}

fn object_bytes<'a>(
    arena: &Arena,
    resources: &'a [Vec<u8>; 5],
    dictionary_index: usize,
    entry: &DictionaryEntry,
) -> Result<&'a [u8], SimulationNormalArenaPlanError> {
    arena.object_bytes(resources, entry).ok_or_else(|| {
        SimulationNormalArenaPlanError::ObjectRangeOutsideResource {
            dictionary_index: u32::try_from(dictionary_index).unwrap_or(u32::MAX),
            type_id: entry.type_id,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        simulation_collision_asset::SimulationCollisionAsset,
        world::CollisionArena,
    };
    use skate2_formats::{
        pegasus::{
            DMO_DATA_TYPE_ID, INSTANCE_DATA_TYPE_ID, LOCATION_DESC_DATA_TYPE_ID,
            SPLINE_DATA_TYPE_ID,
        },
        rw4::{FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::ops::Range;

    struct ArenaFixture {
        bytes: Vec<u8>,
        dictionary: Vec<DictionaryEntry>,
    }

    impl ArenaFixture {
        fn new() -> Self {
            Self {
                bytes: Vec::new(),
                dictionary: Vec::new(),
            }
        }

        fn push(&mut self, type_id: u32, object: &[u8]) -> ObjectId {
            let index = self.dictionary.len();
            let start = self.bytes.len();
            self.bytes.extend_from_slice(object);
            let end = self.bytes.len();
            self.dictionary.push(DictionaryEntry {
                resource_index: 0,
                object_offset: u32::try_from(start).unwrap(),
                relocation_offset: 0,
                size: u32::try_from(object.len()).unwrap(),
                alignment: 1,
                type_index: 0,
                type_id,
                object_range: Range { start, end },
            });
            ObjectId(u32::try_from(index).unwrap())
        }

        fn alias(&mut self, object: ObjectId) -> ObjectId {
            let entry = self.dictionary[object.entry_index() as usize].clone();
            let index = self.dictionary.len();
            self.dictionary.push(entry);
            ObjectId(u32::try_from(index).unwrap())
        }

        fn finish(
            self,
            static_operations: Vec<SimulationStaticOperation>,
        ) -> Arc<SimulationStaticArenaPlan> {
            let descriptor = ResourceDescriptor {
                size: 0,
                alignment: 1,
            };
            let entry_count = u32::try_from(self.dictionary.len()).unwrap();
            let virtual_size = u32::try_from(self.bytes.len()).unwrap();
            let arena = Arena {
                file_header: FileHeader {
                    magic: *b"\x89RW4xb2\0\r\n\x1a\n",
                    is_big_endian: true,
                    pointer_size_bits: 32,
                    pointer_alignment: 4,
                    unused: 0,
                    major_version: *b"454\0",
                    minor_version: *b"000\0",
                    build_number: 0,
                },
                id: 0x1234_5678,
                entry_count,
                used_entry_count: entry_count,
                alignment: 1,
                virtual_size,
                dictionary_offset: 0,
                sections_offset: 0,
                base_offset: 0,
                unfix_context_offset: 0,
                fix_context_offset: 0,
                resource_descriptor: [descriptor; 5],
                resources_used: [descriptor; 5],
                resource_offsets: [0; 5],
                arena_group_offset: 0,
                dictionary: self.dictionary,
            };
            let asset = SimulationCollisionAsset {
                decoded: DecodedArenaAsset::from_parts_for_test(
                    0xabcd_ef01_2345_6789,
                    WORLD_STREAM_ARENA_ASSET_TYPE,
                    ArenaResources {
                        resources: [self.bytes, Vec::new(), Vec::new(), Vec::new(), Vec::new()],
                    },
                    arena,
                ),
                collision: CollisionArena {
                    id: 0x1234_5678,
                    grind_assets: Vec::new(),
                    aggregate_roots: Vec::new(),
                    primitive_sets: Vec::new(),
                    simple_primitive_sets: Vec::new(),
                    meshes: Vec::new(),
                    collision_models: Vec::new(),
                    instances: Vec::new(),
                },
            };
            Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
                asset,
                static_operations,
            ))
        }
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn toc(entries: &[(u64, u32, ObjectId)]) -> Vec<u8> {
        let names = 20 + entries.len() * 24;
        let mut bytes = vec![0; names + 1];
        put_u32(&mut bytes, 0, u32::try_from(entries.len()).unwrap());
        put_u32(&mut bytes, 4, 20);
        put_u32(&mut bytes, 8, u32::try_from(names).unwrap());
        put_u32(&mut bytes, 16, u32::try_from(names).unwrap());
        for (index, (guid, object_type, object)) in entries.iter().enumerate() {
            let offset = 20 + index * 24;
            put_u32(&mut bytes, offset, u32::try_from(names).unwrap());
            put_u64(&mut bytes, offset + 8, *guid);
            put_u32(&mut bytes, offset + 16, *object_type);
            put_u32(&mut bytes, offset + 20, object.0);
        }
        bytes
    }

    fn instance_data(type_id: u32, records: &[(u64, u64)]) -> Vec<u8> {
        let mut bytes = vec![0; 20 + records.len() * 160];
        put_u32(&mut bytes, 0, type_id);
        put_u32(&mut bytes, 4, u32::try_from(records.len()).unwrap());
        put_u32(&mut bytes, 12, 20);
        let string_list_offset = u32::try_from(bytes.len()).unwrap();
        put_u32(&mut bytes, 16, string_list_offset);
        for (index, &(guid, local_guid)) in records.iter().enumerate() {
            let start = 20 + index * 160;
            put_u64(&mut bytes, start + 96, guid);
            put_u64(&mut bytes, start + 104, local_guid);
        }
        bytes
    }

    fn trigger_data(records: &[(u64, u64)]) -> Vec<u8> {
        let mut bytes = vec![0; 20 + records.len() * 240];
        // Deliberately not the raw TOC type: the native trigger processor has
        // no inner discriminator gate.
        put_u32(&mut bytes, 0, 0xdead_beef);
        put_u32(&mut bytes, 4, u32::try_from(records.len()).unwrap());
        put_u32(&mut bytes, 12, 20);
        let string_list_offset = u32::try_from(bytes.len()).unwrap();
        put_u32(&mut bytes, 16, string_list_offset);
        for (index, &(guid, local_guid)) in records.iter().enumerate() {
            let start = 20 + index * 240;
            put_u64(&mut bytes, start + 176, guid);
            put_u64(&mut bytes, start + 184, local_guid);
        }
        bytes
    }

    #[test]
    fn merges_static_and_registry_assets_in_authored_order() {
        let mut fixture = ArenaFixture::new();
        let painter = fixture.push(WORLD_PAINTER_DATA_TYPE_ID_0F, &[1]);
        let spline = fixture.push(SPLINE_DATA_TYPE_ID, &[2]);
        let rain = fixture.push(RAIN_DATA_TYPE_ID, &[3]);
        let instances = fixture.push(INSTANCE_DATA_TYPE_ID, &instance_data(0xacb3_1c9a, &[]));
        let nav = fixture.push(NAV_MESH_2_DATA_TYPE_ID, &[5]);
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (
                    0xaaaa_bbbb_cccc_dddd,
                    WORLD_PAINTER_DATA_TYPE_ID_0F,
                    painter,
                ),
                (1, SPLINE_DATA_TYPE_ID, spline),
                (0x1111_2222_3333_4444, RAIN_DATA_TYPE_ID, rain),
                (2, INSTANCE_DATA_TYPE_ID, instances),
                (0xffff_eeee_dddd_cccc, NAV_MESH_2_DATA_TYPE_ID, nav),
            ]),
        );
        let static_plan = fixture.finish(vec![
            SimulationStaticOperation::Grind {
                source_toc_index: 1,
                source_dictionary_index: 1,
                grind_asset_index: 0,
            },
            SimulationStaticOperation::Environment {
                source_toc_index: 3,
                source_dictionary_index: 3,
                source_inline_index: 0,
                source_resource_index: 0,
                source_record_start: 20,
                instance_index: 0,
            },
            SimulationStaticOperation::Environment {
                source_toc_index: 3,
                source_dictionary_index: 3,
                source_inline_index: 1,
                source_resource_index: 0,
                source_record_start: 180,
                instance_index: 1,
            },
        ]);

        let plan = SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap();
        assert_eq!(plan.presentation_arena_cost(), 2);
        assert_eq!(
            plan.operations()
                .iter()
                .copied()
                .map(SimulationNormalOperation::simulation_kind)
                .collect::<Vec<_>>(),
            vec![22, 6, 25, 10, 10, 23]
        );
        assert!(matches!(
            plan.operations()[0],
            SimulationNormalOperation::RegistryAsset {
                kind: SimulationRegistryAssetKind::WorldPainter,
                asset_id: 0xaaaa_bbbb_cccc_dddd,
                source_toc_index: 0,
                source_dictionary_index: 0,
                raw_object_type: WORLD_PAINTER_DATA_TYPE_ID_0F,
                raw_object_id: 0,
            }
        ));
        assert!(matches!(
            plan.operations()[5],
            SimulationNormalOperation::RegistryAsset {
                kind: SimulationRegistryAssetKind::NavMesh,
                asset_id: 0xffff_eeee_dddd_cccc,
                source_toc_index: 4,
                source_dictionary_index: 4,
                ..
            }
        ));
    }

    #[test]
    fn all_id_routes_preserve_repeats_and_full_ids() {
        let cases = [
            (
                WORLD_PAINTER_DATA_TYPE_ID_10,
                SimulationRegistryAssetKind::WorldPainter,
            ),
            (
                WORLD_PAINTER_DATA_TYPE_ID_11,
                SimulationRegistryAssetKind::WorldPainter,
            ),
            (AI_PATH_DATA_TYPE_ID, SimulationRegistryAssetKind::AiPath),
            (WAYPOINT_DATA_TYPE_ID, SimulationRegistryAssetKind::Waypoint),
            (
                SPATIAL_MAP_DATA_TYPE_ID,
                SimulationRegistryAssetKind::SpatialMap,
            ),
        ];
        let mut fixture = ArenaFixture::new();
        let mut entries = Vec::new();
        for (index, (raw_type, _)) in cases.iter().enumerate() {
            let object = fixture.push(*raw_type, &[u8::try_from(index).unwrap()]);
            entries.push((0xfeed_0000_0000_0000 | index as u64, *raw_type, object));
        }
        entries.push(entries[2]);
        fixture.push(TABLE_OF_CONTENTS_TYPE_ID, &toc(&entries));

        let plan = SimulationNormalArenaPlan::from_static_plan(fixture.finish(Vec::new())).unwrap();
        assert_eq!(plan.presentation_arena_cost(), 0);
        assert_eq!(plan.operations().len(), 6);
        for (index, (_, expected_kind)) in cases.iter().enumerate() {
            assert!(matches!(
                plan.operations()[index],
                SimulationNormalOperation::RegistryAsset {
                    kind,
                    asset_id,
                    ..
                } if kind == *expected_kind && asset_id == 0xfeed_0000_0000_0000 | index as u64
            ));
        }
        assert!(matches!(
            (plan.operations()[2], plan.operations()[5]),
            (
                SimulationNormalOperation::RegistryAsset {
                    kind: SimulationRegistryAssetKind::AiPath,
                    asset_id: first_id,
                    source_toc_index: 2,
                    source_dictionary_index: 2,
                    raw_object_type: AI_PATH_DATA_TYPE_ID,
                    raw_object_id: 2,
                },
                SimulationNormalOperation::RegistryAsset {
                    kind: SimulationRegistryAssetKind::AiPath,
                    asset_id: second_id,
                    source_toc_index: 5,
                    source_dictionary_index: 2,
                    raw_object_type: AI_PATH_DATA_TYPE_ID,
                    raw_object_id: 2,
                },
            ) if first_id == second_id
        ));
    }

    #[test]
    fn expands_pointer_backed_records_in_authored_order_with_exact_id_lanes() {
        let mut fixture = ArenaFixture::new();
        let dmo = fixture.push(
            INSTANCE_DATA_TYPE_ID,
            &instance_data(
                INSTANCE_DMO_ASSET_TYPE_ID,
                &[
                    (0xfedc_ba98_7654_3210, 0x1111_2222_3333_4444),
                    (0xaaaa_bbbb_cccc_dddd, 0x5555_6666_7777_8888),
                ],
            ),
        );
        let rain = fixture.push(RAIN_DATA_TYPE_ID, &[1]);
        let trigger = fixture.push(
            TRIGGER_INSTANCE_DATA_TYPE_ID,
            &trigger_data(&[(0x0123_4567_89ab_cdef, 0xf0e1_d2c3_b4a5_9687)]),
        );
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (7, INSTANCE_DATA_TYPE_ID, dmo),
                (8, RAIN_DATA_TYPE_ID, rain),
                (9, TRIGGER_INSTANCE_DATA_TYPE_ID, trigger),
                (10, INSTANCE_DATA_TYPE_ID, dmo),
            ]),
        );
        let static_plan = fixture.finish(Vec::new());
        let first = SimulationNormalArenaPlan::from_static_plan(Arc::clone(&static_plan)).unwrap();
        let second = SimulationNormalArenaPlan::from_static_plan(Arc::clone(&static_plan)).unwrap();
        drop(static_plan);

        assert_eq!(
            first
                .operations()
                .iter()
                .copied()
                .map(SimulationNormalOperation::simulation_kind)
                .collect::<Vec<_>>(),
            [11, 11, 25, 16, 11, 11]
        );
        assert!(matches!(
            first.operations()[0],
            SimulationNormalOperation::DmoInstance {
                asset_id: 0xfedc_ba98_7654_3210,
                local_guid: 0x1111_2222_3333_4444,
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_record_offset: 20,
                raw_object_id: 0,
            }
        ));
        assert!(matches!(
            first.operations()[3],
            SimulationNormalOperation::TriggerInstance {
                asset_id: 0xf0e1_d2c3_b4a5_9687,
                primary_guid: 0x0123_4567_89ab_cdef,
                source_toc_index: 2,
                source_dictionary_index: 2,
                source_inline_index: 0,
                source_record_offset: 20,
                raw_object_id: 2,
            }
        ));
        assert!(first.record_source(2).is_none());
        let first_dmo = first.record_source(0).unwrap();
        let aliased_dmo = first.record_source(4).unwrap();
        let distinct_dmo = first.record_source(1).unwrap();
        let other_plan_alias = second.record_source(4).unwrap();
        assert_eq!(first_dmo.kind(), SimulationRecordSourceKind::DmoInstance);
        assert_eq!(first_dmo.bytes().len(), 160);
        assert!(first_dmo.is_same_backing_record(&aliased_dmo));
        assert!(first_dmo.is_same_backing_record(&other_plan_alias));
        assert!(!first_dmo.is_same_backing_record(&distinct_dmo));
        assert_eq!(
            first.record_source(3).unwrap().kind(),
            SimulationRecordSourceKind::TriggerInstance
        );
        assert_eq!(first.record_source(3).unwrap().bytes().len(), 240);
    }

    #[test]
    fn pointer_identity_requires_the_same_retained_arena_owner() {
        fn one_plan() -> SimulationNormalArenaPlan {
            let mut fixture = ArenaFixture::new();
            let dmo = fixture.push(
                INSTANCE_DATA_TYPE_ID,
                &instance_data(
                    INSTANCE_DMO_ASSET_TYPE_ID,
                    &[(0x1111_2222_3333_4444, 0x5555_6666_7777_8888)],
                ),
            );
            fixture.push(
                TABLE_OF_CONTENTS_TYPE_ID,
                &toc(&[(1, INSTANCE_DATA_TYPE_ID, dmo)]),
            );
            SimulationNormalArenaPlan::from_static_plan(fixture.finish(Vec::new())).unwrap()
        }

        let first = one_plan();
        let equal_bytes_different_owner = one_plan();
        assert_eq!(
            first.record_source(0).unwrap().bytes(),
            equal_bytes_different_owner
                .record_source(0)
                .unwrap()
                .bytes()
        );
        assert!(
            !first
                .record_source(0)
                .unwrap()
                .is_same_backing_record(&equal_bytes_different_owner.record_source(0).unwrap())
        );

        let owned = first.record_handle(0).unwrap();
        let owned_clone = owned.clone();
        let different_owner = equal_bytes_different_owner.record_handle(0).unwrap();
        assert!(owned.is_same_backing_record(&owned_clone));
        assert!(!owned.is_same_backing_record(&different_owner));
        drop(first);
        assert_eq!(owned.bytes().len(), 160);
        assert_eq!(
            u64::from_be_bytes(owned.bytes()[96..104].try_into().unwrap()),
            0x1111_2222_3333_4444
        );
    }

    #[test]
    fn distinct_dictionary_aliases_preserve_one_backing_record_identity() {
        let mut fixture = ArenaFixture::new();
        let first_object = fixture.push(
            INSTANCE_DATA_TYPE_ID,
            &instance_data(
                INSTANCE_DMO_ASSET_TYPE_ID,
                &[(0x1111_2222_3333_4444, 0x5555_6666_7777_8888)],
            ),
        );
        let second_object = fixture.alias(first_object);
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (1, INSTANCE_DATA_TYPE_ID, first_object),
                (2, INSTANCE_DATA_TYPE_ID, second_object),
            ]),
        );
        let plan = SimulationNormalArenaPlan::from_static_plan(fixture.finish(Vec::new())).unwrap();
        assert_eq!(plan.operations().len(), 2);
        assert!(matches!(
            plan.operations()[1],
            SimulationNormalOperation::DmoInstance {
                source_dictionary_index: 1,
                raw_object_id: 1,
                ..
            }
        ));
        assert!(
            plan.record_source(0)
                .unwrap()
                .is_same_backing_record(&plan.record_source(1).unwrap())
        );
    }

    #[test]
    fn non_dmo_instances_are_filtered_but_trigger_header_is_not() {
        let mut fixture = ArenaFixture::new();
        let ordinary = fixture.push(
            INSTANCE_DATA_TYPE_ID,
            &instance_data(0x1234_5678, &[(1, 2)]),
        );
        let trigger = fixture.push(TRIGGER_INSTANCE_DATA_TYPE_ID, &trigger_data(&[(3, 4)]));
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (5, INSTANCE_DATA_TYPE_ID, ordinary),
                (6, TRIGGER_INSTANCE_DATA_TYPE_ID, trigger),
            ]),
        );
        let plan = SimulationNormalArenaPlan::from_static_plan(fixture.finish(Vec::new())).unwrap();
        assert_eq!(plan.operations().len(), 1);
        assert!(matches!(
            plan.operations()[0],
            SimulationNormalOperation::TriggerInstance {
                asset_id: 4,
                primary_guid: 3,
                ..
            }
        ));
    }

    #[test]
    fn location_and_dmo_pass_objects_remain_external() {
        let mut fixture = ArenaFixture::new();
        let location = fixture.push(LOCATION_DESC_DATA_TYPE_ID, &[1]);
        let dmo = fixture.push(DMO_DATA_TYPE_ID, &[2]);
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[
                (1, LOCATION_DESC_DATA_TYPE_ID, location),
                (2, DMO_DATA_TYPE_ID, dmo),
                (3, 0x00eb_0064, ObjectId((3 << 22) | 0x63)),
            ]),
        );
        let plan = SimulationNormalArenaPlan::from_static_plan(fixture.finish(Vec::new())).unwrap();
        assert!(plan.operations().is_empty());
    }

    #[test]
    fn no_toc_is_an_empty_supported_subsequence() {
        let plan =
            SimulationNormalArenaPlan::from_static_plan(ArenaFixture::new().finish(Vec::new()))
                .unwrap();
        assert!(plan.operations().is_empty());
    }

    #[test]
    fn rejects_nonlocal_missing_and_wrong_type_registry_sources() {
        let mut nonlocal = ArenaFixture::new();
        nonlocal.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, RAIN_DATA_TYPE_ID, ObjectId((2 << 22) | 7))]),
        );
        assert!(matches!(
            SimulationNormalArenaPlan::from_static_plan(nonlocal.finish(Vec::new())),
            Err(SimulationNormalArenaPlanError::NonLocalObjectReference { section: 2, .. })
        ));

        let mut missing = ArenaFixture::new();
        missing.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, RAIN_DATA_TYPE_ID, ObjectId(99))]),
        );
        assert!(matches!(
            SimulationNormalArenaPlan::from_static_plan(missing.finish(Vec::new())),
            Err(
                SimulationNormalArenaPlanError::MissingLocalObjectReference {
                    dictionary_index: 99,
                    ..
                }
            )
        ));

        let mut wrong = ArenaFixture::new();
        let object = wrong.push(AI_PATH_DATA_TYPE_ID, &[1]);
        wrong.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, RAIN_DATA_TYPE_ID, object)]),
        );
        assert!(matches!(
            SimulationNormalArenaPlan::from_static_plan(wrong.finish(Vec::new())),
            Err(SimulationNormalArenaPlanError::WrongLocalObjectType {
                expected: RAIN_DATA_TYPE_ID,
                actual: AI_PATH_DATA_TYPE_ID,
                ..
            })
        ));
    }

    #[test]
    fn pointer_processors_share_local_exact_type_validation() {
        let mut nonlocal = ArenaFixture::new();
        nonlocal.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, TRIGGER_INSTANCE_DATA_TYPE_ID, ObjectId((2 << 22) | 7))]),
        );
        assert!(matches!(
            SimulationNormalArenaPlan::from_static_plan(nonlocal.finish(Vec::new())),
            Err(SimulationNormalArenaPlanError::NonLocalObjectReference { section: 2, .. })
        ));

        let mut wrong = ArenaFixture::new();
        let instances = wrong.push(
            INSTANCE_DATA_TYPE_ID,
            &instance_data(INSTANCE_DMO_ASSET_TYPE_ID, &[]),
        );
        wrong.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, TRIGGER_INSTANCE_DATA_TYPE_ID, instances)]),
        );
        assert!(matches!(
            SimulationNormalArenaPlan::from_static_plan(wrong.finish(Vec::new())),
            Err(SimulationNormalArenaPlanError::WrongLocalObjectType {
                expected: TRIGGER_INSTANCE_DATA_TYPE_ID,
                actual: INSTANCE_DATA_TYPE_ID,
                ..
            })
        ));
    }

    #[test]
    fn uses_only_first_toc_and_rejects_unmergeable_static_operations() {
        let mut fixture = ArenaFixture::new();
        fixture.push(RAIN_DATA_TYPE_ID, &[1]);
        let ai = fixture.push(AI_PATH_DATA_TYPE_ID, &[2]);
        fixture.push(TABLE_OF_CONTENTS_TYPE_ID, &toc(&[]));
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(2, AI_PATH_DATA_TYPE_ID, ai)]),
        );
        let plan = SimulationNormalArenaPlan::from_static_plan(fixture.finish(Vec::new())).unwrap();
        assert!(plan.operations().is_empty());

        let mut one_toc = ArenaFixture::new();
        let rain = one_toc.push(RAIN_DATA_TYPE_ID, &[1]);
        one_toc.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, RAIN_DATA_TYPE_ID, rain)]),
        );
        let plan = SimulationNormalArenaPlan::from_static_plan(one_toc.finish(Vec::new())).unwrap();
        assert_eq!(plan.operations()[0].simulation_kind(), 25);

        let mut outside = ArenaFixture::new();
        outside.push(TABLE_OF_CONTENTS_TYPE_ID, &toc(&[]));
        assert!(matches!(
            SimulationNormalArenaPlan::from_static_plan(outside.finish(vec![
                SimulationStaticOperation::Grind {
                    source_toc_index: 1,
                    source_dictionary_index: 0,
                    grind_asset_index: 0,
                }
            ])),
            Err(SimulationNormalArenaPlanError::StaticOperationOutsideToc { .. })
        ));
    }

    #[test]
    fn rejects_backward_static_operation_order() {
        let fixture = ArenaFixture::new();
        assert!(matches!(
            SimulationNormalArenaPlan::from_static_plan(fixture.finish(vec![
                SimulationStaticOperation::Grind {
                    source_toc_index: 2,
                    source_dictionary_index: 0,
                    grind_asset_index: 0,
                },
                SimulationStaticOperation::Grind {
                    source_toc_index: 1,
                    source_dictionary_index: 0,
                    grind_asset_index: 1,
                },
            ])),
            Err(SimulationNormalArenaPlanError::StaticOperationOrder {
                static_operation_index: 1,
                previous_toc_index: 2,
                source_toc_index: 1,
            })
        ));
    }

    #[test]
    fn rejects_malformed_toc_and_out_of_resource_target() {
        let mut malformed = ArenaFixture::new();
        malformed.push(TABLE_OF_CONTENTS_TYPE_ID, &[0; 3]);
        assert!(matches!(
            SimulationNormalArenaPlan::from_static_plan(malformed.finish(Vec::new())),
            Err(SimulationNormalArenaPlanError::Pegasus(_))
        ));

        let mut outside = ArenaFixture::new();
        let rain = outside.push(RAIN_DATA_TYPE_ID, &[1]);
        outside.dictionary[rain.entry_index() as usize].object_range = 50..51;
        outside.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(1, RAIN_DATA_TYPE_ID, rain)]),
        );
        assert!(matches!(
            SimulationNormalArenaPlan::from_static_plan(outside.finish(Vec::new())),
            Err(SimulationNormalArenaPlanError::ObjectRangeOutsideResource {
                dictionary_index: 0,
                type_id: RAIN_DATA_TYPE_ID,
            })
        ));
    }

    #[test]
    fn retained_static_owner_survives_the_callers_arc() {
        let mut fixture = ArenaFixture::new();
        let spatial = fixture.push(SPATIAL_MAP_DATA_TYPE_ID, &[1]);
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc(&[(u64::MAX, SPATIAL_MAP_DATA_TYPE_ID, spatial)]),
        );
        let static_plan = fixture.finish(Vec::new());
        let weak = Arc::downgrade(&static_plan);
        let plan = SimulationNormalArenaPlan::from_static_plan(Arc::clone(&static_plan)).unwrap();
        drop(static_plan);
        assert!(weak.upgrade().is_some());
        assert_eq!(
            plan.static_plan().asset().decoded.asset_id,
            0xabcd_ef01_2345_6789
        );
        assert!(matches!(
            plan.operations()[0],
            SimulationNormalOperation::RegistryAsset {
                asset_id: u64::MAX,
                raw_object_id: 0,
                ..
            }
        ));
        let retained = plan
            .static_plan()
            .asset()
            .decoded
            .arena
            .object_bytes(
                &plan.static_plan().asset().decoded.resources.resources,
                &plan.static_plan().asset().decoded.arena.dictionary[0],
            )
            .unwrap();
        assert_eq!(retained, [1]);
    }
}
