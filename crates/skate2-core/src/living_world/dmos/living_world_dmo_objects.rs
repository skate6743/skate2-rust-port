//! `Sk8::LW::LWDynamicObjectMan` entities, priorities and spatial lookups, and
//! the DMO half of `Sk8::LivingWorld::CensusMan`.
//!
//! `EvaluateCensus` 0x82727a38 runs `EvaluateDynamicObjectCensus` 0x82726930
//! (`EvaluateCensusCircle` 0x826f46f0, `EvaluativeCullDynamicObjects`
//! 0x82724090, `EvaluativeSpawnDynamicObjects` 0x8271c690). Spawns go through
//! `SpawnDynamicObject` 0x82726c08 -> `RequestSpawn` 0x826f5890 (Spawn
//! 0x82d31ba0, `AttachToManagers` 0x82d36eb0, `OnSpawnComplete` 0x82d3cc58)
//! and `CheckSpawnedDynamicObjectAgainstSkaters` 0x82725620; culls through
//! `RequestCull` 0x8271a5e8 into the cull map drained by
//! `LWEntityCensusMan::Update` 0x82e67b10. `Manager::UpdatePostTransforms`
//! 0x827031e8 contributes `UpdateTargetTransforms`, the condition cull and
//! `LWDynamicObjectMan::UpdateTransforms` 0x82d3d6f8.
//!
//! Pedestrians, vehicles and props have no ported census; their evaluations
//! and overlap lists are empty, so LivingWorld entity IDs (the shared
//! `NextEntityID`) are consumed by DMOs alone. The cull map is keyed by
//! entity address natively; request order is kept instead.
#[path = "living_world_dmo_rain.rs"]
mod rain_points;
#[path = "living_world_dmo_execution.rs"]
mod execution;
use super::{
    dmo_locators::{
        DynamicObjectLocatorMan, LOCATOR_ENABLED, LOCATOR_IN_SESSION_MARKER,
        LOCATOR_PRIORITY_TRACKING, LOCATOR_STATIC, LocatorError, LocatorKdTree, LocatorServices,
        ObjectPersistenceInput,
    },
    dmo_models::{DmoGrabSpline, DmoModelMan, LwDynamicObjectModel, NULL_ASSET_ID, PRIORITY_CLASS},
};
use crate::{
    ai::vmx::vminfp,
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric, host_dot3,
        length_from_squared_and_estimate,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyHandle,
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
    world::transform_collision_aabb,
};
use skate2_formats::{
    collision::{Volume, VolumeType},
    pegasus::Dmo,
};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    sync::Arc,
};

#[derive(Clone,Copy,Debug)]
pub(crate) enum DmoCullEvent { BeforeCull(u32), Monitor{entity:u32,monitor:u32} }

/// `eBaseEntityType` DYNAMICOBJECTTYPE, `(type << 24) | id` (LWBasicEntity
/// ctor 0x82e61400).
/// `fixed_hash_map<uint, DynamicObject*, 50, 51, false>`: `GetPrevBucketCountOnly(51)`
/// is 47 and overflow is disabled, so the table never rehashes.
const ENTITY_BUCKETS: usize = 47;
const ENTITY_CAPACITY: usize = 50;
/// `LWEntityMan<DynamicObject>::Init` 0x82d36748: `MemStore` 0x50 x 50.
const ENTITY_KD_BLOCKS: usize = 50;
/// `StringToKey("dynamicobjects")`: the `livingworld`, census-range and
/// entity collection key.
pub(crate) const DYNAMIC_OBJECTS_KEY: u64 = 0x20f0_4d29_5272_ee46;
const LIVING_WORLD_CLASS: u64 = 0xf97b_9086_caa5_0c53;
pub(crate) const CENSUS_RANGES_CLASS: u64 = 0xc779_d2e2_1a4f_4f35;
pub(crate) const ENTITIES_CLASS: u64 = 0x60d7_03c7_7cab_1631;
/// `Attrib::Gen::debugmanager(StringToKey("livingworld"))`.
const DEBUG_MANAGER_CLASS: u64 = 0xff19_68a5_c467_bcf9;
const DEBUG_SPAWN_ENABLED_OFFSET: usize = 121;
/// `livingworld/dynamicobjects` weights read by `GeneratePriorityIndexForLocator`
/// 0x82d34b68 and `FillPriorityIndex` 0x82d2c160.
const CAMERA_PLANE_KEY: u64 = 0x74fe_9111_0546_63c9;
const IN_USE_KEY: u64 = 0x8837_71f1_1981_a45d;
const PRIORITY_WEIGHT_KEY: u64 = 0x16ad_b1db_a4aa_77d1;
const SAFETY_WEIGHT_KEY: u64 = 0xc891_b78c_b013_b593;
const DISTANCE_WEIGHT_KEY: u64 = 0xabd7_ea6b_3b93_846e;
/// `__real_4f000000` / `__real_cf000000` for a -1 priority or safety index.
const PRIORITY_UNSET: f32 = f32::from_bits(0x4f00_0000);
const SAFETY_UNSET: f32 = f32::from_bits(0xcf00_0000);
/// `__real_3a83126f` (EvaluativeSpawnDynamicObjects 0x8271cf6c).
const OVERLAP_PRIORITY_MARGIN: f32 = f32::from_bits(0x3a83_126f);
/// `BuildModelOverlapIDList(..., -0.05)` in the spawn loop 0x8271cefc.
const SPAWN_OVERLAP_PADDING: f32 = -0.05;
/// `BuildRecursiveObjectCullList` 0x82d37aa8 padding.
const RECURSIVE_CULL_PADDING: f32 = 0.05;
/// `__real_40666666` (3.5999999) in EvaluateDynamicObjectCensus 0x827269d4.
const SPEED_SCALE: f32 = f32::from_bits(0x4066_6666);
/// `Math::kBadMagnitude` (dynamic initializer 0x82f763a0).
const BAD_MAGNITUDE: f32 = f32::from_bits(0x3586_37bd);
const PASSIVE_QUERY_ITEMS: u32 = 0x100;
const CLOSE_QUERY_ITEMS: u32 = 0x32;
const CANDIDATE_CAPACITY: usize = 256;
/// BuildOverlapIDList 0x82d379ec hands `HorribleUglyVolumeVolumeTest` 50.
const OVERLAP_ASSEMBLIES: usize = 50;
/// `DynamicObject` +0x1110 flag byte.
const FLAG_CONDITION_CULL: u8 = 0x02;
/// Set by `OnEntityCollision` 0x82d3d004..d014 for a non-entity contact whose
/// surface class is 6 or 9; cleared by `ResetCollisionFlags` 0x82d2a790..a7ac.
const FLAG_SURFACE_CONTACT: u8 = 0x04;
const FLAG_STATIC: u8 = 0x10;
const FLAG_IN_USE_BY_PLAYER: u8 = 0x20;
const FLAG_CONSTRUCTED: u8 = 0x80;
/// `DynamicObjectStateOut` keeps each Part's contacts in a `fixed_vector` of 12.
const STATE_OUT_PART_CONTACTS: usize = 12;
/// `pegasus::eAudioSurfaceType_Undefined`, the surface of a contact with an entity.
const AUDIO_SURFACE_UNDEFINED: u32 = 0;
/// `rw::physics::BodyState` values `Spawn` 0x82d31d0c passes.
pub(crate) const BODY_STATE_STATIC: u32 = 1;
pub(crate) const BODY_STATE_ACTIVE: u32 = 4;
/// `InitCollisionBodies` 0x82d3aa28: mass < 5.5 (`blt`, NaN heavy) is group 16.
const LIGHT_MASS: f32 = 5.5;
const GROUP_LIGHT: u32 = 16;
const GROUP_HEAVY: u32 = 14;
/// `rw::collision::Volume::vTableArray[4]`, box.
const BOX_VOLUME: u32 = VolumeType::Box as u32;
/// `vTableArray[2]`, capsule.
const CAPSULE_VOLUME: u32 = VolumeType::Capsule as u32;
/// Characteristics layout +0x64 persistence priority.
const LAYOUT_PERSISTENCE_PRIORITY: usize = 0x64;

#[derive(Debug)]
pub(crate) enum DmoCensusError<W> {
    AssemblyOwner(super::collideable_objects::AssemblyOwnerError),
    Attribute(AttributeRuntimeError),
    Locator(LocatorError<DmoFrameServiceError>),
    Numeric(ReckoningHostNumericError),
    World(W),
    /// Native `FailAssert` sites (NaN input, missing collections).
    Assert(&'static str),
    Allocation,
    /// A native fixed container (capacity 50) would need its disabled
    /// overflow allocation.
    Capacity(&'static str),
}

impl<W: fmt::Debug> fmt::Display for DmoCensusError<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LivingWorld DMO census: {self:?}")
    }
}

impl<W: fmt::Debug> std::error::Error for DmoCensusError<W> {}

impl<W> From<AttributeRuntimeError> for DmoCensusError<W> {
    fn from(error: AttributeRuntimeError) -> Self {
        Self::Attribute(error)
    }
}

impl<W> From<ReckoningHostNumericError> for DmoCensusError<W> {
    fn from(error: ReckoningHostNumericError) -> Self {
        Self::Numeric(error)
    }
}

impl<W> From<LocatorError<DmoFrameServiceError>> for DmoCensusError<W> {
    fn from(error: LocatorError<DmoFrameServiceError>) -> Self {
        Self::Locator(error)
    }
}

/// Locator services during a frame: only the passive-tree numeric seed is
/// read; chunk streaming belongs to the terrain registration pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DmoFrameServiceError {
    Numeric(ReckoningHostNumericError),
    Streaming(&'static str),
}

impl fmt::Display for DmoFrameServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

pub(crate) struct FrameLocatorServices;

impl LocatorServices for FrameLocatorServices {
    type Error = DmoFrameServiceError;
    fn enable_locator_loading(&mut self) -> Result<bool, Self::Error> {
        Err(DmoFrameServiceError::Streaming("kEnableDMOLocatorLoading"))
    }
    fn remove_bad_locators(&mut self) -> Result<bool, Self::Error> {
        Err(DmoFrameServiceError::Streaming(
            "kEnableTheHackToRemoveBadLocators",
        ))
    }
    fn locator_chunk(&mut self, _: u64) -> Result<Option<Vec<Dmo>>, Self::Error> {
        Err(DmoFrameServiceError::Streaming("GetRegAsset(tDMOData)"))
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric
            .reciprocal_sqrt_estimate(value)
            .map_err(DmoFrameServiceError::Numeric)
    }
}

/// `HorribleUglyVolumeVolumeTest` 0x827cb218 inputs: the test volume at its
/// transform, the transformed model AABB and the padding.
pub(crate) struct DmoVolumeTest<'a> {
    pub(crate) volume: &'a Volume,
    pub(crate) transform: &'a [f32; 16],
    pub(crate) bounds: ([f32; 3], [f32; 3]),
    pub(crate) padding: f32,
    pub(crate) max_assemblies: usize,
}

