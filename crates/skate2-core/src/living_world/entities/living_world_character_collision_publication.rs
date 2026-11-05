//! Programmatic LivingWorld Parts -> the original SerialisedAssembly record
//! layout. Create82e0c850, Update82e0cf30, bounds job827cdf78. Shares volume
//! selection, semantic body projection and swept bounds with existing owners.
//! Pool admission and scheduling are not performed by this publication.
use crate::{
    authored_collision_publication::{semantic_motion, semantic_part_transform},
    living_world_grind_services::character_models::{
        SharedCollisionDefinition,
        collision_assembly::{
            CharacterCollisionAssembly, CharacterCollisionBackend, CharacterVolumeSelectionError,
        },
    },
    octree::AabbWords,
    serialised_assembly_bbox::{
        HostSerialisedAssemblyBBoxNumeric, SerialisedAssemblyBodyMotion,
        try_calculate_primitive_part_bbox, try_union_part_bboxes,
    },
    serialised_assembly_template::SerialisedAssemblyVolumeDataState,
    simulation_rigid_body_pool::{
        SimulationRigidBodyFrameIdentity, SimulationRigidBodyObservation,
        SimulationRigidBodyPoolAuthority, SimulationRigidBodyPoolError,
    },
};
use skate2_formats::collision::{AssemblyPartMaterial, BitTable, VolumeType};
use std::{convert::Infallible, sync::Arc};

pub(crate) struct CharacterCollisionPublication {
    definition: SharedCollisionDefinition,
    body_pool: SimulationRigidBodyPoolAuthority,
    bodies: Vec<SimulationRigidBodyObservation>,
    mesh_arenas: Vec<Option<Arc<crate::world::CollisionArena>>>,
    records: Vec<SerialisedAssemblyVolumeDataState>,
    bounds: AabbWords,
    num_static: u32,
    all_parts_frozen: bool,
    group: u32,
    culling: Arc<BitTable>,
    bend_threshold: f32,
    volume_frame: Option<SimulationRigidBodyFrameIdentity>,
    bounds_frame: Option<SimulationRigidBodyFrameIdentity>,
}

#[derive(Debug)]
pub(crate) enum CharacterPublicationError {
    Detached,
    ForeignBackend,
    DefinitionChanged,
    PoisonedDefinition,
    Allocation,
    PartCount,
    MissingBody(usize),
    BodyOccurrenceChanged(usize),
    Body(SimulationRigidBodyPoolError),
    Selection(CharacterVolumeSelectionError),
    SelectionChanged,
    UnsupportedVolume(u32),
    NoPrimitiveBounds,
    FrameUnavailable,
    StaleVolumes,
}

#[derive(Clone, Copy)]
struct LivePart {
    body: SimulationRigidBodyObservation,
    transform: [f32; 16],
    motion: SerialisedAssemblyBodyMotion,
    material: AssemblyPartMaterial,
    user_data: u32,
    state: u32,
}

impl CharacterCollisionPublication {
    pub(crate) fn try_create(
        owner: &CharacterCollisionAssembly,
        backend: &CharacterCollisionBackend,
        bend_threshold: f32,
    ) -> Result<Self, CharacterPublicationError> {
        let parts = read_parts(owner, backend)?;
        // Insert82e0dbb0 calls Create82e0c850 from current Part state, even
        // between simulation frames. Match the existing serialized-assembly
        // Create path: initial geometry/bounds need no solved-frame certificate.
        // A velocity write may have invalidated the host epoch. Preserve None
        // until the real BeginUpdate/EndUpdate refresh; never invent a frame.
        let frame = backend.collision_body_pool().frame_identity();
        let sources = owner
            .try_volume_sources()
            .map_err(CharacterPublicationError::Selection)?;
        let mut records = Vec::new();
        records
            .try_reserve_exact(sources.len())
            .map_err(|_| CharacterPublicationError::Allocation)?;
        let mut bounds = None;
        for source in sources {
            let part = parts
                .get(source.part_ordinal)
                .ok_or(CharacterPublicationError::PartCount)?;
            let record = SerialisedAssemblyVolumeDataState {
                rigid_body_index: source.rigid_body_index,
                volume_type: source.volume_type,
                enabled: source.volume.flags & 1 != 0,
                volume: source.volume,
                padding: source.padding,
                surface_id: source.surface_id,
                material: part.material,
                transform: part.transform,
                group: part.user_data,
                frozen: part.state & 2 != 0,
            };
            // Create unions each primitive immediately; mesh wrappers have
            // records but contribute no CPU bounds. Keep disabled primitives.
            if let Some(candidate) = record_bounds(&record, part)? {
                bounds = Some(match bounds {
                    None => candidate,
                    Some(previous) => infallible(try_union_part_bboxes(
                        &[(previous, 1), (candidate, 1)],
                        &mut HostSerialisedAssemblyBBoxNumeric,
                    )),
                });
            }
            records.push(record);
        }
        let bounds = bounds.ok_or(CharacterPublicationError::NoPrimitiveBounds)?;
        let mesh_arenas = owner.definition.lock().map_err(|_| CharacterPublicationError::PoisonedDefinition)?
            .parts.iter().map(|part| match &part.geometry {
                crate::living_world_grind_services::character_models::ModelCollisionGeometry::VehicleMappedArray { source, .. } => Some(Arc::clone(source)),
                _ => None,
            }).collect();
        let (num_static, all_parts_frozen) = header_state(&parts);
        Ok(Self {
            definition: Arc::clone(&owner.definition),
            body_pool: backend.authority(),
            bodies: parts.iter().map(|part| part.body).collect(),
            mesh_arenas,
            records,
            bounds,
            num_static,
            all_parts_frozen,
            group: owner.group,
            culling: Arc::clone(&owner.culling),
            bend_threshold,
            volume_frame: frame,
            bounds_frame: frame,
        })
    }

