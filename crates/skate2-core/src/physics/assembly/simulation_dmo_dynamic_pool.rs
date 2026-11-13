//! Source-bound DMO Body construction and generic serialized pool membership.
//! Body 0x825ea858, Insert 0x82e0dbb0 and base Remove 0x82e0c1c8.
//! This is the Simulation DMO constructor path used by `SpawnStatic`, distinct
//! from living-world colliders' characteristic-modified group14/16 path.
//! Dynamic mesh/batch and outer Island BodyID/list ownership are subsequent
//! phases; none is replaced by this serialized membership authority.

use super::{
    SimulationDmoAssemblyPartAttachError, SimulationDmoAssemblyPartOwner,
    SimulationDmoAssemblyPartSession, SimulationDmoResolvedLocatorPartOwner,
    SimulationDmoResolvedLocatorPartSessionCreateError, SourceBackend,
};
use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyBodyOnlyPartDetachError,
        character_publication::CharacterCollisionPublication,
        source_parts::DynamicAssemblySourcePartError,
    },
    living_world_grind_services::character_models::collision_assembly::CharacterCollisionAssembly,
    octree::AabbWords,
    serialised_assembly_bbox::SerialisedAssemblyPartBBoxNumeric,
    serialised_assembly_create::{
        PreparedSerialisedAssembly, SerialisedAssemblyCreateError,
        SerialisedAssemblyLiveAssemblyInput, SerialisedAssemblyParamsState,
        SimulationPackedBodyWordProviderError, try_prepare_serialised_assembly_create_from_xenon,
    },
    serialised_assembly_template::{
        SerialisedAssemblyCullingTableSource, SerialisedAssemblyTemplateError,
    },
    simulation_assembly_backend::source::SimulationAssemblySourceError,
    simulation_contact_jacobian::network_preparation::{
        SimulationManagerContactAuthority, SimulationManagerContactDomain,
    },
    simulation_island_map::persistent::SimulationRigidBodyIslandStateError,
    simulation_rigid_body_pool::SimulationRigidBodyPoolAuthority,
};
use std::{fmt, sync::Arc};

#[cfg(test)]
#[path = "simulation_dmo_dynamic_pool_tests.rs"]
pub(crate) mod tests;

#[path = "simulation_dynamic_pool_character.rs"]
pub(crate) mod character;
#[path = "simulation_dmo_dynamic_pool_update.rs"]
mod update;
pub(crate) use update::{DmoMeshGeometry, DmoMeshSlot, SimulationDmoPoolUpdateError};

#[derive(Debug)]
pub(crate) enum SimulationDmoCandidateError {
    Allocation,
    Template(SerialisedAssemblyTemplateError),
    Session(SimulationDmoResolvedLocatorPartSessionCreateError),
    /// `AddToWorld` 0x82e50cfc asserts `numParts <= partTransforms.size()`;
    /// `FillCollisionPartTargetTransforms` supplies one transform.
    LivingWorldPartTransforms {
        parts: usize,
    },
}

impl fmt::Display for SimulationDmoCandidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allocation => f.write_str("DMO candidate allocation failed"),
            Self::Template(error) => write!(f, "DMO candidate template: {error}"),
            Self::Session(error) => write!(f, "DMO candidate Parts: {error}"),
            Self::LivingWorldPartTransforms { parts } => {
                write!(
                    f,
                    "LivingWorld DMO has {parts} Parts for one target transform"
                )
            }
        }
    }
}

/// Retains the selected model's actual Part transaction (and a static
/// candidate's locator) through preflight and post-ID attachment/Create failures.
pub(crate) struct SimulationDmoAssemblyCandidate {
    parts: SimulationDmoAssemblyPartSession,
    /// `SpawnStatic` candidates keep their locator occurrence alive.
    locator: Option<Arc<SimulationDmoResolvedLocatorPartOwner>>,
    world_index: u32,
    requested_state: Option<u32>,
    entity_id: Option<u32>,
    body_id: Option<u32>,
    group: u32,
    part_user_data: Vec<u32>,
    culling_table: Option<SerialisedAssemblyCullingTableSource>,
}

