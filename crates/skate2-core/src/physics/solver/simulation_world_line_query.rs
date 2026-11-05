//! Selected simulation-island World line route, 0x827ab338/0x827b8728.
//! Ground is borrowed from the actual authored-ground owner. Dynamic collections
//! have their real constructor-empty lifetime; publication and `BeginUpdate` are
//! not supplied here. Existing DMO membership remains the sole DMO authority.

use crate::{
    collision_line_collection::{
        CollisionLineCollectionError, CollisionLineCollectionMesh, CollisionLineInput,
        try_publish_collection_line,
    },
    collision_mesh_line::{CollisionMeshLineNearest, CollisionMeshOccurrence},
    collision_query_batches::{
        CollisionQueryBatchConfig, CollisionQueryBatchError, CollisionQueryBatchExecutionError,
        CollisionQueryBatchFamily, CollisionQueryBatchLease, CollisionQueryBatches,
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::SimulationAssemblyBackend,
    simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric,
    simulation_contact_jacobian::network_preparation::{
        SimulationManagerContactAuthority, SimulationManagerContactDomain,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::{
        SimulationDmoAssemblyHandle, SimulationDmoAssemblyPool,
    },
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits},
    simulation_rigid_body_pool::SimulationRigidBodyPoolAuthority,
    static_ground_pool::StaticGroundPool,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{cell::Cell, fmt, sync::Arc};

#[path = "simulation_world_step_query.rs"]
pub(crate) mod step;
#[path = "simulation_world_trajectory_query.rs"]
pub(crate) mod trajectory;
#[path = "simulation_world_line_group.rs"]
pub(crate) mod group;

/// Actual DMO assembly identity survives every mesh/child/aggregate publication.
pub(crate) type SimulationWorldLineHit = CollisionMeshLineNearest<SimulationDmoAssemblyHandle>;

type SourceBackend =
    SimulationAssemblyBackend<AssemblyPartMaterial, SimulationJointFrames, SimulationJointLimits>;

#[cfg(test)]
#[path = "simulation_world_line_query_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "simulation_world_line_corpus.rs"]
pub(crate) mod corpus;

/// Values sampled at the source pool constructors, not replacement tuning.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SimulationWorldLineQueryConfig {
    pub(crate) vehicle_update_in_parallel: bool,
    pub(crate) ground: CollisionQueryBatchConfig,
    pub(crate) dmo: CollisionQueryBatchConfig,
    pub(crate) vehicle: CollisionQueryBatchConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SimulationWorldLineChild {
    Ground,
    Dmo,
    Vehicle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SimulationWorldLineInitError {
    UnsupportedVehicleParallel,
    UnsupportedIslandFlags(u32),
    ForeignBackend,
    ForeignDmoOwner,
    Allocation,
}

#[derive(Debug)]
pub(crate) enum SimulationWorldLineChildError<E> {
    Collection(CollisionLineCollectionError<E>),
    UnpublishedMembership { count: usize },
}

#[derive(Debug)]
pub(crate) enum SimulationWorldLineError<E> {
    ZeroFlags,
    ForeignLease,
    ReleasedLease,
    ForeignDmoOwner,
    Batch {
        child: SimulationWorldLineChild,
        source: CollisionQueryBatchError,
    },
    Execution {
        child: SimulationWorldLineChild,
        source: CollisionQueryBatchExecutionError<SimulationWorldLineChildError<E>>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SimulationWorldLineLeaseError {
    ZeroFlags,
    ForeignLease,
    ReleasedLease,
    ForeignDmoOwner,
    Batch {
        child: SimulationWorldLineChild,
        source: CollisionQueryBatchError,
    },
}
impl fmt::Display for SimulationWorldLineLeaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "World line batch: {self:?}")
    }
}
impl<E> From<SimulationWorldLineLeaseError> for SimulationWorldLineError<E> {
    fn from(value: SimulationWorldLineLeaseError) -> Self {
        match value {
            SimulationWorldLineLeaseError::ZeroFlags => Self::ZeroFlags,
            SimulationWorldLineLeaseError::ForeignLease => Self::ForeignLease,
            SimulationWorldLineLeaseError::ReleasedLease => Self::ReleasedLease,
            SimulationWorldLineLeaseError::ForeignDmoOwner => Self::ForeignDmoOwner,
            SimulationWorldLineLeaseError::Batch { child, source } => Self::Batch { child, source },
        }
    }
}

impl fmt::Display for SimulationWorldLineInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "World line initialization: {self:?}")
    }
}
impl<E: fmt::Debug> fmt::Display for SimulationWorldLineError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "World line query: {self:?}")
    }
}

