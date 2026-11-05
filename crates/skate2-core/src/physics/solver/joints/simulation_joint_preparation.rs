//! Checked pool-backed preparation for recovered Joint Jacobian records.
//!
//! `Simulation::JointBatchBuild` (`0x82ae7a78..0x82ae7b0c`) walks Active
//! Joints in right-link order, reads body B then A state, admits on
//! `((state_b | state_a) & 4) != 0`, and builds a dense record sequence. This
//! module also recovers the separate Joint block of
//! `Simulation::CreateSimulationIslandTables` (`0x82ae3eec..0x82ae3fec`):
//! B-then-A signed labels, exact B `-1` fallback, and stable per-island dense
//! subsequences. The host combines those two native results in one private
//! transaction while adding occurrence, allocation, and frame binding. A
//! checked selected-island sweep consumes this batch through a separately
//! borrowed body phase, the recovered Joint solver, and the exact A0/B0/A2/B2
//! RF store seam.
//!
//! The ordinary-finite host-policy claim covers the estimate transcript and
//! final record. Seven persistent adjacent body metadata words are replaced
//! with zero only because an independent raw-graph discriminator proves them
//! dead at both of those boundaries. Reduction operands and exact native
//! intermediate W images remain unclaimed.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "checked Joint sweep awaits the owning Horus runtime"
    )
)]

use crate::{
    simulation_island_map::{SIMULATION_ISLAND_CAPACITY, SimulationRigidBodyIslandMapSnapshot},
    simulation_joint_build_stage1::SimulationJointBuildStage1Body,
    simulation_joint_build_through_effective_mass::{
        SimulationJointBuildBody, SimulationJointInertRecordWords,
        build_simulation_joint_inert_record,
    },
    simulation_joint_jacobian::{SimulationJointJacobian, solve_joint_horus_record_host},
    simulation_joint_numeric::{
        SimulationJointNumericBackend, SimulationJointReciprocalSite,
        SimulationJointReciprocalSqrtSite, SimulationJointSumSite, SimulationJointVector,
    },
    simulation_joint_pool::{
        SimulationJointActiveBuildVisitError, SimulationJointBuildBinding,
        SimulationJointBuildBindingValidationError, SimulationJointFrames, SimulationJointIdentity,
        SimulationJointLimits, SimulationJointPool, SimulationJointPoolAuthority,
        SimulationJointPoolError,
    },
    simulation_rigid_body_pool::{
        SimulationRigidBodyDynamicUpdatePhase, SimulationRigidBodyDynamicUpdatePhaseIdentity,
        SimulationRigidBodyFrameIdentity, SimulationRigidBodyIndexedSolverDeltaError,
        SimulationRigidBodyIndexedSolverDeltaPlan, SimulationRigidBodyJointBuildSnapshot,
        SimulationRigidBodyObservation, SimulationRigidBodyPoolAuthority,
        SimulationRigidBodyPoolError, SimulationRigidBodySolverDeltaPairRows,
        SimulationRigidBodySolverDeltaStores,
    },
};
use std::{convert::Infallible, num::NonZeroU32};

type ExactJointPool = SimulationJointPool<SimulationJointFrames, SimulationJointLimits>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationJointEndpoint {
    Parent,
    Child,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationJointPreparedBindingKind {
    BodyA,
    BodyB,
    Joint,
}

#[derive(Clone, Copy)]
enum SimulationJointIslandMapSource<'a> {
    Null {
        frame: SimulationRigidBodyFrameIdentity,
    },
    Present(&'a SimulationRigidBodyIslandMapSnapshot),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationJointPreparationReadKind {
    StateB,
    StateA,
    SnapshotA,
    SnapshotB,
    Frames,
    Limits,
    Spy,
    DenseOrdinal,
    BindingTokens,
    IslandLabelB,
    IslandLabelA,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SimulationJointPreparationError<E> {
    IslandMapBodyPoolMismatch,
    IslandMapFrameMismatch,
    ValidationAllocationFailed {
        capacity: usize,
    },
    Pool(SimulationJointPoolError),
    StagingAllocationFailed {
        capacity: usize,
    },
    Body {
        active_ordinal: usize,
        joint: SimulationJointIdentity,
        endpoint: SimulationJointEndpoint,
        source: SimulationRigidBodyPoolError,
    },
    MissingIslandLabel {
        active_ordinal: usize,
        joint: SimulationJointIdentity,
        endpoint: SimulationJointEndpoint,
        body: SimulationRigidBodyObservation,
    },
    IslandLabelOccurrenceMismatch {
        active_ordinal: usize,
        joint: SimulationJointIdentity,
        endpoint: SimulationJointEndpoint,
        expected_body: SimulationRigidBodyObservation,
        mapped_body: SimulationRigidBodyObservation,
    },
    InvalidSelectedIsland {
        active_ordinal: usize,
        joint: SimulationJointIdentity,
        raw_body_b: i8,
        raw_body_a: i8,
        selected: i8,
    },
    IslandCountOverflow {
        island: u8,
        count: usize,
    },
    BindingTokenOverflow {
        active_ordinal: usize,
        joint: SimulationJointIdentity,
        kind: SimulationJointPreparedBindingKind,
        slot: u32,
    },
    DenseOrdinalOverflow {
        active_ordinal: usize,
        joint: SimulationJointIdentity,
        ordinal: usize,
    },
    RecordAllocationFailed {
        capacity: usize,
    },
    IslandAllocationFailed {
        island: u8,
        capacity: usize,
    },
    Numeric {
        dense_ordinal: u32,
        joint: SimulationJointIdentity,
        source: E,
    },
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct SimulationPreparedJointSource {
    joint_pool: SimulationJointPoolAuthority,
    body_pool: SimulationRigidBodyPoolAuthority,
    frame: SimulationRigidBodyFrameIdentity,
    phase: SimulationRigidBodyDynamicUpdatePhaseIdentity,
}

#[derive(Clone, Copy)]
struct SimulationJointRecordBindings {
    solver_delta_a: NonZeroU32,
    solver_delta_b: NonZeroU32,
    joint: NonZeroU32,
}

struct SimulationStagedJointBuildInput {
    island: usize,
    joint: SimulationJointIdentity,
    body_a: SimulationRigidBodyJointBuildSnapshot,
    body_b: SimulationRigidBodyJointBuildSnapshot,
    frames: SimulationJointFrames,
    limits: SimulationJointLimits,
    timestep: f32,
    spy: u32,
    active_ordinal: usize,
    dense_ordinal: u32,
    bindings: Option<SimulationJointRecordBindings>,
}

pub(crate) struct SimulationPreparedJointJacobian {
    joint: SimulationJointIdentity,
    body_a: SimulationRigidBodyObservation,
    body_b: SimulationRigidBodyObservation,
    active_ordinal: usize,
    dense_ordinal: u32,
    record: SimulationJointJacobian,
}

struct SimulationPreparedJointIslandEntry {
    dense_ordinal: u32,
    joint: SimulationJointIdentity,
    body_a: SimulationRigidBodyObservation,
    body_b: SimulationRigidBodyObservation,
}

/// Unpublished, non-cloneable batch of dense Joint records and island tables.
///
/// The immutable Joint borrow prevents add/remove/reuse while this batch can
/// be executed. The batch is sealed to the exclusive body phase used for
/// preparation; execution separately borrows that exact phase, which freezes
/// topology, numeric fields, frame configuration, and RF storage while live.
/// Dropping this value performs no simulation work.
#[must_use = "dropping prepared Joints abandons them without RF effects"]
pub(crate) struct SimulationPreparedJointIslandBatch<'joint> {
    joint_pool: &'joint ExactJointPool,
    source: SimulationPreparedJointSource,
    records: Vec<SimulationPreparedJointJacobian>,
    islands: [Vec<SimulationPreparedJointIslandEntry>; SIMULATION_ISLAND_CAPACITY],
}

impl SimulationPreparedJointIslandBatch<'_> {
    pub(crate) fn record_count(&self) -> usize {
        self.records.len()
    }
}

/// Checked live-pool producer using the existing ordinary-finite host numeric
/// policy and an actual manager-produced map. No test adapter or raw records.
pub(crate) fn try_prepare_active_joint_islands_host<'joint>(
    joint_pool: &'joint ExactJointPool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    map: &SimulationRigidBodyIslandMapSnapshot,
) -> Result<SimulationPreparedJointIslandBatch<'joint>, SimulationJointPreparationError<Infallible>>
{
    try_prepare_active_joint_phase_with_island_map_and_backend(
        joint_pool,
        phase,
        SimulationJointIslandMapSource::Present(map),
        &mut SimulationJointHostNumericBackend,
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationPreparedJointIslandHorusError {
    IslandOutOfRange {
        island: usize,
    },
    FrameMismatch,
    JointPoolSourceMismatch,
    BodyPoolSourceMismatch,
    PhaseMismatch,
    TableEntryMismatch {
        island_ordinal: usize,
        dense_ordinal: u32,
    },
    Joint(SimulationJointBuildBindingValidationError),
    BindingMismatch {
        island_ordinal: usize,
        dense_ordinal: u32,
        joint: SimulationJointIdentity,
        kind: SimulationJointPreparedBindingKind,
        expected: u32,
        actual: u32,
    },
    Body(SimulationRigidBodyIndexedSolverDeltaError),
}

/// Typed pre-effect refusal while flattening all scheduled Joint islands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationPreparedJointHorusSerialPlanError {
    IslandCountOutOfRange {
        island_count: usize,
    },
    Island {
        island: usize,
        source: SimulationPreparedJointIslandHorusError,
    },
    EntryCountOverflow,
    BodySequence(SimulationRigidBodyIndexedSolverDeltaError),
}

/// One phase-bound Joint plan with exact table-order ranges for ten islands.
pub(crate) struct SimulationPreparedJointHorusSerialPlan<'batch> {
    rf: SimulationRigidBodyIndexedSolverDeltaPlan<'batch, SimulationPreparedJointJacobian>,
    ranges: [(usize, usize); SIMULATION_ISLAND_CAPACITY],
    island_count: usize,
}

impl SimulationJointIslandMapSource<'_> {
    const fn frame(self) -> SimulationRigidBodyFrameIdentity {
        match self {
            Self::Null { frame } => frame,
            Self::Present(snapshot) => snapshot.frame_identity(),
        }
    }

    const fn body_pool(self) -> Option<SimulationRigidBodyPoolAuthority> {
        match self {
            Self::Null { .. } => None,
            Self::Present(snapshot) => Some(snapshot.body_pool_authority()),
        }
    }
}

struct SimulationJointHostNumericBackend;

impl SimulationJointNumericBackend for SimulationJointHostNumericBackend {
    type Error = Infallible;

    fn sum3(
        &mut self,
        _site: SimulationJointSumSite,
        left: [f32; 3],
        right: [f32; 3],
    ) -> Result<f32, Self::Error> {
        Ok(left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2])))
    }

    fn reciprocal_estimate(
        &mut self,
        _site: SimulationJointReciprocalSite,
        input: SimulationJointVector,
    ) -> Result<SimulationJointVector, Self::Error> {
        Ok(input.map(f32::recip))
    }

    fn reciprocal_sqrt_estimate(
        &mut self,
        _site: SimulationJointReciprocalSqrtSite,
        input: SimulationJointVector,
    ) -> Result<SimulationJointVector, Self::Error> {
        Ok(input.map(|lane| lane.sqrt().recip()))
    }
}

fn try_prepare_active_joint_phase<'joint>(
    joint_pool: &'joint ExactJointPool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
) -> Result<SimulationPreparedJointIslandBatch<'joint>, SimulationJointPreparationError<Infallible>>
{
    let mut backend = SimulationJointHostNumericBackend;
    try_prepare_active_joint_phase_with_backend(joint_pool, phase, &mut backend)
}

fn try_prepare_active_joint_phase_with_backend<'joint, B>(
    joint_pool: &'joint ExactJointPool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    backend: &mut B,
) -> Result<SimulationPreparedJointIslandBatch<'joint>, SimulationJointPreparationError<B::Error>>
where
    B: SimulationJointNumericBackend,
{
    let frame = phase.frame_identity();
    try_prepare_active_joint_phase_with_island_map_and_backend(
        joint_pool,
        phase,
        SimulationJointIslandMapSource::Null { frame },
        backend,
    )
}