/// Physics World effects of the DMO lifecycle.
pub(crate) trait DmoWorld {
    fn try_initialize_dmo_props(&mut self,object:&mut DynamicObject,entity_ids:&mut crate::world::LivingWorldEntityIdAllocator,census_enable:u8,models:&DmoModelMan)->Result<(),Self::Error>;
    type Error: fmt::Debug;
    /// LWBasicEntity.PostUpdateTransform82e4c2b8 dispatches the DMO override.
    fn try_update_dmo_contained_props(&mut self,children:&[u32],transform:[f32;16])->Result<(),Self::Error>;
    fn try_notify_dmo_monitor(&mut self,objects:&mut DmoObjectMan,monitor:u32,object:u32)->Result<(),Self::Error>;
    fn try_update_reset_capsules(&mut self,objects:&mut DmoObjectMan)->Result<(),Self::Error>;
    fn try_update_uprighting(&mut self,object:&mut DynamicObject)->Result<(),Self::Error>;
    fn try_before_dmo_cull(&mut self,objects:&mut DmoObjectMan,locators:&mut DynamicObjectLocatorMan,object:u32)->Result<(),Self::Error>;
    fn try_notify_dmo_movement(&mut self,monitor:u32,object:u32,delta:[f32;3])->Result<(),Self::Error>;
    /// `AddModelsToWorld` 0x82e5cd40 world half: `cLivingWorldCollisionBody::
    /// Create` 0x82e50b70 (group, Part user data), `AddToWorld` 0x82e50cb0
    /// (`Body::SetPosition` per Part, `Island::AddDMOBody` 0x827b6798,
    /// `RegisterSpyTarget`).
    fn try_add_body(
        &mut self,
        model: &Arc<LwDynamicObjectModel>,
        entity_id: u32,
        group: u32,
        part_transform: &[f32; 16],
        state: u32,
    ) -> Result<SimulationDmoAssemblyHandle, Self::Error>;
    /// `Dmo::SimManager::SpawnStatic` 0x825eb0a0's `Dmo::Body` 0x825ea858: every Part at the
    /// locator's matrix, group 0, added `STATIC_BODY`, with the Part spies.
    fn try_add_static_body(
        &mut self,
        model: &Arc<LwDynamicObjectModel>,
        entity_id: u32,
        transform: &[f32; 16],
    ) -> Result<SimulationDmoAssemblyHandle, Self::Error>;
    /// Entity destructor -> `RemoveFromWorld` 0x82e4c8e8 -> `Island::
    /// RemoveDMOBody` 0x827b7530 (`CleanUpDMOBody`, `DynamicPool::Remove`).
    fn try_remove_body(&mut self, body: &SimulationDmoAssemblyHandle) -> Result<(), Self::Error>;
    /// `DynamicObject::GetPartTransform(0)` 0x82d2a9a0 (`Part::GetTransform`).
    fn try_part_transform(
        &mut self,
        body: &SimulationDmoAssemblyHandle,
    ) -> Result<[f32; 16], Self::Error>;
    /// `LivingWorldView::VolumeTestAgainstDynamicObjects` 0x827cb4b8: DMO pool
    /// members whose Part 0 overlaps, in pool order.
    fn try_volume_test(
        &mut self,
        test: DmoVolumeTest<'_>,
    ) -> Result<Vec<SimulationDmoAssemblyHandle>, Self::Error>;
    /// `CheckSpawnedDynamicObjectAgainstSkaters` 0x82725620 for a new
    /// entity; it re-initializes the Manager capsule for every skater.
    fn try_check_spawned_against_skaters(
        &mut self,
        objects: &mut DmoObjectMan,
        entity_id: u32,
    ) -> Result<(), Self::Error>;
}

/// Census inputs sampled by the LivingWorld Manager.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DmoCensusInputs {
    /// `Manager::GetSimulationCenterPosition` 0x826f4dd0 (local Actor's
    /// SystemReckoning centre of mass), or the stream focus for the initial
    /// spawn, and `GetSimulationCenterVelocity` 0x826f4e48.
    pub(crate) center: [f32; 4],
    pub(crate) velocity: [f32; 4],
    /// `Manager::GetCameraPosition` 0x82703340 / `GetCameraHeading` 0x82703410.
    pub(crate) camera_position: [f32; 4],
    pub(crate) camera_heading: [f32; 4],
    pub(crate) initial: bool,
    /// `FEBackEnd::mFEState == eFE_STATE_IN_GAME`.
    pub(crate) in_game: bool,
    /// `CommonBackEnd::sGhostSystemEnabled` / `Simulation::IsNetworkGame`.
    pub(crate) ghost: bool,
    pub(crate) network_game: bool,
}

/// Census cvars in their native read order.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DmoCensusCvars {
    pub(crate) enable_cull: bool,
    pub(crate) enable_spawn: bool,
    pub(crate) enable_spawn_dynamic_objects: bool,
    pub(crate) one_component_per_frame: bool,
    pub(crate) cull_all_components_per_frame: bool,
    pub(crate) max_dynamic_objects: i32,
    pub(crate) close_by_locator_radius: f32,
}

/// Common EvaluateCensus82727a38 gates and category selection. Both the
/// DMO-only fixture route and live multi-category caller use this plan; only
/// the outer caller advances the one shared component index after spawning.
pub(crate) struct CensusPlan {
    pub(crate) cull_pedestrians: bool,
    pub(crate) spawn_pedestrians: bool,
    pub(crate) cull_dmos: bool,
    pub(crate) cull_vehicles: bool,
    pub(crate) cull_props: bool,
    pub(crate) spawn_dmos: bool,
    pub(crate) spawn_vehicles: bool,
    pub(crate) advance: bool,
}
pub(crate) fn census_plan<E>(
    census: &super::census::CensusState,
    inputs: &DmoCensusInputs,
    cvars: &DmoCensusCvars,
    vault: &Arc<AttributeRuntimeVault>,
) -> Result<Option<CensusPlan>, DmoCensusError<E>> {
    if nan_xyz(inputs.center) {
        return Err(DmoCensusError::Assert("CensusMan::EvaluateCensus position"));
    }
    if census.enable & 0x80 == 0 {
        return Ok(None);
    }
    let mut spawn =
        cvars.enable_spawn && census.enable & 0x40 != 0 && (inputs.in_game || inputs.initial);
    if spawn {
        let debug = vault
            .instance(DEBUG_MANAGER_CLASS, LIVING_WORLD_CLASS)
            .try_cached_layout_field(DEBUG_SPAWN_ENABLED_OFFSET, 1)?
            .is_some_and(|data| data.bytes()[0] != 0);
        spawn = debug && !inputs.ghost && !inputs.network_game;
    }
    let all = !cvars.one_component_per_frame || inputs.initial;
    let cull_all = all || cvars.cull_all_components_per_frame;
    Ok(Some(CensusPlan {
        cull_pedestrians: cvars.enable_cull && (cull_all || census.component_index == 0),
        spawn_pedestrians: spawn && (all || census.component_index == 0),
        cull_dmos: cvars.enable_cull && (cull_all || census.component_index == 2),
        cull_vehicles: cvars.enable_cull && (cull_all || census.component_index == 1),
        cull_props: cvars.enable_cull && (cull_all || census.component_index == 3),
        spawn_dmos: spawn && (all || census.component_index == 2),
        spawn_vehicles: spawn && (all || census.component_index == 1),
        advance: !all,
    }))
}

/// `tObjectPriorityIndex` (20 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ObjectPriorityIndex {
    pub(crate) object_id: u32,
    pub(crate) priority: f32,
    pub(crate) safety: f32,
    pub(crate) other_factors: f32,
    pub(crate) multiplier: f32,
}

/// `less<tObjectPriorityIndex>` inlined by `promote_heap` 0x82d300b0: the
/// parent reduces `(o + s) + p`, the moving value `(p + o) + s`.
fn promote_less(parent: &ObjectPriorityIndex, value: &ObjectPriorityIndex) -> bool {
    let parent_score =
        ((parent.other_factors + parent.safety) + parent.priority) * parent.multiplier;
    let value_score = ((value.priority + value.other_factors) + value.safety) * value.multiplier;
    parent_score > value_score
}

/// `less` inlined by `adjust_heap` 0x82d33350 and by
/// `IsLocatorHigherPriorityThanLowestPriorityObject` 0x82d2da38.
fn adjust_less(left: &ObjectPriorityIndex, right: &ObjectPriorityIndex) -> bool {
    let left_score = ((left.other_factors + left.safety) + left.priority) * left.multiplier;
    let right_score = ((right.other_factors + right.safety) + right.priority) * right.multiplier;
    left_score > right_score
}

/// `priority_queue<tObjectPriorityIndex, fixed_vector<.., 50>, less>`
/// (LWDynamicObjectMan +0x631c); the top is the lowest-priority object.
#[derive(Default)]
struct ObjectPriorityQueue {
    heap: Vec<ObjectPriorityIndex>,
}

impl ObjectPriorityQueue {
    /// `promote_heap` 0x82d300b0.
    fn promote(&mut self, top: usize, mut position: usize, value: ObjectPriorityIndex) {
        while position > top {
            let parent = (position - 1) >> 1;
            if !promote_less(&self.heap[parent], &value) {
                break;
            }
            self.heap[position] = self.heap[parent];
            position = parent;
        }
        self.heap[position] = value;
    }

    /// `adjust_heap` 0x82d33350.
    fn adjust(&mut self, top: usize, size: usize, mut position: usize, value: ObjectPriorityIndex) {
        let mut child = 2 * position + 2;
        while child < size {
            if adjust_less(&self.heap[child], &self.heap[child - 1]) {
                child -= 1;
            }
            self.heap[position] = self.heap[child];
            position = child;
            child = 2 * (child + 1);
        }
        if child == size {
            self.heap[position] = self.heap[child - 1];
            position = child - 1;
        }
        self.promote(top, position, value);
    }

    /// `remove_heap` 0x82d34248: moves `position` to the back.
    fn remove_heap(&mut self, size: usize, position: usize) {
        let bottom = self.heap[size - 1];
        self.heap[size - 1] = self.heap[position];
        self.adjust(0, size - 1, position, bottom);
    }

    /// `priority_queue::push` 0x82d36d58.
    fn push(&mut self, value: ObjectPriorityIndex) -> bool {
        if self.heap.len() == ENTITY_CAPACITY {
            return false;
        }
        self.heap.push(value);
        let last = self.heap.len() - 1;
        self.promote(0, last, value);
        true
    }

    /// `priority_queue::change` 0x82d36e20.
    fn change(&mut self, position: usize) {
        let size = self.heap.len();
        self.remove_heap(size, position);
        let value = self.heap[size - 1];
        self.promote(0, size - 1, value);
    }

    fn position(&self, object_id: u32) -> Option<usize> {
        self.heap
            .iter()
            .position(|entry| entry.object_id == object_id)
    }

    /// `RemoveObjectFromPriorityIndexTracking` 0x82d357b8.
    fn remove(&mut self, object_id: u32) {
        let Some(position) = self.position(object_id) else {
            return;
        };
        let size = self.heap.len();
        self.remove_heap(size, position);
        self.heap.pop();
    }

    /// `GetLowestPriorityObjectID` 0x82d2daa8.
    fn lowest(&self) -> u32 {
        self.heap.first().map_or(0, |entry| entry.object_id)
    }

    /// `IsLocatorHigherPriorityThanLowestPriorityObject` 0x82d2da38.
    fn is_higher_than_lowest(&self, candidate: &ObjectPriorityIndex) -> bool {
        self.heap
            .first()
            .is_none_or(|lowest| adjust_less(candidate, lowest))
    }
}

/// Iteration order of the entity `fixed_hash_map`: `id % 47` buckets walked
/// in order, each a front-inserted singly linked list.
pub(crate) struct EntityOrder {
    buckets: Vec<Vec<u32>>,
}

impl EntityOrder {
    fn new() -> Self {
        Self::with_bucket_count(ENTITY_BUCKETS)
    }

    pub(crate) fn with_bucket_count(count: usize) -> Self {
        Self {
            buckets: vec![Vec::new(); count],
        }
    }

    fn bucket(&self, id: u32) -> usize {
        id as usize % self.buckets.len()
    }

    pub(crate) fn insert(&mut self, id: u32) {
        let bucket = self.bucket(id);
        self.buckets[bucket].insert(0, id);
    }

    pub(crate) fn remove(&mut self, id: u32) {
        let index = self.bucket(id);
        let bucket = &mut self.buckets[index];
        if let Some(position) = bucket.iter().position(|&entry| entry == id) {
            bucket.remove(position);
        }
    }

    /// `GetEntityIDs` 0x82e96d10 and every map walk.
    pub(crate) fn ids(&self) -> Vec<u32> {
        self.buckets.iter().flatten().copied().collect()
    }
}

/// An entity that left the entity map and waits in the cull map.
struct PendingCull {
    entity_id: u32,
    body: SimulationDmoAssemblyHandle,
    // DeInit runs at destruction, not when DetachFromManagers removes the
    // entity. Retain its source references and IDs until that point.
    grab_splines: BTreeMap<u32, DmoGrabSpline>,
}

/// `Sk8::DynamicObject` state the ported lifecycle reads and writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DynamicObjectTransformType {
    Physics,
    HandProp,
}
/// `DynamicObjectStateOut::tDynamicObjectContactInfo` (0x40 bytes): the fields
/// `AudioPresentation::DoSimSubmit` 0x827fb2f8 reads. `mContactPosition`,
/// `mContactNormal`, `mContactForce` and `mContactEntityID` have no ported reader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DmoContactInfo {
    /// +0x30 `mContactAudioSurfaceType`.
    pub(crate) audio_surface: u32,
    /// +0x38 `mCollisionForce`: |relativeVel . normal|.
    pub(crate) collision_force: f32,
}