/// Constructor allocates mesh storage and publishes zero entries, with no
/// spatial index (0x827caff4..0x827cb10c). No mutation API can manufacture a
/// record or publish a spawned object while publication is unsupported.
struct ConstructorCollection {
    slots: Vec<CollisionLineCollectionMesh<'static, SimulationDmoAssemblyHandle>>,
}

impl ConstructorCollection {
    fn try_new(capacity: usize) -> Result<Self, SimulationWorldLineInitError> {
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationWorldLineInitError::Allocation)?;
        Ok(Self { slots })
    }

    fn query<N: AssemblyGroundNumeric + ReckoningNumeric>(
        &self,
        input: CollisionLineInput,
        numeric: &mut N,
    ) -> Result<SimulationWorldLineHit, CollisionLineCollectionError<N::Error>> {
        let mut result = CollisionMeshLineNearest::new();
        try_publish_collection_line(
            self.slots.iter().map(|slot| CollisionLineCollectionMesh {
                occurrence: CollisionMeshOccurrence {
                    mesh: slot.occurrence.mesh,
                    bytes: slot.occurrence.bytes,
                    forward: slot.occurrence.forward,
                    inverse: slot.occurrence.inverse,
                    owner: slot.occurrence.owner.clone(),
                    slot: slot.occurrence.slot,
                },
                local_bounds: slot.local_bounds,
            }),
            input,
            &mut result,
            numeric,
        )?;
        Ok(result)
    }
}

#[derive(Debug)]
struct AcquiredChild {
    child: SimulationWorldLineChild,
    lease: CollisionQueryBatchLease,
}

/// One retained Island acquisition. Dropping it never executes Release.
/// Every child lease also retains its own exact underlying acquisition identity.
#[derive(Debug)]
pub(crate) struct SimulationWorldLineBatch {
    owner: Arc<()>,
    released: Cell<bool>,
    children: Vec<AcquiredChild>,
}

pub(crate) struct SimulationWorldLineQueries {
    occurrence: Arc<()>,
    #[cfg(test)]
    last_line_input: Option<CollisionLineInput>,
    manager: SimulationManagerContactAuthority,
    backend: SimulationRigidBodyPoolAuthority,
    dmo_occurrence: Arc<()>,
    island_flags: u32,
    ground_batches: CollisionQueryBatches<SimulationDmoAssemblyHandle>,
    dmo_batches: CollisionQueryBatches<SimulationDmoAssemblyHandle>,
    vehicle_batches: CollisionQueryBatches<SimulationDmoAssemblyHandle>,
    dmo_collection: ConstructorCollection,
    /// Same native DynamicPool implementation as DMO, separate24-entry owner.
    pub(crate) vehicle: SimulationDmoAssemblyPool,
    /// A host refusal preserves source acquisitions and completed child effects.
    /// No Drop implementation silently releases native batch slots.
    retained_acquisitions: Vec<AcquiredChild>,
}

