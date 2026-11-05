//! Island.AddVehicleBody827b6840 and LWCollisionBody.AddToWorld82e50cb0
//! registration suffix. Uses the existing Island ID allocator, ordered lists
//! and actual spy-target map. Vehicles stay in DynamicPool; pedestrians use
//! their separate base AssemblyPool (AddPedestrianBody827b68e8).
//! Contact lists retain these typed owners; live spawn scheduling is separate.
use super::super::world_line_query::SimulationWorldLineQueries;
use super::*;
use crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoPoolError;
use crate::simulation_dmo_assembly_part_session::dynamic_pool::{
    SimulationDmoAssemblyCandidate, SimulationDmoAssemblyPool,
};
use crate::simulation_dmo_assembly_part_session::dynamic_pool::character::{
    DynamicIslandBody, pedestrian_pool::PedestrianAssemblyPool,
};
use crate::{
    dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError,
    simulation_dmo_assembly_part_session::dynamic_pool::character::{
        CharacterAssemblyCandidate, CharacterInsertFailure,
    },
};
use std::convert::Infallible;

#[derive(Debug)]
pub(crate) enum VehiclePlacementError {
    MissingPool,
    PartTransforms { parts: usize, transforms: usize },
    Part(DynamicAssemblyPartTransformError),
}
pub(crate) enum VehicleAdmissionFailure {
    Placement {
        error: VehiclePlacementError,
        candidate: CharacterAssemblyCandidate,
    },
    Insert(Box<CharacterInsertFailure>),
    /// Pool insertion completed. Preserve the actual handle and partial Island
    /// registration for recovery; dropping an error does not detach native state.
    Island {
        error: VehicleIslandError,
        handle: SimulationDmoAssemblyHandle,
    },
}
impl fmt::Debug for VehicleAdmissionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Placement { error, .. } => f.debug_tuple("Placement").field(error).finish(),
            Self::Insert(error) => f.debug_tuple("Insert").field(error).finish(),
            Self::Island { error, handle } => f
                .debug_struct("Island")
                .field("error", error)
                .field("handle", handle)
                .finish(),
        }
    }
}