fn try_prepare_active_joint_phase_with_island_map_and_backend<'joint, B>(
    joint_pool: &'joint ExactJointPool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    island_map: SimulationJointIslandMapSource<'_>,
    backend: &mut B,
) -> Result<SimulationPreparedJointIslandBatch<'joint>, SimulationJointPreparationError<B::Error>>
where
    B: SimulationJointNumericBackend,
{
    try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
        joint_pool,
        phase,
        island_map,
        backend,
        |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
        |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
        |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
        |_, entries, capacity| entries.try_reserve_exact(capacity).map_err(|_| ()),
        |_, _, _| {},
    )
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one linear preflight path preserves source-local reads and deterministic host failures"
)]
fn try_prepare_active_joint_phase_with_backend_and_reservation<'joint, B>(
    joint_pool: &'joint ExactJointPool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    backend: &mut B,
    reserve_validation: impl FnOnce(&mut Vec<bool>, usize) -> Result<(), ()>,
    reserve_staging: impl FnOnce(&mut Vec<SimulationStagedJointBuildInput>, usize) -> Result<(), ()>,
    reserve_records: impl FnOnce(&mut Vec<SimulationPreparedJointJacobian>, usize) -> Result<(), ()>,
) -> Result<SimulationPreparedJointIslandBatch<'joint>, SimulationJointPreparationError<B::Error>>
where
    B: SimulationJointNumericBackend,
{
    try_prepare_active_joint_phase_with_backend_reservation_and_observer(
        joint_pool,
        phase,
        backend,
        reserve_validation,
        reserve_staging,
        reserve_records,
        |_, _, _| {},
    )
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one linear preflight path preserves source-local reads and deterministic host failures"
)]
fn try_prepare_active_joint_phase_with_backend_reservation_and_observer<'joint, B>(
    joint_pool: &'joint ExactJointPool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    backend: &mut B,
    reserve_validation: impl FnOnce(&mut Vec<bool>, usize) -> Result<(), ()>,
    reserve_staging: impl FnOnce(&mut Vec<SimulationStagedJointBuildInput>, usize) -> Result<(), ()>,
    reserve_records: impl FnOnce(&mut Vec<SimulationPreparedJointJacobian>, usize) -> Result<(), ()>,
    observe: impl FnMut(SimulationJointPreparationReadKind, usize, Option<u32>),
) -> Result<SimulationPreparedJointIslandBatch<'joint>, SimulationJointPreparationError<B::Error>>
where
    B: SimulationJointNumericBackend,
{
    let frame = phase.frame_identity();
    try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
        joint_pool,
        phase,
        SimulationJointIslandMapSource::Null { frame },
        backend,
        reserve_validation,
        reserve_staging,
        reserve_records,
        |_, entries, capacity| entries.try_reserve_exact(capacity).map_err(|_| ()),
        observe,
    )
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one linear preflight path preserves source-local reads and deterministic host failures"
)]
fn try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer<'joint, B>(
    joint_pool: &'joint ExactJointPool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    island_map: SimulationJointIslandMapSource<'_>,
    backend: &mut B,
    reserve_validation: impl FnOnce(&mut Vec<bool>, usize) -> Result<(), ()>,
    reserve_staging: impl FnOnce(&mut Vec<SimulationStagedJointBuildInput>, usize) -> Result<(), ()>,
    reserve_records: impl FnOnce(&mut Vec<SimulationPreparedJointJacobian>, usize) -> Result<(), ()>,
    mut reserve_island: impl FnMut(
        usize,
        &mut Vec<SimulationPreparedJointIslandEntry>,
        usize,
    ) -> Result<(), ()>,
    mut observe: impl FnMut(SimulationJointPreparationReadKind, usize, Option<u32>),
) -> Result<SimulationPreparedJointIslandBatch<'joint>, SimulationJointPreparationError<B::Error>>
where
    B: SimulationJointNumericBackend,
{
    if island_map
        .body_pool()
        .is_some_and(|authority| authority != phase.body_pool_authority())
    {
        return Err(SimulationJointPreparationError::IslandMapBodyPoolMismatch);
    }
    if island_map.frame() != phase.frame_identity() {
        return Err(SimulationJointPreparationError::IslandMapFrameMismatch);
    }
    let source = SimulationPreparedJointSource {
        joint_pool: joint_pool.authority(),
        body_pool: phase.body_pool_authority(),
        frame: phase.frame_identity(),
        phase: phase.phase_identity(),
    };
    let timestep = phase.timestep();
    let mut island_counts = [0_usize; SIMULATION_ISLAND_CAPACITY];

    let staged_result = joint_pool.try_with_active_build_occurrences_and_reservation(
        |occurrences| {
            let capacity = occurrences.len();
            let mut staged = Vec::new();
            reserve_staging(&mut staged, capacity).map_err(|()| {
                SimulationJointPreparationError::StagingAllocationFailed { capacity }
            })?;
            if staged.capacity() < capacity {
                return Err(SimulationJointPreparationError::StagingAllocationFailed { capacity });
            }

            for (active_ordinal, occurrence) in occurrences {
                let joint = occurrence.joint();
                let body_b = occurrence.body_b();
                if phase.is_externally_driven(body_b) || phase.is_externally_driven(occurrence.body_a()) {
                    continue;
                }
                observe(
                    SimulationJointPreparationReadKind::StateB,
                    active_ordinal,
                    None,
                );
                let state_b = phase
                    .try_raw_state_for_constraint(body_b)
                    .map_err(|source| SimulationJointPreparationError::Body {
                        active_ordinal,
                        joint,
                        endpoint: SimulationJointEndpoint::Parent,
                        source,
                    })?;
                let body_a = occurrence.body_a();
                observe(
                    SimulationJointPreparationReadKind::StateA,
                    active_ordinal,
                    None,
                );
                let state_a = phase
                    .try_raw_state_for_constraint(body_a)
                    .map_err(|source| SimulationJointPreparationError::Body {
                        active_ordinal,
                        joint,
                        endpoint: SimulationJointEndpoint::Child,
                        source,
                    })?;
                if ((state_b | state_a) & 4) == 0 {
                    continue;
                }

                if matches!(island_map, SimulationJointIslandMapSource::Present(_)) {
                    observe(
                        SimulationJointPreparationReadKind::IslandLabelB,
                        active_ordinal,
                        None,
                    );
                }
                let raw_body_b = try_joint_island_label(
                    island_map,
                    active_ordinal,
                    joint,
                    SimulationJointEndpoint::Parent,
                    body_b,
                )?;
                if matches!(island_map, SimulationJointIslandMapSource::Present(_)) {
                    observe(
                        SimulationJointPreparationReadKind::IslandLabelA,
                        active_ordinal,
                        None,
                    );
                }
                let raw_body_a = try_joint_island_label(
                    island_map,
                    active_ordinal,
                    joint,
                    SimulationJointEndpoint::Child,
                    body_a,
                )?;
                let island =
                    try_selected_joint_island(active_ordinal, joint, raw_body_b, raw_body_a)?;
                let island_u8 = u8::try_from(island).expect("validated island is in 0..10");
                island_counts[island] = island_counts[island].checked_add(1).ok_or(
                    SimulationJointPreparationError::IslandCountOverflow {
                        island: island_u8,
                        count: island_counts[island],
                    },
                )?;

                observe(
                    SimulationJointPreparationReadKind::SnapshotA,
                    active_ordinal,
                    None,
                );
                let body_a = phase.try_joint_build_snapshot(body_a).map_err(|source| {
                    SimulationJointPreparationError::Body {
                        active_ordinal,
                        joint,
                        endpoint: SimulationJointEndpoint::Child,
                        source,
                    }
                })?;
                observe(
                    SimulationJointPreparationReadKind::SnapshotB,
                    active_ordinal,
                    None,
                );
                let body_b = phase.try_joint_build_snapshot(body_b).map_err(|source| {
                    SimulationJointPreparationError::Body {
                        active_ordinal,
                        joint,
                        endpoint: SimulationJointEndpoint::Parent,
                        source,
                    }
                })?;
                observe(
                    SimulationJointPreparationReadKind::Frames,
                    active_ordinal,
                    None,
                );
                let frames = occurrence.frames();
                observe(
                    SimulationJointPreparationReadKind::Limits,
                    active_ordinal,
                    None,
                );
                let limits = occurrence
                    .try_limits()
                    .map_err(SimulationJointPreparationError::Pool)?;
                observe(
                    SimulationJointPreparationReadKind::Spy,
                    active_ordinal,
                    None,
                );
                let spy = occurrence.spy();
                staged.push(SimulationStagedJointBuildInput {
                    island,
                    joint,
                    body_a,
                    body_b,
                    frames,
                    limits,
                    timestep,
                    spy,
                    active_ordinal,
                    dense_ordinal: 0,
                    bindings: None,
                });
            }
            Ok(staged)
        },
        reserve_validation,
    );

    let mut staged = match staged_result {
        Ok(staged) => staged,
        Err(SimulationJointActiveBuildVisitError::ValidationAllocationFailed { capacity }) => {
            return Err(SimulationJointPreparationError::ValidationAllocationFailed { capacity });
        }
        Err(SimulationJointActiveBuildVisitError::Pool(error)) => {
            return Err(SimulationJointPreparationError::Pool(error));
        }
        Err(SimulationJointActiveBuildVisitError::Visitor(error)) => {
            return Err(error);
        }
    };

    let preflight = staged
        .iter_mut()
        .enumerate()
        .try_for_each(|(dense_ordinal, input)| {
            observe(
                SimulationJointPreparationReadKind::DenseOrdinal,
                input.active_ordinal,
                u32::try_from(dense_ordinal).ok(),
            );
            input.dense_ordinal = u32::try_from(dense_ordinal).map_err(|_| {
                SimulationJointPreparationError::DenseOrdinalOverflow {
                    active_ordinal: input.active_ordinal,
                    joint: input.joint,
                    ordinal: dense_ordinal,
                }
            })?;
            observe(
                SimulationJointPreparationReadKind::BindingTokens,
                input.active_ordinal,
                Some(input.dense_ordinal),
            );
            input.bindings = Some(try_record_bindings(input).map_err(|(kind, slot)| {
                SimulationJointPreparationError::BindingTokenOverflow {
                    active_ordinal: input.active_ordinal,
                    joint: input.joint,
                    kind,
                    slot,
                }
            })?);
            Ok(())
        });
    preflight?;

    let capacity = staged.len();
    let mut records = Vec::new();
    if reserve_records(&mut records, capacity).is_err() || records.capacity() < capacity {
        return Err(SimulationJointPreparationError::RecordAllocationFailed { capacity });
    }

    let mut islands: [Vec<SimulationPreparedJointIslandEntry>; SIMULATION_ISLAND_CAPACITY] =
        std::array::from_fn(|_| Vec::new());
    for (island, (&required, entries)) in island_counts.iter().zip(&mut islands).enumerate() {
        if required == 0 {
            continue;
        }
        let island_u8 = u8::try_from(island).expect("physical island enumeration is in 0..10");
        if reserve_island(island, entries, required).is_err() || entries.capacity() < required {
            return Err(SimulationJointPreparationError::IslandAllocationFailed {
                island: island_u8,
                capacity: required,
            });
        }
    }

    for input in staged {
        let record = compose_staged_joint_record(&input, backend)?;
        islands[input.island].push(SimulationPreparedJointIslandEntry {
            dense_ordinal: record.dense_ordinal,
            joint: record.joint,
            body_a: record.body_a,
            body_b: record.body_b,
        });
        records.push(record);
    }

    Ok(SimulationPreparedJointIslandBatch {
        joint_pool,
        source,
        records,
        islands,
    })
}

/// Prevalidates and preallocates one prepared Joint-table subsequence.
///
/// This is one ordinary-finite Joint sweep, not a complete Horus iteration.
/// All batch, phase, occurrence, table, token, selector, and body checks precede the
/// first record or RF effect. Once execution begins it is infallible except for
/// panic; an earlier/current effect prefix is deliberately not rolled back.
#[allow(
    clippy::too_many_lines,
    reason = "one linear preflight keeps every checked refusal before the first Joint/RF effect"
)]
pub(crate) fn try_prepare_joint_horus_island_sweep<'batch>(
    batch: &'batch mut SimulationPreparedJointIslandBatch<'_>,
    island: usize,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
) -> Result<
    SimulationRigidBodyIndexedSolverDeltaPlan<'batch, SimulationPreparedJointJacobian>,
    SimulationPreparedJointIslandHorusError,
> {
    validate_joint_horus_island(batch, island, phase)?;

    let SimulationPreparedJointIslandBatch {
        records, islands, ..
    } = batch;
    let entries = &islands[island];
    phase
        .try_prepare_indexed_solver_delta_plan(
            records,
            entries
                .iter()
                .map(|entry| (entry.dense_ordinal as usize, entry.body_a, entry.body_b)),
        )
        .map_err(SimulationPreparedJointIslandHorusError::Body)
}

fn validate_joint_horus_island(
    batch: &SimulationPreparedJointIslandBatch<'_>,
    island: usize,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
) -> Result<(), SimulationPreparedJointIslandHorusError> {
    if island >= SIMULATION_ISLAND_CAPACITY {
        return Err(SimulationPreparedJointIslandHorusError::IslandOutOfRange { island });
    }
    if batch.source.frame != phase.frame_identity() {
        return Err(SimulationPreparedJointIslandHorusError::FrameMismatch);
    }
    if batch.source.joint_pool != batch.joint_pool.authority() {
        return Err(SimulationPreparedJointIslandHorusError::JointPoolSourceMismatch);
    }
    if batch.source.body_pool != phase.body_pool_authority() {
        return Err(SimulationPreparedJointIslandHorusError::BodyPoolSourceMismatch);
    }
    if batch.source.phase != phase.phase_identity() {
        return Err(SimulationPreparedJointIslandHorusError::PhaseMismatch);
    }

    let entries = &batch.islands[island];
    for (island_ordinal, entry) in entries.iter().enumerate() {
        let Some(prepared) = batch.records.get(entry.dense_ordinal as usize) else {
            return Err(
                SimulationPreparedJointIslandHorusError::TableEntryMismatch {
                    island_ordinal,
                    dense_ordinal: entry.dense_ordinal,
                },
            );
        };
        if (
            prepared.dense_ordinal,
            prepared.joint,
            prepared.body_a,
            prepared.body_b,
        ) != (entry.dense_ordinal, entry.joint, entry.body_a, entry.body_b)
        {
            return Err(
                SimulationPreparedJointIslandHorusError::TableEntryMismatch {
                    island_ordinal,
                    dense_ordinal: entry.dense_ordinal,
                },
            );
        }
    }

    batch
        .joint_pool
        .try_validate_build_bindings(
            entries.iter().map(|entry| {
                SimulationJointBuildBinding::new(entry.joint, entry.body_a, entry.body_b)
            }),
        )
        .map_err(SimulationPreparedJointIslandHorusError::Joint)?;

    for (island_ordinal, entry) in entries.iter().enumerate() {
        let prepared = &batch.records[entry.dense_ordinal as usize];
        let expected_a = checked_slot_token(entry.body_a.identity().id().index())
            .expect("prepared body-A token remains representable")
            .get();
        let expected_b = checked_slot_token(entry.body_b.identity().id().index())
            .expect("prepared body-B token remains representable")
            .get();
        let expected_joint = checked_slot_token(entry.joint.id().index())
            .expect("prepared Joint token remains representable")
            .get();
        let actual = prepared.record.binding_words();
        for (kind, expected, actual) in [
            (
                SimulationJointPreparedBindingKind::BodyA,
                expected_a,
                actual[0],
            ),
            (
                SimulationJointPreparedBindingKind::BodyB,
                expected_b,
                actual[1],
            ),
            (
                SimulationJointPreparedBindingKind::Joint,
                expected_joint,
                actual[2],
            ),
        ] {
            if actual != expected {
                return Err(SimulationPreparedJointIslandHorusError::BindingMismatch {
                    island_ordinal,
                    dense_ordinal: entry.dense_ordinal,
                    joint: entry.joint,
                    kind,
                    expected,
                    actual,
                });
            }
        }
    }

    Ok(())
}

/// Prevalidates every scheduled Joint island before any solver effect.
pub(crate) fn try_prepare_joint_horus_serial_plan<'batch>(
    batch: &'batch mut SimulationPreparedJointIslandBatch<'_>,
    island_count: usize,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
) -> Result<
    SimulationPreparedJointHorusSerialPlan<'batch>,
    SimulationPreparedJointHorusSerialPlanError,
> {
    if island_count > SIMULATION_ISLAND_CAPACITY {
        return Err(
            SimulationPreparedJointHorusSerialPlanError::IslandCountOutOfRange { island_count },
        );
    }
    let mut ranges = [(0_usize, 0_usize); SIMULATION_ISLAND_CAPACITY];
    let mut entry_count = 0_usize;
    for (island, range) in ranges.iter_mut().enumerate().take(island_count) {
        validate_joint_horus_island(batch, island, phase).map_err(|source| {
            SimulationPreparedJointHorusSerialPlanError::Island { island, source }
        })?;
        let start = entry_count;
        entry_count = entry_count
            .checked_add(batch.islands[island].len())
            .ok_or(SimulationPreparedJointHorusSerialPlanError::EntryCountOverflow)?;
        *range = (start, entry_count);
    }

    let SimulationPreparedJointIslandBatch {
        records, islands, ..
    } = batch;
    let rf = phase
        .try_prepare_indexed_solver_delta_plan_counted(
            records,
            entry_count,
            islands[..island_count].iter().flat_map(|entries| {
                entries
                    .iter()
                    .map(|entry| (entry.dense_ordinal as usize, entry.body_a, entry.body_b))
            }),
        )
        .map_err(SimulationPreparedJointHorusSerialPlanError::BodySequence)?;
    Ok(SimulationPreparedJointHorusSerialPlan {
        rf,
        ranges,
        island_count,
    })
}