impl SimulationDmoAssemblyCandidate {
    /// DMO Body constructor copies the same actual locator matrix into every
    /// fresh Part, retaining Initialize's group0/userData0 and Definition culling.
    /// Network collision queries and `SpawnStatic` iteration are not performed here.
    pub(crate) fn try_from_locator(
        owner: &Arc<SimulationDmoResolvedLocatorPartOwner>,
    ) -> Result<Self, SimulationDmoCandidateError> {
        let template = owner.parts.template();
        let count = template.parts().len();
        let culling_table = template
            .try_definition_culling_table()
            .map_err(SimulationDmoCandidateError::Template)?;
        let mut transforms = Vec::new();
        transforms
            .try_reserve_exact(count)
            .map_err(|_| SimulationDmoCandidateError::Allocation)?;
        transforms.resize(count, owner.locator.transform());
        let mut part_user_data = Vec::new();
        part_user_data
            .try_reserve_exact(count)
            .map_err(|_| SimulationDmoCandidateError::Allocation)?;
        part_user_data.resize(count, 0);
        let parts = owner
            .parts
            .try_begin_session(transforms)
            .map_err(|source| {
                SimulationDmoCandidateError::Session(
                    SimulationDmoResolvedLocatorPartSessionCreateError::PartSession(source),
                )
            })?;
        Ok(Self {
            parts,
            locator: Some(Arc::clone(owner)),
            world_index: 0xffff,
            requested_state: None,
            entity_id: None,
            body_id: None,
            group: 0,
            part_user_data,
            culling_table,
        })
    }

    /// `cLivingWorldCollisionBody::Create` 0x82e50b70: `CreateAssembly`, then
    /// the Assembly group and every Part's user data become the collision
    /// group; `Assembly::Initialize` keeps the Definition's LivingWorld culling
    /// table. `AddToWorld` 0x82e50cb0 `Body::SetPosition`s each Part from
    /// `FillCollisionPartTargetTransforms` 0x82e5f9b0's single transform.
    pub(crate) fn try_from_living_world(
        owner: &Arc<SimulationDmoAssemblyPartOwner>,
        entity_id: u32,
        part_transform: [f32; 16],
        group: u32,
        culling_table: SerialisedAssemblyCullingTableSource,
    ) -> Result<Self, SimulationDmoCandidateError> {
        let count = owner.template().parts().len();
        if count > 1 {
            return Err(SimulationDmoCandidateError::LivingWorldPartTransforms { parts: count });
        }
        let mut transforms = Vec::new();
        transforms
            .try_reserve_exact(count)
            .map_err(|_| SimulationDmoCandidateError::Allocation)?;
        transforms.resize(count, part_transform);
        let mut part_user_data = Vec::new();
        part_user_data
            .try_reserve_exact(count)
            .map_err(|_| SimulationDmoCandidateError::Allocation)?;
        part_user_data.resize(count, group);
        let parts = owner.try_begin_session(transforms).map_err(|source| {
            SimulationDmoCandidateError::Session(
                SimulationDmoResolvedLocatorPartSessionCreateError::PartSession(source),
            )
        })?;
        Ok(Self {
            parts,
            locator: None,
            world_index: 0xffff,
            requested_state: None,
            entity_id: Some(entity_id),
            body_id: None,
            group,
            part_user_data,
            culling_table: Some(culling_table),
        })
    }