    pub(crate) fn invalidate_refresh(&mut self) {
        self.volume_frame = None;
        self.bounds_frame = None;
    }
    /// AssemblyPool.EndUpdate82e0bbb4..bbf0 rereads every live Part state,
    /// independently of the frozen bits copied by SerialisedAssembly.Update.
    pub(crate) fn try_all_parts_frozen(&self, owner: &CharacterCollisionAssembly,
        backend: &CharacterCollisionBackend) -> Result<bool, CharacterPublicationError> {
        let parts = self.matching_parts(owner, backend)?;
        Ok(parts.iter().fold(true, |frozen, part| frozen & (part.state & 2 != 0)))
    }

    fn matching_parts(
        &self,
        owner: &CharacterCollisionAssembly,
        backend: &CharacterCollisionBackend,
    ) -> Result<Vec<LivePart>, CharacterPublicationError> {
        if backend.authority() != self.body_pool {
            return Err(CharacterPublicationError::ForeignBackend);
        }
        if !Arc::ptr_eq(&self.definition, &owner.definition) {
            return Err(CharacterPublicationError::DefinitionChanged);
        }
        let parts = read_parts(owner, backend)?;
        if parts.len() != self.bodies.len() {
            return Err(CharacterPublicationError::PartCount);
        }
        for (index, part) in parts.iter().enumerate() {
            if part.body != self.bodies[index] {
                return Err(CharacterPublicationError::BodyOccurrenceChanged(index));
            }
        }
        Ok(parts)
    }

    /// Only enabled/material/transform/group/frozen change in inline records.
    /// In particular Update does not recopy the Volume's geometry/header.
    pub(crate) fn try_update_volumes(
        &mut self,
        owner: &CharacterCollisionAssembly,
        backend: &CharacterCollisionBackend,
    ) -> Result<(), CharacterPublicationError> {
        self.invalidate_refresh();
        let parts = self.matching_parts(owner, backend)?;
        let frame = backend
            .collision_body_pool()
            .frame_identity()
            .ok_or(CharacterPublicationError::FrameUnavailable)?;
        let selected = owner
            .try_volume_sources()
            .map_err(CharacterPublicationError::Selection)?;
        if selected.len() != self.records.len() {
            return Err(CharacterPublicationError::SelectionChanged);
        }
        for (record, source) in self.records.iter_mut().zip(selected) {
            if record.rigid_body_index != source.rigid_body_index
                || record.volume_type != source.volume_type
            {
                return Err(CharacterPublicationError::SelectionChanged);
            }
            let part = parts
                .get(source.part_ordinal)
                .ok_or(CharacterPublicationError::PartCount)?;
            record.enabled = source.volume.flags & 1 != 0;
            record.material = part.material;
            record.transform = part.transform;
            record.group = part.user_data;
            record.frozen = part.state & 2 != 0;
        }
        (self.num_static, self.all_parts_frozen) = header_state(&parts);
        self.group = owner.group;
        self.culling = Arc::clone(&owner.culling);
        self.volume_frame = Some(frame);
        Ok(())
    }