/// Applies one scheduled Joint range from an all-island preflight plan.
pub(crate) fn apply_prepared_joint_horus_serial_island(
    plan: &mut SimulationPreparedJointHorusSerialPlan<'_>,
    island: usize,
    phase: &mut SimulationRigidBodyDynamicUpdatePhase<'_>,
) {
    assert!(
        island < plan.island_count,
        "Joint serial-plan island must have been scheduled"
    );
    let (start, end) = plan.ranges[island];
    phase.apply_indexed_solver_delta_plan_range(
        &mut plan.rf,
        start..end,
        solve_prepared_joint_horus_record,
    );
}

/// Applies one already-prevalidated Joint island sweep.
pub(crate) fn apply_prepared_joint_horus_island_sweep(
    plan: &mut SimulationRigidBodyIndexedSolverDeltaPlan<'_, SimulationPreparedJointJacobian>,
    phase: &mut SimulationRigidBodyDynamicUpdatePhase<'_>,
) {
    phase.apply_indexed_solver_delta_plan(plan, solve_prepared_joint_horus_record);
}

/// Solves one prevalidated Joint record and stages its exact four RF stores.
pub(crate) fn solve_prepared_joint_horus_record(
    prepared: &mut SimulationPreparedJointJacobian,
    rows: SimulationRigidBodySolverDeltaPairRows,
) -> SimulationRigidBodySolverDeltaStores {
    let (body_a, body_b) = rows.into_rows();
    let solved = solve_joint_horus_record_host(&mut prepared.record, body_a, body_b);
    SimulationRigidBodySolverDeltaStores::new(
        solved.solver_delta_a[0],
        solved.solver_delta_b[0],
        solved.solver_delta_a[2],
        solved.solver_delta_b[2],
    )
}

/// Executes one checked standalone Joint island sweep.
pub(crate) fn try_solve_prepared_joint_horus_island_sweep(
    batch: &mut SimulationPreparedJointIslandBatch<'_>,
    island: usize,
    phase: &mut SimulationRigidBodyDynamicUpdatePhase<'_>,
) -> Result<(), SimulationPreparedJointIslandHorusError> {
    let mut plan = try_prepare_joint_horus_island_sweep(batch, island, phase)?;
    apply_prepared_joint_horus_island_sweep(&mut plan, phase);
    Ok(())
}

#[cfg(test)]
pub(crate) fn prepare_joint_horus_island_batch_for_test<'joint>(
    joint_pool: &'joint SimulationJointPool<SimulationJointFrames, SimulationJointLimits>,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    island_map: &SimulationRigidBodyIslandMapSnapshot,
) -> SimulationPreparedJointIslandBatch<'joint> {
    let mut backend = SimulationJointHostNumericBackend;
    try_prepare_active_joint_phase_with_island_map_and_backend(
        joint_pool,
        phase,
        SimulationJointIslandMapSource::Present(island_map),
        &mut backend,
    )
    .unwrap_or_else(|error| panic!("Joint Horus test batch preparation failed: {error:?}"))
}

#[cfg(test)]
pub(crate) fn prepared_joint_horus_batch_words_for_test(
    batch: &SimulationPreparedJointIslandBatch<'_>,
) -> Vec<[[u32; 4]; 24]> {
    batch
        .records
        .iter()
        .map(|prepared| prepared.record.test_words())
        .collect()
}

fn compose_staged_joint_record<B: SimulationJointNumericBackend>(
    input: &SimulationStagedJointBuildInput,
    backend: &mut B,
) -> Result<SimulationPreparedJointJacobian, SimulationJointPreparationError<B::Error>> {
    let bindings = input
        .bindings
        .expect("every admitted Joint binding was preflighted");
    let inert = build_simulation_joint_inert_record(
        joint_build_body(input.body_a, bindings.solver_delta_a),
        joint_build_body(input.body_b, bindings.solver_delta_b),
        input.frames,
        input.limits,
        input.timestep,
        SimulationJointInertRecordWords {
            joint_binding_word: bindings.joint.get(),
            transient_spy_word: input.spy,
        },
        backend,
    )
    .map_err(|source| SimulationJointPreparationError::Numeric {
        dense_ordinal: input.dense_ordinal,
        joint: input.joint,
        source,
    })?;
    Ok(SimulationPreparedJointJacobian {
        joint: input.joint,
        body_a: input.body_a.observation(),
        body_b: input.body_b.observation(),
        active_ordinal: input.active_ordinal,
        dense_ordinal: input.dense_ordinal,
        record: SimulationJointJacobian::from_inert_rows_with_bindings(
            &inert.record.rows,
            bindings.solver_delta_a,
            bindings.solver_delta_b,
            bindings.joint,
        ),
    })
}

#[allow(
    clippy::similar_names,
    reason = "source-shaped body A/body B slot roles must remain explicit"
)]
fn try_record_bindings(
    input: &SimulationStagedJointBuildInput,
) -> Result<SimulationJointRecordBindings, (SimulationJointPreparedBindingKind, u32)> {
    let body_a_slot = input.body_a.observation().identity().id().index();
    let body_b_slot = input.body_b.observation().identity().id().index();
    let joint_slot = input.joint.id().index();
    Ok(SimulationJointRecordBindings {
        solver_delta_a: checked_slot_token(body_a_slot)
            .ok_or((SimulationJointPreparedBindingKind::BodyA, body_a_slot))?,
        solver_delta_b: checked_slot_token(body_b_slot)
            .ok_or((SimulationJointPreparedBindingKind::BodyB, body_b_slot))?,
        joint: checked_slot_token(joint_slot)
            .ok_or((SimulationJointPreparedBindingKind::Joint, joint_slot))?,
    })
}

fn try_joint_island_label<E>(
    source: SimulationJointIslandMapSource<'_>,
    active_ordinal: usize,
    joint: SimulationJointIdentity,
    endpoint: SimulationJointEndpoint,
    body: SimulationRigidBodyObservation,
) -> Result<i8, SimulationJointPreparationError<E>> {
    let SimulationJointIslandMapSource::Present(map) = source else {
        return Ok(0);
    };
    let Some((mapped_body, raw_label)) = map.assignment(body) else {
        return Err(SimulationJointPreparationError::MissingIslandLabel {
            active_ordinal,
            joint,
            endpoint,
            body,
        });
    };
    if mapped_body != body {
        return Err(
            SimulationJointPreparationError::IslandLabelOccurrenceMismatch {
                active_ordinal,
                joint,
                endpoint,
                expected_body: body,
                mapped_body,
            },
        );
    }
    Ok(raw_label)
}

fn try_selected_joint_island<E>(
    active_ordinal: usize,
    joint: SimulationJointIdentity,
    raw_body_b: i8,
    raw_body_a: i8,
) -> Result<usize, SimulationJointPreparationError<E>> {
    let selected = if raw_body_b == -1 {
        raw_body_a
    } else {
        raw_body_b
    };
    let Ok(island) = usize::try_from(selected) else {
        return Err(SimulationJointPreparationError::InvalidSelectedIsland {
            active_ordinal,
            joint,
            raw_body_b,
            raw_body_a,
            selected,
        });
    };
    if island >= SIMULATION_ISLAND_CAPACITY {
        return Err(SimulationJointPreparationError::InvalidSelectedIsland {
            active_ordinal,
            joint,
            raw_body_b,
            raw_body_a,
            selected,
        });
    }
    Ok(island)
}

const fn checked_slot_token(slot: u32) -> Option<NonZeroU32> {
    match slot.checked_add(1) {
        Some(value) => NonZeroU32::new(value),
        None => None,
    }
}