    /// `Dmo::Body::Body` (`0x825ea858`), the static body `Dmo::SimManager::SpawnStatic`
    /// (`0x825eb0a0`) makes of an online game's locator: `Body::SetPosition` puts every Part at
    /// the locator's matrix, the Assembly group and Part user data keep `Assembly::Initialize`'s
    /// 0, and the Definition's culling table stays. `entity_id` is the object holding it.
    pub(crate) fn try_from_static_locator(
        owner: &Arc<SimulationDmoAssemblyPartOwner>,
        entity_id: u32,
        transform: [f32; 16],
        culling_table: SerialisedAssemblyCullingTableSource,
    ) -> Result<Self, SimulationDmoCandidateError> {
        let count = owner.template().parts().len();
        let mut transforms = Vec::new();
        transforms
            .try_reserve_exact(count)
            .map_err(|_| SimulationDmoCandidateError::Allocation)?;
        transforms.resize(count, transform);
        let mut part_user_data = Vec::new();
        part_user_data
            .try_reserve_exact(count)
            .map_err(|_| SimulationDmoCandidateError::Allocation)?;
        part_user_data.resize(count, 0);
        let parts = owner.try_begin_session(transforms).map_err(|source| {
            SimulationDmoCandidateError::Session(
                SimulationDmoResolvedLocatorPartSessionCreateError::PartSession(source),
            )
        })?;
        Ok(Self {
            parts,
            locator: None,
            world_index: 0xffff,
            requested_state: None,
            entity_id: Some(entity_id),
            body_id: None,
            group: 0,
            part_user_data,
            culling_table: Some(culling_table),
        })
    }

    pub(crate) const fn world_index(&self) -> u32 {
        self.world_index
    }
    pub(crate) const fn requested_state(&self) -> Option<u32> {
        self.requested_state
    }
    pub(crate) fn is_attached(&self) -> bool {
        self.parts.is_attached()
    }
    /// The LivingWorld entity this body was created for, once attached.
    pub(crate) const fn entity_id(&self) -> Option<u32> {
        self.entity_id
    }

    pub(crate) fn part_transforms(&self) -> impl ExactSizeIterator<Item = &[f32; 16]> {
        self.parts.part_transforms()
    }

    pub(crate) fn part_owner(&self) -> &Arc<SimulationDmoAssemblyPartOwner> {
        self.parts.owner()
    }

    pub(crate) const fn group(&self) -> u32 {
        self.group
    }

    /// Explicit recovery for an attached candidate returned by Create refusal.
    /// Issued world IDs are retained, never refunded by detach.
    pub(crate) fn try_detach(
        &mut self,
        backend: &mut SourceBackend,
    ) -> Result<(), DynamicAssemblyBodyOnlyPartDetachError> {
        self.parts.detach(backend)
    }

    fn try_create<N: SerialisedAssemblyPartBBoxNumeric>(
        &self,
        backend: &SourceBackend,
        numeric: &mut N,
    ) -> Result<PreparedSerialisedAssembly, SimulationDmoPoolError<N::Error>> {
        let source_session = &self.parts;
        let template = source_session.owner.template();
        let parts = source_session
            .parts
            .try_source_create_parts(backend, template, &self.part_user_data)
            .map_err(SimulationDmoPoolError::Part)?;
        let context = backend
            .try_source_context(template.owner())
            .map_err(SimulationDmoPoolError::Source)?;
        try_prepare_serialised_assembly_create_from_xenon(
            template,
            context.body_pool,
            context.inertia_owner,
            context.image_domain,
            &parts,
            SerialisedAssemblyLiveAssemblyInput {
                group: self.group,
                culling_table: self.culling_table.as_ref(),
            },
            SerialisedAssemblyParamsState::native_default(),
            numeric,
        )
        .map_err(SimulationDmoPoolError::Create)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SimulationDmoAssemblyHandle(Arc<()>);
impl PartialEq for SimulationDmoAssemblyHandle {
    fn eq(&self, other: &Self) -> bool {
        self.same_as(other)
    }
}
impl Eq for SimulationDmoAssemblyHandle {}
impl SimulationDmoAssemblyHandle {
    pub(crate) fn same_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug)]
pub(crate) enum SimulationDmoPoolError<N> {
    Allocation,
    ForeignManager,
    Uninitialized,
    AlreadyInitialized,
    ForeignBackend,
    UnsupportedParallel,
    Full,
    AlreadyAttached,
    MissingMember,
    WrongMemberKind,
    Labels(SimulationRigidBodyIslandStateError),
    Attach(SimulationDmoAssemblyPartAttachError),
    Detach(DynamicAssemblyBodyOnlyPartDetachError),
    Part(DynamicAssemblySourcePartError),
    Source(SimulationAssemblySourceError),
    Create(SerialisedAssemblyCreateError<N, SimulationPackedBodyWordProviderError>),
}

impl<N: fmt::Debug + fmt::Display> fmt::Display for SimulationDmoPoolError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Labels(error) => write!(f, "DMO manager labels: {error:?}"),
            Self::Attach(error) => write!(f, "DMO attachment: {error}"),
            Self::Detach(error) => write!(f, "DMO detach: {error}"),
            Self::Part(error) => write!(f, "DMO Part: {error}"),
            Self::Source(error) => write!(f, "DMO source: {error}"),
            Self::Create(error) => write!(f, "DMO Create: {error}"),
            other => write!(f, "DMO serialized pool: {other:?}"),
        }
    }
}

