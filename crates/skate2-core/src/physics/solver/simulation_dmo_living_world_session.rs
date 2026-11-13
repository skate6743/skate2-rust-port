//! LivingWorld dynamic objects against the World's DMO pool.
//!
//! Bodies: `cLWDynamicObjectCollision::InitCollisionBodies` 0x82d3aa28,
//! `cLivingWorldCollisionBody::Create` 0x82e50b70 / `AddToWorld` 0x82e50cb0
//! (`Island::AddDMOBody` 0x827b6798) and `Island::RemoveDMOBody` 0x827b7530.
//! Queries: `Manager::DoesCapsuleAndAABBoxIntersectAnyDynamicObject`
//! 0x82725930, `DoesCapsuleAndAABBoxIntersectDynamicObject` 0x82722a40,
//! `CheckSpawnedDynamicObjectAgainstSkaters` 0x82725620 and
//! `DynamicPool::HorribleUglyVolumeVolumeTest` 0x827cb218, each through
//! `PhysicsUtility::VolumeVolumeListOverlapTest` 0x82e4bc00.

use super::{
    SimulationAssemblyConstraintSession,
    physical_island::vehicle::{LivingWorldBodyKind, VehicleIslandError},
};
use crate::{
    actor_construction::{
        ActorConstruction, ActorConstructionError, physical_mount::ActorPhysicalMountError,
    },
    debug_overlay::DebugOverlayError,
    dynamic_assembly_session::DynamicAssemblyBodyOnlyPartDetachError,
    dynamic_clustered_mesh_template::{
        DynamicClusteredMeshTemplateError, resolve_aggregate_target,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_grind_services::{
        dmo_locators::LocatorError,
        dmo_models::LwDynamicObjectModel,
        dmo_objects::{
            BODY_STATE_STATIC, DmoFrameServiceError, DmoObjectMan, DmoVolumeTest, DmoWorld,
            DynamicObject, SkaterTestCapsule, SkaterTestCvars, aabb_separated,
            build_skater_test_aabbox, build_skater_test_capsule, model_bounds,
        },
    },
    narrow_phase::{GpPrimitive, rw_overlap},
    physical_player_phys_out::construct::ConcretePhysOutError,
    physics::FilteredPhysicsState,
    reckoning_numeric::{ReckoningHostNumericError, host_dot3},
    serialised_assembly_bbox::HostSerialisedAssemblyBBoxNumeric,
    simulation_dmo_assembly_part_session::{
        SimulationDmoAssemblyPartOwner, SimulationDmoAssemblyPartOwnerError,
        dynamic_pool::{
            SimulationDmoAssemblyCandidate, SimulationDmoAssemblyHandle,
            SimulationDmoCandidateError, SimulationDmoPoolError, SimulationDmoPoolUpdateError,
        },
    },
    world::{CollisionAggregateRef, CollisionArena},
};
use skate2_formats::collision::{AssemblyPartMaterial, Volume, VolumeType};
use std::{convert::Infallible, fmt, sync::Arc};

/// `GetDynamicObjectsWithinRange` request from 0x8272599c (clamped to 50).
const CAPSULE_QUERY_ITEMS: u32 = 0x40;
/// `HorribleUglyVolumeVolumeTest` 0x827cb2cc: `cmplwi volumes, 0x40; bge`.
const VOLUME_TEST_ASSEMBLIES: usize = 0x40;
/// `GetSimulationCenterVelocity` 0x826f4ea8 `__real_38d1b717`.
const CENSUS_STILL_SPEED_SQUARED: f32 = f32::from_bits(0x38d1_b717);
/// `Actor+0x258` bit 29, the pending external reset `Actor::Teleport` sets.
const EXTERNAL_RESET_PENDING: u32 = 0x2000_0000;

/// The DMO pipeline cvars `Island::Step_UpdatePrePhysics2` 0x827b2904..2978
/// and `Step_Collision1` sample, captured for the frame whose
/// `World::BeginDMOUpdate` ran.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DmoFrameSettings {
    /// `kRunDmoGroundCollisionPipeline` and `kDrawDmoGroundContacts`.
    pub(crate) run_ground: bool,
    pub(crate) draw_ground_contacts: bool,
    /// `kRunSkaterDmoCollisionPipeline`.
    pub(crate) run_skater_dmo: bool,
    /// `kRunDmoDmoCollisionPipeline`.
    pub(crate) run_dmo_dmo: bool,
    pub(crate) run_dmo_vehicle: bool,
    pub(crate) run_skater_vehicle: bool,
    pub(crate) run_pedestrian_ground: bool,
    pub(crate) draw_pedestrian_ground_contacts: bool,
    pub(crate) run_dmo_pedestrian: bool,
    pub(crate) run_pedestrian_vehicle: bool,
    pub(crate) run_pedestrian_pedestrian: bool,
    pub(crate) run_skater_pedestrian: bool,
}

#[derive(Debug)]
pub(crate) enum DmoWorldError {
    PedestrianCull(String),
    MissingPool,
    MissingWorldQueries,
    DynamicQuerySync(crate::collision_query_batches::CollisionQueryBatchError),
    Island {
        body: SimulationDmoAssemblyHandle,
        source: VehicleIslandError,
    },
    MissingMember {
        entity_id: u32,
    },
    MissingPart,
    GrabPart(crate::dynamic_assembly_session::source_parts::DynamicAssemblySourcePartError),
    Overlay(DebugOverlayError),
    Locator(LocatorError<DmoFrameServiceError>),
    Aggregate(DynamicClusteredMeshTemplateError),
    /// `CreateGPInstance` of a volume the port cannot instance.
    Instance {
        entity_id: u32,
    },
    /// `findBestSepDirFuncs` pair without a ported separator.
    Separator {
        entity_id: u32,
    },
    PartOwner(SimulationDmoAssemblyPartOwnerError),
    Candidate(SimulationDmoCandidateError),
    Insert(SimulationDmoPoolError<Infallible>),
    Remove(SimulationDmoPoolError<Infallible>),
    Detach(DynamicAssemblyBodyOnlyPartDetachError),
    Actor(ActorConstructionError),
    Physical(ActorPhysicalMountError),
    PhysOutPoisoned,
    PhysOut(ConcretePhysOutError<Infallible>),
    Numeric(ReckoningHostNumericError),
    Cvar(InputCvarError),
    PoolUpdate(SimulationDmoPoolUpdateError<Infallible>),
}