fn joint_build_body(
    body: SimulationRigidBodyJointBuildSnapshot,
    solver_delta: NonZeroU32,
) -> SimulationJointBuildBody {
    let xyz0 = |value: [f32; 3]| [value[0], value[1], value[2], 0.0];
    SimulationJointBuildBody {
        stage1: SimulationJointBuildStage1Body {
            quaternion: body.quaternion,
            center_of_mass_and_rf_word: [
                body.center_of_mass[0],
                body.center_of_mass[1],
                body.center_of_mass[2],
                f32::from_bits(solver_delta.get()),
            ],
            velocity: xyz0(body.velocity),
            omega: xyz0(body.omega),
            right: xyz0(body.right),
            up: xyz0(body.up),
            at: xyz0(body.at),
            force: xyz0(body.force),
            torque: xyz0(body.torque),
        },
        full_inertia_and_inverse_mass: [
            body.inverse_inertia_full[0],
            body.inverse_inertia_full[1],
            body.inverse_inertia_full[2],
            body.inverse_mass,
        ],
        split_inertia_and_state: [
            body.inverse_inertia_split[0],
            body.inverse_inertia_split[1],
            body.inverse_inertia_split[2],
            f32::from_bits(body.state),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        simulation_joint_pool::SimulationJointHandle,
        simulation_rigid_body_pool::{
            SimulationInertia, SimulationRigidBodyHandle, SimulationRigidBodyPool,
        },
    };
    use std::{cell::RefCell, rc::Rc, sync::Arc};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct InjectedFailure;

    #[derive(Default)]
    struct CountingBackend {
        calls: usize,
        fail_at: Option<usize>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TraceEvent {
        Sum(SimulationJointSumSite, [u32; 3], [u32; 3]),
        Reciprocal(SimulationJointReciprocalSite, [u32; 4]),
        ReciprocalSqrt(SimulationJointReciprocalSqrtSite, [u32; 4]),
    }

    #[derive(Default)]
    struct TraceBackend {
        events: Vec<TraceEvent>,
    }

    impl SimulationJointNumericBackend for TraceBackend {
        type Error = Infallible;

        fn sum3(
            &mut self,
            site: SimulationJointSumSite,
            left: [f32; 3],
            right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            self.events.push(TraceEvent::Sum(
                site,
                left.map(f32::to_bits),
                right.map(f32::to_bits),
            ));
            Ok(left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2])))
        }

        fn reciprocal_estimate(
            &mut self,
            site: SimulationJointReciprocalSite,
            input: SimulationJointVector,
        ) -> Result<SimulationJointVector, Self::Error> {
            self.events
                .push(TraceEvent::Reciprocal(site, input.map(f32::to_bits)));
            Ok(input.map(f32::recip))
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            site: SimulationJointReciprocalSqrtSite,
            input: SimulationJointVector,
        ) -> Result<SimulationJointVector, Self::Error> {
            self.events
                .push(TraceEvent::ReciprocalSqrt(site, input.map(f32::to_bits)));
            Ok(input.map(|lane| lane.sqrt().recip()))
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum PreparationOrderEvent {
        ValidationReserved,
        StagingReserved,
        RecordsReserved,
        IslandReserved,
        NumericCallback,
    }

    struct PreparationOrderBackend {
        events: Rc<RefCell<Vec<PreparationOrderEvent>>>,
    }

    impl PreparationOrderBackend {
        fn record_callback(&self) {
            self.events
                .borrow_mut()
                .push(PreparationOrderEvent::NumericCallback);
        }
    }

    impl SimulationJointNumericBackend for PreparationOrderBackend {
        type Error = Infallible;

        fn sum3(
            &mut self,
            _site: SimulationJointSumSite,
            left: [f32; 3],
            right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            self.record_callback();
            Ok(left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2])))
        }

        fn reciprocal_estimate(
            &mut self,
            _site: SimulationJointReciprocalSite,
            input: SimulationJointVector,
        ) -> Result<SimulationJointVector, Self::Error> {
            self.record_callback();
            Ok(input.map(f32::recip))
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            _site: SimulationJointReciprocalSqrtSite,
            input: SimulationJointVector,
        ) -> Result<SimulationJointVector, Self::Error> {
            self.record_callback();
            Ok(input.map(|lane| lane.sqrt().recip()))
        }
    }

    impl CountingBackend {
        fn before_call(&mut self) -> Result<(), InjectedFailure> {
            let ordinal = self.calls;
            self.calls += 1;
            if self.fail_at == Some(ordinal) {
                Err(InjectedFailure)
            } else {
                Ok(())
            }
        }
    }

    impl SimulationJointNumericBackend for CountingBackend {
        type Error = InjectedFailure;

        fn sum3(
            &mut self,
            _site: SimulationJointSumSite,
            left: [f32; 3],
            right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            self.before_call()?;
            Ok(left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2])))
        }

        fn reciprocal_estimate(
            &mut self,
            _site: SimulationJointReciprocalSite,
            input: SimulationJointVector,
        ) -> Result<SimulationJointVector, Self::Error> {
            self.before_call()?;
            Ok(input.map(f32::recip))
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            _site: SimulationJointReciprocalSqrtSite,
            input: SimulationJointVector,
        ) -> Result<SimulationJointVector, Self::Error> {
            self.before_call()?;
            Ok(input.map(|lane| lane.sqrt().recip()))
        }
    }

    fn identity_matrix() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    fn frames() -> Arc<SimulationJointFrames> {
        Arc::new(SimulationJointFrames {
            quaternion_a: [0.17, -0.23, 0.31, 0.89],
            position_a: [0.41, -0.27, 0.13, 9.0],
            quaternion_b: [-0.19, 0.29, 0.11, 0.91],
            position_b: [-0.37, 0.22, 0.46, -8.0],
            quaternion_l: [0.09, -0.14, 0.21, 0.96],
        })
    }

    fn limits() -> Arc<SimulationJointLimits> {
        Arc::new(SimulationJointLimits {
            p_prism: [-0.7, -0.5, -0.9, 13.0],
            v_prism: [0.8, 0.65, 1.1, -14.0],
            v_twist: 0.15,
            v_swing: -0.07,
            swing_a: 0.72,
            twist_a: 0.63,
            swing_c: 0.37,
            twist_c: -0.24,
            raw_swing_type: 1,
            raw_twist_type: 1,
        })
    }

    fn add_body(pool: &mut SimulationRigidBodyPool, raw_state: u32) -> SimulationRigidBodyHandle {
        let inertia = Arc::new(SimulationInertia::native_default());
        pool.try_add_rigid_body(identity_matrix(), Some(&inertia), raw_state)
            .expect("body add")
            .expect("body capacity")
    }

    fn add_joint(
        pool: &mut ExactJointPool,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
    ) -> SimulationJointHandle {
        pool.try_add_joint(parent, child, &frames(), &limits())
            .expect("Joint add")
            .expect("Joint capacity")
    }

    fn solver_delta_bits(
        pool: &SimulationRigidBodyPool,
        body: SimulationRigidBodyObservation,
    ) -> [[u32; 4]; 4] {
        pool.try_with_solver_delta_row(body, |rows| rows.map(|row| row.map(f32::to_bits)))
            .expect("live solver-delta rows")
    }

    fn after_one_phase_identity_issue(
        mut snapshot: crate::simulation_rigid_body_pool::SimulationRigidBodyPoolSnapshot,
    ) -> crate::simulation_rigid_body_pool::SimulationRigidBodyPoolSnapshot {
        snapshot.next_dynamic_update_phase_identity = snapshot
            .next_dynamic_update_phase_identity
            .and_then(|identity| identity.checked_add(1));
        snapshot
    }

    fn joint_fixture(
        parent_state: u32,
        child_state: u32,
    ) -> (
        SimulationRigidBodyPool,
        ExactJointPool,
        SimulationRigidBodyHandle,
        SimulationRigidBodyHandle,
        SimulationJointHandle,
    ) {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let parent = add_body(&mut bodies, parent_state);
        let child = add_body(&mut bodies, child_state);
        let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
        let joint = add_joint(&mut joints, parent, child);
        (bodies, joints, parent, child, joint)
    }

    fn fnv64(rows: &[[u32; 4]; 24]) -> u64 {
        rows.iter()
            .copied()
            .flatten()
            .fold(0xcbf2_9ce4_8422_2325, |hash, word| {
                word.to_be_bytes().into_iter().fold(hash, |inner, byte| {
                    (inner ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
                })
            })
    }

    #[test]
    fn mutable_normal_limits_reach_actual_build_with_current_coefficients() {
        use crate::{
            simulation_joint_pool::SimulationMutableJointLimits,
            skeleton_normal_joint_limits::{
                SkeletonJointLimitBinding, try_set_normal_joint_limits,
            },
        };
        let (mut bodies, _unused, parent, child, _) = joint_fixture(4, 4);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let mut joints = ExactJointPool::try_new(1).unwrap();
        let owner = SimulationMutableJointLimits::new(*limits());
        let handle = joints
            .try_add_joint_mutable_limits(parent, child, &frames(), &owner)
            .unwrap()
            .unwrap();
        let phase = bodies.try_prepare_dynamic_update_phase().unwrap();
        let before = try_prepare_active_joint_phase(&joints, &phase)
            .unwrap()
            .records[0]
            .record
            .test_words();

        let (mut hierarchy, mut parameters, pose) = crate::skeleton_data_tests::fixture();
        hierarchy.parents.fill(-1);
        hierarchy.parents[1] = 23;
        parameters.bones[1].swing = 2.0;
        parameters.bones[1].twist = 3.0;
        let data = crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap();
        let (_, mut attributes) = crate::skeleton_body_definition::tests::inputs();
        attributes.joints[0].swing_angle = 0.25;
        attributes.joints[0].twist_angle = 0.5;
        attributes.joints[0].ball_joint = 1; // Not a normal-limit setter gate.
        try_set_normal_joint_limits(
            &data,
            &attributes,
            &mut joints,
            &[SkeletonJointLimitBinding {
                joint: handle,
                limits: owner.clone(),
            }],
            &mut |angle| Ok::<_, Infallible>(angle.cos()),
        )
        .unwrap();

        // Independent setter arithmetic oracle, then the already-audited
        // immutable Build path. This tests new storage plumbing, not a new
        // claim that two copies of Build independently prove its numerics.
        let expected = SimulationJointLimits {
            swing_a: 0.5,
            swing_c: 0.5_f32.cos(),
            twist_a: 1.5,
            twist_c: 1.5_f32.cos(),
            ..*limits()
        };
        assert_eq!(owner.try_read().unwrap(), expected);
        let mut reference = ExactJointPool::try_new(1).unwrap();
        reference
            .try_add_joint(parent, child, &frames(), &Arc::new(expected))
            .unwrap()
            .unwrap();
        let actual = try_prepare_active_joint_phase(&joints, &phase).unwrap();
        let reference = try_prepare_active_joint_phase(&reference, &phase).unwrap();
        assert_eq!(
            actual.records[0].record.test_words(),
            reference.records[0].record.test_words()
        );
        assert_ne!(
            actual.records[0].record.test_words(),
            before,
            "normal limits must affect the actual compiled record"
        );
    }

    #[test]
    fn actual_preparation_skips_filtered_poison_and_refuses_selected_poison_before_numerics() {
        use crate::simulation_joint_pool::{
            SimulationMutableJointLimits, mutable_limits_tests::poison,
        };
        for state in [1, 4] {
            let (mut bodies, _unused, parent, child, _) = joint_fixture(state, state);
            bodies.begin_dynamic_update_frame(1.0 / 60.0);
            let mut joints = ExactJointPool::try_new(1).unwrap();
            let owner = SimulationMutableJointLimits::new(*limits());
            joints
                .try_add_joint_mutable_limits(parent, child, &frames(), &owner)
                .unwrap()
                .unwrap();
            poison(&owner);
            let before = joints.snapshot();
            let before_a = solver_delta_bits(&bodies, child.observation());
            let before_b = solver_delta_bits(&bodies, parent.observation());
            let phase = bodies.try_prepare_dynamic_update_phase().unwrap();
            let mut numeric = CountingBackend::default();
            let result = try_prepare_active_joint_phase_with_backend(&joints, &phase, &mut numeric);
            if state == 1 {
                assert_eq!(result.unwrap().record_count(), 0);
            } else {
                assert!(matches!(
                    result,
                    Err(SimulationJointPreparationError::Pool(
                        SimulationJointPoolError::LimitsLockPoisoned
                    ))
                ));
            }
            assert_eq!(numeric.calls, 0);
            drop(phase);
            assert_eq!(joints.snapshot(), before);
            assert_eq!(solver_delta_bits(&bodies, child.observation()), before_a);
            assert_eq!(solver_delta_bits(&bodies, parent.observation()), before_b);
        }
    }

    #[test]
    fn selected_island_uses_only_exact_minus_one_as_the_body_b_sentinel() {
        let (_, _joints, _, _, joint) = joint_fixture(4, 4);
        let identity = joint.identity();
        for (raw_body_b, raw_body_a, expected) in
            [(2, 7, 2), (-1, 7, 7), (0, -1, 0), (9, -128, 9), (2, -2, 2)]
        {
            assert_eq!(
                try_selected_joint_island::<Infallible>(0, identity, raw_body_b, raw_body_a),
                Ok(expected)
            );
        }
        for (raw_body_b, raw_body_a, selected) in [
            (-2, 7, -2),
            (-128, 7, -128),
            (10, 0, 10),
            (127, 0, 127),
            (-1, -2, -2),
            (-1, 10, 10),
            (-1, -1, -1),
        ] {
            assert!(matches!(
                try_selected_joint_island::<Infallible>(
                    0,
                    identity,
                    raw_body_b,
                    raw_body_a
                ),
                Err(SimulationJointPreparationError::InvalidSelectedIsland {
                    active_ordinal: 0,
                    joint: failed_joint,
                    raw_body_b: failed_b,
                    raw_body_a: failed_a,
                    selected: failed_selected,
                }) if failed_joint == identity
                    && failed_b == raw_body_b
                    && failed_a == raw_body_a
                    && failed_selected == selected
            ));
        }
    }

    #[test]
    fn island_map_authority_and_frame_are_checked_even_for_an_empty_joint_pool() {
        let mut bodies = SimulationRigidBodyPool::try_new(0).expect("body pool");
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("started frame");
        let joints = ExactJointPool::try_new(0).expect("Joint pool");

        let mut foreign_bodies = SimulationRigidBodyPool::try_new(0).expect("foreign body pool");
        foreign_bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let foreign_frame = foreign_bodies.frame_identity().expect("foreign frame");
        let foreign_map =
            SimulationRigidBodyIslandMapSnapshot::try_new(&foreign_bodies, foreign_frame, [])
                .expect("foreign map");
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = CountingBackend::default();
        let Err(foreign) = try_prepare_active_joint_phase_with_island_map_and_backend(
            &joints,
            &phase,
            SimulationJointIslandMapSource::Present(&foreign_map),
            &mut backend,
        ) else {
            panic!("foreign island-map authority must fail");
        };
        assert_eq!(
            foreign,
            SimulationJointPreparationError::IslandMapBodyPoolMismatch
        );
        assert_eq!(backend.calls, 0);

        let old_map = SimulationRigidBodyIslandMapSnapshot::try_new(&bodies, frame, [])
            .expect("old-frame map");
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("new body phase");
        let Err(stale_frame) = try_prepare_active_joint_phase_with_island_map_and_backend(
            &joints,
            &phase,
            SimulationJointIslandMapSource::Present(&old_map),
            &mut backend,
        ) else {
            panic!("stale island-map frame must fail");
        };
        assert_eq!(
            stale_frame,
            SimulationJointPreparationError::IslandMapFrameMismatch
        );
        assert_eq!(backend.calls, 0);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one literal mutation transcript keeps active, dense, binding, and island order correlated"
    )]
    fn active_order_admission_is_dense_and_installs_checked_tokens() {
        let mut bodies = SimulationRigidBodyPool::try_new(4).expect("body pool");
        let active_a = add_body(&mut bodies, 4);
        let active_b = add_body(&mut bodies, 4);
        let static_a = add_body(&mut bodies, 1);
        let static_b = add_body(&mut bodies, 1);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);

        let mut joints = ExactJointPool::try_new(3).expect("Joint pool");
        let first = add_joint(&mut joints, active_a, active_b);
        let removed = add_joint(&mut joints, active_b, active_a);
        let rejected = add_joint(&mut joints, static_a, static_b);
        joints.try_remove_joint(removed).expect("remove middle");
        let reused = add_joint(&mut joints, active_b, active_a);
        assert_eq!(reused.id(), removed.id());

        let frame = bodies.frame_identity().expect("started frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [(active_a.observation(), 2), (active_b.observation(), -1)],
        )
        .expect("island map");

        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = SimulationJointHostNumericBackend;
        let mut reads = Vec::new();
        let Ok(prepared) =
            try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Present(&island_map),
                &mut backend,
                |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
                |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
                |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
                |_, entries, capacity| entries.try_reserve_exact(capacity).map_err(|_| ()),
                |kind, active_ordinal, dense_ordinal| {
                    reads.push((kind, active_ordinal, dense_ordinal));
                },
            )
        else {
            panic!("valid Joint phase must prepare");
        };
        assert_eq!(prepared.records.len(), 2);
        assert_eq!(
            prepared
                .records
                .iter()
                .map(|record| (
                    record.joint.id(),
                    record.active_ordinal,
                    record.dense_ordinal
                ))
                .collect::<Vec<_>>(),
            [(first.id(), 0, 0), (reused.id(), 2, 1)]
        );
        assert_eq!(rejected.id().index(), 2);
        assert_eq!(prepared.records[0].record.binding_words(), [2, 1, 1]);
        assert_eq!(prepared.records[1].record.binding_words(), [1, 2, 2]);
        assert_eq!(prepared.records[0].body_a, active_b.observation());
        assert_eq!(prepared.records[0].body_b, active_a.observation());
        assert_eq!(prepared.records[1].body_a, active_a.observation());
        assert_eq!(prepared.records[1].body_b, active_b.observation());
        assert_eq!(prepared.source.joint_pool, joints.authority());
        assert_eq!(prepared.source.body_pool, phase.body_pool_authority());
        assert_eq!(prepared.source.frame, phase.frame_identity());
        assert_eq!(prepared.joint_pool.authority(), joints.authority());
        assert!(
            prepared
                .islands
                .iter()
                .enumerate()
                .all(|(island, entries)| island == 2 || entries.is_empty())
        );
        assert_eq!(
            prepared.islands[2]
                .iter()
                .map(|entry| (
                    entry.dense_ordinal,
                    entry.joint.id(),
                    entry.body_a,
                    entry.body_b,
                ))
                .collect::<Vec<_>>(),
            [
                (
                    0,
                    first.id(),
                    active_b.observation(),
                    active_a.observation()
                ),
                (
                    1,
                    reused.id(),
                    active_a.observation(),
                    active_b.observation()
                ),
            ]
        );
        assert_eq!(
            reads
                .iter()
                .copied()
                .filter(|(_, active_ordinal, _)| *active_ordinal == 1)
                .collect::<Vec<_>>(),
            [
                (SimulationJointPreparationReadKind::StateB, 1, None),
                (SimulationJointPreparationReadKind::StateA, 1, None),
            ]
        );
        assert_eq!(
            reads
                .iter()
                .copied()
                .filter(|(kind, _, _)| {
                    matches!(
                        kind,
                        SimulationJointPreparationReadKind::DenseOrdinal
                            | SimulationJointPreparationReadKind::BindingTokens
                    )
                })
                .collect::<Vec<_>>(),
            [
                (SimulationJointPreparationReadKind::DenseOrdinal, 0, Some(0)),
                (
                    SimulationJointPreparationReadKind::BindingTokens,
                    0,
                    Some(0)
                ),
                (SimulationJointPreparationReadKind::DenseOrdinal, 2, Some(1)),
                (
                    SimulationJointPreparationReadKind::BindingTokens,
                    2,
                    Some(1)
                ),
            ]
        );
    }

    #[test]
    fn missing_body_b_island_label_stops_before_body_a_label_access() {
        let (mut bodies, joints, parent, child, joint) = joint_fixture(4, 4);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("started frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [(child.observation(), 7)],
        )
        .expect("partial island map");
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = CountingBackend::default();
        let mut reads = Vec::new();
        let Err(failure) =
            try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Present(&island_map),
                &mut backend,
                |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
                |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
                |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
                |_, entries, capacity| entries.try_reserve_exact(capacity).map_err(|_| ()),
                |kind, _, _| reads.push(kind),
            )
        else {
            panic!("missing body B label must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::MissingIslandLabel {
                active_ordinal: 0,
                joint: failed_joint,
                endpoint: SimulationJointEndpoint::Parent,
                body,
            } if failed_joint == joint.identity() && body == parent.observation()
        ));
        assert_eq!(
            reads,
            [
                SimulationJointPreparationReadKind::StateB,
                SimulationJointPreparationReadKind::StateA,
                SimulationJointPreparationReadKind::IslandLabelB,
            ]
        );
        assert_eq!(backend.calls, 0);
    }

    #[test]
    fn body_a_island_label_is_required_even_when_body_b_would_win() {
        let (mut bodies, joints, parent, child, joint) = joint_fixture(4, 4);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("started frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [(parent.observation(), 2)],
        )
        .expect("partial island map");
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = CountingBackend::default();
        let mut reads = Vec::new();
        let Err(failure) =
            try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Present(&island_map),
                &mut backend,
                |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
                |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
                |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
                |_, entries, capacity| entries.try_reserve_exact(capacity).map_err(|_| ()),
                |kind, _, _| reads.push(kind),
            )
        else {
            panic!("missing body A label must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::MissingIslandLabel {
                active_ordinal: 0,
                joint: failed_joint,
                endpoint: SimulationJointEndpoint::Child,
                body,
            } if failed_joint == joint.identity() && body == child.observation()
        ));
        assert_eq!(
            reads,
            [
                SimulationJointPreparationReadKind::StateB,
                SimulationJointPreparationReadKind::StateA,
                SimulationJointPreparationReadKind::IslandLabelB,
                SimulationJointPreparationReadKind::IslandLabelA,
            ]
        );
        assert_eq!(backend.calls, 0);
    }

    #[test]
    fn stale_body_b_island_occurrence_is_rejected_before_body_a_label_access() {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let stale_parent = add_body(&mut bodies, 4);
        let child = add_body(&mut bodies, 4);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("started frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [(stale_parent.observation(), 2), (child.observation(), 7)],
        )
        .expect("stale island map");
        bodies
            .try_remove_rigid_body(stale_parent)
            .expect("remove mapped parent");
        let parent = add_body(&mut bodies, 4);
        assert_eq!(parent.id(), stale_parent.id());
        let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
        let joint = add_joint(&mut joints, parent, child);
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = CountingBackend::default();
        let mut reads = Vec::new();
        let Err(failure) =
            try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Present(&island_map),
                &mut backend,
                |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
                |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
                |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
                |_, entries, capacity| entries.try_reserve_exact(capacity).map_err(|_| ()),
                |kind, _, _| reads.push(kind),
            )
        else {
            panic!("stale body B island occurrence must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::IslandLabelOccurrenceMismatch {
                active_ordinal: 0,
                joint: failed_joint,
                endpoint: SimulationJointEndpoint::Parent,
                expected_body,
                mapped_body,
            } if failed_joint == joint.identity()
                && expected_body == parent.observation()
                && mapped_body == stale_parent.observation()
        ));
        assert_eq!(
            reads,
            [
                SimulationJointPreparationReadKind::StateB,
                SimulationJointPreparationReadKind::StateA,
                SimulationJointPreparationReadKind::IslandLabelB,
            ]
        );
        assert_eq!(backend.calls, 0);
    }

    #[test]
    fn stale_body_a_island_occurrence_is_rejected_even_when_body_b_would_win() {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let parent = add_body(&mut bodies, 4);
        let stale_child = add_body(&mut bodies, 4);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("started frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [(parent.observation(), 2), (stale_child.observation(), -128)],
        )
        .expect("stale island map");
        bodies
            .try_remove_rigid_body(stale_child)
            .expect("remove mapped child");
        let child = add_body(&mut bodies, 4);
        assert_eq!(child.id(), stale_child.id());
        let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
        let joint = add_joint(&mut joints, parent, child);
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = CountingBackend::default();
        let mut reads = Vec::new();
        let Err(failure) =
            try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Present(&island_map),
                &mut backend,
                |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
                |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
                |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
                |_, entries, capacity| entries.try_reserve_exact(capacity).map_err(|_| ()),
                |kind, _, _| reads.push(kind),
            )
        else {
            panic!("stale body A island occurrence must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::IslandLabelOccurrenceMismatch {
                active_ordinal: 0,
                joint: failed_joint,
                endpoint: SimulationJointEndpoint::Child,
                expected_body,
                mapped_body,
            } if failed_joint == joint.identity()
                && expected_body == child.observation()
                && mapped_body == stale_child.observation()
        ));
        assert_eq!(
            reads,
            [
                SimulationJointPreparationReadKind::StateB,
                SimulationJointPreparationReadKind::StateA,
                SimulationJointPreparationReadKind::IslandLabelB,
                SimulationJointPreparationReadKind::IslandLabelA,
            ]
        );
        assert_eq!(backend.calls, 0);
    }

    #[test]
    fn mapped_body_a_is_read_but_only_the_selected_label_is_range_checked() {
        for (raw_body_b, raw_body_a, expected_island) in [
            (2, 7, 2_usize),
            (-1, 7, 7),
            (-1, 9, 9),
            (0, -1, 0),
            (9, -128, 9),
            (2, -2, 2),
        ] {
            let (mut bodies, joints, parent, child, _) = joint_fixture(4, 4);
            bodies.begin_dynamic_update_frame(1.0 / 60.0);
            let frame = bodies.frame_identity().expect("started frame");
            let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
                &bodies,
                frame,
                [
                    (parent.observation(), raw_body_b),
                    (child.observation(), raw_body_a),
                ],
            )
            .expect("island map");
            let phase = bodies
                .try_prepare_dynamic_update_phase()
                .expect("body phase");
            let mut backend = CountingBackend::default();
            let Ok(prepared) = try_prepare_active_joint_phase_with_island_map_and_backend(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Present(&island_map),
                &mut backend,
            ) else {
                panic!("valid selected label must prepare");
            };
            assert_eq!(prepared.records.len(), 1);
            assert_eq!(prepared.islands[expected_island].len(), 1);
            assert!(
                prepared
                    .islands
                    .iter()
                    .enumerate()
                    .all(|(island, entries)| { island == expected_island || entries.is_empty() })
            );
            assert_eq!(backend.calls, 17);
        }
    }

    #[test]
    fn invalid_selected_island_labels_fail_before_numeric_work() {
        for (raw_body_b, raw_body_a, selected) in [
            (-2, 7, -2),
            (-128, 7, -128),
            (10, 0, 10),
            (127, 0, 127),
            (-1, -2, -2),
            (-1, 10, 10),
            (-1, -1, -1),
        ] {
            let (mut bodies, joints, parent, child, joint) = joint_fixture(4, 4);
            bodies.begin_dynamic_update_frame(1.0 / 60.0);
            let frame = bodies.frame_identity().expect("started frame");
            let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
                &bodies,
                frame,
                [
                    (parent.observation(), raw_body_b),
                    (child.observation(), raw_body_a),
                ],
            )
            .expect("island map");
            let phase = bodies
                .try_prepare_dynamic_update_phase()
                .expect("body phase");
            let mut backend = CountingBackend::default();
            let Err(failure) = try_prepare_active_joint_phase_with_island_map_and_backend(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Present(&island_map),
                &mut backend,
            ) else {
                panic!("invalid selected island must fail");
            };
            assert!(matches!(
                failure,
                SimulationJointPreparationError::InvalidSelectedIsland {
                    active_ordinal: 0,
                    joint: failed_joint,
                    raw_body_b: failed_b,
                    raw_body_a: failed_a,
                    selected: failed_selected,
                } if failed_joint == joint.identity()
                    && failed_b == raw_body_b
                    && failed_a == raw_body_a
                    && failed_selected == selected
            ));
            assert_eq!(backend.calls, 0);
        }
    }

    #[test]
    fn null_island_map_routes_to_zero_without_label_reads() {
        let (mut bodies, joints, _, _, _) = joint_fixture(4, 4);
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = CountingBackend::default();
        let mut reads = Vec::new();
        let frame = phase.frame_identity();
        let Ok(prepared) =
            try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Null { frame },
                &mut backend,
                |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
                |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
                |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
                |_, entries, capacity| entries.try_reserve_exact(capacity).map_err(|_| ()),
                |kind, _, _| reads.push(kind),
            )
        else {
            panic!("null island map must prepare");
        };
        assert_eq!(prepared.islands[0].len(), 1);
        assert!(prepared.islands[1..].iter().all(Vec::is_empty));
        assert!(!reads.iter().any(|kind| matches!(
            kind,
            SimulationJointPreparationReadKind::IslandLabelB
                | SimulationJointPreparationReadKind::IslandLabelA
        )));
        assert_eq!(backend.calls, 17);
    }

    #[test]
    fn checked_joint_island_sweep_matches_the_pure_record_and_rf_result() {
        let (mut bodies, joints, parent, child, _) = joint_fixture(4, 4);
        let parent_rf = [
            [0.11, -0.17, 0.23, 0.29],
            [1.0, 2.0, 3.0, 4.0],
            [-0.31, 0.37, -0.41, 0.43],
            [5.0, 6.0, 7.0, 8.0],
        ];
        let child_rf = [
            [-0.13, 0.19, -0.27, 0.33],
            [-1.0, -2.0, -3.0, -4.0],
            [0.35, -0.39, 0.45, -0.49],
            [-5.0, -6.0, -7.0, -8.0],
        ];
        bodies
            .try_with_solver_delta_row_mut(parent.observation(), |row| *row = parent_rf)
            .expect("seed parent RF");
        bodies
            .try_with_solver_delta_row_mut(child.observation(), |row| *row = child_rf)
            .expect("seed child RF");
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("started frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [(parent.observation(), 2), (child.observation(), 7)],
        )
        .expect("island map");
        let mut phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = SimulationJointHostNumericBackend;
        let Ok(mut prepared) = try_prepare_active_joint_phase_with_island_map_and_backend(
            &joints,
            &phase,
            SimulationJointIslandMapSource::Present(&island_map),
            &mut backend,
        ) else {
            panic!("Joint batch must prepare");
        };
        let before_empty = prepared.records[0].record.test_words();
        try_solve_prepared_joint_horus_island_sweep(&mut prepared, 1, &mut phase)
            .expect("empty island sweep");
        assert_eq!(prepared.records[0].record.test_words(), before_empty);

        let mut expected_record = prepared.records[0].record;
        let expected_first =
            solve_joint_horus_record_host(&mut expected_record, child_rf, parent_rf);
        try_solve_prepared_joint_horus_island_sweep(&mut prepared, 2, &mut phase)
            .expect("first selected island sweep");
        let expected_second = solve_joint_horus_record_host(
            &mut expected_record,
            expected_first.solver_delta_a,
            expected_first.solver_delta_b,
        );
        try_solve_prepared_joint_horus_island_sweep(&mut prepared, 2, &mut phase)
            .expect("second selected island sweep through the same body phase");
        assert_eq!(
            prepared.records[0].record.test_words(),
            expected_record.test_words()
        );
        drop(prepared);
        drop(phase);
        assert_eq!(
            bodies
                .try_with_solver_delta_row(child.observation(), |row| *row)
                .expect("child RF"),
            expected_second.solver_delta_a
        );
        assert_eq!(
            bodies
                .try_with_solver_delta_row(parent.observation(), |row| *row)
                .expect("parent RF"),
            expected_second.solver_delta_b
        );
        assert_eq!(
            expected_second.solver_delta_a[1].map(f32::to_bits),
            child_rf[1].map(f32::to_bits)
        );
        assert_eq!(
            expected_second.solver_delta_a[3].map(f32::to_bits),
            child_rf[3].map(f32::to_bits)
        );
        assert_eq!(
            expected_second.solver_delta_b[1].map(f32::to_bits),
            parent_rf[1].map(f32::to_bits)
        );
        assert_eq!(
            expected_second.solver_delta_b[3].map(f32::to_bits),
            parent_rf[3].map(f32::to_bits)
        );
    }

    #[test]
    fn stale_same_frame_batch_rejects_a_reacquired_phase_before_record_or_rf_effects() {
        let (mut bodies, joints, parent, child, _) = joint_fixture(4, 4);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let numeric_before = bodies.snapshot().slots;
        let first_phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("first body phase");
        let first_phase_identity = first_phase.phase_identity();
        let Ok(mut batch) = try_prepare_active_joint_phase(&joints, &first_phase) else {
            panic!("first phase must prepare a Joint batch");
        };
        first_phase.integrate_ordinary_finite();
        assert_ne!(bodies.snapshot().slots, numeric_before);

        let rf_before = [parent, child].map(|body| solver_delta_bits(&bodies, body.observation()));
        let body_before = after_one_phase_identity_issue(bodies.snapshot());
        let mut second_phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("second body phase in the same frame");
        assert_eq!(batch.source.frame, second_phase.frame_identity());
        assert_eq!(batch.source.body_pool, second_phase.body_pool_authority());
        assert!(first_phase_identity != second_phase.phase_identity());
        let record_before = batch.records[0].record.test_words();

        assert_eq!(
            try_solve_prepared_joint_horus_island_sweep(&mut batch, 0, &mut second_phase),
            Err(SimulationPreparedJointIslandHorusError::PhaseMismatch)
        );
        assert_eq!(batch.records[0].record.test_words(), record_before);
        drop(second_phase);
        drop(batch);
        assert_eq!(
            [parent, child].map(|body| solver_delta_bits(&bodies, body.observation())),
            rf_before
        );
        assert_eq!(bodies.snapshot(), body_before);
    }

    #[test]
    fn real_foreign_pool_and_new_frame_errors_precede_phase_identity_checks() {
        let (mut bodies, joints, _, _, _) = joint_fixture(4, 4);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let first_phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("first body phase");
        let first_frame = first_phase.frame_identity();
        let Ok(mut batch) = try_prepare_active_joint_phase(&joints, &first_phase) else {
            panic!("first phase must prepare a Joint batch");
        };
        drop(first_phase);

        let mut foreign_bodies = SimulationRigidBodyPool::try_new(0).expect("foreign body pool");
        foreign_bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let mut foreign_phase = foreign_bodies
            .try_prepare_dynamic_update_phase()
            .expect("foreign phase");
        assert_eq!(first_frame, foreign_phase.frame_identity());
        assert_eq!(
            try_solve_prepared_joint_horus_island_sweep(&mut batch, 0, &mut foreign_phase),
            Err(SimulationPreparedJointIslandHorusError::BodyPoolSourceMismatch)
        );
        drop(foreign_phase);

        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let mut later_frame_phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("later-frame phase");
        assert_ne!(first_frame, later_frame_phase.frame_identity());
        assert_eq!(
            try_solve_prepared_joint_horus_island_sweep(&mut batch, 0, &mut later_frame_phase),
            Err(SimulationPreparedJointIslandHorusError::FrameMismatch)
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one pre-effect corruption matrix proves the checked batch refuses every detached identity"
    )]
    fn checked_joint_island_sweep_rejects_sources_tables_and_tokens_before_effects() {
        let (mut bodies, joints, parent, child, _) = joint_fixture(4, 4);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let old_frame = bodies.frame_identity().expect("old frame");
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("current frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [(parent.observation(), 2), (child.observation(), 0)],
        )
        .expect("island map");
        let body_before = after_one_phase_identity_issue(bodies.snapshot());
        let rf_before = [parent, child].map(|body| {
            (
                body.observation(),
                solver_delta_bits(&bodies, body.observation()),
            )
        });
        let joint_before = joints.snapshot();
        let foreign_joints = ExactJointPool::try_new(0).expect("foreign Joint pool");
        let foreign_bodies = SimulationRigidBodyPool::try_new(0).expect("foreign body pool");
        let mut phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = SimulationJointHostNumericBackend;
        let Ok(mut prepared) = try_prepare_active_joint_phase_with_island_map_and_backend(
            &joints,
            &phase,
            SimulationJointIslandMapSource::Present(&island_map),
            &mut backend,
        ) else {
            panic!("Joint batch must prepare");
        };
        let record_before = prepared.records[0].record;

        assert_eq!(
            try_solve_prepared_joint_horus_island_sweep(
                &mut prepared,
                SIMULATION_ISLAND_CAPACITY,
                &mut phase,
            ),
            Err(SimulationPreparedJointIslandHorusError::IslandOutOfRange {
                island: SIMULATION_ISLAND_CAPACITY,
            })
        );
        prepared.source.frame = old_frame;
        assert_eq!(
            try_solve_prepared_joint_horus_island_sweep(&mut prepared, 2, &mut phase),
            Err(SimulationPreparedJointIslandHorusError::FrameMismatch)
        );
        prepared.source.frame = frame;
        let joint_source = prepared.source.joint_pool;
        prepared.source.joint_pool = foreign_joints.authority();
        assert_eq!(
            try_solve_prepared_joint_horus_island_sweep(&mut prepared, 2, &mut phase),
            Err(SimulationPreparedJointIslandHorusError::JointPoolSourceMismatch)
        );
        prepared.source.joint_pool = joint_source;
        let body_source = prepared.source.body_pool;
        prepared.source.body_pool = foreign_bodies.authority();
        assert_eq!(
            try_solve_prepared_joint_horus_island_sweep(&mut prepared, 2, &mut phase),
            Err(SimulationPreparedJointIslandHorusError::BodyPoolSourceMismatch)
        );
        prepared.source.body_pool = body_source;

        prepared.islands[2][0].dense_ordinal = u32::MAX;
        assert_eq!(
            try_solve_prepared_joint_horus_island_sweep(&mut prepared, 2, &mut phase),
            Err(
                SimulationPreparedJointIslandHorusError::TableEntryMismatch {
                    island_ordinal: 0,
                    dense_ordinal: u32::MAX,
                }
            )
        );
        prepared.islands[2][0].dense_ordinal = 0;
        prepared.records[0].dense_ordinal = 1;
        assert_eq!(
            try_solve_prepared_joint_horus_island_sweep(&mut prepared, 2, &mut phase),
            Err(
                SimulationPreparedJointIslandHorusError::TableEntryMismatch {
                    island_ordinal: 0,
                    dense_ordinal: 0,
                }
            )
        );
        prepared.records[0].dense_ordinal = 0;

        let original_words = record_before.test_words();
        for (kind, tokens) in [
            (
                SimulationJointPreparedBindingKind::BodyA,
                [3_u32, original_words[1][3], original_words[12][3]],
            ),
            (
                SimulationJointPreparedBindingKind::BodyB,
                [original_words[0][3], 3, original_words[12][3]],
            ),
            (
                SimulationJointPreparedBindingKind::Joint,
                [original_words[0][3], original_words[1][3], 2],
            ),
        ] {
            prepared.records[0].record = SimulationJointJacobian::from_inert_rows_with_bindings(
                &original_words,
                NonZeroU32::new(tokens[0]).expect("A token"),
                NonZeroU32::new(tokens[1]).expect("B token"),
                NonZeroU32::new(tokens[2]).expect("Joint token"),
            );
            assert!(matches!(
                try_solve_prepared_joint_horus_island_sweep(&mut prepared, 2, &mut phase),
                Err(SimulationPreparedJointIslandHorusError::BindingMismatch {
                    island_ordinal: 0,
                    dense_ordinal: 0,
                    kind: failed_kind,
                    ..
                }) if failed_kind == kind
            ));
        }
        prepared.records[0].record = record_before;
        assert_eq!(prepared.records[0].record.test_words(), original_words);
        drop(prepared);
        assert_eq!(bodies.snapshot(), body_before);
        for (body, expected) in rf_before {
            assert_eq!(solver_delta_bits(&bodies, body), expected);
        }
        assert_eq!(joints.snapshot(), joint_before);
    }

    #[test]
    fn jointly_cross_paired_table_and_record_sidecars_fail_pool_revalidation() {
        let mut bodies = SimulationRigidBodyPool::try_new(4).expect("body pool");
        let first_parent = add_body(&mut bodies, 4);
        let first_child = add_body(&mut bodies, 4);
        let second_parent = add_body(&mut bodies, 4);
        let second_child = add_body(&mut bodies, 4);
        let mut joints = ExactJointPool::try_new(2).expect("Joint pool");
        add_joint(&mut joints, first_parent, first_child);
        let second = add_joint(&mut joints, second_parent, second_child);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("started frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [
                (first_parent.observation(), 2),
                (first_child.observation(), 0),
                (second_parent.observation(), 2),
                (second_child.observation(), 0),
            ],
        )
        .expect("island map");
        let body_before = after_one_phase_identity_issue(bodies.snapshot());
        let rf_before = [first_parent, first_child, second_parent, second_child].map(|body| {
            (
                body.observation(),
                solver_delta_bits(&bodies, body.observation()),
            )
        });
        let mut phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = SimulationJointHostNumericBackend;
        let Ok(mut prepared) = try_prepare_active_joint_phase_with_island_map_and_backend(
            &joints,
            &phase,
            SimulationJointIslandMapSource::Present(&island_map),
            &mut backend,
        ) else {
            panic!("Joint batch must prepare");
        };
        let record_before = prepared.records[0].record.test_words();
        prepared.islands[2][0].joint = second.identity();
        prepared.records[0].joint = second.identity();
        assert!(matches!(
            try_solve_prepared_joint_horus_island_sweep(&mut prepared, 2, &mut phase),
            Err(SimulationPreparedJointIslandHorusError::Joint(
                SimulationJointBuildBindingValidationError::EndpointMismatch {
                    ordinal: 0,
                    joint,
                    ..
                }
            )) if joint == second.identity()
        ));
        assert_eq!(prepared.records[0].record.test_words(), record_before);
        drop(prepared);
        assert_eq!(bodies.snapshot(), body_before);
        for (body, expected) in rf_before {
            assert_eq!(solver_delta_bits(&bodies, body), expected);
        }
    }

    #[test]
    fn nonmonotonic_repeated_joint_table_uses_exact_order_and_then_live_rf_rows() {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let parent = add_body(&mut bodies, 4);
        let child = add_body(&mut bodies, 4);
        let mut joints = ExactJointPool::try_new(3).expect("Joint pool");
        add_joint(&mut joints, parent, child);
        add_joint(&mut joints, parent, child);
        add_joint(&mut joints, parent, child);
        let parent_rf = [
            [0.07, -0.11, 0.13, -0.17],
            [1.0, 2.0, 3.0, 4.0],
            [0.19, -0.23, 0.29, -0.31],
            [5.0, 6.0, 7.0, 8.0],
        ];
        let child_rf = [
            [-0.05, 0.09, -0.15, 0.21],
            [-1.0, -2.0, -3.0, -4.0],
            [-0.25, 0.27, -0.33, 0.35],
            [-5.0, -6.0, -7.0, -8.0],
        ];
        bodies
            .try_with_solver_delta_row_mut(parent.observation(), |row| *row = parent_rf)
            .expect("seed parent RF");
        bodies
            .try_with_solver_delta_row_mut(child.observation(), |row| *row = child_rf)
            .expect("seed child RF");
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("started frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [(parent.observation(), 2), (child.observation(), 0)],
        )
        .expect("island map");
        let mut phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = SimulationJointHostNumericBackend;
        let Ok(mut prepared) = try_prepare_active_joint_phase_with_island_map_and_backend(
            &joints,
            &phase,
            SimulationJointIslandMapSource::Present(&island_map),
            &mut backend,
        ) else {
            panic!("Joint batch must prepare");
        };
        let copy_entry =
            |entry: &SimulationPreparedJointIslandEntry| SimulationPreparedJointIslandEntry {
                dense_ordinal: entry.dense_ordinal,
                joint: entry.joint,
                body_a: entry.body_a,
                body_b: entry.body_b,
            };
        let entry_2 = copy_entry(&prepared.islands[2][2]);
        let repeated_entry_2 = copy_entry(&entry_2);
        let entry_0 = copy_entry(&prepared.islands[2][0]);
        prepared.islands[2] = vec![entry_2, entry_0, repeated_entry_2];

        let untouched_record_1 = prepared.records[1].record.test_words();
        let mut expected_record_0 = prepared.records[0].record;
        let mut expected_record_2 = prepared.records[2].record;
        let first = solve_joint_horus_record_host(&mut expected_record_2, child_rf, parent_rf);
        let second = solve_joint_horus_record_host(
            &mut expected_record_0,
            first.solver_delta_a,
            first.solver_delta_b,
        );
        let third = solve_joint_horus_record_host(
            &mut expected_record_2,
            second.solver_delta_a,
            second.solver_delta_b,
        );
        try_solve_prepared_joint_horus_island_sweep(&mut prepared, 2, &mut phase)
            .expect("nonmonotonic repeated sweep");
        assert_eq!(
            prepared.records[0].record.test_words(),
            expected_record_0.test_words()
        );
        assert_eq!(prepared.records[1].record.test_words(), untouched_record_1);
        assert_eq!(
            prepared.records[2].record.test_words(),
            expected_record_2.test_words()
        );
        drop(prepared);
        assert_eq!(
            bodies
                .try_with_solver_delta_row(child.observation(), |row| row
                    .map(|v| v.map(f32::to_bits)))
                .expect("child RF"),
            third.solver_delta_a.map(|v| v.map(f32::to_bits))
        );
        assert_eq!(
            bodies
                .try_with_solver_delta_row(parent.observation(), |row| row
                    .map(|v| v.map(f32::to_bits)))
                .expect("parent RF"),
            third.solver_delta_b.map(|v| v.map(f32::to_bits))
        );
    }

    #[test]
    fn later_joint_binding_mismatch_leaves_the_earlier_record_and_all_rf_inert() {
        let mut bodies = SimulationRigidBodyPool::try_new(4).expect("body pool");
        let first_parent = add_body(&mut bodies, 4);
        let first_child = add_body(&mut bodies, 4);
        let second_parent = add_body(&mut bodies, 4);
        let second_child = add_body(&mut bodies, 4);
        let mut joints = ExactJointPool::try_new(2).expect("Joint pool");
        add_joint(&mut joints, first_parent, first_child);
        let second = add_joint(&mut joints, second_parent, second_child);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("started frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [
                (first_parent.observation(), 2),
                (first_child.observation(), 0),
                (second_parent.observation(), 2),
                (second_child.observation(), 0),
            ],
        )
        .expect("island map");
        let body_before = after_one_phase_identity_issue(bodies.snapshot());
        let rf_before = [first_parent, first_child, second_parent, second_child].map(|body| {
            (
                body.observation(),
                solver_delta_bits(&bodies, body.observation()),
            )
        });
        let mut phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = SimulationJointHostNumericBackend;
        let Ok(mut prepared) = try_prepare_active_joint_phase_with_island_map_and_backend(
            &joints,
            &phase,
            SimulationJointIslandMapSource::Present(&island_map),
            &mut backend,
        ) else {
            panic!("Joint batch must prepare");
        };
        let first_before = prepared.records[0].record.test_words();
        let second_words = prepared.records[1].record.test_words();
        prepared.records[1].record = SimulationJointJacobian::from_inert_rows_with_bindings(
            &second_words,
            NonZeroU32::new(second_words[0][3]).expect("A token"),
            NonZeroU32::new(second_words[1][3]).expect("B token"),
            NonZeroU32::new(3).expect("wrong Joint token"),
        );
        assert!(matches!(
            try_solve_prepared_joint_horus_island_sweep(&mut prepared, 2, &mut phase),
            Err(SimulationPreparedJointIslandHorusError::BindingMismatch {
                island_ordinal: 1,
                dense_ordinal: 1,
                joint,
                kind: SimulationJointPreparedBindingKind::Joint,
                expected: 2,
                actual: 3,
            }) if joint == second.identity()
        ));
        assert_eq!(prepared.records[0].record.test_words(), first_before);
        drop(prepared);
        assert_eq!(bodies.snapshot(), body_before);
        for (body, expected) in rf_before {
            assert_eq!(solver_delta_bits(&bodies, body), expected);
        }
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the literal nonmonotonic ring transcript proves cross-island stable subsequences"
    )]
    fn nonmonotonic_active_ring_builds_stable_dense_island_subsequences() {
        let mut bodies = SimulationRigidBodyPool::try_new(8).expect("body pool");
        let j0_parent = add_body(&mut bodies, 4);
        let j0_child = add_body(&mut bodies, 1);
        let static_parent = add_body(&mut bodies, 1);
        let static_child = add_body(&mut bodies, 1);
        let j3_parent = add_body(&mut bodies, 1);
        let j3_child = add_body(&mut bodies, 4);
        let j4_parent = add_body(&mut bodies, 4);
        let j4_child = add_body(&mut bodies, 1);

        let mut joints = ExactJointPool::try_new(5).expect("Joint pool");
        let j0 = add_joint(&mut joints, j0_parent, j0_child);
        let removed = add_joint(&mut joints, j0_parent, j0_child);
        let j2 = add_joint(&mut joints, static_parent, static_child);
        let j3 = add_joint(&mut joints, j3_parent, j3_child);
        let j4 = add_joint(&mut joints, j4_parent, j4_child);
        joints.try_remove_joint(removed).expect("remove middle");
        let reused = add_joint(&mut joints, j3_parent, j3_child);
        assert_eq!(reused.id(), removed.id());

        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("started frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [
                (j0_parent.observation(), 2),
                (j0_child.observation(), 0),
                (j3_parent.observation(), -1),
                (j3_child.observation(), 1),
                (j4_parent.observation(), 2),
                (j4_child.observation(), 9),
            ],
        )
        .expect("island map");
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = CountingBackend::default();
        let mut reads = Vec::new();
        let Ok(prepared) =
            try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Present(&island_map),
                &mut backend,
                |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
                |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
                |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
                |_, entries, capacity| entries.try_reserve_exact(capacity).map_err(|_| ()),
                |kind, active_ordinal, dense_ordinal| {
                    reads.push((kind, active_ordinal, dense_ordinal));
                },
            )
        else {
            panic!("valid nonmonotonic Joint ring must prepare");
        };
        assert_eq!(
            prepared
                .records
                .iter()
                .map(|record| (
                    record.joint.id(),
                    record.active_ordinal,
                    record.dense_ordinal
                ))
                .collect::<Vec<_>>(),
            [
                (j0.id(), 0, 0),
                (j3.id(), 2, 1),
                (j4.id(), 3, 2),
                (reused.id(), 4, 3),
            ]
        );
        assert_eq!(j2.id().index(), 2);
        assert_eq!(
            prepared.islands[1]
                .iter()
                .map(|entry| (entry.joint.id(), entry.dense_ordinal))
                .collect::<Vec<_>>(),
            [(j3.id(), 1), (reused.id(), 3)]
        );
        assert_eq!(
            prepared.islands[2]
                .iter()
                .map(|entry| (entry.joint.id(), entry.dense_ordinal))
                .collect::<Vec<_>>(),
            [(j0.id(), 0), (j4.id(), 2)]
        );
        assert!(
            prepared
                .islands
                .iter()
                .enumerate()
                .all(|(island, entries)| { matches!(island, 1 | 2) || entries.is_empty() })
        );
        assert_eq!(
            reads
                .iter()
                .copied()
                .filter(|(_, active_ordinal, _)| *active_ordinal == 1)
                .collect::<Vec<_>>(),
            [
                (SimulationJointPreparationReadKind::StateB, 1, None),
                (SimulationJointPreparationReadKind::StateA, 1, None),
            ]
        );
        assert_eq!(backend.calls, 68);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one literal transcript keeps the host zero-W adapter boundary auditable"
    )]
    fn manually_staged_cone_arc_host_adapter_matches_record_and_callback_oracles() {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let parent = add_body(&mut bodies, 4);
        let child = add_body(&mut bodies, 4);
        let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
        let joint = add_joint(&mut joints, parent, child);
        let body_a = SimulationRigidBodyJointBuildSnapshot {
            observation: child.observation(),
            state: 4,
            quaternion: [0.11, -0.07, 0.19, 0.96],
            center_of_mass: [1.2, -0.8, 0.4],
            velocity: [0.31, -0.22, 0.17],
            omega: [0.14, 0.27, -0.19],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            at: [0.0, 0.0, 1.0],
            force: [0.21, -0.16, 0.12],
            torque: [-0.17, 0.08, 0.23],
            inverse_inertia_full: [1.7, 0.13, -0.08],
            inverse_mass: 2.2,
            inverse_inertia_split: [0.09, 1.4, 0.04],
        };
        let body_b = SimulationRigidBodyJointBuildSnapshot {
            observation: parent.observation(),
            state: 4,
            quaternion: [-0.13, 0.21, 0.05, 0.95],
            center_of_mass: [-1.1, 0.6, 1.3],
            velocity: [-0.18, 0.34, -0.29],
            omega: [-0.11, 0.16, 0.24],
            right: [0.87, 0.29, -0.19],
            up: [-0.24, 0.94, 0.23],
            at: [0.25, -0.15, 0.96],
            force: [-0.14, 0.25, -0.06],
            torque: [0.19, -0.21, 0.1],
            inverse_inertia_full: [1.2, -0.09, 0.17],
            inverse_mass: 1.8,
            inverse_inertia_split: [-0.11, 1.05, 0.07],
        };
        let bindings = SimulationJointRecordBindings {
            solver_delta_a: NonZeroU32::new(2).expect("child token"),
            solver_delta_b: NonZeroU32::new(1).expect("parent token"),
            joint: NonZeroU32::new(1).expect("Joint token"),
        };
        let staged = SimulationStagedJointBuildInput {
            island: 0,
            joint: joint.identity(),
            body_a,
            body_b,
            frames: *frames(),
            limits: *limits(),
            timestep: 1.0 / 60.0,
            spy: 7,
            active_ordinal: 0,
            dense_ordinal: 0,
            bindings: Some(bindings),
        };
        let mut backend = TraceBackend::default();
        let prepared = compose_staged_joint_record(&staged, &mut backend)
            .expect("ordinary-finite host backend cannot fail");
        assert_eq!(prepared.record.binding_words(), [2, 1, 1]);
        assert_eq!(fnv64(&prepared.record.test_words()), 0x3e59_e745_0bd5_0ab2);
        assert_eq!(
            backend.events,
            [
                TraceEvent::Sum(
                    SimulationJointSumSite::QuaternionA,
                    [0x3de1_47ae, 0xbd8f_5c29, 0x3e42_8f5c],
                    [0x3e2e_147b, 0xbe6b_851f, 0x3e9e_b852],
                ),
                TraceEvent::Sum(
                    SimulationJointSumSite::QuaternionB,
                    [0xbe05_1eb8, 0x3e57_0a3d, 0x3d4c_cccd],
                    [0xbe42_8f5c, 0x3e94_7ae1, 0x3de1_47ae],
                ),
                TraceEvent::Sum(
                    SimulationJointSumSite::LimitFrameQuaternion,
                    [0xbe05_1eb8, 0x3e57_0a3d, 0x3d4c_cccd],
                    [0x3db8_51ec, 0xbe0f_5c29, 0x3e57_0a3d],
                ),
                TraceEvent::Sum(
                    SimulationJointSumSite::MotionUp,
                    [0xbbda_8d66, 0x3c33_c458, 0xbb97_7610],
                    [0xbeed_00f2, 0x3f60_be20, 0x3d93_0d5a],
                ),
                TraceEvent::Sum(
                    SimulationJointSumSite::MotionRight,
                    [0xbbda_8d66, 0x3c33_c458, 0xbb97_7610],
                    [0x3f5b_a726, 0x3eef_6e14, 0xbe3b_da33],
                ),
                TraceEvent::Sum(
                    SimulationJointSumSite::MotionAt,
                    [0xbbda_8d66, 0x3c33_c458, 0xbb97_7610],
                    [0x3e47_c7e9, 0x3cdf_bd86, 0x3f7a_c486],
                ),
                TraceEvent::Sum(
                    SimulationJointSumSite::PredictedUp,
                    [0xc03e_7cd7, 0x3fdb_1186, 0x3fa9_f7e6],
                    [0xbeed_00f2, 0x3f60_be20, 0x3d93_0d5a],
                ),
                TraceEvent::Sum(
                    SimulationJointSumSite::PredictedRight,
                    [0xc03e_7cd7, 0x3fdb_1186, 0x3fa9_f7e6],
                    [0x3f5b_a726, 0x3eef_6e14, 0xbe3b_da33],
                ),
                TraceEvent::Sum(
                    SimulationJointSumSite::PredictedAt,
                    [0xc03e_7cd7, 0x3fdb_1186, 0x3fa9_f7e6],
                    [0x3e47_c7e9, 0x3cdf_bd86, 0x3f7a_c486],
                ),
                TraceEvent::ReciprocalSqrt(
                    SimulationJointReciprocalSqrtSite::Cone,
                    [0x3f17_6fec; 4],
                ),
                TraceEvent::Reciprocal(SimulationJointReciprocalSite::Cone, [0x3f44_e54e; 4],),
                TraceEvent::Reciprocal(SimulationJointReciprocalSite::TwistArc, [0xbea4_eafc; 4],),
                TraceEvent::Sum(
                    SimulationJointSumSite::AngularUp,
                    [0xbf06_fe20, 0x3f69_13ac, 0xbebd_f2c7],
                    [0xbb85_41ad, 0xbafa_db77, 0x3be9_a771],
                ),
                TraceEvent::Sum(
                    SimulationJointSumSite::AngularRight,
                    [0x3eda_78b7, 0x3f07_414d, 0x3f30_aabe],
                    [0xbb85_41ad, 0xbafa_db77, 0x3be9_a771],
                ),
                TraceEvent::Sum(
                    SimulationJointSumSite::AngularAt,
                    [0x3f53_06b7, 0x3e52_88ad, 0xbf2a_c6c6],
                    [0xbb85_41ad, 0xbafa_db77, 0x3be9_a771],
                ),
                TraceEvent::Reciprocal(
                    SimulationJointReciprocalSite::FinalAngular,
                    [0x3fad_4451, 0x4030_72b5, 0x3ff9_8d8c, 0x407c_dc9b],
                ),
                TraceEvent::Reciprocal(
                    SimulationJointReciprocalSite::FinalLinear,
                    [0x408c_2a0e, 0x408d_7831, 0x408f_894b, 0x4093_e319],
                ),
            ]
        );
    }

    #[test]
    fn empty_and_one_sided_state_admission_preserve_the_exact_dense_rule() {
        let mut empty_bodies = SimulationRigidBodyPool::try_new(0).expect("empty body pool");
        let empty_joints = ExactJointPool::try_new(0).expect("empty Joint pool");
        let empty_phase = empty_bodies
            .try_prepare_dynamic_update_phase()
            .expect("empty body phase");
        let empty_frame = empty_phase.frame_identity();
        let Ok(empty) = try_prepare_active_joint_phase(&empty_joints, &empty_phase) else {
            panic!("empty phase must prepare");
        };
        assert!(empty.records.is_empty());
        assert_eq!(empty.source.joint_pool, empty_joints.authority());
        assert_eq!(empty.source.body_pool, empty_phase.body_pool_authority());
        assert_eq!(empty.source.frame, empty_frame);

        for (parent_state, child_state, admitted) in
            [(4, 1, true), (1, 4, true), (4, 4, true), (1, 1, false)]
        {
            let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
            let parent = add_body(&mut bodies, parent_state);
            let child = add_body(&mut bodies, child_state);
            let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
            add_joint(&mut joints, parent, child);
            let phase = bodies
                .try_prepare_dynamic_update_phase()
                .expect("body phase");
            let mut backend = CountingBackend::default();
            let Ok(prepared) =
                try_prepare_active_joint_phase_with_backend(&joints, &phase, &mut backend)
            else {
                panic!("valid state combination must prepare");
            };
            assert_eq!(prepared.records.len(), usize::from(admitted));
            assert_eq!(backend.calls, if admitted { 17 } else { 0 });
        }
    }

    #[test]
    fn stale_parent_occurrence_fails_before_numeric_work_and_leaves_phase_usable() {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let parent = add_body(&mut bodies, 4);
        let child = add_body(&mut bodies, 4);
        let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
        let joint = add_joint(&mut joints, parent, child);
        bodies.try_remove_rigid_body(parent).expect("remove parent");
        let replacement = add_body(&mut bodies, 4);
        assert_eq!(replacement.id(), parent.id());
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let expected_frame = phase.frame_identity();
        let mut backend = CountingBackend::default();
        let Err(failure) =
            try_prepare_active_joint_phase_with_backend(&joints, &phase, &mut backend)
        else {
            panic!("stale endpoint must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::Body {
                active_ordinal: 0,
                joint: failed_joint,
                endpoint: SimulationJointEndpoint::Parent,
                source: SimulationRigidBodyPoolError::StaleHandle { slot: 0 },
            } if failed_joint == joint.identity()
        ));
        assert_eq!(backend.calls, 0);
        assert_eq!(phase.frame_identity(), expected_frame);
    }

    #[test]
    fn stale_child_and_both_foreign_endpoints_are_attributed_before_numeric_work() {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let parent = add_body(&mut bodies, 4);
        let child = add_body(&mut bodies, 4);
        let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
        let joint = add_joint(&mut joints, parent, child);
        bodies.try_remove_rigid_body(child).expect("remove child");
        let replacement = add_body(&mut bodies, 4);
        assert_eq!(replacement.id(), child.id());
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = CountingBackend::default();
        let Err(stale_child) =
            try_prepare_active_joint_phase_with_backend(&joints, &phase, &mut backend)
        else {
            panic!("stale child must fail");
        };
        assert!(matches!(
            stale_child,
            SimulationJointPreparationError::Body {
                active_ordinal: 0,
                joint: failed_joint,
                endpoint: SimulationJointEndpoint::Child,
                source: SimulationRigidBodyPoolError::StaleHandle { slot: 1 },
            } if failed_joint == joint.identity()
        ));
        assert_eq!(backend.calls, 0);

        let mut foreign_bodies = SimulationRigidBodyPool::try_new(2).expect("foreign body pool");
        add_body(&mut foreign_bodies, 4);
        add_body(&mut foreign_bodies, 4);
        let foreign_phase = foreign_bodies
            .try_prepare_dynamic_update_phase()
            .expect("foreign phase");
        let Err(foreign_parent) =
            try_prepare_active_joint_phase_with_backend(&joints, &foreign_phase, &mut backend)
        else {
            panic!("foreign parent must fail");
        };
        assert!(matches!(
            foreign_parent,
            SimulationJointPreparationError::Body {
                active_ordinal: 0,
                endpoint: SimulationJointEndpoint::Parent,
                source: SimulationRigidBodyPoolError::ForeignHandle,
                ..
            }
        ));
        assert_eq!(backend.calls, 0);

        let mut local_bodies = SimulationRigidBodyPool::try_new(1).expect("local body pool");
        let local_parent = add_body(&mut local_bodies, 4);
        let mut other_bodies = SimulationRigidBodyPool::try_new(1).expect("other body pool");
        let foreign_child = add_body(&mut other_bodies, 4);
        let mut mixed_joints = ExactJointPool::try_new(1).expect("mixed Joint pool");
        add_joint(&mut mixed_joints, local_parent, foreign_child);
        let local_phase = local_bodies
            .try_prepare_dynamic_update_phase()
            .expect("local phase");
        let Err(foreign_child) =
            try_prepare_active_joint_phase_with_backend(&mixed_joints, &local_phase, &mut backend)
        else {
            panic!("foreign child must fail");
        };
        assert!(matches!(
            foreign_child,
            SimulationJointPreparationError::Body {
                active_ordinal: 0,
                endpoint: SimulationJointEndpoint::Child,
                source: SimulationRigidBodyPoolError::ForeignHandle,
                ..
            }
        ));
        assert_eq!(backend.calls, 0);
    }

    #[test]
    fn all_reservations_precede_numeric_callbacks_and_reject_dishonest_success() {
        fn fixture() -> (SimulationRigidBodyPool, ExactJointPool) {
            let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
            let parent = add_body(&mut bodies, 4);
            let child = add_body(&mut bodies, 4);
            let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
            add_joint(&mut joints, parent, child);
            (bodies, joints)
        }

        let (mut bodies, joints) = fixture();
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = CountingBackend::default();
        let Err(validation) = try_prepare_active_joint_phase_with_backend_and_reservation(
            &joints,
            &phase,
            &mut backend,
            |_, _| Ok(()),
            |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
            |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
        ) else {
            panic!("dishonest validation reservation must fail");
        };
        assert!(matches!(
            validation,
            SimulationJointPreparationError::ValidationAllocationFailed { capacity: 1 }
        ));
        assert_eq!(backend.calls, 0);

        let (mut bodies, joints) = fixture();
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let Err(staging) = try_prepare_active_joint_phase_with_backend_and_reservation(
            &joints,
            &phase,
            &mut backend,
            |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
            |_, _| Ok(()),
            |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
        ) else {
            panic!("dishonest staging reservation must fail");
        };
        assert!(matches!(
            staging,
            SimulationJointPreparationError::StagingAllocationFailed { capacity: 1 }
        ));
        assert_eq!(backend.calls, 0);

        let (mut bodies, joints) = fixture();
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let Err(records) = try_prepare_active_joint_phase_with_backend_and_reservation(
            &joints,
            &phase,
            &mut backend,
            |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
            |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
            |_, _| Ok(()),
        ) else {
            panic!("dishonest record reservation must fail");
        };
        assert!(matches!(
            records,
            SimulationJointPreparationError::RecordAllocationFailed { capacity: 1 }
        ));
        assert_eq!(backend.calls, 0);

        let (mut bodies, joints) = fixture();
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let frame = phase.frame_identity();
        let Err(island) =
            try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Null { frame },
                &mut backend,
                |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
                |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
                |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
                |_, _, _| Ok(()),
                |_, _, _| {},
            )
        else {
            panic!("dishonest island reservation must fail");
        };
        assert!(matches!(
            island,
            SimulationJointPreparationError::IslandAllocationFailed {
                island: 0,
                capacity: 1,
            }
        ));
        assert_eq!(backend.calls, 0);
    }

    #[test]
    fn successful_reservations_finish_before_the_first_numeric_callback() {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let parent = add_body(&mut bodies, 4);
        let child = add_body(&mut bodies, 4);
        let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
        add_joint(&mut joints, parent, child);
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut backend = PreparationOrderBackend {
            events: Rc::clone(&events),
        };
        let validation_events = Rc::clone(&events);
        let staging_events = Rc::clone(&events);
        let record_events = Rc::clone(&events);
        let island_events = Rc::clone(&events);

        let frame = phase.frame_identity();
        let Ok(prepared) =
            try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Null { frame },
                &mut backend,
                move |seen, capacity| {
                    seen.try_reserve_exact(capacity).map_err(|_| ())?;
                    validation_events
                        .borrow_mut()
                        .push(PreparationOrderEvent::ValidationReserved);
                    Ok(())
                },
                move |staged, capacity| {
                    staged.try_reserve_exact(capacity).map_err(|_| ())?;
                    staging_events
                        .borrow_mut()
                        .push(PreparationOrderEvent::StagingReserved);
                    Ok(())
                },
                move |records, capacity| {
                    records.try_reserve_exact(capacity).map_err(|_| ())?;
                    record_events
                        .borrow_mut()
                        .push(PreparationOrderEvent::RecordsReserved);
                    Ok(())
                },
                move |_, entries, capacity| {
                    entries.try_reserve_exact(capacity).map_err(|_| ())?;
                    island_events
                        .borrow_mut()
                        .push(PreparationOrderEvent::IslandReserved);
                    Ok(())
                },
                |_, _, _| {},
            )
        else {
            panic!("valid reservations must prepare");
        };
        assert_eq!(prepared.records.len(), 1);
        let events = events.borrow();
        assert_eq!(
            &events[..4],
            [
                PreparationOrderEvent::ValidationReserved,
                PreparationOrderEvent::StagingReserved,
                PreparationOrderEvent::RecordsReserved,
                PreparationOrderEvent::IslandReserved,
            ]
        );
        assert_eq!(events.len(), 21);
        assert!(
            events[4..]
                .iter()
                .all(|event| *event == PreparationOrderEvent::NumericCallback)
        );
    }

    #[test]
    fn later_nonempty_island_reservation_failure_precedes_all_numeric_callbacks() {
        let mut bodies = SimulationRigidBodyPool::try_new(4).expect("body pool");
        let first_parent = add_body(&mut bodies, 4);
        let first_child = add_body(&mut bodies, 4);
        let second_parent = add_body(&mut bodies, 4);
        let second_child = add_body(&mut bodies, 4);
        let mut joints = ExactJointPool::try_new(2).expect("Joint pool");
        add_joint(&mut joints, first_parent, first_child);
        add_joint(&mut joints, second_parent, second_child);
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let frame = bodies.frame_identity().expect("started frame");
        let island_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &bodies,
            frame,
            [
                (first_parent.observation(), 1),
                (first_child.observation(), 0),
                (second_parent.observation(), -1),
                (second_child.observation(), 3),
            ],
        )
        .expect("island map");
        let body_before = after_one_phase_identity_issue(bodies.snapshot());
        let joint_before = joints.snapshot();
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let attempts = Rc::new(RefCell::new(Vec::new()));
        let reservation_attempts = Rc::clone(&attempts);
        let mut backend = CountingBackend::default();
        let Err(failure) =
            try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Present(&island_map),
                &mut backend,
                |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
                |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
                |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
                move |island, entries, capacity| {
                    reservation_attempts.borrow_mut().push((island, capacity));
                    if island == 3 {
                        return Err(());
                    }
                    entries.try_reserve_exact(capacity).map_err(|_| ())
                },
                |_, _, _| {},
            )
        else {
            panic!("later island reservation must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::IslandAllocationFailed {
                island: 3,
                capacity: 1,
            }
        ));
        assert_eq!(&*attempts.borrow(), &[(1, 1), (3, 1)]);
        assert_eq!(phase.frame_identity(), frame);
        assert_eq!(backend.calls, 0);
        assert_eq!(joints.snapshot(), joint_before);
        assert_eq!(bodies.snapshot(), body_before);
    }

    #[test]
    fn dropping_a_successful_prepared_batch_is_body_joint_and_rf_inert() {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let parent = add_body(&mut bodies, 4);
        let child = add_body(&mut bodies, 4);
        let parent_rf = [
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ];
        let child_rf = [
            [-1.0, -2.0, -3.0, -4.0],
            [-5.0, -6.0, -7.0, -8.0],
            [-9.0, -10.0, -11.0, -12.0],
            [-13.0, -14.0, -15.0, -16.0],
        ];
        bodies
            .try_with_solver_delta_row_mut(parent.observation(), |row| *row = parent_rf)
            .expect("seed parent RF");
        bodies
            .try_with_solver_delta_row_mut(child.observation(), |row| *row = child_rf)
            .expect("seed child RF");
        let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
        let joint = add_joint(&mut joints, parent, child);
        let body_before = after_one_phase_identity_issue(bodies.snapshot());
        let joint_before = joints.snapshot();

        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let Ok(prepared) = try_prepare_active_joint_phase(&joints, &phase) else {
            panic!("valid phase must prepare");
        };
        assert_eq!(prepared.records.len(), 1);
        drop(prepared);

        assert_eq!(joints.snapshot(), joint_before);

        joints
            .try_remove_joint(joint)
            .expect("remove after batch drop");
        let replacement = add_joint(&mut joints, parent, child);
        assert_eq!(replacement.id(), joint.id());
        assert_ne!(replacement.identity(), joint.identity());
        let Ok(reprepared) = try_prepare_active_joint_phase(&joints, &phase) else {
            panic!("the same borrowed body phase must support a replacement Joint batch");
        };
        assert_eq!(reprepared.records.len(), 1);
        assert_eq!(reprepared.records[0].joint, replacement.identity());
        drop(reprepared);
        drop(phase);

        assert_eq!(bodies.snapshot(), body_before);
        assert_eq!(
            bodies
                .try_with_solver_delta_row(parent.observation(), |row| *row)
                .expect("parent RF"),
            parent_rf
        );
        assert_eq!(
            bodies
                .try_with_solver_delta_row(child.observation(), |row| *row)
                .expect("child RF"),
            child_rf
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one literal allocation-failure matrix proves the borrowed phase remains usable at every boundary"
    )]
    fn real_reservation_errors_leave_the_borrowed_phase_usable_without_pool_mutation() {
        fn fixture() -> (SimulationRigidBodyPool, ExactJointPool) {
            let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
            let parent = add_body(&mut bodies, 4);
            let child = add_body(&mut bodies, 4);
            let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
            add_joint(&mut joints, parent, child);
            (bodies, joints)
        }

        let (mut bodies, joints) = fixture();
        let body_before = after_one_phase_identity_issue(bodies.snapshot());
        let joint_before = joints.snapshot();
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let frame = phase.frame_identity();
        let mut backend = CountingBackend::default();
        let Err(failure) = try_prepare_active_joint_phase_with_backend_and_reservation(
            &joints,
            &phase,
            &mut backend,
            |_, _| Err(()),
            |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
            |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
        ) else {
            panic!("validation reservation error must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::ValidationAllocationFailed { capacity: 1 }
        ));
        assert_eq!(phase.frame_identity(), frame);
        assert_eq!(backend.calls, 0);
        assert_eq!(joints.snapshot(), joint_before);
        assert_eq!(bodies.snapshot(), body_before);

        let (mut bodies, joints) = fixture();
        let body_before = after_one_phase_identity_issue(bodies.snapshot());
        let joint_before = joints.snapshot();
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let frame = phase.frame_identity();
        let Err(failure) = try_prepare_active_joint_phase_with_backend_and_reservation(
            &joints,
            &phase,
            &mut backend,
            |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
            |_, _| Err(()),
            |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
        ) else {
            panic!("staging reservation error must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::StagingAllocationFailed { capacity: 1 }
        ));
        assert_eq!(phase.frame_identity(), frame);
        assert_eq!(backend.calls, 0);
        assert_eq!(joints.snapshot(), joint_before);
        assert_eq!(bodies.snapshot(), body_before);

        let (mut bodies, joints) = fixture();
        let body_before = after_one_phase_identity_issue(bodies.snapshot());
        let joint_before = joints.snapshot();
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let frame = phase.frame_identity();
        let Err(failure) = try_prepare_active_joint_phase_with_backend_and_reservation(
            &joints,
            &phase,
            &mut backend,
            |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
            |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
            |_, _| Err(()),
        ) else {
            panic!("record reservation error must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::RecordAllocationFailed { capacity: 1 }
        ));
        assert_eq!(phase.frame_identity(), frame);
        assert_eq!(backend.calls, 0);
        assert_eq!(joints.snapshot(), joint_before);
        assert_eq!(bodies.snapshot(), body_before);

        let (mut bodies, joints) = fixture();
        let body_before = after_one_phase_identity_issue(bodies.snapshot());
        let joint_before = joints.snapshot();
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let frame = phase.frame_identity();
        let Err(failure) =
            try_prepare_active_joint_phase_with_island_map_backend_reservation_and_observer(
                &joints,
                &phase,
                SimulationJointIslandMapSource::Null { frame },
                &mut backend,
                |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
                |staged, capacity| staged.try_reserve_exact(capacity).map_err(|_| ()),
                |records, capacity| records.try_reserve_exact(capacity).map_err(|_| ()),
                |_, _, _| Err(()),
                |_, _, _| {},
            )
        else {
            panic!("island reservation error must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::IslandAllocationFailed {
                island: 0,
                capacity: 1,
            }
        ));
        assert_eq!(phase.frame_identity(), frame);
        assert_eq!(backend.calls, 0);
        assert_eq!(joints.snapshot(), joint_before);
        assert_eq!(bodies.snapshot(), body_before);
    }

    #[test]
    fn interior_numeric_failure_returns_no_batch_and_exact_callback_prefix() {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let parent = add_body(&mut bodies, 4);
        let child = add_body(&mut bodies, 4);
        let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
        let joint = add_joint(&mut joints, parent, child);
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = CountingBackend {
            calls: 0,
            fail_at: Some(5),
        };
        let Err(failure) =
            try_prepare_active_joint_phase_with_backend(&joints, &phase, &mut backend)
        else {
            panic!("injected callback must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::Numeric {
                dense_ordinal: 0,
                joint: failed_joint,
                source: InjectedFailure,
            } if failed_joint == joint.identity()
        ));
        assert_eq!(backend.calls, 6);

        let mut retry_backend = CountingBackend::default();
        let Ok(retry) =
            try_prepare_active_joint_phase_with_backend(&joints, &phase, &mut retry_backend)
        else {
            panic!("same borrowed phase must retry");
        };
        assert_eq!(retry.records.len(), 1);
        assert_eq!(retry_backend.calls, 17);
    }

    #[test]
    fn every_numeric_callback_failure_leaves_the_same_borrowed_phase_inert() {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let parent = add_body(&mut bodies, 4);
        let child = add_body(&mut bodies, 4);
        let mut joints = ExactJointPool::try_new(1).expect("Joint pool");
        let joint = add_joint(&mut joints, parent, child);
        let body_before = after_one_phase_identity_issue(bodies.snapshot());
        let joint_before = joints.snapshot();
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let frame = phase.frame_identity();

        for fail_at in 0..17 {
            let mut backend = CountingBackend {
                calls: 0,
                fail_at: Some(fail_at),
            };
            let Err(failure) =
                try_prepare_active_joint_phase_with_backend(&joints, &phase, &mut backend)
            else {
                panic!("every injected callback must fail");
            };
            assert!(matches!(
                failure,
                SimulationJointPreparationError::Numeric {
                    dense_ordinal: 0,
                    joint: failed_joint,
                    source: InjectedFailure,
                } if failed_joint == joint.identity()
            ));
            assert_eq!(backend.calls, fail_at + 1);
            assert_eq!(phase.frame_identity(), frame);
        }

        drop(phase);
        assert_eq!(bodies.snapshot(), body_before);
        assert_eq!(joints.snapshot(), joint_before);
    }

    #[test]
    fn later_record_failure_reports_dense_identity_and_leaves_phase_retryable() {
        let mut bodies = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let parent = add_body(&mut bodies, 4);
        let child = add_body(&mut bodies, 4);
        let mut joints = ExactJointPool::try_new(2).expect("Joint pool");
        let first = add_joint(&mut joints, parent, child);
        let second = add_joint(&mut joints, child, parent);
        let phase = bodies
            .try_prepare_dynamic_update_phase()
            .expect("body phase");
        let mut backend = CountingBackend {
            calls: 0,
            fail_at: Some(17),
        };
        let Err(failure) =
            try_prepare_active_joint_phase_with_backend(&joints, &phase, &mut backend)
        else {
            panic!("second record callback must fail");
        };
        assert!(matches!(
            failure,
            SimulationJointPreparationError::Numeric {
                dense_ordinal: 1,
                joint: failed_joint,
                source: InjectedFailure,
            } if failed_joint == second.identity()
        ));
        assert_ne!(first.identity(), second.identity());
        assert_eq!(backend.calls, 18);

        let mut retry_backend = CountingBackend::default();
        let Ok(retry) =
            try_prepare_active_joint_phase_with_backend(&joints, &phase, &mut retry_backend)
        else {
            panic!("same borrowed phase must build both records");
        };
        assert_eq!(retry.records.len(), 2);
        assert_eq!(retry_backend.calls, 34);
    }

    #[test]
    fn slot_token_encoding_is_nonzero_checked_and_never_wraps() {
        assert_eq!(checked_slot_token(0).map(NonZeroU32::get), Some(1));
        assert_eq!(
            checked_slot_token(u32::MAX - 1).map(NonZeroU32::get),
            Some(u32::MAX)
        );
        assert_eq!(checked_slot_token(u32::MAX), None);
    }
}
