//! Evidence-bounded foundation for `rw::physics::DriveJacobian` construction.
//!
//! `DriveJacobian::Build` (`0x82ae4bf8..0x82ae5918`) begins by clearing three
//! 128-byte cache lines at output offsets `0`, `0x80`, and `0x100` and then
//! fills one `0x180`-byte record. This module freezes that value layout and the
//! complete pure ordinary-finite Build composer. A crate-private preparation
//! seam couples copied, occurrence-bound pool observations to unpublished
//! records. The runtime-capable private executor applies one phase-sealed,
//! frame-bound island Drive sweep to live RF rows. The older direct/global
//! executor remains only as a unit-test evidence harness. No batch is exposed,
//! and the complete Contact/Joint/Drive Horus pipeline remains open.
//!
//! The PDB names the packed words at `+0x0c/+0x1c` `m_ia/m_ib`. Runtime code
//! uses them as pointers to private per-body 0x40-byte solver-delta rows, not
//! logical body IDs. The host fields below are private 32-bit tokens. They are
//! never native pointers, truncated host pointers, or public mutation
//! capabilities.
//!
//! `Jacobian_RQD::Create` (`0x82ae3750..0x82ae3930`) is recovered completely
//! for ordinary finite f32 inputs. The coefficient helpers preserve the
//! explicit multiply/FMA and strict-compare graph, but accept reciprocal or
//! reciprocal-square-root estimate seeds from the caller because Xenon
//! `vrefp`/`vrsqrtefp` seed bits are not yet proven. NaNs, infinities,
//! denormals, signed zero, VMX exception handling, and horizontal reduction
//! behavior remain explicit hardware boundaries.
//!
//! The complete composer crosses those boundaries through private typed site
//! callbacks for `vrefp`, `vrsqrtefp`, and `vmsum3fp128`. Calls are lazy and
//! retain raw branch/state chronology. The PC host policy uses `recip()`,
//! `sqrt().recip()`, and nested FMA as an explicit ordinary-finite host model;
//! it is not labeled Xenon-bit-exact.

use crate::simulation_constraint_math::{
    compose_body_frame_quaternion_from_dot3, create_jacobian_rqd_rows, horus_columns_from,
    horus_cross_add, horus_madd, horus_negative_columns_from, horus_nmsub, pack_xyz_word,
};
use crate::simulation_drive_pool::{
    SimulationDriveBuildBinding, SimulationDriveBuildBindingValidationError,
    SimulationDriveBuildSnapshot, SimulationDriveDynamics, SimulationDriveDynamicsParams,
    SimulationDriveEndpoint, SimulationDriveFrames, SimulationDriveIdentity, SimulationDrivePool,
    SimulationDrivePoolAuthority, SimulationDrivePoolError,
};
use crate::simulation_island_map::{
    SIMULATION_ISLAND_CAPACITY, SimulationRigidBodyIslandMapSnapshot,
};
#[cfg(test)]
use crate::simulation_rigid_body_pool::SimulationRigidBodySolverDeltaTransactionError;
use crate::simulation_rigid_body_pool::{
    SimulationRigidBodyDriveBuildSnapshot, SimulationRigidBodyDynamicUpdatePhase,
    SimulationRigidBodyDynamicUpdatePhaseIdentity, SimulationRigidBodyFrameIdentity,
    SimulationRigidBodyIndexedSolverDeltaError, SimulationRigidBodyIndexedSolverDeltaPlan,
    SimulationRigidBodyObservation, SimulationRigidBodyPool, SimulationRigidBodyPoolAuthority,
    SimulationRigidBodySolverDeltaPairRows, SimulationRigidBodySolverDeltaStores,
};

const SOFT_DRIVE: u32 = 1;
const HARD_DRIVE: u32 = 2;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SolverDeltaRowToken(u32);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DriveSlotToken(u32);

/// Exact host value layout corresponding to native `rw::physics::DriveJacobian`.
///
/// Native reference: `DriveJacobian::Build` at `0x82ae4bf8`; PDB size `0x180`.
/// The three private token words intentionally have host ownership semantics,
/// despite occupying positions that contain native pointers or padding.
/// The PDB-pad words following the three angular axes are not necessarily zero:
/// Build's full-vector stores retain each normalized axis's W residue there.
#[repr(C, align(16))]
#[derive(Clone, Copy, PartialEq)]
pub struct SimulationDriveJacobian {
    pub attachment_offset_a: [f32; 3],
    solver_delta_a: SolverDeltaRowToken,
    pub attachment_offset_b: [f32; 3],
    solver_delta_b: SolverDeltaRowToken,
    pub position_impulse: [f32; 3],
    pub linear_dynamics: f32,
    pub angular_impulse: [f32; 3],
    pub angular_dynamics: f32,
    pub linear_transpose_right: [f32; 3],
    pub spy: u32,
    pub angular_transpose_right: [f32; 3],
    pad_0: f32,
    pub linear_transpose_up: [f32; 3],
    pub position_limit_x: f32,
    pub angular_transpose_up: [f32; 3],
    pub position_limit_y: f32,
    pub linear_transpose_at: [f32; 3],
    pub angular_limit_x: f32,
    pub angular_transpose_at: [f32; 3],
    pub angular_limit_y: f32,
    pub position_offset: [f32; 3],
    pub position_limit_z: f32,
    pub angular_offset: [f32; 3],
    pub angular_limit_z: f32,
    pub linear_right: [f32; 3],
    drive_slot: DriveSlotToken,
    pub linear_up: [f32; 3],
    pad_1: f32,
    pub linear_at: [f32; 3],
    pad_2: f32,
    pub angular_right: [f32; 3],
    pad_3: f32,
    pub angular_up: [f32; 3],
    pad_4: f32,
    pub angular_at: [f32; 3],
    pad_5: f32,
    pub inverse_inertia_a_right: [f32; 3],
    pub inverse_mass_a: f32,
    pub inverse_inertia_a_up: [f32; 3],
    pad_6: f32,
    pub inverse_inertia_a_at: [f32; 3],
    pad_7: f32,
    pub inverse_inertia_b_right: [f32; 3],
    pub inverse_mass_b: f32,
    pub inverse_inertia_b_up: [f32; 3],
    pad_8: f32,
    pub inverse_inertia_b_at: [f32; 3],
    pad_9: f32,
}

impl SimulationDriveJacobian {
    /// Reproduces the all-zero result of Build's three initial `dcbz128` lines.
    ///
    /// Native anchors: `0x82ae4c14`, `0x82ae4c20`, and `0x82ae4c28`.
    #[must_use]
    pub const fn zeroed() -> Self {
        Self {
            attachment_offset_a: [0.0; 3],
            solver_delta_a: SolverDeltaRowToken(0),
            attachment_offset_b: [0.0; 3],
            solver_delta_b: SolverDeltaRowToken(0),
            position_impulse: [0.0; 3],
            linear_dynamics: 0.0,
            angular_impulse: [0.0; 3],
            angular_dynamics: 0.0,
            linear_transpose_right: [0.0; 3],
            spy: 0,
            angular_transpose_right: [0.0; 3],
            pad_0: 0.0,
            linear_transpose_up: [0.0; 3],
            position_limit_x: 0.0,
            angular_transpose_up: [0.0; 3],
            position_limit_y: 0.0,
            linear_transpose_at: [0.0; 3],
            angular_limit_x: 0.0,
            angular_transpose_at: [0.0; 3],
            angular_limit_y: 0.0,
            position_offset: [0.0; 3],
            position_limit_z: 0.0,
            angular_offset: [0.0; 3],
            angular_limit_z: 0.0,
            linear_right: [0.0; 3],
            drive_slot: DriveSlotToken(0),
            linear_up: [0.0; 3],
            pad_1: 0.0,
            linear_at: [0.0; 3],
            pad_2: 0.0,
            angular_right: [0.0; 3],
            pad_3: 0.0,
            angular_up: [0.0; 3],
            pad_4: 0.0,
            angular_at: [0.0; 3],
            pad_5: 0.0,
            inverse_inertia_a_right: [0.0; 3],
            inverse_mass_a: 0.0,
            inverse_inertia_a_up: [0.0; 3],
            pad_6: 0.0,
            inverse_inertia_a_at: [0.0; 3],
            pad_7: 0.0,
            inverse_inertia_b_right: [0.0; 3],
            inverse_mass_b: 0.0,
            inverse_inertia_b_up: [0.0; 3],
            pad_8: 0.0,
            inverse_inertia_b_at: [0.0; 3],
            pad_9: 0.0,
        }
    }

    /// Reports whether both private solver destinations have been installed.
    ///
    /// This reveals no row index and grants no mutation capability. The
    /// foundation slice intentionally has no public binding operation.
    #[must_use]
    pub const fn has_solver_delta_bindings(&self) -> bool {
        self.solver_delta_a.0 != 0 && self.solver_delta_b.0 != 0
    }
}

impl Default for SimulationDriveJacobian {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// Staged value result of one direct Horus Drive-record solve.
///
/// Native commits vectors in the interleaved order `A[0], B[0], A[2], B[2]`.
/// Keeping both complete blocks by value lets a later pool seam reproduce that
/// order even when both private row tokens designate the same body, without
/// ever creating aliased mutable references. The complete copies also make
/// untouched `d1/d3` observable in tests; a caller must assign only returned
/// vectors `0` and `2`, never replace an entire RF block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveHorusRows {
    pub solver_delta_a: [[f32; 4]; 4],
    pub solver_delta_b: [[f32; 4]; 4],
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SimulationDriveHorusAccumulatorStoreOrder {
    #[cfg(test)]
    AngularThenPosition,
    PositionThenAngular,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationDriveHorusAccumulator {
    Angular,
    Position,
}

/// Applies the direct-array Horus Drive consumer to one staged record.
///
/// Native anchors are `rw::physics::HorusPipeline`
/// `0x82ae2540..0x82ae2820`, with the per-record body at
/// `0x82ae2568..0x82ae2814`. This helper preserves the exact right/up/at
/// accumulation order, fused `vmaddfp`/`vnmsubfp` graph, min-then-max clamp,
/// A-add/B-subtract signs, and packed fourth-lane arithmetic. It mutates only
/// the Jacobian's accumulated impulse XYZ; their adjacent dynamics words are
/// retained exactly. Returned RF vectors `+0x10/+0x30` are untouched.
///
/// This is an ordinary-finite host model. Xenon `vminfp128`/`vmaxfp128` NaN
/// and signed-zero selection, denormal/flush mode, infinities, overflow, and
/// exceptional fused behavior remain unclaimed. Native `m_ia/m_ib` resolve RF
/// rows before this helper; their packed pointer bits then also participate in
/// fourth-lane numeric arithmetic. The safe host deliberately substitutes
/// private token bits, so native pointer-derived W parity is outside the claim
/// and cannot affect the proven XYZ graph.
#[cfg(test)]
pub(crate) fn solve_drive_horus_record_host(
    jacobian: &mut SimulationDriveJacobian,
    solver_delta_a: [[f32; 4]; 4],
    solver_delta_b: [[f32; 4]; 4],
) -> SimulationDriveHorusRows {
    solve_drive_horus_record_with_order(
        jacobian,
        solver_delta_a,
        solver_delta_b,
        SimulationDriveHorusAccumulatorStoreOrder::AngularThenPosition,
        |_, _| {},
    )
}

/// Applies the island-indirect accumulator chronology to the common graph.
///
/// Island Horus stores Position then Angular at `0x82ae3054..0x82ae3058`.
/// The dependency-equivalent arithmetic graph is reused only inside the
/// ordinary-finite host boundary; native instruction scheduling is not claimed.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the checked island Drive sweep")
)]
fn solve_drive_horus_island_record_host(
    jacobian: &mut SimulationDriveJacobian,
    solver_delta_a: [[f32; 4]; 4],
    solver_delta_b: [[f32; 4]; 4],
) -> SimulationDriveHorusRows {
    solve_drive_horus_island_record_with_observer(
        jacobian,
        solver_delta_a,
        solver_delta_b,
        |_, _| {},
    )
}

fn solve_drive_horus_island_record_with_observer(
    jacobian: &mut SimulationDriveJacobian,
    solver_delta_a: [[f32; 4]; 4],
    solver_delta_b: [[f32; 4]; 4],
    observe_accumulator_store: impl FnMut(SimulationDriveHorusAccumulator, &SimulationDriveJacobian),
) -> SimulationDriveHorusRows {
    solve_drive_horus_record_with_order(
        jacobian,
        solver_delta_a,
        solver_delta_b,
        SimulationDriveHorusAccumulatorStoreOrder::PositionThenAngular,
        observe_accumulator_store,
    )
}

#[allow(
    clippy::similar_names,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "source-shaped A/B names and uninterrupted native operation order are intentional"
)]
fn solve_drive_horus_record_with_order(
    jacobian: &mut SimulationDriveJacobian,
    mut solver_delta_a: [[f32; 4]; 4],
    mut solver_delta_b: [[f32; 4]; 4],
    accumulator_store_order: SimulationDriveHorusAccumulatorStoreOrder,
    mut observe_accumulator_store: impl FnMut(SimulationDriveHorusAccumulator, &SimulationDriveJacobian),
) -> SimulationDriveHorusRows {
    let attachment_a = pack_xyz_word(
        jacobian.attachment_offset_a,
        f32::from_bits(jacobian.solver_delta_a.0),
    );
    let attachment_b = pack_xyz_word(
        jacobian.attachment_offset_b,
        f32::from_bits(jacobian.solver_delta_b.0),
    );
    let old_position_impulse = pack_xyz_word(jacobian.position_impulse, jacobian.linear_dynamics);
    let old_angular_impulse = pack_xyz_word(jacobian.angular_impulse, jacobian.angular_dynamics);

    let linear_transpose = [
        pack_xyz_word(
            jacobian.linear_transpose_right,
            f32::from_bits(jacobian.spy),
        ),
        pack_xyz_word(jacobian.linear_transpose_up, jacobian.position_limit_x),
        pack_xyz_word(jacobian.linear_transpose_at, jacobian.angular_limit_x),
    ];
    let angular_transpose = [
        pack_xyz_word(jacobian.angular_transpose_right, jacobian.pad_0),
        pack_xyz_word(jacobian.angular_transpose_up, jacobian.position_limit_y),
        pack_xyz_word(jacobian.angular_transpose_at, jacobian.angular_limit_y),
    ];
    let position_offset = pack_xyz_word(jacobian.position_offset, jacobian.position_limit_z);
    let angular_offset = pack_xyz_word(jacobian.angular_offset, jacobian.angular_limit_z);

    let point_a = horus_cross_add(solver_delta_a[2], attachment_a, solver_delta_a[0]);
    let point_b = horus_cross_add(solver_delta_b[2], attachment_b, solver_delta_b[0]);
    let relative = vsub(point_b, point_a);
    // Both native forms project angular RF directly (82ae25c4/82ae2f60).
    // Attachment-point motion is a separate input to the linear projection.
    let angular_relative = vsub(solver_delta_b[2], solver_delta_a[2]);
    let angular_projection =
        horus_columns_from(angular_transpose, angular_relative, old_angular_impulse);
    let position_projection = horus_columns_from(linear_transpose, relative, old_position_impulse);
    let angular_candidate = horus_madd(
        angular_projection,
        [jacobian.angular_dynamics; 4],
        angular_offset,
    );
    let position_candidate = horus_madd(
        position_projection,
        [jacobian.linear_dynamics; 4],
        position_offset,
    );

    let angular_upper = [
        jacobian.angular_limit_x,
        jacobian.angular_limit_y,
        jacobian.angular_limit_z,
        jacobian.angular_limit_z,
    ];
    let position_upper = [
        jacobian.position_limit_x,
        jacobian.position_limit_y,
        jacobian.position_limit_z,
        jacobian.position_limit_z,
    ];
    let new_angular_impulse = horus_min_then_max(angular_candidate, angular_upper);
    let new_position_impulse = horus_min_then_max(position_candidate, position_upper);
    let angular_change = vsub(new_angular_impulse, old_angular_impulse);
    let position_change = vsub(new_position_impulse, old_position_impulse);

    // Both native forms retain the original packed dynamics word through vsel.
    // Only the two accumulator stores differ: direct Horus is Angular then
    // Position, while island-indirect Horus is Position then Angular. These
    // ordinary Rust assignments preserve semantic/panic chronology for tests;
    // emitted physical machine-store ordering is not claimed.
    match accumulator_store_order {
        #[cfg(test)]
        SimulationDriveHorusAccumulatorStoreOrder::AngularThenPosition => {
            jacobian.angular_impulse = first3(new_angular_impulse);
            observe_accumulator_store(SimulationDriveHorusAccumulator::Angular, jacobian);
            jacobian.position_impulse = first3(new_position_impulse);
            observe_accumulator_store(SimulationDriveHorusAccumulator::Position, jacobian);
        }
        SimulationDriveHorusAccumulatorStoreOrder::PositionThenAngular => {
            jacobian.position_impulse = first3(new_position_impulse);
            observe_accumulator_store(SimulationDriveHorusAccumulator::Position, jacobian);
            jacobian.angular_impulse = first3(new_angular_impulse);
            observe_accumulator_store(SimulationDriveHorusAccumulator::Angular, jacobian);
        }
    }

    let linear_axes = [
        pack_xyz_word(jacobian.linear_right, f32::from_bits(jacobian.drive_slot.0)),
        pack_xyz_word(jacobian.linear_up, jacobian.pad_1),
        pack_xyz_word(jacobian.linear_at, jacobian.pad_2),
    ];
    let angular_axes = [
        pack_xyz_word(jacobian.angular_right, jacobian.pad_3),
        pack_xyz_word(jacobian.angular_up, jacobian.pad_4),
        pack_xyz_word(jacobian.angular_at, jacobian.pad_5),
    ];
    let inverse_inertia_a = [
        pack_xyz_word(jacobian.inverse_inertia_a_right, jacobian.inverse_mass_a),
        pack_xyz_word(jacobian.inverse_inertia_a_up, jacobian.pad_6),
        pack_xyz_word(jacobian.inverse_inertia_a_at, jacobian.pad_7),
    ];
    let inverse_inertia_b = [
        pack_xyz_word(jacobian.inverse_inertia_b_right, jacobian.inverse_mass_b),
        pack_xyz_word(jacobian.inverse_inertia_b_up, jacobian.pad_8),
        pack_xyz_word(jacobian.inverse_inertia_b_at, jacobian.pad_9),
    ];

    let linear_response = horus_columns_from(linear_axes, position_change, [0.0; 4]);
    let angular_response = horus_columns_from(angular_axes, angular_change, [0.0; 4]);
    let response_a = horus_cross_add(attachment_a, linear_response, angular_response);
    let response_b = horus_cross_add(attachment_b, linear_response, angular_response);

    let new_a_0 = horus_madd(
        linear_response,
        [jacobian.inverse_mass_a; 4],
        solver_delta_a[0],
    );
    let new_b_0 = horus_nmsub(
        linear_response,
        [jacobian.inverse_mass_b; 4],
        solver_delta_b[0],
    );
    let new_a_2 = horus_columns_from(inverse_inertia_a, response_a, solver_delta_a[2]);
    let new_b_2 = horus_negative_columns_from(inverse_inertia_b, response_b, solver_delta_b[2]);

    // These staged assignments retain all four source vectors. A later caller
    // must commit only A[0], B[0], A[2], B[2], in exactly that interleaved
    // native store order. It must not replace either complete RF block.
    solver_delta_a[0] = new_a_0;
    solver_delta_b[0] = new_b_0;
    solver_delta_a[2] = new_a_2;
    solver_delta_b[2] = new_b_2;
    SimulationDriveHorusRows {
        solver_delta_a,
        solver_delta_b,
    }
}

/// Geometry-only body input consumed by the first pure `DriveJacobian::Build`
/// continuation.
///
/// Native body A is the semantic child and body B is the semantic parent. The
/// packed words adjacent to each native XYZ axis are deliberately absent: the
/// recovered attachment graph consumes only XYZ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveBodyGeometryInput {
    pub quaternion: [f32; 4],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub at: [f32; 3],
}

/// Value-only result of Build's quaternion, RQD, linear-axis, and attachment
/// geometry subgraph.
///
/// This is not a complete Jacobian. In particular it contains no prediction,
/// effective-mass, limit, solver-row, Drive-slot, or Horus state.
/// `packed_linear_axes` retains the computational W lanes needed by angular
/// fallback and the eventual `pad_1/pad_2` stores; the named XYZ fields remain
/// convenient semantic projections of those same vectors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveGeometryHost {
    pub attachment_offset_a: [f32; 3],
    pub attachment_offset_b: [f32; 3],
    pub linear_right: [f32; 3],
    pub linear_up: [f32; 3],
    pub linear_at: [f32; 3],
    pub packed_linear_axes: [[f32; 4]; 3],
    pub rqd: SimulationDriveJacobianRqd,
}

/// Body vectors consumed by Build's point-prediction graph.
///
/// Native reads the PDB-named force and torque vectors directly in this slice;
/// no inverse-mass or inverse-inertia operation occurs here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDrivePredictionBodyInput {
    pub center_of_mass: [f32; 3],
    pub velocity: [f32; 3],
    pub omega: [f32; 3],
    pub force: [f32; 3],
    pub torque: [f32; 3],
}

/// Relative point values produced before Drive coefficient application.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDrivePointPredictionHost {
    pub position_error: [f32; 3],
    pub velocity_delta_h: [f32; 3],
    pub acceleration_delta_h2: [f32; 3],
}

/// Global linear bias and its Right/Up/At projections.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveLinearBiasHost {
    pub bias: [f32; 3],
    pub projection: [f32; 3],
}

/// Angular body values consumed by the remaining Build subgraph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveAngularBodyInput {
    pub omega: [f32; 3],
    pub torque: [f32; 3],
}

/// Caller-supplied Xenon estimate results for the angular Build subgraph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveAngularEstimateSeeds {
    pub axis_reciprocal_sqrt: [f32; 4],
    pub coefficients: SimulationDriveReciprocalSeeds,
    pub hard_reciprocal_sqrt: f32,
}

/// Value-only result of angular-axis construction, prediction, and bias.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveAngularBuildHost {
    /// Complete T vectors. Their W lanes later become `pad_3/pad_4/pad_5`.
    pub angular_axes: [[f32; 4]; 3],
    /// Angular error before the optional HARD gate; bias uses the gated copy.
    pub angular_error: [f32; 3],
    pub omega_projection: [f32; 3],
    pub acceleration_projection: [f32; 3],
    pub bias_projection: [f32; 3],
    pub coefficients: SimulationDriveCoefficients,
    pub strength_h2: f32,
    pub normalization_denominator: [f32; 4],
    pub used_global_fallback: bool,
    pub hard_outside_limit: bool,
}

/// Body-side values consumed by Build's effective-mass tail.
///
/// `inverse_inertia_full` is native `mIfull=[Ixx,Ixy,Ixz]`, while
/// `inverse_inertia_split` is native `mIsplt=[Izz,Iyy,Iyz]`. Only state bit 4
/// admits this body's tensor and inverse-mass contribution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveEffectiveMassBodyInput {
    pub state: u32,
    pub attachment_offset: [f32; 3],
    pub inverse_inertia_full: [f32; 3],
    pub inverse_mass: f32,
    pub inverse_inertia_split: [f32; 3],
}

/// Pre-reciprocal result of Build's independently gated effective-mass graph.
///
/// The column/mass fields are the exact values later stored in the A/B tail of
/// the `0x180`-byte Jacobian. Denominators intentionally remain
/// pre-reciprocal so this value makes no claim about Xenon `vrefp` seed bits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveEffectiveMassHost {
    pub inverse_inertia_a: [[f32; 3]; 3],
    pub inverse_mass_a: f32,
    pub inverse_inertia_b: [[f32; 3]; 3],
    pub inverse_mass_b: f32,
    pub linear_denominator: [f32; 3],
    pub angular_denominator: [f32; 3],
}

/// Inputs consumed by Build's reciprocal and final numeric-store tail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveFinalizationInput {
    pub effective_mass: SimulationDriveEffectiveMassHost,
    pub packed_linear_axes: [[f32; 4]; 3],
    pub packed_angular_axes: [[f32; 4]; 3],
    /// Right/Up/At projections, not the upstream world-space bias vector.
    pub linear_bias_projection: [f32; 3],
    /// Right/Up/At projections returned by the angular Build subgraph.
    pub angular_bias_projection: [f32; 3],
    pub linear_strength_h2: f32,
    pub angular_strength_h2: f32,
    pub linear_dynamics: f32,
    pub angular_dynamics: f32,
}

/// Caller-supplied Xenon reciprocal-estimate results for the final tail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveFinalReciprocalSeeds {
    pub linear: [f32; 3],
    pub angular: [f32; 3],
}

/// Numeric intermediates and record values produced by Build's final tail.
///
/// Transpose rows and limit vectors are in semantic Right/Up/At order. Full
/// L/T axes and inverse-inertia columns retain every native W lane. Attachment
/// offsets, private tokens, and the Drive spy word are earlier stores and
/// deliberately remain outside this capability-free value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveFinalizedHost {
    pub packed_linear_axes: [[f32; 4]; 3],
    pub packed_angular_axes: [[f32; 4]; 3],
    pub packed_inverse_inertia_a: [[f32; 4]; 3],
    pub packed_inverse_inertia_b: [[f32; 4]; 3],
    pub inverse_linear_denominator: [f32; 3],
    pub inverse_angular_denominator: [f32; 3],
    pub position_offset: [f32; 3],
    pub angular_offset: [f32; 3],
    pub position_limit: [f32; 3],
    pub angular_limit: [f32; 3],
    pub linear_transpose: [[f32; 3]; 3],
    pub angular_transpose: [[f32; 3]; 3],
    /// PDB `m_pad0`; the packed TT-right store duplicates TT-right X here.
    pub angular_transpose_right_pad: f32,
    pub position_impulse: [f32; 3],
    pub linear_dynamics: f32,
    pub angular_impulse: [f32; 3],
    pub angular_dynamics: f32,
}

/// Complete body values consumed by the pure Build composer.
///
/// This is a copied numeric snapshot, not a body identity or pool capability.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SimulationDriveBuildBodyInput {
    state: u32,
    quaternion: [f32; 4],
    center_of_mass: [f32; 3],
    velocity: [f32; 3],
    omega: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    at: [f32; 3],
    force: [f32; 3],
    torque: [f32; 3],
    inverse_inertia_full: [f32; 3],
    inverse_mass: f32,
    inverse_inertia_split: [f32; 3],
}

/// Explicit estimate results for every hardware-sensitive Build site.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SimulationDriveBuildEstimateSeeds {
    linear_coefficients: SimulationDriveReciprocalSeeds,
    linear_hard_reciprocal_sqrt: f32,
    angular: SimulationDriveAngularEstimateSeeds,
    final_reciprocals: SimulationDriveFinalReciprocalSeeds,
}

/// Inert host words occupying native pointer-shaped record lanes.
///
/// These values grant no body, solver-row, Drive, or pool authority. A future
/// batch sidecar must retain the real occurrence-bound capabilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationDriveRecordBindings {
    solver_delta_a: u32,
    solver_delta_b: u32,
    drive: u32,
}

/// Complete copied inputs for the pure ordinary-finite Build composer.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SimulationDriveBuildInput {
    body_a: SimulationDriveBuildBodyInput,
    body_b: SimulationDriveBuildBodyInput,
    frames: SimulationDriveFrames,
    dynamics: SimulationDriveDynamics,
    time_step: f32,
    spy: u32,
    bindings: SimulationDriveRecordBindings,
}

/// Typed domain of one private nonzero host binding word.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private direct Drive-phase boundary")
)]
pub(crate) enum SimulationDrivePreparedBindingKind {
    BodyA,
    BodyB,
    Drive,
}

/// Typed failure while preparing a complete unpublished Drive-record batch.
#[derive(Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private direct Drive-phase boundary")
)]
enum SimulationDriveBatchPreparationError<E> {
    Pool(SimulationDrivePoolError),
    StagingAllocationFailed {
        capacity: usize,
    },
    RecordAllocationFailed {
        capacity: usize,
    },
    BindingTokenOverflow {
        drive: SimulationDriveIdentity,
        kind: SimulationDrivePreparedBindingKind,
        slot: u32,
    },
    Numeric {
        ordinal: usize,
        drive: SimulationDriveIdentity,
        source: E,
    },
}

/// Pool authorities that bind even an empty prepared batch to its source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private direct Drive-phase boundary")
)]
struct SimulationDrivePreparedBatchSource {
    drive_pool: SimulationDrivePoolAuthority,
    body_pool: SimulationRigidBodyPoolAuthority,
}

/// One composed record plus the capabilities required for later revalidation.
///
/// The three words inside `record` are checked, nonzero `slot + 1` tokens.
/// They are deterministic private host sentinels, not native pointer bits and
/// not authority. Only the occurrence-qualified sidecar below can identify the
/// Drive and body allocations for the private RF transaction.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private direct Drive-phase boundary")
)]
pub(crate) struct SimulationPreparedDriveJacobian {
    drive: SimulationDriveIdentity,
    body_a: SimulationRigidBodyObservation,
    body_b: SimulationRigidBodyObservation,
    record: SimulationDriveJacobian,
}

/// Fully checked value input held after token preflight and before composition.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private direct Drive-phase boundary")
)]
struct SimulationPreparedDriveBuildInput {
    drive: SimulationDriveIdentity,
    body_a: SimulationRigidBodyObservation,
    body_b: SimulationRigidBodyObservation,
    input: SimulationDriveBuildInput,
}

/// Complete dense Active-ring result staged without pool or RF mutation.
///
/// This type deliberately does not implement `Clone`: later publication must
/// consume or exclusively borrow one validated preparation rather than fork it.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private direct Drive-phase boundary")
)]
struct SimulationPreparedDriveJacobianBatch {
    source: SimulationDrivePreparedBatchSource,
    records: Vec<SimulationPreparedDriveJacobian>,
}

#[derive(Clone, Copy)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the owning StartFrame island phase")
)]
enum SimulationDriveIslandMapSource<'a> {
    Null {
        frame: SimulationRigidBodyFrameIdentity,
    },
    Present(&'a SimulationRigidBodyIslandMapSnapshot),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationPreparedDriveIslandEntry {
    dense_ordinal: u32,
    drive: SimulationDriveIdentity,
    body_a: SimulationRigidBodyObservation,
    body_b: SimulationRigidBodyObservation,
}

/// One phase-sealed, frame-bound dense Drive batch and its ten stable island subsequences.
///
/// This type deliberately does not implement `Clone`: records and their
/// occurrence-qualified table entries cannot be detached or cross-paired.
pub(crate) struct SimulationPreparedDriveIslandBatch {
    source: SimulationDrivePreparedBatchSource,
    frame: SimulationRigidBodyFrameIdentity,
    phase: SimulationRigidBodyDynamicUpdatePhaseIdentity,
    records: Vec<SimulationPreparedDriveJacobian>,
    islands: [Vec<SimulationPreparedDriveIslandEntry>; SIMULATION_ISLAND_CAPACITY],
}

impl SimulationPreparedDriveIslandBatch {
    pub(crate) fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Diagnostic copy of the live rows around the board hook drive.
    pub(crate) fn diagnostic_first_drive_angular_rows(&self) -> Vec<String> {
        self.records
            .iter()
            .filter(|prepared| prepared.record.drive_slot.0 <= 3)
            .map(|prepared| {
                let row = &prepared.record;
                format!(
                    "slot={} angular-impulse={:?} angular-offset={:?} angular-limit={:?} angular-dynamics={} inverse-mass-b={} angular-axes={:?}",
                    row.drive_slot.0,
                    row.angular_impulse,
                    row.angular_offset,
                    [row.angular_limit_x, row.angular_limit_y, row.angular_limit_z],
                    row.angular_dynamics,
                    row.inverse_mass_b,
                    [row.angular_right, row.angular_up, row.angular_at],
                )
            })
            .collect()
    }
}

/// Existing live-pool Build graph and host numeric policy, with a genuine
/// manager map. This does not normalize the retained frames before Build.
pub(crate) fn try_prepare_active_drive_islands_host(
    drive_pool: &SimulationDrivePool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    map: &SimulationRigidBodyIslandMapSnapshot,
) -> Result<
    SimulationPreparedDriveIslandBatch,
    SimulationDriveIslandBatchPreparationError<std::convert::Infallible>,
> {
    try_prepare_active_drive_island_batch_with_backend(
        drive_pool,
        phase,
        SimulationDriveIslandMapSource::Present(map),
        &mut SimulationDriveHostNumericBackend,
    )
}

struct SimulationPreparedDriveIslandBuildInput {
    island: usize,
    prepared: SimulationPreparedDriveBuildInput,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SimulationDriveIslandBatchPreparationError<E> {
    Pool(SimulationDrivePoolError),
    IslandMapBodyPoolMismatch,
    IslandMapFrameMismatch,
    MissingIslandLabel {
        ordinal: usize,
        drive: SimulationDriveIdentity,
        endpoint: SimulationDriveEndpoint,
        body: SimulationRigidBodyObservation,
    },
    IslandLabelOccurrenceMismatch {
        ordinal: usize,
        drive: SimulationDriveIdentity,
        endpoint: SimulationDriveEndpoint,
        expected_body: SimulationRigidBodyObservation,
        mapped_body: SimulationRigidBodyObservation,
    },
    InvalidSelectedIsland {
        ordinal: usize,
        drive: SimulationDriveIdentity,
        raw_body_b: i8,
        raw_body_a: i8,
        selected: i8,
    },
    DenseOrdinalOverflow {
        ordinal: usize,
    },
    IslandCountOverflow {
        island: u8,
        count: usize,
    },
    StagingAllocationFailed {
        capacity: usize,
    },
    RecordAllocationFailed {
        capacity: usize,
    },
    IslandAllocationFailed {
        island: u8,
        capacity: usize,
    },
    BindingTokenOverflow {
        drive: SimulationDriveIdentity,
        kind: SimulationDrivePreparedBindingKind,
        slot: u32,
    },
    Numeric {
        ordinal: usize,
        drive: SimulationDriveIdentity,
        source: E,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationPreparedDriveIslandHorusError {
    IslandOutOfRange {
        island: usize,
    },
    FrameMismatch,
    DrivePoolSourceMismatch,
    BodyPoolSourceMismatch,
    PhaseMismatch,
    Drive(SimulationDriveBuildBindingValidationError),
    TableEntryMismatch {
        island_ordinal: usize,
        dense_ordinal: u32,
    },
    BindingMismatch {
        island_ordinal: usize,
        dense_ordinal: u32,
        drive: SimulationDriveIdentity,
        kind: SimulationDrivePreparedBindingKind,
        expected: u32,
        actual: u32,
    },
    Body(SimulationRigidBodyIndexedSolverDeltaError),
}

/// Typed pre-effect refusal while flattening all scheduled Drive islands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationPreparedDriveHorusSerialPlanError {
    IslandCountOutOfRange {
        island_count: usize,
    },
    Island {
        island: usize,
        source: SimulationPreparedDriveIslandHorusError,
    },
    EntryCountOverflow,
    BodySequence(SimulationRigidBodyIndexedSolverDeltaError),
}

/// One phase-bound Drive plan with exact table-order ranges for ten islands.
pub(crate) struct SimulationPreparedDriveHorusSerialPlan<'batch> {
    rf: SimulationRigidBodyIndexedSolverDeltaPlan<'batch, SimulationPreparedDriveJacobian>,
    ranges: [(usize, usize); SIMULATION_ISLAND_CAPACITY],
    island_count: usize,
}

impl SimulationDriveIslandMapSource<'_> {
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

/// Typed pre-effect refusal from the test-only direct/global Drive harness.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg(test)]
enum SimulationPreparedDriveHorusError {
    DrivePoolSourceMismatch,
    BodyPoolSourceMismatch,
    Drive(SimulationDriveBuildBindingValidationError),
    BindingMismatch {
        ordinal: usize,
        drive: SimulationDriveIdentity,
        kind: SimulationDrivePreparedBindingKind,
        expected: u32,
        actual: u32,
    },
    Body(SimulationRigidBodySolverDeltaTransactionError),
}

/// One hardware-sensitive numeric instruction site inside Drive Build.
///
/// Variants are semantic names for fixed raw addresses, not a portable claim
/// about VMX exception state. Keeping sites distinct lets a trace backend catch
/// merged, eager, skipped, or reordered instructions even when operands match.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationDriveDotSite {
    QuaternionA,
    QuaternionB,
    LinearHardLengthSquared,
    LinearBiasUp,
    LinearBiasRight,
    LinearBiasAt,
    AngularRightOmega,
    AngularRightAcceleration,
    AngularUpOmega,
    AngularUpAcceleration,
    AngularAtOmega,
    AngularAtAcceleration,
    AngularHardLengthSquared,
    BodyAAngularUp,
    BodyAAngularRight,
    BodyAAngularAt,
    BodyALinearRight,
    BodyALinearUp,
    BodyALinearAt,
    BodyBAngularUp,
    BodyBAngularRight,
    BodyBAngularAt,
    BodyBLinearRight,
    BodyBLinearUp,
    BodyBLinearAt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationDriveReciprocalSite {
    LinearSoftDynamics,
    LinearSoftSpring,
    LinearSoftDamping,
    LinearNonSoftSpring,
    AngularSoftDynamics,
    AngularSoftSpring,
    AngularSoftDamping,
    AngularNonSoftSpring,
    FinalLinear,
    FinalAngular,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationDriveReciprocalSqrtSite {
    LinearHardLength,
    AngularAxisNormalization,
    AngularHardLength,
}

/// Private boundary for the three hardware-sensitive VMX operations in Build.
///
/// The caller receives the exact input at the exact taken site. Proven Newton
/// refinement and all other arithmetic remain owned by the composer.
trait SimulationDriveNumericBackend {
    type Error;

    fn reciprocal_estimate(
        &mut self,
        site: SimulationDriveReciprocalSite,
        input: [f32; 4],
    ) -> Result<[f32; 4], Self::Error>;

    fn reciprocal_sqrt_estimate(
        &mut self,
        site: SimulationDriveReciprocalSqrtSite,
        input: [f32; 4],
    ) -> Result<[f32; 4], Self::Error>;

    fn sum3(
        &mut self,
        site: SimulationDriveDotSite,
        left: [f32; 3],
        right: [f32; 3],
    ) -> Result<f32, Self::Error>;
}

/// Deterministic ordinary-finite PC policy for Build's numeric boundary.
///
/// This deliberately does not claim Xenon estimate LUT, denormal, NaN,
/// infinity, signed-zero, exception, or FPSCR parity.
#[derive(Default)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the pool-backed prepared Drive batch"
    )
)]
struct SimulationDriveHostNumericBackend;

impl SimulationDriveNumericBackend for SimulationDriveHostNumericBackend {
    type Error = std::convert::Infallible;

    fn reciprocal_estimate(
        &mut self,
        _site: SimulationDriveReciprocalSite,
        input: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        Ok(input.map(f32::recip))
    }

    fn reciprocal_sqrt_estimate(
        &mut self,
        _site: SimulationDriveReciprocalSqrtSite,
        input: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        Ok(input.map(|value| value.sqrt().recip()))
    }

    fn sum3(
        &mut self,
        _site: SimulationDriveDotSite,
        left: [f32; 3],
        right: [f32; 3],
    ) -> Result<f32, Self::Error> {
        Ok(drive_effective_mass_dot3_host(left, right))
    }
}

/// Legacy explicit-seed adapter retained only for the independent Build oracle.
/// Future pool-backed composition will use the lazy ordinary-finite backend instead.
struct SimulationDriveExplicitSeedBackend {
    seeds: SimulationDriveBuildEstimateSeeds,
}

impl SimulationDriveNumericBackend for SimulationDriveExplicitSeedBackend {
    type Error = std::convert::Infallible;

    fn reciprocal_estimate(
        &mut self,
        site: SimulationDriveReciprocalSite,
        _input: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        let seed = match site {
            SimulationDriveReciprocalSite::LinearSoftDynamics => {
                self.seeds.linear_coefficients.dynamics
            }
            SimulationDriveReciprocalSite::LinearSoftSpring
            | SimulationDriveReciprocalSite::LinearNonSoftSpring => {
                self.seeds.linear_coefficients.spring
            }
            SimulationDriveReciprocalSite::LinearSoftDamping => {
                self.seeds.linear_coefficients.damping
            }
            SimulationDriveReciprocalSite::AngularSoftDynamics => {
                self.seeds.angular.coefficients.dynamics
            }
            SimulationDriveReciprocalSite::AngularSoftSpring
            | SimulationDriveReciprocalSite::AngularNonSoftSpring => {
                self.seeds.angular.coefficients.spring
            }
            SimulationDriveReciprocalSite::AngularSoftDamping => {
                self.seeds.angular.coefficients.damping
            }
            SimulationDriveReciprocalSite::FinalLinear => {
                let [x, y, z] = self.seeds.final_reciprocals.linear;
                return Ok([x, y, z, x]);
            }
            SimulationDriveReciprocalSite::FinalAngular => {
                let [x, y, z] = self.seeds.final_reciprocals.angular;
                return Ok([x, y, z, x]);
            }
        };
        Ok([seed; 4])
    }

    fn reciprocal_sqrt_estimate(
        &mut self,
        site: SimulationDriveReciprocalSqrtSite,
        _input: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        match site {
            SimulationDriveReciprocalSqrtSite::LinearHardLength => {
                Ok([self.seeds.linear_hard_reciprocal_sqrt; 4])
            }
            SimulationDriveReciprocalSqrtSite::AngularAxisNormalization => {
                Ok(self.seeds.angular.axis_reciprocal_sqrt)
            }
            SimulationDriveReciprocalSqrtSite::AngularHardLength => {
                Ok([self.seeds.angular.hard_reciprocal_sqrt; 4])
            }
        }
    }

    fn sum3(
        &mut self,
        _site: SimulationDriveDotSite,
        left: [f32; 3],
        right: [f32; 3],
    ) -> Result<f32, Self::Error> {
        Ok(drive_effective_mass_dot3_host(left, right))
    }
}

/// Reconstructs Build's first value-only geometry subgraph.
///
/// Native anchors are `0x82ae4c98..0x82ae4f14`, with the later separated
/// output stores documented in the evidence packet. The exact body-times-frame
/// quaternion convention, YZXW/YZXX/ZXYX permutations, fused attachment graph,
/// and `0x3fb504f3` sqrt-two constant are preserved. Position W is ignored.
///
/// This host-bounded helper allocates nothing, branches on no input, mutates no
/// caller state, and exposes no RF-row capability. The nested host FMA used for
/// native `vmsum3fp128` is the explicit ordinary-finite reduction policy;
/// Xenon reduction rounding and exceptional VMX behavior remain unclaimed.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the remaining recovered DriveJacobian builder"
    )
)]
pub(crate) fn build_drive_geometry_host(
    body_a: SimulationDriveBodyGeometryInput,
    body_b: SimulationDriveBodyGeometryInput,
    frames: SimulationDriveFrames,
) -> SimulationDriveGeometryHost {
    let quaternion_a = compose_drive_quaternion_host(body_a.quaternion, frames.quaternion_a);
    let quaternion_b = compose_drive_quaternion_host(body_b.quaternion, frames.quaternion_b);
    let packed_linear_axes = drive_basis_host(quaternion_b);
    let [linear_right, linear_up, linear_at] = packed_linear_axes;
    SimulationDriveGeometryHost {
        attachment_offset_a: drive_attachment_offset_host(body_a, frames.position_a),
        attachment_offset_b: drive_attachment_offset_host(body_b, frames.position_b),
        linear_right: first3(linear_right),
        linear_up: first3(linear_up),
        linear_at: first3(linear_at),
        packed_linear_axes,
        rqd: SimulationDriveJacobianRqd::create(quaternion_a, quaternion_b),
    }
}

/// Reconstructs Build's child-A/parent-B point prediction.
///
/// Native anchors are `0x82ae4ea4..0x82ae4f90`, `0x82ae5098`,
/// `0x82ae515c`, and `0x82ae5170`. The cross-add preserves this Build slice's
/// N-then-M graph, which is algebraically equivalent to the Horus consumer's
/// helper but rounds differently. Acceleration is scaled sequentially as
/// `(point_acceleration * h) * h`, never grouped as `point_acceleration*(h*h)`.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the remaining recovered DriveJacobian builder"
    )
)]
#[allow(
    clippy::similar_names,
    reason = "source-shaped A/B point names preserve the native operand roles"
)]
pub(crate) fn predict_drive_points_host(
    body_a: SimulationDrivePredictionBodyInput,
    body_b: SimulationDrivePredictionBodyInput,
    attachment_offset_a: [f32; 3],
    attachment_offset_b: [f32; 3],
    time_step: f32,
) -> SimulationDrivePointPredictionHost {
    let position_a: [f32; 3] =
        std::array::from_fn(|lane| body_a.center_of_mass[lane] + attachment_offset_a[lane]);
    let position_b: [f32; 3] =
        std::array::from_fn(|lane| body_b.center_of_mass[lane] + attachment_offset_b[lane]);
    let velocity_a =
        drive_prediction_cross_add_host(body_a.omega, attachment_offset_a, body_a.velocity);
    let velocity_b =
        drive_prediction_cross_add_host(body_b.omega, attachment_offset_b, body_b.velocity);
    let acceleration_a =
        drive_prediction_cross_add_host(body_a.torque, attachment_offset_a, body_a.force);
    let acceleration_b =
        drive_prediction_cross_add_host(body_b.torque, attachment_offset_b, body_b.force);
    let velocity_a_h = velocity_a.map(|value| value * time_step);
    let velocity_b_h = velocity_b.map(|value| value * time_step);
    let acceleration_a_h = acceleration_a.map(|value| value * time_step);
    let acceleration_b_h = acceleration_b.map(|value| value * time_step);
    let acceleration_a_h2 = acceleration_a_h.map(|value| value * time_step);
    let acceleration_b_h2 = acceleration_b_h.map(|value| value * time_step);

    SimulationDrivePointPredictionHost {
        position_error: std::array::from_fn(|lane| position_b[lane] - position_a[lane]),
        velocity_delta_h: std::array::from_fn(|lane| velocity_b_h[lane] - velocity_a_h[lane]),
        acceleration_delta_h2: std::array::from_fn(|lane| {
            acceleration_b_h2[lane] - acceleration_a_h2[lane]
        }),
    }
}

/// Applies the recovered linear coefficients after the separate HARD gate.
///
/// `position_error_after_hard_gate` is explicit so callers cannot move the
/// limiter after bias construction. Native anchors are
/// `0x82ae51c8..0x82ae5238`. Projection executes Up, Right, At and is returned
/// in the packed semantic order Right, Up, At. The nested host dot is the
/// documented ordinary-finite policy for native `vmsum3fp128`.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the remaining recovered DriveJacobian builder"
    )
)]
pub(crate) fn build_drive_linear_bias_host(
    prediction: SimulationDrivePointPredictionHost,
    position_error_after_hard_gate: [f32; 3],
    coefficients: SimulationDriveCoefficients,
    linear_axes: [[f32; 3]; 3],
) -> SimulationDriveLinearBiasHost {
    let velocity_term = prediction
        .velocity_delta_h
        .map(|value| value * coefficients.damping_factor);
    let position_velocity: [f32; 3] = std::array::from_fn(|lane| {
        position_error_after_hard_gate[lane]
            .mul_add(coefficients.spring_factor, velocity_term[lane])
    });
    let bias = std::array::from_fn(|lane| {
        prediction.acceleration_delta_h2[lane]
            .mul_add(coefficients.dynamics, position_velocity[lane])
    });

    let up = drive_effective_mass_dot3_host(bias, linear_axes[1]);
    let right = drive_effective_mass_dot3_host(bias, linear_axes[0]);
    let at = drive_effective_mass_dot3_host(bias, linear_axes[2]);
    SimulationDriveLinearBiasHost {
        bias,
        projection: [right, up, at],
    }
}

fn drive_prediction_cross_add_host(
    source: [f32; 3],
    attachment_offset: [f32; 3],
    base: [f32; 3],
) -> [f32; 3] {
    let source_yzx = [source[1], source[2], source[0]];
    let source_zxy = [source[2], source[0], source[1]];
    let offset_yzx = [
        attachment_offset[1],
        attachment_offset[2],
        attachment_offset[0],
    ];
    let offset_zxy = [
        attachment_offset[2],
        attachment_offset[0],
        attachment_offset[1],
    ];
    std::array::from_fn(|lane| {
        let first = (-offset_yzx[lane]).mul_add(source_zxy[lane], base[lane]);
        offset_zxy[lane].mul_add(source_yzx[lane], first)
    })
}

/// Reconstructs Build's packed angular axes, angular prediction, and bias.
///
/// Native anchors are `0x82ae5138..0x82ae54a4`. `dot3` is the replaceable
/// `vmsum3fp128` boundary and is called six times in exact Ri-omega,
/// Ri-acceleration, Up-omega, Up-acceleration, At-omega, At-acceleration order;
/// HARD adds one final `(angular_error, angular_error)` call. All estimate seeds
/// are explicit because Xenon estimate result bits remain unproven.
///
/// The ordinary-finite host branch spelling preserves native's one global
/// fallback decision: equality in any clamped q XYZ lane selects all three
/// complete packed L axes. NaN/min/max/vsel behavior, denormals, exceptional
/// fused arithmetic, and FPSCR side effects remain outside the claim.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the remaining recovered DriveJacobian builder"
    )
)]
#[allow(
    clippy::float_cmp,
    clippy::manual_clamp,
    clippy::similar_names,
    clippy::too_many_arguments,
    reason = "source-shaped comparisons, clamp order, and A/B operand roles are intentional"
)]
pub(crate) fn build_drive_angular_host<D>(
    rqd: &SimulationDriveJacobianRqd,
    packed_linear_axes: [[f32; 4]; 3],
    body_a: SimulationDriveAngularBodyInput,
    body_b: SimulationDriveAngularBodyInput,
    params: SimulationDriveDynamicsParams,
    time_step: f32,
    seeds: SimulationDriveAngularEstimateSeeds,
    mut dot3: D,
) -> SimulationDriveAngularBuildHost
where
    D: FnMut([f32; 3], [f32; 3]) -> f32,
{
    let strength_h = params.strength * time_step;
    let damping_h = params.damping * time_step;
    let strength_h2 = strength_h * time_step;

    let rows = rqd.rows();
    let q = [rows[0][3], rows[1][3], rows[2][3], 0.0];
    let clamped_square = q.map(|value| (value * value).max(0.0).min(1.0));
    let used_global_fallback =
        clamped_square[0] == 1.0 || clamped_square[1] == 1.0 || clamped_square[2] == 1.0;
    let normalization_denominator = if used_global_fallback {
        [1.0; 4]
    } else {
        clamped_square.map(|value| 1.0 - value)
    };
    let inverse_length: [f32; 4] = std::array::from_fn(|lane| {
        refine_drive_reciprocal_sqrt(
            normalization_denominator[lane],
            seeds.axis_reciprocal_sqrt[lane],
        )
    });
    let angular_axes = std::array::from_fn(|axis| {
        let candidate = if used_global_fallback {
            packed_linear_axes[axis]
        } else {
            [rows[axis][0], rows[axis][1], rows[axis][2], rows[axis][0]]
        };
        candidate.map(|value| value * inverse_length[axis])
    });
    let angular_error_full: [f32; 4] =
        std::array::from_fn(|lane| (q[lane] * inverse_length[lane]) * 2.0);
    let angular_error = first3(angular_error_full);

    let torque_delta: [f32; 3] =
        std::array::from_fn(|lane| body_b.torque[lane] - body_a.torque[lane]);
    let omega_delta: [f32; 3] = std::array::from_fn(|lane| body_b.omega[lane] - body_a.omega[lane]);
    let acceleration_h = torque_delta.map(|value| value * time_step);
    let omega_h = omega_delta.map(|value| value * time_step);
    let acceleration_h2 = acceleration_h.map(|value| value * time_step);

    let right = first3(angular_axes[0]);
    let up = first3(angular_axes[1]);
    let at = first3(angular_axes[2]);
    let omega_right = dot3(right, omega_h);
    let acceleration_right = dot3(right, acceleration_h2);
    let omega_up = dot3(up, omega_h);
    let acceleration_up = dot3(up, acceleration_h2);
    let omega_at = dot3(at, omega_h);
    let acceleration_at = dot3(at, acceleration_h2);
    let omega_projection = [omega_right, omega_up, omega_at];
    let acceleration_projection = [acceleration_right, acceleration_up, acceleration_at];

    let coefficients =
        drive_coefficients_from_damping_h(params, time_step, damping_h, seeds.coefficients);
    let (gated_error, hard_outside_limit) = if params.raw_type == HARD_DRIVE {
        let length_squared = dot3(angular_error, angular_error);
        let gate = apply_hard_drive_gate(
            params.raw_type,
            angular_error,
            length_squared,
            time_step,
            params.spring,
            seeds.hard_reciprocal_sqrt,
        );
        (gate.error, gate.outside_limit)
    } else {
        (angular_error, false)
    };
    let bias_projection = drive_angular_bias_host(
        gated_error,
        omega_projection,
        acceleration_projection,
        coefficients,
    );

    SimulationDriveAngularBuildHost {
        angular_axes,
        angular_error,
        omega_projection,
        acceleration_projection,
        bias_projection,
        coefficients,
        strength_h2,
        normalization_denominator,
        used_global_fallback,
        hard_outside_limit,
    }
}

fn drive_angular_bias_host(
    gated_error: [f32; 3],
    omega_projection: [f32; 3],
    acceleration_projection: [f32; 3],
    coefficients: SimulationDriveCoefficients,
) -> [f32; 3] {
    std::array::from_fn(|axis| {
        let damped = omega_projection[axis] * coefficients.damping_factor;
        let spring = gated_error[axis].mul_add(coefficients.spring_factor, damped);
        acceleration_projection[axis].mul_add(coefficients.dynamics, spring)
    })
}

/// Reconstructs Build's state-gated effective-mass graph before `vrefp`.
///
/// Native anchors are `0x82ae5474..0x82ae5748`. A and B are gated
/// independently by state bit 4. For each admitted body this preserves the
/// source `Y` permutation/negative-FMA cross graph, symmetric-tensor column
/// expansion, tensor multiply FMA order, and A-then-B denominator addition.
///
/// The nested host `dot3` is the explicit ordinary-finite policy for native
/// `vmsum3fp128`; its internal Xenon reduction rounding and exceptional VMX
/// behavior remain unclaimed. Zero denominators are returned unchanged: the
/// later unconditional native `vrefp(0)` behavior remains a hardware boundary.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the remaining recovered DriveJacobian builder"
    )
)]
pub(crate) fn build_drive_effective_mass_host(
    body_a: SimulationDriveEffectiveMassBodyInput,
    body_b: SimulationDriveEffectiveMassBodyInput,
    linear_axes: [[f32; 3]; 3],
    angular_axes: [[f32; 3]; 3],
) -> SimulationDriveEffectiveMassHost {
    let contribution_a = drive_effective_mass_body_host(body_a, linear_axes, angular_axes);
    let contribution_b = drive_effective_mass_body_host(body_b, linear_axes, angular_axes);
    let linear_a = contribution_a
        .linear
        .map(|value| value + contribution_a.inverse_mass);
    let linear_b = contribution_b
        .linear
        .map(|value| value + contribution_b.inverse_mass);

    SimulationDriveEffectiveMassHost {
        inverse_inertia_a: contribution_a.inverse_inertia,
        inverse_mass_a: contribution_a.inverse_mass,
        inverse_inertia_b: contribution_b.inverse_inertia,
        inverse_mass_b: contribution_b.inverse_mass,
        linear_denominator: std::array::from_fn(|axis| linear_a[axis] + linear_b[axis]),
        angular_denominator: std::array::from_fn(|axis| {
            contribution_a.angular[axis] + contribution_b.angular[axis]
        }),
    }
}

/// Reconstructs Build's reciprocal and final numeric-store tail.
///
/// Native anchors are `0x82ae574c..0x82ae5904`. Linear and angular reciprocal
/// estimates are explicit inputs; both two-round Newton graphs are
/// unconditional, including for zero denominators. LT/TT are transposes of
/// the source axes scaled by their corresponding reciprocal denominators.
///
/// This helper returns only plain values and neither binds solver rows nor
/// publishes a batch record. Xenon `vrefp` seed bits, exceptional vector
/// arithmetic, denormal handling, and FPSCR behavior remain unclaimed.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the remaining recovered DriveJacobian builder"
    )
)]
pub(crate) fn finalize_drive_jacobian_host(
    input: SimulationDriveFinalizationInput,
    seeds: SimulationDriveFinalReciprocalSeeds,
) -> SimulationDriveFinalizedHost {
    let packed_inverse_inertia_a = input
        .effective_mass
        .inverse_inertia_a
        .map(|column| pack_xyz_word(column, input.effective_mass.inverse_mass_a));
    let packed_inverse_inertia_b = input
        .effective_mass
        .inverse_inertia_b
        .map(|column| pack_xyz_word(column, input.effective_mass.inverse_mass_b));
    let (inverse_linear_denominator, inverse_angular_denominator) = refine_drive_final_reciprocals(
        input.effective_mass.linear_denominator,
        input.effective_mass.angular_denominator,
        seeds.linear,
        seeds.angular,
    );

    let position_offset = std::array::from_fn(|axis| {
        inverse_linear_denominator[axis] * input.linear_bias_projection[axis]
    });
    let angular_offset = std::array::from_fn(|axis| {
        inverse_angular_denominator[axis] * input.angular_bias_projection[axis]
    });
    let position_limit =
        inverse_linear_denominator.map(|reciprocal| reciprocal * input.linear_strength_h2);
    let angular_limit =
        inverse_angular_denominator.map(|reciprocal| reciprocal * input.angular_strength_h2);
    let linear_transpose = std::array::from_fn(|row| {
        std::array::from_fn(|axis| {
            input.packed_linear_axes[axis][row] * inverse_linear_denominator[axis]
        })
    });
    let angular_transpose = std::array::from_fn(|row| {
        std::array::from_fn(|axis| {
            input.packed_angular_axes[axis][row] * inverse_angular_denominator[axis]
        })
    });

    SimulationDriveFinalizedHost {
        packed_linear_axes: input.packed_linear_axes,
        packed_angular_axes: input.packed_angular_axes,
        packed_inverse_inertia_a,
        packed_inverse_inertia_b,
        inverse_linear_denominator,
        inverse_angular_denominator,
        position_offset,
        angular_offset,
        position_limit,
        angular_limit,
        linear_transpose,
        angular_transpose,
        angular_transpose_right_pad: angular_transpose[0][0],
        position_impulse: [0.0; 3],
        linear_dynamics: input.linear_dynamics,
        angular_impulse: [0.0; 3],
        angular_dynamics: input.angular_dynamics,
    }
}

/// Oracle adapter that composes one Drive Jacobian from explicit seed values.
///
/// This preserves the older independent 96-word golden interface. Live pool
/// integration must use the lazy backend path below because several estimate
/// inputs exist only after earlier Build stages and untaken instructions must
/// not be requested. The inert binding words remain numeric placeholders only.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the private Drive batch preparation seam"
    )
)]
fn compose_drive_jacobian_with_explicit_seeds(
    input: &SimulationDriveBuildInput,
    seeds: SimulationDriveBuildEstimateSeeds,
) -> SimulationDriveJacobian {
    let mut backend = SimulationDriveExplicitSeedBackend { seeds };
    match compose_drive_jacobian_with_backend(input, &mut backend) {
        Ok(record) => record,
        Err(never) => match never {},
    }
}

/// Composes every numeric field while requesting hardware-bound results at
/// their exact typed, taken sites and in raw instruction order.
///
/// The backend receives values only. This function validates no identity,
/// retains no capability, and returns no partial record after a backend error.
#[allow(
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "source-shaped body A/child and body B/parent roles are intentional"
)]
fn compose_drive_jacobian_with_backend<B: SimulationDriveNumericBackend>(
    input: &SimulationDriveBuildInput,
    backend: &mut B,
) -> Result<SimulationDriveJacobian, B::Error> {
    let geometry = build_drive_geometry_with_backend(
        drive_build_geometry_input(input.body_a),
        drive_build_geometry_input(input.body_b),
        input.frames,
        backend,
    )?;
    let prediction = predict_drive_points_host(
        drive_build_prediction_input(input.body_a),
        drive_build_prediction_input(input.body_b),
        geometry.attachment_offset_a,
        geometry.attachment_offset_b,
        input.time_step,
    );
    let linear_coefficients = drive_coefficients_with_backend(
        input.dynamics.linear,
        input.time_step,
        SimulationDriveReciprocalSite::LinearSoftDynamics,
        SimulationDriveReciprocalSite::LinearSoftSpring,
        SimulationDriveReciprocalSite::LinearSoftDamping,
        SimulationDriveReciprocalSite::LinearNonSoftSpring,
        backend,
    )?;
    let linear_error = if input.dynamics.linear.raw_type == HARD_DRIVE {
        let length_squared = backend.sum3(
            SimulationDriveDotSite::LinearHardLengthSquared,
            prediction.position_error,
            prediction.position_error,
        )?;
        let reciprocal_sqrt_seed = backend.reciprocal_sqrt_estimate(
            SimulationDriveReciprocalSqrtSite::LinearHardLength,
            [length_squared; 4],
        )?[0];
        apply_hard_drive_gate(
            input.dynamics.linear.raw_type,
            prediction.position_error,
            length_squared,
            input.time_step,
            input.dynamics.linear.spring,
            reciprocal_sqrt_seed,
        )
        .error
    } else {
        prediction.position_error
    };
    let linear_axes = [
        geometry.linear_right,
        geometry.linear_up,
        geometry.linear_at,
    ];
    let linear_bias = build_drive_linear_bias_with_backend(
        prediction,
        linear_error,
        linear_coefficients,
        linear_axes,
        backend,
    )?;
    let angular = build_drive_angular_with_backend(
        &geometry.rqd,
        geometry.packed_linear_axes,
        drive_build_angular_input(input.body_a),
        drive_build_angular_input(input.body_b),
        input.dynamics.angular,
        input.time_step,
        backend,
    )?;
    let angular_axes = angular.angular_axes.map(first3);
    let effective_mass = build_drive_effective_mass_with_backend(
        drive_build_effective_mass_input(input.body_a, geometry.attachment_offset_a),
        drive_build_effective_mass_input(input.body_b, geometry.attachment_offset_b),
        linear_axes,
        angular_axes,
        backend,
    )?;
    let linear_seed = backend.reciprocal_estimate(
        SimulationDriveReciprocalSite::FinalLinear,
        pack_xyz_word(
            effective_mass.linear_denominator,
            effective_mass.linear_denominator[0],
        ),
    )?;
    let angular_seed = backend.reciprocal_estimate(
        SimulationDriveReciprocalSite::FinalAngular,
        pack_xyz_word(
            effective_mass.angular_denominator,
            effective_mass.angular_denominator[0],
        ),
    )?;
    let linear_strength_h = input.dynamics.linear.strength * input.time_step;
    let finalized = finalize_drive_jacobian_host(
        SimulationDriveFinalizationInput {
            effective_mass,
            packed_linear_axes: geometry.packed_linear_axes,
            packed_angular_axes: angular.angular_axes,
            linear_bias_projection: linear_bias.projection,
            angular_bias_projection: angular.bias_projection,
            linear_strength_h2: linear_strength_h * input.time_step,
            angular_strength_h2: angular.strength_h2,
            linear_dynamics: linear_coefficients.dynamics,
            angular_dynamics: angular.coefficients.dynamics,
        },
        SimulationDriveFinalReciprocalSeeds {
            linear: first3(linear_seed),
            angular: first3(angular_seed),
        },
    );

    Ok(assemble_drive_jacobian_record(
        geometry.attachment_offset_a,
        geometry.attachment_offset_b,
        input.spy,
        input.bindings,
        &finalized,
    ))
}

/// Copies and composes the admitted Active Drive subsequence into scratch.
///
/// Native `Simulation::DriveBatchBuild` at `0x82ae79e0..0x82ae7a74`
/// publishes `DR_Count` before each void Build. The safe host instead obtains
/// a complete occurrence-bound snapshot, checks every private token, reserves
/// the complete output, and only then makes the first fallible numeric request.
/// Any error returns no batch and this seam never mutates either pool or RF.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for checked Drive RF revalidation and publication"
    )
)]
fn try_prepare_active_drive_jacobian_batch_with_backend<B: SimulationDriveNumericBackend>(
    drive_pool: &SimulationDrivePool,
    body_pool: &SimulationRigidBodyPool,
    backend: &mut B,
) -> Result<SimulationPreparedDriveJacobianBatch, SimulationDriveBatchPreparationError<B::Error>> {
    let snapshots = drive_pool
        .try_active_build_snapshots(body_pool)
        .map_err(SimulationDriveBatchPreparationError::Pool)?;
    try_prepare_drive_jacobian_snapshots_with_backend(
        SimulationDrivePreparedBatchSource {
            drive_pool: drive_pool.authority(),
            body_pool: body_pool.authority(),
        },
        &snapshots,
        backend,
    )
}

/// Ordinary-finite host preparation policy for one complete Active batch.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for checked Drive RF revalidation and publication"
    )
)]
fn try_prepare_active_drive_jacobian_batch(
    drive_pool: &SimulationDrivePool,
    body_pool: &SimulationRigidBodyPool,
) -> Result<
    SimulationPreparedDriveJacobianBatch,
    SimulationDriveBatchPreparationError<std::convert::Infallible>,
> {
    let mut backend = SimulationDriveHostNumericBackend;
    try_prepare_active_drive_jacobian_batch_with_backend(drive_pool, body_pool, &mut backend)
}

/// Builds the dense Drive records and all ten stable island subsequences as one
/// unpublished, occurrence-qualified, phase-sealed and frame-bound value.
///
/// Native table facts are frozen at `0x82ae3fe4..0x82ae40e0`: admitted records
/// resolve B's signed label then A's, select B unless it is exactly `-1`, and
/// append once in dense Active-ring order. Typed label/occurrence/range errors,
/// pre-reservation, frame identity, and all-or-error publication are safe-host
/// policy. No numeric backend call occurs before every label and destination
/// capacity is accepted.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private island Drive-table preparation boundary")
)]
fn try_prepare_active_drive_island_batch_with_backend<B: SimulationDriveNumericBackend>(
    drive_pool: &SimulationDrivePool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    island_map: SimulationDriveIslandMapSource<'_>,
    backend: &mut B,
) -> Result<SimulationPreparedDriveIslandBatch, SimulationDriveIslandBatchPreparationError<B::Error>>
{
    try_prepare_active_drive_island_batch_with_backend_and_reservation(
        drive_pool,
        phase,
        island_map,
        backend,
        |inputs, capacity| inputs.try_reserve_exact(capacity).map_err(|_| ()),
        reserve_drive_records,
        |_, entries, capacity| entries.try_reserve_exact(capacity).map_err(|_| ()),
    )
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "test-injected island Drive-table allocation boundary"
    )
)]
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the uninterrupted preflight/allocation/backend chronology is the safety contract"
)]
fn try_prepare_active_drive_island_batch_with_backend_and_reservation<
    B: SimulationDriveNumericBackend,
>(
    drive_pool: &SimulationDrivePool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    island_map: SimulationDriveIslandMapSource<'_>,
    backend: &mut B,
    reserve_inputs: impl FnOnce(
        &mut Vec<SimulationPreparedDriveIslandBuildInput>,
        usize,
    ) -> Result<(), ()>,
    reserve_records: impl FnOnce(&mut Vec<SimulationPreparedDriveJacobian>, usize) -> Result<(), ()>,
    mut reserve_island: impl FnMut(
        usize,
        &mut Vec<SimulationPreparedDriveIslandEntry>,
        usize,
    ) -> Result<(), ()>,
) -> Result<SimulationPreparedDriveIslandBatch, SimulationDriveIslandBatchPreparationError<B::Error>>
{
    if drive_pool.body_pool_authority() != phase.body_pool_authority() {
        return Err(SimulationDriveIslandBatchPreparationError::Pool(
            SimulationDrivePoolError::BodyPoolMismatch,
        ));
    }
    if island_map
        .body_pool()
        .is_some_and(|authority| authority != phase.body_pool_authority())
    {
        return Err(SimulationDriveIslandBatchPreparationError::IslandMapBodyPoolMismatch);
    }
    let frame = phase.frame_identity();
    if island_map.frame() != frame {
        return Err(SimulationDriveIslandBatchPreparationError::IslandMapFrameMismatch);
    }

    let mut snapshots = drive_pool
        .try_active_build_snapshots_for_phase(phase)
        .map_err(SimulationDriveIslandBatchPreparationError::Pool)?;
    snapshots.retain(|snapshot| {
        !phase.is_externally_driven(snapshot.body_a.observation())
            && !phase.is_externally_driven(snapshot.body_b.observation())
    });
    let capacity = snapshots.len();
    let mut island_counts = [0_usize; SIMULATION_ISLAND_CAPACITY];

    for (ordinal, snapshot) in snapshots.iter().enumerate() {
        try_drive_record_bindings(snapshot).map_err(|(kind, slot)| {
            SimulationDriveIslandBatchPreparationError::BindingTokenOverflow {
                drive: snapshot.drive,
                kind,
                slot,
            }
        })?;
        let _ = u32::try_from(ordinal).map_err(|_| {
            SimulationDriveIslandBatchPreparationError::DenseOrdinalOverflow { ordinal }
        })?;
    }
    for (ordinal, snapshot) in snapshots.iter().enumerate() {
        let island = try_selected_drive_island(island_map, ordinal, snapshot)?;
        let island_u8 = u8::try_from(island).expect("validated island is in 0..10");
        island_counts[island] = island_counts[island].checked_add(1).ok_or(
            SimulationDriveIslandBatchPreparationError::IslandCountOverflow {
                island: island_u8,
                count: island_counts[island],
            },
        )?;
    }

    if capacity == 0 {
        return Ok(SimulationPreparedDriveIslandBatch {
            source: SimulationDrivePreparedBatchSource {
                drive_pool: drive_pool.authority(),
                body_pool: phase.body_pool_authority(),
            },
            frame,
            phase: phase.phase_identity(),
            records: Vec::new(),
            islands: std::array::from_fn(|_| Vec::new()),
        });
    }

    let mut inputs = Vec::new();
    reserve_inputs(&mut inputs, capacity).map_err(|()| {
        SimulationDriveIslandBatchPreparationError::StagingAllocationFailed { capacity }
    })?;
    if inputs.capacity() < capacity {
        return Err(
            SimulationDriveIslandBatchPreparationError::StagingAllocationFailed { capacity },
        );
    }
    for (ordinal, snapshot) in snapshots.iter().enumerate() {
        let Ok(island) = try_selected_drive_island::<B::Error>(island_map, ordinal, snapshot)
        else {
            unreachable!("immutable island map and snapshot were fully preflighted");
        };
        let bindings = try_drive_record_bindings(snapshot).map_err(|(kind, slot)| {
            SimulationDriveIslandBatchPreparationError::BindingTokenOverflow {
                drive: snapshot.drive,
                kind,
                slot,
            }
        })?;
        inputs.push(SimulationPreparedDriveIslandBuildInput {
            island,
            prepared: SimulationPreparedDriveBuildInput {
                drive: snapshot.drive,
                body_a: snapshot.body_a.observation(),
                body_b: snapshot.body_b.observation(),
                input: drive_build_input_from_snapshot(snapshot, bindings),
            },
        });
    }

    let mut records = Vec::new();
    reserve_records(&mut records, capacity).map_err(|()| {
        SimulationDriveIslandBatchPreparationError::RecordAllocationFailed { capacity }
    })?;
    if records.capacity() < capacity {
        return Err(
            SimulationDriveIslandBatchPreparationError::RecordAllocationFailed { capacity },
        );
    }
    let mut islands: [Vec<SimulationPreparedDriveIslandEntry>; SIMULATION_ISLAND_CAPACITY] =
        std::array::from_fn(|_| Vec::new());
    for (island, (&required, entries)) in island_counts.iter().zip(&mut islands).enumerate() {
        if required == 0 {
            continue;
        }
        let island_u8 = u8::try_from(island).expect("physical island enumeration is in 0..10");
        reserve_island(island, entries, required).map_err(|()| {
            SimulationDriveIslandBatchPreparationError::IslandAllocationFailed {
                island: island_u8,
                capacity: required,
            }
        })?;
        if entries.capacity() < required {
            return Err(
                SimulationDriveIslandBatchPreparationError::IslandAllocationFailed {
                    island: island_u8,
                    capacity: required,
                },
            );
        }
    }

    for (ordinal, staged) in inputs.into_iter().enumerate() {
        let record = compose_drive_jacobian_with_backend(&staged.prepared.input, backend).map_err(
            |source| SimulationDriveIslandBatchPreparationError::Numeric {
                ordinal,
                drive: staged.prepared.drive,
                source,
            },
        )?;
        let dense_ordinal = u32::try_from(ordinal)
            .expect("dense ordinal was checked before every allocation and numeric call");
        records.push(SimulationPreparedDriveJacobian {
            drive: staged.prepared.drive,
            body_a: staged.prepared.body_a,
            body_b: staged.prepared.body_b,
            record,
        });
        islands[staged.island].push(SimulationPreparedDriveIslandEntry {
            dense_ordinal,
            drive: staged.prepared.drive,
            body_a: staged.prepared.body_a,
            body_b: staged.prepared.body_b,
        });
    }

    debug_assert_eq!(islands.iter().map(Vec::len).sum::<usize>(), records.len());
    Ok(SimulationPreparedDriveIslandBatch {
        source: SimulationDrivePreparedBatchSource {
            drive_pool: drive_pool.authority(),
            body_pool: phase.body_pool_authority(),
        },
        frame,
        phase: phase.phase_identity(),
        records,
        islands,
    })
}

fn try_drive_island_label<E>(
    source: SimulationDriveIslandMapSource<'_>,
    ordinal: usize,
    drive: SimulationDriveIdentity,
    endpoint: SimulationDriveEndpoint,
    body: SimulationRigidBodyObservation,
) -> Result<i8, SimulationDriveIslandBatchPreparationError<E>> {
    let SimulationDriveIslandMapSource::Present(map) = source else {
        return Ok(0);
    };
    let Some((mapped_body, raw_label)) = map.assignment(body) else {
        return Err(
            SimulationDriveIslandBatchPreparationError::MissingIslandLabel {
                ordinal,
                drive,
                endpoint,
                body,
            },
        );
    };
    if mapped_body != body {
        return Err(
            SimulationDriveIslandBatchPreparationError::IslandLabelOccurrenceMismatch {
                ordinal,
                drive,
                endpoint,
                expected_body: body,
                mapped_body,
            },
        );
    }
    Ok(raw_label)
}

fn try_selected_drive_island<E>(
    source: SimulationDriveIslandMapSource<'_>,
    ordinal: usize,
    snapshot: &SimulationDriveBuildSnapshot,
) -> Result<usize, SimulationDriveIslandBatchPreparationError<E>> {
    let body_b = snapshot.body_b.observation();
    let raw_body_b = try_drive_island_label(
        source,
        ordinal,
        snapshot.drive,
        SimulationDriveEndpoint::Parent,
        body_b,
    )?;
    let body_a = snapshot.body_a.observation();
    let raw_body_a = try_drive_island_label(
        source,
        ordinal,
        snapshot.drive,
        SimulationDriveEndpoint::Child,
        body_a,
    )?;
    let selected = if raw_body_b == -1 {
        raw_body_a
    } else {
        raw_body_b
    };
    let Ok(island) = usize::try_from(selected) else {
        return Err(
            SimulationDriveIslandBatchPreparationError::InvalidSelectedIsland {
                ordinal,
                drive: snapshot.drive,
                raw_body_b,
                raw_body_a,
                selected,
            },
        );
    };
    if island >= SIMULATION_ISLAND_CAPACITY {
        return Err(
            SimulationDriveIslandBatchPreparationError::InvalidSelectedIsland {
                ordinal,
                drive: snapshot.drive,
                raw_body_b,
                raw_body_a,
                selected,
            },
        );
    }
    Ok(island)
}

/// Executes one test-only direct/global Horus Drive sweep over a prepared batch.
///
/// Raw `HorusPipeline` anchors are `0x82ae253c..0x82ae2814`. Records execute
/// in prepared dense order. Every Drive occurrence, semantic endpoint, private
/// token, body occurrence, and temporary plan is checked before the first
/// record/RF effect. The body-owned transaction then copies live A/B rows per
/// record and commits A0, B0, A2, B2 (`0x82ae27b4`, `0x82ae27b8`,
/// `0x82ae2804`, `0x82ae2810`). Self-pairs therefore retain native B-wins
/// behavior and later shared-body records observe earlier commits.
///
/// This is a direct arithmetic/RF evidence harness, not a runtime seam or a
/// complete Horus pass:
/// native iterations run Contact, Joint, then Drive, and observed gameplay
/// uses island-indirect record ordering. Hardware exceptional behavior and
/// native pointer-derived packed W parity remain outside the claim.
///
/// Every returned error precedes mutation. If the closed arithmetic callback
/// were to panic after changing the current record, earlier records would stay
/// committed while that current record could have new accumulators and no RF
/// stores. No panic path is caught or rolled back.
#[cfg(test)]
fn try_solve_prepared_drive_horus_sweep(
    batch: &mut SimulationPreparedDriveJacobianBatch,
    drive_pool: &SimulationDrivePool,
    body_pool: &mut SimulationRigidBodyPool,
) -> Result<(), SimulationPreparedDriveHorusError> {
    if batch.source.drive_pool != drive_pool.authority() {
        return Err(SimulationPreparedDriveHorusError::DrivePoolSourceMismatch);
    }
    if batch.source.body_pool != body_pool.authority() {
        return Err(SimulationPreparedDriveHorusError::BodyPoolSourceMismatch);
    }

    drive_pool
        .try_validate_build_bindings(
            body_pool,
            batch.records.iter().map(|prepared| {
                SimulationDriveBuildBinding::new(prepared.drive, prepared.body_a, prepared.body_b)
            }),
        )
        .map_err(SimulationPreparedDriveHorusError::Drive)?;

    for (ordinal, prepared) in batch.records.iter().enumerate() {
        let expected_a = checked_nonzero_slot_token(prepared.body_a.identity().id().index())
            .expect("prepared body-A slot token was checked before publication")
            .get();
        let expected_b = checked_nonzero_slot_token(prepared.body_b.identity().id().index())
            .expect("prepared body-B slot token was checked before publication")
            .get();
        let expected_drive = checked_nonzero_slot_token(prepared.drive.id().index())
            .expect("prepared Drive slot token was checked before publication")
            .get();
        for (kind, expected, actual) in [
            (
                SimulationDrivePreparedBindingKind::BodyA,
                expected_a,
                prepared.record.solver_delta_a.0,
            ),
            (
                SimulationDrivePreparedBindingKind::BodyB,
                expected_b,
                prepared.record.solver_delta_b.0,
            ),
            (
                SimulationDrivePreparedBindingKind::Drive,
                expected_drive,
                prepared.record.drive_slot.0,
            ),
        ] {
            if actual != expected {
                return Err(SimulationPreparedDriveHorusError::BindingMismatch {
                    ordinal,
                    drive: prepared.drive,
                    kind,
                    expected,
                    actual,
                });
            }
        }
    }

    let transaction = body_pool
        .try_begin_solver_delta_transaction(
            batch
                .records
                .iter()
                .map(|prepared| (prepared.body_a, prepared.body_b)),
        )
        .map_err(SimulationPreparedDriveHorusError::Body)?;
    transaction
        .try_apply_all(&mut batch.records, |prepared, rows| {
            let (body_a, body_b) = rows.into_rows();
            let solved = solve_drive_horus_record_host(&mut prepared.record, body_a, body_b);
            SimulationRigidBodySolverDeltaStores::new(
                solved.solver_delta_a[0],
                solved.solver_delta_b[0],
                solved.solver_delta_a[2],
                solved.solver_delta_b[2],
            )
        })
        .map_err(SimulationPreparedDriveHorusError::Body)
}

/// Executes one checked island-indirect Drive-phase sweep.
///
/// The selected table is consumed in stable stored order. Every source,
/// frame, phase identity, dense ordinal, sidecar, Drive binding, private token,
/// item selector, and body occurrence is rejected before the first record/RF effect. The
/// closed ordinary-finite callback uses Position-then-Angular accumulator
/// chronology and the RF phase retains A0, B0, A2, B2 stores.
///
/// This is not a complete Horus iteration: Contact and Joint phases and the
/// runtime outer loop remain separate. Panic retains the same prefix/current
/// record boundary as the private indexed RF executor and is not rolled back.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private checked island Drive-phase boundary")
)]
#[allow(
    clippy::too_many_lines,
    reason = "the all-preflight-before-effect island validation chronology is intentionally linear"
)]
pub(crate) fn try_solve_prepared_drive_horus_island_sweep(
    batch: &mut SimulationPreparedDriveIslandBatch,
    island: usize,
    drive_pool: &SimulationDrivePool,
    phase: &mut SimulationRigidBodyDynamicUpdatePhase<'_>,
) -> Result<(), SimulationPreparedDriveIslandHorusError> {
    try_solve_prepared_drive_horus_island_sweep_with_observer(
        batch,
        island,
        drive_pool,
        phase,
        |_, _, _| {},
    )
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "test-injected island accumulator-store boundary")
)]
#[allow(
    clippy::too_many_lines,
    reason = "the all-preflight-before-effect island validation chronology is intentionally linear"
)]
fn try_solve_prepared_drive_horus_island_sweep_with_observer(
    batch: &mut SimulationPreparedDriveIslandBatch,
    island: usize,
    drive_pool: &SimulationDrivePool,
    phase: &mut SimulationRigidBodyDynamicUpdatePhase<'_>,
    mut observe_accumulator_store: impl FnMut(
        usize,
        SimulationDriveHorusAccumulator,
        &SimulationDriveJacobian,
    ),
) -> Result<(), SimulationPreparedDriveIslandHorusError> {
    let mut plan = try_prepare_drive_horus_island_sweep(batch, island, drive_pool, phase)?;
    apply_prepared_drive_horus_island_sweep_with_observer(
        &mut plan,
        phase,
        &mut observe_accumulator_store,
    );
    Ok(())
}

/// Prevalidates and preallocates one prepared Drive-table subsequence.
#[allow(
    clippy::too_many_lines,
    reason = "the all-preflight-before-effect island validation chronology is intentionally linear"
)]
pub(crate) fn try_prepare_drive_horus_island_sweep<'batch>(
    batch: &'batch mut SimulationPreparedDriveIslandBatch,
    island: usize,
    drive_pool: &SimulationDrivePool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
) -> Result<
    SimulationRigidBodyIndexedSolverDeltaPlan<'batch, SimulationPreparedDriveJacobian>,
    SimulationPreparedDriveIslandHorusError,
> {
    validate_drive_horus_island(batch, island, drive_pool, phase)?;

    let SimulationPreparedDriveIslandBatch {
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
        .map_err(SimulationPreparedDriveIslandHorusError::Body)
}

fn validate_drive_horus_island(
    batch: &SimulationPreparedDriveIslandBatch,
    island: usize,
    drive_pool: &SimulationDrivePool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
) -> Result<(), SimulationPreparedDriveIslandHorusError> {
    if island >= SIMULATION_ISLAND_CAPACITY {
        return Err(SimulationPreparedDriveIslandHorusError::IslandOutOfRange { island });
    }
    if batch.frame != phase.frame_identity() {
        return Err(SimulationPreparedDriveIslandHorusError::FrameMismatch);
    }
    if batch.source.drive_pool != drive_pool.authority() {
        return Err(SimulationPreparedDriveIslandHorusError::DrivePoolSourceMismatch);
    }
    let body_pool_authority = phase.body_pool_authority();
    if batch.source.body_pool != body_pool_authority {
        return Err(SimulationPreparedDriveIslandHorusError::BodyPoolSourceMismatch);
    }
    if batch.phase != phase.phase_identity() {
        return Err(SimulationPreparedDriveIslandHorusError::PhaseMismatch);
    }

    let entries = &batch.islands[island];
    for (island_ordinal, entry) in entries.iter().enumerate() {
        let Some(prepared) = batch.records.get(entry.dense_ordinal as usize) else {
            return Err(
                SimulationPreparedDriveIslandHorusError::TableEntryMismatch {
                    island_ordinal,
                    dense_ordinal: entry.dense_ordinal,
                },
            );
        };
        if (entry.drive, entry.body_a, entry.body_b)
            != (prepared.drive, prepared.body_a, prepared.body_b)
        {
            return Err(
                SimulationPreparedDriveIslandHorusError::TableEntryMismatch {
                    island_ordinal,
                    dense_ordinal: entry.dense_ordinal,
                },
            );
        }
    }

    drive_pool
        .try_validate_build_bindings_for_body_pool_authority(
            body_pool_authority,
            entries.iter().map(|entry| {
                SimulationDriveBuildBinding::new(entry.drive, entry.body_a, entry.body_b)
            }),
        )
        .map_err(SimulationPreparedDriveIslandHorusError::Drive)?;

    for (island_ordinal, entry) in entries.iter().enumerate() {
        let prepared = &batch.records[entry.dense_ordinal as usize];
        let expected_a = checked_nonzero_slot_token(entry.body_a.identity().id().index())
            .expect("prepared body-A token was checked before island publication")
            .get();
        let expected_b = checked_nonzero_slot_token(entry.body_b.identity().id().index())
            .expect("prepared body-B token was checked before island publication")
            .get();
        let expected_drive = checked_nonzero_slot_token(entry.drive.id().index())
            .expect("prepared Drive token was checked before island publication")
            .get();
        for (kind, expected, actual) in [
            (
                SimulationDrivePreparedBindingKind::BodyA,
                expected_a,
                prepared.record.solver_delta_a.0,
            ),
            (
                SimulationDrivePreparedBindingKind::BodyB,
                expected_b,
                prepared.record.solver_delta_b.0,
            ),
            (
                SimulationDrivePreparedBindingKind::Drive,
                expected_drive,
                prepared.record.drive_slot.0,
            ),
        ] {
            if actual != expected {
                return Err(SimulationPreparedDriveIslandHorusError::BindingMismatch {
                    island_ordinal,
                    dense_ordinal: entry.dense_ordinal,
                    drive: entry.drive,
                    kind,
                    expected,
                    actual,
                });
            }
        }
    }

    Ok(())
}

/// Prevalidates every scheduled Drive island before any solver effect.
pub(crate) fn try_prepare_drive_horus_serial_plan<'batch>(
    batch: &'batch mut SimulationPreparedDriveIslandBatch,
    island_count: usize,
    drive_pool: &SimulationDrivePool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
) -> Result<
    SimulationPreparedDriveHorusSerialPlan<'batch>,
    SimulationPreparedDriveHorusSerialPlanError,
> {
    if island_count > SIMULATION_ISLAND_CAPACITY {
        return Err(
            SimulationPreparedDriveHorusSerialPlanError::IslandCountOutOfRange { island_count },
        );
    }
    let mut ranges = [(0_usize, 0_usize); SIMULATION_ISLAND_CAPACITY];
    let mut entry_count = 0_usize;
    for (island, range) in ranges.iter_mut().enumerate().take(island_count) {
        validate_drive_horus_island(batch, island, drive_pool, phase).map_err(|source| {
            SimulationPreparedDriveHorusSerialPlanError::Island { island, source }
        })?;
        let start = entry_count;
        entry_count = entry_count
            .checked_add(batch.islands[island].len())
            .ok_or(SimulationPreparedDriveHorusSerialPlanError::EntryCountOverflow)?;
        *range = (start, entry_count);
    }

    let SimulationPreparedDriveIslandBatch {
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
        .map_err(SimulationPreparedDriveHorusSerialPlanError::BodySequence)?;
    Ok(SimulationPreparedDriveHorusSerialPlan {
        rf,
        ranges,
        island_count,
    })
}

/// Applies one scheduled Drive range from an all-island preflight plan.
pub(crate) fn apply_prepared_drive_horus_serial_island(
    plan: &mut SimulationPreparedDriveHorusSerialPlan<'_>,
    island: usize,
    phase: &mut SimulationRigidBodyDynamicUpdatePhase<'_>,
) {
    assert!(
        island < plan.island_count,
        "Drive serial-plan island must have been scheduled"
    );
    let (start, end) = plan.ranges[island];
    phase.apply_indexed_solver_delta_plan_range(
        &mut plan.rf,
        start..end,
        solve_prepared_drive_horus_island_record,
    );
}

fn apply_prepared_drive_horus_island_sweep_with_observer(
    plan: &mut SimulationRigidBodyIndexedSolverDeltaPlan<'_, SimulationPreparedDriveJacobian>,
    phase: &mut SimulationRigidBodyDynamicUpdatePhase<'_>,
    mut observe_accumulator_store: impl FnMut(
        usize,
        SimulationDriveHorusAccumulator,
        &SimulationDriveJacobian,
    ),
) {
    let mut execution_ordinal = 0_usize;
    phase.apply_indexed_solver_delta_plan(plan, |prepared, rows| {
        let (body_a, body_b) = rows.into_rows();
        let island_ordinal = execution_ordinal;
        let solved = solve_drive_horus_island_record_with_observer(
            &mut prepared.record,
            body_a,
            body_b,
            |accumulator, record| {
                observe_accumulator_store(island_ordinal, accumulator, record);
            },
        );
        execution_ordinal += 1;
        SimulationRigidBodySolverDeltaStores::new(
            solved.solver_delta_a[0],
            solved.solver_delta_b[0],
            solved.solver_delta_a[2],
            solved.solver_delta_b[2],
        )
    });
}

/// Solves one prevalidated island Drive record and stages its four RF stores.
pub(crate) fn solve_prepared_drive_horus_island_record(
    prepared: &mut SimulationPreparedDriveJacobian,
    rows: SimulationRigidBodySolverDeltaPairRows,
) -> SimulationRigidBodySolverDeltaStores {
    let (body_a, body_b) = rows.into_rows();
    let solved = solve_drive_horus_island_record_host(&mut prepared.record, body_a, body_b);
    SimulationRigidBodySolverDeltaStores::new(
        solved.solver_delta_a[0],
        solved.solver_delta_b[0],
        solved.solver_delta_a[2],
        solved.solver_delta_b[2],
    )
}

#[cfg(test)]
pub(crate) fn prepare_drive_horus_island_batch_for_test(
    drive_pool: &SimulationDrivePool,
    phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    island_map: &SimulationRigidBodyIslandMapSnapshot,
) -> SimulationPreparedDriveIslandBatch {
    let mut backend = SimulationDriveHostNumericBackend;
    try_prepare_active_drive_island_batch_with_backend(
        drive_pool,
        phase,
        SimulationDriveIslandMapSource::Present(island_map),
        &mut backend,
    )
    .unwrap_or_else(|error| panic!("Drive Horus test batch preparation failed: {error:?}"))
}

#[cfg(test)]
pub(crate) fn prepared_drive_horus_batch_words_for_test(
    batch: &SimulationPreparedDriveIslandBatch,
) -> Vec<[[u32; 4]; 24]> {
    batch
        .records
        .iter()
        .map(|prepared| drive_record_words_for_test(&prepared.record))
        .collect()
}

#[cfg(test)]
fn drive_record_words_for_test(value: &SimulationDriveJacobian) -> [[u32; 4]; 24] {
    let row = |values: [f32; 3], word: u32| {
        [
            values[0].to_bits(),
            values[1].to_bits(),
            values[2].to_bits(),
            word,
        ]
    };
    [
        row(value.attachment_offset_a, value.solver_delta_a.0),
        row(value.attachment_offset_b, value.solver_delta_b.0),
        row(value.position_impulse, value.linear_dynamics.to_bits()),
        row(value.angular_impulse, value.angular_dynamics.to_bits()),
        row(value.linear_transpose_right, value.spy),
        row(value.angular_transpose_right, value.pad_0.to_bits()),
        row(value.linear_transpose_up, value.position_limit_x.to_bits()),
        row(value.angular_transpose_up, value.position_limit_y.to_bits()),
        row(value.linear_transpose_at, value.angular_limit_x.to_bits()),
        row(value.angular_transpose_at, value.angular_limit_y.to_bits()),
        row(value.position_offset, value.position_limit_z.to_bits()),
        row(value.angular_offset, value.angular_limit_z.to_bits()),
        row(value.linear_right, value.drive_slot.0),
        row(value.linear_up, value.pad_1.to_bits()),
        row(value.linear_at, value.pad_2.to_bits()),
        row(value.angular_right, value.pad_3.to_bits()),
        row(value.angular_up, value.pad_4.to_bits()),
        row(value.angular_at, value.pad_5.to_bits()),
        row(
            value.inverse_inertia_a_right,
            value.inverse_mass_a.to_bits(),
        ),
        row(value.inverse_inertia_a_up, value.pad_6.to_bits()),
        row(value.inverse_inertia_a_at, value.pad_7.to_bits()),
        row(
            value.inverse_inertia_b_right,
            value.inverse_mass_b.to_bits(),
        ),
        row(value.inverse_inertia_b_up, value.pad_8.to_bits()),
        row(value.inverse_inertia_b_at, value.pad_9.to_bits()),
    ]
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for checked Drive RF revalidation and publication"
    )
)]
fn try_prepare_drive_jacobian_snapshots_with_backend<B: SimulationDriveNumericBackend>(
    source: SimulationDrivePreparedBatchSource,
    snapshots: &[SimulationDriveBuildSnapshot],
    backend: &mut B,
) -> Result<SimulationPreparedDriveJacobianBatch, SimulationDriveBatchPreparationError<B::Error>> {
    try_prepare_drive_jacobian_snapshots_with_backend_and_reservation(
        source,
        snapshots,
        backend,
        reserve_drive_inputs,
        reserve_drive_records,
    )
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for checked Drive RF revalidation and publication"
    )
)]
fn try_prepare_drive_jacobian_snapshots_with_backend_and_reservation<
    B: SimulationDriveNumericBackend,
>(
    source: SimulationDrivePreparedBatchSource,
    snapshots: &[SimulationDriveBuildSnapshot],
    backend: &mut B,
    reserve_inputs: impl FnOnce(&mut Vec<SimulationPreparedDriveBuildInput>, usize) -> Result<(), ()>,
    reserve_records: impl FnOnce(&mut Vec<SimulationPreparedDriveJacobian>, usize) -> Result<(), ()>,
) -> Result<SimulationPreparedDriveJacobianBatch, SimulationDriveBatchPreparationError<B::Error>> {
    let capacity = snapshots.len();

    // Token failure is impossible for a live pool with representable capacity,
    // but preflight every record before allocation or any backend effect.
    for snapshot in snapshots {
        try_drive_record_bindings(snapshot)
            .map_err(|error| map_binding_error::<B::Error>(snapshot.drive, error))?;
    }

    let mut inputs = Vec::new();
    reserve_inputs(&mut inputs, capacity)
        .map_err(|()| SimulationDriveBatchPreparationError::StagingAllocationFailed { capacity })?;
    if inputs.capacity() < capacity {
        return Err(SimulationDriveBatchPreparationError::StagingAllocationFailed { capacity });
    }
    for snapshot in snapshots {
        let bindings = try_drive_record_bindings(snapshot)
            .map_err(|error| map_binding_error::<B::Error>(snapshot.drive, error))?;
        inputs.push(SimulationPreparedDriveBuildInput {
            drive: snapshot.drive,
            body_a: snapshot.body_a.observation(),
            body_b: snapshot.body_b.observation(),
            input: drive_build_input_from_snapshot(snapshot, bindings),
        });
    }

    let mut records = Vec::new();
    reserve_records(&mut records, capacity)
        .map_err(|()| SimulationDriveBatchPreparationError::RecordAllocationFailed { capacity })?;
    if records.capacity() < capacity {
        return Err(SimulationDriveBatchPreparationError::RecordAllocationFailed { capacity });
    }

    for (ordinal, prepared) in inputs.into_iter().enumerate() {
        let record =
            compose_drive_jacobian_with_backend(&prepared.input, backend).map_err(|source| {
                SimulationDriveBatchPreparationError::Numeric {
                    ordinal,
                    drive: prepared.drive,
                    source,
                }
            })?;
        records.push(SimulationPreparedDriveJacobian {
            drive: prepared.drive,
            body_a: prepared.body_a,
            body_b: prepared.body_b,
            record,
        });
    }
    Ok(SimulationPreparedDriveJacobianBatch { source, records })
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for checked Drive RF revalidation and publication"
    )
)]
fn reserve_drive_inputs(
    inputs: &mut Vec<SimulationPreparedDriveBuildInput>,
    capacity: usize,
) -> Result<(), ()> {
    inputs.try_reserve_exact(capacity).map_err(|_| ())
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for checked Drive RF revalidation and publication"
    )
)]
fn reserve_drive_records(
    records: &mut Vec<SimulationPreparedDriveJacobian>,
    capacity: usize,
) -> Result<(), ()> {
    records.try_reserve_exact(capacity).map_err(|_| ())
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private direct Drive-phase boundary")
)]
const fn map_binding_error<E>(
    drive: SimulationDriveIdentity,
    error: (SimulationDrivePreparedBindingKind, u32),
) -> SimulationDriveBatchPreparationError<E> {
    SimulationDriveBatchPreparationError::BindingTokenOverflow {
        drive,
        kind: error.0,
        slot: error.1,
    }
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private direct Drive-phase boundary")
)]
#[allow(
    clippy::similar_names,
    reason = "body A/child and body B/parent token roles must remain explicit"
)]
fn try_drive_record_bindings(
    snapshot: &SimulationDriveBuildSnapshot,
) -> Result<SimulationDriveRecordBindings, (SimulationDrivePreparedBindingKind, u32)> {
    let body_a_slot = snapshot.body_a.observation().identity().id().index();
    let body_b_slot = snapshot.body_b.observation().identity().id().index();
    let drive_slot = snapshot.drive.id().index();
    Ok(SimulationDriveRecordBindings {
        solver_delta_a: checked_nonzero_slot_token(body_a_slot)
            .ok_or((SimulationDrivePreparedBindingKind::BodyA, body_a_slot))?
            .get(),
        solver_delta_b: checked_nonzero_slot_token(body_b_slot)
            .ok_or((SimulationDrivePreparedBindingKind::BodyB, body_b_slot))?
            .get(),
        drive: checked_nonzero_slot_token(drive_slot)
            .ok_or((SimulationDrivePreparedBindingKind::Drive, drive_slot))?
            .get(),
    })
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private direct Drive-phase boundary")
)]
const fn checked_nonzero_slot_token(slot: u32) -> Option<std::num::NonZeroU32> {
    match slot.checked_add(1) {
        Some(value) => std::num::NonZeroU32::new(value),
        None => None,
    }
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private direct Drive-phase boundary")
)]
const fn drive_build_input_from_snapshot(
    snapshot: &SimulationDriveBuildSnapshot,
    bindings: SimulationDriveRecordBindings,
) -> SimulationDriveBuildInput {
    SimulationDriveBuildInput {
        body_a: drive_build_body_input_from_snapshot(snapshot.body_a),
        body_b: drive_build_body_input_from_snapshot(snapshot.body_b),
        frames: snapshot.frames,
        dynamics: snapshot.dynamics,
        time_step: snapshot.time_step,
        spy: snapshot.spy,
        bindings,
    }
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "private direct Drive-phase boundary")
)]
const fn drive_build_body_input_from_snapshot(
    snapshot: SimulationRigidBodyDriveBuildSnapshot,
) -> SimulationDriveBuildBodyInput {
    SimulationDriveBuildBodyInput {
        state: snapshot.state,
        quaternion: snapshot.quaternion,
        center_of_mass: snapshot.center_of_mass,
        velocity: snapshot.velocity,
        omega: snapshot.omega,
        right: snapshot.right,
        up: snapshot.up,
        at: snapshot.at,
        force: snapshot.force,
        torque: snapshot.torque,
        inverse_inertia_full: snapshot.inverse_inertia_full,
        inverse_mass: snapshot.inverse_mass,
        inverse_inertia_split: snapshot.inverse_inertia_split,
    }
}

fn build_drive_geometry_with_backend<B: SimulationDriveNumericBackend>(
    body_a: SimulationDriveBodyGeometryInput,
    body_b: SimulationDriveBodyGeometryInput,
    frames: SimulationDriveFrames,
    backend: &mut B,
) -> Result<SimulationDriveGeometryHost, B::Error> {
    let quaternion_a = compose_drive_quaternion_with_backend(
        body_a.quaternion,
        frames.quaternion_a,
        SimulationDriveDotSite::QuaternionA,
        backend,
    )?;
    let quaternion_b = compose_drive_quaternion_with_backend(
        body_b.quaternion,
        frames.quaternion_b,
        SimulationDriveDotSite::QuaternionB,
        backend,
    )?;
    let packed_linear_axes = drive_basis_host(quaternion_b);
    let [linear_right, linear_up, linear_at] = packed_linear_axes;
    Ok(SimulationDriveGeometryHost {
        attachment_offset_a: drive_attachment_offset_host(body_a, frames.position_a),
        attachment_offset_b: drive_attachment_offset_host(body_b, frames.position_b),
        linear_right: first3(linear_right),
        linear_up: first3(linear_up),
        linear_at: first3(linear_at),
        packed_linear_axes,
        rqd: SimulationDriveJacobianRqd::create(quaternion_a, quaternion_b),
    })
}

fn compose_drive_quaternion_with_backend<B: SimulationDriveNumericBackend>(
    body: [f32; 4],
    frame: [f32; 4],
    site: SimulationDriveDotSite,
    backend: &mut B,
) -> Result<[f32; 4], B::Error> {
    let dot3 = backend.sum3(site, first3(body), first3(frame))?;
    Ok(compose_body_frame_quaternion_from_dot3(body, frame, dot3))
}

#[allow(
    clippy::too_many_arguments,
    reason = "four distinct raw reciprocal sites are intentionally explicit"
)]
fn drive_coefficients_with_backend<B: SimulationDriveNumericBackend>(
    params: SimulationDriveDynamicsParams,
    time_step: f32,
    soft_dynamics_site: SimulationDriveReciprocalSite,
    soft_spring_site: SimulationDriveReciprocalSite,
    soft_damping_site: SimulationDriveReciprocalSite,
    non_soft_spring_site: SimulationDriveReciprocalSite,
    backend: &mut B,
) -> Result<SimulationDriveCoefficients, B::Error> {
    let damping_h = time_step * params.damping;
    if params.raw_type == SOFT_DRIVE {
        let spring_h = time_step * params.spring;
        let spring_h2 = spring_h * time_step;
        let numerator = spring_h2 + damping_h;
        let denominator = (1.0 + spring_h2) + damping_h;
        let input = [denominator; 4];
        let dynamics_seed = backend.reciprocal_estimate(soft_dynamics_site, input)?[0];
        let spring_seed = backend.reciprocal_estimate(soft_spring_site, input)?[0];
        let damping_seed = backend.reciprocal_estimate(soft_damping_site, input)?[0];
        let [dynamics_reciprocal, spring_reciprocal, damping_reciprocal] =
            refine_drive_reciprocals_interleaved(
                denominator,
                [dynamics_seed, spring_seed, damping_seed],
            );
        Ok(SimulationDriveCoefficients {
            dynamics: dynamics_reciprocal * numerator,
            spring_factor: spring_reciprocal * spring_h2,
            damping_factor: damping_reciprocal * damping_h,
        })
    } else {
        let denominator = 1.0 + damping_h;
        let spring_seed = backend.reciprocal_estimate(non_soft_spring_site, [denominator; 4])?[0];
        let spring_reciprocal = refine_drive_reciprocal(denominator, spring_seed);
        Ok(SimulationDriveCoefficients {
            dynamics: 1.0,
            spring_factor: spring_reciprocal * 1.0,
            damping_factor: 1.0,
        })
    }
}

fn build_drive_linear_bias_with_backend<B: SimulationDriveNumericBackend>(
    prediction: SimulationDrivePointPredictionHost,
    position_error_after_hard_gate: [f32; 3],
    coefficients: SimulationDriveCoefficients,
    linear_axes: [[f32; 3]; 3],
    backend: &mut B,
) -> Result<SimulationDriveLinearBiasHost, B::Error> {
    let velocity_term = prediction
        .velocity_delta_h
        .map(|value| value * coefficients.damping_factor);
    let position_velocity: [f32; 3] = std::array::from_fn(|lane| {
        position_error_after_hard_gate[lane]
            .mul_add(coefficients.spring_factor, velocity_term[lane])
    });
    let bias = std::array::from_fn(|lane| {
        prediction.acceleration_delta_h2[lane]
            .mul_add(coefficients.dynamics, position_velocity[lane])
    });
    let up = backend.sum3(SimulationDriveDotSite::LinearBiasUp, bias, linear_axes[1])?;
    let right = backend.sum3(
        SimulationDriveDotSite::LinearBiasRight,
        bias,
        linear_axes[0],
    )?;
    let at = backend.sum3(SimulationDriveDotSite::LinearBiasAt, bias, linear_axes[2])?;
    Ok(SimulationDriveLinearBiasHost {
        bias,
        projection: [right, up, at],
    })
}

#[allow(
    clippy::float_cmp,
    clippy::manual_clamp,
    clippy::similar_names,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "source-shaped comparisons, clamp order, and A/B roles are intentional"
)]
fn build_drive_angular_with_backend<B: SimulationDriveNumericBackend>(
    rqd: &SimulationDriveJacobianRqd,
    packed_linear_axes: [[f32; 4]; 3],
    body_a: SimulationDriveAngularBodyInput,
    body_b: SimulationDriveAngularBodyInput,
    params: SimulationDriveDynamicsParams,
    time_step: f32,
    backend: &mut B,
) -> Result<SimulationDriveAngularBuildHost, B::Error> {
    let strength_h = params.strength * time_step;
    let strength_h2 = strength_h * time_step;
    let rows = rqd.rows();
    let q = [rows[0][3], rows[1][3], rows[2][3], 0.0];
    let clamped_square = q.map(|value| (value * value).max(0.0).min(1.0));
    let used_global_fallback =
        clamped_square[0] == 1.0 || clamped_square[1] == 1.0 || clamped_square[2] == 1.0;
    let normalization_denominator = if used_global_fallback {
        [1.0; 4]
    } else {
        clamped_square.map(|value| 1.0 - value)
    };
    let estimate = backend.reciprocal_sqrt_estimate(
        SimulationDriveReciprocalSqrtSite::AngularAxisNormalization,
        normalization_denominator,
    )?;
    let inverse_length: [f32; 4] = std::array::from_fn(|lane| {
        refine_drive_reciprocal_sqrt(normalization_denominator[lane], estimate[lane])
    });
    let angular_axes = std::array::from_fn(|axis| {
        let candidate = if used_global_fallback {
            packed_linear_axes[axis]
        } else {
            [rows[axis][0], rows[axis][1], rows[axis][2], rows[axis][0]]
        };
        candidate.map(|value| value * inverse_length[axis])
    });
    let angular_error_full: [f32; 4] =
        std::array::from_fn(|lane| (q[lane] * inverse_length[lane]) * 2.0);
    let angular_error = first3(angular_error_full);

    let torque_delta: [f32; 3] =
        std::array::from_fn(|lane| body_b.torque[lane] - body_a.torque[lane]);
    let omega_delta: [f32; 3] = std::array::from_fn(|lane| body_b.omega[lane] - body_a.omega[lane]);
    let acceleration_h = torque_delta.map(|value| value * time_step);
    let omega_h = omega_delta.map(|value| value * time_step);
    let acceleration_h2 = acceleration_h.map(|value| value * time_step);
    let right = first3(angular_axes[0]);
    let up = first3(angular_axes[1]);
    let at = first3(angular_axes[2]);
    let omega_right = backend.sum3(SimulationDriveDotSite::AngularRightOmega, right, omega_h)?;
    let acceleration_right = backend.sum3(
        SimulationDriveDotSite::AngularRightAcceleration,
        right,
        acceleration_h2,
    )?;
    let omega_up = backend.sum3(SimulationDriveDotSite::AngularUpOmega, up, omega_h)?;
    let acceleration_up = backend.sum3(
        SimulationDriveDotSite::AngularUpAcceleration,
        up,
        acceleration_h2,
    )?;
    let omega_at = backend.sum3(SimulationDriveDotSite::AngularAtOmega, at, omega_h)?;
    let acceleration_at = backend.sum3(
        SimulationDriveDotSite::AngularAtAcceleration,
        at,
        acceleration_h2,
    )?;
    let omega_projection = [omega_right, omega_up, omega_at];
    let acceleration_projection = [acceleration_right, acceleration_up, acceleration_at];
    let coefficients = drive_coefficients_with_backend(
        params,
        time_step,
        SimulationDriveReciprocalSite::AngularSoftDynamics,
        SimulationDriveReciprocalSite::AngularSoftSpring,
        SimulationDriveReciprocalSite::AngularSoftDamping,
        SimulationDriveReciprocalSite::AngularNonSoftSpring,
        backend,
    )?;
    let (gated_error, hard_outside_limit) = if params.raw_type == HARD_DRIVE {
        let length_squared = backend.sum3(
            SimulationDriveDotSite::AngularHardLengthSquared,
            angular_error,
            angular_error,
        )?;
        let reciprocal_sqrt_seed = backend.reciprocal_sqrt_estimate(
            SimulationDriveReciprocalSqrtSite::AngularHardLength,
            [length_squared; 4],
        )?[0];
        let gate = apply_hard_drive_gate(
            params.raw_type,
            angular_error,
            length_squared,
            time_step,
            params.spring,
            reciprocal_sqrt_seed,
        );
        (gate.error, gate.outside_limit)
    } else {
        (angular_error, false)
    };
    let bias_projection = drive_angular_bias_host(
        gated_error,
        omega_projection,
        acceleration_projection,
        coefficients,
    );
    Ok(SimulationDriveAngularBuildHost {
        angular_axes,
        angular_error,
        omega_projection,
        acceleration_projection,
        bias_projection,
        coefficients,
        strength_h2,
        normalization_denominator,
        used_global_fallback,
        hard_outside_limit,
    })
}

fn build_drive_effective_mass_with_backend<B: SimulationDriveNumericBackend>(
    body_a: SimulationDriveEffectiveMassBodyInput,
    body_b: SimulationDriveEffectiveMassBodyInput,
    linear_axes: [[f32; 3]; 3],
    angular_axes: [[f32; 3]; 3],
    backend: &mut B,
) -> Result<SimulationDriveEffectiveMassHost, B::Error> {
    let contribution_a = drive_effective_mass_body_with_backend(
        body_a,
        linear_axes,
        angular_axes,
        [
            SimulationDriveDotSite::BodyAAngularRight,
            SimulationDriveDotSite::BodyAAngularUp,
            SimulationDriveDotSite::BodyAAngularAt,
        ],
        [
            SimulationDriveDotSite::BodyALinearRight,
            SimulationDriveDotSite::BodyALinearUp,
            SimulationDriveDotSite::BodyALinearAt,
        ],
        backend,
    )?;
    let contribution_b = drive_effective_mass_body_with_backend(
        body_b,
        linear_axes,
        angular_axes,
        [
            SimulationDriveDotSite::BodyBAngularRight,
            SimulationDriveDotSite::BodyBAngularUp,
            SimulationDriveDotSite::BodyBAngularAt,
        ],
        [
            SimulationDriveDotSite::BodyBLinearRight,
            SimulationDriveDotSite::BodyBLinearUp,
            SimulationDriveDotSite::BodyBLinearAt,
        ],
        backend,
    )?;
    let linear_a = contribution_a
        .linear
        .map(|value| value + contribution_a.inverse_mass);
    let linear_b = contribution_b
        .linear
        .map(|value| value + contribution_b.inverse_mass);
    Ok(SimulationDriveEffectiveMassHost {
        inverse_inertia_a: contribution_a.inverse_inertia,
        inverse_mass_a: contribution_a.inverse_mass,
        inverse_inertia_b: contribution_b.inverse_inertia,
        inverse_mass_b: contribution_b.inverse_mass,
        linear_denominator: std::array::from_fn(|axis| linear_a[axis] + linear_b[axis]),
        angular_denominator: std::array::from_fn(|axis| {
            contribution_a.angular[axis] + contribution_b.angular[axis]
        }),
    })
}

fn drive_effective_mass_body_with_backend<B: SimulationDriveNumericBackend>(
    body: SimulationDriveEffectiveMassBodyInput,
    linear_axes: [[f32; 3]; 3],
    angular_axes: [[f32; 3]; 3],
    angular_sites: [SimulationDriveDotSite; 3],
    linear_sites: [SimulationDriveDotSite; 3],
    backend: &mut B,
) -> Result<DriveEffectiveMassBodyHost, B::Error> {
    if body.state & 4 == 0 {
        return Ok(DriveEffectiveMassBodyHost {
            inverse_inertia: [[0.0; 3]; 3],
            inverse_mass: 0.0,
            linear: [0.0; 3],
            angular: [0.0; 3],
        });
    }
    let [ixx, ixy, ixz] = body.inverse_inertia_full;
    let [izz, iyy, iyz] = body.inverse_inertia_split;
    let inverse_inertia = [[ixx, ixy, ixz], [ixy, iyy, iyz], [ixz, iyz, izz]];

    let angular_right_vector =
        drive_effective_mass_tensor_multiply_host(inverse_inertia, angular_axes[0]);
    let angular_right = backend.sum3(angular_sites[0], angular_right_vector, angular_axes[0])?;
    let angular_up_vector =
        drive_effective_mass_tensor_multiply_host(inverse_inertia, angular_axes[1]);
    let angular_up = backend.sum3(angular_sites[1], angular_up_vector, angular_axes[1])?;
    let angular_at_vector =
        drive_effective_mass_tensor_multiply_host(inverse_inertia, angular_axes[2]);
    let angular_at = backend.sum3(angular_sites[2], angular_at_vector, angular_axes[2])?;

    let linear_right_cross =
        drive_effective_mass_cross_host(body.attachment_offset, linear_axes[0]);
    let linear_right_vector =
        drive_effective_mass_tensor_multiply_host(inverse_inertia, linear_right_cross);
    let linear_right = backend.sum3(linear_sites[0], linear_right_vector, linear_right_cross)?;
    let linear_up_cross = drive_effective_mass_cross_host(body.attachment_offset, linear_axes[1]);
    let linear_up_vector =
        drive_effective_mass_tensor_multiply_host(inverse_inertia, linear_up_cross);
    let linear_up = backend.sum3(linear_sites[1], linear_up_vector, linear_up_cross)?;
    let linear_at_cross = drive_effective_mass_cross_host(body.attachment_offset, linear_axes[2]);
    let linear_at_vector =
        drive_effective_mass_tensor_multiply_host(inverse_inertia, linear_at_cross);
    let linear_at = backend.sum3(linear_sites[2], linear_at_vector, linear_at_cross)?;

    Ok(DriveEffectiveMassBodyHost {
        inverse_inertia,
        inverse_mass: body.inverse_mass,
        linear: [linear_right, linear_up, linear_at],
        angular: [angular_right, angular_up, angular_at],
    })
}

fn drive_build_geometry_input(
    body: SimulationDriveBuildBodyInput,
) -> SimulationDriveBodyGeometryInput {
    SimulationDriveBodyGeometryInput {
        quaternion: body.quaternion,
        right: body.right,
        up: body.up,
        at: body.at,
    }
}

fn drive_build_prediction_input(
    body: SimulationDriveBuildBodyInput,
) -> SimulationDrivePredictionBodyInput {
    SimulationDrivePredictionBodyInput {
        center_of_mass: body.center_of_mass,
        velocity: body.velocity,
        omega: body.omega,
        force: body.force,
        torque: body.torque,
    }
}

const fn drive_build_angular_input(
    body: SimulationDriveBuildBodyInput,
) -> SimulationDriveAngularBodyInput {
    SimulationDriveAngularBodyInput {
        omega: body.omega,
        torque: body.torque,
    }
}

const fn drive_build_effective_mass_input(
    body: SimulationDriveBuildBodyInput,
    attachment_offset: [f32; 3],
) -> SimulationDriveEffectiveMassBodyInput {
    SimulationDriveEffectiveMassBodyInput {
        state: body.state,
        attachment_offset,
        inverse_inertia_full: body.inverse_inertia_full,
        inverse_mass: body.inverse_mass,
        inverse_inertia_split: body.inverse_inertia_split,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "one complete literal prevents silently omitted 0x180 record fields"
)]
fn assemble_drive_jacobian_record(
    attachment_offset_a: [f32; 3],
    attachment_offset_b: [f32; 3],
    spy: u32,
    bindings: SimulationDriveRecordBindings,
    finalized: &SimulationDriveFinalizedHost,
) -> SimulationDriveJacobian {
    SimulationDriveJacobian {
        attachment_offset_a,
        solver_delta_a: SolverDeltaRowToken(bindings.solver_delta_a),
        attachment_offset_b,
        solver_delta_b: SolverDeltaRowToken(bindings.solver_delta_b),
        position_impulse: finalized.position_impulse,
        linear_dynamics: finalized.linear_dynamics,
        angular_impulse: finalized.angular_impulse,
        angular_dynamics: finalized.angular_dynamics,
        linear_transpose_right: finalized.linear_transpose[0],
        spy,
        angular_transpose_right: finalized.angular_transpose[0],
        pad_0: finalized.angular_transpose_right_pad,
        linear_transpose_up: finalized.linear_transpose[1],
        position_limit_x: finalized.position_limit[0],
        angular_transpose_up: finalized.angular_transpose[1],
        position_limit_y: finalized.position_limit[1],
        linear_transpose_at: finalized.linear_transpose[2],
        angular_limit_x: finalized.angular_limit[0],
        angular_transpose_at: finalized.angular_transpose[2],
        angular_limit_y: finalized.angular_limit[1],
        position_offset: finalized.position_offset,
        position_limit_z: finalized.position_limit[2],
        angular_offset: finalized.angular_offset,
        angular_limit_z: finalized.angular_limit[2],
        linear_right: first3(finalized.packed_linear_axes[0]),
        drive_slot: DriveSlotToken(bindings.drive),
        linear_up: first3(finalized.packed_linear_axes[1]),
        pad_1: finalized.packed_linear_axes[1][3],
        linear_at: first3(finalized.packed_linear_axes[2]),
        pad_2: finalized.packed_linear_axes[2][3],
        angular_right: first3(finalized.packed_angular_axes[0]),
        pad_3: finalized.packed_angular_axes[0][3],
        angular_up: first3(finalized.packed_angular_axes[1]),
        pad_4: finalized.packed_angular_axes[1][3],
        angular_at: first3(finalized.packed_angular_axes[2]),
        pad_5: finalized.packed_angular_axes[2][3],
        inverse_inertia_a_right: first3(finalized.packed_inverse_inertia_a[0]),
        inverse_mass_a: finalized.packed_inverse_inertia_a[0][3],
        inverse_inertia_a_up: first3(finalized.packed_inverse_inertia_a[1]),
        pad_6: finalized.packed_inverse_inertia_a[1][3],
        inverse_inertia_a_at: first3(finalized.packed_inverse_inertia_a[2]),
        pad_7: finalized.packed_inverse_inertia_a[2][3],
        inverse_inertia_b_right: first3(finalized.packed_inverse_inertia_b[0]),
        inverse_mass_b: finalized.packed_inverse_inertia_b[0][3],
        inverse_inertia_b_up: first3(finalized.packed_inverse_inertia_b[1]),
        pad_8: finalized.packed_inverse_inertia_b[1][3],
        inverse_inertia_b_at: first3(finalized.packed_inverse_inertia_b[2]),
        pad_9: finalized.packed_inverse_inertia_b[2][3],
    }
}

fn refine_drive_final_reciprocals(
    linear_denominator: [f32; 3],
    angular_denominator: [f32; 3],
    linear_seed: [f32; 3],
    angular_seed: [f32; 3],
) -> ([f32; 3], [f32; 3]) {
    let linear_residual_0: [f32; 3] =
        std::array::from_fn(|lane| (-linear_seed[lane]).mul_add(linear_denominator[lane], 1.0));
    let angular_residual_0: [f32; 3] =
        std::array::from_fn(|lane| (-angular_seed[lane]).mul_add(angular_denominator[lane], 1.0));
    let linear_refined_0: [f32; 3] = std::array::from_fn(|lane| {
        linear_seed[lane].mul_add(linear_residual_0[lane], linear_seed[lane])
    });
    let angular_refined_0: [f32; 3] = std::array::from_fn(|lane| {
        angular_seed[lane].mul_add(angular_residual_0[lane], angular_seed[lane])
    });
    let linear_residual_1: [f32; 3] = std::array::from_fn(|lane| {
        (-linear_refined_0[lane]).mul_add(linear_denominator[lane], 1.0)
    });
    let angular_residual_1: [f32; 3] = std::array::from_fn(|lane| {
        (-angular_refined_0[lane]).mul_add(angular_denominator[lane], 1.0)
    });
    let linear_refined_1: [f32; 3] = std::array::from_fn(|lane| {
        linear_refined_0[lane].mul_add(linear_residual_1[lane], linear_refined_0[lane])
    });
    let angular_refined_1: [f32; 3] = std::array::from_fn(|lane| {
        angular_refined_0[lane].mul_add(angular_residual_1[lane], angular_refined_0[lane])
    });
    (
        linear_refined_1.map(|value| value * 1.0),
        angular_refined_1.map(|value| value * 1.0),
    )
}

#[derive(Clone, Copy)]
struct DriveEffectiveMassBodyHost {
    inverse_inertia: [[f32; 3]; 3],
    inverse_mass: f32,
    linear: [f32; 3],
    angular: [f32; 3],
}

fn drive_effective_mass_body_host(
    body: SimulationDriveEffectiveMassBodyInput,
    linear_axes: [[f32; 3]; 3],
    angular_axes: [[f32; 3]; 3],
) -> DriveEffectiveMassBodyHost {
    if body.state & 4 == 0 {
        return DriveEffectiveMassBodyHost {
            inverse_inertia: [[0.0; 3]; 3],
            inverse_mass: 0.0,
            linear: [0.0; 3],
            angular: [0.0; 3],
        };
    }

    let [ixx, ixy, ixz] = body.inverse_inertia_full;
    let [izz, iyy, iyz] = body.inverse_inertia_split;
    let inverse_inertia = [[ixx, ixy, ixz], [ixy, iyy, iyz], [ixz, iyz, izz]];
    let linear = std::array::from_fn(|axis| {
        let cross = drive_effective_mass_cross_host(body.attachment_offset, linear_axes[axis]);
        drive_effective_mass_dot3_host(
            drive_effective_mass_tensor_multiply_host(inverse_inertia, cross),
            cross,
        )
    });
    let angular = std::array::from_fn(|axis| {
        drive_effective_mass_dot3_host(
            drive_effective_mass_tensor_multiply_host(inverse_inertia, angular_axes[axis]),
            angular_axes[axis],
        )
    });
    DriveEffectiveMassBodyHost {
        inverse_inertia,
        inverse_mass: body.inverse_mass,
        linear,
        angular,
    }
}

fn drive_effective_mass_cross_host(arm: [f32; 3], axis: [f32; 3]) -> [f32; 3] {
    let arm_yzx = [arm[1], arm[2], arm[0]];
    let axis_yzx = [axis[1], axis[2], axis[0]];
    let product: [f32; 3] = std::array::from_fn(|lane| arm[lane] * axis_yzx[lane]);
    let difference: [f32; 3] =
        std::array::from_fn(|lane| (-arm_yzx[lane]).mul_add(axis[lane], product[lane]));
    [difference[1], difference[2], difference[0]]
}

fn drive_effective_mass_tensor_multiply_host(columns: [[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|lane| {
        columns[2][lane].mul_add(
            vector[2],
            columns[1][lane].mul_add(vector[1], columns[0][lane] * vector[0]),
        )
    })
}

fn drive_effective_mass_dot3_host(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn compose_drive_quaternion_host(body: [f32; 4], frame: [f32; 4]) -> [f32; 4] {
    compose_body_frame_quaternion_from_dot3(body, frame, dot3_host(body, frame))
}

/// Also matches `SkeletonData::Init` at `0x82b56a10..0x82b56b5c`:
/// unnormalized sqrt-two expansion, including the noncanonical axis W lanes.
/// Exceptional-value payload and Xenon denormal behavior remain host boundaries.
pub(crate) fn drive_basis_host(quaternion: [f32; 4]) -> [[f32; 4]; 3] {
    let scaled = vmul(quaternion, [f32::from_bits(0x3fb5_04f3); 4]);
    let negative_squares = vnmsub(scaled, [0.5; 4], scaled);
    let products = vmul(scaled, yzxx(scaled));
    let w_products = vmul([scaled[3]; 4], zxyx(scaled));
    let sum = vadd(products, w_products);
    let difference = vsub(products, w_products);
    let diagonal = vadd(negative_squares, yzxx(negative_squares));
    [
        [diagonal[1], sum[0], difference[2], diagonal[0]],
        [difference[0], diagonal[2], sum[1], difference[0]],
        [sum[2], difference[1], diagonal[0], sum[0]],
    ]
}

fn drive_attachment_offset_host(
    body: SimulationDriveBodyGeometryInput,
    position: [f32; 4],
) -> [f32; 3] {
    std::array::from_fn(|lane| {
        body.at[lane].mul_add(
            position[2],
            body.up[lane].mul_add(position[1], body.right[lane] * position[0]),
        )
    })
}

fn first3(value: [f32; 4]) -> [f32; 3] {
    [value[0], value[1], value[2]]
}

fn vadd(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] + right[lane])
}

fn vsub(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] - right[lane])
}

fn vmul(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] * right[lane])
}

fn vnmsub(multiplier: [f32; 4], addend: [f32; 4], multiplicand: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| (-multiplier[lane]).mul_add(multiplicand[lane], addend[lane]))
}

fn yzxx(value: [f32; 4]) -> [f32; 4] {
    [value[1], value[2], value[0], value[0]]
}

fn zxyx(value: [f32; 4]) -> [f32; 4] {
    [value[2], value[0], value[1], value[0]]
}

fn horus_min_then_max(value: [f32; 4], upper: [f32; 4]) -> [f32; 4] {
    let lower = upper.map(|lane| 0.0 - lane);
    std::array::from_fn(|lane| value[lane].min(upper[lane]).max(lower[lane]))
}

fn dot3_host(left: [f32; 4], right: [f32; 4]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

/// Exact `0x40` value produced by native `Jacobian_RQD::Create`.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationDriveJacobianRqd {
    rows: [[f32; 4]; 4],
}

impl SimulationDriveJacobianRqd {
    /// Reconstructs `Jacobian_RQD::Create` (`0x82ae3750..0x82ae3930`).
    ///
    /// Every multiplication, addition, and subtraction below corresponds to
    /// the native VMX operation order. For ordinary finite f32 values Rust's
    /// strict floating-point operations preserve this graph. Exceptional VMX
    /// behavior remains outside this claim.
    #[must_use]
    pub fn create(a: [f32; 4], b: [f32; 4]) -> Self {
        Self {
            rows: create_jacobian_rqd_rows(a, b),
        }
    }

    #[must_use]
    pub const fn rows(&self) -> &[[f32; 4]; 4] {
        &self.rows
    }
}

/// Caller-provided Xenon reciprocal-estimate seeds for one coefficient block.
///
/// Native SOFT Drive code issues three independent `vrefp` instructions for
/// the same denominator. The non-SOFT path consumes only `spring`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationDriveReciprocalSeeds {
    pub dynamics: f32,
    pub spring: f32,
    pub damping: f32,
}

/// Three coefficients selected by one native linear or angular Drive block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationDriveCoefficients {
    pub dynamics: f32,
    pub spring_factor: f32,
    pub damping_factor: f32,
}

/// Applies Build's two fused Newton reciprocal refinements to an explicit seed.
///
/// Native occurrences begin at `0x82ae4fec` for linear SOFT coefficients.
#[must_use]
pub fn refine_drive_reciprocal(input: f32, seed: f32) -> f32 {
    let residual_0 = (-seed).mul_add(input, 1.0);
    let refined_0 = seed.mul_add(residual_0, seed);
    let residual_1 = (-refined_0).mul_add(input, 1.0);
    refined_0.mul_add(residual_1, refined_0)
}

/// Applies Build's two fused Newton reciprocal-square-root refinements.
///
/// The caller supplies the `vrsqrtefp` seed. This is the operation graph at
/// `0x82ae50fc..0x82ae512c`, without claiming Xenon seed parity.
#[must_use]
pub fn refine_drive_reciprocal_sqrt(input: f32, seed: f32) -> f32 {
    let half = 0.5_f32;
    let square_0 = seed * seed;
    let residual_0 = (-input).mul_add(square_0, 1.0);
    let refined_0 = (seed * half).mul_add(residual_0, seed);
    let square_1 = refined_0 * refined_0;
    let residual_1 = (-input).mul_add(square_1, 1.0);
    (refined_0 * half).mul_add(residual_1, refined_0)
}

/// Builds the exact SOFT-versus-non-SOFT coefficient graph for one Params block.
///
/// Linear native anchors are `0x82ae4f94..0x82ae5094`; angular anchors are
/// `0x82ae52c0..0x82ae53c0`. Only raw type `1` is SOFT. HARD (`2`), `NO_DRIVE`
/// (`0`), and every unknown raw value use the same non-SOFT coefficient path.
#[must_use]
pub fn drive_coefficients(
    params: SimulationDriveDynamicsParams,
    time_step: f32,
    seeds: SimulationDriveReciprocalSeeds,
) -> SimulationDriveCoefficients {
    let damping_h = time_step * params.damping;
    drive_coefficients_from_damping_h(params, time_step, damping_h, seeds)
}

fn drive_coefficients_from_damping_h(
    params: SimulationDriveDynamicsParams,
    time_step: f32,
    damping_h: f32,
    seeds: SimulationDriveReciprocalSeeds,
) -> SimulationDriveCoefficients {
    if params.raw_type == SOFT_DRIVE {
        let spring_h = time_step * params.spring;
        let spring_h2 = spring_h * time_step;
        let numerator = spring_h2 + damping_h;
        let denominator = (1.0 + spring_h2) + damping_h;
        let [dynamics_reciprocal, spring_reciprocal, damping_reciprocal] =
            refine_drive_reciprocals_interleaved(
                denominator,
                [seeds.dynamics, seeds.spring, seeds.damping],
            );
        SimulationDriveCoefficients {
            dynamics: dynamics_reciprocal * numerator,
            spring_factor: spring_reciprocal * spring_h2,
            damping_factor: damping_reciprocal * damping_h,
        }
    } else {
        let denominator = 1.0 + damping_h;
        let spring_reciprocal = refine_drive_reciprocal(denominator, seeds.spring);
        SimulationDriveCoefficients {
            dynamics: 1.0,
            spring_factor: spring_reciprocal * 1.0,
            damping_factor: 1.0,
        }
    }
}

fn refine_drive_reciprocals_interleaved(input: f32, seeds: [f32; 3]) -> [f32; 3] {
    let residual_0 = seeds.map(|seed| (-seed).mul_add(input, 1.0));
    let refined_0: [f32; 3] =
        std::array::from_fn(|lane| seeds[lane].mul_add(residual_0[lane], seeds[lane]));
    let residual_1 = refined_0.map(|refined| (-refined).mul_add(input, 1.0));
    std::array::from_fn(|lane| refined_0[lane].mul_add(residual_1[lane], refined_0[lane]))
}

/// Result of the native HARD Drive strict maximum-magnitude selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationHardDriveGate {
    pub error: [f32; 3],
    pub outside_limit: bool,
}

/// Applies the HARD-only strict maximum-magnitude clamp to an error vector.
///
/// `length_squared` is explicit because native uses `vmsum3fp128`, whose
/// internal reduction rounding is not established. `reciprocal_sqrt_seed`
/// likewise represents the unavailable Xenon `vrsqrtefp` result. For raw types
/// other than exactly `2`, native does not execute this block and the input is
/// returned unchanged. For HARD, equality and below select exact 1.0 and leave
/// the input bitwise unchanged after an explicit multiply; only
/// `length_squared > (time_step*spring)^2` selects the normalized scale. The
/// first reciprocal-square-root refinement round executes before the strict
/// comparison. The second round, candidate-scale multiply, selection, and
/// final error multiply then execute on every HARD route.
#[must_use]
pub fn apply_hard_drive_gate(
    raw_type: u32,
    error: [f32; 3],
    length_squared: f32,
    time_step: f32,
    spring: f32,
    reciprocal_sqrt_seed: f32,
) -> SimulationHardDriveGate {
    if raw_type != HARD_DRIVE {
        return SimulationHardDriveGate {
            error,
            outside_limit: false,
        };
    }

    let seed_square = reciprocal_sqrt_seed * reciprocal_sqrt_seed;
    let seed_half = reciprocal_sqrt_seed * 0.5;
    let limit = time_step * spring;
    let residual_0 = (-length_squared).mul_add(seed_square, 1.0);
    let limit_squared = limit * limit;
    let refined_0 = seed_half.mul_add(residual_0, reciprocal_sqrt_seed);
    let outside_limit = length_squared > limit_squared;
    let refined_square = refined_0 * refined_0;
    let refined_half = refined_0 * 0.5;
    let residual_1 = (-length_squared).mul_add(refined_square, 1.0);
    let reciprocal_length = refined_half.mul_add(residual_1, refined_0);
    let candidate_scale = limit * reciprocal_length;
    let scale = if outside_limit { candidate_scale } else { 1.0 };
    SimulationHardDriveGate {
        error: error.map(|lane| lane * scale),
        outside_limit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, offset_of, size_of};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum NumericSite {
        Reciprocal(SimulationDriveReciprocalSite),
        ReciprocalSqrt(SimulationDriveReciprocalSqrtSite),
        Sum3(SimulationDriveDotSite),
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum NumericCall {
        Reciprocal(SimulationDriveReciprocalSite, [u32; 4]),
        ReciprocalSqrt(SimulationDriveReciprocalSqrtSite, [u32; 4]),
        Sum3(SimulationDriveDotSite, [u32; 3], [u32; 3]),
    }

    #[derive(Default)]
    struct TracingHostNumericBackend {
        calls: Vec<NumericCall>,
        angular_hard_length_override: Option<f32>,
    }

    impl TracingHostNumericBackend {
        fn sites(&self) -> Vec<NumericSite> {
            self.calls
                .iter()
                .map(|call| match *call {
                    NumericCall::Reciprocal(site, _) => NumericSite::Reciprocal(site),
                    NumericCall::ReciprocalSqrt(site, _) => NumericSite::ReciprocalSqrt(site),
                    NumericCall::Sum3(site, _, _) => NumericSite::Sum3(site),
                })
                .collect()
        }
    }

    impl SimulationDriveNumericBackend for TracingHostNumericBackend {
        type Error = std::convert::Infallible;

        fn reciprocal_estimate(
            &mut self,
            site: SimulationDriveReciprocalSite,
            input: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            self.calls
                .push(NumericCall::Reciprocal(site, input.map(f32::to_bits)));
            Ok(input.map(f32::recip))
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            site: SimulationDriveReciprocalSqrtSite,
            input: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            self.calls
                .push(NumericCall::ReciprocalSqrt(site, input.map(f32::to_bits)));
            Ok(input.map(|value| value.sqrt().recip()))
        }

        fn sum3(
            &mut self,
            site: SimulationDriveDotSite,
            left: [f32; 3],
            right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            self.calls.push(NumericCall::Sum3(
                site,
                left.map(f32::to_bits),
                right.map(f32::to_bits),
            ));
            Ok(
                if site == SimulationDriveDotSite::AngularHardLengthSquared {
                    self.angular_hard_length_override
                        .unwrap_or_else(|| drive_effective_mass_dot3_host(left, right))
                } else {
                    drive_effective_mass_dot3_host(left, right)
                },
            )
        }
    }

    fn numeric_backend_input(
        linear_type: u32,
        angular_type: u32,
        state_a: u32,
        state_b: u32,
    ) -> SimulationDriveBuildInput {
        let body = |state, x| SimulationDriveBuildBodyInput {
            state,
            quaternion: [0.0, 0.0, 0.0, 1.0],
            center_of_mass: [x, x * 0.5, -x],
            velocity: [x * 0.25, -x * 0.125, x * 0.0625],
            omega: [-x * 0.5, x * 0.75, x * 0.25],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            at: [0.0, 0.0, 1.0],
            force: [x, -x * 2.0, x * 3.0],
            torque: [-x * 0.75, x * 0.5, x * 0.125],
            inverse_inertia_full: [1.0 + x * 0.125, x * 0.03125, -x * 0.015_625],
            inverse_mass: 0.5 + x * 0.0625,
            inverse_inertia_split: [1.5 + x * 0.125, 1.25 + x * 0.0625, x * 0.007_812_5],
        };
        SimulationDriveBuildInput {
            body_a: body(state_a, 1.0),
            body_b: body(state_b, 2.0),
            frames: SimulationDriveFrames {
                quaternion_a: [0.125, -0.25, 0.0625, 0.875],
                position_a: [0.5, -0.25, 0.75, 11.0],
                quaternion_b: [-0.0625, 0.1875, -0.125, 0.9375],
                position_b: [-0.375, 0.625, 0.25, -13.0],
            },
            dynamics: SimulationDriveDynamics {
                linear: SimulationDriveDynamicsParams {
                    spring: 2.0,
                    damping: 0.5,
                    strength: 3.0,
                    raw_type: linear_type,
                },
                angular: SimulationDriveDynamicsParams {
                    spring: 0.25,
                    damping: 0.75,
                    strength: 1.5,
                    raw_type: angular_type,
                },
            },
            time_step: 0.125,
            spy: 0x1122_3344,
            bindings: SimulationDriveRecordBindings {
                solver_delta_a: 0xa1a2_a3a4,
                solver_delta_b: 0xb1b2_b3b4,
                drive: 0xd1d2_d3d4,
            },
        }
    }

    fn second_prepared_numeric_input() -> SimulationDriveBuildInput {
        let q = f32::from_bits(0x3f35_04f3);
        SimulationDriveBuildInput {
            body_a: SimulationDriveBuildBodyInput {
                state: 4,
                quaternion: [0.0, 0.0, q, q],
                center_of_mass: [-0.75, -0.375, 0.75],
                velocity: [-0.1875, 0.09375, -0.046_875],
                omega: [0.375, -0.5625, -0.1875],
                right: [0.0, 1.0, 0.0],
                up: [-1.0, 0.0, 0.0],
                at: [0.0, 0.0, 1.0],
                force: [-0.75, 1.5, -2.25],
                torque: [0.5625, -0.375, -0.09375],
                inverse_inertia_full: [0.90625, -0.023_437_5, 0.011_718_75],
                inverse_mass: 0.453_125,
                inverse_inertia_split: [1.40625, 1.203_125, -0.005_859_375],
            },
            body_b: SimulationDriveBuildBodyInput {
                state: 0x14,
                quaternion: [q, 0.0, 0.0, q],
                center_of_mass: [1.5, 0.75, -1.5],
                velocity: [0.375, -0.1875, 0.09375],
                omega: [-0.75, 1.125, 0.375],
                right: [1.0, 0.0, 0.0],
                up: [0.0, 0.0, 1.0],
                at: [0.0, -1.0, 0.0],
                force: [1.5, -3.0, 4.5],
                torque: [-1.125, 0.75, 0.1875],
                inverse_inertia_full: [1.1875, 0.046_875, -0.023_437_5],
                inverse_mass: 0.59375,
                inverse_inertia_split: [1.6875, 1.34375, 0.011_718_75],
            },
            frames: SimulationDriveFrames {
                quaternion_a: [0.25, 0.0625, -0.125, 0.9375],
                position_a: [-0.75, 0.375, 0.5, 17.0],
                quaternion_b: [-0.125, 0.3125, 0.0625, 0.875],
                position_b: [0.625, -0.5, 1.125, -19.0],
            },
            dynamics: SimulationDriveDynamics {
                linear: SimulationDriveDynamicsParams {
                    spring: 1.25,
                    damping: 0.375,
                    strength: 2.5,
                    raw_type: HARD_DRIVE,
                },
                angular: SimulationDriveDynamicsParams {
                    spring: 0.5,
                    damping: 0.625,
                    strength: 1.75,
                    raw_type: SOFT_DRIVE,
                },
            },
            time_step: 0.0625,
            spy: 0x5566_7788,
            bindings: SimulationDriveRecordBindings {
                solver_delta_a: 1,
                solver_delta_b: 2,
                drive: 2,
            },
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "independent literal trace freezes every hardware-boundary operand"
    )]
    fn hard_soft_both_expected_calls() -> Vec<NumericCall> {
        use NumericCall::{Reciprocal as R, ReciprocalSqrt as S, Sum3 as D};
        use SimulationDriveDotSite as Dot;
        use SimulationDriveReciprocalSite as Recip;
        use SimulationDriveReciprocalSqrtSite as Rsqrt;
        vec![
            D(
                Dot::QuaternionA,
                [0, 0, 0],
                [0x3e00_0000, 0xbe80_0000, 0x3d80_0000],
            ),
            D(
                Dot::QuaternionB,
                [0, 0, 0],
                [0xbd80_0000, 0x3e40_0000, 0xbe00_0000],
            ),
            R(Recip::LinearNonSoftSpring, [0x3f88_0000; 4]),
            D(
                Dot::LinearHardLengthSquared,
                [0x3e00_0000, 0x3fb0_0000, 0xbfc0_0000],
                [0x3e00_0000, 0x3fb0_0000, 0xbfc0_0000],
            ),
            S(Rsqrt::LinearHardLength, [0x4085_0000; 4]),
            D(
                Dot::LinearBiasUp,
                [0xbc63_a194, 0x3d54_03a6, 0xbdd2_8da2],
                [0x3e58_0000, 0x3f76_0000, 0xbe28_0000],
            ),
            D(
                Dot::LinearBiasRight,
                [0xbc63_a194, 0x3d54_03a6, 0xbdd2_8da2],
                [0x3f66_0000, 0xbe84_0000, 0xbeab_ffff],
            ),
            D(
                Dot::LinearBiasAt,
                [0xbc63_a194, 0x3d54_03a6, 0xbdd2_8da2],
                [0x3ebb_ffff, 0x3d90_0000, 0x3f6c_0000],
            ),
            S(
                Rsqrt::AngularAxisNormalization,
                [0x3f76_9f00, 0x3f59_b700, 0x3f78_1700, 0x3f80_0000],
            ),
            D(
                Dot::AngularRightOmega,
                [0x3f62_2e90, 0xbc43_9de4, 0x3d53_eb0d],
                [0xbd80_0000, 0x3dc0_0000, 0x3d00_0000],
            ),
            D(
                Dot::AngularRightAcceleration,
                [0x3f62_2e90, 0xbc43_9de4, 0x3d53_eb0d],
                [0xbc40_0000, 0x3c00_0000, 0x3b00_0000],
            ),
            D(
                Dot::AngularUpOmega,
                [0x3dc7_85fc, 0x3f5b_0abf, 0x3dea_391d],
                [0xbd80_0000, 0x3dc0_0000, 0x3d00_0000],
            ),
            D(
                Dot::AngularUpAcceleration,
                [0x3dc7_85fc, 0x3f5b_0abf, 0x3dea_391d],
                [0xbc40_0000, 0x3c00_0000, 0x3b00_0000],
            ),
            D(
                Dot::AngularAtOmega,
                [0xbdba_e90c, 0xbca2_87de, 0x3f61_82ea],
                [0xbd80_0000, 0x3dc0_0000, 0x3d00_0000],
            ),
            D(
                Dot::AngularAtAcceleration,
                [0xbdba_e90c, 0xbca2_87de, 0x3f61_82ea],
                [0xbc40_0000, 0x3c00_0000, 0x3b00_0000],
            ),
            R(Recip::AngularSoftDynamics, [0x3f8c_8000; 4]),
            R(Recip::AngularSoftSpring, [0x3f8c_8000; 4]),
            R(Recip::AngularSoftDamping, [0x3f8c_8000; 4]),
            D(
                Dot::BodyAAngularRight,
                [0x3f7e_26f3, 0x3c4a_3d3d, 0x3d8f_b841],
                [0x3f62_2e90, 0xbc43_9de4, 0x3d53_eb0d],
            ),
            D(
                Dot::BodyAAngularUp,
                [0x3e09_c844, 0x3f90_4017, 0x3e43_97b2],
                [0x3dc7_85fc, 0x3f5b_0abf, 0x3dea_391d],
            ),
            D(
                Dot::BodyAAngularAt,
                [0xbdef_bb9b, 0xbcb4_4eba, 0x3fb7_6404],
                [0xbdba_e90c, 0xbca2_87de, 0x3f61_82ea],
            ),
            D(
                Dot::BodyALinearRight,
                [0x3eac_7400, 0x3f8e_a080, 0x3e21_8c00],
                [0x3e8e_0000, 0x3f57_8000, 0x3dc4_0000],
            ),
            D(
                Dot::BodyALinearUp,
                [0xbf43_f600, 0x3e98_b200, 0x3f61_0300],
                [0xbf2e_0000, 0x3e76_0000, 0x3f08_8000],
            ),
            D(
                Dot::BodyALinearAt,
                [0xbea7_1c00, 0xbe80_b601, 0x3e56_4c00],
                [0xbe91_0000, 0xbe3e_0002, 0x3e02_0000],
            ),
            D(
                Dot::BodyBAngularRight,
                [0x3f8d_0fab, 0x3d22_3ff6, 0x3d80_8038],
                [0x3f62_2e90, 0xbc43_9de4, 0x3d53_eb0d],
            ),
            D(
                Dot::BodyBAngularUp,
                [0x3e2f_cd89, 0x3f97_9977, 0x3e57_848d],
                [0x3dc7_85fc, 0x3f5b_0abf, 0x3dea_391d],
            ),
            D(
                Dot::BodyBAngularAt,
                [0xbe12_4714, 0xbc9d_739f, 0x3fc5_a5d9],
                [0xbdba_e90c, 0xbca2_87de, 0x3f61_82ea],
            ),
            D(
                Dot::BodyBLinearRight,
                [0xbe25_0fff, 0x3df4_4002, 0xbf4e_b100],
                [0xbe14_ffff, 0x3dca_0002, 0xbeee_0000],
            ),
            D(
                Dot::BodyBLinearUp,
                [0xbed3_c800, 0xbd28_c000, 0xbf59_cb00],
                [0xbeaf_8000, 0xbc10_0000, 0xbefc_0000],
            ),
            D(
                Dot::BodyBLinearAt,
                [0x3f3b_cc00, 0x3f21_ea00, 0xbeea_affe],
                [0x3f0f_0000, 0x3ee0_0000, 0xbe82_ffff],
            ),
            R(
                Recip::FinalLinear,
                [0x4029_4008, 0x4033_c60f, 0x400a_0ac7, 0x4029_4008],
            ),
            R(
                Recip::FinalAngular,
                [0x3fed_b4d0, 0x4003_6ada, 0x4029_664c, 0x3fed_b4d0],
            ),
        ]
    }

    fn prepared_hard_soft_expected_record(
        bindings: SimulationDriveRecordBindings,
    ) -> [[u32; 4]; 24] {
        [
            [
                0x3f00_0000,
                0xbe80_0000,
                0x3f40_0000,
                bindings.solver_delta_a,
            ],
            [
                0xbec0_0000,
                0x3f20_0000,
                0x3e80_0000,
                bindings.solver_delta_b,
            ],
            [0, 0, 0, 0x3f80_0000],
            [0, 0, 0, 0x3db6_34d7],
            [0x3ead_f199, 0x3d99_cb1b, 0x3e2e_52d9, 0x1122_3344],
            [0x3ef3_96b3, 0x3d42_55c0, 0xbd0d_3b36, 0x3ef3_96b3],
            [0xbdc7_a83c, 0x3eaf_2750, 0x3d05_8645, 0x3c91_3489],
            [0xbbd2_abd9, 0x3ed5_5893, 0xbbf5_9ea0, 0x3c88_b48a],
            [0xbe02_1464, 0xbd6f_3bf1, 0x3eda_d4f0, 0x3c4e_c6b4],
            [0x3ce4_3a2c, 0x3d64_21e1, 0x3eaa_660c, 0x3c3b_01d9],
            [0x3b57_db31, 0x3cb9_b3ec, 0xbd36_c284, 0x3cb2_085c],
            [0xbb76_672f, 0x3ba1_b7df, 0x3a22_b918, 0x3c11_13bc],
            [0x3f66_0000, 0xbe84_0000, 0xbeab_ffff, bindings.drive],
            [0x3e58_0000, 0x3f76_0000, 0xbe28_0000, 0x3e58_0000],
            [0x3ebb_ffff, 0x3d90_0000, 0x3f6c_0000, 0xbe84_0000],
            [0x3f62_2e90, 0xbc43_9de4, 0x3d53_eb0d, 0x3f62_2e90],
            [0x3dc7_85fc, 0x3f5b_0abf, 0x3dea_391d, 0x3dc7_85fc],
            [0xbdba_e90c, 0xbca2_87de, 0x3f61_82ea, 0xbdba_e90c],
            [0x3f90_0000, 0x3d00_0000, 0xbc80_0000, 0x3f10_0000],
            [0x3d00_0000, 0x3fa8_0000, 0x3c00_0000, 0x3f10_0000],
            [0xbc80_0000, 0x3c00_0000, 0x3fd0_0000, 0x3f10_0000],
            [0x3fa0_0000, 0x3d80_0000, 0xbd00_0000, 0x3f20_0000],
            [0x3d80_0000, 0x3fb0_0000, 0x3c80_0000, 0x3f20_0000],
            [0xbd00_0000, 0x3c80_0000, 0x3fe0_0000, 0x3f20_0000],
        ]
    }

    fn prepared_second_expected_record(bindings: SimulationDriveRecordBindings) -> [[u32; 4]; 24] {
        [
            [
                0xbec0_0000,
                0xbf40_0000,
                0x3f00_0000,
                bindings.solver_delta_a,
            ],
            [
                0x3f20_0000,
                0xbf90_0000,
                0xbf00_0000,
                bindings.solver_delta_b,
            ],
            [0, 0, 0, 0x3f80_0000],
            [0, 0, 0, 0x3d21_6180],
            [0x3e37_a006, 0xbd88_af0b, 0x3e39_97d2, 0x5566_7788],
            [0x3e5b_b9d4, 0xbe5a_e956, 0x3e97_bb32, 0x3e5b_b9d4],
            [0x3e01_9e21, 0x3dd8_6a84, 0xbe65_4311, 0x3b10_050a],
            [0x3ec7_d88e, 0x3e23_b946, 0xbe20_368c, 0x3b63_ce6a],
            [0x3be6_6e76, 0x3e99_c4ec, 0x3e03_01c4, 0x3b5b_c127],
            [0x3cd9_a215, 0x3e99_9b1f, 0x3e8a_8986, 0x3b44_fe74],
            [0x3c90_ae88, 0xbb17_2ea3, 0x3cf5_0ff4, 0x3b5a_5844],
            [0x3ae0_047e, 0x3a9b_14db, 0xbaaf_3687, 0x3b51_bed4],
            [0x3f4c_0000, 0x3f0f_fffe, 0x3d00_0000, bindings.drive],
            [0xbe3f_fffe, 0x3e98_0004, 0x3f57_fffe, 0xbe3f_fffe],
            [0x3f07_ffff, 0xbf27_fffe, 0x3ec0_0003, 0x3f0f_fffe],
            [0x3edf_f889, 0x3f4b_b4f1, 0x3d5d_d671, 0x3edf_f889],
            [0xbef8_ec15, 0x3eba_2b31, 0x3f2e_a9f5, 0xbef8_ec15],
            [0x3f22_0b08, 0xbeab_19f4, 0x3f13_f3cf, 0x3f22_0b08],
            [0x3f68_0000, 0xbcc0_0000, 0x3c40_0000, 0x3ee8_0000],
            [0xbcc0_0000, 0x3f9a_0000, 0xbbc0_0000, 0x3ee8_0000],
            [0x3c40_0000, 0xbbc0_0000, 0x3fb4_0000, 0x3ee8_0000],
            [0x3f98_0000, 0x3d40_0000, 0xbcc0_0000, 0x3f18_0000],
            [0x3d40_0000, 0x3fac_0000, 0x3c40_0000, 0x3f18_0000],
            [0xbcc0_0000, 0x3c40_0000, 0x3fd8_0000, 0x3f18_0000],
        ]
    }

    fn identity_matrix() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    fn zero_dynamic_inertia() -> std::sync::Arc<crate::simulation_rigid_body_pool::SimulationInertia>
    {
        std::sync::Arc::new(crate::simulation_rigid_body_pool::SimulationInertia {
            inverse_tensor: [0.0; 3],
            inverse_mass: 0.0,
            ..crate::simulation_rigid_body_pool::SimulationInertia::native_default()
        })
    }

    fn try_prepare_active_drive_island_batch_for_test<B: SimulationDriveNumericBackend>(
        drive_pool: &SimulationDrivePool,
        body_pool: &mut SimulationRigidBodyPool,
        island_map: SimulationDriveIslandMapSource<'_>,
        backend: &mut B,
    ) -> Result<
        SimulationPreparedDriveIslandBatch,
        SimulationDriveIslandBatchPreparationError<B::Error>,
    > {
        let phase = body_pool.try_prepare_dynamic_update_phase().expect("phase");
        try_prepare_active_drive_island_batch_with_backend(drive_pool, &phase, island_map, backend)
    }

    fn install_prepared_body_fixture(
        target: &mut SimulationRigidBodyDriveBuildSnapshot,
        source: SimulationDriveBuildBodyInput,
    ) {
        target.state = source.state;
        target.quaternion = source.quaternion;
        target.center_of_mass = source.center_of_mass;
        target.velocity = source.velocity;
        target.omega = source.omega;
        target.right = source.right;
        target.up = source.up;
        target.at = source.at;
        target.force = source.force;
        target.torque = source.torque;
        target.inverse_inertia_full = source.inverse_inertia_full;
        target.inverse_mass = source.inverse_mass;
        target.inverse_inertia_split = source.inverse_inertia_split;
    }

    struct PreparedNumericSnapshotFixture {
        source: SimulationDrivePreparedBatchSource,
        snapshots: Vec<SimulationDriveBuildSnapshot>,
        drives: Vec<SimulationDriveIdentity>,
        body_a: SimulationRigidBodyObservation,
        body_b: SimulationRigidBodyObservation,
    }

    struct PreparedLivePoolFixture {
        body_pool: SimulationRigidBodyPool,
        drive_pool: SimulationDrivePool,
        batch: SimulationPreparedDriveJacobianBatch,
        child: crate::simulation_rigid_body_pool::SimulationRigidBodyHandle,
        parent: crate::simulation_rigid_body_pool::SimulationRigidBodyHandle,
        drives: Vec<crate::simulation_drive_pool::SimulationDriveHandle>,
        frames: std::sync::Arc<std::sync::RwLock<SimulationDriveFrames>>,
        dynamics: std::sync::Arc<std::sync::RwLock<SimulationDriveDynamics>>,
    }

    struct PreparedLiveIslandFixture {
        body_pool: SimulationRigidBodyPool,
        drive_pool: SimulationDrivePool,
        batch: SimulationPreparedDriveIslandBatch,
        child: crate::simulation_rigid_body_pool::SimulationRigidBodyHandle,
        parent: crate::simulation_rigid_body_pool::SimulationRigidBodyHandle,
        drives: Vec<crate::simulation_drive_pool::SimulationDriveHandle>,
        frames: std::sync::Arc<std::sync::RwLock<SimulationDriveFrames>>,
        dynamics: std::sync::Arc<std::sync::RwLock<SimulationDriveDynamics>>,
    }

    struct TwoIslandDriveFixture {
        body_pool: SimulationRigidBodyPool,
        drive_pool: SimulationDrivePool,
        pairs: [(
            crate::simulation_rigid_body_pool::SimulationRigidBodyHandle,
            crate::simulation_rigid_body_pool::SimulationRigidBodyHandle,
        ); 2],
    }

    type PreparedLiveEffectBits = (Vec<[[u32; 4]; 24]>, [[u32; 4]; 4], [[u32; 4]; 4]);

    type PreparedLiveIslandEffect = (
        Vec<[[u32; 4]; 24]>,
        [[u32; 4]; 4],
        [[u32; 4]; 4],
        crate::simulation_rigid_body_pool::SimulationRigidBodyPoolSnapshot,
    );

    fn prepared_live_pool_fixture(drive_count: usize) -> PreparedLivePoolFixture {
        let inertia = std::sync::Arc::new(crate::simulation_rigid_body_pool::SimulationInertia {
            inverse_tensor: [0.0; 3],
            inverse_mass: 0.0,
            ..crate::simulation_rigid_body_pool::SimulationInertia::native_default()
        });
        let mut body_pool = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let child = body_pool
            .try_add_rigid_body(identity_matrix(), Some(&inertia), 4)
            .expect("child Add")
            .expect("child capacity");
        let parent = body_pool
            .try_add_rigid_body(identity_matrix(), Some(&inertia), 4)
            .expect("parent Add")
            .expect("parent capacity");
        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        body_pool.begin_dynamic_update_frame(input.time_step);
        let frames = std::sync::Arc::new(std::sync::RwLock::new(input.frames));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(input.dynamics));
        let mut drive_pool =
            SimulationDrivePool::try_new(drive_count, &body_pool).expect("Drive pool");
        let mut drives = Vec::new();
        for _ in 0..drive_count {
            drives.push(
                drive_pool
                    .try_add_drive(&body_pool, parent, child, &frames, &dynamics)
                    .expect("Drive Add")
                    .expect("Drive capacity"),
            );
        }
        let batch = try_prepare_active_drive_jacobian_batch(&drive_pool, &body_pool)
            .expect("prepared live batch");
        PreparedLivePoolFixture {
            body_pool,
            drive_pool,
            batch,
            child,
            parent,
            drives,
            frames,
            dynamics,
        }
    }

    fn prepared_live_island_fixture(drive_count: usize, island: i8) -> PreparedLiveIslandFixture {
        let inertia = std::sync::Arc::new(
            crate::simulation_rigid_body_pool::SimulationInertia::native_default(),
        );
        let mut body_pool = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let child = body_pool
            .try_add_rigid_body(identity_matrix(), Some(&inertia), 4)
            .expect("child Add")
            .expect("child capacity");
        let parent = body_pool
            .try_add_rigid_body(identity_matrix(), Some(&inertia), 4)
            .expect("parent Add")
            .expect("parent capacity");
        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        body_pool.begin_dynamic_update_frame(input.time_step);
        let frames = std::sync::Arc::new(std::sync::RwLock::new(input.frames));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(input.dynamics));
        let mut drive_pool =
            SimulationDrivePool::try_new(drive_count, &body_pool).expect("Drive pool");
        let mut drives = Vec::new();
        for _ in 0..drive_count {
            drives.push(
                drive_pool
                    .try_add_drive(&body_pool, parent, child, &frames, &dynamics)
                    .expect("Drive Add")
                    .expect("Drive capacity"),
            );
        }
        let frame = body_pool.frame_identity().expect("frame");
        let map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &body_pool,
            frame,
            [
                (parent.observation(), island),
                (child.observation(), island),
            ],
        )
        .expect("island map");
        let mut backend = SimulationDriveHostNumericBackend;
        let batch = {
            let phase = body_pool.try_prepare_dynamic_update_phase().expect("phase");
            try_prepare_active_drive_island_batch_with_backend(
                &drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut backend,
            )
            .expect("prepared island batch")
        };
        PreparedLiveIslandFixture {
            body_pool,
            drive_pool,
            batch,
            child,
            parent,
            drives,
            frames,
            dynamics,
        }
    }

    fn prepared_live_island_map(
        fixture: &PreparedLiveIslandFixture,
        island: i8,
    ) -> SimulationRigidBodyIslandMapSnapshot {
        SimulationRigidBodyIslandMapSnapshot::try_new(
            &fixture.body_pool,
            fixture.body_pool.frame_identity().expect("frame"),
            [
                (fixture.parent.observation(), island),
                (fixture.child.observation(), island),
            ],
        )
        .expect("island map")
    }

    fn two_island_drive_fixture() -> TwoIslandDriveFixture {
        let mut body_pool = SimulationRigidBodyPool::try_new(4).expect("body pool");
        let inertia = zero_dynamic_inertia();
        let pairs = std::array::from_fn(|_| {
            let child = body_pool
                .try_add_rigid_body(identity_matrix(), Some(&inertia), 4)
                .expect("child Add")
                .expect("child capacity");
            let parent = body_pool
                .try_add_rigid_body(identity_matrix(), Some(&inertia), 4)
                .expect("parent Add")
                .expect("parent capacity");
            (child, parent)
        });
        let inputs = [
            numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4),
            second_prepared_numeric_input(),
        ];
        body_pool.begin_dynamic_update_frame(inputs[0].time_step);
        let mut drive_pool = SimulationDrivePool::try_new(2, &body_pool).expect("Drive pool");
        for (ordinal, &(child, parent)) in pairs.iter().enumerate() {
            let frames = std::sync::Arc::new(std::sync::RwLock::new(inputs[ordinal].frames));
            let dynamics = std::sync::Arc::new(std::sync::RwLock::new(inputs[ordinal].dynamics));
            drive_pool
                .try_add_drive(&body_pool, parent, child, &frames, &dynamics)
                .expect("Drive Add")
                .expect("Drive capacity");
        }
        TwoIslandDriveFixture {
            body_pool,
            drive_pool,
            pairs,
        }
    }

    fn two_island_map(fixture: &TwoIslandDriveFixture) -> SimulationRigidBodyIslandMapSnapshot {
        let [(child_0, parent_0), (child_1, parent_1)] = fixture.pairs;
        SimulationRigidBodyIslandMapSnapshot::try_new(
            &fixture.body_pool,
            fixture.body_pool.frame_identity().expect("frame"),
            [
                (parent_0.observation(), 1),
                (child_0.observation(), 1),
                (parent_1.observation(), 3),
                (child_1.observation(), 3),
            ],
        )
        .expect("two-island map")
    }

    fn seed_prepared_live_island_rf(fixture: &mut PreparedLiveIslandFixture) {
        for (body, seed) in [(fixture.child, 1.0), (fixture.parent, 17.0)] {
            fixture
                .body_pool
                .try_with_solver_delta_row_mut(body.observation(), |row| {
                    for (ordinal, lane) in (0_u8..).zip(row.iter_mut().flatten()) {
                        *lane = seed + f32::from(ordinal);
                    }
                })
                .expect("seed prepared island RF");
        }
    }

    fn prepared_live_island_effect(
        fixture: &PreparedLiveIslandFixture,
    ) -> PreparedLiveIslandEffect {
        let records = fixture
            .batch
            .records
            .iter()
            .map(|prepared| drive_record_bits(&prepared.record))
            .collect();
        let child = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.child.observation(), |row| bits_rows(*row))
            .expect("child RF bits");
        let parent = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.parent.observation(), |row| bits_rows(*row))
            .expect("parent RF bits");
        (records, child, parent, fixture.body_pool.snapshot())
    }

    fn after_one_dynamic_update_phase_identity_issue(
        mut effect: PreparedLiveIslandEffect,
    ) -> PreparedLiveIslandEffect {
        effect.3.next_dynamic_update_phase_identity = effect
            .3
            .next_dynamic_update_phase_identity
            .and_then(|identity| identity.checked_add(1));
        effect
    }

    fn seed_prepared_live_rf(fixture: &mut PreparedLivePoolFixture) {
        for (body, seed) in [(fixture.child, 1.0), (fixture.parent, 17.0)] {
            fixture
                .body_pool
                .try_with_solver_delta_row_mut(body.observation(), |row| {
                    for (ordinal, lane) in (0_u8..).zip(row.iter_mut().flatten()) {
                        *lane = seed + f32::from(ordinal);
                    }
                })
                .expect("seed prepared RF");
        }
    }

    fn prepared_live_effect_bits(fixture: &PreparedLivePoolFixture) -> PreparedLiveEffectBits {
        let records = fixture
            .batch
            .records
            .iter()
            .map(|prepared| drive_record_bits(&prepared.record))
            .collect();
        let child = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.child.observation(), |row| bits_rows(*row))
            .expect("child RF bits");
        let parent = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.parent.observation(), |row| bits_rows(*row))
            .expect("parent RF bits");
        (records, child, parent)
    }

    fn prepared_numeric_snapshots(drive_count: usize) -> PreparedNumericSnapshotFixture {
        let mut body_pool = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let child = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("child Add")
            .expect("child capacity");
        let parent = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("parent Add")
            .expect("parent capacity");
        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        body_pool.begin_dynamic_update_frame(input.time_step);
        let frames = std::sync::Arc::new(std::sync::RwLock::new(input.frames));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(input.dynamics));
        let mut drive_pool =
            SimulationDrivePool::try_new(drive_count, &body_pool).expect("Drive pool");
        let mut drives = Vec::new();
        for _ in 0..drive_count {
            drives.push(
                drive_pool
                    .try_add_drive(&body_pool, parent, child, &frames, &dynamics)
                    .expect("Drive Add")
                    .expect("Drive capacity"),
            );
        }
        let mut snapshots = drive_pool
            .try_active_build_snapshots(&body_pool)
            .expect("Build snapshots");
        for snapshot in &mut snapshots {
            install_prepared_body_fixture(&mut snapshot.body_a, input.body_a);
            install_prepared_body_fixture(&mut snapshot.body_b, input.body_b);
            snapshot.frames = input.frames;
            snapshot.dynamics = input.dynamics;
            snapshot.time_step = input.time_step;
            snapshot.spy = input.spy;
        }
        PreparedNumericSnapshotFixture {
            source: SimulationDrivePreparedBatchSource {
                drive_pool: drive_pool.authority(),
                body_pool: body_pool.authority(),
            },
            snapshots,
            drives: drives
                .into_iter()
                .map(crate::simulation_drive_pool::SimulationDriveHandle::identity)
                .collect(),
            body_a: child.observation(),
            body_b: parent.observation(),
        }
    }

    fn bits4(values: [f32; 4]) -> [u32; 4] {
        values.map(f32::to_bits)
    }

    fn bits3(values: [f32; 3]) -> [u32; 3] {
        values.map(f32::to_bits)
    }

    fn bits_rows(values: [[f32; 4]; 4]) -> [[u32; 4]; 4] {
        values.map(bits4)
    }

    fn bits3_word(values: [f32; 3], word: u32) -> [u32; 4] {
        [
            values[0].to_bits(),
            values[1].to_bits(),
            values[2].to_bits(),
            word,
        ]
    }

    fn drive_record_bits(value: &SimulationDriveJacobian) -> [[u32; 4]; 24] {
        [
            bits3_word(value.attachment_offset_a, value.solver_delta_a.0),
            bits3_word(value.attachment_offset_b, value.solver_delta_b.0),
            bits3_word(value.position_impulse, value.linear_dynamics.to_bits()),
            bits3_word(value.angular_impulse, value.angular_dynamics.to_bits()),
            bits3_word(value.linear_transpose_right, value.spy),
            bits3_word(value.angular_transpose_right, value.pad_0.to_bits()),
            bits3_word(value.linear_transpose_up, value.position_limit_x.to_bits()),
            bits3_word(value.angular_transpose_up, value.position_limit_y.to_bits()),
            bits3_word(value.linear_transpose_at, value.angular_limit_x.to_bits()),
            bits3_word(value.angular_transpose_at, value.angular_limit_y.to_bits()),
            bits3_word(value.position_offset, value.position_limit_z.to_bits()),
            bits3_word(value.angular_offset, value.angular_limit_z.to_bits()),
            bits3_word(value.linear_right, value.drive_slot.0),
            bits3_word(value.linear_up, value.pad_1.to_bits()),
            bits3_word(value.linear_at, value.pad_2.to_bits()),
            bits3_word(value.angular_right, value.pad_3.to_bits()),
            bits3_word(value.angular_up, value.pad_4.to_bits()),
            bits3_word(value.angular_at, value.pad_5.to_bits()),
            bits3_word(
                value.inverse_inertia_a_right,
                value.inverse_mass_a.to_bits(),
            ),
            bits3_word(value.inverse_inertia_a_up, value.pad_6.to_bits()),
            bits3_word(value.inverse_inertia_a_at, value.pad_7.to_bits()),
            bits3_word(
                value.inverse_inertia_b_right,
                value.inverse_mass_b.to_bits(),
            ),
            bits3_word(value.inverse_inertia_b_up, value.pad_8.to_bits()),
            bits3_word(value.inverse_inertia_b_at, value.pad_9.to_bits()),
        ]
    }

    fn geometry_fixture() -> (
        SimulationDriveBodyGeometryInput,
        SimulationDriveBodyGeometryInput,
        SimulationDriveFrames,
    ) {
        (
            SimulationDriveBodyGeometryInput {
                quaternion: [0.125, -0.375, 0.25, 0.875],
                right: [f32::from_bits(0x3cf4_54ab), -3.25, 0.125],
                up: [f32::from_bits(0xc06e_53fc), 7.5, -4.0],
                at: [f32::from_bits(0x3758_43fb), -0.75, 9.0],
            },
            SimulationDriveBodyGeometryInput {
                quaternion: [-0.25, 0.375, -0.5, 0.75],
                right: [0.36, 0.48, -0.8],
                up: [-0.8, 0.6, 0.0],
                at: [0.48, 0.64, 0.6],
            },
            SimulationDriveFrames {
                quaternion_a: [-0.5, 0.0625, 0.3125, 0.75],
                position_a: [
                    f32::from_bits(0x44b6_7e23),
                    f32::from_bits(0x454e_8ed8),
                    f32::from_bits(0x3bc8_70ea),
                    f32::from_bits(0x7fc1_2345),
                ],
                quaternion_b: [0.3125, -0.125, 0.25, 0.875],
                position_b: [-7.0, 8.0, -9.0, -12_345.0],
            },
        )
    }

    fn effective_mass_fixture() -> (
        SimulationDriveEffectiveMassBodyInput,
        SimulationDriveEffectiveMassBodyInput,
        [[f32; 3]; 3],
        [[f32; 3]; 3],
    ) {
        (
            SimulationDriveEffectiveMassBodyInput {
                state: 4,
                attachment_offset: [0.5, -1.0, 2.0],
                inverse_inertia_full: [2.0, 0.5, -0.25],
                inverse_mass: 0.125,
                inverse_inertia_split: [4.0, 3.0, 0.75],
            },
            SimulationDriveEffectiveMassBodyInput {
                state: 4,
                attachment_offset: [-0.75, 0.25, 1.5],
                inverse_inertia_full: [1.5, -0.375, 0.625],
                inverse_mass: 0.25,
                inverse_inertia_split: [2.5, 1.25, -0.5],
            },
            [[1.0, 2.0, -1.0], [0.5, -2.0, 3.0], [-1.5, 0.25, 2.0]],
            [[0.25, 1.0, -2.0], [-1.0, 0.5, 1.5], [2.0, -0.75, 0.125]],
        )
    }

    #[allow(
        clippy::type_complexity,
        reason = "one private tuple keeps every independently varied oracle input visible"
    )]
    fn prediction_fixture() -> (
        SimulationDrivePredictionBodyInput,
        SimulationDrivePredictionBodyInput,
        [f32; 3],
        [f32; 3],
        f32,
        SimulationDriveCoefficients,
        [[f32; 3]; 3],
    ) {
        (
            SimulationDrivePredictionBodyInput {
                center_of_mass: [1.125, -2.25, 3.5],
                velocity: [0.31, -0.47, 0.83],
                omega: [-0.73, 0.29, 0.41],
                force: [1.17, -0.63, 0.38],
                torque: [-0.52, 0.91, -0.27],
            },
            SimulationDrivePredictionBodyInput {
                center_of_mass: [-1.75, 0.625, 2.125],
                velocity: [-0.26, 0.74, -0.59],
                omega: [0.67, -0.35, 0.22],
                force: [-0.88, 0.57, 1.09],
                torque: [0.43, -0.76, 0.61],
            },
            [
                f32::from_bits(0x3cab_3679),
                f32::from_bits(0xbf47_58e3),
                f32::from_bits(0x3f24_fdf4),
            ],
            [
                f32::from_bits(0xbf65_5326),
                f32::from_bits(0x3d3a_c713),
                f32::from_bits(0xbeb9_b3d2),
            ],
            f32::from_bits(0x3d4c_cccd),
            SimulationDriveCoefficients {
                dynamics: 0.73,
                spring_factor: -0.44,
                damping_factor: 0.29,
            },
            [
                [0.76, -0.48, 0.31],
                [0.52, 0.84, -0.13],
                [-0.19, 0.27, 0.94],
            ],
        )
    }

    fn angular_fixture() -> (
        SimulationDriveJacobianRqd,
        [[f32; 4]; 3],
        SimulationDriveAngularEstimateSeeds,
    ) {
        (
            SimulationDriveJacobianRqd {
                rows: [
                    [0.25, -0.5, 0.75, 0.25],
                    [-0.625, 0.375, 0.125, -0.5],
                    [0.875, -0.25, -0.375, 0.75],
                    [0.0, 0.0, 0.0, 0.625],
                ],
            },
            [
                [0.125, 0.25, 0.5, -0.75],
                [-0.375, 0.625, -0.875, 0.0625],
                [0.75, -0.5, 0.25, -0.125],
            ],
            SimulationDriveAngularEstimateSeeds {
                axis_reciprocal_sqrt: [1.03125, 1.15625, 1.515_625, 1.0],
                coefficients: SimulationDriveReciprocalSeeds {
                    dynamics: 1.0,
                    spring: 1.0,
                    damping: 1.0,
                },
                hard_reciprocal_sqrt: 1.0,
            },
        )
    }

    fn zero_angular_body() -> SimulationDriveAngularBodyInput {
        SimulationDriveAngularBodyInput {
            omega: [0.0; 3],
            torque: [0.0; 3],
        }
    }

    fn no_drive_angular_params() -> SimulationDriveDynamicsParams {
        SimulationDriveDynamicsParams {
            spring: 0.75,
            damping: 0.5,
            strength: 0.25,
            raw_type: 0,
        }
    }

    fn horus_fixture() -> (SimulationDriveJacobian, [[f32; 4]; 4], [[f32; 4]; 4]) {
        let mut jacobian = SimulationDriveJacobian::zeroed();
        jacobian.attachment_offset_a = [0.3, -0.5, 0.7];
        jacobian.solver_delta_a = SolverDeltaRowToken(0.125_f32.to_bits());
        jacobian.attachment_offset_b = [-0.4, 0.8, -0.2];
        jacobian.solver_delta_b = SolverDeltaRowToken((-0.25_f32).to_bits());
        jacobian.position_impulse = [0.11, -0.22, 0.33];
        jacobian.linear_dynamics = -0.75;
        jacobian.angular_impulse = [-0.17, 0.29, -0.41];
        jacobian.angular_dynamics = 0.65;
        jacobian.linear_transpose_right = [0.71, -0.13, 0.29];
        jacobian.spy = 0.03_f32.to_bits();
        jacobian.angular_transpose_right = [-0.43, 0.17, 0.61];
        jacobian.pad_0 = -0.09;
        jacobian.linear_transpose_up = [-0.37, 0.83, 0.19];
        jacobian.position_limit_x = 0.31;
        jacobian.angular_transpose_up = [0.79, -0.53, 0.11];
        jacobian.position_limit_y = 0.27;
        jacobian.linear_transpose_at = [0.23, -0.31, 0.97];
        jacobian.angular_limit_x = 0.22;
        jacobian.angular_transpose_at = [-0.27, 0.47, -0.73];
        jacobian.angular_limit_y = 0.18;
        jacobian.position_offset = [0.08, -0.04, 0.12];
        jacobian.position_limit_z = 0.19;
        jacobian.angular_offset = [-0.06, 0.14, -0.1];
        jacobian.angular_limit_z = 0.26;
        jacobian.linear_right = [0.9, -0.2, 0.3];
        jacobian.drive_slot = DriveSlotToken(0.17_f32.to_bits());
        jacobian.linear_up = [-0.4, 1.1, 0.25];
        jacobian.pad_1 = -0.21;
        jacobian.linear_at = [0.15, -0.35, 0.8];
        jacobian.pad_2 = 0.23;
        jacobian.angular_right = [-0.3, 0.7, -0.1];
        jacobian.pad_3 = -0.27;
        jacobian.angular_up = [0.6, -0.2, 0.5];
        jacobian.pad_4 = 0.29;
        jacobian.angular_at = [-0.45, 0.15, 0.95];
        jacobian.pad_5 = -0.33;
        jacobian.inverse_inertia_a_right = [0.8, 0.1, -0.05];
        jacobian.inverse_mass_a = 0.4;
        jacobian.inverse_inertia_a_up = [0.1, 1.2, 0.07];
        jacobian.pad_6 = -0.39;
        jacobian.inverse_inertia_a_at = [-0.05, 0.07, 1.5];
        jacobian.pad_7 = 0.41;
        jacobian.inverse_inertia_b_right = [1.1, -0.08, 0.04];
        jacobian.inverse_mass_b = 0.6;
        jacobian.inverse_inertia_b_up = [-0.08, 0.95, -0.06];
        jacobian.pad_8 = 0.47;
        jacobian.inverse_inertia_b_at = [0.04, -0.06, 1.35];
        jacobian.pad_9 = -0.49;

        let solver_delta_a = [
            [0.25, -0.5, 0.75, 1.0],
            [11.0, 12.0, 13.0, 14.0],
            [-0.2, 0.4, -0.6, 0.8],
            [31.0, 32.0, 33.0, 34.0],
        ];
        let solver_delta_b = [
            [-0.3, 0.6, -0.9, 1.2],
            [41.0, 42.0, 43.0, 44.0],
            [0.15, -0.35, 0.55, -0.75],
            [61.0, 62.0, 63.0, 64.0],
        ];
        (jacobian, solver_delta_a, solver_delta_b)
    }

    #[test]
    fn disabled_zero_strength_limits_clamp_nonzero_candidates_without_rf_effect() {
        // Synthetic numeric discriminator for source HookDisable*'s resulting
        // zero caps; mounted coverage separately checks the real third Build
        // record. Deliberately nonzero incoming RF and bias prevent a vacuous
        // all-zero-input test. No production record/endpoint is fabricated.
        let (mut record, a, b) = horus_fixture();
        record.position_impulse = [0.0; 3];
        record.angular_impulse = [0.0; 3];
        record.linear_dynamics = 1.0;
        record.angular_dynamics = 1.0;
        record.position_limit_x = 0.0;
        record.position_limit_y = 0.0;
        record.position_limit_z = 0.0;
        record.angular_limit_x = 0.0;
        record.angular_limit_y = 0.0;
        record.angular_limit_z = 0.0;
        let solved = solve_drive_horus_island_record_host(&mut record, a, b);
        assert_eq!(record.position_impulse.map(f32::to_bits), [0; 3]);
        assert_eq!(record.angular_impulse.map(f32::to_bits), [0; 3]);
        assert_eq!(
            solved.solver_delta_a.map(|r| r.map(f32::to_bits)),
            a.map(|r| r.map(f32::to_bits))
        );
        assert_eq!(
            solved.solver_delta_b.map(|r| r.map(f32::to_bits)),
            b.map(|r| r.map(f32::to_bits))
        );
    }

    fn horus_second_representable_record() -> SimulationDriveJacobian {
        let (mut jacobian, _, _) = horus_fixture();
        jacobian.attachment_offset_a = [-0.15, 0.45, -0.65];
        jacobian.solver_delta_a = SolverDeltaRowToken(2);
        jacobian.attachment_offset_b = [0.55, -0.25, 0.35];
        jacobian.solver_delta_b = SolverDeltaRowToken(3);
        jacobian.position_impulse = [-0.09, 0.18, -0.27];
        jacobian.linear_dynamics = 0.55;
        jacobian.angular_impulse = [0.13, -0.21, 0.34];
        jacobian.angular_dynamics = -0.6;
        jacobian.position_limit_x = 0.28;
        jacobian.position_limit_y = 0.24;
        jacobian.position_limit_z = 0.21;
        jacobian.angular_limit_x = 0.20;
        jacobian.angular_limit_y = 0.17;
        jacobian.angular_limit_z = 0.23;
        jacobian.position_offset = [-0.05, 0.09, -0.07];
        jacobian.angular_offset = [0.12, -0.08, 0.04];
        jacobian.linear_right = [-0.77, 0.24, 0.31];
        jacobian.drive_slot = DriveSlotToken(2);
        jacobian.linear_up = [0.43, 0.96, -0.28];
        jacobian.pad_1 = 0.27;
        jacobian.linear_at = [-0.18, 0.39, 0.72];
        jacobian.pad_2 = -0.16;
        jacobian.angular_right = [0.34, -0.62, 0.14];
        jacobian.pad_3 = 0.22;
        jacobian.angular_up = [-0.57, 0.26, -0.48];
        jacobian.pad_4 = -0.31;
        jacobian.angular_at = [0.41, -0.19, 0.88];
        jacobian.pad_5 = 0.36;
        jacobian.inverse_mass_a = 0.35;
        jacobian.inverse_mass_b = 0.7;
        jacobian
    }

    #[test]
    fn geometry_composes_body_times_frame_and_feeds_rqd() {
        let (body_a, body_b, frames) = geometry_fixture();
        let inputs_before = (body_a, body_b, frames);
        let quaternion_a = compose_drive_quaternion_host(body_a.quaternion, frames.quaternion_a);
        let quaternion_b = compose_drive_quaternion_host(body_b.quaternion, frames.quaternion_b);
        assert_eq!(
            bits4(quaternion_a),
            [0xbef4_0000, 0xbec8_0000, 0x3e90_0000, 0x3f2a_0000]
        );
        assert_eq!(
            bits4(quaternion_b),
            [0x3d40_0000, 0x3e10_0000, 0xbeac_0000, 0x3f68_0000]
        );
        assert_eq!(
            bits4(compose_drive_quaternion_host(
                frames.quaternion_a,
                body_a.quaternion,
            )),
            [0xbe58_0000, 0xbd80_0000, 0x3f24_0000, 0x3f2a_0000]
        );

        let geometry = build_drive_geometry_host(body_a, body_b, frames);
        assert_eq!(
            geometry.rqd,
            SimulationDriveJacobianRqd::create(quaternion_a, quaternion_b)
        );
        assert_eq!(body_a, inputs_before.0);
        assert_eq!(body_b, inputs_before.1);
        assert_eq!(
            bits4(frames.quaternion_a),
            bits4(inputs_before.2.quaternion_a)
        );
        assert_eq!(bits4(frames.position_a), bits4(inputs_before.2.position_a));
        assert_eq!(
            bits4(frames.quaternion_b),
            bits4(inputs_before.2.quaternion_b)
        );
        assert_eq!(bits4(frames.position_b), bits4(inputs_before.2.position_b));
    }

    #[test]
    fn geometry_basis_preserves_native_sqrt2_fma_rounding() {
        let (body_a, body_b, frames) = geometry_fixture();
        let geometry = build_drive_geometry_host(body_a, body_b, frames);
        assert_eq!(
            bits3(geometry.linear_right),
            [0x3f3c_1800, 0xbf18_8000, 0xbe92_9fff]
        );
        assert_eq!(
            bits3(geometry.linear_up),
            [0x3f1f_4000, 0x3f45_1800, 0xbc1c_0008]
        );
        assert_eq!(
            bits3(geometry.linear_at),
            [0x3e64_bffe, 0xbe37_c000, 0x3f74_c000]
        );
        assert_eq!(
            geometry.packed_linear_axes.map(bits4),
            [
                [0x3f3c_1800, 0xbf18_8000, 0xbe92_9fff, 0x3f74_c000],
                [0x3f1f_4000, 0x3f45_1800, 0xbc1c_0008, 0x3f1f_4000],
                [0x3e64_bffe, 0xbe37_c000, 0x3f74_c000, 0xbf18_8000],
            ]
        );

        let [x, y, z, w] = compose_drive_quaternion_host(body_b.quaternion, frames.quaternion_b);
        let textbook = [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y + w * z),
                2.0 * (x * z - w * y),
            ],
            [
                2.0 * (x * y - w * z),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z + w * x),
            ],
            [
                2.0 * (x * z + w * y),
                2.0 * (y * z - w * x),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ];
        assert_eq!(
            textbook.map(bits3),
            [
                [0x3f3c_1800, 0xbf18_8000, 0xbe92_a000],
                [0x3f1f_4000, 0x3f45_1800, 0xbc1c_0000],
                [0x3e64_c000, 0xbe37_c000, 0x3f74_c000],
            ]
        );
        assert_ne!(
            [
                geometry.linear_right,
                geometry.linear_up,
                geometry.linear_at,
            ]
            .map(bits3),
            textbook.map(bits3)
        );
    }

    #[test]
    fn geometry_attachment_offsets_keep_fmas_and_ignore_position_w() {
        let (body_a, body_b, frames) = geometry_fixture();
        let geometry = build_drive_geometry_host(body_a, body_b, frames);
        assert_eq!(
            bits3(geometry.attachment_offset_a),
            [0xc63f_9e64, 0x469c_944a, 0xc64b_b4a7]
        );
        assert_eq!(
            bits3(geometry.attachment_offset_b),
            [0xc153_d70a, 0xc08a_3d70, 0x3e4c_ccb8]
        );

        let defused_a = std::array::from_fn(|lane| {
            let first = body_a.right[lane] * frames.position_a[0];
            let second = body_a.up[lane] * frames.position_a[1] + first;
            body_a.at[lane] * frames.position_a[2] + second
        });
        assert_eq!(bits3(defused_a), [0xc63f_9e63, 0x469c_9449, 0xc64b_b4a7]);

        let mut changed_w = frames;
        changed_w.position_a[3] = f32::NEG_INFINITY;
        let changed = build_drive_geometry_host(body_a, body_b, changed_w);
        assert_eq!(
            bits3(changed.attachment_offset_a),
            bits3(geometry.attachment_offset_a)
        );
    }

    #[test]
    fn point_prediction_matches_build_specific_fused_oracle() {
        let (body_a, body_b, arm_a, arm_b, time_step, _, _) = prediction_fixture();
        let prediction = predict_drive_points_host(body_a, body_b, arm_a, arm_b, time_step);
        assert_eq!(
            bits3(prediction.position_error),
            [0xc072_ab37, 0x406c_c155, 0xc018_75f6]
        );
        assert_eq!(
            bits3(prediction.velocity_delta_h),
            [0xbd44_74cd, 0x3d1f_1ad6, 0xbde7_f942]
        );
        assert_eq!(
            bits3(prediction.acceleration_delta_h2),
            [0xbbb2_7492, 0x3a9d_4b81, 0xba5c_ecd4]
        );
    }

    #[test]
    fn point_prediction_preserves_child_a_parent_b_signs_and_force_torque_roles() {
        let (body_a, body_b, arm_a, arm_b, time_step, _, _) = prediction_fixture();
        let swapped = predict_drive_points_host(body_b, body_a, arm_b, arm_a, time_step);
        assert_eq!(
            bits3(swapped.position_error),
            [0x4072_ab37, 0xc06c_c155, 0x4018_75f6]
        );
        assert_eq!(
            bits3(swapped.velocity_delta_h),
            [0x3d44_74cd, 0xbd1f_1ad6, 0x3de7_f942]
        );
        assert_eq!(
            bits3(swapped.acceleration_delta_h2),
            [0x3bb2_7492, 0xba9d_4b81, 0x3a5c_ecd4]
        );

        let force_torque_a = SimulationDrivePredictionBodyInput {
            force: body_a.torque,
            torque: body_a.force,
            ..body_a
        };
        let force_torque_b = SimulationDrivePredictionBodyInput {
            force: body_b.torque,
            torque: body_b.force,
            ..body_b
        };
        let role_reversed =
            predict_drive_points_host(force_torque_a, force_torque_b, arm_a, arm_b, time_step);
        assert_eq!(
            bits3(role_reversed.acceleration_delta_h2),
            [0x3b03_ad08, 0xbbb5_d1c4, 0x3bb8_3028]
        );
    }

    #[test]
    fn point_prediction_scales_acceleration_by_h_then_h() {
        let zero = SimulationDrivePredictionBodyInput {
            center_of_mass: [0.0; 3],
            velocity: [0.0; 3],
            omega: [0.0; 3],
            force: [0.0; 3],
            torque: [0.0; 3],
        };
        let acceleration = f32::from_bits(0xc112_13c4);
        let time_step = f32::from_bits(0x3ee4_1eb1);
        let body_b = SimulationDrivePredictionBodyInput {
            force: [acceleration, 0.0, 0.0],
            ..zero
        };
        let prediction = predict_drive_points_host(zero, body_b, [0.0; 3], [0.0; 3], time_step);
        let grouped = acceleration * (time_step * time_step);
        assert_eq!(prediction.acceleration_delta_h2[0].to_bits(), 0xbfe7_fc04);
        assert_eq!(grouped.to_bits(), 0xbfe7_fc03);
    }

    #[test]
    fn point_prediction_cross_distinguishes_both_fused_operations() {
        let source = [
            f32::from_bits(0x3adb_3966),
            f32::from_bits(0x4339_5b47),
            f32::from_bits(0xc37e_dd3c),
        ];
        let arm = [
            f32::from_bits(0x41e4_12d5),
            f32::from_bits(0xc485_3ee2),
            f32::from_bits(0x4bc1_6473),
        ];
        let base = [
            f32::from_bits(0x3c93_ebd8),
            f32::from_bits(0xb433_aba1),
            f32::from_bits(0xc4fd_259e),
        ];
        let source_yzx = [source[1], source[2], source[0]];
        let source_zxy = [source[2], source[0], source[1]];
        let arm_yzx = [arm[1], arm[2], arm[0]];
        let arm_zxy = [arm[2], arm[0], arm[1]];
        let fused_first: [f32; 3] =
            std::array::from_fn(|lane| (-arm_yzx[lane]).mul_add(source_zxy[lane], base[lane]));
        let n_defused_first: [f32; 3] =
            std::array::from_fn(|lane| base[lane] - arm_yzx[lane] * source_zxy[lane]);
        let n_defused: [f32; 3] = std::array::from_fn(|lane| {
            arm_zxy[lane].mul_add(source_yzx[lane], n_defused_first[lane])
        });
        let m_defused: [f32; 3] =
            std::array::from_fn(|lane| arm_zxy[lane] * source_yzx[lane] + fused_first[lane]);
        assert_eq!(
            bits3(drive_prediction_cross_add_host(source, arm, base)),
            [0x4f8c_0479, 0xc741_fe44, 0xc5e4_7a98]
        );
        assert_eq!(bits3(n_defused), [0x4f8c_0479, 0xc741_fe44, 0xc5e4_7a99]);
        assert_eq!(bits3(m_defused), [0x4f8c_0478, 0xc741_fe44, 0xc5e4_7a98]);
    }

    #[test]
    fn linear_bias_uses_post_hard_error_and_native_fma_projection_graph() {
        let (body_a, body_b, arm_a, arm_b, time_step, coefficients, linear_axes) =
            prediction_fixture();
        let prediction = predict_drive_points_host(body_a, body_b, arm_a, arm_b, time_step);
        let result = build_drive_linear_bias_host(
            prediction,
            prediction.position_error,
            coefficients,
            linear_axes,
        );
        assert_eq!(bits3(result.bias), [0x3fd3_4260, 0xbfce_ca62, 0x3f81_e1d9]);
        assert_eq!(
            bits3(result.projection),
            [0x4016_0a3d, 0xbf21_77c5, 0x3e50_eebf]
        );

        let limited_error = [1.0, -2.0, 0.5];
        let limited = build_drive_linear_bias_host(
            prediction,
            limited_error,
            SimulationDriveCoefficients {
                dynamics: 0.0,
                spring_factor: 1.0,
                damping_factor: 0.0,
            },
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        );
        assert_eq!(bits3(limited.bias), bits3(limited_error));
        assert_eq!(bits3(limited.projection), bits3(limited_error));
    }

    #[test]
    fn angular_axes_preserve_nonfallback_xyz_w_and_error_graph() {
        let (rqd, packed_linear_axes, seeds) = angular_fixture();
        let result = build_drive_angular_host(
            &rqd,
            packed_linear_axes,
            zero_angular_body(),
            zero_angular_body(),
            no_drive_angular_params(),
            0.125,
            seeds,
            drive_effective_mass_dot3_host,
        );
        assert!(!result.used_global_fallback);
        assert_eq!(
            bits4(result.normalization_denominator),
            [0x3f70_0000, 0x3f40_0000, 0x3ee0_0000, 0x3f80_0000]
        );
        assert_eq!(
            result.angular_axes.map(bits4),
            [
                [0x3e84_32a5, 0xbf04_32a5, 0x3f46_4bf8, 0x3e84_32a5],
                [0xbf38_c088, 0x3edd_b3d7, 0x3e13_cd3a, 0xbf38_c088],
                [0x3fa9_53fd, 0xbec1_848f, 0xbf11_236b, 0x3fa9_53fd],
            ]
        );
        assert_eq!(
            bits3(result.angular_error),
            [0x3f04_32a5, 0xbf93_cd3a, 0x4011_236b]
        );
    }

    #[test]
    fn angular_fallback_is_global_for_each_clamped_q_lane_and_keeps_packed_w() {
        let (base_rqd, packed_linear_axes, mut seeds) = angular_fixture();
        seeds.axis_reciprocal_sqrt = [0.5; 4];
        let expected_axes = [
            [0x3dde_6800, 0x3e5e_6800, 0x3ede_6800, 0xbf26_ce00],
            [0xbea6_ce00, 0x3f0b_0100, 0xbf42_9b00, 0x3d5e_6800],
            [0x3f26_ce00, 0xbede_6800, 0x3e5e_6800, 0xbdde_6800],
        ];
        for (axis, q, expected_error) in [
            (0, 1.0, [0x3fde_6800, 0x3ede_6800, 0x3ede_6800]),
            (1, -1.0, [0x3ede_6800, 0xbfde_6800, 0x3ede_6800]),
            (2, 1.25, [0x3ede_6800, 0x3ede_6800, 0x400b_0100]),
        ] {
            let mut rqd = base_rqd;
            for row in &mut rqd.rows[..3] {
                row[3] = 0.25;
            }
            rqd.rows[axis][3] = q;
            let result = build_drive_angular_host(
                &rqd,
                packed_linear_axes,
                zero_angular_body(),
                zero_angular_body(),
                no_drive_angular_params(),
                0.125,
                seeds,
                drive_effective_mass_dot3_host,
            );
            assert!(result.used_global_fallback);
            assert_eq!(result.angular_axes.map(bits4), expected_axes);
            assert_eq!(bits3(result.angular_error), expected_error);
            assert_eq!(bits4(result.normalization_denominator), [0x3f80_0000; 4]);
        }

        let mut below_rqd = base_rqd;
        below_rqd.rows[0][3] = f32::from_bits(0x3f7f_ffff);
        below_rqd.rows[1][3] = 0.0;
        below_rqd.rows[2][3] = 0.0;
        let below = build_drive_angular_host(
            &below_rqd,
            packed_linear_axes,
            zero_angular_body(),
            zero_angular_body(),
            no_drive_angular_params(),
            0.125,
            seeds,
            drive_effective_mass_dot3_host,
        );
        assert!(!below.used_global_fallback);
        assert_eq!(below.normalization_denominator[0].to_bits(), 0x3400_0000);
    }

    #[test]
    fn angular_prediction_preserves_dot_order_body_signs_and_sequential_h() {
        let (mut rqd, _, mut seeds) = angular_fixture();
        rqd.rows[0][3] = 1.0;
        rqd.rows[1][3] = 0.0;
        rqd.rows[2][3] = 0.0;
        seeds.axis_reciprocal_sqrt = [1.0; 4];
        let identity = [
            [1.0, 0.0, 0.0, -0.25],
            [0.0, 1.0, 0.0, 0.375],
            [0.0, 0.0, 1.0, -0.5],
        ];
        let body_a = SimulationDriveAngularBodyInput {
            omega: [f32::from_bits(0xbd02_3e22), -2.0, 3.0],
            torque: [0.0; 3],
        };
        let body_b = SimulationDriveAngularBodyInput {
            omega: [f32::from_bits(0x3bba_74b3), 5.0, -7.0],
            torque: [0.0; 3],
        };
        let omega_h = f32::from_bits(0x3f7f_875d);
        let mut calls = Vec::new();
        let omega_result = build_drive_angular_host(
            &rqd,
            identity,
            body_a,
            body_b,
            no_drive_angular_params(),
            omega_h,
            seeds,
            |left, right| {
                calls.push((bits3(left), bits3(right)));
                drive_effective_mass_dot3_host(left, right)
            },
        );
        assert_eq!(calls.len(), 6);
        assert_eq!(
            bits3(omega_result.omega_projection),
            [0x3d19_445c, 0x40df_9671, 0xc11f_b49a]
        );
        assert_eq!(calls[0].0, bits3([1.0, 0.0, 0.0]));
        assert_eq!(calls[1].0, bits3([1.0, 0.0, 0.0]));
        assert_eq!(calls[2].0, bits3([0.0, 1.0, 0.0]));
        assert_eq!(calls[3].0, bits3([0.0, 1.0, 0.0]));
        assert_eq!(calls[4].0, bits3([0.0, 0.0, 1.0]));
        assert_eq!(calls[5].0, bits3([0.0, 0.0, 1.0]));
        for call in [0, 2, 4] {
            assert_eq!(calls[call].1, [0x3d19_445c, 0x40df_9671, 0xc11f_b49a]);
        }
        for call in [1, 3, 5] {
            assert_eq!(calls[call].1, [0, 0, 0]);
        }

        let alpha_a = f32::from_bits(0x42d1_33c1);
        let alpha_b = f32::from_bits(0x3f1e_a03e);
        let alpha_h = f32::from_bits(0x3fae_3a20);
        let alpha_result = build_drive_angular_host(
            &rqd,
            identity,
            SimulationDriveAngularBodyInput {
                omega: [0.0; 3],
                torque: [alpha_a, 1.0, -2.0],
            },
            SimulationDriveAngularBodyInput {
                omega: [0.0; 3],
                torque: [alpha_b, 4.0, 5.0],
            },
            no_drive_angular_params(),
            alpha_h,
            seeds,
            drive_effective_mass_dot3_host,
        );
        let grouped = (alpha_b - alpha_a) * (alpha_h * alpha_h);
        let scale_first = (alpha_b * alpha_h - alpha_a * alpha_h) * alpha_h;
        assert_eq!(
            bits3(alpha_result.acceleration_projection),
            [0xc340_a630, 0x40b1_dc99, 0x414f_815d]
        );
        assert_eq!(grouped.to_bits(), 0xc340_a62f);
        assert_eq!(scale_first.to_bits(), 0xc340_a62e);
    }

    #[test]
    fn angular_soft_coefficients_keep_three_independent_estimate_seeds() {
        let (rqd, packed_linear_axes, mut seeds) = angular_fixture();
        seeds.coefficients = SimulationDriveReciprocalSeeds {
            dynamics: f32::from_bits(0x3f60_0000),
            spring: f32::from_bits(0x3f70_0000),
            damping: f32::from_bits(0x3f80_0000),
        };
        let result = build_drive_angular_host(
            &rqd,
            packed_linear_axes,
            zero_angular_body(),
            zero_angular_body(),
            SimulationDriveDynamicsParams {
                spring: 0.75,
                damping: 0.5,
                strength: 0.25,
                raw_type: SOFT_DRIVE,
            },
            0.125,
            seeds,
            drive_effective_mass_dot3_host,
        );
        assert_eq!(
            [
                result.coefficients.dynamics.to_bits(),
                result.coefficients.spring_factor.to_bits(),
                result.coefficients.damping_factor.to_bits(),
            ],
            [0x3d8d_7f10, 0x3c32_bc0b, 0x3d6e_4e35]
        );
        assert_eq!(result.strength_h2.to_bits(), 0x3b80_0000);

        let sequential = build_drive_angular_host(
            &rqd,
            packed_linear_axes,
            zero_angular_body(),
            zero_angular_body(),
            SimulationDriveDynamicsParams {
                strength: f32::from_bits(0xc112_13c4),
                ..no_drive_angular_params()
            },
            f32::from_bits(0x3ee4_1eb1),
            seeds,
            drive_effective_mass_dot3_host,
        );
        assert_eq!(sequential.strength_h2.to_bits(), 0xbfe7_fc04);
        let grouped = f32::from_bits(0xc112_13c4)
            * (f32::from_bits(0x3ee4_1eb1) * f32::from_bits(0x3ee4_1eb1));
        assert_eq!(grouped.to_bits(), 0xbfe7_fc03);
    }

    #[test]
    fn angular_hard_adds_seventh_error_dot_and_keeps_strict_equality() {
        let (mut rqd, packed_linear_axes, mut seeds) = angular_fixture();
        rqd.rows[0][3] = 1.0;
        rqd.rows[1][3] = 0.0;
        rqd.rows[2][3] = 0.0;
        seeds.axis_reciprocal_sqrt = [1.0; 4];
        seeds.hard_reciprocal_sqrt = 0.5;
        let mut calls = Vec::new();
        let result = build_drive_angular_host(
            &rqd,
            packed_linear_axes,
            zero_angular_body(),
            zero_angular_body(),
            SimulationDriveDynamicsParams {
                spring: 4.0,
                damping: 0.5,
                strength: 1.0,
                raw_type: HARD_DRIVE,
            },
            0.5,
            seeds,
            |left, right| {
                calls.push((bits3(left), bits3(right)));
                drive_effective_mass_dot3_host(left, right)
            },
        );
        assert_eq!(calls.len(), 7);
        assert_eq!(calls[6].0, bits3(result.angular_error));
        assert_eq!(calls[6].1, bits3(result.angular_error));
        assert!(!result.hard_outside_limit);
    }

    #[test]
    fn angular_hard_outside_gate_feeds_the_bias_graph() {
        let (mut rqd, _, mut seeds) = angular_fixture();
        rqd.rows[0][3] = 1.0;
        rqd.rows[1][3] = 0.0;
        rqd.rows[2][3] = 0.0;
        seeds.axis_reciprocal_sqrt = [1.0; 4];
        seeds.hard_reciprocal_sqrt = 0.5;
        let result = build_drive_angular_host(
            &rqd,
            [
                [1.0, 0.0, 0.0, 0.25],
                [0.0, 1.0, 0.0, -0.5],
                [0.0, 0.0, 1.0, 0.75],
            ],
            zero_angular_body(),
            zero_angular_body(),
            SimulationDriveDynamicsParams {
                spring: 1.0,
                damping: 0.0,
                strength: 0.0,
                raw_type: HARD_DRIVE,
            },
            0.5,
            seeds,
            drive_effective_mass_dot3_host,
        );
        assert!(result.hard_outside_limit);
        assert_eq!(bits3(result.angular_error), [0x4000_0000, 0, 0]);
        assert_eq!(bits3(result.bias_projection), [0x3f00_0000, 0, 0]);
    }

    #[test]
    fn angular_unknown_types_stay_on_the_six_dot_non_soft_route() {
        let (rqd, packed_linear_axes, mut seeds) = angular_fixture();
        let baseline = build_drive_angular_host(
            &rqd,
            packed_linear_axes,
            zero_angular_body(),
            zero_angular_body(),
            no_drive_angular_params(),
            0.125,
            seeds,
            drive_effective_mass_dot3_host,
        );
        seeds.hard_reciprocal_sqrt = f32::NAN;
        for raw_type in [3, 0x8000_0001] {
            let mut calls = 0;
            let unknown = build_drive_angular_host(
                &rqd,
                packed_linear_axes,
                zero_angular_body(),
                zero_angular_body(),
                SimulationDriveDynamicsParams {
                    raw_type,
                    ..no_drive_angular_params()
                },
                0.125,
                seeds,
                |left, right| {
                    calls += 1;
                    drive_effective_mass_dot3_host(left, right)
                },
            );
            assert_eq!(calls, 6);
            assert!(!unknown.hard_outside_limit);
            assert_eq!(unknown, baseline);
        }
    }

    #[test]
    fn angular_bias_distinguishes_both_fused_operations() {
        let error = f32::from_bits(0x3c96_536a);
        let omega = f32::from_bits(0xc2c7_a79b);
        let alpha = f32::from_bits(0xb92b_9fa6);
        let spring = f32::from_bits(0x4237_4389);
        let damping = f32::from_bits(0x3d40_9220);
        let dynamics = f32::from_bits(0x46c3_5e87);
        let fused = drive_angular_bias_host(
            [error, 0.0, 0.0],
            [omega, 0.0, 0.0],
            [alpha, 0.0, 0.0],
            SimulationDriveCoefficients {
                dynamics,
                spring_factor: spring,
                damping_factor: damping,
            },
        );
        let damped = omega * damping;
        let inner_defused = error * spring + damped;
        let inner_wrong = alpha.mul_add(dynamics, inner_defused);
        let outer_defused = alpha * dynamics + error.mul_add(spring, damped);
        assert_eq!(fused[0].to_bits(), 0xc0fe_4253);
        assert_eq!(inner_wrong.to_bits(), 0xc0fe_4254);
        assert_eq!(outer_defused.to_bits(), 0xc0fe_4254);
    }

    #[test]
    fn linear_bias_distinguishes_both_ordered_fmas() {
        let position_error = f32::from_bits(0x3c96_536a);
        let velocity_delta = f32::from_bits(0xc2c7_a79b);
        let damping = f32::from_bits(0x3d40_9220);
        let spring = f32::from_bits(0x4237_4389);
        let acceleration_delta = f32::from_bits(0xb92b_9fa6);
        let dynamics = f32::from_bits(0x46c3_5e87);
        let result = build_drive_linear_bias_host(
            SimulationDrivePointPredictionHost {
                position_error: [0.0; 3],
                velocity_delta_h: [velocity_delta, 0.0, 0.0],
                acceleration_delta_h2: [acceleration_delta, 0.0, 0.0],
            },
            [position_error, 0.0, 0.0],
            SimulationDriveCoefficients {
                dynamics,
                spring_factor: spring,
                damping_factor: damping,
            },
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        );
        let velocity_term = velocity_delta * damping;
        let inner_defused = position_error * spring + velocity_term;
        let inner_wrong = acceleration_delta.mul_add(dynamics, inner_defused);
        let fused_inner = position_error.mul_add(spring, velocity_term);
        let outer_wrong = acceleration_delta * dynamics + fused_inner;
        assert_eq!(result.bias[0].to_bits(), 0xc0fe_4253);
        assert_eq!(inner_wrong.to_bits(), 0xc0fe_4254);
        assert_eq!(outer_wrong.to_bits(), 0xc0fe_4254);
    }

    #[test]
    fn effective_mass_matches_independent_noncommuting_tensor_oracle() {
        let (body_a, body_b, linear_axes, angular_axes) = effective_mass_fixture();
        let result = build_drive_effective_mass_host(body_a, body_b, linear_axes, angular_axes);

        assert_eq!(
            result.inverse_inertia_a.map(bits3),
            [
                [0x4000_0000, 0x3f00_0000, 0xbe80_0000],
                [0x3f00_0000, 0x4040_0000, 0x3f40_0000],
                [0xbe80_0000, 0x3f40_0000, 0x4080_0000],
            ]
        );
        assert_eq!(result.inverse_mass_a.to_bits(), 0x3e00_0000);
        assert_eq!(
            result.inverse_inertia_b.map(bits3),
            [
                [0x3fc0_0000, 0xbec0_0000, 0x3f20_0000],
                [0xbec0_0000, 0x3fa0_0000, 0xbf00_0000],
                [0x3f20_0000, 0xbf00_0000, 0x4020_0000],
            ]
        );
        assert_eq!(result.inverse_mass_b.to_bits(), 0x3e80_0000);
        assert_eq!(
            bits3(result.linear_denominator),
            [0x42b5_2800, 0x420c_d000, 0x42ac_0c00]
        );
        assert_eq!(
            bits3(result.angular_denominator),
            [0x41e9_4000, 0x4192_8000, 0x4182_1000]
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one uninterrupted golden freezes every related packed final-store value"
    )]
    fn finalizer_matches_full_transpose_offset_limit_oracle() {
        let (body_a, body_b, linear_axes, angular_axes) = effective_mass_fixture();
        let effective_mass =
            build_drive_effective_mass_host(body_a, body_b, linear_axes, angular_axes);
        let packed_linear_axes = [
            [1.0, 2.0, -1.0, 8.0],
            [0.5, -2.0, 3.0, -4.0],
            [-1.5, 0.25, 2.0, 6.0],
        ];
        let packed_angular_axes = [
            [0.25, 1.0, -2.0, 0.375],
            [-1.0, 0.5, 1.5, -0.625],
            [2.0, -0.75, 0.125, 1.25],
        ];
        let result = finalize_drive_jacobian_host(
            SimulationDriveFinalizationInput {
                effective_mass,
                packed_linear_axes,
                packed_angular_axes,
                linear_bias_projection: [0.75, -1.25, 2.5],
                angular_bias_projection: [-0.5, 1.75, -2.25],
                linear_strength_h2: 0.125,
                angular_strength_h2: 0.375,
                linear_dynamics: 0.625,
                angular_dynamics: -0.75,
            },
            SimulationDriveFinalReciprocalSeeds {
                linear: [
                    f32::from_bits(0x3c34_e1ed),
                    f32::from_bits(0x3ce8_b4e5),
                    f32::from_bits(0x3c3e_75b0),
                ],
                angular: [
                    f32::from_bits(0x3d0c_7c05),
                    f32::from_bits(0x3d5f_ac1f),
                    f32::from_bits(0x3d7b_f0bf),
                ],
            },
        );

        assert_eq!(
            result.packed_linear_axes.map(bits4),
            packed_linear_axes.map(bits4)
        );
        assert_eq!(
            result.packed_angular_axes.map(bits4),
            packed_angular_axes.map(bits4)
        );
        assert_eq!(
            result.packed_inverse_inertia_a.map(bits4),
            [
                [0x4000_0000, 0x3f00_0000, 0xbe80_0000, 0x3e00_0000],
                [0x3f00_0000, 0x4040_0000, 0x3f40_0000, 0x3e00_0000],
                [0xbe80_0000, 0x3f40_0000, 0x4080_0000, 0x3e00_0000],
            ]
        );
        assert_eq!(
            result.packed_inverse_inertia_b.map(bits4),
            [
                [0x3fc0_0000, 0xbec0_0000, 0x3f20_0000, 0x3e80_0000],
                [0xbec0_0000, 0x3fa0_0000, 0xbf00_0000, 0x3e80_0000],
                [0x3f20_0000, 0xbf00_0000, 0x4020_0000, 0x3e80_0000],
            ]
        );
        assert_eq!(
            bits3(result.inverse_linear_denominator),
            [0x3c34_e1ed, 0x3ce8_b4e5, 0x3c3e_75b0]
        );
        assert_eq!(
            bits3(result.inverse_angular_denominator),
            [0x3d0c_7c05, 0x3d5f_ac1f, 0x3d7b_f0bf]
        );
        assert_eq!(
            bits3(result.position_offset),
            [0x3c07_a972, 0xbd11_710f, 0x3cee_131c]
        );
        assert_eq!(
            bits3(result.position_limit),
            [0x3ab4_e1ed, 0x3b68_b4e5, 0x3abe_75b0]
        );
        assert_eq!(
            bits3(result.angular_offset),
            [0xbc8c_7c05, 0x3dc3_b69b, 0xbe0d_b76b]
        );
        assert_eq!(
            bits3(result.angular_limit),
            [0x3c52_ba08, 0x3ca7_c117, 0x3cbc_f48f]
        );
        assert_eq!(
            result.linear_transpose.map(bits3),
            [
                [0x3c34_e1ed, 0x3c68_b4e5, 0xbc8e_d844],
                [0x3cb4_e1ed, 0xbd68_b4e5, 0x3b3e_75b0],
                [0xbc34_e1ed, 0x3dae_87ac, 0x3cbe_75b0],
            ]
        );
        assert_eq!(
            result.angular_transpose.map(bits3),
            [
                [0x3c0c_7c05, 0xbd5f_ac1f, 0x3dfb_f0bf],
                [0x3d0c_7c05, 0x3cdf_ac1f, 0xbd3c_f48f],
                [0xbd8c_7c05, 0x3da7_c117, 0x3bfb_f0bf],
            ]
        );
        assert_eq!(result.angular_transpose_right_pad.to_bits(), 0x3c0c_7c05);
        assert_eq!(bits3(result.position_impulse), [0; 3]);
        assert_eq!(result.linear_dynamics.to_bits(), 0x3f20_0000);
        assert_eq!(bits3(result.angular_impulse), [0; 3]);
        assert_eq!(result.angular_dynamics.to_bits(), 0xbf40_0000);
    }

    #[test]
    fn finalizer_keeps_unconditional_zero_denominator_reciprocals() {
        let result = finalize_drive_jacobian_host(
            SimulationDriveFinalizationInput {
                effective_mass: SimulationDriveEffectiveMassHost {
                    inverse_inertia_a: [[0.0; 3]; 3],
                    inverse_mass_a: 0.0,
                    inverse_inertia_b: [[0.0; 3]; 3],
                    inverse_mass_b: 0.0,
                    linear_denominator: [0.0; 3],
                    angular_denominator: [0.0; 3],
                },
                packed_linear_axes: [[0.0; 4]; 3],
                packed_angular_axes: [[0.0; 4]; 3],
                linear_bias_projection: [0.0; 3],
                angular_bias_projection: [0.0; 3],
                linear_strength_h2: 0.0,
                angular_strength_h2: 0.0,
                linear_dynamics: 0.0,
                angular_dynamics: 0.0,
            },
            SimulationDriveFinalReciprocalSeeds {
                linear: [1.0; 3],
                angular: [1.0; 3],
            },
        );
        assert_eq!(result.packed_inverse_inertia_a.map(bits4), [[0; 4]; 3]);
        assert_eq!(result.packed_inverse_inertia_b.map(bits4), [[0; 4]; 3]);
        assert_eq!(bits3(result.inverse_linear_denominator), [0x4080_0000; 3]);
        assert_eq!(bits3(result.inverse_angular_denominator), [0x4080_0000; 3]);
    }

    #[test]
    fn finalizer_keeps_distinct_linear_angular_denominator_and_seed_roles() {
        let result = finalize_drive_jacobian_host(
            SimulationDriveFinalizationInput {
                effective_mass: SimulationDriveEffectiveMassHost {
                    inverse_inertia_a: [[0.0; 3]; 3],
                    inverse_mass_a: 0.0,
                    inverse_inertia_b: [[0.0; 3]; 3],
                    inverse_mass_b: 0.0,
                    linear_denominator: [2.0, 3.0, 5.0],
                    angular_denominator: [7.0, 11.0, 13.0],
                },
                packed_linear_axes: [[0.0; 4]; 3],
                packed_angular_axes: [[0.0; 4]; 3],
                linear_bias_projection: [0.0; 3],
                angular_bias_projection: [0.0; 3],
                linear_strength_h2: 0.0,
                angular_strength_h2: 0.0,
                linear_dynamics: 0.0,
                angular_dynamics: 0.0,
            },
            SimulationDriveFinalReciprocalSeeds {
                linear: [0.4, 0.3, 0.2],
                angular: [0.1, 0.08, 0.07],
            },
        );
        assert_eq!(
            bits3(result.inverse_linear_denominator),
            [0x3eff_9724, 0x3eaa_a64c, 0x3e4c_cccd]
        );
        assert_eq!(
            bits3(result.inverse_angular_denominator),
            [0x3e11_19ce, 0x3dba_24a9, 0x3d9d_8733]
        );
    }

    #[test]
    fn finalizer_reciprocals_retain_the_fused_newton_graph() {
        let denominator = f32::from_bits(0x72bb_3b18);
        let seed = f32::from_bits(0x0c2f_038f);
        let (linear, angular) = refine_drive_final_reciprocals(
            [denominator, 1.0, 1.0],
            [1.0, denominator, 1.0],
            [seed, 1.0, 1.0],
            [1.0, seed, 1.0],
        );
        let residual_0 = 1.0 - seed * denominator;
        let refined_0 = seed * residual_0 + seed;
        let residual_1 = 1.0 - refined_0 * denominator;
        let defused = refined_0 * residual_1 + refined_0;
        assert_eq!(linear[0].to_bits(), 0x0c2f_038f);
        assert_eq!(angular[1].to_bits(), 0x0c2f_038f);
        assert_eq!(defused.to_bits(), 0x0c2f_0390);
    }

    #[test]
    fn complete_composer_does_not_consume_untaken_hard_seeds() {
        let body = SimulationDriveBuildBodyInput {
            state: 0,
            quaternion: [0.0, 0.0, 0.0, 1.0],
            center_of_mass: [0.0; 3],
            velocity: [0.0; 3],
            omega: [0.0; 3],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            at: [0.0, 0.0, 1.0],
            force: [0.0; 3],
            torque: [0.0; 3],
            inverse_inertia_full: [0.0; 3],
            inverse_mass: 0.0,
            inverse_inertia_split: [0.0; 3],
        };
        let params = SimulationDriveDynamicsParams {
            spring: 0.0,
            damping: 0.0,
            strength: 0.0,
            raw_type: 0,
        };
        let input = SimulationDriveBuildInput {
            body_a: body,
            body_b: body,
            frames: SimulationDriveFrames {
                quaternion_a: [0.0, 0.0, 0.0, 1.0],
                position_a: [0.0; 4],
                quaternion_b: [0.0, 0.0, 0.0, 1.0],
                position_b: [0.0; 4],
            },
            dynamics: crate::simulation_drive_pool::SimulationDriveDynamics {
                linear: params,
                angular: params,
            },
            time_step: 1.0,
            spy: 0x1357_9bdf,
            bindings: SimulationDriveRecordBindings {
                solver_delta_a: 1,
                solver_delta_b: 2,
                drive: 3,
            },
        };
        let seeds = SimulationDriveBuildEstimateSeeds {
            linear_coefficients: SimulationDriveReciprocalSeeds {
                dynamics: f32::NAN,
                spring: 1.0,
                damping: f32::NAN,
            },
            linear_hard_reciprocal_sqrt: 1.0,
            angular: SimulationDriveAngularEstimateSeeds {
                axis_reciprocal_sqrt: [1.0; 4],
                coefficients: SimulationDriveReciprocalSeeds {
                    dynamics: f32::NAN,
                    spring: 1.0,
                    damping: f32::NAN,
                },
                hard_reciprocal_sqrt: 1.0,
            },
            final_reciprocals: SimulationDriveFinalReciprocalSeeds {
                linear: [1.0; 3],
                angular: [1.0; 3],
            },
        };
        let baseline = compose_drive_jacobian_with_explicit_seeds(&input, seeds);
        let poisoned_hard = compose_drive_jacobian_with_explicit_seeds(
            &input,
            SimulationDriveBuildEstimateSeeds {
                linear_hard_reciprocal_sqrt: f32::from_bits(0x7fc1_2345),
                angular: SimulationDriveAngularEstimateSeeds {
                    hard_reciprocal_sqrt: f32::from_bits(0x7fc5_4321),
                    ..seeds.angular
                },
                ..seeds
            },
        );
        assert!(baseline == poisoned_hard);
        assert!(baseline.has_solver_delta_bindings());
        assert_eq!(baseline.spy, 0x1357_9bdf);
        assert_eq!(baseline.drive_slot, DriveSlotToken(3));
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one coherent external golden freezes every word of the complete record"
    )]
    fn complete_composer_matches_all_record_words_oracle() {
        let body_a = SimulationDriveBuildBodyInput {
            state: 4,
            quaternion: [0.125, -0.375, 0.25, 0.875],
            center_of_mass: [1.125, -2.25, 3.5],
            velocity: [0.31, -0.47, 0.83],
            omega: [-0.73, 0.29, 0.41],
            right: [
                f32::from_bits(0x3f18_0000),
                f32::from_bits(0x3eb0_0000),
                f32::from_bits(0x3f38_0000),
            ],
            up: [
                f32::from_bits(0xbf08_0000),
                f32::from_bits(0x3f58_0000),
                f32::from_bits(0x3d00_0004),
            ],
            at: [
                f32::from_bits(0xbf18_0000),
                f32::from_bits(0xbed0_0000),
                f32::from_bits(0x3f30_0000),
            ],
            force: [1.17, -0.63, 0.38],
            torque: [-0.52, 0.91, -0.27],
            inverse_inertia_full: [2.0, 0.5, -0.25],
            inverse_mass: 0.125,
            inverse_inertia_split: [4.0, 3.0, 0.75],
        };
        let body_b = SimulationDriveBuildBodyInput {
            state: 0x14,
            quaternion: [-0.25, 0.375, -0.5, 0.75],
            center_of_mass: [-1.75, 0.625, 2.125],
            velocity: [-0.26, 0.74, -0.59],
            omega: [0.67, -0.35, 0.22],
            right: [
                f32::from_bits(0x3e60_0003),
                f32::from_bits(0xbf6f_ffff),
                f32::from_bits(0xbe9f_fffe),
            ],
            up: [
                f32::from_bits(0x3f0f_ffff),
                f32::from_bits(0x3ec0_0001),
                f32::from_bits(0xbf3f_ffff),
            ],
            at: [
                f32::from_bits(0x3f4f_ffff),
                f32::from_bits(0),
                f32::from_bits(0x3f18_0000),
            ],
            force: [-0.88, 0.57, 1.09],
            torque: [0.43, -0.76, 0.61],
            inverse_inertia_full: [1.5, -0.375, 0.625],
            inverse_mass: 0.25,
            inverse_inertia_split: [2.5, 1.25, -0.5],
        };
        let input = SimulationDriveBuildInput {
            body_a,
            body_b,
            frames: SimulationDriveFrames {
                quaternion_a: [-0.5, 0.0625, 0.3125, 0.75],
                position_a: [0.37, -0.81, 0.46, f32::from_bits(0x7fc1_2345)],
                quaternion_b: [0.3125, -0.125, 0.25, 0.875],
                position_b: [-0.58, 0.33, -0.72, f32::NEG_INFINITY],
            },
            dynamics: crate::simulation_drive_pool::SimulationDriveDynamics {
                linear: SimulationDriveDynamicsParams {
                    spring: 0.75,
                    damping: 0.31,
                    strength: f32::from_bits(0x3d4c_cccd),
                    raw_type: HARD_DRIVE,
                },
                angular: SimulationDriveDynamicsParams {
                    spring: 0.65,
                    damping: 0.28,
                    strength: 0.37,
                    raw_type: SOFT_DRIVE,
                },
            },
            time_step: f32::from_bits(0x3c23_d70a),
            spy: 0xddee_ff00,
            bindings: SimulationDriveRecordBindings {
                solver_delta_a: 0x1122_3344,
                solver_delta_b: 0x5566_7788,
                drive: 0x99aa_bbcc,
            },
        };
        let result = compose_drive_jacobian_with_explicit_seeds(
            &input,
            SimulationDriveBuildEstimateSeeds {
                linear_coefficients: SimulationDriveReciprocalSeeds {
                    dynamics: f32::from_bits(0x3f7d_70a4),
                    spring: f32::from_bits(0x3f7d_70a4),
                    damping: f32::from_bits(0x3f7d_70a4),
                },
                linear_hard_reciprocal_sqrt: f32::from_bits(0x3e24_faea),
                angular: SimulationDriveAngularEstimateSeeds {
                    axis_reciprocal_sqrt: [
                        f32::from_bits(0x3f89_db6a),
                        f32::from_bits(0x3f9f_27dc),
                        f32::from_bits(0x3f8d_b8aa),
                        f32::from_bits(0x3f80_0000),
                    ],
                    coefficients: SimulationDriveReciprocalSeeds {
                        dynamics: f32::from_bits(0x3f7d_70a4),
                        spring: f32::from_bits(0x3f7d_70a5),
                        damping: f32::from_bits(0x3f7d_70a3),
                    },
                    hard_reciprocal_sqrt: f32::from_bits(0x7fc1_2345),
                },
                final_reciprocals: SimulationDriveFinalReciprocalSeeds {
                    linear: [
                        f32::from_bits(0x3e9f_1e01),
                        f32::from_bits(0x3e43_c2b5),
                        f32::from_bits(0x3f03_e68f),
                    ],
                    angular: [
                        f32::from_bits(0x3e85_53e4),
                        f32::from_bits(0x3e98_a06d),
                        f32::from_bits(0x3dca_6a1e),
                    ],
                },
            },
        );

        assert_eq!(
            drive_record_bits(&result),
            [
                [0x3ec0_f5c4, 0xbf3e_3d70, 0x3f0e_8f5c, 0x1122_3344],
                [0xbf06_b852, 0x3f2a_e147, 0xbefc_ccce, 0x5566_7788],
                [0, 0, 0, 0x3f80_0000],
                [0, 0, 0, 0x3b3b_3968],
                [0x3e69_d1e7, 0x3df3_8dbe, 0x3deb_b886, 0xddee_ff00],
                [0x3e51_4ae6, 0xbd30_5048, 0xbc9c_fa14, 0x3e51_4ae6],
                [0xbe3d_92bf, 0x3e16_b730, 0xbdbd_597a, 0x35d0_8ee1],
                [0xbc75_ebe1, 0x3e79_c51c, 0x3e00_6695, 0x3580_4b2d],
                [0xbdb6_451d, 0xbaee_9559, 0x3efc_355a, 0x3721_9f1c],
                [0x3df9_2e62, 0xbdad_190f, 0x3e03_55ca, 0x3737_79f7],
                [0xbb8c_f8bc, 0x39a1_745f, 0xbc48_9954, 0x362c_e282],
                [0x37b1_dd19, 0x37bd_55a6, 0xb785_7849, 0x36f6_027f],
                [0x3f3c_1800, 0xbf18_8000, 0xbe92_9fff, 0x99aa_bbcc],
                [0x3f1f_4000, 0x3f45_1800, 0xbc1c_0008, 0x3f1f_4000],
                [0x3e64_bffe, 0xbe37_c000, 0x3f74_c000, 0xbf18_8000],
                [0x3f48_f68b, 0xbd6c_2255, 0x3eef_43a0, 0x3f48_f68b],
                [0xbe15_2178, 0x3f53_4317, 0xbe92_692a, 0xbe15_2178],
                [0xbdc6_0cd0, 0x3f21_ff35, 0x3f25_b2f8, 0xbdc6_0cd0],
                [0x4000_0000, 0x3f00_0000, 0xbe80_0000, 0x3e00_0000],
                [0x3f00_0000, 0x4040_0000, 0x3f40_0000, 0x3e00_0000],
                [0xbe80_0000, 0x3f40_0000, 0x4080_0000, 0x3e00_0000],
                [0x3fc0_0000, 0xbec0_0000, 0x3f20_0000, 0x3e80_0000],
                [0xbec0_0000, 0x3fa0_0000, 0xbf00_0000, 0x3e80_0000],
                [0x3f20_0000, 0xbf00_0000, 0x4020_0000, 0x3e80_0000],
            ]
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one test couples the complete literal backend trace and 96-word record"
    )]
    fn numeric_backend_is_lazy_site_tagged_and_preserves_raw_call_order() {
        use NumericSite::{Reciprocal as R, ReciprocalSqrt as S, Sum3 as D};
        use SimulationDriveDotSite as Dot;
        use SimulationDriveReciprocalSite as Recip;
        use SimulationDriveReciprocalSqrtSite as Rsqrt;

        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        let mut host = SimulationDriveHostNumericBackend;
        let host_record = compose_drive_jacobian_with_backend(&input, &mut host)
            .expect("ordinary-finite backend is infallible");
        let mut trace = TracingHostNumericBackend::default();
        let trace_record = compose_drive_jacobian_with_backend(&input, &mut trace)
            .expect("tracing host backend is infallible");

        assert_eq!(
            drive_record_bits(&trace_record),
            drive_record_bits(&host_record)
        );
        assert_eq!(
            drive_record_bits(&host_record),
            [
                [0x3f00_0000, 0xbe80_0000, 0x3f40_0000, 0xa1a2_a3a4],
                [0xbec0_0000, 0x3f20_0000, 0x3e80_0000, 0xb1b2_b3b4],
                [0, 0, 0, 0x3f80_0000],
                [0, 0, 0, 0x3db6_34d7],
                [0x3ead_f199, 0x3d99_cb1b, 0x3e2e_52d9, 0x1122_3344],
                [0x3ef3_96b3, 0x3d42_55c0, 0xbd0d_3b36, 0x3ef3_96b3],
                [0xbdc7_a83c, 0x3eaf_2750, 0x3d05_8645, 0x3c91_3489],
                [0xbbd2_abd9, 0x3ed5_5893, 0xbbf5_9ea0, 0x3c88_b48a],
                [0xbe02_1464, 0xbd6f_3bf1, 0x3eda_d4f0, 0x3c4e_c6b4],
                [0x3ce4_3a2c, 0x3d64_21e1, 0x3eaa_660c, 0x3c3b_01d9],
                [0x3b57_db31, 0x3cb9_b3ec, 0xbd36_c284, 0x3cb2_085c],
                [0xbb76_672f, 0x3ba1_b7df, 0x3a22_b918, 0x3c11_13bc],
                [0x3f66_0000, 0xbe84_0000, 0xbeab_ffff, 0xd1d2_d3d4],
                [0x3e58_0000, 0x3f76_0000, 0xbe28_0000, 0x3e58_0000],
                [0x3ebb_ffff, 0x3d90_0000, 0x3f6c_0000, 0xbe84_0000],
                [0x3f62_2e90, 0xbc43_9de4, 0x3d53_eb0d, 0x3f62_2e90],
                [0x3dc7_85fc, 0x3f5b_0abf, 0x3dea_391d, 0x3dc7_85fc],
                [0xbdba_e90c, 0xbca2_87de, 0x3f61_82ea, 0xbdba_e90c],
                [0x3f90_0000, 0x3d00_0000, 0xbc80_0000, 0x3f10_0000],
                [0x3d00_0000, 0x3fa8_0000, 0x3c00_0000, 0x3f10_0000],
                [0xbc80_0000, 0x3c00_0000, 0x3fd0_0000, 0x3f10_0000],
                [0x3fa0_0000, 0x3d80_0000, 0xbd00_0000, 0x3f20_0000],
                [0x3d80_0000, 0x3fb0_0000, 0x3c80_0000, 0x3f20_0000],
                [0xbd00_0000, 0x3c80_0000, 0x3fe0_0000, 0x3f20_0000],
            ]
        );
        assert_eq!(
            trace.sites(),
            [
                D(Dot::QuaternionA),
                D(Dot::QuaternionB),
                R(Recip::LinearNonSoftSpring),
                D(Dot::LinearHardLengthSquared),
                S(Rsqrt::LinearHardLength),
                D(Dot::LinearBiasUp),
                D(Dot::LinearBiasRight),
                D(Dot::LinearBiasAt),
                S(Rsqrt::AngularAxisNormalization),
                D(Dot::AngularRightOmega),
                D(Dot::AngularRightAcceleration),
                D(Dot::AngularUpOmega),
                D(Dot::AngularUpAcceleration),
                D(Dot::AngularAtOmega),
                D(Dot::AngularAtAcceleration),
                R(Recip::AngularSoftDynamics),
                R(Recip::AngularSoftSpring),
                R(Recip::AngularSoftDamping),
                D(Dot::BodyAAngularRight),
                D(Dot::BodyAAngularUp),
                D(Dot::BodyAAngularAt),
                D(Dot::BodyALinearRight),
                D(Dot::BodyALinearUp),
                D(Dot::BodyALinearAt),
                D(Dot::BodyBAngularRight),
                D(Dot::BodyBAngularUp),
                D(Dot::BodyBAngularAt),
                D(Dot::BodyBLinearRight),
                D(Dot::BodyBLinearUp),
                D(Dot::BodyBLinearAt),
                R(Recip::FinalLinear),
                R(Recip::FinalAngular),
            ]
        );
        assert_eq!(trace.calls, hard_soft_both_expected_calls());
        let angular_axis_input = trace
            .calls
            .iter()
            .find_map(|call| match *call {
                NumericCall::ReciprocalSqrt(Rsqrt::AngularAxisNormalization, input) => Some(input),
                _ => None,
            })
            .expect("one packed angular-axis estimate");
        assert_ne!(angular_axis_input[0], angular_axis_input[1]);
        assert_eq!(angular_axis_input[3], 1.0_f32.to_bits());
        for site in [Recip::FinalLinear, Recip::FinalAngular] {
            let input = trace
                .calls
                .iter()
                .find_map(|call| match *call {
                    NumericCall::Reciprocal(candidate, input) if candidate == site => Some(input),
                    _ => None,
                })
                .expect("unconditional packed final estimate");
            assert_eq!(input[3], input[0]);
        }
    }

    #[test]
    fn numeric_backend_unknown_types_and_inactive_body_skip_forbidden_sites() {
        use NumericSite::{Reciprocal as R, ReciprocalSqrt as S, Sum3 as D};
        use SimulationDriveDotSite as Dot;
        use SimulationDriveReciprocalSite as Recip;
        use SimulationDriveReciprocalSqrtSite as Rsqrt;

        let input = numeric_backend_input(3, 0x8000_0001, 0, 4);
        let mut trace = TracingHostNumericBackend::default();
        compose_drive_jacobian_with_backend(&input, &mut trace)
            .expect("tracing host backend is infallible");
        let sites = trace.sites();

        assert_eq!(
            sites,
            [
                D(Dot::QuaternionA),
                D(Dot::QuaternionB),
                R(Recip::LinearNonSoftSpring),
                D(Dot::LinearBiasUp),
                D(Dot::LinearBiasRight),
                D(Dot::LinearBiasAt),
                S(Rsqrt::AngularAxisNormalization),
                D(Dot::AngularRightOmega),
                D(Dot::AngularRightAcceleration),
                D(Dot::AngularUpOmega),
                D(Dot::AngularUpAcceleration),
                D(Dot::AngularAtOmega),
                D(Dot::AngularAtAcceleration),
                R(Recip::AngularNonSoftSpring),
                D(Dot::BodyBAngularRight),
                D(Dot::BodyBAngularUp),
                D(Dot::BodyBAngularAt),
                D(Dot::BodyBLinearRight),
                D(Dot::BodyBLinearUp),
                D(Dot::BodyBLinearAt),
                R(Recip::FinalLinear),
                R(Recip::FinalAngular),
            ]
        );
    }

    #[test]
    fn numeric_backend_soft_and_hard_below_limit_keep_exact_taken_sites() {
        use NumericSite::{Reciprocal as R, ReciprocalSqrt as S, Sum3 as D};
        use SimulationDriveDotSite as Dot;
        use SimulationDriveReciprocalSite as Recip;
        use SimulationDriveReciprocalSqrtSite as Rsqrt;

        let mut input = numeric_backend_input(SOFT_DRIVE, HARD_DRIVE, 4, 0);
        input.dynamics.angular.spring = 1_000.0;
        let mut trace = TracingHostNumericBackend::default();
        compose_drive_jacobian_with_backend(&input, &mut trace)
            .expect("tracing host backend is infallible");

        assert_eq!(
            trace.sites(),
            [
                D(Dot::QuaternionA),
                D(Dot::QuaternionB),
                R(Recip::LinearSoftDynamics),
                R(Recip::LinearSoftSpring),
                R(Recip::LinearSoftDamping),
                D(Dot::LinearBiasUp),
                D(Dot::LinearBiasRight),
                D(Dot::LinearBiasAt),
                S(Rsqrt::AngularAxisNormalization),
                D(Dot::AngularRightOmega),
                D(Dot::AngularRightAcceleration),
                D(Dot::AngularUpOmega),
                D(Dot::AngularUpAcceleration),
                D(Dot::AngularAtOmega),
                D(Dot::AngularAtAcceleration),
                R(Recip::AngularNonSoftSpring),
                D(Dot::AngularHardLengthSquared),
                S(Rsqrt::AngularHardLength),
                D(Dot::BodyAAngularRight),
                D(Dot::BodyAAngularUp),
                D(Dot::BodyAAngularAt),
                D(Dot::BodyALinearRight),
                D(Dot::BodyALinearUp),
                D(Dot::BodyALinearAt),
                R(Recip::FinalLinear),
                R(Recip::FinalAngular),
            ]
        );
        for site in [
            Recip::LinearSoftDynamics,
            Recip::LinearSoftSpring,
            Recip::LinearSoftDamping,
            Recip::AngularNonSoftSpring,
        ] {
            assert!(
                trace
                    .calls
                    .contains(&NumericCall::Reciprocal(site, [0x3f8c_0000; 4],))
            );
        }
        assert!(trace.calls.contains(&NumericCall::Sum3(
            Dot::AngularHardLengthSquared,
            [0xbec7_b12f, 0x3f56_b45b, 0xbeb6_d8d9],
            [0xbec7_b12f, 0x3f56_b45b, 0xbeb6_d8d9],
        )));
        assert!(trace.calls.contains(&NumericCall::ReciprocalSqrt(
            Rsqrt::AngularHardLength,
            [0x3f7b_a986; 4],
        )));
    }

    #[test]
    fn numeric_backend_hard_equality_still_requests_reciprocal_sqrt() {
        use SimulationDriveReciprocalSqrtSite as Rsqrt;

        let mut input = numeric_backend_input(0, HARD_DRIVE, 0, 4);
        input.dynamics.angular.spring = 2.0;
        let limit = input.time_step * input.dynamics.angular.spring;
        let limit_squared = limit * limit;
        let mut trace = TracingHostNumericBackend {
            calls: Vec::new(),
            angular_hard_length_override: Some(limit_squared),
        };
        compose_drive_jacobian_with_backend(&input, &mut trace)
            .expect("tracing host backend is infallible");

        assert!(trace.calls.contains(&NumericCall::ReciprocalSqrt(
            Rsqrt::AngularHardLength,
            [limit_squared.to_bits(); 4],
        )));
    }

    fn effective_mass_trace_sites(state_a: u32, state_b: u32) -> Vec<NumericSite> {
        use SimulationDriveDotSite as Dot;
        let input = numeric_backend_input(0, 0, state_a, state_b);
        let mut trace = TracingHostNumericBackend::default();
        compose_drive_jacobian_with_backend(&input, &mut trace)
            .expect("tracing host backend is infallible");
        trace
            .sites()
            .into_iter()
            .filter(|site| {
                matches!(
                    site,
                    NumericSite::Sum3(
                        Dot::BodyAAngularRight
                            | Dot::BodyAAngularUp
                            | Dot::BodyAAngularAt
                            | Dot::BodyALinearRight
                            | Dot::BodyALinearUp
                            | Dot::BodyALinearAt
                            | Dot::BodyBAngularRight
                            | Dot::BodyBAngularUp
                            | Dot::BodyBAngularAt
                            | Dot::BodyBLinearRight
                            | Dot::BodyBLinearUp
                            | Dot::BodyBLinearAt
                    )
                )
            })
            .collect()
    }

    #[test]
    fn numeric_backend_body_state_matrix_is_exact_and_ordered() {
        use NumericSite::Sum3 as D;
        use SimulationDriveDotSite as Dot;
        let a = [
            D(Dot::BodyAAngularRight),
            D(Dot::BodyAAngularUp),
            D(Dot::BodyAAngularAt),
            D(Dot::BodyALinearRight),
            D(Dot::BodyALinearUp),
            D(Dot::BodyALinearAt),
        ];
        let b = [
            D(Dot::BodyBAngularRight),
            D(Dot::BodyBAngularUp),
            D(Dot::BodyBAngularAt),
            D(Dot::BodyBLinearRight),
            D(Dot::BodyBLinearUp),
            D(Dot::BodyBLinearAt),
        ];
        assert_eq!(effective_mass_trace_sites(0, 0), []);
        assert_eq!(effective_mass_trace_sites(4, 0), a);
        assert_eq!(effective_mass_trace_sites(0, 4), b);
        assert_eq!(effective_mass_trace_sites(4, 4), [a, b].concat());
    }

    #[test]
    fn numeric_backend_keeps_fallback_and_zero_final_vectors_packed() {
        use SimulationDriveReciprocalSite as Recip;
        use SimulationDriveReciprocalSqrtSite as Rsqrt;

        let mut fallback = numeric_backend_input(0, 0, 0, 0);
        fallback.frames.quaternion_a = [0.0, 0.0, 0.0, 1.0];
        fallback.frames.quaternion_b = [1.0, 0.0, 0.0, 0.0];
        let mut trace = TracingHostNumericBackend::default();
        compose_drive_jacobian_with_backend(&fallback, &mut trace)
            .expect("tracing host backend is infallible");
        assert!(trace.calls.contains(&NumericCall::ReciprocalSqrt(
            Rsqrt::AngularAxisNormalization,
            [1.0_f32.to_bits(); 4],
        )));
        assert!(
            trace
                .calls
                .contains(&NumericCall::Reciprocal(Recip::FinalLinear, [0; 4],))
        );
        assert!(
            trace
                .calls
                .contains(&NumericCall::Reciprocal(Recip::FinalAngular, [0; 4],))
        );
    }

    struct FailingNumericBackend {
        target: NumericSite,
        calls: Vec<NumericSite>,
    }

    impl SimulationDriveNumericBackend for FailingNumericBackend {
        type Error = NumericSite;

        fn reciprocal_estimate(
            &mut self,
            site: SimulationDriveReciprocalSite,
            input: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            let call = NumericSite::Reciprocal(site);
            self.calls.push(call);
            if call == self.target {
                Err(call)
            } else {
                Ok(input.map(f32::recip))
            }
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            site: SimulationDriveReciprocalSqrtSite,
            input: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            let call = NumericSite::ReciprocalSqrt(site);
            self.calls.push(call);
            if call == self.target {
                Err(call)
            } else {
                Ok(input.map(|value| value.sqrt().recip()))
            }
        }

        fn sum3(
            &mut self,
            site: SimulationDriveDotSite,
            left: [f32; 3],
            right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            let call = NumericSite::Sum3(site);
            self.calls.push(call);
            if call == self.target {
                Err(call)
            } else {
                Ok(drive_effective_mass_dot3_host(left, right))
            }
        }
    }

    #[test]
    fn numeric_backend_failure_stops_at_the_exact_site() {
        use NumericSite::{Reciprocal as R, Sum3 as D};
        use SimulationDriveDotSite as Dot;
        use SimulationDriveReciprocalSite as Recip;

        let target = D(Dot::LinearHardLengthSquared);
        let mut backend = FailingNumericBackend {
            target,
            calls: Vec::new(),
        };
        let result = compose_drive_jacobian_with_backend(
            &numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4),
            &mut backend,
        );
        let Err(error) = result else {
            panic!("injected interior site must fail");
        };
        assert_eq!(error, target);
        assert_eq!(
            backend.calls,
            [
                D(Dot::QuaternionA),
                D(Dot::QuaternionB),
                R(Recip::LinearNonSoftSpring),
                target,
            ]
        );
    }

    #[test]
    fn prepared_batch_binds_real_occurrences_to_exact_complete_record() {
        let fixture = prepared_numeric_snapshots(1);
        let mut backend = TracingHostNumericBackend::default();
        let batch = try_prepare_drive_jacobian_snapshots_with_backend(
            fixture.source,
            &fixture.snapshots,
            &mut backend,
        )
        .expect("complete prepared batch");

        assert_eq!(batch.source, fixture.source);
        assert_eq!(batch.records.len(), 1);
        let prepared = &batch.records[0];
        assert_eq!(prepared.drive, fixture.drives[0]);
        assert_eq!(prepared.body_a, fixture.body_a);
        assert_eq!(prepared.body_b, fixture.body_b);
        assert_eq!(
            drive_record_bits(&prepared.record),
            prepared_hard_soft_expected_record(SimulationDriveRecordBindings {
                solver_delta_a: 1,
                solver_delta_b: 2,
                drive: 1,
            })
        );
        assert_eq!(backend.calls, hard_soft_both_expected_calls());
        assert!(prepared.record.has_solver_delta_bindings());
        assert!(!std::mem::needs_drop::<SimulationPreparedDriveJacobian>());
        assert!(std::mem::needs_drop::<SimulationPreparedDriveJacobianBatch>());
    }

    #[test]
    fn prepared_batch_composes_each_distinct_record_input_independently() {
        let mut fixture = prepared_numeric_snapshots(2);
        let second_input = second_prepared_numeric_input();
        let second = &mut fixture.snapshots[1];
        install_prepared_body_fixture(&mut second.body_a, second_input.body_a);
        install_prepared_body_fixture(&mut second.body_b, second_input.body_b);
        second.frames = second_input.frames;
        second.dynamics = second_input.dynamics;
        second.time_step = second_input.time_step;
        second.spy = second_input.spy;
        fixture.snapshots.swap(0, 1);
        let batch = try_prepare_drive_jacobian_snapshots_with_backend(
            fixture.source,
            &fixture.snapshots,
            &mut SimulationDriveHostNumericBackend,
        )
        .expect("two distinct complete records");

        assert_eq!(batch.source, fixture.source);
        assert_eq!(batch.records.len(), 2);
        assert_eq!(
            drive_record_bits(&batch.records[0].record),
            prepared_second_expected_record(SimulationDriveRecordBindings {
                solver_delta_a: 1,
                solver_delta_b: 2,
                drive: 2,
            })
        );
        assert_eq!(
            drive_record_bits(&batch.records[1].record),
            prepared_hard_soft_expected_record(SimulationDriveRecordBindings {
                solver_delta_a: 1,
                solver_delta_b: 2,
                drive: 1,
            })
        );
        assert_eq!(batch.records[0].drive, fixture.drives[1]);
        assert_eq!(batch.records[1].drive, fixture.drives[0]);
        assert_ne!(
            drive_record_bits(&batch.records[0].record),
            drive_record_bits(&batch.records[1].record)
        );
    }

    #[test]
    fn prepared_batch_preserves_self_and_shared_body_roles_without_deduplication() {
        let mut body_pool = SimulationRigidBodyPool::try_new(3).expect("body pool");
        let mut bodies = Vec::new();
        for _ in 0..3 {
            bodies.push(
                body_pool
                    .try_add_rigid_body(identity_matrix(), None, 4)
                    .expect("body Add")
                    .expect("body capacity"),
            );
        }
        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        body_pool.begin_dynamic_update_frame(input.time_step);
        let frames = std::sync::Arc::new(std::sync::RwLock::new(input.frames));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(input.dynamics));
        let mut drive_pool = SimulationDrivePool::try_new(3, &body_pool).expect("Drive pool");
        let pairs = [
            (bodies[0], bodies[0]),
            (bodies[0], bodies[1]),
            (bodies[2], bodies[0]),
        ];
        let drives = pairs.map(|(parent, child)| {
            drive_pool
                .try_add_drive(&body_pool, parent, child, &frames, &dynamics)
                .expect("Drive Add")
                .expect("Drive capacity")
        });
        let snapshots = drive_pool
            .try_active_build_snapshots(&body_pool)
            .expect("Build snapshots");
        let batch = try_prepare_drive_jacobian_snapshots_with_backend(
            SimulationDrivePreparedBatchSource {
                drive_pool: drive_pool.authority(),
                body_pool: body_pool.authority(),
            },
            &snapshots,
            &mut SimulationDriveHostNumericBackend,
        )
        .expect("prepared batch");

        assert_eq!(batch.records.len(), 3);
        assert_eq!(
            batch
                .records
                .iter()
                .map(|record| record.drive)
                .collect::<Vec<_>>(),
            drives.map(crate::simulation_drive_pool::SimulationDriveHandle::identity)
        );
        assert_eq!(batch.records[0].body_a, bodies[0].observation());
        assert_eq!(batch.records[0].body_b, bodies[0].observation());
        assert_eq!(batch.records[1].body_a, bodies[1].observation());
        assert_eq!(batch.records[1].body_b, bodies[0].observation());
        assert_eq!(batch.records[2].body_a, bodies[0].observation());
        assert_eq!(batch.records[2].body_b, bodies[2].observation());
        let tokens = batch.records.iter().map(|record| {
            (
                record.record.solver_delta_a.0,
                record.record.solver_delta_b.0,
                record.record.drive_slot.0,
            )
        });
        assert_eq!(
            tokens.collect::<Vec<_>>(),
            [(1, 1, 1), (2, 1, 2), (1, 3, 3)]
        );
    }

    #[test]
    fn active_pool_preparation_keeps_dense_right_ring_order_not_slot_order() {
        let mut body_pool = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let child = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("child Add")
            .expect("child capacity");
        let parent = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("parent Add")
            .expect("parent capacity");
        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        let frames = std::sync::Arc::new(std::sync::RwLock::new(input.frames));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(input.dynamics));
        let mut drive_pool = SimulationDrivePool::try_new(2, &body_pool).expect("Drive pool");
        let first = drive_pool
            .try_add_drive(&body_pool, parent, child, &frames, &dynamics)
            .expect("first Add")
            .expect("first capacity");
        let second = drive_pool
            .try_add_drive(&body_pool, child, parent, &frames, &dynamics)
            .expect("second Add")
            .expect("second capacity");
        drive_pool.try_remove_drive(first).expect("remove first");
        let reused_first = drive_pool
            .try_add_drive(&body_pool, parent, child, &frames, &dynamics)
            .expect("reused Add")
            .expect("reused capacity");

        let batch = try_prepare_active_drive_jacobian_batch(&drive_pool, &body_pool)
            .expect("prepared right-ring batch");

        assert_eq!(batch.records.len(), 2);
        assert_eq!(batch.records[0].drive, second.identity());
        assert_eq!(batch.records[1].drive, reused_first.identity());
        assert_eq!(batch.records[0].record.drive_slot.0, 2);
        assert_eq!(batch.records[1].record.drive_slot.0, 1);
        assert_eq!(batch.records[0].body_a, parent.observation());
        assert_eq!(batch.records[0].body_b, child.observation());
        assert_eq!(batch.records[1].body_a, child.observation());
        assert_eq!(batch.records[1].body_b, parent.observation());
    }

    #[test]
    fn prepared_batch_checks_tokens_and_reserves_before_numeric_calls() {
        assert_eq!(
            checked_nonzero_slot_token(0).map(std::num::NonZeroU32::get),
            Some(1)
        );
        assert_eq!(
            checked_nonzero_slot_token(u32::MAX - 1).map(std::num::NonZeroU32::get),
            Some(u32::MAX)
        );
        assert_eq!(checked_nonzero_slot_token(u32::MAX), None);

        let fixture = prepared_numeric_snapshots(1);
        assert_eq!(
            map_binding_error::<usize>(
                fixture.drives[0],
                (SimulationDrivePreparedBindingKind::Drive, u32::MAX),
            ),
            SimulationDriveBatchPreparationError::BindingTokenOverflow {
                drive: fixture.drives[0],
                kind: SimulationDrivePreparedBindingKind::Drive,
                slot: u32::MAX,
            }
        );
        let mut backend = TracingHostNumericBackend::default();
        let mut reserve_calls = 0;
        let result = try_prepare_drive_jacobian_snapshots_with_backend_and_reservation(
            fixture.source,
            &fixture.snapshots,
            &mut backend,
            reserve_drive_inputs,
            |records, capacity| {
                reserve_calls += 1;
                assert!(records.is_empty());
                assert_eq!(capacity, 1);
                Err(())
            },
        );
        match result {
            Err(SimulationDriveBatchPreparationError::RecordAllocationFailed { capacity }) => {
                assert_eq!(capacity, 1);
            }
            _ => panic!("injected reservation failure must return no batch"),
        }
        assert_eq!(reserve_calls, 1);
        assert!(backend.calls.is_empty());

        let result = try_prepare_drive_jacobian_snapshots_with_backend_and_reservation(
            fixture.source,
            &fixture.snapshots,
            &mut backend,
            reserve_drive_inputs,
            |_, _| Ok(()),
        );
        assert!(matches!(
            result,
            Err(SimulationDriveBatchPreparationError::RecordAllocationFailed { capacity: 1 })
        ));
        assert!(backend.calls.is_empty());

        let result = try_prepare_drive_jacobian_snapshots_with_backend_and_reservation(
            fixture.source,
            &fixture.snapshots,
            &mut backend,
            |inputs, capacity| {
                assert!(inputs.is_empty());
                assert_eq!(capacity, 1);
                Err(())
            },
            reserve_drive_records,
        );
        assert!(matches!(
            result,
            Err(SimulationDriveBatchPreparationError::StagingAllocationFailed { capacity: 1 })
        ));
        assert!(backend.calls.is_empty());
    }

    struct FailAtNumericCall {
        fail_at: usize,
        calls: usize,
    }

    impl FailAtNumericCall {
        fn before_call(&mut self) -> Result<(), usize> {
            let ordinal = self.calls;
            self.calls += 1;
            if ordinal == self.fail_at {
                Err(ordinal)
            } else {
                Ok(())
            }
        }
    }

    impl SimulationDriveNumericBackend for FailAtNumericCall {
        type Error = usize;

        fn reciprocal_estimate(
            &mut self,
            _site: SimulationDriveReciprocalSite,
            input: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            self.before_call()?;
            Ok(input.map(f32::recip))
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            _site: SimulationDriveReciprocalSqrtSite,
            input: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            self.before_call()?;
            Ok(input.map(|value| value.sqrt().recip()))
        }

        fn sum3(
            &mut self,
            _site: SimulationDriveDotSite,
            left: [f32; 3],
            right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            self.before_call()?;
            Ok(drive_effective_mass_dot3_host(left, right))
        }
    }

    struct LockProbeNumericBackend {
        frames: std::sync::Arc<std::sync::RwLock<SimulationDriveFrames>>,
        dynamics: std::sync::Arc<std::sync::RwLock<SimulationDriveDynamics>>,
        replacement_frames: SimulationDriveFrames,
        replacement_dynamics: SimulationDriveDynamics,
        probed: bool,
    }

    impl LockProbeNumericBackend {
        fn probe_once(&mut self) -> Result<(), &'static str> {
            if !self.probed {
                *self.frames.try_write().map_err(|_| "frames still locked")? =
                    self.replacement_frames;
                *self
                    .dynamics
                    .try_write()
                    .map_err(|_| "dynamics still locked")? = self.replacement_dynamics;
                self.probed = true;
            }
            Ok(())
        }
    }

    impl SimulationDriveNumericBackend for LockProbeNumericBackend {
        type Error = &'static str;

        fn reciprocal_estimate(
            &mut self,
            _site: SimulationDriveReciprocalSite,
            input: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            self.probe_once()?;
            Ok(input.map(f32::recip))
        }

        fn reciprocal_sqrt_estimate(
            &mut self,
            _site: SimulationDriveReciprocalSqrtSite,
            input: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            self.probe_once()?;
            Ok(input.map(|value| value.sqrt().recip()))
        }

        fn sum3(
            &mut self,
            _site: SimulationDriveDotSite,
            left: [f32; 3],
            right: [f32; 3],
        ) -> Result<f32, Self::Error> {
            self.probe_once()?;
            Ok(drive_effective_mass_dot3_host(left, right))
        }
    }

    #[test]
    fn prepared_batch_numeric_failure_drops_completed_private_prefix() {
        let fixture = prepared_numeric_snapshots(2);
        let mut backend = FailAtNumericCall {
            fail_at: 32,
            calls: 0,
        };
        let result = try_prepare_drive_jacobian_snapshots_with_backend(
            fixture.source,
            &fixture.snapshots,
            &mut backend,
        );

        match result {
            Err(SimulationDriveBatchPreparationError::Numeric {
                ordinal,
                drive,
                source,
            }) => {
                assert_eq!(ordinal, 1);
                assert_eq!(drive, fixture.drives[1]);
                assert_eq!(source, 32);
            }
            _ => panic!("second-record numeric failure must return no batch"),
        }
        assert_eq!(backend.calls, 33);
    }

    #[test]
    fn active_pool_snapshot_failure_precedes_every_numeric_call() {
        let mut body_pool = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let child = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("child Add")
            .expect("child capacity");
        let parent = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("parent Add")
            .expect("parent capacity");
        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        let frames = std::sync::Arc::new(std::sync::RwLock::new(input.frames));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(input.dynamics));
        let mut drive_pool = SimulationDrivePool::try_new(1, &body_pool).expect("Drive pool");
        let drive = drive_pool
            .try_add_drive(&body_pool, parent, child, &frames, &dynamics)
            .expect("Drive Add")
            .expect("Drive capacity");
        let _ = std::panic::catch_unwind({
            let dynamics = std::sync::Arc::clone(&dynamics);
            move || {
                let _guard = dynamics.write().expect("initial write");
                panic!("poison dynamics");
            }
        });
        let before_drive = drive_pool.snapshot();
        let before_body = body_pool.snapshot();
        let mut backend = TracingHostNumericBackend::default();

        let result = try_prepare_active_drive_jacobian_batch_with_backend(
            &drive_pool,
            &body_pool,
            &mut backend,
        );
        match result {
            Err(SimulationDriveBatchPreparationError::Pool(
                SimulationDrivePoolError::DynamicsPoisoned { slot },
            )) => assert_eq!(slot, drive.id().index()),
            _ => panic!("poisoned snapshot must return no prepared batch"),
        }
        assert!(backend.calls.is_empty());
        assert_eq!(drive_pool.snapshot(), before_drive);
        assert_eq!(body_pool.snapshot(), before_body);
    }

    #[test]
    fn active_pool_preparation_does_not_read_or_mutate_solver_rows() {
        let mut body_pool = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let child = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("child Add")
            .expect("child capacity");
        let parent = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("parent Add")
            .expect("parent capacity");
        for (body, base) in [(child, 1.0), (parent, 17.0)] {
            body_pool
                .try_with_solver_delta_row_mut(body.observation(), |row| {
                    for (ordinal, lane) in (0_u8..).zip(row.iter_mut().flatten()) {
                        *lane = base + f32::from(ordinal);
                    }
                })
                .expect("seed RF row");
        }
        let child_row = body_pool
            .try_with_solver_delta_row(child.observation(), |row| *row)
            .expect("child row");
        let parent_row = body_pool
            .try_with_solver_delta_row(parent.observation(), |row| *row)
            .expect("parent row");
        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        let frames = std::sync::Arc::new(std::sync::RwLock::new(input.frames));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(input.dynamics));
        let mut drive_pool = SimulationDrivePool::try_new(1, &body_pool).expect("Drive pool");
        drive_pool
            .try_add_drive(&body_pool, parent, child, &frames, &dynamics)
            .expect("Drive Add")
            .expect("Drive capacity");
        let before_drive = drive_pool.snapshot();
        let before_body = body_pool.snapshot();

        let batch = try_prepare_active_drive_jacobian_batch(&drive_pool, &body_pool)
            .expect("prepared batch");

        assert_eq!(batch.records.len(), 1);
        assert_eq!(drive_pool.snapshot(), before_drive);
        assert_eq!(body_pool.snapshot(), before_body);
        assert_eq!(
            body_pool
                .try_with_solver_delta_row(child.observation(), |row| *row)
                .expect("child row"),
            child_row
        );
        assert_eq!(
            body_pool
                .try_with_solver_delta_row(parent.observation(), |row| *row)
                .expect("parent row"),
            parent_row
        );
    }

    #[test]
    fn active_pool_preparation_releases_payload_locks_before_backend_calls() {
        let mut body_pool = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let child = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("child Add")
            .expect("child capacity");
        let parent = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("parent Add")
            .expect("parent capacity");
        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        body_pool.begin_dynamic_update_frame(input.time_step);
        let frames = std::sync::Arc::new(std::sync::RwLock::new(input.frames));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(input.dynamics));
        let mut drive_pool = SimulationDrivePool::try_new(1, &body_pool).expect("Drive pool");
        drive_pool
            .try_add_drive(&body_pool, parent, child, &frames, &dynamics)
            .expect("Drive Add")
            .expect("Drive capacity");
        let baseline = try_prepare_active_drive_jacobian_batch(&drive_pool, &body_pool)
            .expect("baseline batch");
        let mut replacement_frames = input.frames;
        replacement_frames.position_a[0] += 1.0;
        let mut replacement_dynamics = input.dynamics;
        replacement_dynamics.linear.strength += 0.5;
        let mut backend = LockProbeNumericBackend {
            frames: std::sync::Arc::clone(&frames),
            dynamics: std::sync::Arc::clone(&dynamics),
            replacement_frames,
            replacement_dynamics,
            probed: false,
        };

        let during_mutation = try_prepare_active_drive_jacobian_batch_with_backend(
            &drive_pool,
            &body_pool,
            &mut backend,
        )
        .expect("payload locks released before backend");

        assert!(backend.probed);
        assert_eq!(
            drive_record_bits(&during_mutation.records[0].record),
            drive_record_bits(&baseline.records[0].record)
        );
        let after_mutation = try_prepare_active_drive_jacobian_batch(&drive_pool, &body_pool)
            .expect("second batch observes live mutation");
        assert_ne!(
            drive_record_bits(&after_mutation.records[0].record),
            drive_record_bits(&baseline.records[0].record)
        );
        assert_eq!(*frames.read().expect("frames"), replacement_frames);
        assert_eq!(*dynamics.read().expect("dynamics"), replacement_dynamics);
    }

    #[test]
    fn active_pool_preparation_keeps_empty_batch_private_and_inert() {
        let body_pool = SimulationRigidBodyPool::try_new(0).expect("body pool");
        let drive_pool = SimulationDrivePool::try_new(0, &body_pool).expect("Drive pool");
        let batch = try_prepare_active_drive_jacobian_batch(&drive_pool, &body_pool)
            .expect("empty preparation");
        let expected_source = SimulationDrivePreparedBatchSource {
            drive_pool: drive_pool.authority(),
            body_pool: body_pool.authority(),
        };
        assert_eq!(batch.source, expected_source);
        assert!(batch.records.is_empty());

        let mut backend = TracingHostNumericBackend::default();
        let traced = try_prepare_active_drive_jacobian_batch_with_backend(
            &drive_pool,
            &body_pool,
            &mut backend,
        )
        .expect("empty traced preparation");
        assert_eq!(traced.source, expected_source);
        assert!(traced.records.is_empty());
        assert!(backend.calls.is_empty());
    }

    #[test]
    fn horus_sweep_checks_both_sources_even_for_an_empty_batch() {
        let mut body_a = SimulationRigidBodyPool::try_new(0).expect("body A");
        let drive_a = SimulationDrivePool::try_new(0, &body_a).expect("Drive A");
        let mut batch = try_prepare_active_drive_jacobian_batch(&drive_a, &body_a)
            .expect("empty prepared batch");
        let drive_foreign = SimulationDrivePool::try_new(0, &body_a).expect("foreign Drive");
        assert_eq!(
            try_solve_prepared_drive_horus_sweep(&mut batch, &drive_foreign, &mut body_a,),
            Err(SimulationPreparedDriveHorusError::DrivePoolSourceMismatch)
        );

        let mut body_foreign = SimulationRigidBodyPool::try_new(0).expect("foreign body");
        assert_eq!(
            try_solve_prepared_drive_horus_sweep(&mut batch, &drive_a, &mut body_foreign,),
            Err(SimulationPreparedDriveHorusError::BodyPoolSourceMismatch)
        );
        let drive_b = SimulationDrivePool::try_new(0, &body_foreign).expect("foreign pair Drive");
        assert_eq!(
            try_solve_prepared_drive_horus_sweep(&mut batch, &drive_b, &mut body_foreign,),
            Err(SimulationPreparedDriveHorusError::DrivePoolSourceMismatch)
        );
        assert!(batch.records.is_empty());
        try_solve_prepared_drive_horus_sweep(&mut batch, &drive_a, &mut body_a)
            .expect("matching empty sweep");
    }

    #[test]
    fn horus_sweep_rejects_later_stale_drive_before_any_record_or_rf_effect() {
        let mut fixture = prepared_live_pool_fixture(2);
        for (body, seed) in [(fixture.child, 1.0), (fixture.parent, 17.0)] {
            fixture
                .body_pool
                .try_with_solver_delta_row_mut(body.observation(), |row| {
                    for (ordinal, lane) in (0_u8..).zip(row.iter_mut().flatten()) {
                        *lane = seed + f32::from(ordinal);
                    }
                })
                .expect("seed RF");
        }
        let before_records = fixture
            .batch
            .records
            .iter()
            .map(|prepared| drive_record_bits(&prepared.record))
            .collect::<Vec<_>>();
        let before_child = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.child.observation(), |row| *row)
            .expect("child row");
        let before_parent = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.parent.observation(), |row| *row)
            .expect("parent row");
        fixture
            .drive_pool
            .try_remove_drive(fixture.drives[1])
            .expect("remove second Drive");
        let reused = fixture
            .drive_pool
            .try_add_drive(
                &fixture.body_pool,
                fixture.child,
                fixture.parent,
                &fixture.frames,
                &fixture.dynamics,
            )
            .expect("reuse second Drive")
            .expect("Drive capacity");
        assert_eq!(reused.id(), fixture.drives[1].id());

        let result = try_solve_prepared_drive_horus_sweep(
            &mut fixture.batch,
            &fixture.drive_pool,
            &mut fixture.body_pool,
        );
        assert!(matches!(
            result,
            Err(SimulationPreparedDriveHorusError::Drive(
                SimulationDriveBuildBindingValidationError::Binding {
                    ordinal: 1,
                    source: SimulationDrivePoolError::StaleHandle { slot: 1 },
                    ..
                }
            ))
        ));
        assert_eq!(
            fixture
                .batch
                .records
                .iter()
                .map(|prepared| drive_record_bits(&prepared.record))
                .collect::<Vec<_>>(),
            before_records
        );
        assert_eq!(
            fixture
                .body_pool
                .try_with_solver_delta_row(fixture.child.observation(), |row| *row)
                .expect("unchanged child"),
            before_child
        );
        assert_eq!(
            fixture
                .body_pool
                .try_with_solver_delta_row(fixture.parent.observation(), |row| *row)
                .expect("unchanged parent"),
            before_parent
        );
    }

    #[test]
    fn horus_sweep_endpoint_token_and_body_failures_are_pre_effect() {
        let mut endpoint = prepared_live_pool_fixture(2);
        let before_endpoint = endpoint
            .batch
            .records
            .iter()
            .map(|prepared| drive_record_bits(&prepared.record))
            .collect::<Vec<_>>();
        endpoint.batch.records[1].body_a = endpoint.parent.observation();
        assert!(matches!(
            try_solve_prepared_drive_horus_sweep(
                &mut endpoint.batch,
                &endpoint.drive_pool,
                &mut endpoint.body_pool,
            ),
            Err(SimulationPreparedDriveHorusError::Drive(
                SimulationDriveBuildBindingValidationError::EndpointMismatch {
                    ordinal: 1,
                    endpoint: crate::simulation_drive_pool::SimulationDriveEndpoint::Child,
                    ..
                }
            ))
        ));
        assert_eq!(
            drive_record_bits(&endpoint.batch.records[0].record),
            before_endpoint[0]
        );

        let mut token = prepared_live_pool_fixture(2);
        let before_token = drive_record_bits(&token.batch.records[0].record);
        token.batch.records[1].record.solver_delta_b = SolverDeltaRowToken(0);
        assert!(matches!(
            try_solve_prepared_drive_horus_sweep(
                &mut token.batch,
                &token.drive_pool,
                &mut token.body_pool,
            ),
            Err(SimulationPreparedDriveHorusError::BindingMismatch {
                ordinal: 1,
                kind: SimulationDrivePreparedBindingKind::BodyB,
                expected: 2,
                actual: 0,
                ..
            })
        ));
        assert_eq!(
            drive_record_bits(&token.batch.records[0].record),
            before_token
        );

        let mut body = prepared_live_pool_fixture(2);
        body.body_pool
            .try_remove_rigid_body(body.parent)
            .expect("remove parent after Build");
        let before_body_records = body
            .batch
            .records
            .iter()
            .map(|prepared| drive_record_bits(&prepared.record))
            .collect::<Vec<_>>();
        assert!(matches!(
            try_solve_prepared_drive_horus_sweep(
                &mut body.batch,
                &body.drive_pool,
                &mut body.body_pool,
            ),
            Err(SimulationPreparedDriveHorusError::Body(
                SimulationRigidBodySolverDeltaTransactionError::Body {
                    ordinal: 0,
                    endpoint: crate::simulation_rigid_body_pool::SimulationRigidBodySolverDeltaEndpoint::BodyB,
                    source: crate::simulation_rigid_body_pool::SimulationRigidBodyPoolError::BodyIsFree { slot: 1 },
                }
            ))
        ));
        assert_eq!(
            body.batch
                .records
                .iter()
                .map(|prepared| drive_record_bits(&prepared.record))
                .collect::<Vec<_>>(),
            before_body_records
        );
    }

    #[test]
    fn horus_sweep_checks_every_endpoint_role_and_token_before_all_effects() {
        use crate::simulation_drive_pool::SimulationDriveEndpoint;

        for endpoint in [
            SimulationDriveEndpoint::Child,
            SimulationDriveEndpoint::Parent,
        ] {
            let mut fixture = prepared_live_pool_fixture(2);
            seed_prepared_live_rf(&mut fixture);
            match endpoint {
                SimulationDriveEndpoint::Child => {
                    fixture.batch.records[1].body_a = fixture.parent.observation();
                }
                SimulationDriveEndpoint::Parent => {
                    fixture.batch.records[1].body_b = fixture.child.observation();
                }
            }
            let before = prepared_live_effect_bits(&fixture);
            let result = try_solve_prepared_drive_horus_sweep(
                &mut fixture.batch,
                &fixture.drive_pool,
                &mut fixture.body_pool,
            );
            let Err(SimulationPreparedDriveHorusError::Drive(
                SimulationDriveBuildBindingValidationError::EndpointMismatch {
                    ordinal,
                    endpoint: actual_endpoint,
                    ..
                },
            )) = result
            else {
                panic!("changed endpoint must fail before effects");
            };
            assert_eq!(ordinal, 1);
            assert_eq!(actual_endpoint, endpoint);
            assert_eq!(prepared_live_effect_bits(&fixture), before);
        }

        for (kind, expected, wrong_nonzero) in [
            (SimulationDrivePreparedBindingKind::BodyA, 1, 2),
            (SimulationDrivePreparedBindingKind::BodyB, 2, 1),
            (SimulationDrivePreparedBindingKind::Drive, 2, 1),
        ] {
            for actual in [0, wrong_nonzero] {
                let mut fixture = prepared_live_pool_fixture(2);
                seed_prepared_live_rf(&mut fixture);
                match kind {
                    SimulationDrivePreparedBindingKind::BodyA => {
                        fixture.batch.records[1].record.solver_delta_a =
                            SolverDeltaRowToken(actual);
                    }
                    SimulationDrivePreparedBindingKind::BodyB => {
                        fixture.batch.records[1].record.solver_delta_b =
                            SolverDeltaRowToken(actual);
                    }
                    SimulationDrivePreparedBindingKind::Drive => {
                        fixture.batch.records[1].record.drive_slot = DriveSlotToken(actual);
                    }
                }
                let before = prepared_live_effect_bits(&fixture);
                let result = try_solve_prepared_drive_horus_sweep(
                    &mut fixture.batch,
                    &fixture.drive_pool,
                    &mut fixture.body_pool,
                );
                let Err(SimulationPreparedDriveHorusError::BindingMismatch {
                    ordinal,
                    kind: actual_kind,
                    expected: actual_expected,
                    actual: rejected,
                    ..
                }) = result
                else {
                    panic!("changed token must fail before effects");
                };
                assert_eq!(ordinal, 1);
                assert_eq!(actual_kind, kind);
                assert_eq!(actual_expected, expected);
                assert_eq!(rejected, actual);
                assert_eq!(prepared_live_effect_bits(&fixture), before);
            }
        }
    }

    #[test]
    fn horus_sweep_rejects_later_body_aba_even_when_one_based_token_repeats() {
        let mut body_pool = SimulationRigidBodyPool::try_new(3).expect("body pool");
        let child = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("child Add")
            .expect("child capacity");
        let parent_0 = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("parent 0 Add")
            .expect("parent 0 capacity");
        let parent_1 = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("parent 1 Add")
            .expect("parent 1 capacity");
        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        let frames = std::sync::Arc::new(std::sync::RwLock::new(input.frames));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(input.dynamics));
        let mut drive_pool = SimulationDrivePool::try_new(2, &body_pool).expect("Drive pool");
        drive_pool
            .try_add_drive(&body_pool, parent_0, child, &frames, &dynamics)
            .expect("Drive 0 Add")
            .expect("Drive 0 capacity");
        drive_pool
            .try_add_drive(&body_pool, parent_1, child, &frames, &dynamics)
            .expect("Drive 1 Add")
            .expect("Drive 1 capacity");
        let mut batch = try_prepare_active_drive_jacobian_batch(&drive_pool, &body_pool)
            .expect("prepared ABA batch");
        for (body, seed) in [(child, 1.0), (parent_0, 17.0), (parent_1, 33.0)] {
            body_pool
                .try_with_solver_delta_row_mut(body.observation(), |row| {
                    for (ordinal, lane) in (0_u8..).zip(row.iter_mut().flatten()) {
                        *lane = seed + f32::from(ordinal);
                    }
                })
                .expect("seed ABA RF");
        }
        body_pool
            .try_remove_rigid_body(parent_1)
            .expect("remove later parent");
        let replacement = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("replacement Add")
            .expect("replacement capacity");
        assert_eq!(replacement.id(), parent_1.id());
        assert_eq!(batch.records[1].record.solver_delta_b.0, 3);
        let before_records = batch
            .records
            .iter()
            .map(|prepared| drive_record_bits(&prepared.record))
            .collect::<Vec<_>>();
        let before_rows = [child, parent_0, replacement].map(|body| {
            body_pool
                .try_with_solver_delta_row(body.observation(), |row| bits_rows(*row))
                .expect("ABA before row")
        });

        let result = try_solve_prepared_drive_horus_sweep(&mut batch, &drive_pool, &mut body_pool);
        assert!(matches!(
            result,
            Err(SimulationPreparedDriveHorusError::Body(
                SimulationRigidBodySolverDeltaTransactionError::Body {
                    ordinal: 1,
                    endpoint: crate::simulation_rigid_body_pool::SimulationRigidBodySolverDeltaEndpoint::BodyB,
                    source: crate::simulation_rigid_body_pool::SimulationRigidBodyPoolError::StaleHandle { slot: 2 },
                }
            ))
        ));
        assert_eq!(
            batch
                .records
                .iter()
                .map(|prepared| drive_record_bits(&prepared.record))
                .collect::<Vec<_>>(),
            before_records
        );
        let after_rows = [child, parent_0, replacement].map(|body| {
            body_pool
                .try_with_solver_delta_row(body.observation(), |row| bits_rows(*row))
                .expect("ABA unchanged row")
        });
        assert_eq!(after_rows, before_rows);
    }

    #[test]
    fn horus_sweep_revalidation_never_relocks_prepared_payloads() {
        let mut fixture = prepared_live_pool_fixture(1);
        seed_prepared_live_rf(&mut fixture);
        let _ = std::panic::catch_unwind({
            let frames = std::sync::Arc::clone(&fixture.frames);
            move || {
                let _guard = frames.write().expect("initial frames write");
                panic!("poison prepared frames");
            }
        });
        let _ = std::panic::catch_unwind({
            let dynamics = std::sync::Arc::clone(&fixture.dynamics);
            move || {
                let _guard = dynamics.write().expect("initial dynamics write");
                panic!("poison prepared dynamics");
            }
        });

        try_solve_prepared_drive_horus_sweep(
            &mut fixture.batch,
            &fixture.drive_pool,
            &mut fixture.body_pool,
        )
        .expect("sweep consumes detached prepared values without payload locks");
    }

    #[test]
    fn horus_sweep_self_alias_matches_legal_token_b_wins_oracle() {
        let mut body_pool = SimulationRigidBodyPool::try_new(1).expect("body pool");
        let body = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("body Add")
            .expect("body capacity");
        let frames = std::sync::Arc::new(std::sync::RwLock::new(SimulationDriveFrames {
            quaternion_a: [0.0; 4],
            position_a: [0.0; 4],
            quaternion_b: [0.0; 4],
            position_b: [0.0; 4],
        }));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(SimulationDriveDynamics {
            linear: no_drive_angular_params(),
            angular: no_drive_angular_params(),
        }));
        let mut drive_pool = SimulationDrivePool::try_new(1, &body_pool).expect("Drive pool");
        let drive = drive_pool
            .try_add_drive(&body_pool, body, body, &frames, &dynamics)
            .expect("Drive Add")
            .expect("Drive capacity");
        let (mut record, initial_row, _) = horus_fixture();
        record.solver_delta_a = SolverDeltaRowToken(1);
        record.solver_delta_b = SolverDeltaRowToken(1);
        record.drive_slot = DriveSlotToken(1);
        let before_record = record;
        body_pool
            .try_with_solver_delta_row_mut(body.observation(), |row| *row = initial_row)
            .expect("seed self RF");
        let mut batch = SimulationPreparedDriveJacobianBatch {
            source: SimulationDrivePreparedBatchSource {
                drive_pool: drive_pool.authority(),
                body_pool: body_pool.authority(),
            },
            records: vec![SimulationPreparedDriveJacobian {
                drive: drive.identity(),
                body_a: body.observation(),
                body_b: body.observation(),
                record,
            }],
        };

        try_solve_prepared_drive_horus_sweep(&mut batch, &drive_pool, &mut body_pool)
            .expect("self Drive sweep");

        let mut expected_record = before_record;
        expected_record.position_impulse = [
            f32::from_bits(0xbe26_e97a),
            f32::from_bits(0x3cad_ab92),
            f32::from_bits(0xbe42_8f5c),
        ];
        expected_record.angular_impulse = [
            f32::from_bits(0xbe2e_978d),
            f32::from_bits(0x3e38_51ec),
            f32::from_bits(0xbe85_1eb8),
        ];
        assert!(batch.records[0].record == expected_record);
        assert_eq!(
            body_pool
                .try_with_solver_delta_row(body.observation(), |row| bits_rows(*row))
                .expect("self RF result"),
            [
                [0x3f00_8a26, 0xbf4d_184c, 0x3f81_9b91, 0x3f8d_134a],
                [0x4130_0000, 0x4140_0000, 0x4150_0000, 0x4160_0000],
                [0x3e55_b197, 0x3eda_bf9e, 0xbf63_70c6, 0x3f94_9e5c],
                [0x41f8_0000, 0x4200_0000, 0x4204_0000, 0x4208_0000],
            ]
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one uninterrupted independent two-record oracle fixture keeps every role auditable"
    )]
    fn horus_sweep_shared_body_matches_sequential_not_precopied_oracle() {
        let mut body_pool = SimulationRigidBodyPool::try_new(3).expect("body pool");
        let body_p = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("P Add")
            .expect("P capacity");
        let body_x = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("X Add")
            .expect("X capacity");
        let body_q = body_pool
            .try_add_rigid_body(identity_matrix(), None, 4)
            .expect("Q Add")
            .expect("Q capacity");
        let frames = std::sync::Arc::new(std::sync::RwLock::new(SimulationDriveFrames {
            quaternion_a: [0.0; 4],
            position_a: [0.0; 4],
            quaternion_b: [0.0; 4],
            position_b: [0.0; 4],
        }));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(SimulationDriveDynamics {
            linear: no_drive_angular_params(),
            angular: no_drive_angular_params(),
        }));
        let mut drive_pool = SimulationDrivePool::try_new(2, &body_pool).expect("Drive pool");
        let drive_0 = drive_pool
            .try_add_drive(&body_pool, body_x, body_p, &frames, &dynamics)
            .expect("Drive 0 Add")
            .expect("Drive 0 capacity");
        let drive_1 = drive_pool
            .try_add_drive(&body_pool, body_q, body_x, &frames, &dynamics)
            .expect("Drive 1 Add")
            .expect("Drive 1 capacity");
        let (mut record_0, row_p, row_x) = horus_fixture();
        record_0.solver_delta_a = SolverDeltaRowToken(1);
        record_0.solver_delta_b = SolverDeltaRowToken(2);
        record_0.drive_slot = DriveSlotToken(1);
        let record_1 = horus_second_representable_record();
        let row_q = [
            [0.45, -0.15, 0.35, -1.1],
            [101.0, 102.0, 103.0, 104.0],
            [-0.25, 0.65, -0.45, 1.3],
            [121.0, 122.0, 123.0, 124.0],
        ];
        for (body, row) in [(body_p, row_p), (body_x, row_x), (body_q, row_q)] {
            body_pool
                .try_with_solver_delta_row_mut(body.observation(), |target| *target = row)
                .expect("seed shared RF");
        }
        let before_0 = record_0;
        let before_1 = record_1;
        let mut batch = SimulationPreparedDriveJacobianBatch {
            source: SimulationDrivePreparedBatchSource {
                drive_pool: drive_pool.authority(),
                body_pool: body_pool.authority(),
            },
            records: vec![
                SimulationPreparedDriveJacobian {
                    drive: drive_0.identity(),
                    body_a: body_p.observation(),
                    body_b: body_x.observation(),
                    record: record_0,
                },
                SimulationPreparedDriveJacobian {
                    drive: drive_1.identity(),
                    body_a: body_x.observation(),
                    body_b: body_q.observation(),
                    record: record_1,
                },
            ],
        };

        try_solve_prepared_drive_horus_sweep(&mut batch, &drive_pool, &mut body_pool)
            .expect("shared Drive sweep");

        let mut expected_0 = before_0;
        expected_0.position_impulse = [
            f32::from_bits(0x3e9e_b852),
            f32::from_bits(0xbe8a_3d71),
            f32::from_bits(0x3e42_8f5c),
        ];
        expected_0.angular_impulse = [
            f32::from_bits(0xbe61_47ae),
            f32::from_bits(0x3e38_51ec),
            f32::from_bits(0xbe85_1eb8),
        ];
        let mut expected_1 = before_1;
        expected_1.position_impulse = [
            f32::from_bits(0x3e8f_5c29),
            f32::from_bits(0xbe75_c28f),
            f32::from_bits(0x3e57_0a3d),
        ];
        expected_1.angular_impulse = [
            f32::from_bits(0xbe4c_cccd),
            f32::from_bits(0x3e2e_147b),
            f32::from_bits(0xbe6b_851f),
        ];
        assert!(batch.records[0].record == expected_0);
        assert!(batch.records[1].record == expected_1);
        // Both correct and incorrectly precopied inputs saturate the second
        // accumulator. Its shared-body response still distinguishes ordering.
        assert_ne!(
            body_pool
                .try_with_solver_delta_row(body_x.observation(), |row| row[0][0].to_bits())
                .expect("shared RF result"),
            0xbefc_8020,
            "independent all-record RF precopy mutation"
        );
        assert_eq!(
            body_pool
                .try_with_solver_delta_row(body_p.observation(), |row| bits_rows(*row))
                .expect("P final"),
            [
                [0x3ea4_a8c1, 0xbf04_b5dd, 0x3f39_652c, 0x3f7d_c726],
                [0x4130_0000, 0x4140_0000, 0x4150_0000, 0x4160_0000],
                [0xbe71_e797, 0x3f17_62a6, 0xbeab_1d2e, 0x3f49_877b],
                [0x41f8_0000, 0x4200_0000, 0x4204_0000, 0x4208_0000],
            ]
        );
        assert_eq!(
            body_pool
                .try_with_solver_delta_row(body_x.observation(), |row| bits_rows(*row))
                .expect("X final"),
            [
                [0xbf19_bea1, 0x3f15_44bb, 0xbf28_b684, 0x3f92_bee0],
                [0x4224_0000, 0x4228_0000, 0x422c_0000, 0x4230_0000],
                [0x3e14_9635, 0x3f24_0ecf, 0xbc8d_3ef8, 0xbfa5_72be],
                [0x4274_0000, 0x4278_0000, 0x427c_0000, 0x4280_0000],
            ]
        );
        assert_eq!(
            body_pool
                .try_with_solver_delta_row(body_q.observation(), |row| bits_rows(*row))
                .expect("Q final"),
            [
                [0x3f56_19b9, 0xbd79_b12f, 0xbd5f_5ae3, 0xbf77_8423],
                [0x42ca_0000, 0x42cc_0000, 0x42ce_0000, 0x42d0_0000],
                [0x3f02_1eb6, 0x3f22_934b, 0x3f56_4711, 0x3fac_e3ff],
                [0x42f2_0000, 0x42f4_0000, 0x42f6_0000, 0x42f8_0000],
            ]
        );
    }

    #[test]
    fn effective_mass_gates_a_and_b_independently_and_preserves_swap_roles() {
        let (body_a, body_b, linear_axes, angular_axes) = effective_mass_fixture();
        let inactive_b = SimulationDriveEffectiveMassBodyInput { state: 0, ..body_b };
        let a_only = build_drive_effective_mass_host(body_a, inactive_b, linear_axes, angular_axes);
        assert_eq!(a_only.inverse_inertia_b.map(bits3), [[0; 3]; 3]);
        assert_eq!(a_only.inverse_mass_b.to_bits(), 0);
        assert_eq!(
            bits3(a_only.linear_denominator),
            [0x425f_8000, 0x4080_0000, 0x42a9_7000]
        );
        assert_eq!(
            bits3(a_only.angular_denominator),
            [0x4185_0000, 0x4152_0000, 0x40ff_8000]
        );

        let inactive_a = SimulationDriveEffectiveMassBodyInput { state: 8, ..body_a };
        let b_only = build_drive_effective_mass_host(inactive_a, body_b, linear_axes, angular_axes);
        assert_eq!(b_only.inverse_inertia_a.map(bits3), [[0; 3]; 3]);
        assert_eq!(b_only.inverse_mass_a.to_bits(), 0);
        assert_eq!(
            bits3(b_only.linear_denominator),
            [0x420a_d000, 0x41f9_a000, 0x3fa7_0000]
        );
        assert_eq!(
            bits3(b_only.angular_denominator),
            [0x4148_8000, 0x40a6_0000, 0x4104_6000]
        );

        let both = build_drive_effective_mass_host(body_a, body_b, linear_axes, angular_axes);
        let swapped = build_drive_effective_mass_host(body_b, body_a, linear_axes, angular_axes);
        assert_eq!(swapped.inverse_inertia_a, both.inverse_inertia_b);
        assert_eq!(
            swapped.inverse_mass_a.to_bits(),
            both.inverse_mass_b.to_bits()
        );
        assert_eq!(swapped.inverse_inertia_b, both.inverse_inertia_a);
        assert_eq!(
            swapped.inverse_mass_b.to_bits(),
            both.inverse_mass_a.to_bits()
        );
        assert_eq!(
            bits3(swapped.linear_denominator),
            bits3(both.linear_denominator)
        );
        assert_eq!(
            bits3(swapped.angular_denominator),
            bits3(both.angular_denominator)
        );
    }

    #[test]
    fn effective_mass_both_inactive_retains_zero_denominator_boundary() {
        let (body_a, body_b, linear_axes, angular_axes) = effective_mass_fixture();
        let result = build_drive_effective_mass_host(
            SimulationDriveEffectiveMassBodyInput { state: 0, ..body_a },
            SimulationDriveEffectiveMassBodyInput { state: 0, ..body_b },
            linear_axes,
            angular_axes,
        );
        assert_eq!(result.inverse_inertia_a.map(bits3), [[0; 3]; 3]);
        assert_eq!(result.inverse_inertia_b.map(bits3), [[0; 3]; 3]);
        assert_eq!(result.inverse_mass_a.to_bits(), 0);
        assert_eq!(result.inverse_mass_b.to_bits(), 0);
        assert_eq!(bits3(result.linear_denominator), [0; 3]);
        assert_eq!(bits3(result.angular_denominator), [0; 3]);
    }

    #[test]
    fn effective_mass_cross_preserves_source_sign_and_negative_fma_order() {
        assert_eq!(
            bits3(drive_effective_mass_cross_host(
                [0.5, -1.0, 2.0],
                [1.0, 2.0, -1.0]
            )),
            [0xc040_0000, 0x4020_0000, 0x4000_0000]
        );

        let arm = [
            f32::from_bits(0x405b_eb74),
            f32::from_bits(0x3b31_bcdd),
            f32::from_bits(0xb769_1ab7),
        ];
        let axis = [
            f32::from_bits(0xc6be_b3d7),
            f32::from_bits(0x371e_3911),
            f32::from_bits(0x3850_ea85),
        ];
        let arm_yzx = [arm[1], arm[2], arm[0]];
        let axis_yzx = [axis[1], axis[2], axis[0]];
        let product: [f32; 3] = std::array::from_fn(|lane| arm[lane] * axis_yzx[lane]);
        let defused_difference: [f32; 3] =
            std::array::from_fn(|lane| product[lane] - arm_yzx[lane] * axis[lane]);
        let defused = [
            defused_difference[1],
            defused_difference[2],
            defused_difference[0],
        ];
        assert_eq!(
            bits3(drive_effective_mass_cross_host(arm, axis)),
            [0x3411_304c, 0x3ead_8f26, 0x4284_670d]
        );
        assert_eq!(bits3(defused), [0x3411_304c, 0x3ead_8f26, 0x4284_670c]);
    }

    #[test]
    fn effective_mass_gate_tests_bit_four_instead_of_exact_state() {
        let (body_a, body_b, linear_axes, angular_axes) = effective_mass_fixture();
        let baseline = build_drive_effective_mass_host(body_a, body_b, linear_axes, angular_axes);
        let extra_bits = build_drive_effective_mass_host(
            SimulationDriveEffectiveMassBodyInput {
                state: 0x8000_0004,
                ..body_a
            },
            SimulationDriveEffectiveMassBodyInput {
                state: 12,
                ..body_b
            },
            linear_axes,
            angular_axes,
        );
        assert_eq!(
            bits3(extra_bits.linear_denominator),
            bits3(baseline.linear_denominator)
        );
        assert_eq!(
            bits3(extra_bits.angular_denominator),
            bits3(baseline.angular_denominator)
        );
        assert_eq!(
            extra_bits.inverse_inertia_a.map(bits3),
            baseline.inverse_inertia_a.map(bits3)
        );
        assert_eq!(
            extra_bits.inverse_inertia_b.map(bits3),
            baseline.inverse_inertia_b.map(bits3)
        );
    }

    #[test]
    fn effective_mass_preserves_per_body_mass_add_before_a_b_accumulation() {
        let body_a = SimulationDriveEffectiveMassBodyInput {
            state: 4,
            attachment_offset: [0.0, 0.0, 1.0],
            inverse_inertia_full: [f32::from_bits(0x4868_f1d5), 0.0, 0.0],
            inverse_mass: f32::from_bits(0x4918_5acf),
            inverse_inertia_split: [0.0; 3],
        };
        let body_b = SimulationDriveEffectiveMassBodyInput {
            state: 4,
            attachment_offset: [0.0, 0.0, 1.0],
            inverse_inertia_full: [f32::from_bits(0x3b4e_7fd9), 0.0, 0.0],
            inverse_mass: f32::from_bits(0x450c_8b73),
            inverse_inertia_split: [0.0; 3],
        };
        let result = build_drive_effective_mass_host(
            body_a,
            body_b,
            [[0.0, 1.0, 0.0], [0.0; 3], [0.0; 3]],
            [[0.0; 3]; 3],
        );
        let reassociated = (body_a.inverse_inertia_full[0] + body_b.inverse_inertia_full[0])
            + (body_a.inverse_mass + body_b.inverse_mass);
        assert_eq!(result.linear_denominator[0].to_bits(), 0x4953_23d0);
        assert_eq!(reassociated.to_bits(), 0x4953_23cf);
    }

    #[test]
    fn effective_mass_tensor_multiply_keeps_both_fused_adds() {
        let body = SimulationDriveEffectiveMassBodyInput {
            state: 4,
            attachment_offset: [0.0; 3],
            inverse_inertia_full: [
                f32::from_bits(0xbf48_f8a4),
                f32::from_bits(0xbcf8_9e87),
                f32::from_bits(0xc170_1fd1),
            ],
            inverse_mass: 0.0,
            inverse_inertia_split: [
                f32::from_bits(0x3e30_8067),
                f32::from_bits(0xc07d_1138),
                f32::from_bits(0xc11a_1209),
            ],
        };
        let axis = [
            f32::from_bits(0xbc61_4037),
            f32::from_bits(0x4147_0105),
            f32::from_bits(0xc12a_c0ca),
        ];
        let result = build_drive_effective_mass_host(
            body,
            SimulationDriveEffectiveMassBodyInput { state: 0, ..body },
            [[0.0; 3]; 3],
            [axis, [0.0; 3], [0.0; 3]],
        );

        let columns = result.inverse_inertia_a;
        assert_eq!(
            bits3(drive_effective_mass_tensor_multiply_host(columns, axis)),
            [0x431f_cc17, 0x4256_570d, 0xc2f2_cd71]
        );
        let defused_tensor: [f32; 3] = std::array::from_fn(|lane| {
            columns[2][lane] * axis[2] + (columns[1][lane] * axis[1] + columns[0][lane] * axis[0])
        });
        let defused = drive_effective_mass_dot3_host(defused_tensor, axis);
        assert_eq!(result.angular_denominator[0].to_bits(), 0x44f4_fc3d);
        assert_eq!(defused.to_bits(), 0x44f4_fc3c);
    }

    #[test]
    fn effective_mass_dot3_keeps_nested_reduction_fmas() {
        let left = [
            f32::from_bits(0x4b63_e2aa),
            f32::from_bits(0xc0c4_87db),
            f32::from_bits(0x4071_9760),
        ];
        let right = [
            f32::from_bits(0xb433_d1c9),
            f32::from_bits(0xbef8_eae6),
            f32::from_bits(0x36f0_aac7),
        ];
        let inner_defused = left[0].mul_add(right[0], left[1] * right[1] + left[2] * right[2]);
        let outer_defused = left[0] * right[0] + left[1].mul_add(right[1], left[2] * right[2]);
        assert_eq!(
            drive_effective_mass_dot3_host(left, right).to_bits(),
            0x3ef8_3131
        );
        assert_eq!(inner_defused.to_bits(), 0x3ef8_3139);
        assert_eq!(outer_defused.to_bits(), 0x3ef8_3130);
    }

    #[test]
    fn island_batch_uses_exact_signed_b_fallback_and_frame_bound_labels() {
        let mut fixture = prepared_live_pool_fixture(1);
        let frame = fixture.body_pool.frame_identity().expect("frame");
        let mut build = |body_b_label, body_a_label| {
            let map = SimulationRigidBodyIslandMapSnapshot::try_new(
                &fixture.body_pool,
                frame,
                [
                    (fixture.parent.observation(), body_b_label),
                    (fixture.child.observation(), body_a_label),
                ],
            )
            .expect("map");
            let mut backend = SimulationDriveHostNumericBackend;
            try_prepare_active_drive_island_batch_for_test(
                &fixture.drive_pool,
                &mut fixture.body_pool,
                SimulationDriveIslandMapSource::Present(&map),
                &mut backend,
            )
        };

        for (body_b, body_a, expected_island) in [(2, 7, 2), (-1, 7, 7), (2, -2, 2)] {
            let batch = build(body_b, body_a).expect("valid signed selection");
            assert_eq!(batch.records.len(), 1);
            assert_eq!(batch.islands[expected_island].len(), 1);
            assert_eq!(batch.islands.iter().map(Vec::len).sum::<usize>(), 1);
            assert_eq!(batch.islands[expected_island][0].dense_ordinal, 0);
            assert_eq!(
                drive_record_bits(&batch.records[0].record),
                drive_record_bits(&fixture.batch.records[0].record)
            );
        }

        for (body_b, body_a, selected) in [(-2, 7, -2), (10, 0, 10), (-1, -2, -2)] {
            assert!(matches!(
                build(body_b, body_a),
                Err(SimulationDriveIslandBatchPreparationError::InvalidSelectedIsland {
                    ordinal: 0,
                    raw_body_b,
                    raw_body_a,
                    selected: actual,
                    ..
                }) if raw_body_b == body_b && raw_body_a == body_a && actual == selected
            ));
        }

        let mut backend = SimulationDriveHostNumericBackend;
        let null_batch = try_prepare_active_drive_island_batch_for_test(
            &fixture.drive_pool,
            &mut fixture.body_pool,
            SimulationDriveIslandMapSource::Null { frame },
            &mut backend,
        )
        .expect("native null map selects zero");
        assert_eq!(null_batch.islands[0].len(), 1);

        let map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &fixture.body_pool,
            frame,
            [(fixture.parent.observation(), 2)],
        )
        .expect("parent-only map");
        assert!(matches!(
            try_prepare_active_drive_island_batch_for_test(
                &fixture.drive_pool,
                &mut fixture.body_pool,
                SimulationDriveIslandMapSource::Present(&map),
                &mut backend,
            ),
            Err(
                SimulationDriveIslandBatchPreparationError::MissingIslandLabel {
                    ordinal: 0,
                    endpoint: SimulationDriveEndpoint::Child,
                    ..
                }
            )
        ));

        let timestep = fixture.body_pool.timestep();
        fixture.body_pool.begin_dynamic_update_frame(timestep);
        assert!(matches!(
            try_prepare_active_drive_island_batch_for_test(
                &fixture.drive_pool,
                &mut fixture.body_pool,
                SimulationDriveIslandMapSource::Present(&map),
                &mut backend,
            ),
            Err(SimulationDriveIslandBatchPreparationError::IslandMapFrameMismatch)
        ));
    }

    #[test]
    fn island_batch_reads_parent_label_before_child_label() {
        let mut fixture = prepared_live_pool_fixture(1);
        let frame = fixture.body_pool.frame_identity().expect("frame");
        let map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &fixture.body_pool,
            frame,
            std::iter::empty(),
        )
        .expect("empty map");
        let mut backend = SimulationDriveHostNumericBackend;
        assert!(matches!(
            try_prepare_active_drive_island_batch_for_test(
                &fixture.drive_pool,
                &mut fixture.body_pool,
                SimulationDriveIslandMapSource::Present(&map),
                &mut backend,
            ),
            Err(
                SimulationDriveIslandBatchPreparationError::MissingIslandLabel {
                    ordinal: 0,
                    endpoint: SimulationDriveEndpoint::Parent,
                    ..
                }
            )
        ));
    }

    #[test]
    fn island_batch_checks_global_binding_then_map_authority_then_frame() {
        let mut fixture = prepared_live_pool_fixture(1);
        let mut foreign_body = SimulationRigidBodyPool::try_new(0).expect("foreign body pool");
        foreign_body.begin_dynamic_update_frame(0.25);
        foreign_body.begin_dynamic_update_frame(0.25);
        let foreign_frame = foreign_body.frame_identity().expect("foreign frame");
        let foreign_map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &foreign_body,
            foreign_frame,
            std::iter::empty(),
        )
        .expect("foreign map");
        let mut other_body = SimulationRigidBodyPool::try_new(0).expect("other body pool");
        let mut backend = TracingHostNumericBackend::default();
        assert!(matches!(
            try_prepare_active_drive_island_batch_for_test(
                &fixture.drive_pool,
                &mut other_body,
                SimulationDriveIslandMapSource::Present(&foreign_map),
                &mut backend,
            ),
            Err(SimulationDriveIslandBatchPreparationError::Pool(
                SimulationDrivePoolError::BodyPoolMismatch,
            ))
        ));
        assert!(backend.calls.is_empty());

        assert!(matches!(
            try_prepare_active_drive_island_batch_for_test(
                &fixture.drive_pool,
                &mut fixture.body_pool,
                SimulationDriveIslandMapSource::Present(&foreign_map),
                &mut backend,
            ),
            Err(SimulationDriveIslandBatchPreparationError::IslandMapBodyPoolMismatch)
        ));
        assert!(backend.calls.is_empty());
    }

    #[test]
    fn island_batch_rejects_occurrence_stale_labels_in_both_endpoint_roles() {
        for endpoint in [
            SimulationDriveEndpoint::Parent,
            SimulationDriveEndpoint::Child,
        ] {
            let mut fixture = prepared_live_island_fixture(1, 2);
            let frame = fixture.body_pool.frame_identity().expect("frame");
            let old_parent = fixture.parent.observation();
            let old_child = fixture.child.observation();
            let old_map = SimulationRigidBodyIslandMapSnapshot::try_new(
                &fixture.body_pool,
                frame,
                [(old_parent, 2), (old_child, 2)],
            )
            .expect("old map");
            fixture
                .drive_pool
                .try_remove_drive(fixture.drives[0])
                .expect("remove Drive");
            let old_body = match endpoint {
                SimulationDriveEndpoint::Parent => fixture.parent,
                SimulationDriveEndpoint::Child => fixture.child,
            };
            fixture
                .body_pool
                .try_remove_rigid_body(old_body)
                .expect("remove body");
            let inertia = std::sync::Arc::new(
                crate::simulation_rigid_body_pool::SimulationInertia::native_default(),
            );
            let replacement = fixture
                .body_pool
                .try_add_rigid_body(identity_matrix(), Some(&inertia), 4)
                .expect("replace body")
                .expect("body capacity");
            assert_eq!(replacement.id(), old_body.id());
            match endpoint {
                SimulationDriveEndpoint::Parent => fixture.parent = replacement,
                SimulationDriveEndpoint::Child => fixture.child = replacement,
            }
            fixture
                .drive_pool
                .try_add_drive(
                    &fixture.body_pool,
                    fixture.parent,
                    fixture.child,
                    &fixture.frames,
                    &fixture.dynamics,
                )
                .expect("replace Drive")
                .expect("Drive capacity");

            let mut backend = TracingHostNumericBackend::default();
            assert!(matches!(
                try_prepare_active_drive_island_batch_for_test(
                    &fixture.drive_pool,
                    &mut fixture.body_pool,
                    SimulationDriveIslandMapSource::Present(&old_map),
                    &mut backend,
                ),
                Err(
                    SimulationDriveIslandBatchPreparationError::IslandLabelOccurrenceMismatch {
                        ordinal: 0,
                        endpoint: actual_endpoint,
                        ..
                    }
                ) if actual_endpoint == endpoint
            ));
            assert!(backend.calls.is_empty());
        }
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the five-Drive independent oracle fixture is intentionally explicit"
    )]
    fn island_batch_keeps_compact_dense_and_stable_filtered_ring_order() {
        let mut body_pool = SimulationRigidBodyPool::try_new(10).expect("body pool");
        let inertia = zero_dynamic_inertia();
        let states = [(4, 0), (0, 4), (0, 0), (4, 0), (0, 4)];
        let labels = [(0, 2), (1, -1), (4, 3), (2, 1), (0, 2)];
        let mut pairs = Vec::new();
        for &(state_a, state_b) in &states {
            let body_a = body_pool
                .try_add_rigid_body(identity_matrix(), Some(&inertia), u32::from(state_a == 0))
                .expect("body A")
                .expect("body A capacity");
            let body_b = body_pool
                .try_add_rigid_body(identity_matrix(), Some(&inertia), u32::from(state_b == 0))
                .expect("body B")
                .expect("body B capacity");
            pairs.push((body_a, body_b));
        }
        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        body_pool.begin_dynamic_update_frame(input.time_step);
        let frames = std::sync::Arc::new(std::sync::RwLock::new(input.frames));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(input.dynamics));
        let mut drive_pool = SimulationDrivePool::try_new(5, &body_pool).expect("Drive pool");
        let drives = pairs
            .iter()
            .map(|&(body_a, body_b)| {
                drive_pool
                    .try_add_drive(&body_pool, body_b, body_a, &frames, &dynamics)
                    .expect("Drive Add")
                    .expect("Drive capacity")
            })
            .collect::<Vec<_>>();
        drive_pool
            .try_remove_drive(drives[1])
            .expect("remove Drive before tail re-add");
        let readded = drive_pool
            .try_add_drive(&body_pool, pairs[1].1, pairs[1].0, &frames, &dynamics)
            .expect("Drive re-add")
            .expect("Drive capacity");
        let frame = body_pool.frame_identity().expect("frame");
        let assignments = pairs
            .iter()
            .zip(labels)
            .enumerate()
            .filter(|(ordinal, _)| *ordinal != 2)
            .flat_map(|(_, (&(body_a, body_b), (label_a, label_b)))| {
                [
                    (body_b.observation(), label_b),
                    (body_a.observation(), label_a),
                ]
            });
        let map = SimulationRigidBodyIslandMapSnapshot::try_new(&body_pool, frame, assignments)
            .expect("map without rejected endpoints");
        let mut backend = SimulationDriveHostNumericBackend;
        let batch = try_prepare_active_drive_island_batch_for_test(
            &drive_pool,
            &mut body_pool,
            SimulationDriveIslandMapSource::Present(&map),
            &mut backend,
        )
        .expect("island batch");
        let direct = try_prepare_active_drive_jacobian_batch(&drive_pool, &body_pool)
            .expect("direct dense comparison batch");

        assert_eq!(batch.records.len(), 4);
        assert_eq!(direct.records.len(), batch.records.len());
        assert_eq!(
            batch
                .records
                .iter()
                .map(|prepared| drive_record_bits(&prepared.record))
                .collect::<Vec<_>>(),
            direct
                .records
                .iter()
                .map(|prepared| drive_record_bits(&prepared.record))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            batch
                .records
                .iter()
                .map(|record| record.drive)
                .collect::<Vec<_>>(),
            [
                drives[0].identity(),
                drives[3].identity(),
                drives[4].identity(),
                readded.identity(),
            ]
        );
        assert_eq!(
            batch.islands[1]
                .iter()
                .map(|entry| entry.dense_ordinal)
                .collect::<Vec<_>>(),
            [1, 3]
        );
        assert_eq!(
            batch.islands[2]
                .iter()
                .map(|entry| entry.dense_ordinal)
                .collect::<Vec<_>>(),
            [0, 2]
        );
        assert_eq!(batch.islands.iter().map(Vec::len).sum::<usize>(), 4);
        for (island, entries) in batch.islands.iter().enumerate() {
            if island != 1 && island != 2 {
                assert!(entries.is_empty());
            }
            for entry in entries {
                let record = &batch.records[entry.dense_ordinal as usize];
                assert_eq!(
                    (entry.drive, entry.body_a, entry.body_b),
                    (record.drive, record.body_a, record.body_b)
                );
            }
        }
    }

    #[test]
    fn island_batch_numeric_failure_reports_dense_record_and_publishes_nothing() {
        let mut fixture = prepared_live_pool_fixture(2);
        let frame = fixture.body_pool.frame_identity().expect("frame");
        let map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &fixture.body_pool,
            frame,
            [
                (fixture.parent.observation(), 2),
                (fixture.child.observation(), 2),
            ],
        )
        .expect("map");
        let mut backend = FailAtNumericCall {
            fail_at: 32,
            calls: 0,
        };
        let result = try_prepare_active_drive_island_batch_for_test(
            &fixture.drive_pool,
            &mut fixture.body_pool,
            SimulationDriveIslandMapSource::Present(&map),
            &mut backend,
        );
        assert!(matches!(
            result,
            Err(SimulationDriveIslandBatchPreparationError::Numeric {
                ordinal: 1,
                source: 32,
                ..
            })
        ));
        assert_eq!(backend.calls, 33);
    }

    #[test]
    fn island_batch_preserves_two_distinct_numeric_records_and_island_ordinals() {
        let mut fixture = two_island_drive_fixture();
        let map = two_island_map(&fixture);
        let mut backend = SimulationDriveHostNumericBackend;
        let batch = try_prepare_active_drive_island_batch_for_test(
            &fixture.drive_pool,
            &mut fixture.body_pool,
            SimulationDriveIslandMapSource::Present(&map),
            &mut backend,
        )
        .expect("combined batch");
        let direct =
            try_prepare_active_drive_jacobian_batch(&fixture.drive_pool, &fixture.body_pool)
                .expect("direct batch");
        assert_eq!(
            batch
                .records
                .iter()
                .map(|prepared| drive_record_bits(&prepared.record))
                .collect::<Vec<_>>(),
            direct
                .records
                .iter()
                .map(|prepared| drive_record_bits(&prepared.record))
                .collect::<Vec<_>>()
        );
        let first = batch.records[0].record;
        let mut second = batch.records[1].record;
        second.solver_delta_a = first.solver_delta_a;
        second.solver_delta_b = first.solver_delta_b;
        second.drive_slot = first.drive_slot;
        assert_ne!(drive_record_bits(&first), drive_record_bits(&second));
        assert_eq!(batch.islands[1][0].dense_ordinal, 0);
        assert_eq!(batch.islands[3][0].dense_ordinal, 1);
    }

    #[test]
    fn island_batch_reserves_occupied_islands_in_numeric_order() {
        let mut fixture = two_island_drive_fixture();
        let map = two_island_map(&fixture);
        let mut backend = TracingHostNumericBackend::default();
        let mut reserve_order = Vec::new();
        let phase = fixture
            .body_pool
            .try_prepare_dynamic_update_phase()
            .expect("phase");
        let result = try_prepare_active_drive_island_batch_with_backend_and_reservation(
            &fixture.drive_pool,
            &phase,
            SimulationDriveIslandMapSource::Present(&map),
            &mut backend,
            |inputs, capacity| inputs.try_reserve_exact(capacity).map_err(|_| ()),
            reserve_drive_records,
            |island, entries, capacity| {
                reserve_order.push(island);
                entries.try_reserve_exact(capacity).map_err(|_| ())?;
                if island == 3 { Err(()) } else { Ok(()) }
            },
        );
        assert!(matches!(
            result,
            Err(
                SimulationDriveIslandBatchPreparationError::IslandAllocationFailed {
                    island: 3,
                    capacity: 1,
                }
            )
        ));
        assert_eq!(reserve_order, [1, 3]);
        assert!(backend.calls.is_empty());
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "all three injected allocation boundaries remain visible in one chronology test"
    )]
    fn island_batch_reserves_every_destination_before_numeric_calls() {
        let mut fixture = prepared_live_pool_fixture(1);
        let frame = fixture.body_pool.frame_identity().expect("frame");
        let map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &fixture.body_pool,
            frame,
            [
                (fixture.parent.observation(), 2),
                (fixture.child.observation(), 2),
            ],
        )
        .expect("map");

        for dishonest_success in [false, true] {
            let mut backend = TracingHostNumericBackend::default();
            let phase = fixture
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("phase");
            let result = try_prepare_active_drive_island_batch_with_backend_and_reservation(
                &fixture.drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut backend,
                |inputs, capacity| {
                    if dishonest_success {
                        Ok(())
                    } else {
                        inputs.try_reserve_exact(capacity).map_err(|_| ())?;
                        Err(())
                    }
                },
                |_, _| panic!("record reservation must not follow staging failure"),
                |_, _, _| panic!("island reservation must not follow staging failure"),
            );
            assert!(matches!(
                result,
                Err(
                    SimulationDriveIslandBatchPreparationError::StagingAllocationFailed {
                        capacity: 1,
                    }
                )
            ));
            assert!(backend.calls.is_empty());
        }

        for dishonest_success in [false, true] {
            let mut backend = TracingHostNumericBackend::default();
            let phase = fixture
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("phase");
            let result = try_prepare_active_drive_island_batch_with_backend_and_reservation(
                &fixture.drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut backend,
                |inputs, capacity| inputs.try_reserve_exact(capacity).map_err(|_| ()),
                |records, capacity| {
                    if dishonest_success {
                        Ok(())
                    } else {
                        records.try_reserve_exact(capacity).map_err(|_| ())?;
                        Err(())
                    }
                },
                |_, _, _| panic!("island reservation must not follow record failure"),
            );
            assert!(matches!(
                result,
                Err(
                    SimulationDriveIslandBatchPreparationError::RecordAllocationFailed {
                        capacity: 1,
                    }
                )
            ));
            assert!(backend.calls.is_empty());
        }

        for dishonest_success in [false, true] {
            let mut backend = TracingHostNumericBackend::default();
            let phase = fixture
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("phase");
            let result = try_prepare_active_drive_island_batch_with_backend_and_reservation(
                &fixture.drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut backend,
                |inputs, capacity| inputs.try_reserve_exact(capacity).map_err(|_| ()),
                reserve_drive_records,
                |island, entries, capacity| {
                    assert_eq!(island, 2);
                    if dishonest_success {
                        Ok(())
                    } else {
                        entries.try_reserve_exact(capacity).map_err(|_| ())?;
                        Err(())
                    }
                },
            );
            assert!(matches!(
                result,
                Err(
                    SimulationDriveIslandBatchPreparationError::IslandAllocationFailed {
                        island: 2,
                        capacity: 1,
                    }
                )
            ));
            assert!(backend.calls.is_empty());
        }
    }

    #[test]
    fn one_island_drive_sweep_matches_the_common_checked_value_oracle() {
        let inertia = std::sync::Arc::new(
            crate::simulation_rigid_body_pool::SimulationInertia::native_default(),
        );
        let mut body_pool = SimulationRigidBodyPool::try_new(2).expect("body pool");
        let child = body_pool
            .try_add_rigid_body(identity_matrix(), Some(&inertia), 4)
            .expect("child Add")
            .expect("child capacity");
        let parent = body_pool
            .try_add_rigid_body(identity_matrix(), Some(&inertia), 4)
            .expect("parent Add")
            .expect("parent capacity");
        let input = numeric_backend_input(HARD_DRIVE, SOFT_DRIVE, 4, 4);
        body_pool.begin_dynamic_update_frame(input.time_step);
        let frames = std::sync::Arc::new(std::sync::RwLock::new(input.frames));
        let dynamics = std::sync::Arc::new(std::sync::RwLock::new(input.dynamics));
        let mut drive_pool = SimulationDrivePool::try_new(1, &body_pool).expect("Drive pool");
        drive_pool
            .try_add_drive(&body_pool, parent, child, &frames, &dynamics)
            .expect("Drive Add")
            .expect("Drive capacity");
        for (body, seed) in [(child, 1.0), (parent, 17.0)] {
            body_pool
                .try_with_solver_delta_row_mut(body.observation(), |row| {
                    for (ordinal, lane) in (0_u8..).zip(row.iter_mut().flatten()) {
                        *lane = seed + f32::from(ordinal);
                    }
                })
                .expect("seed RF");
        }

        let frame = body_pool.frame_identity().expect("frame");
        let map = SimulationRigidBodyIslandMapSnapshot::try_new(
            &body_pool,
            frame,
            [(parent.observation(), 2), (child.observation(), 9)],
        )
        .expect("map");
        let child_before = body_pool
            .try_with_solver_delta_row(child.observation(), |row| *row)
            .expect("child RF");
        let parent_before = body_pool
            .try_with_solver_delta_row(parent.observation(), |row| *row)
            .expect("parent RF");
        let (batch, expected_record, expected) = {
            let mut backend = SimulationDriveHostNumericBackend;
            let mut phase = body_pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut batch = try_prepare_active_drive_island_batch_with_backend(
                &drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut backend,
            )
            .expect("island batch");
            assert_eq!(batch.islands[2].len(), 1);
            let mut expected_record = batch.records[0].record;
            let expected = solve_drive_horus_island_record_host(
                &mut expected_record,
                child_before,
                parent_before,
            );
            try_solve_prepared_drive_horus_island_sweep(&mut batch, 2, &drive_pool, &mut phase)
                .expect("one island sweep");
            (batch, expected_record, expected)
        };

        assert!(batch.records[0].record == expected_record);
        assert_eq!(
            body_pool
                .try_with_solver_delta_row(child.observation(), |row| bits_rows(*row))
                .expect("child result"),
            bits_rows([
                expected.solver_delta_a[0],
                child_before[1],
                expected.solver_delta_a[2],
                child_before[3],
            ])
        );
        assert_eq!(
            body_pool
                .try_with_solver_delta_row(parent.observation(), |row| bits_rows(*row))
                .expect("parent result"),
            bits_rows([
                expected.solver_delta_b[0],
                parent_before[1],
                expected.solver_delta_b[2],
                parent_before[3],
            ])
        );
    }

    #[test]
    fn island_batch_failure_drop_and_repeated_sweeps_reuse_one_phase() {
        let mut fixture = prepared_live_island_fixture(1, 2);
        seed_prepared_live_island_rf(&mut fixture);
        let map = prepared_live_island_map(&fixture, 2);
        let mut expected_a = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.child.observation(), |row| *row)
            .expect("child RF");
        let mut expected_b = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.parent.observation(), |row| *row)
            .expect("parent RF");

        let mut phase = fixture
            .body_pool
            .try_prepare_dynamic_update_phase()
            .expect("phase");
        let mut failing_backend = FailAtNumericCall {
            fail_at: 0,
            calls: 0,
        };
        assert!(matches!(
            try_prepare_active_drive_island_batch_with_backend(
                &fixture.drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut failing_backend,
            ),
            Err(SimulationDriveIslandBatchPreparationError::Numeric {
                ordinal: 0,
                source: 0,
                ..
            })
        ));
        assert_eq!(failing_backend.calls, 1);

        let dropped = try_prepare_active_drive_island_batch_with_backend(
            &fixture.drive_pool,
            &phase,
            SimulationDriveIslandMapSource::Present(&map),
            &mut SimulationDriveHostNumericBackend,
        )
        .expect("retry batch");
        drop(dropped);

        let mut batch = try_prepare_active_drive_island_batch_with_backend(
            &fixture.drive_pool,
            &phase,
            SimulationDriveIslandMapSource::Present(&map),
            &mut SimulationDriveHostNumericBackend,
        )
        .expect("live batch");
        let mut expected_record = batch.records[0].record;
        for _ in 0..2 {
            let solved =
                solve_drive_horus_island_record_host(&mut expected_record, expected_a, expected_b);
            expected_a[0] = solved.solver_delta_a[0];
            expected_b[0] = solved.solver_delta_b[0];
            expected_a[2] = solved.solver_delta_a[2];
            expected_b[2] = solved.solver_delta_b[2];
            try_solve_prepared_drive_horus_island_sweep(
                &mut batch,
                2,
                &fixture.drive_pool,
                &mut phase,
            )
            .expect("same-phase sweep");
        }
        drop(phase);

        assert!(batch.records[0].record == expected_record);
        assert_eq!(
            fixture
                .body_pool
                .try_with_solver_delta_row(fixture.child.observation(), |row| *row)
                .expect("child result"),
            expected_a
        );
        assert_eq!(
            fixture
                .body_pool
                .try_with_solver_delta_row(fixture.parent.observation(), |row| *row)
                .expect("parent result"),
            expected_b
        );
    }

    #[test]
    fn island_batch_rejects_same_frame_phase_after_dynamic_update_consumption() {
        let mut fixture = prepared_live_island_fixture(1, 2);
        seed_prepared_live_island_rf(&mut fixture);
        let map = prepared_live_island_map(&fixture, 2);
        let body_before_update = fixture
            .body_pool
            .try_drive_build_snapshot(fixture.child.observation())
            .expect("body before DynamicUpdate");
        let mut batch = {
            let phase = fixture
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("preparation phase");
            let batch = try_prepare_active_drive_island_batch_with_backend(
                &fixture.drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut SimulationDriveHostNumericBackend,
            )
            .expect("prepared batch");
            phase.integrate_ordinary_finite();
            batch
        };
        let body_after_update = fixture
            .body_pool
            .try_drive_build_snapshot(fixture.child.observation())
            .expect("body after DynamicUpdate");
        assert_ne!(
            body_after_update.velocity.map(f32::to_bits),
            body_before_update.velocity.map(f32::to_bits),
            "consumed phase must make the retained Drive record numerically stale"
        );
        let before_records = batch
            .records
            .iter()
            .map(|prepared| drive_record_bits(&prepared.record))
            .collect::<Vec<_>>();
        let before_child = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.child.observation(), |row| bits_rows(*row))
            .expect("child RF after integration");
        let before_parent = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.parent.observation(), |row| bits_rows(*row))
            .expect("parent RF after integration");

        {
            let mut replacement = fixture
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("replacement phase");
            assert_eq!(
                try_solve_prepared_drive_horus_island_sweep(
                    &mut batch,
                    2,
                    &fixture.drive_pool,
                    &mut replacement,
                ),
                Err(SimulationPreparedDriveIslandHorusError::PhaseMismatch)
            );
        }
        assert_eq!(
            batch
                .records
                .iter()
                .map(|prepared| drive_record_bits(&prepared.record))
                .collect::<Vec<_>>(),
            before_records
        );
        assert_eq!(
            fixture
                .body_pool
                .try_with_solver_delta_row(fixture.child.observation(), |row| bits_rows(*row))
                .expect("unchanged child RF"),
            before_child
        );
        assert_eq!(
            fixture
                .body_pool
                .try_with_solver_delta_row(fixture.parent.observation(), |row| bits_rows(*row))
                .expect("unchanged parent RF"),
            before_parent
        );
    }

    #[test]
    fn island_batch_revalidates_drive_removal_inside_the_matching_phase() {
        let mut fixture = prepared_live_island_fixture(1, 2);
        seed_prepared_live_island_rf(&mut fixture);
        let map = prepared_live_island_map(&fixture, 2);
        let before_child = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.child.observation(), |row| bits_rows(*row))
            .expect("child RF");
        let before_parent = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.parent.observation(), |row| bits_rows(*row))
            .expect("parent RF");

        let mut phase = fixture
            .body_pool
            .try_prepare_dynamic_update_phase()
            .expect("phase");
        let mut batch = try_prepare_active_drive_island_batch_with_backend(
            &fixture.drive_pool,
            &phase,
            SimulationDriveIslandMapSource::Present(&map),
            &mut SimulationDriveHostNumericBackend,
        )
        .expect("batch");
        let before_record = drive_record_bits(&batch.records[0].record);
        fixture
            .drive_pool
            .try_remove_drive(fixture.drives[0])
            .expect("remove Drive during body phase");
        assert!(matches!(
            try_solve_prepared_drive_horus_island_sweep(
                &mut batch,
                2,
                &fixture.drive_pool,
                &mut phase,
            ),
            Err(SimulationPreparedDriveIslandHorusError::Drive(
                SimulationDriveBuildBindingValidationError::Binding {
                    ordinal: 0,
                    source: SimulationDrivePoolError::DriveIsFree { slot: 0 },
                    ..
                }
            ))
        ));
        assert_eq!(drive_record_bits(&batch.records[0].record), before_record);
        drop(phase);
        assert_eq!(
            fixture
                .body_pool
                .try_with_solver_delta_row(fixture.child.observation(), |row| bits_rows(*row))
                .expect("unchanged child RF"),
            before_child
        );
        assert_eq!(
            fixture
                .body_pool
                .try_with_solver_delta_row(fixture.parent.observation(), |row| bits_rows(*row))
                .expect("unchanged parent RF"),
            before_parent
        );
    }

    #[test]
    fn island_sweep_binds_frame_and_sources_even_when_the_table_is_empty() {
        let mut body = SimulationRigidBodyPool::try_new(0).expect("body pool");
        let drive = SimulationDrivePool::try_new(0, &body).expect("Drive pool");
        let frame = body.frame_identity().expect("frame");
        let mut batch = {
            let mut backend = SimulationDriveHostNumericBackend;
            let phase = body
                .try_prepare_dynamic_update_phase()
                .expect("preparation phase");
            try_prepare_active_drive_island_batch_with_backend(
                &drive,
                &phase,
                SimulationDriveIslandMapSource::Null { frame },
                &mut backend,
            )
            .expect("empty island batch")
        };

        let foreign_drive = SimulationDrivePool::try_new(0, &body).expect("foreign Drive pool");
        {
            let mut phase = body
                .try_prepare_dynamic_update_phase()
                .expect("same-frame replacement phase");
            assert_eq!(
                try_solve_prepared_drive_horus_island_sweep(&mut batch, 0, &drive, &mut phase,),
                Err(SimulationPreparedDriveIslandHorusError::PhaseMismatch)
            );
        }
        {
            let mut phase = body.try_prepare_dynamic_update_phase().expect("phase");
            assert_eq!(
                try_solve_prepared_drive_horus_island_sweep(
                    &mut batch,
                    0,
                    &foreign_drive,
                    &mut phase,
                ),
                Err(SimulationPreparedDriveIslandHorusError::DrivePoolSourceMismatch)
            );
        }

        let mut foreign_body = SimulationRigidBodyPool::try_new(0).expect("foreign body pool");
        {
            let mut phase = foreign_body
                .try_prepare_dynamic_update_phase()
                .expect("foreign phase");
            assert_eq!(
                try_solve_prepared_drive_horus_island_sweep(&mut batch, 0, &drive, &mut phase,),
                Err(SimulationPreparedDriveIslandHorusError::BodyPoolSourceMismatch)
            );
        }

        body.begin_dynamic_update_frame(0.125);
        {
            let mut phase = body.try_prepare_dynamic_update_phase().expect("new phase");
            assert_eq!(
                try_solve_prepared_drive_horus_island_sweep(&mut batch, 0, &drive, &mut phase,),
                Err(SimulationPreparedDriveIslandHorusError::FrameMismatch)
            );
        }
        assert!(batch.records.is_empty());
        assert!(batch.islands.iter().all(Vec::is_empty));
    }

    #[test]
    fn island_sweep_rejects_nonempty_old_frame_without_record_rf_or_body_effects() {
        let mut fixture = prepared_live_island_fixture(2, 2);
        seed_prepared_live_island_rf(&mut fixture);
        let timestep = fixture.body_pool.timestep();
        fixture.body_pool.begin_dynamic_update_frame(timestep);
        let before =
            after_one_dynamic_update_phase_identity_issue(prepared_live_island_effect(&fixture));
        {
            let mut phase = fixture
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("new phase");
            assert_eq!(
                try_solve_prepared_drive_horus_island_sweep(
                    &mut fixture.batch,
                    2,
                    &fixture.drive_pool,
                    &mut phase,
                ),
                Err(SimulationPreparedDriveIslandHorusError::FrameMismatch)
            );
        }
        assert_eq!(prepared_live_island_effect(&fixture), before);
    }

    #[test]
    fn island_sweep_rejects_later_sidecar_and_token_corruption_without_effects() {
        let mut bad_ordinal = prepared_live_island_fixture(2, 2);
        seed_prepared_live_island_rf(&mut bad_ordinal);
        let map = prepared_live_island_map(&bad_ordinal, 2);
        let before = after_one_dynamic_update_phase_identity_issue(prepared_live_island_effect(
            &bad_ordinal,
        ));
        {
            let mut phase = bad_ordinal
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("phase");
            bad_ordinal.batch = try_prepare_active_drive_island_batch_with_backend(
                &bad_ordinal.drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut SimulationDriveHostNumericBackend,
            )
            .expect("matching batch");
            bad_ordinal.batch.islands[2][1].dense_ordinal = u32::MAX;
            assert_eq!(
                try_solve_prepared_drive_horus_island_sweep(
                    &mut bad_ordinal.batch,
                    2,
                    &bad_ordinal.drive_pool,
                    &mut phase,
                ),
                Err(
                    SimulationPreparedDriveIslandHorusError::TableEntryMismatch {
                        island_ordinal: 1,
                        dense_ordinal: u32::MAX,
                    }
                )
            );
        }
        assert_eq!(prepared_live_island_effect(&bad_ordinal), before);

        let mut bad_token = prepared_live_island_fixture(2, 2);
        seed_prepared_live_island_rf(&mut bad_token);
        let map = prepared_live_island_map(&bad_token, 2);
        let mut before =
            after_one_dynamic_update_phase_identity_issue(prepared_live_island_effect(&bad_token));
        before.0[1][12][3] = 0;
        {
            let mut phase = bad_token
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("phase");
            bad_token.batch = try_prepare_active_drive_island_batch_with_backend(
                &bad_token.drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut SimulationDriveHostNumericBackend,
            )
            .expect("matching batch");
            bad_token.batch.records[1].record.drive_slot = DriveSlotToken(0);
            assert!(matches!(
                try_solve_prepared_drive_horus_island_sweep(
                    &mut bad_token.batch,
                    2,
                    &bad_token.drive_pool,
                    &mut phase,
                ),
                Err(SimulationPreparedDriveIslandHorusError::BindingMismatch {
                    island_ordinal: 1,
                    dense_ordinal: 1,
                    kind: SimulationDrivePreparedBindingKind::Drive,
                    actual: 0,
                    ..
                })
            ));
        }
        assert_eq!(prepared_live_island_effect(&bad_token), before);
    }

    #[test]
    fn island_sweep_rejects_replacement_phase_before_drive_or_body_aba_checks() {
        let mut stale_drive = prepared_live_island_fixture(2, 2);
        seed_prepared_live_island_rf(&mut stale_drive);
        stale_drive
            .drive_pool
            .try_remove_drive(stale_drive.drives[1])
            .expect("remove second Drive");
        let reused = stale_drive
            .drive_pool
            .try_add_drive(
                &stale_drive.body_pool,
                stale_drive.parent,
                stale_drive.child,
                &stale_drive.frames,
                &stale_drive.dynamics,
            )
            .expect("re-add Drive")
            .expect("Drive capacity");
        assert_eq!(reused.id(), stale_drive.drives[1].id());
        let before = after_one_dynamic_update_phase_identity_issue(prepared_live_island_effect(
            &stale_drive,
        ));
        {
            let mut phase = stale_drive
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("phase");
            assert_eq!(
                try_solve_prepared_drive_horus_island_sweep(
                    &mut stale_drive.batch,
                    2,
                    &stale_drive.drive_pool,
                    &mut phase,
                ),
                Err(SimulationPreparedDriveIslandHorusError::PhaseMismatch)
            );
        }
        assert_eq!(prepared_live_island_effect(&stale_drive), before);

        let mut stale_body = prepared_live_island_fixture(1, 2);
        stale_body
            .body_pool
            .try_remove_rigid_body(stale_body.child)
            .expect("remove child");
        let inertia = std::sync::Arc::new(
            crate::simulation_rigid_body_pool::SimulationInertia::native_default(),
        );
        let reused_child = stale_body
            .body_pool
            .try_add_rigid_body(identity_matrix(), Some(&inertia), 4)
            .expect("re-add child")
            .expect("body capacity");
        assert_eq!(reused_child.id(), stale_body.child.id());
        stale_body.child = reused_child;
        seed_prepared_live_island_rf(&mut stale_body);
        let before =
            after_one_dynamic_update_phase_identity_issue(prepared_live_island_effect(&stale_body));
        {
            let mut phase = stale_body
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("phase");
            assert_eq!(
                try_solve_prepared_drive_horus_island_sweep(
                    &mut stale_body.batch,
                    2,
                    &stale_body.drive_pool,
                    &mut phase,
                ),
                Err(SimulationPreparedDriveIslandHorusError::PhaseMismatch)
            );
        }
        assert_eq!(prepared_live_island_effect(&stale_body), before);
    }

    #[test]
    fn island_sweep_observes_position_then_angular_and_first_store_panic_boundary() {
        let mut fixture = prepared_live_island_fixture(1, 2);
        seed_prepared_live_island_rf(&mut fixture);
        let map = prepared_live_island_map(&fixture, 2);
        let old_position = bits3(fixture.batch.records[0].record.position_impulse);
        let old_angular = bits3(fixture.batch.records[0].record.angular_impulse);
        let mut stores = Vec::new();
        {
            let mut phase = fixture
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("phase");
            fixture.batch = try_prepare_active_drive_island_batch_with_backend(
                &fixture.drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut SimulationDriveHostNumericBackend,
            )
            .expect("matching batch");
            try_solve_prepared_drive_horus_island_sweep_with_observer(
                &mut fixture.batch,
                2,
                &fixture.drive_pool,
                &mut phase,
                |ordinal, accumulator, record| {
                    stores.push((
                        ordinal,
                        accumulator,
                        bits3(record.position_impulse),
                        bits3(record.angular_impulse),
                    ));
                },
            )
            .expect("observed island sweep");
        }
        let final_position = bits3(fixture.batch.records[0].record.position_impulse);
        let final_angular = bits3(fixture.batch.records[0].record.angular_impulse);
        assert_ne!(final_position, old_position);
        assert_eq!(stores.len(), 2);
        assert_eq!(stores[0].0, 0);
        assert_eq!(stores[0].1, SimulationDriveHorusAccumulator::Position);
        assert_eq!(stores[0].2, final_position);
        assert_eq!(stores[0].3, old_angular);
        assert_eq!(stores[1].0, 0);
        assert_eq!(stores[1].1, SimulationDriveHorusAccumulator::Angular);
        assert_eq!(stores[1].2, final_position);
        assert_eq!(stores[1].3, final_angular);

        let mut panic_fixture = prepared_live_island_fixture(1, 2);
        seed_prepared_live_island_rf(&mut panic_fixture);
        let map = prepared_live_island_map(&panic_fixture, 2);
        let before = after_one_dynamic_update_phase_identity_issue(prepared_live_island_effect(
            &panic_fixture,
        ));
        let old_position = bits3(panic_fixture.batch.records[0].record.position_impulse);
        let old_angular = bits3(panic_fixture.batch.records[0].record.angular_impulse);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut phase = panic_fixture
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("phase");
            panic_fixture.batch = try_prepare_active_drive_island_batch_with_backend(
                &panic_fixture.drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut SimulationDriveHostNumericBackend,
            )
            .expect("matching batch");
            let _ = try_solve_prepared_drive_horus_island_sweep_with_observer(
                &mut panic_fixture.batch,
                2,
                &panic_fixture.drive_pool,
                &mut phase,
                |ordinal, accumulator, _| {
                    assert_eq!(ordinal, 0);
                    assert_eq!(accumulator, SimulationDriveHorusAccumulator::Position);
                    panic!("stop after first accumulator store");
                },
            );
        }));
        assert!(panic.is_err());
        assert_ne!(
            bits3(panic_fixture.batch.records[0].record.position_impulse),
            old_position
        );
        assert_eq!(
            bits3(panic_fixture.batch.records[0].record.angular_impulse),
            old_angular
        );
        assert_eq!(
            prepared_live_island_effect(&panic_fixture).1,
            before.1,
            "panic precedes every body-A RF store"
        );
        assert_eq!(
            prepared_live_island_effect(&panic_fixture).2,
            before.2,
            "panic precedes every body-B RF store"
        );
        assert_eq!(prepared_live_island_effect(&panic_fixture).3, before.3);
    }

    #[test]
    fn island_sweep_keeps_table_order_duplicates_shared_history_and_other_islands_inert() {
        let mut fixture = prepared_live_island_fixture(3, 2);
        seed_prepared_live_island_rf(&mut fixture);
        let map = prepared_live_island_map(&fixture, 2);
        let original = fixture.batch.islands[2].clone();
        fixture.batch.islands[2] = vec![original[2], original[0], original[2]];
        fixture.batch.islands[3] = vec![original[1]];

        let mut expected_records = fixture
            .batch
            .records
            .iter()
            .map(|prepared| prepared.record)
            .collect::<Vec<_>>();
        let mut expected_a = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.child.observation(), |row| *row)
            .expect("child RF");
        let mut expected_b = fixture
            .body_pool
            .try_with_solver_delta_row(fixture.parent.observation(), |row| *row)
            .expect("parent RF");
        for index in [2_usize, 0, 2] {
            let solved = solve_drive_horus_island_record_host(
                &mut expected_records[index],
                expected_a,
                expected_b,
            );
            expected_a[0] = solved.solver_delta_a[0];
            expected_b[0] = solved.solver_delta_b[0];
            expected_a[2] = solved.solver_delta_a[2];
            expected_b[2] = solved.solver_delta_b[2];
        }

        let mut position_stores = Vec::new();
        {
            let mut phase = fixture
                .body_pool
                .try_prepare_dynamic_update_phase()
                .expect("phase");
            fixture.batch = try_prepare_active_drive_island_batch_with_backend(
                &fixture.drive_pool,
                &phase,
                SimulationDriveIslandMapSource::Present(&map),
                &mut SimulationDriveHostNumericBackend,
            )
            .expect("matching batch");
            fixture.batch.islands[2] = vec![original[2], original[0], original[2]];
            fixture.batch.islands[3] = vec![original[1]];
            try_solve_prepared_drive_horus_island_sweep_with_observer(
                &mut fixture.batch,
                2,
                &fixture.drive_pool,
                &mut phase,
                |ordinal, accumulator, record| {
                    if accumulator == SimulationDriveHorusAccumulator::Position {
                        position_stores.push((ordinal, record.drive_slot.0));
                    }
                },
            )
            .expect("ordered duplicate sweep");
        }

        assert_eq!(position_stores, [(0, 3), (1, 1), (2, 3)]);
        assert_eq!(
            fixture
                .batch
                .records
                .iter()
                .map(|prepared| drive_record_bits(&prepared.record))
                .collect::<Vec<_>>(),
            expected_records
                .iter()
                .map(drive_record_bits)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            fixture
                .body_pool
                .try_with_solver_delta_row(fixture.child.observation(), |row| bits_rows(*row))
                .expect("child RF result"),
            bits_rows(expected_a)
        );
        assert_eq!(
            fixture
                .body_pool
                .try_with_solver_delta_row(fixture.parent.observation(), |row| bits_rows(*row))
                .expect("parent RF result"),
            bits_rows(expected_b)
        );
    }

    #[test]
    fn native_layout_freezes_every_pdb_offset() {
        assert_eq!(size_of::<SimulationDriveJacobian>(), 0x180);
        assert_eq!(align_of::<SimulationDriveJacobian>(), 0x10);
        assert_eq!(size_of::<SolverDeltaRowToken>(), 4);
        assert_eq!(size_of::<SimulationDriveJacobianRqd>(), 0x40);
        assert_eq!(align_of::<SimulationDriveJacobianRqd>(), 0x10);

        let offsets = [
            offset_of!(SimulationDriveJacobian, attachment_offset_a),
            offset_of!(SimulationDriveJacobian, solver_delta_a),
            offset_of!(SimulationDriveJacobian, attachment_offset_b),
            offset_of!(SimulationDriveJacobian, solver_delta_b),
            offset_of!(SimulationDriveJacobian, position_impulse),
            offset_of!(SimulationDriveJacobian, linear_dynamics),
            offset_of!(SimulationDriveJacobian, angular_impulse),
            offset_of!(SimulationDriveJacobian, angular_dynamics),
            offset_of!(SimulationDriveJacobian, linear_transpose_right),
            offset_of!(SimulationDriveJacobian, spy),
            offset_of!(SimulationDriveJacobian, angular_transpose_right),
            offset_of!(SimulationDriveJacobian, pad_0),
            offset_of!(SimulationDriveJacobian, linear_transpose_up),
            offset_of!(SimulationDriveJacobian, position_limit_x),
            offset_of!(SimulationDriveJacobian, angular_transpose_up),
            offset_of!(SimulationDriveJacobian, position_limit_y),
            offset_of!(SimulationDriveJacobian, linear_transpose_at),
            offset_of!(SimulationDriveJacobian, angular_limit_x),
            offset_of!(SimulationDriveJacobian, angular_transpose_at),
            offset_of!(SimulationDriveJacobian, angular_limit_y),
            offset_of!(SimulationDriveJacobian, position_offset),
            offset_of!(SimulationDriveJacobian, position_limit_z),
            offset_of!(SimulationDriveJacobian, angular_offset),
            offset_of!(SimulationDriveJacobian, angular_limit_z),
            offset_of!(SimulationDriveJacobian, linear_right),
            offset_of!(SimulationDriveJacobian, drive_slot),
            offset_of!(SimulationDriveJacobian, linear_up),
            offset_of!(SimulationDriveJacobian, pad_1),
            offset_of!(SimulationDriveJacobian, linear_at),
            offset_of!(SimulationDriveJacobian, pad_2),
            offset_of!(SimulationDriveJacobian, angular_right),
            offset_of!(SimulationDriveJacobian, pad_3),
            offset_of!(SimulationDriveJacobian, angular_up),
            offset_of!(SimulationDriveJacobian, pad_4),
            offset_of!(SimulationDriveJacobian, angular_at),
            offset_of!(SimulationDriveJacobian, pad_5),
            offset_of!(SimulationDriveJacobian, inverse_inertia_a_right),
            offset_of!(SimulationDriveJacobian, inverse_mass_a),
            offset_of!(SimulationDriveJacobian, inverse_inertia_a_up),
            offset_of!(SimulationDriveJacobian, pad_6),
            offset_of!(SimulationDriveJacobian, inverse_inertia_a_at),
            offset_of!(SimulationDriveJacobian, pad_7),
            offset_of!(SimulationDriveJacobian, inverse_inertia_b_right),
            offset_of!(SimulationDriveJacobian, inverse_mass_b),
            offset_of!(SimulationDriveJacobian, inverse_inertia_b_up),
            offset_of!(SimulationDriveJacobian, pad_8),
            offset_of!(SimulationDriveJacobian, inverse_inertia_b_at),
            offset_of!(SimulationDriveJacobian, pad_9),
        ];
        assert_eq!(
            offsets,
            [
                0x00, 0x0c, 0x10, 0x1c, 0x20, 0x2c, 0x30, 0x3c, 0x40, 0x4c, 0x50, 0x5c, 0x60, 0x6c,
                0x70, 0x7c, 0x80, 0x8c, 0x90, 0x9c, 0xa0, 0xac, 0xb0, 0xbc, 0xc0, 0xcc, 0xd0, 0xdc,
                0xe0, 0xec, 0xf0, 0xfc, 0x100, 0x10c, 0x110, 0x11c, 0x120, 0x12c, 0x130, 0x13c,
                0x140, 0x14c, 0x150, 0x15c, 0x160, 0x16c, 0x170, 0x17c,
            ]
        );
    }

    #[test]
    fn zeroed_record_has_positive_zero_impulses_pads_and_private_tokens() {
        let value = SimulationDriveJacobian::zeroed();
        assert_eq!(value.position_impulse.map(f32::to_bits), [0; 3]);
        assert_eq!(value.angular_impulse.map(f32::to_bits), [0; 3]);
        assert_eq!(value.solver_delta_a, SolverDeltaRowToken(0));
        assert_eq!(value.solver_delta_b, SolverDeltaRowToken(0));
        assert_eq!(value.drive_slot, DriveSlotToken(0));
        assert!(!value.has_solver_delta_bindings());
        assert_eq!(
            [
                value.pad_0.to_bits(),
                value.pad_1.to_bits(),
                value.pad_2.to_bits(),
                value.pad_3.to_bits(),
                value.pad_4.to_bits(),
                value.pad_5.to_bits(),
                value.pad_6.to_bits(),
                value.pad_7.to_bits(),
                value.pad_8.to_bits(),
                value.pad_9.to_bits(),
            ],
            [0; 10]
        );
        assert!(value == SimulationDriveJacobian::default());
    }

    #[test]
    fn horus_record_matches_full_fused_packed_lane_oracle() {
        let (mut jacobian, solver_delta_a, solver_delta_b) = horus_fixture();
        let before = jacobian;
        let output = solve_drive_horus_record_host(&mut jacobian, solver_delta_a, solver_delta_b);

        let mut expected_jacobian = before;
        expected_jacobian.position_impulse = [
            f32::from_bits(0x3e9e_b852),
            f32::from_bits(0xbe8a_3d71),
            f32::from_bits(0x3e42_8f5c),
        ];
        expected_jacobian.angular_impulse = [
            f32::from_bits(0xbe61_47ae),
            f32::from_bits(0x3e38_51ec),
            f32::from_bits(0xbe85_1eb8),
        ];
        assert!(jacobian == expected_jacobian);
        assert_eq!(
            bits_rows(output.solver_delta_a),
            [
                [0x3ea4_a8c1, 0xbf04_b5dd, 0x3f39_652c, 0x3f80_a138],
                [0x4130_0000, 0x4140_0000, 0x4150_0000, 0x4160_0000],
                [0xbe71_e797, 0x3f17_62a6, 0xbeab_1d2e, 0x3f49_877b],
                [0x41f8_0000, 0x4200_0000, 0x4204_0000, 0x4208_0000],
            ]
        );
        assert_eq!(
            bits_rows(output.solver_delta_b),
            [
                [0xbed0_96bc, 0x3f20_aa65, 0xbf5c_7e28, 0x3f98_a7c6],
                [0x4224_0000, 0x4228_0000, 0x422c_0000, 0x4230_0000],
                [0x3eb0_4f2b, 0xbea2_31e7, 0x3f18_ffa7, 0xbf22_3e18],
                [0x4274_0000, 0x4278_0000, 0x427c_0000, 0x4280_0000],
            ]
        );

        // Native Drive Horus never consumes or stores d1/d3.
        assert_eq!(bits4(output.solver_delta_a[1]), bits4(solver_delta_a[1]));
        assert_eq!(bits4(output.solver_delta_a[3]), bits4(solver_delta_a[3]));
        assert_eq!(bits4(output.solver_delta_b[1]), bits4(solver_delta_b[1]));
        assert_eq!(bits4(output.solver_delta_b[3]), bits4(solver_delta_b[3]));
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "both native store orders and first-store panic states form one mutation oracle"
    )]
    fn direct_and_island_horus_freeze_opposite_accumulator_store_chronology() {
        let (original, solver_delta_a, solver_delta_b) = horus_fixture();
        let mut expected_record = original;
        let expected_rows =
            solve_drive_horus_record_host(&mut expected_record, solver_delta_a, solver_delta_b);

        for (order, expected_kinds) in [
            (
                SimulationDriveHorusAccumulatorStoreOrder::AngularThenPosition,
                [
                    SimulationDriveHorusAccumulator::Angular,
                    SimulationDriveHorusAccumulator::Position,
                ],
            ),
            (
                SimulationDriveHorusAccumulatorStoreOrder::PositionThenAngular,
                [
                    SimulationDriveHorusAccumulator::Position,
                    SimulationDriveHorusAccumulator::Angular,
                ],
            ),
        ] {
            let mut record = original;
            let mut stores = Vec::new();
            let rows = solve_drive_horus_record_with_order(
                &mut record,
                solver_delta_a,
                solver_delta_b,
                order,
                |kind, record| stores.push((kind, *record)),
            );

            assert_eq!(stores.len(), 2);
            assert_eq!([stores[0].0, stores[1].0], expected_kinds);
            assert_eq!(
                stores[0].1.linear_dynamics.to_bits(),
                original.linear_dynamics.to_bits()
            );
            assert_eq!(
                stores[0].1.angular_dynamics.to_bits(),
                original.angular_dynamics.to_bits()
            );
            assert_eq!(
                stores[1].1.linear_dynamics.to_bits(),
                original.linear_dynamics.to_bits()
            );
            assert_eq!(
                stores[1].1.angular_dynamics.to_bits(),
                original.angular_dynamics.to_bits()
            );
            match order {
                SimulationDriveHorusAccumulatorStoreOrder::AngularThenPosition => {
                    assert_eq!(
                        bits3(stores[0].1.angular_impulse),
                        bits3(expected_record.angular_impulse)
                    );
                    assert_eq!(
                        bits3(stores[0].1.position_impulse),
                        bits3(original.position_impulse)
                    );
                }
                SimulationDriveHorusAccumulatorStoreOrder::PositionThenAngular => {
                    assert_eq!(
                        bits3(stores[0].1.position_impulse),
                        bits3(expected_record.position_impulse)
                    );
                    assert_eq!(
                        bits3(stores[0].1.angular_impulse),
                        bits3(original.angular_impulse)
                    );
                }
            }
            assert!(stores[1].1 == expected_record);
            assert!(record == expected_record);
            assert_eq!(rows, expected_rows);
        }

        for order in [
            SimulationDriveHorusAccumulatorStoreOrder::AngularThenPosition,
            SimulationDriveHorusAccumulatorStoreOrder::PositionThenAngular,
        ] {
            let mut record = original;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                solve_drive_horus_record_with_order(
                    &mut record,
                    solver_delta_a,
                    solver_delta_b,
                    order,
                    |_, _| panic!("stop after first accumulator store"),
                );
            }));
            assert!(result.is_err());
            match order {
                SimulationDriveHorusAccumulatorStoreOrder::AngularThenPosition => {
                    assert_eq!(
                        bits3(record.angular_impulse),
                        bits3(expected_record.angular_impulse)
                    );
                    assert_eq!(
                        bits3(record.position_impulse),
                        bits3(original.position_impulse)
                    );
                }
                SimulationDriveHorusAccumulatorStoreOrder::PositionThenAngular => {
                    assert_eq!(
                        bits3(record.position_impulse),
                        bits3(expected_record.position_impulse)
                    );
                    assert_eq!(
                        bits3(record.angular_impulse),
                        bits3(original.angular_impulse)
                    );
                }
            }
        }

        let mut island_record = original;
        let island_rows = solve_drive_horus_island_record_host(
            &mut island_record,
            solver_delta_a,
            solver_delta_b,
        );
        assert!(island_record == expected_record);
        assert_eq!(island_rows, expected_rows);
    }

    #[test]
    fn horus_fused_negative_multiply_subtract_has_a_bit_discriminator() {
        let input = f32::from_bits(0x4c7f_ffff);
        let seed = f32::from_bits(0x3280_0001);
        let fused = horus_nmsub([seed; 4], [input; 4], [1.0; 4]);
        let defused = 1.0 - seed * input;
        assert_eq!(bits4(fused), [0xb37f_fffe; 4]);
        assert_eq!(defused.to_bits(), 0);
    }

    #[test]
    fn horus_clamp_is_min_then_max_and_builds_lower_with_zero_subtraction() {
        let value = [0.25, -0.5, 2.0, -3.0];
        let upper = [-1.0, 0.75, 1.25, 1.25];
        let native = horus_min_then_max(value, upper);
        let lower = upper.map(|lane| 0.0 - lane);
        let reversed = std::array::from_fn(|lane| value[lane].max(lower[lane]).min(upper[lane]));
        assert_eq!(
            bits4(native),
            [0x3f80_0000, 0xbf00_0000, 0x3fa0_0000, 0xbfa0_0000]
        );
        assert_eq!(
            bits4(reversed),
            [0xbf80_0000, 0xbf00_0000, 0x3fa0_0000, 0xbfa0_0000]
        );
    }

    #[test]
    fn horus_angular_constraint_distinguishes_translation_from_spin() {
        // Independent axis-decoupling oracle: pure translation cannot change
        // an angular constraint; pure relative spin must change it, even when
        // both attachment points are at their respective centres of mass.
        for island in [false, true] {
            for (row, expected) in [(0, [0.0, 0.0, 0.0]), (2, [1.0, -2.0, 3.0])] {
                let mut jacobian = SimulationDriveJacobian::zeroed();
                jacobian.angular_dynamics = 1.0;
                jacobian.angular_transpose_right = [1.0, 0.0, 0.0];
                jacobian.angular_transpose_up = [0.0, 1.0, 0.0];
                jacobian.angular_transpose_at = [0.0, 0.0, 1.0];
                jacobian.angular_limit_x = 10.0;
                jacobian.angular_limit_y = 10.0;
                jacobian.angular_limit_z = 10.0;
                let a = [[0.0; 4]; 4];
                let mut b = a;
                b[row] = [1.0, -2.0, 3.0, 0.0];
                if island {
                    solve_drive_horus_island_record_host(&mut jacobian, a, b);
                } else {
                    solve_drive_horus_record_host(&mut jacobian, a, b);
                }
                assert_eq!(bits3(jacobian.angular_impulse), bits3(expected));
                assert_eq!(bits3(jacobian.position_impulse), [0; 3]);
            }
        }
    }

    #[test]
    fn horus_simple_response_applies_a_positive_and_b_negative() {
        let mut jacobian = SimulationDriveJacobian::zeroed();
        jacobian.linear_dynamics = 1.0;
        jacobian.position_offset = [1.0, 0.0, 0.0];
        jacobian.position_limit_x = 10.0;
        jacobian.position_limit_y = 10.0;
        jacobian.position_limit_z = 10.0;
        jacobian.linear_right = [2.0, 3.0, 4.0];
        jacobian.inverse_mass_a = 0.5;
        jacobian.inverse_mass_b = 0.25;
        let a = [[0.0; 4]; 4];
        let b = [[0.0; 4]; 4];

        let output = solve_drive_horus_record_host(&mut jacobian, a, b);
        assert_eq!(
            bits4(output.solver_delta_a[0]),
            [0x3f80_0000, 0x3fc0_0000, 0x4000_0000, 0]
        );
        assert_eq!(
            bits4(output.solver_delta_b[0]),
            [0xbf00_0000, 0xbf40_0000, 0xbf80_0000, 0]
        );
        assert_eq!(bits3(jacobian.position_impulse), [0x3f80_0000, 0, 0]);
        assert_eq!(bits4(output.solver_delta_a[1]), bits4(a[1]));
        assert_eq!(bits4(output.solver_delta_a[2]), bits4(a[2]));
        assert_eq!(bits4(output.solver_delta_a[3]), bits4(a[3]));
        assert_eq!(bits4(output.solver_delta_b[1]), bits4(b[1]));
        assert_eq!(bits4(output.solver_delta_b[2]), bits4(b[2]));
        assert_eq!(bits4(output.solver_delta_b[3]), bits4(b[3]));
    }

    #[test]
    fn rqd_matches_raw_four_row_discriminator() {
        let a = [
            f32::from_bits(0x3e19_999a),
            f32::from_bits(0xbe4c_cccd),
            f32::from_bits(0x3e99_999a),
            f32::from_bits(0x3f59_999a),
        ];
        let b = [
            f32::from_bits(0xbdcc_cccd),
            f32::from_bits(0x3e80_0000),
            f32::from_bits(0xbeaa_aaab),
            f32::from_bits(0x3f66_6666),
        ];
        let rows = SimulationDriveJacobianRqd::create(a, b).rows;
        assert_eq!(
            bits4(rows[0]),
            [0x3f66_6667, 0x3d34_e818, 0xbde6_666a, 0xbe58_bf26]
        );
        assert_eq!(
            bits4(rows[1]),
            [0x3d91_1114, 0x3f54_7ae1, 0x3e44_4446, 0x3ebe_b851]
        );
        assert_eq!(
            bits4(rows[2]),
            [0xbd42_8f58, 0x3dbb_bbbe, 0x3f3a_e147, 0xbf12_2224]
        );
        assert_eq!(
            bits4(rows[3]),
            [0x3e69_d038, 0xbed3_3333, 0x3f09_2c60, 0x3f19_9999]
        );
    }

    #[test]
    fn reciprocal_refinement_retains_fused_residual() {
        let input = f32::from_bits(0x4c7f_ffff);
        let seed = f32::from_bits(0x3280_0001);
        let fused_residual = (-seed).mul_add(input, 1.0);
        let defused_residual = 1.0 - seed * input;
        assert_eq!(fused_residual.to_bits(), 0xb37f_fffe);
        assert_eq!(defused_residual.to_bits(), 0);
        assert_eq!(refine_drive_reciprocal(input, seed).to_bits(), 0x3280_0001);

        // This second finite fixture carries the fused/defused difference all
        // the way through the second refinement, rather than discriminating
        // only an intermediate which later reconverges.
        let input = f32::from_bits(0x72bb_3b18);
        let seed = f32::from_bits(0x0c2f_038f);
        let fused = refine_drive_reciprocal(input, seed);
        let defused_residual_0 = 1.0 - seed * input;
        let defused_refined_0 = seed * defused_residual_0 + seed;
        let defused_residual_1 = 1.0 - defused_refined_0 * input;
        let defused = defused_refined_0 * defused_residual_1 + defused_refined_0;
        assert_eq!(fused.to_bits(), 0x0c2f_038f);
        assert_eq!(defused.to_bits(), 0x0c2f_0390);
    }

    #[test]
    fn reciprocal_sqrt_refinement_has_a_final_fused_discriminator() {
        let input = f32::from_bits(0x688c_3031);
        let seed = f32::from_bits(0x2af4_9e50);
        let fused = refine_drive_reciprocal_sqrt(input, seed);
        let half = 0.5;
        let square_0 = seed * seed;
        let residual_0 = 1.0 - input * square_0;
        let refined_0 = (seed * half) * residual_0 + seed;
        let square_1 = refined_0 * refined_0;
        let residual_1 = 1.0 - input * square_1;
        let defused = (refined_0 * half) * residual_1 + refined_0;
        assert_eq!(fused.to_bits(), 0x2af4_9e51);
        assert_eq!(defused.to_bits(), 0x2af4_9e50);
    }

    #[test]
    fn soft_and_non_soft_coefficients_follow_distinct_native_paths() {
        let seeds = SimulationDriveReciprocalSeeds {
            dynamics: 0.8,
            spring: 0.8,
            damping: 0.8,
        };
        let soft = drive_coefficients(
            SimulationDriveDynamicsParams {
                spring: 2.0,
                damping: 0.5,
                strength: 123.0,
                raw_type: SOFT_DRIVE,
            },
            0.25,
            seeds,
        );
        assert_eq!(soft.dynamics.to_bits(), 0x3e4c_cccd);
        assert_eq!(soft.spring_factor.to_bits(), 0x3dcc_cccd);
        assert_eq!(soft.damping_factor.to_bits(), 0x3dcc_cccd);

        let non_soft_seeds = SimulationDriveReciprocalSeeds {
            dynamics: f32::NAN,
            spring: 1.0 / 1.125,
            damping: f32::NAN,
        };
        let no_drive = drive_coefficients(
            SimulationDriveDynamicsParams {
                spring: 999.0,
                damping: 0.5,
                strength: -999.0,
                raw_type: 0,
            },
            0.25,
            non_soft_seeds,
        );
        let hard = drive_coefficients(
            SimulationDriveDynamicsParams {
                raw_type: HARD_DRIVE,
                ..SimulationDriveDynamicsParams {
                    spring: 999.0,
                    damping: 0.5,
                    strength: -999.0,
                    raw_type: 0,
                }
            },
            0.25,
            non_soft_seeds,
        );
        let unknown = drive_coefficients(
            SimulationDriveDynamicsParams {
                raw_type: 0x8000_0001,
                ..SimulationDriveDynamicsParams {
                    spring: 999.0,
                    damping: 0.5,
                    strength: -999.0,
                    raw_type: 0,
                }
            },
            0.25,
            non_soft_seeds,
        );
        assert_eq!(no_drive, hard);
        assert_eq!(hard, unknown);
        assert_eq!(hard.dynamics.to_bits(), 0x3f80_0000);
        assert_eq!(hard.damping_factor.to_bits(), 0x3f80_0000);
        assert_eq!(hard.spring_factor.to_bits(), 0x3f63_8e39);
    }

    #[test]
    fn hard_clamp_is_strict_and_unknown_types_do_not_enter_it() {
        let error = [2.0, 0.0, 0.0];
        let equality = apply_hard_drive_gate(HARD_DRIVE, error, 4.0, 1.0, 2.0, 0.5);
        assert!(!equality.outside_limit);
        assert_eq!(equality.error.map(f32::to_bits), error.map(f32::to_bits));

        let below = f32::from_bits(4.0_f32.to_bits() - 1);
        let inside = apply_hard_drive_gate(HARD_DRIVE, error, below, 1.0, 2.0, 0.5);
        assert!(!inside.outside_limit);
        assert_eq!(inside.error.map(f32::to_bits), error.map(f32::to_bits));

        let above = f32::from_bits(4.0_f32.to_bits() + 1);
        let clamped = apply_hard_drive_gate(HARD_DRIVE, error, above, 1.0, 2.0, 0.5);
        assert!(clamped.outside_limit);
        assert_eq!(clamped.error.map(f32::to_bits), [0x3fff_ffff, 0, 0]);

        for raw_type in [0, SOFT_DRIVE, 3, 0x8000_0001] {
            let bypassed = apply_hard_drive_gate(raw_type, error, above, 1.0, 2.0, f32::NAN);
            assert!(!bypassed.outside_limit);
            assert_eq!(bypassed.error.map(f32::to_bits), error.map(f32::to_bits));
        }

        let negative_spring = apply_hard_drive_gate(
            HARD_DRIVE,
            [2.0, 0.0, 0.0],
            4.5,
            0.5,
            -4.0,
            f32::from_bits(0x3eff_ffff),
        );
        assert!(negative_spring.outside_limit);
        assert_eq!(negative_spring.error[0].to_bits(), 0xbff1_5900);
    }
}