pub(crate) struct DynamicObject {
    pub(crate) contained_props: Vec<u32>,
    pub(crate) contained_props_initialized: bool,
    pub(crate) current_losim: u32,
    pub(crate) explicit_update_requested: bool,
    pub(crate) player_distance: super::entity_core::distance::EntityDistanceStack,
    pub(crate) attrib_cache: Option<Arc<super::attrib_cache::LwAttribCache>>,
    pub(crate) entity_id: u32,
    /// LivingWorldEntity.mpRAINData; invalidated when streamed RAIN is removed.
    pub(crate) rain_data: Option<crate::living_world_rain::spatial::RainItem>,
    /// LWCommonCoreData.mModelKey, from livingworld_entities layout+8.
    /// Presentation binds livingworld_models to this key, not characteristics.
    pub(crate) presentation_model_key: u64,
    /// +0x10f0 locator ID (`kNullAssetID` until `SetLocatorID`).
    pub(crate) locator_id: u64,
    pub(crate) model: Arc<LwDynamicObjectModel>,
    /// Core data `mTransform` (+0x10), `mTargetTransform` (+0x50) and the
    /// velocity (+0x90) written by `LWBasicEntity::UpdateTransform` 0x82e5b358.
    pub(crate) transform: [f32; 16],
    pub(crate) target_transform: [f32; 16],
    pub(crate) velocity: [f32; 4],
    /// Complete-object+4272; native MemStore clear until UpdateTransform's
    /// UpdateFarthestCorners82d2c7d0 writes all four values.
    pub(crate) farthest_corners: [[f32;4];4],
    /// Core data +240 bit 7 (spawn info `bAllowCull`).
    pub(crate) allow_cull: bool,
    /// +0x1110 flag byte.
    pub(crate) flags: u8,
    /// DynamicObject+0x1108, distinct from ResetCollisionFlags' surface bits.
    pub(crate) skater_collision_flags: u32,
    /// `mStateOut` (+0xd50) Part 0's contacts. `LWDynamicObjectMan::ResetCollisionFlags`
    /// empties only Part 0's list; the other Parts' lists have no reader and are not kept.
    pub(crate) part0_contacts: Vec<DmoContactInfo>,
    /// +0x1104 `GetSafetyIndex` 0x82d362e8 (constructor stores 1).
    pub(crate) safety_index: u32,
    /// Complete-object+1100, UpdatePositionalWorldPainterData82d393e8.
    pub(crate) wp_safety_index: u32,
    /// Complete-object+110c, not IDynamicObject-interface-relative+1080.
    pub(crate) transform_type: DynamicObjectTransformType,
    /// Complete-object+10f8; reset by hand-prop release/unset.
    pub(crate) hand_prop_cull_timer: f32,
    /// cLWDynamicObjectCollision ctor82d2b9d8 passes false to its base.
    pub(crate) collision_ignore_gravity: bool,
    /// Native ordered mMonitoringEntities vector; remove shifts the suffix.
    pub(crate) monitoring_entities: Vec<u32>,
    /// The world collision body in the Island DMO pool.
    pub(crate) body: SimulationDmoAssemblyHandle,
    pub(crate) group: u32,
    /// LWGrabbableObject::mObjectGrabSplineMap: ordered IDs, capacity 64.
    pub(crate) grab_splines: BTreeMap<u32, DmoGrabSpline>,
    pub(crate) grind_splines: BTreeMap<u32, super::dmo_models::DmoGrindSpline>,
    /// A static body `Dmo::SimManager::SpawnStatic` (0x825eb0a0) placed in an online game, which
    /// `Dmo::PresManager`'s `DmoPresentationEntity` draws without the living world's fade.
    pub(crate) online_static: bool,
}

impl DynamicObject {
    /// DynamicObject::IsUpright 0x82d2ac50, AngleBetweenVectors 0x829b0528.
    pub(crate) fn try_is_upright(&self, threshold: f32) -> Result<bool, crate::reckoning_numeric::ReckoningHostNumericError> {
        use crate::reckoning_numeric::{HostReckoningNumeric, host_dot3, ReckoningNumeric, try_acos};
        let up = [self.transform[4], self.transform[5], self.transform[6], self.transform[7]];
        let angle = if host_dot3(up, up) > 0.0001 {
            // AngleBetweenVectors829b0528 uses ONE Newton step for both vectors.
            let normalize=|v:[f32;4]|->Result<[f32;4],ReckoningHostNumericError> {
                let sq=host_dot3(v,v);
                let seed=HostReckoningNumeric.reciprocal_sqrt_estimate(sq)?;
                let inverse=(seed*0.5).mul_add((-sq).mul_add(seed*seed,1.),seed);
                Ok(v.map(|x|x*inverse))
            };
            let cosine = host_dot3(normalize(up)?,normalize([0.,1.,0.,0.])?);
            let cosine = if cosine < -1. { -1. } else { cosine };
            let cosine = if cosine > 1. { 1. } else { cosine };
            try_acos(cosine, &mut HostReckoningNumeric)?
        } else { 0. };
        Ok(angle * 57.29578 < threshold)
    }

    /// Original four corners use the two larger dimensions reduced by the
    /// smallest half extent, then the SAME model bbox transform as collision.
    fn update_farthest_corners(&mut self){
        let d=self.model.dimensions;
        let (a,b)=if d[1]>d[0]&&d[2]>d[0]{
            ([0.,d[1]*0.5-d[0]*0.5,0.,0.],[0.,0.,d[2]*0.5-d[0]*0.5,0.])
        }else if !(d[1]>d[0])&&d[2]>d[1]{
            ([d[0]*0.5-d[1]*0.5,0.,0.,0.],[0.,0.,d[2]*0.5-d[1]*0.5,0.])
        }else{
            ([d[0]*0.5-d[2]*0.5,0.,0.,0.],[0.,d[1]*0.5-d[2]*0.5,0.,0.])
        };
        let local:[[f32;4];4]=[
            std::array::from_fn(|i|a[i]+b[i]),std::array::from_fn(|i|a[i]-b[i]),
            std::array::from_fn(|i|-a[i]-b[i]),std::array::from_fn(|i|-a[i]+b[i])];
        let transform=self.model.bbox_transform(&self.transform);
        self.farthest_corners=local.map(|point|std::array::from_fn(|i|{
            let x=transform[i].mul_add(point[0],transform[12+i]);
            let y=transform[4+i].mul_add(point[1],x);
            transform[8+i].mul_add(point[2],y)
        }));
    }
    /// IDynamicObject::IsSetAsHandProp82d2ad08, interface at complete+8c.
    pub(crate) fn is_set_as_hand_prop(&self) -> bool {
        self.transform_type == DynamicObjectTransformType::HandProp
    }
    /// `IsStatic` 0x82d36320.
    pub(crate) const fn is_static(&self) -> bool {
        self.flags & FLAG_STATIC != 0
    }

    /// `IsInUseByPlayer` 0x82d362f8.
    pub(crate) const fn is_in_use_by_player(&self) -> bool {
        self.flags & FLAG_IN_USE_BY_PLAYER != 0
    }

    /// `SetInUseByPlayer` 0x82d36308.
    pub(crate) fn set_in_use_by_player(&mut self, in_use: bool) {
        self.flags =
            (self.flags & !FLAG_IN_USE_BY_PLAYER) | if in_use { FLAG_IN_USE_BY_PLAYER } else { 0 };
    }

    /// `OnEntityCollision` 0x82d3cfdc..d01c for a contact whose other body has no owner:
    /// the surface-contact flag on surface class 6 or 9, then `AddContact`.
    pub(crate) fn add_surface_contact(&mut self, part: u32, surface_id: u32, collision_force: f32) {
        let (audio_surface, surface_flag) = surface_contact(surface_id);
        if surface_flag {
            self.flags |= FLAG_SURFACE_CONTACT;
        }
        add_part0_contact(&mut self.part0_contacts, part, DmoContactInfo { audio_surface, collision_force });
    }

    /// `OnEntityCollision` 0x82d3d038..d060 for a pedestrian, vehicle or dynamic object: the
    /// surface stays undefined.
    pub(crate) fn add_entity_contact(&mut self, part: u32, collision_force: f32) {
        let info = DmoContactInfo { audio_surface: AUDIO_SURFACE_UNDEFINED, collision_force };
        add_part0_contact(&mut self.part0_contacts, part, info);
    }
}

/// `OnEntityCollision` 0x82d3cfe8..d000: the audio surface `surfaceID & 0x7f`, and whether the
/// surface class (`surfaceID >> 7 & 0x1f`) is 6 or 9.
fn surface_contact(surface_id: u32) -> (u32, bool) {
    let class = (surface_id >> 7) & 0x1f;
    (surface_id & 0x7f, class == 6 || class == 9)
}

/// `DynamicObjectStateOut::AddContact` 0x82d35f38 for Part 0's list: a Part keeps its first 12.
fn add_part0_contact(contacts: &mut Vec<DmoContactInfo>, part: u32, info: DmoContactInfo) {
    if part == 0 && contacts.len() < STATE_OUT_PART_CONTACTS {
        contacts.push(info);
    }
}

/// `LWDynamicObjectMan` entity state owned by the LivingWorld Manager.
pub(crate) struct DmoObjectMan {
    pub(crate) execution: super::execution::ExecutionMan<50, DynamicObject>,
    pub(crate) game_sim_events: Option<Arc<crate::hermes_game_sim_queue::GameSimEventRoute>>,
    collideables: super::collideable_objects::CollideableObjectMan,
    /// The entity map's members.
    objects: HashMap<u32, DynamicObject>,
    /// Constructed spawn prefix retained until children and manager attachment succeed.
    spawn_unfinished: Option<DynamicObject>,
    order: EntityOrder,
    grabbables: super::spline_registration::GrabbableObjectMan,
    priorities: ObjectPriorityQueue,
    /// Cull map in request order; its bodies stay in the world until
    /// `LWEntityCensusMan::Update` destroys the entities.
    cull_map: Vec<PendingCull>,
    /// `mEntityKDTree`, rebuilt by `UpdateSpatialLookups` 0x82d367b0.
    spatial: LocatorKdTree,
    /// `Manager::mIntersectionCapsuleVolume` (+0x1b5bc).
    intersection_capsule: IntersectionCapsule,
    pub(crate) reset_capsules:Option<[SkaterTestCapsule;2]>,
}

impl DmoObjectMan {
    /// PC Skate 3 physics: the objects with grind splines, by entity ID (their rails move with them).
    pub(crate) fn grindable_objects(&self) -> Vec<&DynamicObject> {
        let mut objects: Vec<_> = self.objects.values().filter(|object| !object.grind_splines.is_empty()).collect();
        objects.sort_unstable_by_key(|object| object.entity_id);
        objects
    }
    /// LWComponentMan.InvalidateAllRAINData82d2f8a8: clear every live entity's
    /// cached RAIN pointer; this is independent of census/cull-map processing.
    pub(crate) fn invalidate_rain_data(&mut self) {
        for object in self.objects.values_mut() {
            object.rain_data = None;
        }
    }
    /// Shared RequestExplicitCull path for census and streamed locator removal.
    /// No world destruction here: the existing census cull map owns that step.
    pub(crate) fn try_request_explicit_cull<S: LocatorServices>(
        &mut self,
        entity_id: u32,
        locators: &mut DynamicObjectLocatorMan,
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        self.try_request_explicit_cull_with_monitors(entity_id,locators,services,|_,monitor,_|
            Err(format!("monitor {monitor:#x} requires concrete pedestrian cull callback")))
    }
    pub(crate) fn try_request_explicit_cull_with_monitors<S:LocatorServices>(
        &mut self,entity_id:u32,locators:&mut DynamicObjectLocatorMan,services:&mut S,
        mut notify:impl FnMut(&mut Self,u32,u32)->Result<(),String>,
    )->Result<(),LocatorError<S::Error>>{
        self.try_request_explicit_cull_with_lifecycle(entity_id,locators,services,|objects,_,event|match event{
            DmoCullEvent::BeforeCull(id)=>{
                if objects.object(id).is_some_and(|object|!object.contained_props.is_empty()){
                    return Err("DMO child props require synchronous before-cull services".into());
                }
                Ok(())
            }
            DmoCullEvent::Monitor{entity,monitor}=>notify(objects,monitor,entity),
        })
    }
    pub(crate) fn try_request_explicit_cull_with_lifecycle<S:LocatorServices>(
        &mut self,entity_id:u32,locators:&mut DynamicObjectLocatorMan,services:&mut S,
        mut notify:impl FnMut(&mut Self,&mut DynamicObjectLocatorMan,DmoCullEvent)->Result<(),String>,
    )->Result<(),LocatorError<S::Error>>{
        if self
            .cull_map
            .iter()
            .any(|pending| pending.entity_id == entity_id)
        {
            return Ok(());
        }
        if !self.objects.contains_key(&entity_id) {return Ok(());}
        //OnCullRequested82d3cda0 invokes base child/sink cleanup before
        //NotifyMonitoringEntities takes its snapshot, not at destruction.
        notify(self,locators,DmoCullEvent::BeforeCull(entity_id)).map_err(LocatorError::CullCallback)?;
        let monitors=self.objects.get(&entity_id).ok_or_else(||LocatorError::CullCallback("object disappeared during child cleanup".into()))?.monitoring_entities.clone();
        //82d3bd80 copies the four-entry monitor list before callbacks mutate it.
        for monitor in monitors {notify(self,locators,DmoCullEvent::Monitor{entity:entity_id,monitor}).map_err(LocatorError::CullCallback)?;}
        let object=self.objects.get(&entity_id).ok_or_else(||LocatorError::CullCallback("object disappeared during monitor notification".into()))?;
        let locator_id = object.locator_id;
        self.priorities.remove(entity_id);
        if locator_id != NULL_ASSET_ID {
            self.objects
                .get_mut(&entity_id)
                .expect("entity checked")
                .locator_id = NULL_ASSET_ID;
            locators.detach_object(locator_id, services)?;
        }
        self.game_sim_events.as_ref().ok_or_else(||LocatorError::CullCallback("GameSim route absent".into()))?
            .send(crate::hermes_game_sim_queue::LivingWorldMessage::RequestCull(entity_id)).map_err(LocatorError::CullCallback)?;
        self.order.remove(entity_id);
        let level=self.objects.get(&entity_id).expect("entity checked").current_losim;
        self.execution.remove(level,entity_id)
            .map_err(|e|LocatorError::CullCallback(format!("DMO execution detach: {e:?}")))?;
        let object = self.objects.remove(&entity_id).expect("entity checked");
        self.cull_map.push(PendingCull {
            entity_id,
            body: object.body,
            grab_splines: object.grab_splines,
        });
        Ok(())
    }
    /// `LWEntityMan<DynamicObject>` ctor 0x82d366d0 + Init 0x82d36748, with
    /// DMO-only storage. Shared CensusMan fields belong to LivingWorld.
    pub(crate) fn try_construct() -> Option<Box<Self>> {
        let mut objects = HashMap::new();
        objects.try_reserve(ENTITY_CAPACITY).ok()?;
        Some(Box::new(Self {
            execution: super::execution::ExecutionMan::new(),
            game_sim_events:None,
            collideables: super::collideable_objects::CollideableObjectMan::new(),
            objects,
            spawn_unfinished:None,
            order: EntityOrder::new(),
            grabbables: super::spline_registration::GrabbableObjectMan::new(),
            priorities: ObjectPriorityQueue::default(),
            cull_map: Vec::new(),
            spatial: LocatorKdTree::try_with_blocks(ENTITY_KD_BLOCKS)?,
            // Manager ctor 0x8272205c..2074: CreateCollisionCapsule(1.0, 1.0).
            reset_capsules:None,
            intersection_capsule: IntersectionCapsule {
                radius: 1.0,
                half_height: 1.0,
            },
        }))
    }