pub(crate) struct SimulationDmoInsertFailure<N> {
    pub(crate) error: SimulationDmoPoolError<N>,
    pub(crate) candidate: SimulationDmoAssemblyCandidate,
}
impl<N: fmt::Debug> fmt::Debug for SimulationDmoInsertFailure<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimulationDmoInsertFailure")
            .field("error", &self.error)
            .field("world_index", &self.candidate.world_index)
            .field("attached", &self.candidate.is_attached())
            .finish_non_exhaustive()
    }
}

struct Member {
    handle: SimulationDmoAssemblyHandle,
    content: MemberContent,
    bounds: AabbWords,
}

enum MemberContent {
    Source {
        candidate: SimulationDmoAssemblyCandidate,
        publication: PreparedSerialisedAssembly,
    },
    Character {
        candidate: character::CharacterAssemblyCandidate,
        publication: CharacterCollisionPublication,
    },
}
impl MemberContent {
    fn source(&self) -> Option<(&SimulationDmoAssemblyCandidate, &PreparedSerialisedAssembly)> {
        match self {
            Self::Source {
                candidate,
                publication,
            } => Some((candidate, publication)),
            _ => None,
        }
    }
    fn world_part_transforms(&self, backend: &SourceBackend) -> Vec<[f32; 16]> {
        match self {
            Self::Source { candidate, .. } => candidate.parts.world_part_transforms(backend),
            Self::Character { candidate, .. } => {
                candidate.assembly.parts.world_part_transforms(backend)
            }
        }
    }
}

pub(crate) struct SimulationDmoAssemblyPool {
    /// Both native DMO and vehicle pools are DynamicPool. Keep their storage
    /// separate while sharing implementation (InitWorld: capacities50/24).
    capacity: usize,
    occurrence: Arc<()>,
    manager: SimulationManagerContactAuthority,
    backend: SimulationRigidBodyPoolAuthority,
    members: Vec<Member>,
    /// One bound Part owner per LivingWorld model: every body of a model is
    /// created from its single edited HiLOD Definition.
    living_world_owners: Vec<(u64, Arc<SimulationDmoAssemblyPartOwner>)>,
    /// `DynamicPool::mMeshes`, rebuilt by every `BeginUpdate`.
    meshes: Vec<DmoMeshSlot>,
    /// This frame's `BeginUpdate` filled the mesh collection and returned before the base
    /// `AssemblyPool::BeginUpdate`, so its `EndUpdate` publishes the retained records.
    base_update_skipped: bool,
}