impl fmt::Display for DmoWorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LivingWorld DMO world: {self:?}")
    }
}

impl std::error::Error for DmoWorldError {}

/// `VolumeVolumeListOverlapTest` 0x82e4bcc0..bd10 and
/// `InstanceSimpleMappedArrayForVolumeAssemblyListTest` 0x82e4a328: a
/// primitive is one `CreateGPInstance` at the Part transform; an aggregate is
/// instanced only when it is a `SimpleMappedArray` (0x80002), every child in
/// order (enabled or not), nested mapped arrays recursively with the same
/// transform. Other aggregates contribute no instance.
fn append_list_test_instances(
    arena: &CollisionArena,
    volume: &Volume,
    aggregate: Option<CollisionAggregateRef>,
    transform: [f32; 16],
    entity_id: u32,
    out: &mut Vec<GpPrimitive>,
) -> Result<(), DmoWorldError> {
    if volume.volume_type() != Some(VolumeType::Aggregate) {
        out.push(
            GpPrimitive::from_volume(volume, Some(transform))
                .ok_or(DmoWorldError::Instance { entity_id })?,
        );
        return Ok(());
    }
    let Some(CollisionAggregateRef::SimplePrimitiveSet(index)) = aggregate else {
        return Ok(());
    };
    let set = arena
        .simple_primitive_sets
        .get(index)
        .ok_or(DmoWorldError::Instance { entity_id })?;
    for child in &set.volumes {
        let nested = if child.volume_type() == Some(VolumeType::Aggregate) {
            Some(
                resolve_aggregate_target(arena, child.shape_words[0])
                    .map_err(DmoWorldError::Aggregate)?,
            )
        } else {
            None
        };
        append_list_test_instances(arena, child, nested, transform, entity_id, out)?;
    }
    Ok(())
}

/// LWModel vslot72 returns the HiLOD collision definition (+58), not the
/// bounding box volume at vslot40. Reset queries use its first volume.
pub(crate) fn reset_models_overlap(a:&LwDynamicObjectModel,at:[f32;16],b:&LwDynamicObjectModel,bt:[f32;16],padding:f32)->Result<bool,DmoWorldError> {
    // VolumeVolumeListOverlapTest82e4bc70: unpadded model bounds gate the padded shape test.
    if aabb_separated(model_bounds(a,&at),model_bounds(b,&bt)) {return Ok(false);}
    let aa=model_list_test_instances(a,at)?;
    let bb=model_list_test_instances(b,bt)?;
    for a in &aa {for b in &bb {
        if rw_overlap(a,b,padding).ok_or(DmoWorldError::Separator{entity_id:0})? {return Ok(true);}
    }}
    Ok(false)
}

fn model_list_test_instances(model:&LwDynamicObjectModel,transform:[f32;16])
    ->Result<Vec<GpPrimitive>,DmoWorldError>{
    let template=&model.world_collision;
    let part=template.parts().first().ok_or(DmoWorldError::MissingPart)?;
    let arena=&template.owner().static_plan().asset().collision;
    let root=arena.aggregate_roots.get(part.root_index).ok_or(DmoWorldError::MissingPart)?;
    let mut instances=Vec::new();
    append_list_test_instances(arena,&root.volume,root.aggregate,transform,0,&mut instances)?;
    Ok(instances)
}

/// BuildObjectLocatorResetListVolumeTestInfo82d38ba8 publishes the actual
/// model HiLOD volume with its initial locator transform and model AABB.
pub(crate) fn reset_model_overlaps_capsule(model:&LwDynamicObjectModel,
    transform:[f32;16],capsule:SkaterTestCapsule)->Result<bool,DmoWorldError>{
    if aabb_separated(capsule.bounds(),model_bounds(model,&transform)){return Ok(false);}
    let volume=capsule.capsule.volume();
    let probe=GpPrimitive::from_volume(&volume,Some(capsule.transform))
        .ok_or(DmoWorldError::Instance{entity_id:0})?;
    for instance in model_list_test_instances(model,transform)?{
        if rw_overlap(&probe,&instance,0.).ok_or(DmoWorldError::Separator{entity_id:0})?{
            return Ok(true);
        }
    }
    Ok(false)
}

/// `Manager::Update1` 0x82728258..82b0's census gate for the local skater:
/// `Actor::IsValid` 0x825ed890 (the filtered physics state is not Teleport
/// (5), and `GetSimulationFlags() & 1`, which `SkaterManager::SpawnPlayer`
/// 0x826006ac always passes as 1) and no pending external reset.
pub(crate) fn local_skater_census_gate(actor: &ActorConstruction) -> Result<bool, DmoWorldError> {
    let physical = actor
        .cached_physical_allocation()
        .map_err(DmoWorldError::Physical)?;
    let teleporting = physical
        .owner()
        .phys_out()
        .lock()
        .map_err(|_| DmoWorldError::PhysOutPoisoned)?
        .try_filtered_output()
        .map_err(DmoWorldError::PhysOut)?
        .filtered_physics_state
        == FilteredPhysicsState::Teleport;
    if teleporting {
        return Ok(false);
    }
    let flags = actor
        .try_input_generation_flags()
        .map_err(DmoWorldError::Actor)?;
    Ok(flags & EXTERNAL_RESET_PENDING == 0)
}