impl SimulationWorldLineQueries {
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn skate3_island_flags(&self) -> u32 { self.island_flags }
    /// DynamicPool.Remove827c92b0 and BeginUpdate827c93f0 synchronize these
    /// four job families before changing mesh storage. Sync does not release
    /// a caller's batch lease; completed synchronous results remain retained.
    pub(crate) fn try_sync_vehicle_jobs(&self) -> Result<(), CollisionQueryBatchError> {
        self.vehicle_batches.sync_jobs_all()
    }
    pub(crate) fn try_sync_dmo_jobs(&self) -> Result<(), CollisionQueryBatchError> {
        self.dmo_batches.sync_jobs_all()
    }
    pub(crate) fn belongs_to(&self, manager: SimulationManagerContactAuthority) -> bool {
        self.manager == manager
    }
    /// Scoped constructor route of `Island::InitWorld` 0x827b42c0. The enclosing
    /// session supplies its selected island flags and the original sampled
    /// constructor batch parameters. This does not advance `BeginUpdate`.
    pub(crate) fn try_new(
        manager: &SimulationManagerContactDomain,
        backend: &SourceBackend,
        dmos: &SimulationDmoAssemblyPool,
        island_flags: u32,
        mut config: SimulationWorldLineQueryConfig,
    ) -> Result<Self, SimulationWorldLineInitError> {
        if !matches!(island_flags, 1 | 3) {
            return Err(SimulationWorldLineInitError::UnsupportedIslandFlags(
                island_flags,
            ));
        }
        if manager.assembly_body_pool_authority() != backend.authority() {
            return Err(SimulationWorldLineInitError::ForeignBackend);
        }
        if !dmos.matches_authorities(&manager.assembly_pool_authority(), &backend.authority()) {
            return Err(SimulationWorldLineInitError::ForeignDmoOwner);
        }
        // DynamicPool constructor supplies literal one, not batchTestCount,
        // to StepSelector allocation at 0x827cb1ac.
        config.dmo.step_selector.tests = 1;
        config.vehicle.step_selector.tests = 1;
        if config.vehicle_update_in_parallel {
            return Err(SimulationWorldLineInitError::UnsupportedVehicleParallel);
        }
        let vehicle = SimulationDmoAssemblyPool::try_new_vehicle(manager, backend, false)
            .map_err(|_| SimulationWorldLineInitError::Allocation)?;
        Ok(Self {
            occurrence: Arc::new(()),
            #[cfg(test)]
            last_line_input: None,
            manager: manager.assembly_pool_authority(),
            backend: backend.authority(),
            dmo_occurrence: Arc::clone(dmos.occurrence()),
            island_flags,
            ground_batches: CollisionQueryBatches::new(config.ground),
            dmo_batches: CollisionQueryBatches::new(config.dmo),
            vehicle_batches: CollisionQueryBatches::new(config.vehicle),
            dmo_collection: ConstructorCollection::try_new(50)?,
            vehicle,
            retained_acquisitions: Vec::new(),
        })
    }

    fn batches_mut(
        &mut self,
        child: SimulationWorldLineChild,
    ) -> &mut CollisionQueryBatches<SimulationDmoAssemblyHandle> {
        match child {
            SimulationWorldLineChild::Ground => &mut self.ground_batches,
            SimulationWorldLineChild::Dmo => &mut self.dmo_batches,
            SimulationWorldLineChild::Vehicle => &mut self.vehicle_batches,
        }
    }

    /// `World::LineCollisionTest` 0x827ab338, `BF_Default=7`. Caller already selected
    /// this World's simulation island. Ground is the same borrowed pool used by
    /// the scoped authored-ground solver, not a copied collection.
    pub(crate) fn try_line_test<N: AssemblyGroundNumeric + ReckoningNumeric>(
        &mut self,
        ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool,
        input: CollisionLineInput,
        label: &str,
        numeric: &mut N,
    ) -> Result<Option<SimulationWorldLineHit>, SimulationWorldLineError<N::Error>> {
        self.try_scoped_line_test(ground, dmos, input, label, 7, numeric)
    }

    /// `LivingWorld::Manager::DoesAnyDynamicObjectIntersectCapsule` 0x82703930:
    /// acquire the World line batch with `BF_Dmo`, then execute the same
    /// start/end/radius AddTest and batch lifecycle as other line requests.
    /// The selected host World has only the simulation island; native World's
    /// separate visual-island flag can override requested flags to BF_Default.
    pub(crate) fn try_dmo_capsule_test<N: AssemblyGroundNumeric + ReckoningNumeric>(
        &mut self,
        ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool,
        input: CollisionLineInput,
        numeric: &mut N,
    ) -> Result<bool, SimulationWorldLineError<N::Error>> {
        self.try_scoped_line_test(ground, dmos, input, "LWMan::DynamicObjectTest", 2, numeric)
            .map(|result| result.is_some())
    }