    /// Bounds job writes every temporary primitive box before the final union.
    /// Native aggregate entries have validity0, independently of volume flags.
    pub(crate) fn try_stage_bounds(
        &self,
        owner: &CharacterCollisionAssembly,
        backend: &CharacterCollisionBackend,
    ) -> Result<AabbWords, CharacterPublicationError> {
        let parts = self.matching_parts(owner, backend)?;
        let frame = backend.collision_body_pool().frame_identity();
        if frame.is_none() || frame != self.volume_frame {
            return Err(CharacterPublicationError::StaleVolumes);
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(self.records.len())
            .map_err(|_| CharacterPublicationError::Allocation)?;
        for record in &self.records {
            let part = parts
                .get(usize::from(record.rigid_body_index))
                .ok_or(CharacterPublicationError::PartCount)?;
            entries.push(match record_bounds(record, part)? {
                Some(bounds) => (bounds, 1),
                None => (AabbWords::from_f32([0.; 4], [0.; 4]), 0),
            });
        }
        Ok(infallible(try_union_part_bboxes(
            &entries,
            &mut HostSerialisedAssemblyBBoxNumeric,
        )))
    }

    pub(crate) fn publish_unstamped_bounds(&mut self, bounds: AabbWords) {
        self.bounds = bounds;
        self.bounds_frame = None;
    }
    pub(crate) fn stamp_bounds(&mut self, frame: SimulationRigidBodyFrameIdentity) {
        self.bounds_frame = Some(frame);
    }
    /// Actual backing of the copied inline aggregate record for this Part.
    /// Retained at Create, alongside the original Volume pointer projection.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn part_mesh_owner(&self, part: usize) -> Option<Arc<crate::world::CollisionArena>> {
        self.mesh_arenas.get(part).and_then(Option::as_ref).map(Arc::clone)
    }
    pub(crate) fn part_mesh_arena(&self, part: usize) -> Option<&crate::world::CollisionArena> {
        self.mesh_arenas.get(part).and_then(Option::as_deref)
    }
    pub(crate) fn records(&self) -> &[SerialisedAssemblyVolumeDataState] {
        &self.records
    }
    pub(crate) fn rigid_bodies(&self) -> &[SimulationRigidBodyObservation] {
        &self.bodies
    }
    pub(crate) fn bounds(&self) -> AabbWords {
        self.bounds
    }
    pub(crate) fn num_static(&self) -> u32 {
        self.num_static
    }
    pub(crate) fn all_parts_frozen(&self) -> bool {
        self.all_parts_frozen
    }
    pub(crate) fn group(&self) -> u32 {
        self.group
    }
    pub(crate) fn culling_table(&self) -> &BitTable {
        &self.culling
    }
    pub(crate) fn bend_threshold(&self) -> f32 {
        self.bend_threshold
    }
    pub(crate) fn body_pool_authority(&self) -> SimulationRigidBodyPoolAuthority {
        self.body_pool
    }
    pub(crate) fn has_bounds_for_frame(&self, frame: SimulationRigidBodyFrameIdentity) -> bool {
        self.volume_frame == Some(frame) && self.bounds_frame == Some(frame)
    }
}

fn read_parts(
    owner: &CharacterCollisionAssembly,
    backend: &CharacterCollisionBackend,
) -> Result<Vec<LivePart>, CharacterPublicationError> {
    let session = &owner.parts;
    match session.attached_authority {
        None => return Err(CharacterPublicationError::Detached),
        Some(authority) if authority != backend.authority() => {
            return Err(CharacterPublicationError::ForeignBackend);
        }
        _ => {}
    }
    let definition = owner
        .definition
        .lock()
        .map_err(|_| CharacterPublicationError::PoisonedDefinition)?;
    let attachment = &session.transaction;
    let count = attachment.parts.len();
    if count != definition.parts.len() || count != owner.part_user_data.len() {
        return Err(CharacterPublicationError::PartCount);
    }
    let mut parts = Vec::new();
    parts
        .try_reserve_exact(count)
        .map_err(|_| CharacterPublicationError::Allocation)?;
    let pool = backend.collision_body_pool();
    for (index, part) in attachment.parts.iter().enumerate() {
        let body = part
            .body
            .ok_or(CharacterPublicationError::MissingBody(index))?;
        let observation = pool
            .try_observe(body)
            .map_err(CharacterPublicationError::Body)?;
        let snapshot = pool
            .try_serialised_assembly_snapshot(observation)
            .map_err(CharacterPublicationError::Body)?;
        parts.push(LivePart {
            body: observation,
            transform: semantic_part_transform(&snapshot, &attachment.template.metadata[index]),
            motion: semantic_motion(
                snapshot.velocity,
                snapshot.omega,
                snapshot.force,
                snapshot.kinematic_value,
                snapshot.torque,
                snapshot.cooldown,
            ),
            material: definition.parts[index].material,
            user_data: owner.part_user_data[index],
            state: snapshot.state,
        });
    }
    Ok(parts)
}

fn record_bounds(
    record: &SerialisedAssemblyVolumeDataState,
    part: &LivePart,
) -> Result<Option<AabbWords>, CharacterPublicationError> {
    if record.volume_type == VolumeType::Aggregate {
        return Ok(None);
    }
    infallible(try_calculate_primitive_part_bbox(
        &record.volume,
        &record.transform,
        part.motion,
        &mut HostSerialisedAssemblyBBoxNumeric,
    ))
    .map(Some)
    .ok_or(CharacterPublicationError::UnsupportedVolume(
        record.volume.vtable_offset,
    ))
}
fn header_state(parts: &[LivePart]) -> (u32, bool) {
    (
        parts
            .iter()
            .fold(0, |count, part| count + u32::from(part.state & 1 != 0)),
        parts.iter().all(|part| part.state & 2 != 0),
    )
}
fn infallible<T>(value: Result<T, Infallible>) -> T {
    match value {
        Ok(value) => value,
        Err(never) => match never {},
    }
}