/// `Manager::GetSimulationCenterPosition` 0x826f4dd0 (`Actor::GetCentreOfMass`
/// 0x825eda80) and `GetSimulationCenterVelocity` 0x826f4e48 (`Actor::
/// GetVelocity` 0x825ed338, zeroed when `|v|^2 - 1e-4 <= 0`). The W lanes
/// reach no census consumer.
pub(crate) fn local_skater_census_centre(
    actor: &ActorConstruction,
) -> Result<([f32; 4], [f32; 4]), DmoWorldError> {
    let physical = actor
        .cached_physical_allocation()
        .map_err(DmoWorldError::Physical)?;
    let (centre, velocity) = physical
        .owner()
        .phys_out()
        .lock()
        .map_err(|_| DmoWorldError::PhysOutPoisoned)?
        .try_system_centre_and_damped_velocity()
        .map_err(DmoWorldError::PhysOut)?;
    let lanes = |vector: crate::skeleton_root_transforms::SkeletonRootVector| {
        [
            vector.xyz[0],
            vector.xyz[1],
            vector.xyz[2],
            vector.w.unwrap_or(0.0),
        ]
    };
    let velocity = lanes(velocity);
    // Raw826f4ef8 BLE: unordered falls through and retains the source vector.
    let stationary = host_dot3(velocity, velocity) - CENSUS_STILL_SPEED_SQUARED <= 0.0;
    Ok((lanes(centre), if stationary { [0.0; 4] } else { velocity }))
}

/// Actor.RequestHideSkateboard825f0da8 -> retained physical/controller path.
pub(crate) fn request_actor_hide_skateboard(world:&mut crate::physical_player_assembly::PhysicalAssemblyWorld,
    actor:&ActorConstruction,cvars:&InputCvarReferences)->Result<(),DmoWorldError>{
    let physical=actor.cached_physical_allocation().map_err(DmoWorldError::Physical)?;
    let owner=physical.owner();
    if !owner.phys_out().lock().map_err(|_|DmoWorldError::PhysOutPoisoned)?
        .try_state_output().map_err(DmoWorldError::PhysOut)?.is_offboard {
        return Err(DmoWorldError::PedestrianCull("Actor.RequestHideSkateboard requires offboard".into()));
    }
    let pool=world.skaters.as_ref().ok_or_else(||DmoWorldError::PedestrianCull("hide board skater pool absent".into()))?;
    let mut board=None;
    for handle in pool.handles(){
        let (kind,actual)=pool.try_physical_member_owner(&world.backend,handle)
            .map_err(|e|DmoWorldError::PedestrianCull(format!("hide board owner: {e:?}")))?;
        if kind==crate::dynamic_assembly_session::skater_pool::physical::SimulationPhysicalBodyKind::Board
            && Arc::ptr_eq(actual,owner){board=Some(handle.clone());break;}
    }
    let board=board.ok_or_else(||DmoWorldError::PedestrianCull("Actor retained board absent".into()))?;
    world.try_request_owner_hide_board(owner,&board,cvars).map_err(DmoWorldError::PedestrianCull)
}

/// `PhysOut_Skeleton` rows `CheckSpawnedDynamicObjectAgainstSkaters` reads
/// through `IActor::GetPhysOut`: +0x170 root matrix, +0x20 head, +0x90/+0xa0
/// feet. The W lanes reach no consumer.
fn skater_test_inputs(
    actor: &ActorConstruction,
) -> Result<([f32; 16], [f32; 4], [f32; 4]), DmoWorldError> {
    let physical = actor
        .cached_physical_allocation()
        .map_err(DmoWorldError::Physical)?;
    let output = physical
        .owner()
        .phys_out()
        .lock()
        .map_err(|_| DmoWorldError::PhysOutPoisoned)?;
    let skeleton = output
        .try_skeleton_output()
        .map_err(DmoWorldError::PhysOut)?;
    let lanes = |xyz: [f32; 3]| [xyz[0], xyz[1], xyz[2], 0.0];
    let matrix = skeleton.skeleton_root_matrix.xyz_scratch();
    let head = lanes(skeleton.skeleton_head_pos.xyz);
    let left = lanes(skeleton.skeleton_foot_left_pos.xyz);
    let right = lanes(skeleton.skeleton_foot_right_pos.xyz);
    let root = lanes(skeleton.skeleton_root_matrix.columns()[3].xyz);
    // 0x827257d8..589c: the foot midpoint with Y the lowest of right, left
    // and the root (`vcmpgtfp.` of splats; unordered keeps the later).
    let middle: [f32; 4] = std::array::from_fn(|lane| (left[lane] + right[lane]) * 0.5);
    let lower = if right[1] > left[1] { left } else { right };
    let lowest = if lower[1] > root[1] { root } else { lower };
    Ok((matrix, head, [middle[0], lowest[1], middle[2], middle[3]]))
}

impl<C> SimulationAssemblyConstraintSession<AssemblyPartMaterial, C> {
    /// `InitCollisionBodies` 0x82d3aa28 (the `Create_2` branch every ported
    /// model takes), `cLivingWorldCollisionBody::Create` 0x82e50b70 and
    /// `AddToWorld` 0x82e50cb0: `Body::SetPosition` per Part, then
    /// `Island::AddDMOBody` inserts into the DMO pool with the body state.
    /// Every body of one model shares its bound Part owner.
    pub(crate) fn try_add_living_world_body(
        &mut self,
        model: &Arc<LwDynamicObjectModel>,
        entity_id: u32,
        group: u32,
        part_transform: &[f32; 16],
        state: u32,
    ) -> Result<SimulationDmoAssemblyHandle, DmoWorldError> {
        let owner = self.try_dmo_part_owner(model)?;
        let candidate = SimulationDmoAssemblyCandidate::try_from_living_world(
            &owner,
            entity_id,
            *part_transform,
            group,
            model.culling_table.clone(),
        )
        .map_err(DmoWorldError::Candidate)?;
        self.try_admit_dmo_body(candidate, state)
    }