    fn try_scoped_line_test<N: AssemblyGroundNumeric + ReckoningNumeric>(
        &mut self,
        ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool,
        input: CollisionLineInput,
        label: &str,
        flags: u32,
        numeric: &mut N,
    ) -> Result<Option<SimulationWorldLineHit>, SimulationWorldLineError<N::Error>> {
        #[cfg(test)]
        {
            self.last_line_input = Some(input);
        }
        let batch = self.try_acquire_line_batch(dmos, label, flags)?;
        let outcome = (|| {
            self.try_begin(&batch)?;
            let _ = self.try_add_line(&batch, input)?;
            self.try_end(&batch, ground, dmos, numeric)?;
            self.try_sync(&batch)?;
            let nearest = self.try_get_result(&batch, 0)?;
            self.try_release(&batch)?;
            Ok(nearest)
        })();
        if outcome.is_err() {
            self.retained_acquisitions.extend(batch.children);
        }
        outcome
    }

    #[cfg(test)]
    pub(crate) fn last_line_input_for_test(&self) -> Option<CollisionLineInput> {
        self.last_line_input
    }

    fn require_dmo_owner(
        &self,
        dmos: &SimulationDmoAssemblyPool,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        if !dmos.matches_authorities(&self.manager, &self.backend)
            || !Arc::ptr_eq(&self.dmo_occurrence, dmos.occurrence())
        {
            return Err(SimulationWorldLineLeaseError::ForeignDmoOwner);
        }
        Ok(())
    }

    fn require_batch_owner(
        &self,
        batch: &SimulationWorldLineBatch,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        if !Arc::ptr_eq(&self.occurrence, &batch.owner) {
            return Err(SimulationWorldLineLeaseError::ForeignLease);
        }
        if batch.released.get() {
            return Err(SimulationWorldLineLeaseError::ReleasedLease);
        }
        Ok(())
    }

    /// Direct Island acquisition 0x827b8728. Flag bits select Ground, DMO,
    /// then the conditional Vehicle child; no World visual-island override.
    /// A later acquisition failure retains all earlier successful acquisitions.
    pub(crate) fn try_acquire_line_batch(
        &mut self,
        dmos: &SimulationDmoAssemblyPool,
        label: &str,
        flags: u32,
    ) -> Result<SimulationWorldLineBatch, SimulationWorldLineLeaseError> {
        // Source assertion flags != BF_None at 0x827b874c..0x827b8760
        // precedes all child acquisitions. Unknown nonzero bits are permitted.
        if flags == 0 {
            return Err(SimulationWorldLineLeaseError::ZeroFlags);
        }
        self.require_dmo_owner(dmos)?;
        debug_assert!(
            self.vehicle
                .matches_authorities(&self.manager, &self.backend)
        );
        let mut children = Vec::with_capacity(3);
        for (child, selected) in [
            (SimulationWorldLineChild::Ground, flags & 1 != 0),
            (SimulationWorldLineChild::Dmo, flags & 2 != 0),
            (
                SimulationWorldLineChild::Vehicle,
                flags & 4 != 0 && self.island_flags == 3,
            ),
        ] {
            if !selected {
                continue;
            }
            match self
                .batches_mut(child)
                .acquire(CollisionQueryBatchFamily::Line, label)
            {
                Ok(lease) => children.push(AcquiredChild { child, lease }),
                Err(source) => {
                    self.retained_acquisitions.extend(children);
                    return Err(SimulationWorldLineLeaseError::Batch { child, source });
                }
            }
        }
        Ok(SimulationWorldLineBatch {
            owner: Arc::clone(&self.occurrence),
            released: Cell::new(false),
            children,
        })
    }

    pub(crate) fn try_begin(
        &mut self,
        batch: &SimulationWorldLineBatch,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        self.require_batch_owner(batch)?;
        for entry in &batch.children {
            self.batches_mut(entry.child)
                .begin(&entry.lease)
                .map_err(|source| SimulationWorldLineLeaseError::Batch {
                    child: entry.child,
                    source,
                })?;
        }
        Ok(())
    }