impl SimulationDmoAssemblyPool {
    pub(crate) fn occurrence(&self) -> &Arc<()> {
        &self.occurrence
    }
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "paired authority check borrows both owner keys"
    )]
    pub(crate) fn matches_authorities(
        &self,
        manager: &SimulationManagerContactAuthority,
        backend: &SimulationRigidBodyPoolAuthority,
    ) -> bool {
        &self.manager == manager && &self.backend == backend
    }

    /// Actual DMO capacity50 from `Island::InitWorld` 0x827b45dc/0x827b4618.
    /// Caller supplies the sampled DMO parallel cvar; unsupported true refuses.
    pub(crate) fn try_new(
        manager: &SimulationManagerContactDomain,
        backend: &SourceBackend,
        update_in_parallel: bool,
    ) -> Result<Self, SimulationDmoPoolError<std::convert::Infallible>> {
        Self::try_new_with_capacity(manager, backend, update_in_parallel, 50)
    }

    pub(crate) fn try_new_vehicle(
        manager: &SimulationManagerContactDomain,
        backend: &SourceBackend,
        update_in_parallel: bool,
    ) -> Result<Self, SimulationDmoPoolError<std::convert::Infallible>> {
        // Selected island flags1/3, raw827b479c..47b8.
        Self::try_new_with_capacity(manager, backend, update_in_parallel, 24)
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }

    fn try_new_with_capacity(
        manager: &SimulationManagerContactDomain,
        backend: &SourceBackend,
        update_in_parallel: bool,
        capacity: usize,
    ) -> Result<Self, SimulationDmoPoolError<std::convert::Infallible>> {
        if manager.assembly_body_pool_authority() != backend.authority() {
            return Err(SimulationDmoPoolError::ForeignBackend);
        }
        if update_in_parallel {
            return Err(SimulationDmoPoolError::UnsupportedParallel);
        }
        let mut members = Vec::new();
        members
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationDmoPoolError::Allocation)?;
        Ok(Self {
            capacity,
            occurrence: Arc::new(()),
            manager: manager.assembly_pool_authority(),
            backend: backend.authority(),
            members,
            living_world_owners: Vec::new(),
            meshes: Vec::new(),
            base_update_skipped: false,
        })
    }

    pub(crate) fn living_world_owner(
        &self,
        model_id: u64,
    ) -> Option<&Arc<SimulationDmoAssemblyPartOwner>> {
        self.living_world_owners
            .iter()
            .find(|(id, _)| *id == model_id)
            .map(|(_, owner)| owner)
    }

    pub(crate) fn remember_living_world_owner(
        &mut self,
        model_id: u64,
        owner: Arc<SimulationDmoAssemblyPartOwner>,
    ) {
        self.living_world_owners.push((model_id, owner));
    }

    /// Source ID -> Add -> raw slot -> Create -> bounds -> count.
    /// Host preflight/staging is explicit: failure returns the SAME candidate,
    /// possibly attached, instead of a native partially written raw slot. Add's
    /// native return is ignored; this checked owner refuses unsafe continuation.
    pub(crate) fn try_insert<N: SerialisedAssemblyPartBBoxNumeric>(
        &mut self,
        manager: &mut SimulationManagerContactDomain,
        backend: &mut SourceBackend,
        mut candidate: SimulationDmoAssemblyCandidate,
        state: u32,
        numeric: &mut N,
    ) -> Result<SimulationDmoAssemblyHandle, Box<SimulationDmoInsertFailure<N::Error>>> {
        let result = (|| {
            self.check_insert(
                manager,
                backend,
                candidate.parts.owner.backend_authority,
                candidate.is_attached(),
            )?;
            let handle = SimulationDmoAssemblyHandle(Arc::new(()));
            candidate.world_index = manager
                .try_allocate_assembly_world_index(self.backend)
                .map_err(SimulationDmoPoolError::Labels)?;
            candidate.requested_state = Some(state);
            candidate
                .parts
                .attach(backend, state)
                .map_err(SimulationDmoPoolError::Attach)?;
            let publication = candidate.try_create(backend, numeric)?;
            Ok((handle, publication))
        })();
        match result {
            Ok((handle, publication)) => {
                let bounds = publication.header().aabb;
                self.members.push(Member {
                    handle: handle.clone(),
                    content: MemberContent::Source {
                        candidate,
                        publication,
                    },
                    bounds,
                });
                Ok(handle)
            }
            Err(error) => Err(Box::new(SimulationDmoInsertFailure { error, candidate })),
        }
    }

    fn check_insert<N>(
        &self,
        manager: &SimulationManagerContactDomain,
        backend: &SourceBackend,
        candidate_backend: SimulationRigidBodyPoolAuthority,
        attached: bool,
    ) -> Result<(), SimulationDmoPoolError<N>> {
        if self.backend != backend.authority() || candidate_backend != self.backend {
            return Err(SimulationDmoPoolError::ForeignBackend);
        }
        if self.manager != manager.assembly_pool_authority() {
            return Err(SimulationDmoPoolError::ForeignManager);
        }
        if attached {
            return Err(SimulationDmoPoolError::AlreadyAttached);
        }
        if self.members.len() >= self.capacity {
            return Err(SimulationDmoPoolError::Full);
        }
        Ok(())
    }

    pub(crate) fn len(&self) -> usize {
        self.members.len()
    }
    pub(crate) fn handles(&self) -> impl ExactSizeIterator<Item = &SimulationDmoAssemblyHandle> {
        self.members.iter().map(|member| &member.handle)
    }
    pub(crate) fn publication(
        &self,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Option<&PreparedSerialisedAssembly> {
        self.members
            .iter()
            .find(|member| member.handle.same_as(handle))
            .and_then(|member| member.content.source().map(|(_, publication)| publication))
    }
    pub(crate) fn candidate(
        &self,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Option<&SimulationDmoAssemblyCandidate> {
        self.members
            .iter()
            .find(|member| member.handle.same_as(handle))
            .and_then(|member| member.content.source().map(|(candidate, _)| candidate))
    }

    /// `Part::GetTransform` 0x826c3480 of every Part of one member.
    pub(crate) fn world_part_transforms(
        &self,
        backend: &SourceBackend,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Option<Vec<[f32; 16]>> {
        self.members
            .iter()
            .find(|member| member.handle.same_as(handle))
            .map(|member| member.content.world_part_transforms(backend))
    }

    pub(crate) fn try_grab_part_snapshot(
        &self,
        backend: &SourceBackend,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Result<
        Option<crate::dynamic_assembly_session::source_parts::SourcePartGrabSnapshot>,
        DynamicAssemblySourcePartError,
    > {
        let member = self
            .island_body(handle)
            .ok_or(DynamicAssemblySourcePartError::NotAttached)?;
        if member.parts.part_transforms().len() == 0 {
            return Ok(None);
        }
        member.parts.try_grab_part_snapshot(backend, 0).map(Some)
    }

    /// Base Remove only: enclosing `DynamicPool` must synchronize all four batches
    /// first and remove matching mesh occurrences afterward. No active rebuild.
    pub(crate) fn try_remove_serialized(
        &mut self,
        backend: &mut SourceBackend,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Result<SimulationDmoAssemblyCandidate, SimulationDmoPoolError<std::convert::Infallible>>
    {
        if self.backend != backend.authority() {
            return Err(SimulationDmoPoolError::ForeignBackend);
        }
        let index = self
            .members
            .iter()
            .position(|member| member.handle.same_as(handle))
            .ok_or(SimulationDmoPoolError::MissingMember)?;
        let MemberContent::Source { candidate, .. } = &mut self.members[index].content else {
            return Err(SimulationDmoPoolError::WrongMemberKind);
        };
        candidate
            .parts
            .detach(backend)
            .map_err(SimulationDmoPoolError::Detach)?;
        let MemberContent::Source { candidate, .. } = self.members.swap_remove(index).content
        else {
            unreachable!()
        };
        Ok(candidate)
    }
}