    /// `Dmo::SimManager::SpawnStatic` (`0x825eb0a0`)'s `Dmo::Body` of the model at an online
    /// game's locator, added as a `STATIC_BODY` the way a living world body is: the DMO pool's
    /// insert, the Island's body ID and list, then the Part spies of the object `entity_id`
    /// holding it.
    pub(crate) fn try_add_static_dmo_body(
        &mut self,
        model: &Arc<LwDynamicObjectModel>,
        entity_id: u32,
        transform: &[f32; 16],
    ) -> Result<SimulationDmoAssemblyHandle, DmoWorldError> {
        let owner = self.try_dmo_part_owner(model)?;
        let candidate = SimulationDmoAssemblyCandidate::try_from_static_locator(
            &owner,
            entity_id,
            *transform,
            model.culling_table.clone(),
        )
        .map_err(DmoWorldError::Candidate)?;
        self.try_admit_dmo_body(candidate, BODY_STATE_STATIC)
    }

    /// The model's Part owner, bound to the backend by its first body.
    fn try_dmo_part_owner(
        &mut self,
        model: &Arc<LwDynamicObjectModel>,
    ) -> Result<Arc<SimulationDmoAssemblyPartOwner>, DmoWorldError> {
        let pool = self.dmos.as_mut().ok_or(DmoWorldError::MissingPool)?;
        if let Some(owner) = pool.living_world_owner(model.model_id) {
            return Ok(Arc::clone(owner));
        }
        let owner = SimulationDmoAssemblyPartOwner::try_new_living_world(
            Arc::clone(&model.asset),
            model.assembly_object_id,
            Arc::clone(&model.world_collision),
            &self.backend,
        )
        .map_err(DmoWorldError::PartOwner)?;
        pool.remember_living_world_owner(model.model_id, Arc::clone(&owner));
        Ok(owner)
    }

    /// `Island::AddDMOBody` 0x827b6798 for a positioned candidate: the pool's insert, then the
    /// body ID and list, then `AddToWorld`'s Part spies.
    fn try_admit_dmo_body(
        &mut self,
        candidate: SimulationDmoAssemblyCandidate,
        state: u32,
    ) -> Result<SimulationDmoAssemblyHandle, DmoWorldError> {
        match self.try_insert_dmo(candidate, state, &mut HostSerialisedAssemblyBBoxNumeric) {
            Ok(handle) => {
                // AddDMOBody827b6798 publishes Body ID/list AFTER pool Insert;
                // AddToWorld82e50cb0 then registers every Part spy. Preserve a
                // partially admitted handle on failure, never detach silently.
                self.try_add_living_world_body_membership(LivingWorldBodyKind::Dmo, &handle, state)
                    .map_err(|source| DmoWorldError::Island {
                        body: handle.clone(),
                        source,
                    })?;
                self.try_register_living_world_body_targets(LivingWorldBodyKind::Dmo, &handle)
                    .map_err(|source| DmoWorldError::Island {
                        body: handle.clone(),
                        source,
                    })?;
                Ok(handle)
            }
            Err(failure) => {
                let mut failure = *failure;
                if failure.candidate.is_attached() {
                    failure
                        .candidate
                        .try_detach(&mut self.backend)
                        .map_err(DmoWorldError::Detach)?;
                }
                Err(DmoWorldError::Insert(failure.error))
            }
        }
    }

    /// `Island::RemoveDMOBody` 0x827b7530 -> `CleanUpDMOBody` 0x827b6b90 ->
    /// `DynamicPool::Remove` 0x827c92b0 (base Remove, then every mesh the
    /// assembly owns); the Parts detach with their bodies.
    pub(crate) fn try_remove_living_world_body(
        &mut self,
        body: &SimulationDmoAssemblyHandle,
    ) -> Result<(), DmoWorldError> {
        self.try_remove_living_world_collision(LivingWorldBodyKind::Dmo, body)
            .map_err(|source| DmoWorldError::Island {
                body: body.clone(),
                source,
            })?;
        Ok(())
    }

    /// `World::BeginDMOUpdate` 0x827adec0 -> `Island::Step_UpdatePrePhysics1`
    /// 0x827b36e8 -> `DynamicPool::BeginUpdate` 0x827c93f0: the mesh
    /// collection rebuilt from every member's Part volumes and transforms.
    pub(crate) fn try_begin_dmo_update(
        &mut self,
        ignore_volume_enable: bool,
    ) -> Result<(), DmoWorldError> {
        self.world_line_queries
            .as_ref()
            .ok_or(DmoWorldError::MissingWorldQueries)?
            .try_sync_dmo_jobs()
            .map_err(DmoWorldError::DynamicQuerySync)?;
        // A full mesh collection makes that pool's BeginUpdate return before the base
        // AssemblyPool::BeginUpdate (its EndUpdate then publishes the retained records);
        // the next pool's BeginUpdate still runs.
        let pool = self.dmos.as_mut().ok_or(DmoWorldError::MissingPool)?;
        pool.try_rebuild_meshes::<Infallible>(&self.backend, ignore_volume_enable)
            .map_err(DmoWorldError::PoolUpdate)?;
        // Step_UpdatePrePhysics1: DMO, then vehicle, then pedestrian.
        // DynamicPool.BeginUpdate synchronizes all four query job families
        // before rebuilding meshes. It does not release their batch leases.
        let queries = self
            .world_line_queries
            .as_mut()
            .ok_or(DmoWorldError::MissingWorldQueries)?;
        queries
            .try_sync_vehicle_jobs()
            .map_err(DmoWorldError::DynamicQuerySync)?;
        queries
            .vehicle
            .try_rebuild_meshes::<Infallible>(&self.backend, ignore_volume_enable)
            .map_err(DmoWorldError::PoolUpdate)?;
        Ok(())
    }