    /// Aggregate `AddTest` 0x827bbe50 visits ALL children. A false capacity bool
    /// never suppresses later child admission. Typed host errors retain prefix.
    pub(crate) fn try_add_line(
        &mut self,
        batch: &SimulationWorldLineBatch,
        input: CollisionLineInput,
    ) -> Result<bool, SimulationWorldLineLeaseError> {
        self.require_batch_owner(batch)?;
        let mut accepted = true;
        for entry in &batch.children {
            let child_accepted = self
                .batches_mut(entry.child)
                .add_line(&entry.lease, input)
                .map_err(|source| SimulationWorldLineLeaseError::Batch {
                    child: entry.child,
                    source,
                })?;
            accepted &= child_accepted;
        }
        Ok(accepted)
    }

    /// Existing synchronous local-job policy, with results retained in the
    /// SAME acquired children until the explicit post-solve Release.
    pub(crate) fn try_end<N: AssemblyGroundNumeric + ReckoningNumeric>(
        &mut self,
        batch: &SimulationWorldLineBatch,
        ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool,
        numeric: &mut N,
    ) -> Result<(), SimulationWorldLineError<N::Error>> {
        use SimulationWorldLineChild::{Dmo, Ground, Vehicle};
        self.require_batch_owner(batch)?;
        self.require_dmo_owner(dmos)?;
        for entry in &batch.children {
            let result = match entry.child {
                Ground => self.ground_batches.end_line(&entry.lease, |input| {
                    ground
                        .try_run_line_test(input, numeric)
                        .map(|hit| CollisionMeshLineNearest {
                            position: hit.position,
                            normal: hit.normal,
                            line_parameter: hit.line_parameter,
                            surface_id: hit.surface_id,
                            owner: None,
                            provenance: hit.provenance,
                            hit_count: hit.hit_count,
                        })
                        .map_err(SimulationWorldLineChildError::Collection)
                }),
                // DynamicPool line batches read the mesh collection the last
                // BeginUpdate (and any later Remove) published.
                Dmo => self.dmo_batches.end_line(&entry.lease, |input| {
                    dmos.try_line_query(input, numeric)
                        .map_err(SimulationWorldLineChildError::Collection)
                }),
                Vehicle => self.vehicle_batches.end_line(&entry.lease, |input| {
                    self.vehicle
                        .try_line_query(input, numeric)
                        .map_err(SimulationWorldLineChildError::Collection)
                }),
            };
            result.map_err(|source| SimulationWorldLineError::Execution {
                child: entry.child,
                source,
            })?;
        }
        Ok(())
    }

    pub(crate) fn try_sync(
        &mut self,
        batch: &SimulationWorldLineBatch,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        self.require_batch_owner(batch)?;
        for entry in &batch.children {
            self.batches_mut(entry.child)
                .sync(&entry.lease)
                .map_err(|source| SimulationWorldLineLeaseError::Batch {
                    child: entry.child,
                    source,
                })?;
        }
        Ok(())
    }

    /// Each child's count gates this index separately (0x827bbf08). Strictly
    /// ordered less selects; equal/unordered preserves the earlier child.
    pub(crate) fn try_get_result(
        &mut self,
        batch: &SimulationWorldLineBatch,
        index: u32,
    ) -> Result<Option<SimulationWorldLineHit>, SimulationWorldLineLeaseError> {
        self.require_batch_owner(batch)?;
        let mut nearest = None;
        // Island returns its sole child directly; only AggregateLineBatch
        // supplies the per-child count gate.
        if let [entry] = batch.children.as_slice() {
            let candidate = self
                .batches_mut(entry.child)
                .get_line_result(&entry.lease, index)
                .map_err(|source| SimulationWorldLineLeaseError::Batch {
                    child: entry.child,
                    source,
                })?;
            select_nearest(&mut nearest, candidate);
            return Ok(nearest);
        }
        for entry in &batch.children {
            let batches = self.batches_mut(entry.child);
            let count = batches.test_count(&entry.lease).map_err(|source| {
                SimulationWorldLineLeaseError::Batch {
                    child: entry.child,
                    source,
                }
            })?;
            if index >= count {
                continue;
            }
            let candidate = batches
                .get_line_result(&entry.lease, index)
                .map_err(|source| SimulationWorldLineLeaseError::Batch {
                    child: entry.child,
                    source,
                })?;
            select_nearest(&mut nearest, candidate);
        }
        Ok(nearest)
    }