/// AddDMOBody827b6798, AddVehicleBody827b6840 and AddPedestrianBody827b68e8 use the
/// SAME Island Body-ID allocator, lists and spy map. Only pool selection differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LivingWorldBodyKind {
    Dmo,
    Vehicle,
    Pedestrian,
}
impl LivingWorldBodyKind {
    pub(crate) fn body(self, handle: &SimulationDmoAssemblyHandle) -> SimulationIslandBodyHandle {
        match self {
            Self::Dmo => SimulationIslandBodyHandle::Dmo(handle.clone()),
            Self::Vehicle => SimulationIslandBodyHandle::Vehicle(handle.clone()),
            Self::Pedestrian => SimulationIslandBodyHandle::Pedestrian(handle.clone()),
        }
    }
}
pub(crate) struct LivingWorldIslandMember {
    pub(super) kind: LivingWorldBodyKind,
    pub(super) handle: SimulationDmoAssemblyHandle,
    pub(super) entity_id: u32,
    pub(super) body_id: u32,
    pub(super) requested_state: u32,
    pub(super) list_published: bool,
    pub(super) registered_parts: usize,
    pub(super) registration_complete: bool,
}
#[derive(Debug)]
pub(crate) enum VehicleIslandError {
    MissingPool,
    ForeignManager,
    MissingMember,
    MissingEntityOwner,
    AlreadyMember,
    StateMismatch,
    WrongCollisionGroup(u32),
    IncompleteMembership,
    Allocation,
    Query(crate::collision_query_batches::CollisionQueryBatchError),
    Pool(SimulationDmoPoolError<Infallible>),
    Spy(DynamicAssemblyPartContactSpyError),
    TargetMismatch(usize),
}
pub(super) enum LivingWorldPool<'a> {
    Dynamic(&'a SimulationDmoAssemblyPool),
    Pedestrian(&'a PedestrianAssemblyPool),
}
impl<'a> LivingWorldPool<'a> {
    fn matches_authorities(&self, manager: &crate::simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
        backend: &crate::simulation_rigid_body_pool::SimulationRigidBodyPoolAuthority) -> bool {
        match self { Self::Dynamic(p) => p.matches_authorities(manager, backend),
            Self::Pedestrian(p) => p.matches_authorities(manager, backend) }
    }
    pub(super) fn island_body(&self, handle: &SimulationDmoAssemblyHandle) -> Option<DynamicIslandBody<'a>> {
        match self { Self::Dynamic(p) => p.island_body(handle), Self::Pedestrian(p) => p.island_body(handle) }
    }
}
enum LivingWorldPoolMut<'a> {
    Dynamic(&'a mut SimulationDmoAssemblyPool),
    Pedestrian(&'a mut PedestrianAssemblyPool),
}
impl LivingWorldPoolMut<'_> {
    fn matches_authorities(&self, manager: &crate::simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
        backend: &crate::simulation_rigid_body_pool::SimulationRigidBodyPoolAuthority) -> bool {
        match self { Self::Dynamic(p) => p.matches_authorities(manager, backend),
            Self::Pedestrian(p) => p.matches_authorities(manager, backend) }
    }
    fn island_body(&self, handle: &SimulationDmoAssemblyHandle) -> Option<DynamicIslandBody<'_>> {
        match self { Self::Dynamic(p) => p.island_body(handle), Self::Pedestrian(p) => p.island_body(handle) }
    }
    fn try_set_island_body_id(&mut self, backend: &crate::simulation_assembly_backend::SimulationAssemblyBackend<
        AssemblyPartMaterial, crate::simulation_joint_pool::SimulationJointFrames, crate::simulation_joint_pool::SimulationJointLimits>,
        handle: &SimulationDmoAssemblyHandle, id: u32,
    ) -> Result<(), SimulationDmoPoolError<Infallible>> {
        match self { Self::Dynamic(p) => p.try_set_island_body_id(backend, handle, id),
            Self::Pedestrian(p) => p.try_set_island_body_id(backend, handle, id) }
    }
}
fn pool<'a>(
    kind: LivingWorldBodyKind,
    dmos: Option<&'a SimulationDmoAssemblyPool>,
    queries: Option<&'a SimulationWorldLineQueries>,
    pedestrians: Option<&'a PedestrianAssemblyPool>,
) -> Result<LivingWorldPool<'a>, VehicleIslandError> {
    match kind {
        LivingWorldBodyKind::Dmo => dmos.map(LivingWorldPool::Dynamic),
        LivingWorldBodyKind::Vehicle => queries.map(|q| LivingWorldPool::Dynamic(&q.vehicle)),
        LivingWorldBodyKind::Pedestrian => pedestrians.map(LivingWorldPool::Pedestrian),
    }
    .ok_or(VehicleIslandError::MissingPool)
}
fn pool_mut<'a>(
    kind: LivingWorldBodyKind,
    dmos: Option<&'a mut SimulationDmoAssemblyPool>,
    queries: Option<&'a mut SimulationWorldLineQueries>,
    pedestrians: Option<&'a mut PedestrianAssemblyPool>,
) -> Result<LivingWorldPoolMut<'a>, VehicleIslandError> {
    match kind {
        LivingWorldBodyKind::Dmo => dmos.map(LivingWorldPoolMut::Dynamic),
        LivingWorldBodyKind::Vehicle => queries.map(|q| LivingWorldPoolMut::Dynamic(&mut q.vehicle)),
        LivingWorldBodyKind::Pedestrian => pedestrians.map(LivingWorldPoolMut::Pedestrian),
    }
    .ok_or(VehicleIslandError::MissingPool)
}
pub(crate) enum RemovedLivingWorldBody {
    Dmo(SimulationDmoAssemblyCandidate),
    Vehicle(CharacterAssemblyCandidate),
    Pedestrian(CharacterAssemblyCandidate),
}
#[derive(Debug)]
pub(crate) enum LivingWorldBodyDriveError<N, T> {
    Island(VehicleIslandError),
    Drive(crate::dynamic_assembly_session::living_world_drive::LivingWorldPartDriveError<N, T>),
}
impl SimulationPhysicalBodyRegistry {
    fn living_world_index(
        &self,
        kind: LivingWorldBodyKind,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Option<usize> {
        self.living_world_members
            .iter()
            .position(|member| member.kind == kind && member.handle.same_as(handle))
    }
    pub(super) fn update_living_world_registered_count(
        &mut self,
        kind: LivingWorldBodyKind,
        handle: &SimulationDmoAssemblyHandle,
    ) {
        if let Some(index) = self.living_world_index(kind, handle) {
            self.living_world_members[index].registered_parts = self.targets.values().filter(|target|
                matches!(target, IslandSpyTarget::LivingWorld { kind: k, member, .. } if *k == kind && member.same_as(handle))
            ).count();
        }
    }
}
impl<C> SimulationAssemblyConstraintSession<AssemblyPartMaterial, C> {
    pub(crate) fn try_set_living_world_collision_group(&mut self, kind: LivingWorldBodyKind,
        handle: &SimulationDmoAssemblyHandle, group: u32) -> Result<(), VehicleIslandError> {
        let mut owner = pool_mut(kind, self.dmos.as_mut(), self.world_line_queries.as_mut(), self.pedestrians.as_mut())?;
        match &mut owner {
            LivingWorldPoolMut::Dynamic(p) => p.try_set_collision_group(handle, group),
            LivingWorldPoolMut::Pedestrian(p) => p.try_set_collision_group(handle, group),
        }.map_err(VehicleIslandError::Pool)
    }
    pub(super) fn living_world_pool(&self, kind: LivingWorldBodyKind) -> Result<LivingWorldPool<'_>, VehicleIslandError> {
        pool(kind, self.dmos.as_ref(), self.world_line_queries.as_ref(), self.pedestrians.as_ref())
    }
    pub(crate) fn try_pedestrian_collision_part_transform(&self, handle: &SimulationDmoAssemblyHandle,
        part: usize,
    ) -> Result<[f32; 16], String> {
        self.living_world_body_member_for_contacts(LivingWorldBodyKind::Pedestrian, handle)
            .map_err(|e| format!("pedestrian part membership: {e:?}"))?;
        self.pedestrians.as_ref().ok_or("pedestrian pool absent")?
            .island_body(handle).ok_or("pedestrian body absent")?.parts
            .world_part_transform(&self.backend, part)
            .ok_or_else(|| format!("pedestrian Part.GetTransform index out of range: {part}"))
    }
    pub(crate) fn try_pedestrian_collision_part_velocities(&self, handle: &SimulationDmoAssemblyHandle,
        part: usize,
    ) -> Result<([f32; 4], [f32; 4]), String> {
        self.living_world_body_member_for_contacts(LivingWorldBodyKind::Pedestrian, handle)
            .map_err(|e| format!("pedestrian velocity membership: {e:?}"))?;
        self.pedestrians.as_ref().ok_or("pedestrian pool absent")?
            .island_body(handle).ok_or("pedestrian body absent")?.parts
            .try_live_part_velocities_host(&self.backend, part)
            .map_err(|e| format!("pedestrian Part body velocities: {e:?}"))
    }
    /// `Body::SetVelocity` on Part `part` of a pedestrian collision body (TU3's shove, 0x82e38cc0).
    pub(crate) fn try_set_pedestrian_collision_part_velocity(&mut self, handle: &SimulationDmoAssemblyHandle,
        part: usize, velocity: [f32; 3],
    ) -> Result<(), String> {
        self.living_world_body_member_for_contacts(LivingWorldBodyKind::Pedestrian, handle)
            .map_err(|e| format!("pedestrian velocity membership: {e:?}"))?;
        let candidate = self.pedestrians.as_ref().ok_or("pedestrian pool absent")?
            .island_body(handle).ok_or("pedestrian body absent")?;
        candidate.parts.try_set_body_part_velocity(&mut self.backend, part, velocity)
            .map_err(|e| format!("pedestrian Body.SetVelocity: {e:?}"))
    }
    /// PC port (Skate 3 physics): the Part of a pedestrian collision Body that a rigid body is.
    pub(crate) fn pedestrian_body_part(&self, handle: &SimulationDmoAssemblyHandle,
        body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
    ) -> Option<u32> {
        let publication = self.pedestrians.as_ref()?.publication(handle)?;
        u32::try_from(publication.rigid_bodies().iter().position(|candidate| *candidate == body)?).ok()
    }
    /// DynamicObject.GetPartAngularVelocity82d2ab68 reads the HiLOD world
    /// collision body's Part `part` RigidBody mOmega (+0x30).
    pub(crate) fn try_dmo_collision_part_velocities(&self, handle: &SimulationDmoAssemblyHandle,
        part: usize,
    ) -> Result<([f32; 4], [f32; 4]), String> {
        self.living_world_body_member_for_contacts(LivingWorldBodyKind::Dmo, handle)
            .map_err(|e| format!("DMO velocity membership: {e:?}"))?;
        self.dmos.as_ref().ok_or("DMO pool absent")?
            .island_body(handle).ok_or("DMO body absent")?.parts
            .try_live_part_velocities_host(&self.backend, part)
            .map_err(|e| format!("DMO Part body velocities: {e:?}"))
    }
    /// UpdateCollisionBodyPartsFromTransform82e506d8 on the actual admitted
    /// Parts. No second integrator and no target-pose teleport.
    pub(crate) fn try_drive_living_world_collision_body<N, T>(
        &mut self,
        kind: LivingWorldBodyKind,
        handle: &SimulationDmoAssemblyHandle,
        targets: &[[f32; 16]],
        dt: f32,
        ignore_gravity: bool,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        numeric: &mut N,
        trig: &mut T,
    ) -> Result<(), LivingWorldBodyDriveError<N::Error, T::Error>>
    where
        N: crate::reckoning_numeric::ReckoningNumeric,
        T: crate::reckoning_sin_cos::ReckoningSinCosNumeric,
    {
        self.living_world_body_member_for_contacts(kind, handle)
            .map_err(LivingWorldBodyDriveError::Island)?;
        let candidate = pool(kind, self.dmos.as_ref(), self.world_line_queries.as_ref(), self.pedestrians.as_ref())
            .map_err(LivingWorldBodyDriveError::Island)?
            .island_body(handle)
            .ok_or(LivingWorldBodyDriveError::Island(
                VehicleIslandError::MissingMember,
            ))?;
        candidate
            .parts
            .try_update_living_world_collision_parts(
                &mut self.backend,
                targets,
                dt,
                ignore_gravity,
                cvars,
                numeric,
                trig,
            )
            .map_err(LivingWorldBodyDriveError::Drive)
    }
    pub(crate) fn try_remove_vehicle_collision(
        &mut self,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Result<CharacterAssemblyCandidate, VehicleIslandError> {
        match self.try_remove_living_world_collision(LivingWorldBodyKind::Vehicle, handle)? {
            RemovedLivingWorldBody::Vehicle(candidate) => Ok(candidate),
            RemovedLivingWorldBody::Dmo(_) | RemovedLivingWorldBody::Pedestrian(_) => unreachable!("selected vehicle pool"),
        }
    }
    pub(crate) fn try_remove_pedestrian_collision(&mut self, handle: &SimulationDmoAssemblyHandle)
        -> Result<CharacterAssemblyCandidate, VehicleIslandError> {
        match self.try_remove_living_world_collision(LivingWorldBodyKind::Pedestrian, handle)? {
            RemovedLivingWorldBody::Pedestrian(candidate) => Ok(candidate),
            _ => unreachable!("selected pedestrian pool"),
        }
    }
    /// CleanUpVehicle827b6a98/CleanUpDMO827b6b90 remove spies then contacts
    /// then DynamicPool.Remove (job sync, detach, swap-last, owned meshes).
    /// RemoveBody finally erases static then dynamic membership. No ID refund.
    pub(crate) fn try_remove_living_world_collision(
        &mut self,
        kind: LivingWorldBodyKind,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Result<RemovedLivingWorldBody, VehicleIslandError> {
        use VehicleIslandError as E;
        self.living_world_body_member_for_contacts(kind, handle)?;
        let index = self
            .physical_bodies
            .living_world_index(kind, handle)
            .ok_or(E::MissingMember)?;
        let candidate = pool(kind, self.dmos.as_ref(), self.world_line_queries.as_ref(), self.pedestrians.as_ref())?
            .island_body(handle)
            .ok_or(E::MissingMember)?;
        let count = candidate.parts.part_transforms().len();
        self.physical_bodies.living_world_members[index].registration_complete = false;
        for part in 0..count {
            let observation = candidate
                .parts
                .try_remove_part_contact_spy(&mut self.backend, part)
                .map_err(E::Spy)?;
            if let Some(previous) = self.physical_bodies.targets.remove(&observation) {
                self.physical_bodies.note_replaced_target(previous);
            }
        }
        self.physical_contacts
            .remove_island_body(&kind.body(handle));
        let removed = match kind {
            LivingWorldBodyKind::Dmo => {
                self.world_line_queries.as_ref().ok_or(E::MissingPool)?.try_sync_dmo_jobs().map_err(E::Query)?;
                let pool = self.dmos.as_mut().ok_or(E::MissingPool)?;
                let body = pool.try_remove_serialized(&mut self.backend, handle).map_err(E::Pool)?;
                pool.remove_owned_meshes(handle);
                RemovedLivingWorldBody::Dmo(body)
            }
            LivingWorldBodyKind::Vehicle => {
                let queries = self.world_line_queries.as_mut().ok_or(E::MissingPool)?;
                queries.try_sync_vehicle_jobs().map_err(E::Query)?;
                let body = queries.vehicle.try_remove_character(&mut self.backend, handle).map_err(E::Pool)?;
                queries.vehicle.remove_owned_meshes(handle);
                RemovedLivingWorldBody::Vehicle(body)
            }
            LivingWorldBodyKind::Pedestrian => {
                // CleanUpPedestrian827b6c88 invokes the BASE pool Remove.
                // Host base updates complete synchronously; no pending job,
                // DynamicPool line work or dynamic mesh slots exist here.
                let pool = self.pedestrians.as_mut().ok_or(E::MissingPool)?;
                RemovedLivingWorldBody::Pedestrian(pool.try_remove(&mut self.backend, handle).map_err(E::Pool)?)
            }
        };
        let body = kind.body(handle);
        self.physical_bodies
            .static_order
            .retain(|v| !v.same_body(&body));
        self.physical_bodies
            .dynamic_order
            .retain(|v| !v.same_body(&body));
        self.physical_bodies
            .living_world_members
            .retain(|m| m.kind != kind || !m.handle.same_as(handle));
        Ok(removed)
    }
    /// New detached body branch of AddToWorld82e50cb0. The existing-body early
    /// return belongs to the entity's retained collision owner. Position every
    /// Part before Insert, then Island ID/list, then ascending spy registration.
    pub(crate) fn try_admit_vehicle_collision(
        &mut self,
        candidate: CharacterAssemblyCandidate,
        transforms: &[[f32; 16]],
        requested_state: u32,
    ) -> Result<SimulationDmoAssemblyHandle, VehicleAdmissionFailure> {
        self.try_admit_character_collision(LivingWorldBodyKind::Vehicle, candidate, transforms, requested_state)
    }
    /// AddPedestrianBody827b68e8 uses the separate base AssemblyPool but the
    /// same placement, Island ID/list and spy-registration sequence.
    pub(crate) fn try_admit_pedestrian_collision(&mut self, candidate: CharacterAssemblyCandidate,
        transforms: &[[f32; 16]], requested_state: u32,
    ) -> Result<SimulationDmoAssemblyHandle, VehicleAdmissionFailure> {
        self.try_admit_character_collision(LivingWorldBodyKind::Pedestrian, candidate, transforms, requested_state)
    }
    fn try_admit_character_collision(&mut self, kind: LivingWorldBodyKind,
        mut candidate: CharacterAssemblyCandidate, transforms: &[[f32; 16]], requested_state: u32,
    ) -> Result<SimulationDmoAssemblyHandle, VehicleAdmissionFailure> {
        let placement = (|| {
            if self.living_world_pool(kind).is_err() {
                return Err(VehiclePlacementError::MissingPool);
            }
            let count = candidate.assembly.parts.part_transforms().len();
            if count > transforms.len() {
                return Err(VehiclePlacementError::PartTransforms {
                    parts: count,
                    transforms: transforms.len(),
                });
            }
            for (index, transform) in transforms.iter().take(count).enumerate() {
                // Body.SetPosition82de8090, null Island branch: Part.SetTransform.
                candidate
                    .assembly
                    .parts
                    .try_set_detached_part_transform(index, *transform)
                    .map_err(VehiclePlacementError::Part)?;
            }
            Ok(())
        })();
        if let Err(error) = placement {
            return Err(VehicleAdmissionFailure::Placement { error, candidate });
        }
        let handle = match kind {
            LivingWorldBodyKind::Vehicle => self.world_line_queries.as_mut().expect("checked above").vehicle
                .try_insert_character(&mut self.contacts, &mut self.backend, candidate, requested_state),
            LivingWorldBodyKind::Pedestrian => self.pedestrians.as_mut().expect("checked above")
                .try_insert(&mut self.contacts, &mut self.backend, candidate, requested_state),
            LivingWorldBodyKind::Dmo => unreachable!("character admission only used by vehicle/pedestrian"),
        }.map_err(VehicleAdmissionFailure::Insert)?;
        self.try_add_living_world_body_membership(kind, &handle, requested_state)
            .map_err(|error| VehicleAdmissionFailure::Island {
                error,
                handle: handle.clone(),
            })?;
        self.try_register_living_world_body_targets(kind, &handle)
            .map_err(|error| VehicleAdmissionFailure::Island {
                error,
                handle: handle.clone(),
            })?;
        Ok(handle)
    }
    pub(super) fn vehicle_body_member_for_contacts(
        &self,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Result<&LivingWorldIslandMember, VehicleIslandError> {
        self.living_world_body_member_for_contacts(LivingWorldBodyKind::Vehicle, handle)
    }
    pub(super) fn living_world_body_member_for_contacts(
        &self,
        kind: LivingWorldBodyKind,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Result<&LivingWorldIslandMember, VehicleIslandError> {
        use VehicleIslandError as E;
        let pool = pool(kind, self.dmos.as_ref(), self.world_line_queries.as_ref(), self.pedestrians.as_ref())?;
        if !pool.matches_authorities(
            &self.contacts.assembly_pool_authority(),
            &self.backend.authority(),
        ) {
            return Err(E::ForeignManager);
        }
        let member = &self.physical_bodies.living_world_members[self
            .physical_bodies
            .living_world_index(kind, handle)
            .ok_or(E::MissingMember)?];
        let candidate = pool.island_body(handle).ok_or(E::MissingMember)?;
        if candidate.body_id != Some(member.body_id)
            || candidate.entity_id != Some(member.entity_id)
            || candidate.requested_state != Some(member.requested_state)
        {
            return Err(E::StateMismatch);
        }
        if !candidate.parts.is_attached() {
            return Err(E::MissingMember);
        }
        Ok(member)
    }
    pub(crate) fn try_add_vehicle_body_membership(
        &mut self,
        handle: &SimulationDmoAssemblyHandle,
        state: u32,
    ) -> Result<u32, VehicleIslandError> {
        self.try_add_living_world_body_membership(LivingWorldBodyKind::Vehicle, handle, state)
    }
    pub(crate) fn try_add_living_world_body_membership(
        &mut self,
        kind: LivingWorldBodyKind,
        handle: &SimulationDmoAssemblyHandle,
        state: u32,
    ) -> Result<u32, VehicleIslandError> {
        use VehicleIslandError as E;
        let mut pool = pool_mut(kind, self.dmos.as_mut(), self.world_line_queries.as_mut(), self.pedestrians.as_mut())?;
        if !pool.matches_authorities(
            &self.contacts.assembly_pool_authority(),
            &self.backend.authority(),
        ) {
            return Err(E::ForeignManager);
        }
        let candidate = pool.island_body(handle).ok_or(E::MissingMember)?;
        if kind == LivingWorldBodyKind::Vehicle && candidate.group != 10 {
            return Err(E::WrongCollisionGroup(candidate.group));
        }
        // A static body of SpawnStatic 0x825eb0a0 keeps the Assembly's group 0.
        let spawn_static = candidate.group == 0 && state == 1;
        if kind == LivingWorldBodyKind::Dmo && !matches!(candidate.group, 14..=16) && !spawn_static {
            return Err(E::WrongCollisionGroup(candidate.group));
        }
        if kind == LivingWorldBodyKind::Pedestrian && !matches!(candidate.group, 11..=13) {
            return Err(E::WrongCollisionGroup(candidate.group));
        }
        if candidate.requested_state != Some(state) {
            return Err(E::StateMismatch);
        }
        if self
            .physical_bodies
            .living_world_index(kind, handle)
            .is_some()
        {
            return Err(E::AlreadyMember);
        }
        let entity_id = candidate.entity_id.ok_or(E::MissingEntityOwner)?;
        self.physical_bodies
            .living_world_members
            .try_reserve(1)
            .map_err(|_| E::Allocation)?;
        let body_id = self.physical_bodies.allocate_body_id(state);
        pool.try_set_island_body_id(&self.backend, handle, body_id)
            .map_err(E::Pool)?;
        self.physical_bodies
            .living_world_members
            .push(LivingWorldIslandMember {
                kind,
                handle: handle.clone(),
                entity_id,
                body_id,
                requested_state: state,
                list_published: false,
                registered_parts: 0,
                registration_complete: false,
            });
        let list = if state == 1 {
            &mut self.physical_bodies.static_order
        } else {
            &mut self.physical_bodies.dynamic_order
        };
        list.try_reserve(1).map_err(|_| E::Allocation)?;
        list.push(kind.body(handle));
        self.physical_bodies
            .living_world_members
            .last_mut()
            .expect("inserted above")
            .list_published = true;
        Ok(body_id)
    }
    pub(crate) fn try_register_vehicle_body_targets(
        &mut self,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Result<(), VehicleIslandError> {
        self.try_register_living_world_body_targets(LivingWorldBodyKind::Vehicle, handle)
    }
    /// RegisterSpyTarget827b7830: original Part order, state |=8 before map store.
    pub(crate) fn try_register_living_world_body_targets(
        &mut self,
        kind: LivingWorldBodyKind,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Result<(), VehicleIslandError> {
        use VehicleIslandError as E;
        self.living_world_body_member_for_contacts(kind, handle)?;
        let index = self
            .physical_bodies
            .living_world_index(kind, handle)
            .ok_or(E::MissingMember)?;
        if !self.physical_bodies.living_world_members[index].list_published {
            return Err(E::IncompleteMembership);
        }
        let candidate = pool(kind, self.dmos.as_ref(), self.world_line_queries.as_ref(), self.pedestrians.as_ref())?
            .island_body(handle)
            .ok_or(E::MissingMember)?;
        let parts = candidate.parts;
        let count = parts.part_transforms().len();
        self.physical_bodies.living_world_members[index].registration_complete = false;
        for part in 0..count {
            self.physical_bodies
                .targets
                .try_reserve(1)
                .map_err(|_| E::Allocation)?;
            let observation = parts
                .try_register_part_contact_spy(&mut self.backend, part)
                .map_err(E::Spy)?;
            if let Some(previous) = self.physical_bodies.targets.insert(
                observation,
                IslandSpyTarget::LivingWorld {
                    kind,
                    member: handle.clone(),
                    part: u32::try_from(part).expect("validated Part capacity"),
                },
            ) {
                self.physical_bodies.note_replaced_target(previous);
            }
            self.physical_bodies
                .update_living_world_registered_count(kind, handle);
        }
        for part in 0..count {
            let observation = parts
                .try_part_contact_spy_observation(&self.backend, part)
                .map_err(E::Spy)?;
            if !matches!(self.physical_bodies.targets.get(&observation),
                Some(IslandSpyTarget::LivingWorld { kind: k, member, part: tag }) if *k == kind && member.same_as(handle) && *tag as usize == part)
            {
                return Err(E::TargetMismatch(part));
            }
        }
        self.physical_bodies.living_world_members[index].registration_complete = true;
        Ok(())
    }
}
#[cfg(test)]
#[path = "living_world_vehicle_island_tests.rs"]
mod tests;