    /// Arms the next physics step's `EndDMOUpdate` (serialized pool update
    /// and the DMO pipelines) with this frame's pipeline cvars.
    pub(crate) fn schedule_dmo_frame(&mut self, settings: DmoFrameSettings) {
        self.dmo_frame = Some(settings);
    }

    /// Hand-prop release calls the original Body.SetVelocity for Part0.
    /// Reuses the same DMO pool and activation/velocity backend primitives.
    pub(crate) fn try_set_dmo_part_velocity(
        &mut self, body: &SimulationDmoAssemblyHandle, part: usize, velocity: [f32; 3],
    ) -> Result<(), String> {
        let candidate = self.dmos.as_ref().ok_or("hand prop DMO pool absent")?
            .island_body(body).ok_or("hand prop DMO body absent")?;
        candidate.parts.try_set_body_part_velocity(&mut self.backend, part, velocity)
            .map_err(|e| format!("hand prop Body.SetVelocity: {e:?}"))
    }

    /// `DynamicObject::GetPartTransform(0)` 0x82d2a9a0: `Part::GetTransform`.
    pub(crate) fn try_living_world_part_transform(
        &self,
        body: &SimulationDmoAssemblyHandle,
    ) -> Result<[f32; 16], DmoWorldError> {
        let pool = self.dmos.as_ref().ok_or(DmoWorldError::MissingPool)?;
        pool.world_part_transforms(&self.backend, body)
            .ok_or(DmoWorldError::MissingPart)?
            .first()
            .copied()
            .ok_or(DmoWorldError::MissingPart)
    }

    pub(crate) fn try_living_world_grab_part(
        &self,
        body: &SimulationDmoAssemblyHandle,
    ) -> Result<
        Option<crate::dynamic_assembly_session::source_parts::SourcePartGrabSnapshot>,
        DmoWorldError,
    > {
        self.dmos
            .as_ref()
            .ok_or(DmoWorldError::MissingPool)?
            .try_grab_part_snapshot(&self.backend, body)
            .map_err(DmoWorldError::GrabPart)
    }

    /// Vehicle GetGrabSpline82d1ed40 uses its actual attached skater assembly.
    /// The same Part reader supports both source and constructed definitions.
    pub(crate) fn try_vehicle_grab_part(
        &self,
        body: &SimulationDmoAssemblyHandle,
    ) -> Result<
        Option<crate::dynamic_assembly_session::source_parts::SourcePartGrabSnapshot>,
        DmoWorldError,
    > {
        self.world_line_queries
            .as_ref()
            .ok_or(DmoWorldError::MissingWorldQueries)?
            .vehicle
            .try_grab_part_snapshot(&self.backend, body)
            .map_err(DmoWorldError::GrabPart)
    }

    /// `LivingWorldView::VolumeTestAgainstDynamicObjects` 0x827cb4b8 ->
    /// `HorribleUglyVolumeVolumeTest` 0x827cb218: Part 0 of each of the first
    /// 64 pool members with the member's pool AABB, then every overlapping
    /// member in pool order. The fixed-vector capacity argument is unread.
    pub(crate) fn try_volume_test_dynamic_objects(
        &self,
        test: &DmoVolumeTest<'_>,
    ) -> Result<Vec<SimulationDmoAssemblyHandle>, DmoWorldError> {
        let probe = GpPrimitive::from_volume(test.volume, Some(*test.transform))
            .ok_or(DmoWorldError::Instance { entity_id: 0 })?;
        self.try_instances_test_dynamic_objects(&[probe],test.bounds,test.padding)
    }

    /// BuildRecursiveObjectResetList82d37c70 supplies model vslot72's first
    /// HiLOD volume, which can be a mapped array rather than a primitive.
    pub(crate) fn try_model_volume_test_dynamic_objects(&self,model:&LwDynamicObjectModel,
        transform:[f32;16])->Result<Vec<SimulationDmoAssemblyHandle>,DmoWorldError>{
        let probes=model_list_test_instances(model,transform)?;
        self.try_instances_test_dynamic_objects(&probes,model_bounds(model,&transform),0.)
    }

    fn try_instances_test_dynamic_objects(&self,probes:&[GpPrimitive],
        bounds:([f32;3],[f32;3]),padding:f32)->Result<Vec<SimulationDmoAssemblyHandle>,DmoWorldError>{
        let pool = self.dmos.as_ref().ok_or(DmoWorldError::MissingPool)?;
        let mut found = Vec::new();
        let mut instances = Vec::new();
        for handle in pool.handles().take(VOLUME_TEST_ASSEMBLIES) {
            let candidate = pool
                .candidate(handle)
                .ok_or(DmoWorldError::MissingMember { entity_id: 0 })?;
            let publication = pool
                .publication(handle)
                .ok_or(DmoWorldError::MissingMember { entity_id: 0 })?;
            let aabb = publication.header().aabb;
            let (minimum, maximum) = (aabb.min_f32(), aabb.max_f32());
            let member = (
                [minimum[0], minimum[1], minimum[2]],
                [maximum[0], maximum[1], maximum[2]],
            );
            if aabb_separated(member, bounds) {
                continue;
            }
            let transforms = pool
                .world_part_transforms(&self.backend, handle)
                .ok_or(DmoWorldError::MissingPart)?;
            let template = candidate.part_owner().template();
            let (Some(part), Some(transform)) = (template.parts().first(), transforms.first())
            else {
                return Err(DmoWorldError::MissingPart);
            };
            let arena = &template.owner().static_plan().asset().collision;
            let root = &arena.aggregate_roots[part.root_index];
            instances.clear();
            append_list_test_instances(
                arena,
                &root.volume,
                root.aggregate,
                *transform,
                0,
                &mut instances,
            )?;
            let mut overlaps=false;
            for probe in probes {
                for instance in &instances {
                    if rw_overlap(probe, instance, padding)
                    .ok_or(DmoWorldError::Separator { entity_id: 0 })?
                    {
                        overlaps=true;
                        break;
                    }
                }
                if overlaps {break;}
            }
            if overlaps {found.push(handle.clone());}
        }
        Ok(found)
    }