    pub(crate) fn try_release(
        &mut self,
        batch: &SimulationWorldLineBatch,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        self.require_batch_owner(batch)?;
        for entry in &batch.children {
            self.batches_mut(entry.child)
                .release(&entry.lease)
                .map_err(|source| SimulationWorldLineLeaseError::Batch {
                    child: entry.child,
                    source,
                })?;
        }
        batch.released.set(true);
        Ok(())
    }

    /// Test-only observation of completed local jobs, without native Get/Sync.
    #[cfg(test)]
    pub(crate) fn completed_line_for_test(
        &mut self,
        batch: &SimulationWorldLineBatch,
        index: u32,
    ) -> Option<SimulationWorldLineHit> {
        self.require_batch_owner(batch).unwrap();
        let mut nearest = None;
        for entry in &batch.children {
            let child = self.batches_mut(entry.child);
            if index < child.test_count(&entry.lease).unwrap() {
                select_nearest(
                    &mut nearest,
                    child.completed_line_for_test(&entry.lease, index).unwrap(),
                );
            }
        }
        nearest
    }
}

fn require_empty_membership<E>(count: usize) -> Result<(), SimulationWorldLineChildError<E>> {
    if count == 0 {
        Ok(())
    } else {
        Err(SimulationWorldLineChildError::UnpublishedMembership { count })
    }
}

/// `AggregateLineBatch::GetResult` 0x827bbec8: strictly ordered less replaces;
/// equal and unordered comparisons preserve the earlier child.
fn select_nearest(
    nearest: &mut Option<SimulationWorldLineHit>,
    candidate: Option<&SimulationWorldLineHit>,
) {
    if let Some(candidate) = candidate
        && nearest
            .as_ref()
            .is_none_or(|current| candidate.line_parameter < current.line_parameter)
    {
        *nearest = Some(CollisionMeshLineNearest {
            position: candidate.position,
            normal: candidate.normal,
            line_parameter: candidate.line_parameter,
            surface_id: candidate.surface_id,
            owner: candidate.owner.clone(),
            provenance: candidate.provenance,
            hit_count: candidate.hit_count,
        });
    }
}

impl SimulationWorldLineQueryConfig {
    /// Ground World constructor827b19d4..19fc, then selected Island DMO/Vehicle constructors.
    /// This captures each provider separately in its original read order.
    pub(crate) fn try_sample_selected(
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        island_flags: u32,
    ) -> Result<Self, crate::input_cvar_runtime::InputCvarError> {
        use crate::collision_query_batches::CollisionQueryBatchCapacity as Capacity;
        let [line, boxes, trajectory, tests] = cvars.try_ground_query_counts()?;
        let ground = CollisionQueryBatchConfig {
            line: Capacity {
                batches: line,
                tests,
            },
            boxes: Capacity {
                batches: boxes,
                tests,
            },
            trajectory: Capacity {
                batches: trajectory,
                tests: 1,
            },
            step_selector: Capacity {
                batches: 18,
                tests: 1,
            },
        };
        let dynamic = |other_count| -> Result<
            CollisionQueryBatchConfig,
            crate::input_cvar_runtime::InputCvarError,
        > {
            let [line, boxes, trajectory, step, tests] =
                cvars.try_dynamic_query_counts(island_flags, other_count)?;
            Ok(CollisionQueryBatchConfig {
                line: Capacity {
                    batches: line,
                    tests,
                },
                boxes: Capacity {
                    batches: boxes,
                    tests,
                },
                trajectory: Capacity {
                    batches: trajectory,
                    tests,
                },
                step_selector: Capacity {
                    batches: step,
                    tests: 1,
                },
            })
        };
        let dmo = dynamic(1)?;
        let vehicle_update_in_parallel = cvars.try_vehicle_pool_update_in_parallel()?;
        let vehicle = dynamic(0)?;
        Ok(Self {
            vehicle_update_in_parallel,
            ground,
            dmo,
            vehicle,
        })
    }
}