    pub(crate) const fn intersection_capsule(&self) -> IntersectionCapsule {
        self.intersection_capsule
    }

    /// `CheckSpawnedDynamicObjectAgainstSkaters` 0x82725880..58b4 hands the
    /// Manager capsule to `BuildSkaterTestCapsule`, re-initializing it.
    pub(crate) fn set_intersection_capsule(&mut self, capsule: IntersectionCapsule) {
        self.intersection_capsule = capsule;
    }

    pub(crate) fn is_pending_cull(&self,entity:u32)->bool {
        self.cull_map.iter().any(|pending|pending.entity_id==entity)
    }

    pub(crate) fn object(&self, entity_id: u32) -> Option<&DynamicObject> {
        self.objects.get(&entity_id)
    }

    pub(crate) fn object_mut(&mut self, entity_id: u32) -> Option<&mut DynamicObject> {
        self.objects.get_mut(&entity_id)
    }

    /// GetGrabSplineOwnerObjectID 0x82e57dd0; absent IDs return zero.
    pub(crate) fn grab_spline_owner(&self, spline_id: u32) -> u32 {
        self.grabbables.owner(spline_id)
    }

    fn register_grab_splines(
        &mut self,
        entity_id: u32,
        sources: &[DmoGrabSpline],
        ids: &mut super::spline_registration::LivingWorldSplineIds,
    ) -> BTreeMap<u32, DmoGrabSpline> {
        self.grabbables.register(entity_id, sources, ids)
    }

    /// `mEntityMap.mnElementCount`.
    pub(crate) fn count(&self) -> usize {
        self.objects.len()
    }

    /// Entity IDs in native map order.
    pub(crate) fn ids(&self) -> Vec<u32> {
        self.order.ids()
    }

    /// LWDynamicObjectMan::AddEntityVolumesToBatch82d312f8. The caller uses
    /// the same stable owner token as its other masked volume sources.
    pub(crate) fn add_avoidance_volumes(
        &self, batch: &mut crate::collision_volume_line::VolumeLineBatch,
        vault: &Arc<AttributeRuntimeVault>, mut owner_mask: impl FnMut(u32) -> u32,
    ) -> Result<(), AttributeRuntimeError> {
        let attributes = vault.instance(
            skate2_formats::attrib::string_hash64("physics_skeleton"),
            skate2_formats::attrib::string_hash64("default"),
        );
        // Native samples threshold once before entity traversal.
        let minimum_mass = attributes.try_scalar_word(0x9ab5_7b38_d5bb_9449, 0)?
            .map_or(0., f32::from_bits);
        for id in self.ids() {
            let object = self.object(id).expect("retained native entity order");
            if object.is_set_as_hand_prop() || !(object.model.mass >= minimum_mass) { continue; }
            if !batch.add_volume(crate::collision_volume_line::VolumeData {
                transform: object.model.bbox_transform(&object.transform),
                dimensions: object.model.dimensions.map(|v| v * 0.5), mask: owner_mask(id),
            }) { break; }
        }
        Ok(())
    }

    /// `LWCollideableObjectMan::GetAssemblyOwnerID`: the entity whose world
    /// collision body is `body`, or 0.
    pub(crate) fn owner_of(&self, body: &SimulationDmoAssemblyHandle) -> u32 {
        self.collideables.owner_of(body)
    }

    /// `GetDynamicObjectsWithinRange` 0x82d3be78 (max clamped to 50) ->
    /// `KDTree<uint>::FindNear` 0x82d28c00.
    pub(crate) fn try_objects_within_range(
        &self,
        point: [f32; 2],
        radius: f32,
        max_items: u32,
    ) -> Result<Vec<(u32, f32)>, LocatorError<DmoFrameServiceError>> {
        let limit = max_items.min(ENTITY_KD_BLOCKS as u32);
        let found = self
            .spatial
            .find_near(point, radius, limit, &mut FrameLocatorServices)?;
        Ok(found
            .into_iter()
            .map(|(id, distance)| {
                (
                    u32::try_from(id).expect("entity KD objects are 32-bit IDs"),
                    distance,
                )
            })
            .collect())
    }

    /// `LWDynamicObjectMan::ResetCollisionFlags` 0x82d303b0, in map order:
    /// `DynamicObject::ResetCollisionFlags` 0x82d2a778 clears the surface
    /// contact bit and `SetInUseByPlayer(false)`, then Part 0's StateOut
    /// contact list is emptied (0x82d30460..68). The base
    /// `LWCollideableObject` half resets its collision body's report flags,
    /// which no ported consumer reads.
    pub(crate) fn reset_collision_flags(&mut self) {
        for id in self.order.ids() {
            let object = self
                .objects
                .get_mut(&id)
                .expect("entity order names live entities");
            object.flags &= !FLAG_SURFACE_CONTACT;
            object.set_in_use_by_player(false);
            object.part0_contacts.clear();
        }
    }

    /// `ClearSpatialLookups` 0x82d36770 then `UpdateSpatialLookups` 0x82d367b0.
    pub(crate) fn rebuild_spatial_lookups(&mut self) {
        self.clear_spatial_lookups();
        self.update_spatial_lookups();
    }

    /// `ClearSpatialLookups` 0x82d36770: `MemStore::Reset` and a null root.
    pub(crate) fn clear_spatial_lookups(&mut self) {
        self.spatial.reset();
    }

    /// `UpdateSpatialLookups` 0x82d367b0: every entity (core data +0xa0, the
    /// transform's XZ) inserted in map order without a reset, so entities
    /// `AddEntity` already inserted since the last clear appear twice.
    pub(crate) fn update_spatial_lookups(&mut self) {
        for id in self.order.ids() {
            let transform = self.objects[&id].transform;
            // `KDTree::Insert`'s full-store false is not read.
            self.spatial
                .insert(u64::from(id), [transform[12], transform[14]]);
        }
    }

    /// `LWEntityMan::AddEntity` 0x82d368d0: map insert and a KD insert.
    fn attach_spawn_unfinished(&mut self) -> Result<(), &'static str> {
        if self.objects.len() == ENTITY_CAPACITY {
            return Err("DMO entity map");
        }
        let object=self.spawn_unfinished.as_ref().ok_or("DMO pending spawn absent")?;
        let id = object.entity_id;
        let coord = [object.transform[12], object.transform[14]];
        self.execution.add(object.current_losim,id).map_err(|_|"DMO execution attach")?;
        let object=self.spawn_unfinished.take().expect("pending spawn checked");
        self.objects.insert(id, object);
        self.order.insert(id);
        self.spatial.insert(u64::from(id), coord);
        Ok(())
    }
}

/// Everything the census and post-transform passes borrow.
pub(crate) struct DmoFrame<'a, W: DmoWorld> {
    pub(crate) census: &'a mut super::census::CensusState,
    pub(crate) entity_ids: &'a mut crate::world::LivingWorldEntityIdAllocator,
    pub(crate) spline_ids: &'a mut super::spline_registration::LivingWorldSplineIds,
    pub(crate) vault: &'a Arc<AttributeRuntimeVault>,
    pub(crate) models: &'a DmoModelMan,
    pub(crate) locators: &'a mut DynamicObjectLocatorMan,
    pub(crate) objects: &'a mut DmoObjectMan,
    pub(crate) world: &'a mut W,
}

type FrameResult<T, W> = Result<T, DmoCensusError<<W as DmoWorld>::Error>>;

/// `tLWCensusCircle`: NoSpawn, Spawn, Cull, Shift, Speed.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CensusCircle {
    pub(crate) no_spawn: f32,
    pub(crate) spawn: f32,
    pub(crate) cull: f32,
    pub(crate) shift: f32,
    pub(crate) speed: f32,
}

fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_bits(u32::from_be_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked layout"),
    ))
}

/// `EvaluateCensusCircle` 0x826f46f0: a `min.Speed >= max.Speed` (or
/// unordered) pair copies the minimum; otherwise the fsel-clamped blend.
pub(crate) fn evaluate_census_circle(
    speed: f32,
    minimum: CensusCircle,
    maximum: CensusCircle,
) -> CensusCircle {
    if !(minimum.speed < maximum.speed) {
        return minimum;
    }
    let fsel = |test: f32, greater_equal: f32, less: f32| {
        if test >= 0.0 { greater_equal } else { less }
    };
    let offset = speed - minimum.speed;
    let range = maximum.speed - minimum.speed;
    let positive = fsel(-offset, 0.0, offset);
    let bounded = fsel(offset.abs() - positive, positive, offset.abs());
    let ratio = bounded / range;
    let low = fsel(-ratio, 0.0, ratio);
    let t = fsel(1.0 - low, low, 1.0);
    let lerp = |low: f32, high: f32| (high - low).mul_add(t, low);
    CensusCircle {
        no_spawn: lerp(minimum.no_spawn, maximum.no_spawn),
        spawn: lerp(minimum.spawn, maximum.spawn),
        cull: lerp(minimum.cull, maximum.cull),
        shift: lerp(minimum.shift, maximum.shift),
        speed: lerp(minimum.speed, maximum.speed),
    }
}

fn nan_xyz(vector: [f32; 4]) -> bool {
    vector[0].is_nan() || vector[1].is_nan() || vector[2].is_nan()
}

fn matrix_has_nan(matrix: &[f32; 16]) -> bool {
    (0..4).any(|row| {
        nan_xyz([
            matrix[row * 4],
            matrix[row * 4 + 1],
            matrix[row * 4 + 2],
            matrix[row * 4 + 3],
        ])
    })
}

fn row3(matrix: &[f32; 16]) -> [f32; 4] {
    [matrix[12], matrix[13], matrix[14], matrix[15]]
}

/// `vmsum3fp`, `vrsqrtefp`, two refinements and the zero select.
pub(crate) fn host_length(vector: [f32; 4]) -> Result<f32, ReckoningHostNumericError> {
    let squared = host_dot3(vector, vector);
    let estimate = HostReckoningNumeric.reciprocal_sqrt_estimate(squared)?;
    Ok(length_from_squared_and_estimate(squared, estimate))
}