    /// `DoesCapsuleAndAABBoxIntersectAnyDynamicObject` 0x82725930 after the
    /// capsule and AABB are built: `GetDynamicObjectsWithinRange` 0x82d3be78 at
    /// the capsule transform's (x, z) (`VectorPermute<0,6,0,0>`) within
    /// `kSpawnTestCapsuleMinDistance`, 64 IDs clamped to 50, then each live
    /// entity in `FindNear` order until one intersects.
    pub(crate) fn try_capsule_intersects_any_dynamic_object(
        &self,
        objects: &DmoObjectMan,
        capsule: &SkaterTestCapsule,
        bounds: ([f32; 3], [f32; 3]),
        min_distance: f32,
        show: bool,
    ) -> Result<bool, DmoWorldError> {
        let point = [capsule.transform[12], capsule.transform[14]];
        let found = objects
            .try_objects_within_range(point, min_distance, CAPSULE_QUERY_ITEMS)
            .map_err(DmoWorldError::Locator)?;
        for (entity_id, _) in found {
            let Some(object) = objects.object(entity_id) else {
                continue;
            };
            // The Manager's own objects; SpawnStatic's bodies are not among them.
            if object.online_static {
                continue;
            }
            if self.try_capsule_intersects_dynamic_object(
                objects,
                object,
                capsule,
                bounds,
                min_distance,
                show,
            )? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `DoesCapsuleAndAABBoxIntersectDynamicObject` 0x82722a40. The capsule
    /// argument is never read: `VolumeVolumeListOverlapTest` receives the
    /// Manager's `mIntersectionCapsuleVolume` (0x82722dc4) at the caller's
    /// capsule transform and AABB with zero padding.
    pub(crate) fn try_capsule_intersects_dynamic_object(
        &self,
        objects: &DmoObjectMan,
        object: &DynamicObject,
        capsule: &SkaterTestCapsule,
        bounds: ([f32; 3], [f32; 3]),
        min_distance: f32,
        show: bool,
    ) -> Result<bool, DmoWorldError> {
        let entity_id = object.entity_id;
        // 0x82722a80..2ae0: GetCenterPosition 0x82d361c8 (the bbox centre
        // through GetBBoxTransform) against the capsule translation.
        let centre = object.model.bbox_transform(&object.transform);
        let delta: [f32; 4] =
            std::array::from_fn(|lane| capsule.transform[12 + lane] - centre[12 + lane]);
        if host_dot3(delta, delta) > min_distance * min_distance {
            return Ok(false);
        }
        // 0x82722af4..2b30: kShowSpawnTestCapsule asserts the DebugOverlay.
        if show {
            self.debug_overlay
                .require()
                .map_err(DmoWorldError::Overlay)?;
        }
        // 0x82722c10..2c94: the model AABB through GetTransform (core
        // mTransform) against the skater AABB.
        let model = model_bounds(&object.model, &object.transform);
        if aabb_separated(model, bounds) {
            return Ok(false);
        }
        let pool = self.dmos.as_ref().ok_or(DmoWorldError::MissingPool)?;
        let candidate = pool
            .candidate(&object.body)
            .ok_or(DmoWorldError::MissingMember { entity_id })?;
        let transforms = pool
            .world_part_transforms(&self.backend, &object.body)
            .ok_or(DmoWorldError::MissingMember { entity_id })?;
        let template = candidate.part_owner().template();
        let arena = &template.owner().static_plan().asset().collision;
        let test_volume = objects.intersection_capsule().volume();
        let test = GpPrimitive::from_volume(&test_volume, Some(capsule.transform))
            .ok_or(DmoWorldError::Instance { entity_id })?;
        let capsule_bounds = capsule.bounds();
        let mut instances = Vec::new();
        for (part, transform) in template.parts().iter().zip(transforms) {
            // Every Part carries the object's transformed model AABB.
            if aabb_separated(model, capsule_bounds) {
                continue;
            }
            let root = &arena.aggregate_roots[part.root_index];
            instances.clear();
            append_list_test_instances(
                arena,
                &root.volume,
                root.aggregate,
                transform,
                entity_id,
                &mut instances,
            )?;
            for instance in &instances {
                if rw_overlap(&test, instance, 0.0).ok_or(DmoWorldError::Separator { entity_id })? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

/// The World half of one LivingWorld DMO pass: the physics World and the
/// SkaterManager's skaters (`Simulation::mWorkspace.mSkaterMan`) in order.
pub(crate) struct LivingWorldDmoWorld<'a, C> {
    pub(crate) prop_recipes:Option<crate::selected_living_world_recipes::props::PropRecipeFrame<'a>>,
    pub(crate) census_targets:&'a mut crate::living_world_grind_services::attrib_cache::CensusTargetCache,
    pub(crate) frame_owners: crate::pedestrian_graph::backend::frame::FrameOwners<'a>,
    pub(crate) random: &'a mut crate::native_skater_random::NativeSkaterRandom,
    pub(crate) actors: &'a [Arc<ActorConstruction>],
    pub(crate) painter: Option<&'a crate::world_painter_layer::WorldPainterLayerMan>,
    pub(crate) avoid: Option<&'a crate::pedestrian_navigation::wander::ResolvedAvoidVolumes>,
    pub(crate) vault: &'a Arc<crate::attribute_runtime::AttributeRuntimeVault>,
    pub(crate) world: &'a mut SimulationAssemblyConstraintSession<AssemblyPartMaterial, C>,
    pub(crate) skaters: &'a [&'a ActorConstruction],
    pub(crate) cvars: &'a InputCvarReferences,
}

impl DmoWorld for LivingWorldDmoWorld<'_, crate::skateboard_hook::SkateboardHookDefinition> {
    type Error = DmoWorldError;
    fn try_initialize_dmo_props(&mut self,object:&mut crate::living_world_grind_services::dmo_objects::DynamicObject,
        entity_ids:&mut crate::world::LivingWorldEntityIdAllocator,census_enable:u8,
        models:&crate::living_world_grind_services::dmo_models::DmoModelMan)->Result<(),Self::Error>{
        if object.model.waypoint_groups.is_empty(){object.contained_props_initialized=true;return Ok(());}
        let recipes=self.prop_recipes.as_mut().ok_or_else(||DmoWorldError::PedestrianCull("DMO child spawn recipe context absent".into()))?;
        let mut owners=crate::selected_living_world_recipes::props::PropSpawnOwners{
            props:self.frame_owners.props,entity_ids,census_targets:self.census_targets,models,census_enable};
        recipes.initialize_dmo_props(object,&mut owners,self.frame_owners.pedestrians,self.cvars)
            .map_err(DmoWorldError::PedestrianCull)
    }
    fn try_update_dmo_contained_props(&mut self,children:&[u32],transform:[f32;16])->Result<(),Self::Error>{
        self.frame_owners.props.update_dmo_contained_prop_transforms(children,
            std::array::from_fn(|axis|std::array::from_fn(|lane|transform[axis*4+lane])))
            .map_err(DmoWorldError::PedestrianCull)
    }
    /// DynamicObject.UpdateUprighting82d2cfa0, ApplyTorque82d2a588.
    fn try_update_uprighting(&mut self,object:&mut crate::living_world_grind_services::dmo_objects::DynamicObject)->Result<(),Self::Error> {
        use crate::reckoning_numeric::{try_normalize_safe,cross_full4,try_length,HostReckoningNumeric};
        if object.flags&0x40==0 {return Ok(());}
        let Some(part)=self.world.try_living_world_grab_part(&object.body)? else{return Ok(());};
        let Some(body)=part.body else{return Ok(());};
        if object.try_is_upright(self.cvars.try_dynamic_object_upright_angle().map_err(DmoWorldError::Cvar)?).map_err(DmoWorldError::Numeric)? {
            object.flags&=!0x40; return Ok(());
        }
        let inverse=part.inertia.inverse_tensor;
        let up=[object.transform[4],object.transform[5],object.transform[6],object.transform[7]];
        let mut direction=try_normalize_safe(cross_full4(up,[0.,1.,0.,0.]),[0.;4],&mut HostReckoningNumeric).map_err(DmoWorldError::Numeric)?;
        let scale=if direction[..3].iter().all(|x|x.abs()<=f32::from_bits(0x34000000)) {
            let axis=if inverse[0]>inverse[2] {0} else {2};
            let local=if axis==0 {[1.,0.,0.,0.]} else {[0.,0.,1.,0.]};
            direction=std::array::from_fn(|i|object.transform[8+i].mul_add(local[2],object.transform[4+i].mul_add(local[1],object.transform[i]*local[0])));
            1./inverse[axis]
        } else {1./try_length([inverse[0],inverse[1],inverse[2],part.inverse_tensor_w],&mut HostReckoningNumeric).map_err(DmoWorldError::Numeric)?};
        let acceleration=object.model.characteristics.try_cached_layout_field(0x54,4)
            .map_err(|e|DmoWorldError::PedestrianCull(format!("uprighting characteristic: {e:?}")))?
            .map_or(0.,|data|f32::from_be_bytes(data.bytes().try_into().expect("uprighting scalar")));
        let torque=direction.map(|x|x*(scale*acceleration));
        self.world.backend.rigid_bodies.try_apply_grab_world_torque(body,crate::skeleton_root_transforms::SkeletonRootVector::known(torque),0.)
            .map_err(|e|DmoWorldError::PedestrianCull(format!("uprighting torque: {e:?}")))
    }

    fn try_update_reset_capsules(&mut self,objects:&mut DmoObjectMan)->Result<(),Self::Error>{
        // Selected session supplies the actual local Actor; native no-local
        // branch leaves its preceding retained volumes unchanged.
        let Some(actor)=self.skaters.first().copied() else{return Ok(());};
        let physical=actor.cached_physical_allocation().map_err(DmoWorldError::Physical)?;
        let output=physical.owner().phys_out().lock().map_err(|_|DmoWorldError::PhysOutPoisoned)?;
        let skeleton=output.try_skeleton_output().map_err(DmoWorldError::PhysOut)?;
        let left=skeleton.skeleton_foot_left_pos.xyz;
        let right=skeleton.skeleton_foot_right_pos.xyz;
        let feet:[f32;4]=std::array::from_fn(|i|if i<3{(left[i]+right[i])*0.5}else{0.});
        let delta=std::array::from_fn(|i|if i<3{skeleton.skeleton_head_pos.xyz[i]-feet[i]}else{0.});
        let height=crate::living_world_grind_services::dmo_objects::host_length(delta).map_err(DmoWorldError::Numeric)?;
        let [radius,board_radius,board_height]=self.cvars.try_dmo_reset_capsule_tuning().map_err(DmoWorldError::Cvar)?;
        let center:[f32;4]=std::array::from_fn(|i|[0.0f32,0.5,0.,0.][i].mul_add(height,feet[i]));
        let transform=[1.,0.,0.,0., -0.,-0.,-1.,-0., 0.,1.,0.,0., center[0],center[1],center[2],center[3]];
        let board=output.try_camera_board_output().map_err(DmoWorldError::PhysOut)?.effective_transform.xyz_scratch();
        let capsule=|radius,half_height,transform|->Result<SkaterTestCapsule,DmoWorldError>{
            let shape=crate::living_world_grind_services::dmo_objects::IntersectionCapsule{radius,half_height};
            let bounds=crate::serialised_assembly_bbox::try_get_primitive_bbox(&shape.volume(),Some(&transform),1,
                &mut HostSerialisedAssemblyBBoxNumeric).map_err(|e|DmoWorldError::PedestrianCull(format!("reset capsule bounds: {e:?}")))?
                .ok_or_else(||DmoWorldError::PedestrianCull("reset capsule primitive absent".into()))?;
            Ok(SkaterTestCapsule{capsule:shape,transform,minimum:bounds.min_f32(),maximum:bounds.max_f32()})
        };
        objects.reset_capsules=Some([capsule(radius,height,transform)?,capsule(board_radius,board_height,board)?]);
        Ok(())
    }
    fn try_before_dmo_cull(&mut self,objects:&mut DmoObjectMan,
        locators:&mut crate::living_world_grind_services::dmo_locators::DynamicObjectLocatorMan,
        object:u32)->Result<(),Self::Error>{
        let children=objects.object(object).ok_or_else(||DmoWorldError::PedestrianCull("DMO cull entity absent".into()))?.contained_props.clone();
        if children.is_empty(){return Ok(());}
        let painter=self.painter.ok_or_else(||DmoWorldError::PedestrianCull("DMO child cull painter absent".into()))?;
        let avoid=self.avoid.ok_or_else(||DmoWorldError::PedestrianCull("DMO child cull avoid volumes absent".into()))?;
        self.frame_owners.before_dmo_cull(self.world,objects,locators,object,self.cvars,
            painter,self.vault,self.random,self.actors,avoid).map_err(DmoWorldError::PedestrianCull)
    }
    fn try_notify_dmo_movement(&mut self,monitor:u32,object:u32,delta:[f32;3])->Result<(),Self::Error>{
        if let Some(entity)=self.frame_owners.pedestrians.get_mut(monitor){
            entity.monitored_object_moved(object,delta,self.cvars).map_err(DmoWorldError::PedestrianCull)?;
        }
        Ok(())
    }
    fn try_notify_dmo_monitor(&mut self,objects:&mut DmoObjectMan,monitor:u32,object:u32)->Result<(),Self::Error>{
        if let Some(entity)=self.frame_owners.pedestrians.get_mut(monitor){
            entity.monitored_object_being_culled(object,self.world,objects,self.cvars).map_err(DmoWorldError::PedestrianCull)?;
        }
        Ok(())
    }

    fn try_add_body(
        &mut self,
        model: &Arc<LwDynamicObjectModel>,
        entity_id: u32,
        group: u32,
        part_transform: &[f32; 16],
        state: u32,
    ) -> Result<SimulationDmoAssemblyHandle, Self::Error> {
        self.world
            .try_add_living_world_body(model, entity_id, group, part_transform, state)
    }

    fn try_add_static_body(
        &mut self,
        model: &Arc<LwDynamicObjectModel>,
        entity_id: u32,
        transform: &[f32; 16],
    ) -> Result<SimulationDmoAssemblyHandle, Self::Error> {
        self.world.try_add_static_dmo_body(model, entity_id, transform)
    }

    fn try_remove_body(&mut self, body: &SimulationDmoAssemblyHandle) -> Result<(), Self::Error> {
        self.world.try_remove_living_world_body(body)
    }

    fn try_part_transform(
        &mut self,
        body: &SimulationDmoAssemblyHandle,
    ) -> Result<[f32; 16], Self::Error> {
        self.world.try_living_world_part_transform(body)
    }

    fn try_volume_test(
        &mut self,
        test: DmoVolumeTest<'_>,
    ) -> Result<Vec<SimulationDmoAssemblyHandle>, Self::Error> {
        self.world.try_volume_test_dynamic_objects(&test)
    }

    /// `CheckSpawnedDynamicObjectAgainstSkaters` 0x82725620: for every skater,
    /// `BuildSkaterTestCapsule` re-initializes the Manager capsule from its
    /// PhysOut skeleton, `BuildSkaterTestAABBox` bounds it, and an intersecting
    /// new object sets the skater's `mRequestCollideWithWorldOnly`.
    fn try_check_spawned_against_skaters(
        &mut self,
        objects: &mut DmoObjectMan,
        entity_id: u32,
    ) -> Result<(), Self::Error> {
        for &actor in self.skaters {
            let (matrix, head, feet) = skater_test_inputs(actor)?;
            let test_cvars = SkaterTestCvars {
                width: self
                    .cvars
                    .try_spawn_test_capsule_width()
                    .map_err(DmoWorldError::Cvar)?,
                height: self
                    .cvars
                    .try_spawn_test_capsule_height()
                    .map_err(DmoWorldError::Cvar)?,
                feet_padding: self
                    .cvars
                    .try_spawn_test_capsule_feet_padding()
                    .map_err(DmoWorldError::Cvar)?,
                head_offset: self
                    .cvars
                    .try_spawn_test_actor_aabb_head_offset()
                    .map_err(DmoWorldError::Cvar)?,
            };
            let show = self
                .cvars
                .try_show_spawn_test_capsule()
                .map_err(DmoWorldError::Cvar)?;
            let capsule = build_skater_test_capsule(&matrix, head, feet, &test_cvars)
                .map_err(DmoWorldError::Numeric)?;
            objects.set_intersection_capsule(capsule.capsule);
            if show {
                self.world
                    .debug_overlay
                    .require()
                    .map_err(DmoWorldError::Overlay)?;
            }
            let bounds = build_skater_test_aabbox(head, feet, &test_cvars);
            if show {
                self.world
                    .debug_overlay
                    .require()
                    .map_err(DmoWorldError::Overlay)?;
            }
            let min_distance = self
                .cvars
                .try_spawn_test_capsule_min_distance()
                .map_err(DmoWorldError::Cvar)?;
            let Some(object) = objects.object(entity_id) else {
                continue;
            };
            if self.world.try_capsule_intersects_dynamic_object(
                objects,
                object,
                &capsule,
                bounds,
                min_distance,
                show,
            )? {
                actor
                    .try_request_collide_with_world_only()
                    .map_err(DmoWorldError::Actor)?;
            }
        }
        Ok(())
    }
}