fn length<W>(vector: [f32; 4]) -> Result<f32, DmoCensusError<W>> {
    Ok(host_length(vector)?)
}

/// `Math::NormalizeSafe` 0x824b18e0: `dot - kBadMagnitude^2 > 0` scales by
/// the refined reciprocal square root, otherwise the default vector.
fn host_normalize_safe(
    vector: [f32; 4],
    default: [f32; 4],
) -> Result<[f32; 4], ReckoningHostNumericError> {
    let squared = host_dot3(vector, vector);
    if !(squared - BAD_MAGNITUDE * BAD_MAGNITUDE > 0.0) {
        return Ok(default);
    }
    let estimate = HostReckoningNumeric.reciprocal_sqrt_estimate(squared)?;
    let inverse = refine_drive_reciprocal_sqrt(squared, estimate);
    Ok(vector.map(|lane| lane * inverse))
}

fn normalize_safe<W>(vector: [f32; 4], default: [f32; 4]) -> Result<[f32; 4], DmoCensusError<W>> {
    Ok(host_normalize_safe(vector, default)?)
}

fn livingworld_scalar(
    instance: &AttributeRuntimeInstance,
    key: u64,
) -> Result<f32, AttributeRuntimeError> {
    Ok(instance
        .try_get_data(key, 0)?
        .map_or(0.0, |data| read_f32(data.bytes(), 0)))
}

/// Inputs to the shared tail of the two priority builders.
struct PriorityTerms {
    position: [f32; 4],
    priority_index: i32,
    safety_index: i32,
    in_use: bool,
    /// `NormalizeSafe` default: `gIVector` for locators, the camera heading
    /// for objects.
    default_direction: [f32; 4],
}

/// `livingworld_dynamicobject_priority` layout int at +0 (zero default area
/// without a collection).
fn priority_value(
    vault: &Arc<AttributeRuntimeVault>,
    model: &LwDynamicObjectModel,
) -> Result<i32, AttributeRuntimeError> {
    let collection = model.try_priority_collection()?;
    let instance = vault.instance(PRIORITY_CLASS, collection);
    Ok(instance.try_cached_layout_field(0, 4)?.map_or(0, |data| {
        i32::from_be_bytes(data.bytes().try_into().expect("four bytes"))
    }))
}

/// Shared arithmetic of `GeneratePriorityIndexForLocator` 0x82d34b68 and
/// `FillPriorityIndex` 0x82d2c160.
fn build_priority_index<W>(
    vault: &Arc<AttributeRuntimeVault>,
    inputs: &DmoCensusInputs,
    object_id: u32,
    terms: &PriorityTerms,
) -> Result<ObjectPriorityIndex, DmoCensusError<W>> {
    let livingworld = vault.instance(LIVING_WORLD_CLASS, DYNAMIC_OBJECTS_KEY);
    // -|pos - centre|: vmsum3fp, two refinements, zero select, sign xor.
    let delta: [f32; 4] = std::array::from_fn(|lane| terms.position[lane] - inputs.center[lane]);
    let distance = f32::from_bits(length::<W>(delta)?.to_bits() ^ 0x8000_0000);
    let to_object: [f32; 4] =
        std::array::from_fn(|lane| terms.position[lane] - inputs.camera_position[lane]);
    let direction = normalize_safe::<W>(to_object, terms.default_direction)?;
    let facing = host_dot3(inputs.camera_heading, direction) > 0.0;
    let camera_plane = if facing {
        livingworld_scalar(&livingworld, CAMERA_PLANE_KEY)?
    } else {
        1.0
    };
    let in_use = if terms.in_use {
        livingworld_scalar(&livingworld, IN_USE_KEY)?
    } else {
        1.0
    };
    let use_and_plane = in_use * camera_plane;
    let priority = if terms.priority_index == -1 {
        PRIORITY_UNSET
    } else {
        #[expect(
            clippy::cast_precision_loss,
            reason = "native fcfid/frsp of the sign-extended index"
        )]
        let index = terms.priority_index as f32;
        (index * livingworld_scalar(&livingworld, PRIORITY_WEIGHT_KEY)?) * in_use
    };
    let safety = if terms.safety_index == -1 {
        SAFETY_UNSET
    } else {
        #[expect(
            clippy::cast_precision_loss,
            reason = "native fcfid/frsp of the negated sign-extended index"
        )]
        let index = terms.safety_index.wrapping_neg() as f32;
        (index * in_use) * livingworld_scalar(&livingworld, SAFETY_WEIGHT_KEY)?
    };
    let weighted = livingworld_scalar(&livingworld, DISTANCE_WEIGHT_KEY)? * distance;
    Ok(ObjectPriorityIndex {
        object_id,
        priority,
        safety,
        other_factors: weighted / (use_and_plane * camera_plane),
        multiplier: camera_plane,
    })
}

/// `LWModel::CreateBBoxVolume` 0x82e51218 -> `CreateCollisionBox` 0x82e4b630
/// -> `BoxVolume::Initialize` 0x82adcb90: identity rows gI/gJ/gK, a zero W
/// row, half extents `dims * 0.5`, radius 0, group 0, surface 0, flags 1.
pub(crate) fn bbox_volume(model: &LwDynamicObjectModel) -> Volume {
    bbox_volume_for_dimensions(model.dimensions)
}

pub(crate) fn bbox_volume_for_dimensions(dimensions: [f32; 4]) -> Volume {
    let half = dimensions.map(|lane| lane * 0.5);
    Volume {
        transform: [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 0.0,
        ],
        vtable_offset: BOX_VOLUME,
        shape_words: [half[0].to_bits(), half[1].to_bits(), half[2].to_bits()],
        radius: 0.0,
        group_id: 0,
        surface_id: 0,
        flags: 1,
    }
}

/// `AABBoxTemplate::Transform` 0x825e9480 of the model's `[min, max]`.
pub(crate) fn model_bounds(
    model: &LwDynamicObjectModel,
    transform: &[f32; 16],
) -> ([f32; 3], [f32; 3]) {
    transform_collision_aabb(
        [model.bbox_min[0], model.bbox_min[1], model.bbox_min[2]],
        [model.bbox_max[0], model.bbox_max[1], model.bbox_max[2]],
        *transform,
    )
}

/// The `vcmpgtfp` pair, `vor` and `vsldoi 12`/`vcmpeqfp.` against zero of
/// every AABB/AABB test (0x82722c70..2c94, 0x82e4bc98..bcb8): X/Y/Z only,
/// strict, so an unordered lane never separates.
pub(crate) fn aabb_separated(first: ([f32; 3], [f32; 3]), second: ([f32; 3], [f32; 3])) -> bool {
    (0..3).any(|axis| first.0[axis] > second.1[axis] || second.0[axis] > first.1[axis])
}

/// `vmaxfp`: the first NaN operand, quieted; +0 over -0.
fn vmaxfp(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        f32::from_bits(a.to_bits() | 0x0040_0000)
    } else if b.is_nan() {
        f32::from_bits(b.to_bits() | 0x0040_0000)
    } else if a > b || (a == b && b.is_sign_negative()) {
        a
    } else {
        b
    }
}

/// A `CapsuleVolume` half height and radius (`CapsuleVolume::Initialize`
/// 0x82ad7fa0 stores +0x44 and +0x50).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct IntersectionCapsule {
    pub(crate) radius: f32,
    pub(crate) half_height: f32,
}

impl IntersectionCapsule {
    /// `CapsuleVolume::Initialize` 0x82ad7fa0: `vTableArray[2]`, rows gI/gJ/gK
    /// and a zero W row, group/surface 0 and flags 1. The +0x48/+0x4c words
    /// keep allocator contents natively; capsules never read them.
    pub(crate) fn volume(self) -> Volume {
        Volume {
            transform: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                0.0, 0.0, 0.0, 0.0,
            ],
            vtable_offset: CAPSULE_VOLUME,
            shape_words: [self.half_height.to_bits(), 0, 0],
            radius: self.radius,
            group_id: 0,
            surface_id: 0,
            flags: 1,
        }
    }
}

/// `DMO_SpawnTest` cvars in `BuildSkaterTestCapsule` 0x827041a0 and
/// `BuildSkaterTestAABBox` 0x82704488 read order.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SkaterTestCvars {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) feet_padding: f32,
    pub(crate) head_offset: f32,
}

/// `BuildSkaterTestCapsule` outputs: the capsule it initializes, its
/// transform and its AABB.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SkaterTestCapsule {
    pub(crate) capsule: IntersectionCapsule,
    pub(crate) transform: [f32; 16],
    pub(crate) minimum: [f32; 4],
    pub(crate) maximum: [f32; 4],
}

impl SkaterTestCapsule {
    pub(crate) fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        (
            [self.minimum[0], self.minimum[1], self.minimum[2]],
            [self.maximum[0], self.maximum[1], self.maximum[2]],
        )
    }
}

/// `Manager::BuildSkaterTestCapsule` 0x827041a0. `dir = NormalizeSafe(head -
/// feet, 0)`; top `dir * (height - width) + feet` (`vmaddcfp128`), bottom
/// `dir * (width - padding) + feet`; half height `|top - bottom| * 0.5`
/// (`vmsum3fp`, two refinements, zero select, `fmuls`); transform rows
/// `[row0, -row2, row1, (top + bottom) * 0.5]`; AABB `[min - (w,w,w,0),
/// max + (w,w,w,0)]`.
pub(crate) fn build_skater_test_capsule(
    matrix: &[f32; 16],
    head: [f32; 4],
    feet: [f32; 4],
    cvars: &SkaterTestCvars,
) -> Result<SkaterTestCapsule, ReckoningHostNumericError> {
    let delta: [f32; 4] = std::array::from_fn(|lane| head[lane] - feet[lane]);
    let direction = host_normalize_safe(delta, [0.0; 4])?;
    let bottom_scale = cvars.width - cvars.feet_padding;
    let top_scale = cvars.height - cvars.width;
    let top: [f32; 4] = std::array::from_fn(|lane| direction[lane].mul_add(top_scale, feet[lane]));
    let bottom: [f32; 4] =
        std::array::from_fn(|lane| direction[lane].mul_add(bottom_scale, feet[lane]));
    let span: [f32; 4] = std::array::from_fn(|lane| top[lane] - bottom[lane]);
    let centre: [f32; 4] = std::array::from_fn(|lane| (top[lane] + bottom[lane]) * 0.5);
    let half_height = host_length(span)? * 0.5;
    let pad = [cvars.width, cvars.width, cvars.width, 0.0];
    let minimum = std::array::from_fn(|lane| vminfp(top[lane], bottom[lane]) - pad[lane]);
    let maximum = std::array::from_fn(|lane| vmaxfp(top[lane], bottom[lane]) + pad[lane]);
    let mut transform = [0.0; 16];
    for lane in 0..4 {
        transform[lane] = matrix[lane];
        transform[4 + lane] = f32::from_bits(matrix[8 + lane].to_bits() ^ 0x8000_0000);
        transform[8 + lane] = matrix[4 + lane];
        transform[12 + lane] = centre[lane];
    }
    Ok(SkaterTestCapsule {
        capsule: IntersectionCapsule {
            radius: cvars.width,
            half_height,
        },
        transform,
        minimum,
        maximum,
    })
}

/// `Manager::BuildSkaterTestAABBox` 0x82704488: the min/max of `head + (w,
/// headOffset, w, 0)` and `feet - (w, 0, w, 0)`.
pub(crate) fn build_skater_test_aabbox(
    head: [f32; 4],
    feet: [f32; 4],
    cvars: &SkaterTestCvars,
) -> ([f32; 3], [f32; 3]) {
    let up = [cvars.width, cvars.head_offset, cvars.width, 0.0];
    let down = [cvars.width, 0.0, cvars.width, 0.0];
    let top: [f32; 4] = std::array::from_fn(|lane| head[lane] + up[lane]);
    let base: [f32; 4] = std::array::from_fn(|lane| feet[lane] - down[lane]);
    (
        std::array::from_fn(|axis| vminfp(top[axis], base[axis])),
        std::array::from_fn(|axis| vmaxfp(top[axis], base[axis])),
    )
}

impl<W: DmoWorld> DmoFrame<'_, W> {
    /// `GeneratePriorityIndexForLocator` 0x82d34b68; `None` for a missing
    /// locator (the native false return).
    fn generate_priority_index_for_locator(
        &mut self,
        locator_id: u64,
        inputs: &DmoCensusInputs,
    ) -> FrameResult<Option<ObjectPriorityIndex>, W> {
        let Some(locator) = self.locators.locator(locator_id) else {
            return Ok(None);
        };
        let in_session = locator.flags & LOCATOR_IN_SESSION_MARKER != 0;
        let position = row3(&locator.transform);
        let model = self
            .models
            .model(locator.model_id)
            .ok_or(DmoCensusError::Assert(
                "GeneratePriorityIndexForLocator model",
            ))?;
        let priority_index = if in_session {
            -1
        } else {
            priority_value(self.vault, model)?
        };
        let safety_index = self
            .locators
            .persistence_data(locator_id)
            .map_or(1, |data| data.safety);
        let terms = PriorityTerms {
            position,
            priority_index,
            safety_index,
            in_use: in_session,
            default_direction: [1.0, 0.0, 0.0, 0.0],
        };
        build_priority_index(self.vault, inputs, 0, &terms).map(Some)
    }

    /// `DynamicObject::FillPriorityIndex` 0x82d2c160. In use means
    /// `IsInUseByPlayer`, a hand prop or the
    /// locator's in-session marker; the priority index is -1 only for the
    /// marker.
    fn fill_priority_index(
        &self,
        entity_id: u32,
        inputs: &DmoCensusInputs,
    ) -> FrameResult<ObjectPriorityIndex, W> {
        let object = self
            .objects
            .object(entity_id)
            .ok_or(DmoCensusError::Assert("FillPriorityIndex entity"))?;
        let mut in_session = false;
        if object.locator_id != NULL_ASSET_ID
            && let Some(locator) = self.locators.locator(object.locator_id)
        {
            in_session = locator.flags & LOCATOR_IN_SESSION_MARKER != 0;
        }
        let priority_index = if in_session {
            -1
        } else {
            priority_value(self.vault, &object.model)?
        };
        let terms = PriorityTerms {
            position: row3(&object.transform),
            priority_index,
            safety_index: i32::from_ne_bytes(object.safety_index.to_ne_bytes()),
            in_use: object.is_in_use_by_player() || object.is_set_as_hand_prop() || in_session,
            default_direction: inputs.camera_heading,
        };
        build_priority_index(self.vault, inputs, entity_id, &terms)
    }

    /// `EvaluateCensus` 0x82727a38, DMO component only.
    pub(crate) fn try_evaluate_census(
        &mut self,
        inputs: &DmoCensusInputs,
        cvars: &DmoCensusCvars,
    ) -> FrameResult<(), W> {
        let Some(plan) = census_plan(self.census, inputs, cvars, self.vault)? else {
            return Ok(());
        };
        if !self.census.sampled_fe {
            self.try_census_update()?;
        }
        if plan.cull_dmos {
            self.try_evaluate_dynamic_object_census(inputs, cvars, true, false)?;
        }
        self.try_census_update()?;
        if plan.spawn_dmos {
            self.try_evaluate_dynamic_object_census(inputs, cvars, false, true)?;
        }
        if plan.advance {
            self.census.component_index = (self.census.component_index + 1) % 4;
        }
        Ok(())
    }

    /// `CensusMan::Update` 0x82702780: +0xa9 bit 7, then each entity census
    /// `Update` (the DMO one drains the cull map).
    pub(crate) fn try_census_update(&mut self) -> FrameResult<(), W> {
        self.census.sampled_fe = true;
        self.try_process_cull_map()
    }

    /// `LWEntityCensusMan::Update` 0x82e67b10: `Cull` 0x82d2b878 ->
    /// `DestroyEntity` for every mapped entity, then the map is cleared.
    pub(crate) fn try_process_cull_map(&mut self) -> FrameResult<(), W> {
        for pending in std::mem::take(&mut self.objects.cull_map) {
            // DynamicObject destructor 0x82d36088 calls Grabbable::DeInit
            // 0x82e62c00 before Collideable::DeInit removes the body.
            for id in pending.grab_splines.keys() {
                self.objects.grabbables.remove(*id);
            }
            // Collideable.DeInit removes the assembly map entry before the
            // collision object's destructor removes its body from the Island.
            self.objects.collideables.remove(&pending.body);
            self.world
                .try_remove_body(&pending.body)
                .map_err(DmoCensusError::World)?;
        }
        Ok(())
    }

    /// `EvaluateDynamicObjectCensus` 0x82726930.
    pub(crate) fn try_evaluate_dynamic_object_census(
        &mut self,
        inputs: &DmoCensusInputs,
        cvars: &DmoCensusCvars,
        do_cull: bool,
        do_spawn: bool,
    ) -> FrameResult<(), W> {
        let ranges = self
            .vault
            .instance(CENSUS_RANGES_CLASS, self.census.dynamic_objects_range_key);
        if ranges.is_null() {
            return Err(DmoCensusError::Assert("CensusMan.cpp line 398"));
        }
        let layout = ranges
            .try_cached_layout_field(0, 0x28)?
            .ok_or(DmoCensusError::Assert("livingworld_census_ranges layout"))?;
        let bytes = layout.bytes();
        let circle = |base: usize| CensusCircle {
            no_spawn: read_f32(bytes, base),
            spawn: read_f32(bytes, base + 4),
            cull: read_f32(bytes, base + 8),
            shift: read_f32(bytes, base + 12),
            speed: read_f32(bytes, base + 16),
        };
        let speed = length::<W::Error>(inputs.velocity)? * SPEED_SCALE;
        let evaluated = evaluate_census_circle(speed, circle(0), circle(0x14));
        let direction = normalize_safe::<W::Error>(inputs.velocity, [0.0; 4])?;
        let center: [f32; 4] = std::array::from_fn(|lane| {
            direction[lane].mul_add(evaluated.shift, inputs.center[lane])
        });
        if do_cull {
            self.try_evaluative_cull(center, evaluated.cull, true)?;
        }
        if do_spawn && cvars.enable_spawn_dynamic_objects && self.census.enable & 0x04 != 0 {
            self.try_evaluative_spawn(
                inputs,
                cvars,
                center,
                evaluated.no_spawn,
                evaluated.spawn,
                evaluated.cull,
            )?;
        }
        Ok(())
    }

    /// `EvaluativeCullDynamicObjects` 0x82724090 over the map-order snapshot.
    fn try_evaluative_cull(
        &mut self,
        center: [f32; 4],
        cull_radius: f32,
        cull_outside: bool,
    ) -> FrameResult<(), W> {
        for id in self.objects.order.ids() {
            let Some(object) = self.objects.object(id) else {
                continue;
            };
            if !object.allow_cull {
                continue;
            }
            let delta = [
                center[0] - object.transform[12],
                0.0,
                center[2] - object.transform[14],
                0.0,
            ];
            let difference = host_dot3(delta, delta) - cull_radius * cull_radius;
            let cull = if cull_outside {
                !(difference < 0.0)
            } else {
                !(difference > 0.0)
            };
            if cull {
                self.try_request_explicit_recursive_cull(id)?;
            }
        }
        Ok(())
    }

    /// `DynamicObject::RequestExplicitRecursiveCull` 0x82d38378.
    fn try_request_explicit_recursive_cull(&mut self, entity_id: u32) -> FrameResult<(), W> {
        let mut list = Vec::new();
        self.try_build_recursive_cull_list(entity_id, &mut list)?;
        for id in list {
            if id != entity_id {
                self.try_request_explicit_cull(id)?;
            }
        }
        self.try_request_explicit_cull(entity_id)
    }

    /// `BuildRecursiveObjectCullList` 0x82d37aa8: the model bbox volume at
    /// `GetBBoxTransform(transform)`, recursing into every newly added ID.
    fn try_build_recursive_cull_list(
        &mut self,
        entity_id: u32,
        list: &mut Vec<u32>,
    ) -> FrameResult<(), W> {
        let Some(object) = self.objects.object(entity_id) else {
            return Ok(());
        };
        let model = Arc::clone(&object.model);
        let transform = object.transform;
        let bbox_transform = model.bbox_transform(&transform);
        let before = list.len();
        self.try_build_overlap_id_list(
            list,
            &bbox_volume(&model),
            &bbox_transform,
            model_bounds(&model, &transform),
            RECURSIVE_CULL_PADDING,
        )?;
        let added = list[before..].to_vec();
        for id in added {
            if id != entity_id {
                self.try_build_recursive_cull_list(id, list)?;
            }
        }
        Ok(())
    }

    /// `BuildOverlapIDList` 0x82d37958: overlapping pool members mapped to
    /// owner IDs; zero and repeats are dropped.
    fn try_build_overlap_id_list(
        &mut self,
        list: &mut Vec<u32>,
        volume: &Volume,
        transform: &[f32; 16],
        bounds: ([f32; 3], [f32; 3]),
        padding: f32,
    ) -> FrameResult<(), W> {
        let bodies = self
            .world
            .try_volume_test(DmoVolumeTest {
                volume,
                transform,
                bounds,
                padding,
                max_assemblies: OVERLAP_ASSEMBLIES,
            })
            .map_err(DmoCensusError::World)?;
        for body in &bodies {
            let owner = self.objects.owner_of(body);
            if owner != 0 && !list.contains(&owner) {
                if list.len() == ENTITY_CAPACITY {
                    return Err(DmoCensusError::Capacity("overlap ID list"));
                }
                list.push(owner);
            }
        }
        Ok(())
    }

    /// `DynamicObject::RequestExplicitCull` 0x82d36390 -> `RequestCull`
    /// 0x8271a5e8. A first request runs `OnCullRequested` 0x82d3cda0
    /// (priority tracking removal, `DetachObjectFromLocator`),
    /// `DetachFromManagers` 0x82d36f30 (map erase only) and enters the map.
    pub(crate) fn try_request_explicit_cull(&mut self, entity_id: u32) -> FrameResult<(), W> {
        let world=&mut self.world;
        self.objects.try_request_explicit_cull_with_lifecycle(
            entity_id,
            self.locators,
            &mut FrameLocatorServices,
            |objects,locators,event|match event{
                DmoCullEvent::BeforeCull(id)=>world.try_before_dmo_cull(objects,locators,id),
                DmoCullEvent::Monitor{entity,monitor}=>world.try_notify_dmo_monitor(objects,monitor,entity),
            }.map_err(|e|format!("{e:?}")),
        )?;
        Ok(())
    }

    /// `EvaluativeSpawnDynamicObjects` 0x8271c690.
    #[expect(
        clippy::too_many_lines,
        reason = "one native census routine with its candidate expansion"
    )]
    fn try_evaluative_spawn(
        &mut self,
        inputs: &DmoCensusInputs,
        cvars: &DmoCensusCvars,
        center: [f32; 4],
        inner_radius: f32,
        outer_radius: f32,
        cull_radius: f32,
    ) -> FrameResult<(), W> {
        let mut budget = if inputs.initial {
            100
        } else {
            cvars.max_dynamic_objects
        };
        if center[0].is_nan() || center[2].is_nan() {
            return Err(DmoCensusError::Assert("CensusMan.cpp line 1448"));
        }
        let found = self.locators.passive_locators_within_range(
            [center[0], center[2]],
            outer_radius,
            PASSIVE_QUERY_ITEMS,
            &mut FrameLocatorServices,
        )?;
        let mut candidates: Vec<u64> = Vec::new();
        candidates
            .try_reserve_exact(CANDIDATE_CAPACITY)
            .map_err(|_| DmoCensusError::Allocation)?;
        for (id, distance) in found {
            // 0x8271c83c: outside the initial spawn, `dist < NoSpawn` nulls it.
            if !inputs.initial && distance < inner_radius {
                continue;
            }
            candidates.push(id);
        }
        let mut index = 0;
        while index < candidates.len() {
            let candidate = candidates[index];
            index += 1;
            let Some(locator) = self.locators.locator(candidate) else {
                continue;
            };
            let Some(model) = self.models.model(locator.model_id) else {
                continue;
            };
            let own_id = locator.locator_id;
            let bbox = model.bbox_transform(&locator.transform);
            let first_radius = length::<W::Error>(model.dimensions)?;
            let close = self.locators.passive_locators_within_range(
                [bbox[12], bbox[14]],
                cvars.close_by_locator_radius,
                CLOSE_QUERY_ITEMS,
                &mut FrameLocatorServices,
            )?;
            for (other, _) in close {
                if candidates.len() >= CANDIDATE_CAPACITY {
                    break;
                }
                if other == own_id {
                    continue;
                }
                let Some(other_locator) = self.locators.locator(other) else {
                    continue;
                };
                let delta: [f32; 4] =
                    std::array::from_fn(|lane| other_locator.transform[12 + lane] - center[lane]);
                if host_dot3(delta, delta) - cull_radius * cull_radius > 0.0 {
                    continue;
                }
                if candidates.contains(&other) {
                    continue;
                }
                let Some(other_model) = self.models.model(other_locator.model_id) else {
                    continue;
                };
                let other_bbox = other_model.bbox_transform(&other_locator.transform);
                let second_radius = length::<W::Error>(other_model.dimensions)?;
                let offset = [
                    other_bbox[12] - bbox[12],
                    0.0,
                    other_bbox[14] - bbox[14],
                    0.0,
                ];
                let reach = (second_radius + first_radius) * 0.5;
                if host_dot3(offset, offset) - reach * reach < 0.0 {
                    candidates.push(other);
                }
            }
        }
        for candidate in candidates {
            if budget <= 0 {
                break;
            }
            let Some(locator) = self.locators.locator(candidate) else {
                continue;
            };
            if locator.flags & LOCATOR_ENABLED == 0 || locator.user_object_id != 0 {
                continue;
            }
            let transform = locator.transform;
            let model_id = locator.model_id;
            let locator_id = locator.locator_id;
            if model_id == NULL_ASSET_ID {
                continue;
            }
            let Some(model) = self.models.model(model_id).map(Arc::clone) else {
                continue;
            };
            let mut lowest = 0;
            let mut accepted = self.generate_priority_index_for_locator(locator_id, inputs)?;
            if let Some(priority) = accepted
                && i64::from(cvars.max_dynamic_objects) <= self.objects.count() as i64
            {
                if self.objects.priorities.is_higher_than_lowest(&priority) {
                    lowest = self.objects.priorities.lowest();
                } else {
                    accepted = None;
                }
            }
            let Some(priority) = accepted else {
                continue;
            };
            let score = priority.safety + priority.priority;
            let mut overlaps = Vec::new();
            self.try_build_overlap_id_list(
                &mut overlaps,
                &bbox_volume(&model),
                &transform,
                model_bounds(&model, &transform),
                SPAWN_OVERLAP_PADDING,
            )?;
            let mut blocked = false;
            for &id in &overlaps {
                if self.objects.object(id).is_none() {
                    continue;
                }
                let other = self.fill_priority_index(id, inputs)?;
                if score <= (other.safety + other.priority) + OVERLAP_PRIORITY_MARGIN {
                    blocked = true;
                    break;
                }
            }
            if blocked {
                continue;
            }
            for &id in &overlaps {
                self.try_request_explicit_cull(id)?;
            }
            // No vehicle is ever spawned: `LWVehicleMan::BuildModelOverlapIDList`
            // returns an empty list.
            if lowest != 0 {
                self.try_request_explicit_cull(lowest)?;
            }
            self.try_process_cull_map()?;
            if self.try_spawn_dynamic_object_id(locator_id, &model, &transform, cvars)?.is_some() {
                budget -= 1;
            }
        }
        Ok(())
    }

    /// `SpawnDynamicObject` 0x82726c08 with `[bInitial, bAllowCull = 1,
    /// bForce = 0, bAllowWorldCollisions = 1]`: census bit 7, the
    /// `livingworld_entities` collection, a registered model and
    /// `kMAXDYNAMICOBJECTS > count`; then `RequestSpawn` and
    /// `CheckSpawnedDynamicObjectAgainstSkaters`.
    pub(crate) fn try_spawn_dynamic_object_id(
        &mut self,
        locator_id: u64,
        model: &Arc<LwDynamicObjectModel>,
        transform: &[f32; 16],
        cvars: &DmoCensusCvars,
    ) -> FrameResult<Option<u32>, W> {
        if self.census.enable & 0x80 == 0 {
            return Ok(None);
        }
        if !self
            .vault
            .has_collection(ENTITIES_CLASS, DYNAMIC_OBJECTS_KEY)
        {
            return Ok(None);
        }
        if model.model_id == NULL_ASSET_ID || self.models.model(model.model_id).is_none() {
            return Ok(None);
        }
        // `mValue <= mnElementCount` compares unsigned words.
        let maximum = u32::from_ne_bytes(cvars.max_dynamic_objects.to_ne_bytes());
        if maximum == 0 || maximum as usize <= self.objects.count() {
            return Ok(None);
        }
        let Some(entity_id) = self.try_spawn(locator_id, model, transform)? else {
            return Ok(None);
        };
        self.world
            .try_check_spawned_against_skaters(&mut *self.objects, entity_id)
            .map_err(DmoCensusError::World)?;
        Ok(Some(entity_id))
    }

    /// `LWDynamicObjectCensusMan::Spawn` 0x82d31ba0, `AttachToManagers`
    /// 0x82d36eb0 and `OnSpawnComplete` 0x82d3cc58.
    fn try_spawn(
        &mut self,
        locator_id: u64,
        model: &Arc<LwDynamicObjectModel>,
        transform: &[f32; 16],
    ) -> FrameResult<Option<u32>, W> {
        if self.objects.spawn_unfinished.is_some(){return Err(DmoCensusError::Capacity("DMO unfinished spawn prefix"));}
        // SpawnDynamicObject82726d74 -> LWCommonSpawnInfo+10; Spawn82d31ba0
        // copies this into creation info; ctor82d3de80 publishes mModelKey.
        let presentation_model_key = self
            .vault
            .instance(ENTITIES_CLASS, DYNAMIC_OBJECTS_KEY)
            .try_cached_layout_field(8, 8)?
            .map_or(0, |data| {
                u64::from_be_bytes(data.bytes().try_into().expect("checked field size"))
            });
        // Constructor 0x82d3dc58: flags `(0 & 0x19) | 0x80`, STATIC from the
        // locator's bit 3; InitCollisionBodies' group from the mass.
        let entity_id = self
            .entity_ids
            .allocate(crate::world::LivingWorldEntityType::DYNAMIC_OBJECT);
        let mut flags = FLAG_CONSTRUCTED;
        if self
            .locators
            .locator(locator_id)
            .is_some_and(|locator| locator.flags & LOCATOR_STATIC != 0)
        {
            flags |= FLAG_STATIC;
        }
        let group = if model.mass < LIGHT_MASS {
            GROUP_LIGHT
        } else {
            GROUP_HEAVY
        };
        // SetTargetTransform(M); AddModelsToWorld's SetTransform(target) and
        // FillCollisionPartTargetTransforms: row3 = fma(velocity 0, dt, row3).
        let target = super::collision_drive::collision_part_target(
            std::array::from_fn(|axis| std::array::from_fn(|lane| transform[axis * 4 + lane])),
            [0.; 4],
            true,
            0.,
        );
        let part = std::array::from_fn(|i| target[i / 4][i % 4]);
        let state = if flags & FLAG_STATIC != 0 {
            BODY_STATE_STATIC
        } else {
            BODY_STATE_ACTIVE
        };
        let body = self
            .world
            .try_add_body(model, entity_id, group, &part, state)
            .map_err(DmoCensusError::World)?;
        let object = DynamicObject {
            contained_props: Vec::new(),
            contained_props_initialized: false,
            current_losim: 0,
            explicit_update_requested: false,
            player_distance: super::entity_core::distance::EntityDistanceStack::from_cleared_memstore(),
            attrib_cache: None,
            entity_id,
            rain_data: None,
            presentation_model_key,
            locator_id: NULL_ASSET_ID,
            model: Arc::clone(model),
            transform: *transform,
            target_transform: *transform,
            velocity: [0.0; 4],
            farthest_corners: [[0.;4];4],
            allow_cull: true,
            flags,
            skater_collision_flags: 0,
            part0_contacts: Vec::new(),
            safety_index: 1,
            wp_safety_index: 1,
            transform_type: DynamicObjectTransformType::Physics,
            hand_prop_cull_timer: 0.,
            collision_ignore_gravity: false,
            monitoring_entities: Vec::new(),
            body,
            group,
            grab_splines: self.objects.register_grab_splines(
                entity_id,
                &model.grab_splines,
                self.spline_ids,
            ),
            grind_splines: self.spline_ids.register_grinds(&model.grind_splines),
            online_static: false,
        };
        self.objects.spawn_unfinished=Some(object);
        self.objects
            .collideables
            .register(&self.objects.spawn_unfinished.as_ref().expect("pending spawn retained").body, entity_id)
            .map_err(DmoCensusError::AssemblyOwner)?;
        // Spawn82d31d60 initializes children before RequestSpawn attaches the parent.
        self.world.try_initialize_dmo_props(self.objects.spawn_unfinished.as_mut().expect("pending spawn retained"),
            self.entity_ids,self.census.enable,self.models).map_err(DmoCensusError::World)?;
        self.objects
            .attach_spawn_unfinished()
            .map_err(DmoCensusError::Capacity)?;
        // OnSpawnComplete: AttachObjectToLocator, then priority tracking when
        // the locator allows it (or there is no locator).
        let mut track = true;
        if locator_id != NULL_ASSET_ID {
            if let Some(object) = self.objects.object_mut(entity_id) {
                object.locator_id = locator_id;
            }
            self.locators
                .attach_object(locator_id, entity_id, &mut FrameLocatorServices)?;
            if let Some(locator) = self.locators.locator(locator_id) {
                track = locator.flags & LOCATOR_PRIORITY_TRACKING != 0;
            }
        }
        if track {
            self.try_add_priority_tracking(entity_id)?;
        }
        Ok(Some(entity_id))
    }

    /// `Dmo::SimManager::SpawnStatic` (`0x825eb0a0`)'s body at one locator of an online game,
    /// held by a dynamic object so it collides, grinds and draws as the living world's do: static
    /// (`IsStatic`, which nothing grabs), never culled, with no locator, priority, children or
    /// grab splines, as a `Dmo::Body` has none. A full entity map takes no more, as a full
    /// `mStaticSim` (50) does not.
    pub(crate) fn try_spawn_static(
        &mut self,
        model: &Arc<LwDynamicObjectModel>,
        transform: &[f32; 16],
    ) -> FrameResult<Option<u32>, W> {
        if self.objects.spawn_unfinished.is_some() {
            return Err(DmoCensusError::Capacity("DMO unfinished spawn prefix"));
        }
        if self.objects.count() >= ENTITY_CAPACITY {
            return Ok(None);
        }
        let presentation_model_key = self
            .vault
            .instance(ENTITIES_CLASS, DYNAMIC_OBJECTS_KEY)
            .try_cached_layout_field(8, 8)?
            .map_or(0, |data| {
                u64::from_be_bytes(data.bytes().try_into().expect("checked field size"))
            });
        let entity_id = self
            .entity_ids
            .allocate(crate::world::LivingWorldEntityType::DYNAMIC_OBJECT);
        let body = self
            .world
            .try_add_static_body(model, entity_id, transform)
            .map_err(DmoCensusError::World)?;
        let object = DynamicObject {
            contained_props: Vec::new(),
            contained_props_initialized: false,
            current_losim: 0,
            explicit_update_requested: false,
            player_distance: super::entity_core::distance::EntityDistanceStack::from_cleared_memstore(),
            attrib_cache: None,
            entity_id,
            rain_data: None,
            presentation_model_key,
            locator_id: NULL_ASSET_ID,
            model: Arc::clone(model),
            transform: *transform,
            target_transform: *transform,
            velocity: [0.0; 4],
            farthest_corners: [[0.; 4]; 4],
            allow_cull: false,
            flags: FLAG_CONSTRUCTED | FLAG_STATIC,
            skater_collision_flags: 0,
            part0_contacts: Vec::new(),
            safety_index: 1,
            wp_safety_index: 1,
            transform_type: DynamicObjectTransformType::Physics,
            hand_prop_cull_timer: 0.,
            collision_ignore_gravity: false,
            monitoring_entities: Vec::new(),
            body,
            group: 0,
            grab_splines: BTreeMap::new(),
            grind_splines: self.spline_ids.register_grinds(&model.grind_splines),
            online_static: true,
        };
        self.objects.spawn_unfinished = Some(object);
        self.objects
            .collideables
            .register(&self.objects.spawn_unfinished.as_ref().expect("pending spawn retained").body, entity_id)
            .map_err(DmoCensusError::AssemblyOwner)?;
        self.objects
            .attach_spawn_unfinished()
            .map_err(DmoCensusError::Capacity)?;
        Ok(Some(entity_id))
    }

    /// `AddObjectToPriorityIndexTracking` 0x82d377e8: a zeroed index with
    /// only the object ID, pushed once.
    fn try_add_priority_tracking(&mut self, entity_id: u32) -> FrameResult<(), W> {
        if self.objects.priorities.position(entity_id).is_some() {
            return Ok(());
        }
        let index = ObjectPriorityIndex {
            object_id: entity_id,
            priority: 0.0,
            safety: 0.0,
            other_factors: 0.0,
            multiplier: 0.0,
        };
        if !self.objects.priorities.push(index) {
            return Err(DmoCensusError::Capacity("object priority queue"));
        }
        Ok(())
    }

    /// `Manager::UpdatePostTransforms` 0x827031e8, DMO portion:
    /// `UpdateTargetTransforms`, `RequestAllConditionBasedCull` 0x82d30558,
    /// then `LWDynamicObjectMan::UpdateTransforms` 0x82d3d6f8 (component
    /// transforms, `UpdatePriorities` 0x82d38a18, `UpdatePersistence`
    /// 0x82d3d470).
    pub(crate) fn try_update_post_transforms(
        &mut self,
        inputs: &DmoCensusInputs,
        dt: f32,
    ) -> FrameResult<(), W> {
        self.try_update_post_targets(dt)?;
        self.try_update_post_positions(inputs, dt)
    }
    pub(crate) fn try_update_post_targets(&mut self, dt:f32) -> FrameResult<(), W> {
        for id in self.objects.order.ids() {
            self.try_update_target_transform(id,dt)?;
        }
        for id in self.objects.order.ids() {
            if self
                .objects
                .object(id)
                .is_some_and(|object| object.flags & FLAG_CONDITION_CULL != 0)
            {
                self.try_request_explicit_cull(id)?;
            }
        }
        Ok(())
    }
    pub(crate) fn try_update_post_positions(
        &mut self,
        inputs: &DmoCensusInputs,
        dt: f32,
    ) -> FrameResult<(), W> {
        for id in self.objects.order.ids() {
            self.try_update_transform(id, dt)?;
            // ComponentMan.UpdateTransforms82d2f664: the post callback belongs
            // to each entity, before advancing to the next hash-map member.
            let object=self.objects.object(id).ok_or(DmoCensusError::Assert("PostUpdateTransform entity absent"))?;
            self.world.try_update_dmo_contained_props(&object.contained_props,object.transform)
                .map_err(DmoCensusError::World)?;
        }
        for id in self.objects.order.ids() {
            self.try_update_object_priority_index(id, inputs)?;
        }
        self.locators.advance_tick();
        for id in self.objects.order.ids() {
            self.try_update_persistence(id)?;
        }
        self.world.try_update_reset_capsules(self.objects).map_err(DmoCensusError::World)?;
        Ok(())
    }

    /// `DynamicObject::UpdateTargetTransform` 0x82d2e7a8: Part 0's transform
    /// (the body always exists here), a NaN check setting the condition-cull
    /// flag, and `SetTargetTransform` for a clean matrix. `UpdateUprighting`
    /// acts only on flag 0x40, which nothing sets.
    fn try_update_target_transform(&mut self, entity_id: u32, dt:f32) -> FrameResult<(), W> {
        let Some(object) = self.objects.object(entity_id) else {
            return Ok(());
        };
        // UpdateTargetTransform82d2e7c8/9dc: type1 retains the target just
        // supplied by its pedestrian; only type0 copies the physical Part.
        if object.is_set_as_hand_prop() { return Ok(()); }
        let body = object.body.clone();
        let transform = self
            .world
            .try_part_transform(&body)
            .map_err(DmoCensusError::World)?;
        let object = self
            .objects
            .object_mut(entity_id)
            .expect("entity checked above");
        let invalid = matrix_has_nan(&transform);
        object.flags =
            (object.flags & !FLAG_CONDITION_CULL) | if invalid { FLAG_CONDITION_CULL } else { 0 };
        if !invalid {
            object.target_transform = transform;
            self.world.try_update_uprighting(object).map_err(DmoCensusError::World)?;
        }
        // UpdateTargetTransform82d2e970..e9c8: only released/no-locator objects;
        // held transform type1 returned above, so its timer stays unchanged.
        if object.locator_id==NULL_ASSET_ID {object.hand_prop_cull_timer-=dt;}
        Ok(())
    }

    /// `DynamicObject::UpdateTransform` 0x82d3d148: `LWBasicEntity::
    /// UpdateTransform` 0x82e5b358 (velocity from the refined reciprocal of
    /// `dT`, then the target becomes the transform) and the locator follows.
    fn try_update_transform(&mut self, entity_id: u32, dt: f32) -> FrameResult<(), W> {
        //82d3d148..1e8: notify a snapshot of monitors BEFORE publishing the
        //target transform. The VMX comparison tests whether any XYZ lane
        //exceeds SMALL_FLOAT_153; the W lane is replaced by X.
        let Some(object) = self.objects.object(entity_id) else { return Ok(()); };
        let delta: [f32;3] = std::array::from_fn(|i|object.target_transform[12+i]-object.transform[12+i]);
        if delta.iter().any(|value|value.abs()>f32::from_bits(0x3780_0000)) {
            let monitors=object.monitoring_entities.clone(); //82d3bc98 snapshot
            for monitor in monitors {
                self.world.try_notify_dmo_movement(monitor,entity_id,delta).map_err(DmoCensusError::World)?;
            }
        }
        let Some(object) = self.objects.object_mut(entity_id) else {
            return Ok(());
        };
        object.velocity = super::entity_core::target_velocity(
            std::array::from_fn(|lane| object.transform[12 + lane]),
            std::array::from_fn(|lane| object.target_transform[12 + lane]),
            dt,
            &mut HostReckoningNumeric,
        )?;
        object.transform = object.target_transform;
        object.update_farthest_corners();
        let locator_id = object.locator_id;
        let transform = object.transform;
        if locator_id != NULL_ASSET_ID {
            self.locators
                .set_locator_transform::<DmoFrameServiceError>(locator_id, &transform)?;
        }
        Ok(())
    }

    /// `UpdateObjectPriorityIndex` 0x82d378b8: refill the heap slot in place,
    /// then `change`.
    fn try_update_object_priority_index(
        &mut self,
        entity_id: u32,
        inputs: &DmoCensusInputs,
    ) -> FrameResult<(), W> {
        let Some(position) = self.objects.priorities.position(entity_id) else {
            return Ok(());
        };
        let index = self.fill_priority_index(entity_id, inputs)?;
        self.objects.priorities.heap[position] = index;
        self.objects.priorities.change(position);
        Ok(())
    }

    /// `UpdateObjectPersistentData` 0x82d3c4b0 for one entity.
    fn try_update_persistence(&mut self, entity_id: u32) -> FrameResult<(), W> {
        let Some(object) = self.objects.object(entity_id) else {
            return Ok(());
        };
        if object.locator_id == NULL_ASSET_ID {
            return Ok(());
        }
        let priority = object
            .model
            .characteristics
            .try_cached_layout_field(LAYOUT_PERSISTENCE_PRIORITY, 4)?
            .map_or(0, |data| {
                i32::from_be_bytes(data.bytes().try_into().expect("four bytes"))
            });
        let input = ObjectPersistenceInput {
            locator_id: object.locator_id,
            transform: object.transform,
            safety: i32::from_ne_bytes(object.safety_index.to_ne_bytes()),
            priority,
        };
        self.locators
            .update_object_persistent_data(input, &mut FrameLocatorServices)?;
        Ok(())
    }

    /// Spatial lookups for the next queries: `ClearSpatialLookups` then
    /// `UpdateSpatialLookups` (Manager::Update1 after the census).
    pub(crate) fn rebuild_spatial_lookups(&mut self) {
        self.objects.rebuild_spatial_lookups();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(id: u32, priority: f32) -> ObjectPriorityIndex {
        ObjectPriorityIndex {
            object_id: id,
            priority,
            safety: 0.0,
            other_factors: 0.0,
            multiplier: 1.0,
        }
    }

    #[test]
    fn state_out_contacts_keep_part_zero_first_twelve_with_the_surface_bits() {
        assert_eq!(surface_contact(0x300 | 0x05), (0x05, true));
        assert_eq!(surface_contact(0x480 | 0x7f), (0x7f, true));
        assert_eq!(surface_contact(0x380 | 0x21), (0x21, false));
        assert_eq!(surface_contact(0xf300 | 0x01), (0x01, true));
        let mut contacts = Vec::new();
        for index in 0..13 {
            let info = DmoContactInfo { audio_surface: index, collision_force: index as f32 };
            add_part0_contact(&mut contacts, 0, info);
        }
        add_part0_contact(&mut contacts, 1, DmoContactInfo { audio_surface: 99, collision_force: 99.0 });
        assert_eq!(contacts.len(), 12);
        assert_eq!(contacts.last().map(|info| info.audio_surface), Some(11));
    }

    #[test]
    fn priority_heap_keeps_the_lowest_score_on_top() {
        let mut queue = ObjectPriorityQueue::default();
        for (id, priority) in [(1, 5.0), (2, 1.0), (3, 9.0), (4, 3.0)] {
            assert!(queue.push(index(id, priority)));
        }
        assert_eq!(queue.lowest(), 2);
        queue.remove(2);
        assert_eq!(queue.lowest(), 4);
        let position = queue.position(1).unwrap();
        queue.heap[position] = index(1, 0.5);
        queue.change(position);
        assert_eq!(queue.lowest(), 1);
        assert!(queue.is_higher_than_lowest(&index(9, 2.0)));
        assert!(!queue.is_higher_than_lowest(&index(9, 0.25)));
    }

    #[test]
    fn entity_order_walks_buckets_with_front_insertion() {
        let mut order = EntityOrder::new();
        for id in [48, 1, 95, 2] {
            order.insert(id);
        }
        // 48 % 47 == 1 and 95 % 47 == 1: newest first within bucket 1.
        assert_eq!(order.ids(), vec![95, 1, 48, 2]);
        order.remove(1);
        assert_eq!(order.ids(), vec![95, 48, 2]);
    }

    #[test]
    fn grab_id_sequence_consumes_rejected_entries_and_owner_capacity_is_separate() {
        let mut manager = DmoObjectMan::try_construct().unwrap();
        // Registry-only fixture: registration retains references without
        // reading geometry, just as AddGrabSpline copies a pointer.
        let source = DmoGrabSpline {
            entry_index: 0,
            header_offset: 0,
            data: Arc::new(skate2_formats::pegasus::GrabData {
                grabs_offset: 0,
                grabs: vec![],
            }),
        };
        let many = vec![source.clone(); 65];
        let mut ids = super::super::spline_registration::LivingWorldSplineIds::new();
        let first = manager.register_grab_splines(1, &many, &mut ids);
        assert_eq!(first.len(), 64);
        assert_eq!(ids.next_grab, 66);
        assert_eq!(manager.grab_spline_owner(64), 1);
        assert_eq!(manager.grab_spline_owner(65), 0);
        for owner in 2..=8 {
            manager.register_grab_splines(owner, &many[..64], &mut ids);
        }
        assert_eq!(manager.grabbables.len(), 512);
        let next_id = ids.next_grab;
        let orphan = manager.register_grab_splines(9, &[source.clone()], &mut ids);
        assert!(orphan.contains_key(&next_id));
        assert_eq!(manager.grab_spline_owner(next_id), 0);
        ids.next_grab = u32::MAX;
        let wrapped = manager.register_grab_splines(10, &[source.clone(), source], &mut ids);
        assert_eq!(
            wrapped.keys().copied().collect::<Vec<_>>(),
            vec![0, u32::MAX]
        );
        assert_eq!(ids.next_grab, 1);
    }

    #[test]
    fn equal_speed_circles_copy_the_minimum() {
        let circle = CensusCircle {
            no_spawn: 0.0,
            spawn: 60.0,
            cull: 75.0,
            shift: 0.0,
            speed: 0.0,
        };
        let out = evaluate_census_circle(12.0, circle, circle);
        assert_eq!(out.spawn, 60.0);
        assert_eq!(out.cull, 75.0);
    }

    #[test]
    fn census_circle_interpolates_with_the_fsel_clamp() {
        let low = CensusCircle {
            spawn: 10.0,
            speed: 0.0,
            ..CensusCircle::default()
        };
        let high = CensusCircle {
            spawn: 30.0,
            speed: 10.0,
            ..CensusCircle::default()
        };
        assert_eq!(evaluate_census_circle(5.0, low, high).spawn, 20.0);
        assert_eq!(evaluate_census_circle(50.0, low, high).spawn, 30.0);
        assert_eq!(evaluate_census_circle(-5.0, low, high).spawn, 10.0);
    }

    #[test]
    fn normalize_safe_keeps_the_default_below_the_bad_magnitude() {
        let default = [1.0, 0.0, 0.0, 0.0];
        assert_eq!(normalize_safe::<()>([0.0; 4], default).unwrap(), default);
        let unit = normalize_safe::<()>([0.0, 3.0, 4.0, 0.0], default).unwrap();
        assert!((unit[1] - 0.6).abs() < 1e-6 && (unit[2] - 0.8).abs() < 1e-6);
    }
}
