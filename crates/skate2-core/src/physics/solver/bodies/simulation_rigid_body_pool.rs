//! Safe semantic model of the preallocated `rw::physics::Simulation` body pool.
//!
//! `Simulation::AddRigidBody` (`0x82ae6e00..0x82ae79e0`) takes the head of the
//! free circular list, while `Simulation::RemoveRigidBody`
//! (`0x82ae3190..0x82ae3224`) appends to its tail. `RigidBody::Reset`
//! (`0x827c8f88..0x827c9084`) resets body state without touching the PDB-named
//! `mId` word, links, owning Simulation, or inertia pointer. Native `mId/+0x1c`
//! is actually a pointer to that body's private RF solver-delta row. This module
//! uses an unrelated stable host slot ID plus a separate RF-row table rather
//! than truncating native pointers or claiming native byte layout.
//!
//! Native inertia storage is nonowning. The host pool instead retains an
//! shared owner while its semantic pointer would remain in a slot, including while
//! that slot is free. This deliberate lifetime extension prevents dangling
//! Rust references and is not a native ownership claim. Handles additionally
//! carry process-unique pool authority and a non-reused occurrence number;
//! native code has neither recoverable validation nor ABA protection.
//!
//! `Simulation::Initialize` constructs exactly `N` usable bodies, links them
//! into Free in ascending order, assigns each the owning Simulation and a
//! pointer to RF row `i`, and clears the complete RF table once. Its raw Reset
//! loop unusually starts at the sentinel/body allocation base. For `N >= 4` it
//! resets all four sentinels plus the first `N - 4` usable records and misses
//! the final four; for `N < 4` it resets `N` sentinels and misses every usable
//! record. Later metadata stores and every acquisition's Reset make valid
//! native use possible. Reset, Add, Remove, and slot reuse do not clear an RF
//! row; native `DynamicUpdate` does that after consuming it. The safe host
//! representation initializes every semantic slot up front, never exposes
//! uninitialized payload, and does not claim that anomaly as portable behavior.

use crate::simulation_dynamic_update::{
    SimulationDynamicUpdateBody, SimulationDynamicUpdateConfiguration, simulation_dynamic_update,
};
use crate::world::{
    orthonormalize_collision_affine, simulation_quaternion_from_orthonormalized_matrix_host,
    simulation_rotated_inverse_inertia_host,
};
use std::{
    error::Error,
    fmt,
    num::NonZeroU64,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

#[path = "simulation_rigid_body_activation.rs"]
pub(crate) mod activation;
#[path = "simulation_rigid_body_grab_force.rs"]
mod grab_force;
#[path = "simulation_rigid_body_living_world.rs"]
mod living_world_drive;
#[path = "simulation_rigid_body_local_torque.rs"]
mod local_torque;
#[path = "simulation_rigid_body_target_angular.rs"]
pub(crate) mod target_angular;

static NEXT_POOL_AUTHORITY: AtomicU64 = AtomicU64::new(1);

#[path = "../contacts/simulation_contact_feedback_phase.rs"]
pub(crate) mod contact_feedback_phase;

/// Shared source inertia storage. Writes require an exclusive owning-pool borrow
/// and either a matching live body or its retained programmatic Definition.
/// No writable lock or mutation callback escapes.
/// First non-Static admission binds this owner permanently to one host pool,
/// preventing cross-pool aliases from changing an already prepared phase.
#[derive(Clone, Debug)]
pub(crate) struct SimulationMutableInertia(Arc<Mutex<MutableInertiaState>>);

#[derive(Debug)]
struct MutableInertiaState {
    value: SimulationInertia,
    inverse_tensor_w: Option<f32>,
    pool: Option<SimulationRigidBodyPoolAuthority>,
}

impl SimulationMutableInertia {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "XYZ-only mutable owner retained for generic pool tests"
        )
    )]
    pub(crate) fn new(value: SimulationInertia) -> Self {
        Self(Arc::new(Mutex::new(MutableInertiaState {
            value,
            inverse_tensor_w: None,
            pool: None,
        })))
    }

    /// Retains an explicitly supplied native fourth tensor lane. Generic XYZ
    /// owners do not infer this lane from X or from a constructor constant.
    pub(crate) fn new_packed(value: SimulationInertia, w: f32) -> Self {
        Self(Arc::new(Mutex::new(MutableInertiaState {
            value,
            inverse_tensor_w: Some(w),
            pool: None,
        })))
    }

    pub(crate) fn try_read(&self) -> Result<SimulationInertia, SimulationRigidBodyPoolError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| SimulationRigidBodyPoolError::InertiaLockPoisoned)?
            .value)
    }

    /// One coherent read of the retained four-lane storage, if its producer
    /// supplied W. Existing simulation math continues to consume only XYZ.
    pub(crate) fn try_read_packed(&self) -> Result<Option<[f32; 4]>, SimulationRigidBodyPoolError> {
        let state = self
            .0
            .lock()
            .map_err(|_| SimulationRigidBodyPoolError::InertiaLockPoisoned)?;
        Ok(state.inverse_tensor_w.map(|w| {
            let [x, y, z] = state.value.inverse_tensor;
            [x, y, z, w]
        }))
    }
}

#[derive(Clone, Debug)]
enum RetainedInertia {
    Immutable(Arc<SimulationInertia>),
    Mutable(SimulationMutableInertia),
}

impl RetainedInertia {
    fn try_read(&self) -> Result<SimulationInertia, SimulationRigidBodyPoolError> {
        match self {
            Self::Immutable(value) => Ok(**value),
            Self::Mutable(value) => value.try_read(),
        }
    }
}

#[cfg(test)]
#[path = "../simulation_mutable_inertia_tests.rs"]
mod mutable_inertia_tests;

/// Pool-local nonreused host identity for one body-configuration epoch.
///
/// Only the pair `(SimulationRigidBodyPoolAuthority, frame identity)` is
/// globally qualified. The initial safe-host epoch exists before the first
/// `begin_dynamic_update_frame`; each later call advances it once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SimulationRigidBodyFrameIdentity(NonZeroU64);

/// Pool-local nonreused identity of one prepared `DynamicUpdate` phase.
///
/// This is safe-host replay protection, not a recovered native field. A
/// frame identity alone cannot distinguish two phases prepared in the same
/// frame after the first phase has been consumed or dropped.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct SimulationRigidBodyDynamicUpdatePhaseIdentity(NonZeroU64);

/// Exact gravity installed by `Simulation::Initialize`.
pub const SIMULATION_INITIAL_GRAVITY: [f32; 3] = [0.0, -600.0, 0.0];
/// Exact integer cooldown installed by `Simulation::Initialize`.
pub const SIMULATION_INITIAL_COOLDOWN: u32 = 30;
/// Exact minimum-energy scalar installed by `Simulation::Initialize`.
pub const SIMULATION_INITIAL_MINIMUM_ENERGY: f32 = 1.0;
const SPY_BODY: u32 = 8;

/// Private host storage corresponding to one native 0x40-byte RF row.
///
/// `Simulation::Initialize` aligns the RF base to 0x40 at
/// `0x82ae7b30..0x82ae7b3c`, then clears all four vectors once at
/// `0x82ae7b80..0x82ae7bb0` and stores each row address in the PDB-named body
/// `mId/+0x1c` field at `0x82ae7c94/0x82ae7cc8`. Add, Reset, Remove, and reuse
/// preserve these bits. The bounded ordinary-finite Active-body batch in this
/// module consumes and clears all four vectors after each body update.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq)]
struct SimulationRigidBodySolverDeltaRow {
    vectors: [[f32; 4]; 4],
}

/// Which body side failed while planning one ordered RF pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationRigidBodySolverDeltaEndpoint {
    BodyA,
    BodyB,
}

/// Typed pre-effect failure from the private RF transaction boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained for the test-only direct Drive evidence harness"
    )
)]
pub(crate) enum SimulationRigidBodySolverDeltaTransactionError {
    AllocationFailed {
        pair_count: usize,
    },
    Pool(SimulationRigidBodyPoolError),
    Body {
        ordinal: usize,
        endpoint: SimulationRigidBodySolverDeltaEndpoint,
        source: SimulationRigidBodyPoolError,
    },
    ItemCountMismatch {
        pair_count: usize,
        item_count: usize,
    },
}

/// Typed pre-effect failure from one indexed RF sequence on a guarded phase.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for checked Contact/Joint/Drive island sweeps"
    )
)]
pub(crate) enum SimulationRigidBodyIndexedSolverDeltaError {
    AllocationFailed {
        entry_count: usize,
    },
    Pool(SimulationRigidBodyPoolError),
    ItemOutOfRange {
        ordinal: usize,
        item_index: usize,
        item_count: usize,
    },
    Body {
        ordinal: usize,
        endpoint: SimulationRigidBodySolverDeltaEndpoint,
        source: SimulationRigidBodyPoolError,
    },
    SequenceEndedEarly {
        expected: usize,
        actual: usize,
    },
    SequenceHasTrailingEntry {
        expected: usize,
    },
}

/// Reusable, phase-bound item/body index plan for an ordered RF subsequence.
///
/// Preparation resolves every occurrence and reserves the complete entry list
/// before any solver effect. The exclusive `DynamicUpdate` phase prevents body
/// topology changes between preparation and repeated Horus passes. Private
/// indices never leave this module.
pub(crate) struct SimulationRigidBodyIndexedSolverDeltaPlan<'items, T> {
    phase: SimulationRigidBodyDynamicUpdatePhaseIdentity,
    items: &'items mut [T],
    entries: Vec<(usize, usize, usize)>,
}

/// Exclusive owner for one fully prevalidated Contact/Joint/Drive island run.
///
/// All three reusable RF plans exist before this value is created. Owning the
/// mutable `DynamicUpdate` phase keeps their private body indices and the live RF
/// storage stable through every runtime-counted Horus pass.
pub(crate) struct SimulationRigidBodyHorusIslandRfTransaction<
    'phase,
    'pool,
    'contact,
    'joint,
    'drive,
    Contact,
    Joint,
    Drive,
> {
    phase: &'phase mut SimulationRigidBodyDynamicUpdatePhase<'pool>,
    contact: SimulationRigidBodyIndexedSolverDeltaPlan<'contact, Contact>,
    joint: SimulationRigidBodyIndexedSolverDeltaPlan<'joint, Joint>,
    drive: SimulationRigidBodyIndexedSolverDeltaPlan<'drive, Drive>,
}

/// Complete copied A/B rows supplied to one infallible solver callback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationRigidBodySolverDeltaPairRows {
    body_a: [[f32; 4]; 4],
    body_b: [[f32; 4]; 4],
}

impl SimulationRigidBodySolverDeltaPairRows {
    #[must_use]
    pub(crate) const fn into_rows(self) -> ([[f32; 4]; 4], [[f32; 4]; 4]) {
        (self.body_a, self.body_b)
    }
}

/// The only four RF vectors writable by one recovered Joint/Drive occurrence.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(
    clippy::struct_field_names,
    reason = "body A/B and vector 0/2 roles are the recovered store contract"
)]
pub(crate) struct SimulationRigidBodySolverDeltaStores {
    body_a_0: [f32; 4],
    body_b_0: [f32; 4],
    body_a_2: [f32; 4],
    body_b_2: [f32; 4],
}

impl SimulationRigidBodySolverDeltaStores {
    #[must_use]
    #[allow(
        clippy::similar_names,
        reason = "source-shaped A/B vector names make store order auditable"
    )]
    pub(crate) const fn new(
        body_a_0: [f32; 4],
        body_b_0: [f32; 4],
        body_a_2: [f32; 4],
        body_b_2: [f32; 4],
    ) -> Self {
        Self {
            body_a_0,
            body_b_0,
            body_a_2,
            body_b_2,
        }
    }
}

/// All eight RF vectors writable by one recovered Contact occurrence.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(
    clippy::struct_field_names,
    reason = "body A/B and native Contact row order are the recovered store contract"
)]
pub(crate) struct SimulationRigidBodyContactSolverDeltaStores {
    body_a_3: [f32; 4],
    body_b_3: [f32; 4],
    body_a_1: [f32; 4],
    body_b_1: [f32; 4],
    body_a_2: [f32; 4],
    body_b_2: [f32; 4],
    body_a_0: [f32; 4],
    body_b_0: [f32; 4],
}

impl SimulationRigidBodyContactSolverDeltaStores {
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        clippy::similar_names,
        reason = "the eight source-shaped arguments make Contact store order auditable"
    )]
    pub(crate) const fn new(
        body_a_3: [f32; 4],
        body_b_3: [f32; 4],
        body_a_1: [f32; 4],
        body_b_1: [f32; 4],
        body_a_2: [f32; 4],
        body_b_2: [f32; 4],
        body_a_0: [f32; 4],
        body_b_0: [f32; 4],
    ) -> Self {
        Self {
            body_a_3,
            body_b_3,
            body_a_1,
            body_b_1,
            body_a_2,
            body_b_2,
            body_a_0,
            body_b_0,
        }
    }
}

/// Prevalidated, pool-owned plan for one ordered RF sweep.
///
/// Private indices never leave this value. The exclusive pool borrow spans
/// validation through commit, so safe code cannot invalidate an occurrence
/// after preflight or create simultaneous mutable aliases for a self-pair.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained for the test-only direct Drive evidence harness"
    )
)]
pub(crate) struct SimulationRigidBodySolverDeltaTransaction<'a> {
    pool: &'a mut SimulationRigidBodyPool,
    pairs: Vec<(usize, usize)>,
}

#[derive(Clone, Copy, Debug)]
struct SimulationRigidBodyDynamicUpdatePlanEntry {
    index: usize,
    inertia: SimulationInertia,
}

/// Typed pre-effect refusal while preparing an infallible `DynamicUpdate` phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationRigidBodyDynamicUpdatePrepareError {
    Pool(SimulationRigidBodyPoolError),
    AllocationFailed { active_count: usize },
    PhaseIdentityExhausted,
}

/// Exclusive preflight-to-consume owner for one Active-body `DynamicUpdate`.
///
/// The copied plan freezes native Active right-link order, every required
/// inertia value, and the frame configuration before a later Horus phase can
/// have effects. The exclusive pool borrow prevents topology, inertia, and
/// configuration changes while restricted RF transactions run. Dropping this
/// value burns its private replay-protection identity but performs no body or
/// RF work; only explicit consumption integrates bodies and clears RF.
#[must_use = "dropping the phase abandons DynamicUpdate and does not clear RF"]
pub(crate) struct SimulationRigidBodyDynamicUpdatePhase<'pool> {
    pool: &'pool mut SimulationRigidBodyPool,
    active: Vec<SimulationRigidBodyDynamicUpdatePlanEntry>,
    configuration: SimulationDynamicUpdateConfiguration,
    frame_identity: SimulationRigidBodyFrameIdentity,
    phase_identity: SimulationRigidBodyDynamicUpdatePhaseIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimulationRigidBodySolverDeltaStoreKind {
    BodyA3,
    BodyB3,
    BodyA1,
    BodyB1,
    BodyA0,
    BodyB0,
    BodyA2,
    BodyB2,
}

impl SimulationRigidBodySolverDeltaRow {
    const ZERO: Self = Self {
        vectors: [[0.0; 4]; 4],
    };
}

const _: () = {
    assert!(std::mem::size_of::<SimulationRigidBodySolverDeltaRow>() == 0x40);
    assert!(std::mem::align_of::<SimulationRigidBodySolverDeltaRow>() == 0x40);
};

/// Immutable semantic values retained from native `rw::physics::Inertia`.
///
/// This is not a native-layout or `repr(C)` claim. `AddRigidBody` consumes the
/// first two values; the private ordinary-finite `DynamicUpdate` reconstruction
/// additionally consumes the five scalar controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationInertia {
    pub inverse_tensor: [f32; 3],
    pub inverse_mass: f32,
    pub spherical: f32,
    pub max_velocity: f32,
    pub max_omega: f32,
    pub linear_drag: f32,
    pub angular_drag: f32,
}

impl SimulationInertia {
    /// Values installed by `PartDefinition::Initialize`
    /// (`0x82ae9058..0x82ae90e8`).
    #[must_use]
    pub const fn native_default() -> Self {
        Self {
            inverse_tensor: [1.0; 3],
            inverse_mass: 1.0,
            spherical: 1.0,
            max_velocity: f32::MAX,
            max_omega: f32::MAX,
            linear_drag: 0.0,
            angular_drag: 0.0,
        }
    }
}

/// Opaque identity of one pool instance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SimulationRigidBodyPoolAuthority(u64);

/// Stable host identity of one preallocated slot.
///
/// This typed index is unrelated to native `RigidBody::mId/+0x1c`, whose
/// runtime value is a pointer to a 0x40-byte RF solver-delta row.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SimulationRigidBodyId(u32);

impl SimulationRigidBodyId {
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Inert pool-qualified slot identity. Unlike [`SimulationRigidBodyHandle`],
/// this carries no allocation occurrence and grants no body mutation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SimulationRigidBodyIdentity {
    authority: SimulationRigidBodyPoolAuthority,
    id: SimulationRigidBodyId,
}

/// Occurrence-bound read capability for one allocated body. This preserves
/// ABA identity for solver reads but is not accepted by removal APIs.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SimulationRigidBodyObservation {
    authority: SimulationRigidBodyPoolAuthority,
    id: SimulationRigidBodyId,
    occurrence: NonZeroU64,
}

/// Occurrence-checked semantic body values read by
/// `EA::Physics::SerialisedAssembly::Create`.
///
/// Five native adjacent words are deliberately absent: the stasis, inertia,
/// RF-row, and intrusive right/left pointer images are address-domain values,
/// not portable body semantics. The `SerialisedAssembly` compositor obtains
/// those words from its explicit virtual-Xenon packed-word provider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationRigidBodySerialisedAssemblySnapshot {
    pub(crate) observation: SimulationRigidBodyObservation,
    pub(crate) state: u32,
    pub(crate) center_of_mass: [f32; 3],
    pub(crate) velocity: [f32; 3],
    pub(crate) omega: [f32; 3],
    pub(crate) right: [f32; 3],
    pub(crate) up: [f32; 3],
    pub(crate) at: [f32; 3],
    pub(crate) tag: u32,
    pub(crate) force: [f32; 3],
    pub(crate) kinematic_value: f32,
    pub(crate) torque: [f32; 3],
    pub(crate) cooldown: u32,
}

/// Occurrence-checked live fields needed by the Xenon packed-word compositor.
///
/// The two links are current intrusive-ring nodes, not a creation-time
/// snapshot. `has_retained_inertia` reports only whether this safe-host slot
/// currently retains its semantic [`SimulationInertia`] owner. It neither
/// identifies the native source address nor selects the packed inertia word:
/// native state transitions preserve `mInertia`, so that word must eventually
/// come from an occurrence-bound Add-time outcome record: null for a Static
/// add, or the exact bound source address for a Frozen/Active add.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "used by the private SerialisedAssembly Xenon packed-word provider"
    )
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SimulationRigidBodyPackedWordTopologySnapshot {
    pub(crate) observation: SimulationRigidBodyObservation,
    pub(crate) right_link: SimulationRigidBodyNode,
    pub(crate) left_link: SimulationRigidBodyNode,
    pub(crate) has_retained_inertia: bool,
}

/// Occurrence-bound copied body values consumed by Drive Jacobian Build.
///
/// This value contains neither the retained [`SimulationInertia`] owner nor a
/// solver-row reference, removal handle, lock guard, or mutable pool
/// capability. The originating observation remains attached so a later private
/// batch sidecar can revalidate the exact allocation occurrence before solver
/// effects.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the private Drive batch preparation seam"
    )
)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationRigidBodyDriveBuildSnapshot {
    observation: SimulationRigidBodyObservation,
    pub(crate) state: u32,
    pub(crate) quaternion: [f32; 4],
    pub(crate) center_of_mass: [f32; 3],
    pub(crate) velocity: [f32; 3],
    pub(crate) omega: [f32; 3],
    pub(crate) right: [f32; 3],
    pub(crate) up: [f32; 3],
    pub(crate) at: [f32; 3],
    pub(crate) force: [f32; 3],
    pub(crate) torque: [f32; 3],
    pub(crate) inverse_inertia_full: [f32; 3],
    pub(crate) inverse_mass: f32,
    pub(crate) inverse_inertia_split: [f32; 3],
}

/// Occurrence-bound copied body values consumed by network Contact production.
///
/// Native `Island::AddNetworkContacts` copies these values into each Contact
/// before table construction and in-place `ContactBatchBuild`. This inert value
/// has no retained Inertia owner, RF-row reference, raw pointer, handle, or
/// mutation capability. In particular, `kinematic_value` and `cooldown` are
/// separate from the narrower Joint/Drive snapshot read sets.
#[derive(Clone, Copy)]
pub(crate) struct SimulationRigidBodyNetworkContactSnapshot {
    pub(crate) observation: SimulationRigidBodyObservation,
    pub(crate) center_of_mass: [f32; 3],
    pub(crate) velocity: [f32; 3],
    pub(crate) omega: [f32; 3],
    pub(crate) right: [f32; 3],
    pub(crate) up: [f32; 3],
    pub(crate) at: [f32; 3],
    pub(crate) inverse_inertia_full: [f32; 3],
    pub(crate) inverse_mass: f32,
    pub(crate) inverse_inertia_split: [f32; 3],
    pub(crate) state: u32,
    pub(crate) force: [f32; 3],
    pub(crate) kinematic_value: f32,
    pub(crate) torque: [f32; 3],
    pub(crate) cooldown: u32,
}

/// Occurrence-bound copied body values consumed by Joint Jacobian Build.
///
/// Native adjacent list/pointer/tag words are deliberately absent: the raw
/// Build inventory and independent mutation oracle prove they do not change an
/// estimate callback operand or final record word under the ordinary-finite
/// host policy. Reduction callback operands and native intermediate W images
/// are not claimed. The later adapter supplies neutral computational W lanes
/// and separately installs the checked RF-row token.
#[derive(Clone, Copy)]
pub(crate) struct SimulationRigidBodyJointBuildSnapshot {
    pub(crate) observation: SimulationRigidBodyObservation,
    pub(crate) state: u32,
    pub(crate) quaternion: [f32; 4],
    pub(crate) center_of_mass: [f32; 3],
    pub(crate) velocity: [f32; 3],
    pub(crate) omega: [f32; 3],
    pub(crate) right: [f32; 3],
    pub(crate) up: [f32; 3],
    pub(crate) at: [f32; 3],
    pub(crate) force: [f32; 3],
    pub(crate) torque: [f32; 3],
    pub(crate) inverse_inertia_full: [f32; 3],
    pub(crate) inverse_mass: f32,
    pub(crate) inverse_inertia_split: [f32; 3],
}

impl SimulationRigidBodyJointBuildSnapshot {
    #[must_use]
    pub(crate) const fn observation(self) -> SimulationRigidBodyObservation {
        self.observation
    }
}

impl SimulationRigidBodyDriveBuildSnapshot {
    /// Returns the inert read observation retained by this copied snapshot.
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for the private Drive batch preparation seam"
        )
    )]
    pub(crate) const fn observation(self) -> SimulationRigidBodyObservation {
        self.observation
    }
}

impl SimulationRigidBodyObservation {
    #[must_use]
    pub const fn identity(self) -> SimulationRigidBodyIdentity {
        SimulationRigidBodyIdentity {
            authority: self.authority,
            id: self.id,
        }
    }
}

impl SimulationRigidBodyIdentity {
    #[must_use]
    pub const fn authority(self) -> SimulationRigidBodyPoolAuthority {
        self.authority
    }

    #[must_use]
    pub const fn id(self) -> SimulationRigidBodyId {
        self.id
    }
}

/// Copy-only capability naming one allocation occurrence of one slot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SimulationRigidBodyHandle {
    authority: SimulationRigidBodyPoolAuthority,
    id: SimulationRigidBodyId,
    occurrence: NonZeroU64,
}

impl SimulationRigidBodyHandle {
    #[must_use]
    pub const fn id(self) -> SimulationRigidBodyId {
        self.id
    }

    /// Removes the allocation occurrence, producing an observation-only ID.
    #[must_use]
    pub const fn identity(self) -> SimulationRigidBodyIdentity {
        SimulationRigidBodyIdentity {
            authority: self.authority,
            id: self.id,
        }
    }

    /// Removes mutation authority while retaining the allocation occurrence
    /// required to reject stale solver reads.
    #[must_use]
    pub const fn observation(self) -> SimulationRigidBodyObservation {
        SimulationRigidBodyObservation {
            authority: self.authority,
            id: self.id,
            occurrence: self.occurrence,
        }
    }
}

/// One of the four intrusive circular lists retained by the Simulation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SimulationRigidBodyList {
    Free,
    Static,
    Frozen,
    Active,
}

impl SimulationRigidBodyList {
    const ALL: [Self; 4] = [Self::Free, Self::Static, Self::Frozen, Self::Active];

    const fn index(self) -> usize {
        match self {
            Self::Free => 0,
            Self::Static => 1,
            Self::Frozen => 2,
            Self::Active => 3,
        }
    }

    const fn state(self) -> u32 {
        match self {
            Self::Free => 0,
            Self::Static => 1,
            Self::Frozen => 2,
            Self::Active => 4,
        }
    }
}

/// A link target exposed only through read-only snapshots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationRigidBodyNode {
    Anchor(SimulationRigidBodyList),
    Body(SimulationRigidBodyId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Links {
    right: SimulationRigidBodyNode,
    left: SimulationRigidBodyNode,
}

/// Native-width body-list counts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SimulationRigidBodyCounts {
    pub free: u32,
    pub static_bodies: u32,
    pub frozen: u32,
    pub active: u32,
}

impl SimulationRigidBodyCounts {
    fn get(self, list: SimulationRigidBodyList) -> u32 {
        match list {
            SimulationRigidBodyList::Free => self.free,
            SimulationRigidBodyList::Static => self.static_bodies,
            SimulationRigidBodyList::Frozen => self.frozen,
            SimulationRigidBodyList::Active => self.active,
        }
    }

    fn get_mut(&mut self, list: SimulationRigidBodyList) -> &mut u32 {
        match list {
            SimulationRigidBodyList::Free => &mut self.free,
            SimulationRigidBodyList::Static => &mut self.static_bodies,
            SimulationRigidBodyList::Frozen => &mut self.frozen,
            SimulationRigidBodyList::Active => &mut self.active,
        }
    }
}

#[derive(Debug)]
struct RigidBodySlot {
    id: SimulationRigidBodyId,
    occurrence: Option<NonZeroU64>,
    quaternion: [f32; 4],
    center_of_mass: [f32; 3],
    velocity: [f32; 3],
    links: Links,
    omega: [f32; 3],
    right: [f32; 3],
    stasis: SimulationRigidBodyPoolAuthority,
    up: [f32; 3],
    inertia: Option<RetainedInertia>,
    at: [f32; 3],
    tag: u32,
    inverse_inertia_full: [f32; 3],
    inverse_mass: f32,
    inverse_inertia_split: [f32; 3],
    state: u32,
    force: [f32; 3],
    kinematic_value: f32,
    torque: [f32; 3],
    cooldown: u32,
    membership: SimulationRigidBodyList,
}

impl RigidBodySlot {
    fn initialized(
        id: SimulationRigidBodyId,
        stasis: SimulationRigidBodyPoolAuthority,
        links: Links,
    ) -> Self {
        Self {
            id,
            occurrence: None,
            quaternion: [0.0, 0.0, 0.0, 1.0],
            center_of_mass: [0.0; 3],
            velocity: [0.0; 3],
            links,
            omega: [0.0; 3],
            right: [1.0, 0.0, 0.0],
            stasis,
            up: [0.0, 1.0, 0.0],
            inertia: None,
            at: [0.0, 0.0, 1.0],
            tag: 0,
            inverse_inertia_full: [0.0; 3],
            inverse_mass: 0.0,
            inverse_inertia_split: [0.0; 3],
            state: 0,
            force: [0.0; 3],
            kinematic_value: 0.0,
            torque: [0.0; 3],
            cooldown: 0,
            membership: SimulationRigidBodyList::Free,
        }
    }

    /// Exact semantic write set of `RigidBody::Reset`. Links, ID, stasis,
    /// inertia, and the host occurrence capability deliberately survive.
    fn reset_native_fields(&mut self) {
        self.quaternion = [0.0, 0.0, 0.0, 1.0];
        self.center_of_mass = [0.0; 3];
        self.velocity = [0.0; 3];
        self.omega = [0.0; 3];
        self.right = [1.0, 0.0, 0.0];
        self.up = [0.0, 1.0, 0.0];
        self.at = [0.0, 0.0, 1.0];
        self.tag = 0;
        self.inverse_inertia_full = [0.0; 3];
        self.inverse_mass = 0.0;
        self.inverse_inertia_split = [0.0; 3];
        self.state = 0;
        self.force = [0.0; 3];
        self.kinematic_value = 0.0;
        self.torque = [0.0; 3];
        self.cooldown = 0;
    }
}

#[path = "simulation_rigid_body_skateboard_force.rs"]
mod skateboard_force;

#[path = "simulation_rigid_body_angular_velocity.rs"]
mod angular_velocity;
#[path = "simulation_rigid_body_linear_drag.rs"]
mod linear_drag;
#[path = "simulation_rigid_body_set_velocity.rs"]
mod set_velocity;

/// Read-only semantic snapshot of one slot.
#[derive(Clone, Debug, PartialEq)]
pub struct SimulationRigidBodySlotSnapshot {
    pub id: SimulationRigidBodyId,
    pub occurrence: Option<u64>,
    pub quaternion: [f32; 4],
    pub center_of_mass: [f32; 3],
    pub velocity: [f32; 3],
    pub right_link: SimulationRigidBodyNode,
    pub omega: [f32; 3],
    pub left_link: SimulationRigidBodyNode,
    pub right: [f32; 3],
    pub stasis: SimulationRigidBodyPoolAuthority,
    pub up: [f32; 3],
    pub inertia: Option<SimulationInertia>,
    pub at: [f32; 3],
    pub tag: u32,
    pub inverse_inertia_full: [f32; 3],
    pub inverse_mass: f32,
    pub inverse_inertia_split: [f32; 3],
    pub state: u32,
    pub force: [f32; 3],
    pub kinematic_value: f32,
    pub torque: [f32; 3],
    pub cooldown: u32,
    pub membership: SimulationRigidBodyList,
}

/// Complete read-only snapshot, including exact ring order.
#[derive(Clone, Debug, PartialEq)]
pub struct SimulationRigidBodyPoolSnapshot {
    pub authority: SimulationRigidBodyPoolAuthority,
    pub counts: SimulationRigidBodyCounts,
    pub next_occurrence: Option<u64>,
    pub next_dynamic_update_phase_identity: Option<u64>,
    pub gravity: [f32; 3],
    pub cooldown: u32,
    pub timestep: f32,
    pub frequency: f32,
    pub minimum_energy: f32,
    pub free_order: Vec<SimulationRigidBodyId>,
    pub static_order: Vec<SimulationRigidBodyId>,
    pub frozen_order: Vec<SimulationRigidBodyId>,
    pub active_order: Vec<SimulationRigidBodyId>,
    pub slots: Vec<SimulationRigidBodySlotSnapshot>,
}

/// Typed safe-host refusal or invariant failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationRigidBodyPoolError {
    CapacityExceedsU32 { capacity: usize },
    AllocationFailed { capacity: usize },
    DynamicUpdatePlanAllocationFailed { active_count: usize },
    DynamicUpdatePhaseIdentityExhausted,
    FrameIdentityExhausted,
    AuthorityExhausted,
    OccurrenceExhausted,
    ForeignHandle,
    SlotOutOfRange { slot: u32 },
    StaleHandle { slot: u32 },
    BodyIsFree { slot: u32 },
    BodyNotFrozen { slot: u32, state: u32 },
    ActiveBodyMissingInertia { slot: u32 },
    InertiaLockPoisoned,
    InertiaOwnerPoolMismatch,
    InertiaOwnerMismatch { slot: u32 },
    ImmutableInertia { slot: u32 },
    MutableInertiaRequiresSnapshot { slot: u32 },
    PackedInertiaUnavailable { slot: u32 },
    CorruptStructure,
}

impl fmt::Display for SimulationRigidBodyPoolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceedsU32 { capacity } => {
                write!(formatter, "rigid-body capacity {capacity} exceeds u32")
            }
            Self::AllocationFailed { capacity } => {
                write!(formatter, "could not allocate {capacity} rigid-body slots")
            }
            Self::DynamicUpdatePlanAllocationFailed { active_count } => write!(
                formatter,
                "could not allocate DynamicUpdate plan for {active_count} Active bodies"
            ),
            Self::DynamicUpdatePhaseIdentityExhausted => {
                formatter.write_str("rigid-body DynamicUpdate phase identity space is exhausted")
            }
            Self::FrameIdentityExhausted => {
                formatter.write_str("rigid-body frame identity space is exhausted")
            }
            Self::AuthorityExhausted => formatter.write_str("rigid-body authority exhausted"),
            Self::OccurrenceExhausted => formatter.write_str("rigid-body occurrence exhausted"),
            Self::ForeignHandle => formatter.write_str("rigid-body handle belongs to another pool"),
            Self::SlotOutOfRange { slot } => {
                write!(formatter, "rigid-body slot {slot} is out of range")
            }
            Self::StaleHandle { slot } => write!(formatter, "rigid-body slot {slot} is stale"),
            Self::BodyNotFrozen { slot, state } => write!(
                formatter,
                "rigid-body slot {slot} is not Frozen (state {state:#x})"
            ),
            Self::BodyIsFree { slot } => write!(formatter, "rigid-body slot {slot} is free"),
            Self::ActiveBodyMissingInertia { slot } => {
                write!(formatter, "active rigid-body slot {slot} has no inertia")
            }
            Self::InertiaLockPoisoned => formatter.write_str("retained inertia lock is poisoned"),
            Self::InertiaOwnerPoolMismatch => {
                formatter.write_str("mutable inertia belongs to another pool")
            }
            Self::InertiaOwnerMismatch { slot } => write!(
                formatter,
                "rigid-body slot {slot} retains a different inertia owner"
            ),
            Self::ImmutableInertia { slot } => write!(
                formatter,
                "rigid-body slot {slot} has no mutable inertia owner"
            ),
            Self::MutableInertiaRequiresSnapshot { slot } => write!(
                formatter,
                "rigid-body slot {slot} requires a current mutable inertia snapshot"
            ),
            Self::PackedInertiaUnavailable { slot } => write!(
                formatter,
                "rigid-body slot {slot} has no explicitly retained fourth inertia lane"
            ),
            Self::CorruptStructure => formatter.write_str("rigid-body pool structure is corrupt"),
        }
    }
}

impl Error for SimulationRigidBodyPoolError {}

#[derive(Debug)]
pub(crate) enum SimulationRigidBodyDisplacementError<N> {
    Body(SimulationRigidBodyPoolError),
    Numeric(N),
}
impl<N: fmt::Debug> fmt::Display for SimulationRigidBodyDisplacementError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Body(error) => write!(f, "displacement body: {error}"),
            Self::Numeric(error) => write!(f, "displacement reciprocal: {error:?}"),
        }
    }
}

#[path = "external_player_driver.rs"]
mod external_player_driver;

/// Preallocated semantic body pool with four index-backed circular lists.
#[derive(Debug)]
pub struct SimulationRigidBodyPool {
    externally_driven: Vec<SimulationRigidBodyObservation>,
    mode_body_views: Vec<external_player_driver::ModeBodyView>,
    authority: SimulationRigidBodyPoolAuthority,
    /// Host-only ring validation is invariant between topology changes. Body
    /// reads still validate their exact authority/occurrence on every access.
    /// Avoid allocating/walking the entire capacity for every numeric read.
    validated_topology: OnceLock<()>,
    slots: Box<[RigidBodySlot]>,
    solver_delta_rows: Box<[SimulationRigidBodySolverDeltaRow]>,
    anchors: [Links; 4],
    counts: SimulationRigidBodyCounts,
    next_occurrence: Option<NonZeroU64>,
    gravity: [f32; 3],
    cooldown: u32,
    timestep: f32,
    frequency: f32,
    minimum_energy: f32,
    frame_identity: Option<SimulationRigidBodyFrameIdentity>,
    next_frame_identity: Option<NonZeroU64>,
    next_dynamic_update_phase_identity: Option<NonZeroU64>,
}

/// Fully preflighted next native-shaped rigid-body acquisition.
///
/// Preparing this value validates the current rings, selects the exact Free
/// head and destination list, and reserves the next host occurrence without
/// mutating the pool. Dropping it is inert. Consuming it through `commit`
/// performs the existing `AddRigidBody` write graph with no recoverable branch,
/// allocation, or callback after the first mutation.
#[must_use = "dropping an Add reservation leaves the rigid-body pool unchanged"]
pub(crate) struct SimulationRigidBodyAddReservation<'pool> {
    pool: &'pool mut SimulationRigidBodyPool,
    id: SimulationRigidBodyId,
    occurrence: NonZeroU64,
    next_occurrence: Option<NonZeroU64>,
    destination: SimulationRigidBodyList,
}

impl SimulationRigidBodyAddReservation<'_> {
    /// Selected Free slot, without granting a live occurrence before commit.
    #[must_use]
    pub(crate) const fn id(&self) -> SimulationRigidBodyId {
        self.id
    }

    /// Whether native exact-state dispatch selected the Static branch.
    pub(crate) const fn is_static(&self) -> bool {
        matches!(self.destination, SimulationRigidBodyList::Static)
    }

    /// Applies the already-preflighted body acquisition and returns its exact
    /// host allocation occurrence.
    pub(crate) fn commit(
        self,
        matrix: [f32; 16],
        inertia: Option<&Arc<SimulationInertia>>,
    ) -> SimulationRigidBodyHandle {
        let value = if self.is_static() {
            None
        } else {
            inertia.map(|value| **value)
        };
        let retained = if self.is_static() {
            None
        } else {
            inertia.cloned().map(RetainedInertia::Immutable)
        };
        self.commit_retained(matrix, retained, value)
    }

    fn commit_retained(
        self,
        matrix: [f32; 16],
        retained_inertia: Option<RetainedInertia>,
        inertia_value: Option<SimulationInertia>,
    ) -> SimulationRigidBodyHandle {
        let Self {
            pool,
            id,
            occurrence,
            next_occurrence,
            destination,
        } = self;
        let slot_node = SimulationRigidBodyNode::Body(id);

        pool.counts.free = pool.counts.free.wrapping_sub(1);
        pool.slots[id.0 as usize].reset_native_fields();
        pool.slots[id.0 as usize].state = destination.state();
        pool.unlink(slot_node);
        pool.append_to_tail(slot_node, destination);

        {
            let slot = &mut pool.slots[id.0 as usize];
            slot.inertia = retained_inertia;
            match destination {
                SimulationRigidBodyList::Static => {
                    slot.cooldown = pool.cooldown;
                }
                SimulationRigidBodyList::Frozen => {
                    slot.cooldown = pool.cooldown;
                    slot.kinematic_value = f32::MAX;
                    slot.force = pool.gravity;
                }
                SimulationRigidBodyList::Active => {
                    slot.kinematic_value = f32::MAX;
                    slot.force = pool.gravity;
                }
                SimulationRigidBodyList::Free => unreachable!("add destination is never Free"),
            }
        }

        let orthonormalized = orthonormalize_collision_affine(matrix);
        let quaternion = simulation_quaternion_from_orthonormalized_matrix_host(orthonormalized);
        let slot = &mut pool.slots[id.0 as usize];
        slot.right = [orthonormalized[0], orthonormalized[1], orthonormalized[2]];
        slot.up = [orthonormalized[4], orthonormalized[5], orthonormalized[6]];
        slot.at = [orthonormalized[8], orthonormalized[9], orthonormalized[10]];
        slot.center_of_mass = [
            orthonormalized[12],
            orthonormalized[13],
            orthonormalized[14],
        ];
        slot.quaternion = quaternion;
        if let Some(inertia) = inertia_value {
            let rotated = simulation_rotated_inverse_inertia_host(
                orthonormalized,
                inertia.inverse_tensor,
                inertia.inverse_mass,
            );
            slot.inverse_inertia_full = rotated.full;
            slot.inverse_inertia_split = rotated.split;
            slot.inverse_mass = rotated.inverse_mass;
        }
        *pool.counts.get_mut(destination) = pool.counts.get(destination).wrapping_add(1);
        slot.occurrence = Some(occurrence);
        pool.next_occurrence = next_occurrence;

        SimulationRigidBodyHandle {
            authority: pool.authority,
            id,
            occurrence,
        }
    }
}

impl SimulationRigidBodyPool {
    /// Constructs the four rings, inserts slots into Free in ascending order,
    /// and installs Initialize's exact gravity and integer cooldown defaults.
    ///
    /// Allocation and process-authority exhaustion are reported before a pool
    /// is published. Slot IDs are stable host indices; native `mId` values are
    /// pointers into the separate 64-byte-stride RF table and are not modeled
    /// as integers or truncated host pointers.
    ///
    /// # Errors
    ///
    /// Returns a typed error if `capacity` does not fit the native `u32` count,
    /// host allocation fails, or the process-unique authority space is exhausted.
    pub fn try_new(capacity: usize) -> Result<Self, SimulationRigidBodyPoolError> {
        let capacity_u32 = u32::try_from(capacity)
            .map_err(|_| SimulationRigidBodyPoolError::CapacityExceedsU32 { capacity })?;
        let mut storage = Vec::new();
        storage
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationRigidBodyPoolError::AllocationFailed { capacity })?;
        let mut solver_delta_rows = Vec::new();
        solver_delta_rows
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationRigidBodyPoolError::AllocationFailed { capacity })?;
        solver_delta_rows.resize(capacity, SimulationRigidBodySolverDeltaRow::ZERO);
        let authority = fresh_pool_authority(&NEXT_POOL_AUTHORITY)?;
        for index in 0..capacity_u32 {
            let id = SimulationRigidBodyId(index);
            let left = if index == 0 {
                SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Free)
            } else {
                SimulationRigidBodyNode::Body(SimulationRigidBodyId(index - 1))
            };
            let right = if index + 1 == capacity_u32 {
                SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Free)
            } else {
                SimulationRigidBodyNode::Body(SimulationRigidBodyId(index + 1))
            };
            storage.push(RigidBodySlot::initialized(
                id,
                authority,
                Links { right, left },
            ));
        }

        let mut anchors = SimulationRigidBodyList::ALL.map(|list| {
            let node = SimulationRigidBodyNode::Anchor(list);
            Links {
                right: node,
                left: node,
            }
        });
        if capacity_u32 != 0 {
            anchors[SimulationRigidBodyList::Free.index()] = Links {
                right: SimulationRigidBodyNode::Body(SimulationRigidBodyId(0)),
                left: SimulationRigidBodyNode::Body(SimulationRigidBodyId(capacity_u32 - 1)),
            };
        }

        Ok(Self {
            authority,
            validated_topology: OnceLock::new(),
            externally_driven: Vec::new(),
            mode_body_views: Vec::new(),
            slots: storage.into_boxed_slice(),
            solver_delta_rows: solver_delta_rows.into_boxed_slice(),
            anchors,
            counts: SimulationRigidBodyCounts {
                free: capacity_u32,
                ..SimulationRigidBodyCounts::default()
            },
            next_occurrence: NonZeroU64::new(1),
            gravity: SIMULATION_INITIAL_GRAVITY,
            cooldown: SIMULATION_INITIAL_COOLDOWN,
            // Native callers install both values through StartFrame before a
            // tick. Zero is a deterministic safe-host unstarted policy, not a
            // recovered native pre-StartFrame payload claim.
            timestep: 0.0,
            frequency: 0.0,
            minimum_energy: SIMULATION_INITIAL_MINIMUM_ENERGY,
            frame_identity: NonZeroU64::new(1).map(SimulationRigidBodyFrameIdentity),
            next_frame_identity: NonZeroU64::new(2),
            next_dynamic_update_phase_identity: NonZeroU64::new(1),
        })
    }

    #[must_use]
    pub const fn authority(&self) -> SimulationRigidBodyPoolAuthority {
        self.authority
    }

    /// Checks whether this pool is still at its manager-construction boundary.
    ///
    /// The manager's retained static body is created during native manager
    /// construction, after workspace/island setup and before any frame or
    /// gameplay body.
    /// This private predicate prevents a later emptied or already-phased pool
    /// from being relabeled as a fresh manager domain.
    pub(crate) fn try_is_manager_construction_pristine(
        &self,
    ) -> Result<bool, SimulationRigidBodyPoolError> {
        self.validate_integrity()?;
        Ok(self.counts.free as usize == self.slots.len()
            && self.counts.static_bodies == 0
            && self.counts.frozen == 0
            && self.counts.active == 0
            && self.next_occurrence == NonZeroU64::new(1)
            && self.frame_identity == NonZeroU64::new(1).map(SimulationRigidBodyFrameIdentity)
            && self.next_frame_identity == NonZeroU64::new(2)
            && self.next_dynamic_update_phase_identity == NonZeroU64::new(1))
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "reserved for the owning StartFrame island map")
    )]
    pub(crate) fn slot_capacity(&self) -> usize {
        self.slots.len()
    }

    #[must_use]
    pub const fn counts(&self) -> SimulationRigidBodyCounts {
        self.counts
    }

    #[must_use]
    pub const fn gravity(&self) -> [f32; 3] {
        self.gravity
    }

    pub fn set_gravity(&mut self, gravity: [f32; 3]) {
        self.gravity = gravity;
    }

    #[must_use]
    pub const fn cooldown(&self) -> u32 {
        self.cooldown
    }

    pub fn set_cooldown(&mut self, cooldown: u32) {
        self.cooldown = cooldown;
    }

    #[must_use]
    pub const fn timestep(&self) -> f32 {
        self.timestep
    }

    #[must_use]
    pub const fn frequency(&self) -> f32 {
        self.frequency
    }

    #[must_use]
    pub const fn minimum_energy(&self) -> f32 {
        self.minimum_energy
    }

    pub fn set_minimum_energy(&mut self, minimum_energy: f32) {
        self.minimum_energy = minimum_energy;
    }

    /// Installs the two scalar values written by `SimulationManager::StartFrame`.
    ///
    /// The native graph stores `timestep`, computes `1.0 / timestep`, then
    /// stores the frequency. This narrow method does not model `StartFrame`'s
    /// manager counters, jobs, or island bookkeeping. It deliberately performs
    /// no finite/nonzero validation so ordinary inputs retain the source graph;
    /// exceptional division behavior remains a host/Xenon boundary. Advancing
    /// the private host epoch invalidates frame-bound island preparations; that
    /// identity is ABA protection, not a recovered native field.
    pub fn begin_dynamic_update_frame(&mut self, timestep: f32) {
        self.timestep = timestep;
        self.frequency = 1.0 / timestep;
        let Some(frame) = self.next_frame_identity else {
            self.frame_identity = None;
            return;
        };
        self.frame_identity = Some(SimulationRigidBodyFrameIdentity(frame));
        self.next_frame_identity = frame.get().checked_add(1).and_then(NonZeroU64::new);
    }

    /// A host identity for bodies created between frames (`SkaterManager::OnCommand`'s
    /// spawn precedes the next `StartFrame`). Only an absent identity advances; the next
    /// `begin_dynamic_update_frame` renews it as usual.
    pub fn ensure_publication_frame(&mut self) {
        if self.frame_identity.is_some() {
            return;
        }
        let Some(frame) = self.next_frame_identity else {
            return;
        };
        self.frame_identity = Some(SimulationRigidBodyFrameIdentity(frame));
        self.next_frame_identity = frame.get().checked_add(1).and_then(NonZeroU64::new);
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "retained for pool-level evidence tests")
    )]
    pub(crate) const fn frame_identity(&self) -> Option<SimulationRigidBodyFrameIdentity> {
        self.frame_identity
    }

    /// Adds one body using exact raw-state dispatch: `1` Static, `2` Frozen,
    /// and every other value Active with stored state `4`.
    ///
    /// `Ok(None)` is the native empty-free-list result. Host occurrence and
    /// structural checks happen before the first mutation. Once mutation starts,
    /// this method has no recoverable branch or callback.
    ///
    /// # Errors
    ///
    /// Returns an error without pool effects if the retained ring is corrupt or
    /// the host-only non-reused occurrence space has been exhausted.
    pub fn try_add_rigid_body(
        &mut self,
        matrix: [f32; 16],
        inertia: Option<&Arc<SimulationInertia>>,
        raw_state: u32,
    ) -> Result<Option<SimulationRigidBodyHandle>, SimulationRigidBodyPoolError> {
        Ok(self
            .try_prepare_add_rigid_body(raw_state)?
            .map(|reservation| reservation.commit(matrix, inertia)))
    }

    /// Same Add graph with mutable source storage. Host lock/authority refusal
    /// precedes mutation; native empty capacity and Static skip payload reads.
    pub(crate) fn try_add_rigid_body_mutable(
        &mut self,
        matrix: [f32; 16],
        inertia: Option<&SimulationMutableInertia>,
        raw_state: u32,
    ) -> Result<Option<SimulationRigidBodyHandle>, SimulationRigidBodyPoolError> {
        let Some(reservation) = self.try_prepare_add_rigid_body(raw_state)? else {
            return Ok(None);
        };
        if reservation.is_static() || inertia.is_none() {
            return Ok(Some(reservation.commit_retained(matrix, None, None)));
        }
        let owner = inertia.expect("non-null mutable inertia branch");
        let mut state = owner
            .0
            .lock()
            .map_err(|_| SimulationRigidBodyPoolError::InertiaLockPoisoned)?;
        let authority = reservation.pool.authority;
        if state.pool.is_some_and(|pool| pool != authority) {
            return Err(SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch);
        }
        // No fallible work or external callback after publication begins.
        let handle = reservation.commit_retained(
            matrix,
            Some(RetainedInertia::Mutable(owner.clone())),
            Some(state.value),
        );
        state.pool = Some(authority);
        Ok(Some(handle))
    }

    /// Source in-place inertia writes (e.g. `SkeletonBody::SetUpNormal`). Exact
    /// expected shared storage and live occurrence are checked before locking.
    /// The caller supplies a complete explicit value retaining untouched fields;
    /// this does not update derived body tensors, RF, topology, motion or tags.
    /// An explicitly retained fourth tensor lane is left unchanged.
    pub(crate) fn try_write_mutable_inertia(
        &mut self,
        handle: SimulationRigidBodyHandle,
        expected: &SimulationMutableInertia,
        value: SimulationInertia,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.try_write_mutable_inertia_fields(handle, expected, value, None)
    }

    /// Retains the full tensor copied by `SkeletonBody::SetUpNormal` at
    /// `0x82b50614`, including W bits independently of XYZ. The checked host
    /// transaction publishes value and W under one lock, not the native scalar
    /// store prefix. It cannot promote an XYZ-only owner to packed storage.
    pub(crate) fn try_write_mutable_packed_inertia(
        &mut self,
        handle: SimulationRigidBodyHandle,
        expected: &SimulationMutableInertia,
        value: SimulationInertia,
        w: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.try_write_mutable_inertia_fields(handle, expected, value, Some(w))
    }

    fn try_write_mutable_inertia_fields(
        &mut self,
        handle: SimulationRigidBodyHandle,
        expected: &SimulationMutableInertia,
        value: SimulationInertia,
        updated_w: Option<f32>,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        let Some(RetainedInertia::Mutable(owner)) = &self.slots[index].inertia else {
            return Err(SimulationRigidBodyPoolError::ImmutableInertia { slot: handle.id.0 });
        };
        if !Arc::ptr_eq(&owner.0, &expected.0) {
            return Err(SimulationRigidBodyPoolError::InertiaOwnerMismatch { slot: handle.id.0 });
        }
        let mut state = owner
            .0
            .lock()
            .map_err(|_| SimulationRigidBodyPoolError::InertiaLockPoisoned)?;
        if state.pool != Some(self.authority) {
            return Err(SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch);
        }
        if updated_w.is_some() && state.inverse_tensor_w.is_none() {
            return Err(SimulationRigidBodyPoolError::PackedInertiaUnavailable {
                slot: handle.id.0,
            });
        }
        state.value = value;
        if let Some(w) = updated_w {
            state.inverse_tensor_w = Some(w);
        }
        Ok(())
    }

    /// Preflights one native-shaped body acquisition without mutating the pool.
    ///
    /// The empty-Free result precedes ring, occurrence, state, matrix, and
    /// Inertia inspection, matching `AddRigidBody`'s early return. For a
    /// nonempty pool, the complete rings and next host occurrence are checked
    /// before a reservation is returned. Exact state `1` selects Static,
    /// exact `2` selects Frozen, and every other word selects Active.
    pub(crate) fn try_prepare_add_rigid_body(
        &mut self,
        raw_state: u32,
    ) -> Result<Option<SimulationRigidBodyAddReservation<'_>>, SimulationRigidBodyPoolError> {
        if self.counts.free == 0 {
            return Ok(None);
        }
        self.validate_integrity()?;
        let occurrence = self
            .next_occurrence
            .ok_or(SimulationRigidBodyPoolError::OccurrenceExhausted)?;
        let next_occurrence = occurrence.get().checked_add(1).and_then(NonZeroU64::new);
        let free_anchor = SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Free);
        let slot_node = self.links(free_anchor)?.right;
        let SimulationRigidBodyNode::Body(id) = slot_node else {
            return Err(SimulationRigidBodyPoolError::CorruptStructure);
        };
        let destination = if raw_state == 1 {
            SimulationRigidBodyList::Static
        } else if raw_state == 2 {
            SimulationRigidBodyList::Frozen
        } else {
            SimulationRigidBodyList::Active
        };

        Ok(Some(SimulationRigidBodyAddReservation {
            pool: self,
            id,
            occurrence,
            next_occurrence,
            destination,
        }))
    }

    /// Returns a host affine pose for an allocated body.
    ///
    /// XYZ values are the native semantic body fields. Axis W lanes are `+0`
    /// and translation W is `1`; native packed adjacent ID/link/pointer words
    /// are not floating matrix lanes and have no portable host representation.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a foreign, out-of-range, stale, or free handle,
    /// or if the retained ring fails safe-host structural validation.
    pub fn try_rigid_body_pose(
        &self,
        handle: SimulationRigidBodyHandle,
    ) -> Result<[f32; 16], SimulationRigidBodyPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        if let Some(view) = self.mode_body_view(handle.observation()) { return Ok(view.transform); }
        let slot = &self.slots[index];
        Ok([
            slot.right[0],
            slot.right[1],
            slot.right[2],
            0.0,
            slot.up[0],
            slot.up[1],
            slot.up[2],
            0.0,
            slot.at[0],
            slot.at[1],
            slot.at[2],
            0.0,
            slot.center_of_mass[0],
            slot.center_of_mass[1],
            slot.center_of_mass[2],
            1.0,
        ])
    }

    /// Body write set of attached `Part::SetTransform` (0x82b25480).
    /// The caller has already applied the optional inverse Part frame.
    /// Existing occurrences, rings, motion, RF and retained inertia ownership
    /// survive. XYZ stores preserve the separately typed native packed fields.
    /// Pure infallible host numerics expose no callback interleavings with the
    /// source's early inertia capture and later inverse-mass pointer reload.
    pub(crate) fn try_set_part_body_transform_host(
        &mut self,
        handle: SimulationRigidBodyHandle,
        transform: [f32; 16],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        let inertia = self.slots[index]
            .inertia
            .as_ref()
            .map(RetainedInertia::try_read)
            .transpose()?;
        let normalized = orthonormalize_collision_affine(transform);
        let slot = &mut self.slots[index];
        slot.right = [normalized[0], normalized[1], normalized[2]];
        slot.up = [normalized[4], normalized[5], normalized[6]];
        slot.at = [normalized[8], normalized[9], normalized[10]];
        slot.center_of_mass = [normalized[12], normalized[13], normalized[14]];
        // Exclusive pool ownership and sealed mutable writers rule out source
        // pointer/value replacement during the pure quaternion helper.
        slot.quaternion = simulation_quaternion_from_orthonormalized_matrix_host(normalized);
        if let Some(inertia) = inertia {
            let rotated = simulation_rotated_inverse_inertia_host(
                normalized,
                inertia.inverse_tensor,
                inertia.inverse_mass,
            );
            slot.inverse_inertia_full = rotated.full;
            slot.inverse_inertia_split = rotated.split;
            slot.inverse_mass = rotated.inverse_mass;
        }
        Ok(())
    }

    /// `ResetSkeleton`82b45d58..82b45d84 after its Part transform update.
    /// XYZ-only motion stores preserve allocation, rings, packed adjacent
    /// fields, inertia and RF rows. This pool owns the body's immutable stasis
    /// identity; its current gravity is the source stasis+90 vector.
    pub(crate) fn try_reset_part_body_motion_host(
        &mut self,
        handle: SimulationRigidBodyHandle,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        let slot = &mut self.slots[index];
        slot.velocity = [0.0; 3];
        slot.omega = [0.0; 3];
        // Read current stasis gravity only after both earlier source stores.
        // No nullable/replaced stasis collaborator exists in this pool model.
        slot.force = self.gravity;
        slot.torque = [0.0; 3];
        Ok(())
    }

    /// `SkeletonBody::ApplyDisplacementToPart` 0x82b545e0..0x82b546c0.
    /// Captures actual cached mass/force before the per-Part numerical call.
    /// Its refusal preserves this body; successful XYZ stores precede cooldown.
    pub(crate) fn try_apply_part_body_displacement<
        N: crate::reckoning_numeric::ReckoningNumeric,
    >(
        &mut self,
        handle: SimulationRigidBodyHandle,
        displacement: [f32; 4],
        numeric: &mut N,
    ) -> Result<(), SimulationRigidBodyDisplacementError<N::Error>> {
        use SimulationRigidBodyDisplacementError as E;
        let index = self.validate_handle(handle).map_err(E::Body)?;
        self.validate_integrity().map_err(E::Body)?;
        let slot = &mut self.slots[index];
        let inverse_mass = slot.inverse_mass;
        let old_force = slot.force;
        let force = crate::skeleton_general_force::try_accumulate_force(
            displacement,
            inverse_mass,
            old_force,
            numeric,
        )
        .map_err(E::Numeric)?;
        slot.force[0] = force[0];
        slot.force[1] = force[1];
        slot.force[2] = force[2];
        slot.cooldown = 0;
        Ok(())
    }

    /// Borrows an immutable retained inertia capability for an allocated body.
    /// Mutable storage explicitly refuses this immutable-only view; it never
    /// masquerades as a null native pointer. Use a checked current snapshot.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a foreign, out-of-range, stale, or free handle,
    /// or if the retained ring fails safe-host structural validation.
    pub fn try_inertia(
        &self,
        handle: SimulationRigidBodyHandle,
    ) -> Result<Option<&Arc<SimulationInertia>>, SimulationRigidBodyPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        match self.slots[index].inertia.as_ref() {
            None => Ok(None),
            Some(RetainedInertia::Immutable(value)) => Ok(Some(value)),
            Some(RetainedInertia::Mutable(_)) => Err(
                SimulationRigidBodyPoolError::MutableInertiaRequiresSnapshot { slot: handle.id.0 },
            ),
        }
    }

    /// Validates an allocation handle and returns a read-only occurrence-bound
    /// capability. The returned type is deliberately not accepted by removal.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a foreign, out-of-range, stale, or free handle,
    /// or if the retained ring fails safe-host structural validation.
    pub fn try_observe(
        &self,
        handle: SimulationRigidBodyHandle,
    ) -> Result<SimulationRigidBodyObservation, SimulationRigidBodyPoolError> {
        self.validate_handle(handle)?;
        self.validate_integrity()?;
        Ok(handle.observation())
    }

    /// Reads the raw body-state word through an occurrence-bound observation.
    ///
    /// # Errors
    ///
    /// Returns a typed error if the observation is foreign, stale, free, or
    /// structurally invalid.
    pub fn try_raw_state(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<u32, SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        Ok(self.slots[index].state)
    }

    /// State-store prefix of `Island::RegisterSpyTarget` (0x827b7874).
    /// Requires the actual allocation capability; a read-only observation does
    /// not authorize mutation. Island mapping is a separate required suffix.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "source state prefix awaits actual Island owner registration"
        )
    )]
    pub(crate) fn try_register_contact_spy(
        &mut self,
        handle: SimulationRigidBodyHandle,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        self.slots[index].state |= 8;
        Ok(())
    }

    /// State-store prefix of `Island::RemoveSpyTarget` (0x827b5660).
    /// Clears ALL bits above the low three, even when no mapping exists.
    /// Neither operation moves the body between base-state rings.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "source state prefix awaits actual Island owner cleanup"
        )
    )]
    pub(crate) fn try_remove_contact_spy(
        &mut self,
        handle: SimulationRigidBodyHandle,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        self.slots[index].state &= 7;
        Ok(())
    }

    /// Revalidates one occurrence-qualified body without sampling numeric state.
    ///
    /// Contact producer queues use this narrow boundary because native Contact
    /// admission has no body-state predicate. The complete retained ring is
    /// still checked before the observation is accepted.
    pub(crate) fn try_validate_observation(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.validate_observation(observation)?;
        self.validate_integrity()
    }

    /// Current semantic XYZ read for `SpyContactJacobians` (0x82ae4b40/4b60).
    /// Native loads a fourth packed word too; no host pointer bits are invented.
    pub(crate) fn try_contact_spy_center_of_mass(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<[f32; 3], SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        Ok(self.mode_body_view(observation).map_or(self.slots[index].center_of_mass, |view| view.center_of_mass()))
    }

    /// Current XYZ linear velocity for Island contact-list creation: A's side
    /// reads B then A (827ba954/95c), B's side rereads A then B (827baa54/a5c).
    /// Native W contains an intrusive link, not a semantic velocity component.
    pub(crate) fn try_contact_spy_linear_velocity(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<[f32; 3], SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        Ok(self.mode_body_view(observation).map_or(self.slots[index].velocity, |view| view.velocity))
    }

    /// `SkeletonCollision::Update` reads cached `RigidBody.mInvm`+0x7c at
    /// 0x82b4c044/0x82b4c054, NOT the current retained inertia owner's value.
    /// The caller first uses `try_raw_state` and substitutes its source constant
    /// only for EXACT state1, skipping this read. Registered Static9 is distinct.
    /// Occurrence/ring validation is host lifetime policy; no value is remapped,
    /// normalized, clamped, or copied back into the body/owner.
    pub(crate) fn try_contact_response_cached_inverse_mass(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<f32, SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        Ok(self.slots[index].inverse_mass)
    }

    /// Read-only Active right-link traversal for the manager's `StartFrame`
    /// island-label stores (`0x82e0bde4..0x82e0be14`). Numeric rows are not read.
    pub(crate) fn try_for_each_active_observation(
        &self,
        mut visit: impl FnMut(SimulationRigidBodyObservation),
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.validate_integrity()?;
        let anchor = SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Active);
        let mut node = self.anchors[SimulationRigidBodyList::Active.index()].right;
        for _ in 0..self.counts.active {
            let SimulationRigidBodyNode::Body(id) = node else {
                unreachable!("validated Active ring contains body nodes");
            };
            let slot = &self.slots[id.0 as usize];
            visit(SimulationRigidBodyObservation {
                authority: self.authority,
                id,
                occurrence: slot
                    .occurrence
                    .expect("validated allocated body occurrence"),
            });
            node = slot.links.right;
        }
        debug_assert_eq!(node, anchor);
        Ok(())
    }

    /// Host publication view binding persistent per-slot data to CURRENT
    /// allocated occurrences. Slot order is storage order, not execution order.
    pub(crate) fn try_for_each_allocated_observation(
        &self,
        mut visit: impl FnMut(SimulationRigidBodyObservation),
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.validate_integrity()?;
        for slot in &self.slots {
            if slot.membership != SimulationRigidBodyList::Free {
                visit(SimulationRigidBodyObservation {
                    authority: self.authority,
                    id: slot.id,
                    occurrence: slot
                        .occurrence
                        .expect("validated allocated body occurrence"),
                });
            }
        }
        Ok(())
    }

    /// Copies the complete portable read set used by CPU
    /// `SerialisedAssembly::Create` after validating this exact allocation
    /// occurrence and the pool rings.
    pub(crate) fn try_serialised_assembly_snapshot(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<SimulationRigidBodySerialisedAssemblySnapshot, SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        let slot = &self.slots[index];
        let view = self.mode_body_view(observation);
        Ok(SimulationRigidBodySerialisedAssemblySnapshot {
            observation,
            state: slot.state,
            center_of_mass: view.map_or(slot.center_of_mass, |view| view.center_of_mass()),
            velocity: view.map_or(slot.velocity, |view| view.velocity),
            omega: view.map_or(slot.omega, |view| view.omega),
            right: view.map_or(slot.right, |view| view.axis(0)),
            up: view.map_or(slot.up, |view| view.axis(1)),
            at: view.map_or(slot.at, |view| view.axis(2)),
            tag: slot.tag,
            force: slot.force,
            kinematic_value: slot.kinematic_value,
            torque: slot.torque,
            cooldown: slot.cooldown,
        })
    }

    /// Copies the current address-shaped ring inputs needed by the future
    /// `SerialisedAssembly` packed-word provider.
    ///
    /// The exact allocation occurrence and the complete four-ring structure
    /// are validated before any field is returned. The immutable pool borrow
    /// keeps the occurrence and links stable through this copy. This method
    /// deliberately returns native node identities rather than host or Xenon
    /// addresses; address projection belongs to the separately bound
    /// [`SimulationXenonAddressImage`](crate::simulation_xenon_address_image::SimulationXenonAddressImage).
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for the private SerialisedAssembly Xenon packed-word provider"
        )
    )]
    pub(crate) fn try_packed_word_topology_snapshot(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<SimulationRigidBodyPackedWordTopologySnapshot, SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        let slot = &self.slots[index];
        Ok(SimulationRigidBodyPackedWordTopologySnapshot {
            observation,
            right_link: slot.links.right,
            left_link: slot.links.left,
            has_retained_inertia: slot.inertia.is_some(),
        })
    }

    /// Copies the exact numeric body fields consumed by Drive Jacobian Build.
    ///
    /// Authority, slot occurrence, allocation state, and the complete retained
    /// ring structure are validated before the slot is copied. The immutable
    /// pool borrow then keeps that validated occurrence stable through the
    /// copy. No retained `Arc`, solver row, or mutation capability crosses the
    /// return boundary.
    ///
    /// # Errors
    ///
    /// Returns a typed error if the observation is foreign, stale, free, or
    /// structurally invalid.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for the private Drive batch preparation seam"
        )
    )]
    pub(crate) fn try_drive_build_snapshot(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<SimulationRigidBodyDriveBuildSnapshot, SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        let slot = &self.slots[index];
        let view = self.mode_body_view(observation);
        Ok(SimulationRigidBodyDriveBuildSnapshot {
            observation,
            state: slot.state,
            quaternion: view.map_or(slot.quaternion, |view| view.quaternion),
            center_of_mass: view.map_or(slot.center_of_mass, |view| view.center_of_mass()),
            velocity: view.map_or(slot.velocity, |view| view.velocity),
            omega: view.map_or(slot.omega, |view| view.omega),
            right: view.map_or(slot.right, |view| view.axis(0)),
            up: view.map_or(slot.up, |view| view.axis(1)),
            at: view.map_or(slot.at, |view| view.axis(2)),
            force: slot.force,
            torque: slot.torque,
            inverse_inertia_full: view.map_or(slot.inverse_inertia_full, |view| view.inverse_inertia_full),
            inverse_mass: view.map_or(slot.inverse_mass, |view| view.inverse_mass),
            inverse_inertia_split: view.map_or(slot.inverse_inertia_split, |view| view.inverse_inertia_split),
        })
    }

    /// Reads one private RF solver-delta row through an occurrence-bound body
    /// observation.
    ///
    /// The callback boundary deliberately exposes neither a native pointer nor
    /// a public mutation/removal capability. Native row initialization is
    /// recovered at `0x82ae7b80..0x82ae7bb0`; interpreting the four vectors is
    /// deferred until the corresponding solver and `DynamicUpdate` graphs are
    /// implemented.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for the recovered Jacobian and solver slices"
        )
    )]
    pub(crate) fn try_with_solver_delta_row<R>(
        &self,
        observation: SimulationRigidBodyObservation,
        inspect: impl FnOnce(&[[f32; 4]; 4]) -> R,
    ) -> Result<R, SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        Ok(inspect(&self.solver_delta_rows[index].vectors))
    }

    /// Mutates one private RF solver-delta row through an occurrence-bound body
    /// observation.
    ///
    /// This crate-private seam is intended for recovered Jacobian/solver work.
    /// It performs all authority, occurrence, allocation, and ring validation
    /// before invoking the callback. It does not clear or interpret the row.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for the recovered Jacobian and solver slices"
        )
    )]
    pub(crate) fn try_with_solver_delta_row_mut<R>(
        &mut self,
        observation: SimulationRigidBodyObservation,
        mutate: impl FnOnce(&mut [[f32; 4]; 4]) -> R,
    ) -> Result<R, SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        Ok(mutate(&mut self.solver_delta_rows[index].vectors))
    }

    /// Plans an ordered, all-prevalidated RF-row transaction.
    ///
    /// Allocation, complete pool integrity, and every A/B occurrence are
    /// checked before the returned transaction can invoke a solver callback.
    /// Repeated and equal indices are retained exactly; no RF values are
    /// copied during this preflight.
    pub(crate) fn try_begin_solver_delta_transaction<I>(
        &mut self,
        pairs: I,
    ) -> Result<
        SimulationRigidBodySolverDeltaTransaction<'_>,
        SimulationRigidBodySolverDeltaTransactionError,
    >
    where
        I: IntoIterator<
            Item = (
                SimulationRigidBodyObservation,
                SimulationRigidBodyObservation,
            ),
        >,
        I::IntoIter: ExactSizeIterator,
    {
        self.try_begin_solver_delta_transaction_with_reservation(
            pairs.into_iter(),
            |plan, pair_count| plan.try_reserve_exact(pair_count).map_err(|_| ()),
        )
    }

    fn try_begin_solver_delta_transaction_with_reservation<I>(
        &mut self,
        pairs: I,
        reserve: impl FnOnce(&mut Vec<(usize, usize)>, usize) -> Result<(), ()>,
    ) -> Result<
        SimulationRigidBodySolverDeltaTransaction<'_>,
        SimulationRigidBodySolverDeltaTransactionError,
    >
    where
        I: ExactSizeIterator<
            Item = (
                SimulationRigidBodyObservation,
                SimulationRigidBodyObservation,
            ),
        >,
    {
        let pair_count = pairs.len();
        let mut plan = Vec::new();
        reserve(&mut plan, pair_count).map_err(|()| {
            SimulationRigidBodySolverDeltaTransactionError::AllocationFailed { pair_count }
        })?;
        if plan.capacity() < pair_count {
            return Err(
                SimulationRigidBodySolverDeltaTransactionError::AllocationFailed { pair_count },
            );
        }
        self.validate_integrity()
            .map_err(SimulationRigidBodySolverDeltaTransactionError::Pool)?;
        for (ordinal, (body_a, body_b)) in pairs.enumerate() {
            let index_a = self.validate_observation(body_a).map_err(|source| {
                SimulationRigidBodySolverDeltaTransactionError::Body {
                    ordinal,
                    endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyA,
                    source,
                }
            })?;
            let index_b = self.validate_observation(body_b).map_err(|source| {
                SimulationRigidBodySolverDeltaTransactionError::Body {
                    ordinal,
                    endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyB,
                    source,
                }
            })?;
            plan.push((index_a, index_b));
        }
        Ok(SimulationRigidBodySolverDeltaTransaction {
            pool: self,
            pairs: plan,
        })
    }

    /// Visits private RF rows for Active bodies in native right-link order.
    ///
    /// `Simulation::BatchIntegrator` (`0x82ae4a40..0x82ae4a84`) is a do/while,
    /// but the manager suppresses its job when Active count is zero. This host
    /// coordinator preserves that outer guard, validates the complete retained
    /// structure before the first callback, and never visits Static or Frozen
    /// bodies. It intentionally performs no integration or solver behavior.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "reserved for the recovered DynamicUpdate slice")
    )]
    pub(crate) fn try_for_each_active_solver_delta_row(
        &mut self,
        mut visit: impl FnMut(SimulationRigidBodyObservation, &mut [[f32; 4]; 4]),
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.validate_integrity()?;
        if self.counts.active == 0 {
            return Ok(());
        }

        let anchor = SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Active);
        let mut node = self.anchors[SimulationRigidBodyList::Active.index()].right;
        for _ in 0..self.counts.active {
            let SimulationRigidBodyNode::Body(id) = node else {
                unreachable!("validated nonempty Active ring contains only body nodes");
            };
            let index = id.0 as usize;
            let slot = &self.slots[index];
            let occurrence = slot
                .occurrence
                .expect("validated allocated body has an occurrence");
            let observation = SimulationRigidBodyObservation {
                authority: self.authority,
                id,
                occurrence,
            };
            node = slot.links.right;
            visit(observation, &mut self.solver_delta_rows[index].vectors);
        }
        debug_assert_eq!(node, anchor);
        Ok(())
    }

    /// Applies the recovered ordinary-finite `DynamicUpdate` graph to every
    /// Active body in right-link order.
    ///
    /// This is an unconstrained post-solver integration seam. It does not run
    /// Jacobians, joint/drive solving, collision, islands, or manager jobs and
    /// therefore must not be treated as a complete Simulation tick. Before the
    /// first body write it validates all four retained rings, reserves a full
    /// Active-order plan, and proves that every Active body has inertia. The
    /// consumed phase then allocates nothing and has no recoverable branch:
    /// each body is committed in native store order, its complete RF row is
    /// cleared, its force becomes the preflighted pool gravity, and its torque
    /// is cleared.
    ///
    /// Exact Xenon estimate/reduction, denormal, NaN-payload, infinity, and
    /// exceptional signed-zero results remain outside this safe-host model.
    ///
    /// # Errors
    ///
    /// Returns without body or RF-row mutation if any retained ring is corrupt,
    /// the safe-host Active plan cannot be allocated, any Active body lacks an
    /// inertia payload, or a private nonreused frame- or phase-identity space
    /// has been exhausted.
    ///
    /// # Panics
    ///
    /// A panic in the ordinary-finite numeric graph retains the already
    /// integrated body/RF prefix. No rollback is attempted.
    pub fn try_integrate_active_bodies_ordinary_finite(
        &mut self,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let phase = self
            .try_prepare_dynamic_update_phase()
            .map_err(SimulationRigidBodyDynamicUpdatePrepareError::into_pool_error)?;
        phase.integrate_ordinary_finite();
        Ok(())
    }

    /// Preflights every fallible prerequisite for a later `DynamicUpdate`.
    ///
    /// The returned private phase owns the exclusive pool borrow. It can be
    /// retained across restricted RF solving and then consumed without any
    /// returned error, validation, allocation, payload lock, or callback.
    pub(crate) fn try_prepare_dynamic_update_phase(
        &mut self,
    ) -> Result<
        SimulationRigidBodyDynamicUpdatePhase<'_>,
        SimulationRigidBodyDynamicUpdatePrepareError,
    > {
        self.try_prepare_dynamic_update_phase_with_reservation(|active, active_count| {
            active.try_reserve_exact(active_count).map_err(|_| ())
        })
    }

    fn try_prepare_dynamic_update_phase_with_reservation(
        &mut self,
        reserve: impl FnOnce(
            &mut Vec<SimulationRigidBodyDynamicUpdatePlanEntry>,
            usize,
        ) -> Result<(), ()>,
    ) -> Result<
        SimulationRigidBodyDynamicUpdatePhase<'_>,
        SimulationRigidBodyDynamicUpdatePrepareError,
    > {
        self.validate_integrity()
            .map_err(SimulationRigidBodyDynamicUpdatePrepareError::Pool)?;

        let active_count = self.counts.active as usize;
        let mut active = Vec::new();
        reserve(&mut active, active_count).map_err(|()| {
            SimulationRigidBodyDynamicUpdatePrepareError::AllocationFailed { active_count }
        })?;
        if active.capacity() < active_count {
            return Err(
                SimulationRigidBodyDynamicUpdatePrepareError::AllocationFailed { active_count },
            );
        }

        let active_anchor = SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Active);
        let mut node = self.anchors[SimulationRigidBodyList::Active.index()].right;
        for _ in 0..self.counts.active {
            let SimulationRigidBodyNode::Body(id) = node else {
                unreachable!("validated nonempty Active ring contains only body nodes");
            };
            let index = id.0 as usize;
            let slot = &self.slots[index];
            let Some(inertia) = slot
                .inertia
                .as_ref()
                .map(RetainedInertia::try_read)
                .transpose()
                .map_err(SimulationRigidBodyDynamicUpdatePrepareError::Pool)?
            else {
                return Err(SimulationRigidBodyDynamicUpdatePrepareError::Pool(
                    SimulationRigidBodyPoolError::ActiveBodyMissingInertia { slot: id.0 },
                ));
            };
            active.push(SimulationRigidBodyDynamicUpdatePlanEntry { index, inertia });
            node = slot.links.right;
        }
        debug_assert_eq!(node, active_anchor);

        let configuration = SimulationDynamicUpdateConfiguration {
            timestep: self.timestep,
            frequency: self.frequency,
            gravity: self.gravity,
            maximum_cooldown: self.cooldown,
            minimum_energy: self.minimum_energy,
        };
        let frame_identity =
            self.frame_identity
                .ok_or(SimulationRigidBodyDynamicUpdatePrepareError::Pool(
                    SimulationRigidBodyPoolError::FrameIdentityExhausted,
                ))?;
        let phase_identity = self
            .next_dynamic_update_phase_identity
            .ok_or(SimulationRigidBodyDynamicUpdatePrepareError::PhaseIdentityExhausted)?;
        self.next_dynamic_update_phase_identity = phase_identity
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new);
        Ok(SimulationRigidBodyDynamicUpdatePhase {
            pool: self,
            active,
            configuration,
            frame_identity,
            phase_identity: SimulationRigidBodyDynamicUpdatePhaseIdentity(phase_identity),
        })
    }

    /// Removes a body after atomically validating authority, occurrence, and
    /// the complete ring structure. Native-shaped mutation then has no failure
    /// branch and preserves every field except links, state, and membership.
    ///
    /// # Errors
    ///
    /// Returns a typed error before mutation for a foreign, out-of-range, stale,
    /// or free handle, or for a structurally invalid retained ring.
    pub fn try_remove_rigid_body(
        &mut self,
        handle: SimulationRigidBodyHandle,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        self.remove_validated_slot(index);
        Ok(())
    }

    /// Legacy diagnostic snapshot. For checked observation use `try_snapshot`.
    ///
    /// # Panics
    ///
    /// Panics if retained mutable inertia storage has a poisoned lock.
    #[must_use]
    pub fn snapshot(&self) -> SimulationRigidBodyPoolSnapshot {
        self.try_snapshot()
            .expect("retained inertia must be readable for legacy snapshot")
    }

    /// Copies diagnostic state, sampling the currently retained inertia owners.
    ///
    /// # Errors
    ///
    /// Returns an error if any retained mutable inertia lock is poisoned.
    pub fn try_snapshot(
        &self,
    ) -> Result<SimulationRigidBodyPoolSnapshot, SimulationRigidBodyPoolError> {
        Ok(SimulationRigidBodyPoolSnapshot {
            authority: self.authority,
            counts: self.counts,
            next_occurrence: self.next_occurrence.map(NonZeroU64::get),
            next_dynamic_update_phase_identity: self
                .next_dynamic_update_phase_identity
                .map(NonZeroU64::get),
            gravity: self.gravity,
            cooldown: self.cooldown,
            timestep: self.timestep,
            frequency: self.frequency,
            minimum_energy: self.minimum_energy,
            free_order: self.list_order(SimulationRigidBodyList::Free),
            static_order: self.list_order(SimulationRigidBodyList::Static),
            frozen_order: self.list_order(SimulationRigidBodyList::Frozen),
            active_order: self.list_order(SimulationRigidBodyList::Active),
            slots: self
                .slots
                .iter()
                .map(|slot| {
                    Ok(SimulationRigidBodySlotSnapshot {
                        id: slot.id,
                        occurrence: slot.occurrence.map(NonZeroU64::get),
                        quaternion: slot.quaternion,
                        center_of_mass: slot.center_of_mass,
                        velocity: slot.velocity,
                        right_link: slot.links.right,
                        omega: slot.omega,
                        left_link: slot.links.left,
                        right: slot.right,
                        stasis: slot.stasis,
                        up: slot.up,
                        inertia: slot
                            .inertia
                            .as_ref()
                            .map(RetainedInertia::try_read)
                            .transpose()?,
                        at: slot.at,
                        tag: slot.tag,
                        inverse_inertia_full: slot.inverse_inertia_full,
                        inverse_mass: slot.inverse_mass,
                        inverse_inertia_split: slot.inverse_inertia_split,
                        state: slot.state,
                        force: slot.force,
                        kinematic_value: slot.kinematic_value,
                        torque: slot.torque,
                        cooldown: slot.cooldown,
                        membership: slot.membership,
                    })
                })
                .collect::<Result<Vec<_>, SimulationRigidBodyPoolError>>()?,
        })
    }

    fn validate_handle(
        &self,
        handle: SimulationRigidBodyHandle,
    ) -> Result<usize, SimulationRigidBodyPoolError> {
        if handle.authority != self.authority {
            return Err(SimulationRigidBodyPoolError::ForeignHandle);
        }
        let index = handle.id.0 as usize;
        let slot = self
            .slots
            .get(index)
            .ok_or(SimulationRigidBodyPoolError::SlotOutOfRange { slot: handle.id.0 })?;
        if slot.occurrence != Some(handle.occurrence) {
            return Err(SimulationRigidBodyPoolError::StaleHandle { slot: handle.id.0 });
        }
        if slot.membership == SimulationRigidBodyList::Free {
            return Err(SimulationRigidBodyPoolError::BodyIsFree { slot: handle.id.0 });
        }
        Ok(index)
    }

    /// Test probe: raw state word, cooldown and list membership.
    #[cfg(test)]
    pub(crate) fn probe_body(&self, observation: SimulationRigidBodyObservation) -> String {
        match self.validate_observation(observation) {
            Ok(index) => {
                let slot = &self.slots[index];
                format!("state={:#x} cooldown={} list={:?}", slot.state, slot.cooldown, slot.membership)
            }
            Err(e) => format!("{e:?}"),
        }
    }

    fn validate_observation(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<usize, SimulationRigidBodyPoolError> {
        self.validate_handle(SimulationRigidBodyHandle {
            authority: observation.authority,
            id: observation.id,
            occurrence: observation.occurrence,
        })
    }

    fn remove_validated_slot(&mut self, index: usize) {
        let node = SimulationRigidBodyNode::Body(self.slots[index].id);
        self.unlink(node);
        self.append_to_tail(node, SimulationRigidBodyList::Free);
        let source = match self.slots[index].state & 7 {
            1 => SimulationRigidBodyList::Static,
            2 => SimulationRigidBodyList::Frozen,
            _ => SimulationRigidBodyList::Active,
        };
        *self.counts.get_mut(source) = self.counts.get(source).wrapping_sub(1);
        self.slots[index].state = 0;
        self.counts.free = self.counts.free.wrapping_add(1);
    }

    fn unlink(&mut self, node: SimulationRigidBodyNode) {
        let links = self.links(node).expect("validated body node has links");
        self.links_mut(links.right)
            .expect("validated right neighbor")
            .left = links.left;
        self.links_mut(links.left)
            .expect("validated left neighbor")
            .right = links.right;
    }

    fn append_to_tail(&mut self, node: SimulationRigidBodyNode, list: SimulationRigidBodyList) {
        let anchor = SimulationRigidBodyNode::Anchor(list);
        let tail = self.links(anchor).expect("anchor exists").left;
        {
            let links = self.links_mut(node).expect("validated body node");
            links.right = anchor;
            links.left = tail;
        }
        self.links_mut(tail).expect("validated tail").right = node;
        self.links_mut(anchor).expect("anchor exists").left = node;
        if let SimulationRigidBodyNode::Body(id) = node {
            self.slots[id.0 as usize].membership = list;
        }
    }

    fn links(&self, node: SimulationRigidBodyNode) -> Result<Links, SimulationRigidBodyPoolError> {
        match node {
            SimulationRigidBodyNode::Anchor(list) => Ok(self.anchors[list.index()]),
            SimulationRigidBodyNode::Body(id) => self
                .slots
                .get(id.0 as usize)
                .map(|slot| slot.links)
                .ok_or(SimulationRigidBodyPoolError::CorruptStructure),
        }
    }

    fn links_mut(&mut self, node: SimulationRigidBodyNode) -> Option<&mut Links> {
        // All Add/Remove/Activate ring edits pass here. Counts, membership,
        // state and occurrence stores in those operations complete before the
        // next validation. Numeric-only updates do not change these invariants.
        self.validated_topology.take();
        match node {
            SimulationRigidBodyNode::Anchor(list) => self.anchors.get_mut(list.index()),
            SimulationRigidBodyNode::Body(id) => self
                .slots
                .get_mut(id.0 as usize)
                .map(|slot| &mut slot.links),
        }
    }

    fn validate_integrity(&self) -> Result<(), SimulationRigidBodyPoolError> {
        if self.validated_topology.get().is_some() {
            return Ok(());
        }
        let mut seen = vec![false; self.slots.len()];
        for list in SimulationRigidBodyList::ALL {
            let anchor = SimulationRigidBodyNode::Anchor(list);
            let anchor_links = self.links(anchor)?;
            if self.links(anchor_links.right)?.left != anchor
                || self.links(anchor_links.left)?.right != anchor
            {
                return Err(SimulationRigidBodyPoolError::CorruptStructure);
            }
            let mut node = anchor_links.right;
            let mut count = 0_u32;
            while node != anchor {
                let SimulationRigidBodyNode::Body(id) = node else {
                    return Err(SimulationRigidBodyPoolError::CorruptStructure);
                };
                let index = id.0 as usize;
                let Some(slot) = self.slots.get(index) else {
                    return Err(SimulationRigidBodyPoolError::CorruptStructure);
                };
                let state_matches_membership = match list {
                    SimulationRigidBodyList::Free => slot.state == 0,
                    _ => slot.state == list.state() || slot.state == (list.state() | SPY_BODY),
                };
                if seen[index]
                    || slot.id != id
                    || slot.membership != list
                    || !state_matches_membership
                    || slot.stasis != self.authority
                    || (list != SimulationRigidBodyList::Free && slot.occurrence.is_none())
                {
                    return Err(SimulationRigidBodyPoolError::CorruptStructure);
                }
                let links = slot.links;
                if self.links(links.right)?.left != node || self.links(links.left)?.right != node {
                    return Err(SimulationRigidBodyPoolError::CorruptStructure);
                }
                seen[index] = true;
                count = count.wrapping_add(1);
                if count as usize > self.slots.len() {
                    return Err(SimulationRigidBodyPoolError::CorruptStructure);
                }
                node = links.right;
            }
            if count != self.counts.get(list) {
                return Err(SimulationRigidBodyPoolError::CorruptStructure);
            }
        }
        if seen.into_iter().any(|present| !present) {
            return Err(SimulationRigidBodyPoolError::CorruptStructure);
        }
        if self.solver_delta_rows.len() != self.slots.len() {
            return Err(SimulationRigidBodyPoolError::CorruptStructure);
        }
        let _ = self.validated_topology.set(());
        Ok(())
    }

    fn list_order(&self, list: SimulationRigidBodyList) -> Vec<SimulationRigidBodyId> {
        let anchor = SimulationRigidBodyNode::Anchor(list);
        let mut node = self.anchors[list.index()].right;
        let mut result = Vec::new();
        while node != anchor && result.len() < self.slots.len() {
            let SimulationRigidBodyNode::Body(id) = node else {
                break;
            };
            result.push(id);
            let Some(slot) = self.slots.get(id.0 as usize) else {
                break;
            };
            node = slot.links.right;
        }
        result
    }
}

impl SimulationRigidBodyDynamicUpdatePrepareError {
    fn into_pool_error(self) -> SimulationRigidBodyPoolError {
        match self {
            Self::Pool(source) => source,
            Self::AllocationFailed { active_count } => {
                SimulationRigidBodyPoolError::DynamicUpdatePlanAllocationFailed { active_count }
            }
            Self::PhaseIdentityExhausted => {
                SimulationRigidBodyPoolError::DynamicUpdatePhaseIdentityExhausted
            }
        }
    }
}

impl<'pool> SimulationRigidBodyDynamicUpdatePhase<'pool> {
    /// Whether native `PopulateJobTree` would schedule compiler, solver, and
    /// `DynamicUpdate` jobs for this already-preflighted Active ring.
    #[must_use]
    pub(crate) fn has_active_bodies(&self) -> bool {
        !self.active.is_empty()
    }

    #[must_use]
    pub(crate) const fn body_pool_authority(&self) -> SimulationRigidBodyPoolAuthority {
        self.pool.authority
    }

    #[must_use]
    pub(crate) const fn frame_identity(&self) -> SimulationRigidBodyFrameIdentity {
        self.frame_identity
    }

    #[must_use]
    pub(crate) const fn phase_identity(&self) -> SimulationRigidBodyDynamicUpdatePhaseIdentity {
        self.phase_identity
    }

    /// Returns the timestep frozen when this `DynamicUpdate` phase was prepared.
    #[must_use]
    pub(crate) const fn timestep(&self) -> f32 {
        self.configuration.timestep
    }

    /// Revalidates one Contact role without resampling its numeric body state.
    ///
    /// Prepared Contact rows already own their AddNetworkContacts-time numeric
    /// copies. Later staging/table/solver boundaries use this occurrence-only
    /// check while the exclusive phase prevents topology changes.
    pub(crate) fn try_validate_contact_observation(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.pool.validate_observation(observation)?;
        Ok(())
    }

    /// Reads one body state through the phase's occurrence-qualified pool.
    ///
    /// Joint and Drive admission invoke this for body B before body A. No
    /// numeric slot data or RF row is exposed by this narrow proxy.
    pub(crate) fn try_raw_state_for_constraint(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<u32, SimulationRigidBodyPoolError> {
        let index = self.pool.validate_observation(observation)?;
        Ok(self.pool.slots[index].state)
    }

    /// Copies the exact body fields consumed by network Contact production.
    ///
    /// The caller invokes this at its host `AddNetworkContacts` preparation
    /// point. The exclusive phase prevents topology or numeric mutation during
    /// the whole preparation; later table/compiler/solver stages must retain
    /// these copied values and revalidate identity rather than resample them.
    pub(crate) fn try_network_contact_snapshot(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<SimulationRigidBodyNetworkContactSnapshot, SimulationRigidBodyPoolError> {
        self.try_contact_body_snapshot(observation)
    }

    /// Shared semantic body copy for independently recovered Contact producers.
    /// Ground samples motion during bounds/padding calculation and rereads the
    /// final Contact body rows only after reservation.
    pub(crate) fn try_contact_body_snapshot(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<SimulationRigidBodyNetworkContactSnapshot, SimulationRigidBodyPoolError> {
        let index = self.pool.validate_observation(observation)?;
        let slot = &self.pool.slots[index];
        let view = self.pool.mode_body_view(observation);
        Ok(SimulationRigidBodyNetworkContactSnapshot {
            observation,
            center_of_mass: view.map_or(slot.center_of_mass, |view| view.center_of_mass()),
            velocity: view.map_or(slot.velocity, |view| view.velocity),
            omega: view.map_or(slot.omega, |view| view.omega),
            right: view.map_or(slot.right, |view| view.axis(0)),
            up: view.map_or(slot.up, |view| view.axis(1)),
            at: view.map_or(slot.at, |view| view.axis(2)),
            inverse_inertia_full: view.map_or(slot.inverse_inertia_full, |view| view.inverse_inertia_full),
            inverse_mass: view.map_or(slot.inverse_mass, |view| view.inverse_mass),
            inverse_inertia_split: view.map_or(slot.inverse_inertia_split, |view| view.inverse_inertia_split),
            state: slot.state,
            force: slot.force,
            kinematic_value: slot.kinematic_value,
            torque: slot.torque,
            cooldown: slot.cooldown,
        })
    }

    /// Occurrence-qualified topology for the ground job's swept-bounds words.
    pub(crate) fn try_ground_motion_topology(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<SimulationRigidBodyPackedWordTopologySnapshot, SimulationRigidBodyPoolError> {
        self.pool.try_packed_word_topology_snapshot(observation)
    }

    pub(crate) fn ground_image_matches_pool(
        &self,
        image: &crate::simulation_xenon_address_image::SimulationPackedBodyWordImageDomain,
    ) -> bool {
        image.is_same_pool(self.pool)
    }

    /// Copies the exact semantic body values consumed by Joint Build.
    ///
    /// The exclusive phase borrow keeps the validated occurrence and all
    /// copied values stable through preparation and later solver ownership.
    pub(crate) fn try_joint_build_snapshot(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<SimulationRigidBodyJointBuildSnapshot, SimulationRigidBodyPoolError> {
        let index = self.pool.validate_observation(observation)?;
        let slot = &self.pool.slots[index];
        let view = self.pool.mode_body_view(observation);
        Ok(SimulationRigidBodyJointBuildSnapshot {
            observation,
            state: slot.state,
            quaternion: view.map_or(slot.quaternion, |view| view.quaternion),
            center_of_mass: view.map_or(slot.center_of_mass, |view| view.center_of_mass()),
            velocity: view.map_or(slot.velocity, |view| view.velocity),
            omega: view.map_or(slot.omega, |view| view.omega),
            right: view.map_or(slot.right, |view| view.axis(0)),
            up: view.map_or(slot.up, |view| view.axis(1)),
            at: view.map_or(slot.at, |view| view.axis(2)),
            force: slot.force,
            torque: slot.torque,
            inverse_inertia_full: view.map_or(slot.inverse_inertia_full, |view| view.inverse_inertia_full),
            inverse_mass: view.map_or(slot.inverse_mass, |view| view.inverse_mass),
            inverse_inertia_split: view.map_or(slot.inverse_inertia_split, |view| view.inverse_inertia_split),
        })
    }

    /// Copies the exact semantic body values consumed by Drive Build.
    ///
    /// The exclusive phase borrow keeps the validated occurrence and all
    /// copied values stable from Build preparation through later RF solving.
    pub(crate) fn try_drive_build_snapshot(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<SimulationRigidBodyDriveBuildSnapshot, SimulationRigidBodyPoolError> {
        let index = self.pool.validate_observation(observation)?;
        let slot = &self.pool.slots[index];
        let view = self.pool.mode_body_view(observation);
        Ok(SimulationRigidBodyDriveBuildSnapshot {
            observation,
            state: slot.state,
            quaternion: view.map_or(slot.quaternion, |view| view.quaternion),
            center_of_mass: view.map_or(slot.center_of_mass, |view| view.center_of_mass()),
            velocity: view.map_or(slot.velocity, |view| view.velocity),
            omega: view.map_or(slot.omega, |view| view.omega),
            right: view.map_or(slot.right, |view| view.axis(0)),
            up: view.map_or(slot.up, |view| view.axis(1)),
            at: view.map_or(slot.at, |view| view.axis(2)),
            force: slot.force,
            torque: slot.torque,
            inverse_inertia_full: view.map_or(slot.inverse_inertia_full, |view| view.inverse_inertia_full),
            inverse_mass: view.map_or(slot.inverse_mass, |view| view.inverse_mass),
            inverse_inertia_split: view.map_or(slot.inverse_inertia_split, |view| view.inverse_inertia_split),
        })
    }

    /// Temporarily lends the guarded pool to one fully prevalidated RF sweep.
    ///
    /// This cannot mutate body topology, inertia, or frame configuration. The
    /// transaction's borrow prevents simultaneous use or consumption of the
    /// `DynamicUpdate` phase until all ordered RF stores finish.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for the owning Contact/Joint/Drive phase"
        )
    )]
    pub(crate) fn try_begin_solver_delta_transaction<I>(
        &mut self,
        pairs: I,
    ) -> Result<
        SimulationRigidBodySolverDeltaTransaction<'_>,
        SimulationRigidBodySolverDeltaTransactionError,
    >
    where
        I: IntoIterator<
            Item = (
                SimulationRigidBodyObservation,
                SimulationRigidBodyObservation,
            ),
        >,
        I::IntoIter: ExactSizeIterator,
    {
        self.pool.try_begin_solver_delta_transaction(pairs)
    }

    /// Applies an arbitrary ordered subsequence through the guarded RF rows.
    ///
    /// Allocation, complete pool integrity, every item selector, and every A/B
    /// occurrence are checked before the first callback or store. Repeated item
    /// selectors and repeated/equal body occurrences remain in the supplied
    /// order. Each iteration then copies live A/B rows, briefly borrows only its
    /// selected item, and stores exactly A0, B0, A2, B2.
    ///
    /// A returned error has no item or RF effect. A callback panic is not
    /// rolled back: an earlier prefix may be committed and the current item may
    /// be changed before its four RF stores.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "reserved for checked Joint/Drive island sweeps")
    )]
    pub(crate) fn try_apply_indexed_solver_delta_sequence<T, I>(
        &mut self,
        items: &mut [T],
        sequence: I,
        apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodySolverDeltaStores,
    ) -> Result<(), SimulationRigidBodyIndexedSolverDeltaError>
    where
        I: IntoIterator<
            Item = (
                usize,
                SimulationRigidBodyObservation,
                SimulationRigidBodyObservation,
            ),
        >,
        I::IntoIter: ExactSizeIterator,
    {
        self.try_apply_indexed_solver_delta_sequence_with_reservation(
            items,
            sequence.into_iter(),
            apply,
            |plan, entry_count| plan.try_reserve_exact(entry_count).map_err(|_| ()),
            |_, _, _| {},
        )
    }

    /// Resolves one complete ordered RF subsequence without applying effects.
    ///
    /// The returned plan can be replayed across the exact native Horus pass
    /// count without reallocating or resampling occurrence identities.
    pub(crate) fn try_prepare_indexed_solver_delta_plan<'items, T, I>(
        &self,
        items: &'items mut [T],
        sequence: I,
    ) -> Result<
        SimulationRigidBodyIndexedSolverDeltaPlan<'items, T>,
        SimulationRigidBodyIndexedSolverDeltaError,
    >
    where
        I: IntoIterator<
            Item = (
                usize,
                SimulationRigidBodyObservation,
                SimulationRigidBodyObservation,
            ),
        >,
        I::IntoIter: ExactSizeIterator,
    {
        self.try_prepare_indexed_solver_delta_plan_with_reservation(
            items,
            sequence.into_iter(),
            |plan, entry_count| plan.try_reserve_exact(entry_count).map_err(|_| ()),
        )
    }

    /// Resolves one caller-counted flattened RF sequence without effects.
    ///
    /// The explicit count lets a family flatten several island lists through a
    /// general iterator while reserving the complete plan once. Short or
    /// trailing sequences are rejected before publication. The returned plan
    /// retains the exact mutable record-slice borrow, preventing its private
    /// body indices from being paired with another record destination.
    pub(crate) fn try_prepare_indexed_solver_delta_plan_counted<'items, T, I>(
        &self,
        items: &'items mut [T],
        entry_count: usize,
        sequence: I,
    ) -> Result<
        SimulationRigidBodyIndexedSolverDeltaPlan<'items, T>,
        SimulationRigidBodyIndexedSolverDeltaError,
    >
    where
        I: IntoIterator<
            Item = (
                usize,
                SimulationRigidBodyObservation,
                SimulationRigidBodyObservation,
            ),
        >,
    {
        self.try_prepare_indexed_solver_delta_plan_counted_with_reservation(
            items,
            entry_count,
            sequence.into_iter(),
            |plan, count| plan.try_reserve_exact(count).map_err(|_| ()),
        )
    }

    /// Joins three already-prevalidated RF plans under one exclusive phase.
    pub(crate) fn begin_horus_island_rf_transaction<
        'phase,
        'contact,
        'joint,
        'drive,
        Contact,
        Joint,
        Drive,
    >(
        &'phase mut self,
        contact: SimulationRigidBodyIndexedSolverDeltaPlan<'contact, Contact>,
        joint: SimulationRigidBodyIndexedSolverDeltaPlan<'joint, Joint>,
        drive: SimulationRigidBodyIndexedSolverDeltaPlan<'drive, Drive>,
    ) -> SimulationRigidBodyHorusIslandRfTransaction<
        'phase,
        'pool,
        'contact,
        'joint,
        'drive,
        Contact,
        Joint,
        Drive,
    > {
        assert!(
            contact.phase == self.phase_identity
                && joint.phase == self.phase_identity
                && drive.phase == self.phase_identity,
            "all Horus RF plans must belong to the exact retained phase"
        );
        SimulationRigidBodyHorusIslandRfTransaction {
            phase: self,
            contact,
            joint,
            drive,
        }
    }

    fn try_prepare_indexed_solver_delta_plan_with_reservation<'items, T, I>(
        &self,
        items: &'items mut [T],
        sequence: I,
        reserve: impl FnOnce(&mut Vec<(usize, usize, usize)>, usize) -> Result<(), ()>,
    ) -> Result<
        SimulationRigidBodyIndexedSolverDeltaPlan<'items, T>,
        SimulationRigidBodyIndexedSolverDeltaError,
    >
    where
        I: ExactSizeIterator<
            Item = (
                usize,
                SimulationRigidBodyObservation,
                SimulationRigidBodyObservation,
            ),
        >,
    {
        let entry_count = sequence.len();
        self.try_prepare_indexed_solver_delta_plan_counted_with_reservation(
            items,
            entry_count,
            sequence,
            reserve,
        )
    }

    fn try_prepare_indexed_solver_delta_plan_counted_with_reservation<'items, T, I>(
        &self,
        items: &'items mut [T],
        entry_count: usize,
        mut sequence: I,
        reserve: impl FnOnce(&mut Vec<(usize, usize, usize)>, usize) -> Result<(), ()>,
    ) -> Result<
        SimulationRigidBodyIndexedSolverDeltaPlan<'items, T>,
        SimulationRigidBodyIndexedSolverDeltaError,
    >
    where
        I: Iterator<
            Item = (
                usize,
                SimulationRigidBodyObservation,
                SimulationRigidBodyObservation,
            ),
        >,
    {
        let item_count = items.len();
        let mut entries = Vec::new();
        reserve(&mut entries, entry_count).map_err(|()| {
            SimulationRigidBodyIndexedSolverDeltaError::AllocationFailed { entry_count }
        })?;
        if entries.capacity() < entry_count {
            return Err(
                SimulationRigidBodyIndexedSolverDeltaError::AllocationFailed { entry_count },
            );
        }

        self.pool
            .validate_integrity()
            .map_err(SimulationRigidBodyIndexedSolverDeltaError::Pool)?;
        for ordinal in 0..entry_count {
            let Some((item_index, body_a, body_b)) = sequence.next() else {
                return Err(
                    SimulationRigidBodyIndexedSolverDeltaError::SequenceEndedEarly {
                        expected: entry_count,
                        actual: ordinal,
                    },
                );
            };
            if item_index >= item_count {
                return Err(SimulationRigidBodyIndexedSolverDeltaError::ItemOutOfRange {
                    ordinal,
                    item_index,
                    item_count,
                });
            }
            let index_a = self.pool.validate_observation(body_a).map_err(|source| {
                SimulationRigidBodyIndexedSolverDeltaError::Body {
                    ordinal,
                    endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyA,
                    source,
                }
            })?;
            let index_b = self.pool.validate_observation(body_b).map_err(|source| {
                SimulationRigidBodyIndexedSolverDeltaError::Body {
                    ordinal,
                    endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyB,
                    source,
                }
            })?;
            entries.push((item_index, index_a, index_b));
        }
        if sequence.next().is_some() {
            return Err(
                SimulationRigidBodyIndexedSolverDeltaError::SequenceHasTrailingEntry {
                    expected: entry_count,
                },
            );
        }

        Ok(SimulationRigidBodyIndexedSolverDeltaPlan {
            phase: self.phase_identity,
            items,
            entries,
        })
    }

    /// Applies one prevalidated Joint/Drive RF plan without a fallible step.
    pub(crate) fn apply_indexed_solver_delta_plan<T>(
        &mut self,
        plan: &mut SimulationRigidBodyIndexedSolverDeltaPlan<'_, T>,
        apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodySolverDeltaStores,
    ) {
        let entry_count = plan.entries.len();
        self.apply_indexed_solver_delta_plan_range(plan, 0..entry_count, apply);
    }

    /// Applies one prevalidated Joint/Drive subrange without a fallible step.
    ///
    /// The caller retains the only plan containing the exact mutable record
    /// slice. Range and phase assertions occur before the first callback or RF
    /// store; frame orchestration supplies ranges frozen during preflight.
    pub(crate) fn apply_indexed_solver_delta_plan_range<T>(
        &mut self,
        plan: &mut SimulationRigidBodyIndexedSolverDeltaPlan<'_, T>,
        range: std::ops::Range<usize>,
        apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodySolverDeltaStores,
    ) {
        self.apply_indexed_solver_delta_plan_range_with_observer(plan, range, apply, |_, _, _| {});
    }

    fn apply_indexed_solver_delta_plan_with_observer<T>(
        &mut self,
        plan: &mut SimulationRigidBodyIndexedSolverDeltaPlan<'_, T>,
        apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodySolverDeltaStores,
        observe_store: impl FnMut(usize, SimulationRigidBodySolverDeltaStoreKind, [f32; 4]),
    ) {
        let entry_count = plan.entries.len();
        self.apply_indexed_solver_delta_plan_range_with_observer(
            plan,
            0..entry_count,
            apply,
            observe_store,
        );
    }

    fn apply_indexed_solver_delta_plan_range_with_observer<T>(
        &mut self,
        plan: &mut SimulationRigidBodyIndexedSolverDeltaPlan<'_, T>,
        range: std::ops::Range<usize>,
        mut apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodySolverDeltaStores,
        mut observe_store: impl FnMut(usize, SimulationRigidBodySolverDeltaStoreKind, [f32; 4]),
    ) {
        assert!(
            plan.phase == self.phase_identity,
            "prepared RF plan must remain paired with its exact phase"
        );
        assert!(
            range.start <= range.end && range.end <= plan.entries.len(),
            "prepared RF plan range must remain within its exact entry sequence"
        );
        for (ordinal, &(item_index, index_a, index_b)) in plan.entries[range].iter().enumerate() {
            let rows = SimulationRigidBodySolverDeltaPairRows {
                body_a: self.pool.solver_delta_rows[index_a].vectors,
                body_b: self.pool.solver_delta_rows[index_b].vectors,
            };
            let stores = apply(&mut plan.items[item_index], rows);
            self.pool.solver_delta_rows[index_a].vectors[0] = stores.body_a_0;
            observe_store(
                ordinal,
                SimulationRigidBodySolverDeltaStoreKind::BodyA0,
                stores.body_a_0,
            );
            self.pool.solver_delta_rows[index_b].vectors[0] = stores.body_b_0;
            observe_store(
                ordinal,
                SimulationRigidBodySolverDeltaStoreKind::BodyB0,
                stores.body_b_0,
            );
            self.pool.solver_delta_rows[index_a].vectors[2] = stores.body_a_2;
            observe_store(
                ordinal,
                SimulationRigidBodySolverDeltaStoreKind::BodyA2,
                stores.body_a_2,
            );
            self.pool.solver_delta_rows[index_b].vectors[2] = stores.body_b_2;
            observe_store(
                ordinal,
                SimulationRigidBodySolverDeltaStoreKind::BodyB2,
                stores.body_b_2,
            );
        }
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "test-injected island RF reservation boundary")
    )]
    fn try_apply_indexed_solver_delta_sequence_with_reservation<T, I>(
        &mut self,
        items: &mut [T],
        sequence: I,
        mut apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodySolverDeltaStores,
        reserve: impl FnOnce(&mut Vec<(usize, usize, usize)>, usize) -> Result<(), ()>,
        mut observe_store: impl FnMut(usize, SimulationRigidBodySolverDeltaStoreKind, [f32; 4]),
    ) -> Result<(), SimulationRigidBodyIndexedSolverDeltaError>
    where
        I: ExactSizeIterator<
            Item = (
                usize,
                SimulationRigidBodyObservation,
                SimulationRigidBodyObservation,
            ),
        >,
    {
        let mut plan =
            self.try_prepare_indexed_solver_delta_plan_with_reservation(items, sequence, reserve)?;
        self.apply_indexed_solver_delta_plan_with_observer(
            &mut plan,
            &mut apply,
            &mut observe_store,
        );
        Ok(())
    }

    /// Applies an arbitrary Contact subsequence through all guarded RF rows.
    ///
    /// Allocation, complete pool integrity, every item selector, and every A/B
    /// occurrence are checked before the first callback or store. Repeated item
    /// selectors and repeated/equal body occurrences remain in the supplied
    /// order. Each iteration copies both complete live RF blocks before its
    /// callback, then stores exactly A3, B3, A1, B1, A2, B2, A0, B0.
    ///
    /// A returned error has no item or RF effect. A callback panic is not
    /// rolled back: an earlier prefix may be committed and the current item may
    /// be changed before its eight RF stores.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "standalone checked Contact sweep test seam")
    )]
    pub(crate) fn try_apply_indexed_contact_solver_delta_sequence<T, I>(
        &mut self,
        items: &mut [T],
        sequence: I,
        apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodyContactSolverDeltaStores,
    ) -> Result<(), SimulationRigidBodyIndexedSolverDeltaError>
    where
        I: IntoIterator<
            Item = (
                usize,
                SimulationRigidBodyObservation,
                SimulationRigidBodyObservation,
            ),
        >,
        I::IntoIter: ExactSizeIterator,
    {
        self.try_apply_indexed_contact_solver_delta_sequence_with_reservation(
            items,
            sequence.into_iter(),
            apply,
            |plan, entry_count| plan.try_reserve_exact(entry_count).map_err(|_| ()),
            |_, _, _| {},
        )
    }

    /// Applies one prevalidated Contact RF plan without a fallible step.
    pub(crate) fn apply_indexed_contact_solver_delta_plan<T>(
        &mut self,
        plan: &mut SimulationRigidBodyIndexedSolverDeltaPlan<'_, T>,
        apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodyContactSolverDeltaStores,
    ) {
        let entry_count = plan.entries.len();
        self.apply_indexed_contact_solver_delta_plan_range(plan, 0..entry_count, apply);
    }

    /// Applies one prevalidated Contact subrange without a fallible step.
    pub(crate) fn apply_indexed_contact_solver_delta_plan_range<T>(
        &mut self,
        plan: &mut SimulationRigidBodyIndexedSolverDeltaPlan<'_, T>,
        range: std::ops::Range<usize>,
        apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodyContactSolverDeltaStores,
    ) {
        self.apply_indexed_contact_solver_delta_plan_range_with_observer(
            plan,
            range,
            apply,
            |_, _, _| {},
        );
    }

    fn apply_indexed_contact_solver_delta_plan_range_with_observer<T>(
        &mut self,
        plan: &mut SimulationRigidBodyIndexedSolverDeltaPlan<'_, T>,
        range: std::ops::Range<usize>,
        mut apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodyContactSolverDeltaStores,
        mut observe_store: impl FnMut(usize, SimulationRigidBodySolverDeltaStoreKind, [f32; 4]),
    ) {
        assert!(
            plan.phase == self.phase_identity,
            "prepared Contact RF plan must remain paired with its exact phase"
        );
        assert!(
            range.start <= range.end && range.end <= plan.entries.len(),
            "prepared Contact RF plan range must remain within its exact entry sequence"
        );
        for (ordinal, &(item_index, index_a, index_b)) in plan.entries[range].iter().enumerate() {
            let rows = SimulationRigidBodySolverDeltaPairRows {
                body_a: self.pool.solver_delta_rows[index_a].vectors,
                body_b: self.pool.solver_delta_rows[index_b].vectors,
            };
            let stores = apply(&mut plan.items[item_index], rows);
            for (index, row, kind, value) in [
                (
                    index_a,
                    3,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA3,
                    stores.body_a_3,
                ),
                (
                    index_b,
                    3,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB3,
                    stores.body_b_3,
                ),
                (
                    index_a,
                    1,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA1,
                    stores.body_a_1,
                ),
                (
                    index_b,
                    1,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB1,
                    stores.body_b_1,
                ),
                (
                    index_a,
                    2,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA2,
                    stores.body_a_2,
                ),
                (
                    index_b,
                    2,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB2,
                    stores.body_b_2,
                ),
                (
                    index_a,
                    0,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA0,
                    stores.body_a_0,
                ),
                (
                    index_b,
                    0,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB0,
                    stores.body_b_0,
                ),
            ] {
                self.pool.solver_delta_rows[index].vectors[row] = value;
                observe_store(ordinal, kind, value);
            }
        }
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "test-injected Contact RF reservation boundary")
    )]
    fn try_apply_indexed_contact_solver_delta_sequence_with_reservation<T, I>(
        &mut self,
        items: &mut [T],
        sequence: I,
        mut apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodyContactSolverDeltaStores,
        reserve: impl FnOnce(&mut Vec<(usize, usize, usize)>, usize) -> Result<(), ()>,
        mut observe_store: impl FnMut(usize, SimulationRigidBodySolverDeltaStoreKind, [f32; 4]),
    ) -> Result<(), SimulationRigidBodyIndexedSolverDeltaError>
    where
        I: ExactSizeIterator<
            Item = (
                usize,
                SimulationRigidBodyObservation,
                SimulationRigidBodyObservation,
            ),
        >,
    {
        let entry_count = sequence.len();
        let mut plan = Vec::new();
        reserve(&mut plan, entry_count).map_err(|()| {
            SimulationRigidBodyIndexedSolverDeltaError::AllocationFailed { entry_count }
        })?;
        if plan.capacity() < entry_count {
            return Err(
                SimulationRigidBodyIndexedSolverDeltaError::AllocationFailed { entry_count },
            );
        }

        self.pool
            .validate_integrity()
            .map_err(SimulationRigidBodyIndexedSolverDeltaError::Pool)?;
        for (ordinal, (item_index, body_a, body_b)) in sequence.enumerate() {
            if item_index >= items.len() {
                return Err(SimulationRigidBodyIndexedSolverDeltaError::ItemOutOfRange {
                    ordinal,
                    item_index,
                    item_count: items.len(),
                });
            }
            let index_a = self.pool.validate_observation(body_a).map_err(|source| {
                SimulationRigidBodyIndexedSolverDeltaError::Body {
                    ordinal,
                    endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyA,
                    source,
                }
            })?;
            let index_b = self.pool.validate_observation(body_b).map_err(|source| {
                SimulationRigidBodyIndexedSolverDeltaError::Body {
                    ordinal,
                    endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyB,
                    source,
                }
            })?;
            plan.push((item_index, index_a, index_b));
        }

        for (ordinal, (item_index, index_a, index_b)) in plan.into_iter().enumerate() {
            let rows = SimulationRigidBodySolverDeltaPairRows {
                body_a: self.pool.solver_delta_rows[index_a].vectors,
                body_b: self.pool.solver_delta_rows[index_b].vectors,
            };
            let stores = apply(&mut items[item_index], rows);
            for (index, row, kind, value) in [
                (
                    index_a,
                    3,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA3,
                    stores.body_a_3,
                ),
                (
                    index_b,
                    3,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB3,
                    stores.body_b_3,
                ),
                (
                    index_a,
                    1,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA1,
                    stores.body_a_1,
                ),
                (
                    index_b,
                    1,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB1,
                    stores.body_b_1,
                ),
                (
                    index_a,
                    2,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA2,
                    stores.body_a_2,
                ),
                (
                    index_b,
                    2,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB2,
                    stores.body_b_2,
                ),
                (
                    index_a,
                    0,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA0,
                    stores.body_a_0,
                ),
                (
                    index_b,
                    0,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB0,
                    stores.body_b_0,
                ),
            ] {
                self.pool.solver_delta_rows[index].vectors[row] = value;
                observe_store(ordinal, kind, value);
            }
        }
        Ok(())
    }

    /// Runs the already-preflighted Active-body integration with no returned
    /// error. Dropping this owner instead performs no body or RF work, though
    /// its already-issued private phase identity remains consumed.
    /// Retains a shared pool borrow for source post-integration readers; it
    /// cannot be used to mutate bodies or reschedule a frame.
    pub(crate) fn integrate_ordinary_finite(self) -> &'pool SimulationRigidBodyPool {
        let Self {
            pool,
            active,
            configuration,
            frame_identity: _,
            phase_identity: _,
        } = self;
        let (slots, solver_delta_rows) = (&mut pool.slots, &mut pool.solver_delta_rows);
        for SimulationRigidBodyDynamicUpdatePlanEntry { index, inertia } in active {
            let slot = &mut slots[index];
            if pool.externally_driven.iter().any(|body| {
                body.id.0 as usize == index && Some(body.occurrence) == slot.occurrence
            }) {
                continue;
            }
            simulation_dynamic_update(
                SimulationDynamicUpdateBody {
                    quaternion: &mut slot.quaternion,
                    center_of_mass: &mut slot.center_of_mass,
                    velocity: &mut slot.velocity,
                    omega: &mut slot.omega,
                    right: &mut slot.right,
                    up: &mut slot.up,
                    at: &mut slot.at,
                    inverse_inertia_full: &mut slot.inverse_inertia_full,
                    inverse_mass: slot.inverse_mass,
                    inverse_inertia_split: &mut slot.inverse_inertia_split,
                    force: &mut slot.force,
                    kinematic_value: &mut slot.kinematic_value,
                    torque: &mut slot.torque,
                    cooldown: &mut slot.cooldown,
                    solver_delta: &mut solver_delta_rows[index].vectors,
                },
                inertia,
                configuration,
            );
        }
        pool
    }
}

impl<Contact, Joint, Drive>
    SimulationRigidBodyHorusIslandRfTransaction<'_, '_, '_, '_, '_, Contact, Joint, Drive>
{
    /// Executes the snapshotted unsigned pass count in exact C -> J -> D order.
    ///
    /// All allocation, selectors, and occurrences were resolved before this
    /// transaction took ownership of the phase and item slices. Consequently
    /// execution has no recoverable failure boundary; a panic retains its exact
    /// earlier/current native-shaped mutation prefix.
    pub(crate) fn execute_iterations(
        mut self,
        max_iterations: u32,
        mut solve_contact: impl FnMut(
            u32,
            &mut Contact,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodyContactSolverDeltaStores,
        mut solve_joint: impl FnMut(
            u32,
            &mut Joint,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodySolverDeltaStores,
        mut solve_drive: impl FnMut(
            u32,
            &mut Drive,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodySolverDeltaStores,
    ) {
        for pass in 0..max_iterations {
            self.phase
                .apply_indexed_contact_solver_delta_plan(&mut self.contact, |item, rows| {
                    solve_contact(pass, item, rows)
                });
            self.phase
                .apply_indexed_solver_delta_plan(&mut self.joint, |item, rows| {
                    solve_joint(pass, item, rows)
                });
            self.phase
                .apply_indexed_solver_delta_plan(&mut self.drive, |item, rows| {
                    solve_drive(pass, item, rows)
                });
        }
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained for the test-only direct Drive evidence harness"
    )
)]
impl SimulationRigidBodySolverDeltaTransaction<'_> {
    /// Applies one infallible value transform per prevalidated pair.
    ///
    /// Each pair is copied from live RF state immediately before its callback.
    /// Stores then occur exactly A0, B0, A2, B2. Consequently a self-pair has
    /// native B-wins behavior and a later shared-body pair observes all earlier
    /// stores. The structural output type makes vectors 1 and 3 unwritable.
    ///
    /// A returned error always precedes callbacks and mutation. A panic inside
    /// the private callback may leave prior pairs committed and the current
    /// item mutated without any current-pair RF store. It is not rolled back.
    pub(crate) fn try_apply_all<T>(
        self,
        items: &mut [T],
        apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodySolverDeltaStores,
    ) -> Result<(), SimulationRigidBodySolverDeltaTransactionError> {
        self.try_apply_all_with_observer(items, apply, |_, _, _| {})
    }

    fn try_apply_all_with_observer<T>(
        self,
        items: &mut [T],
        mut apply: impl FnMut(
            &mut T,
            SimulationRigidBodySolverDeltaPairRows,
        ) -> SimulationRigidBodySolverDeltaStores,
        mut observe_store: impl FnMut(usize, SimulationRigidBodySolverDeltaStoreKind, [f32; 4]),
    ) -> Result<(), SimulationRigidBodySolverDeltaTransactionError> {
        let pair_count = self.pairs.len();
        if items.len() != pair_count {
            return Err(
                SimulationRigidBodySolverDeltaTransactionError::ItemCountMismatch {
                    pair_count,
                    item_count: items.len(),
                },
            );
        }

        let Self { pool, pairs } = self;
        for (ordinal, ((index_a, index_b), item)) in
            pairs.into_iter().zip(items.iter_mut()).enumerate()
        {
            let rows = SimulationRigidBodySolverDeltaPairRows {
                body_a: pool.solver_delta_rows[index_a].vectors,
                body_b: pool.solver_delta_rows[index_b].vectors,
            };
            let stores = apply(item, rows);
            pool.solver_delta_rows[index_a].vectors[0] = stores.body_a_0;
            observe_store(
                ordinal,
                SimulationRigidBodySolverDeltaStoreKind::BodyA0,
                stores.body_a_0,
            );
            pool.solver_delta_rows[index_b].vectors[0] = stores.body_b_0;
            observe_store(
                ordinal,
                SimulationRigidBodySolverDeltaStoreKind::BodyB0,
                stores.body_b_0,
            );
            pool.solver_delta_rows[index_a].vectors[2] = stores.body_a_2;
            observe_store(
                ordinal,
                SimulationRigidBodySolverDeltaStoreKind::BodyA2,
                stores.body_a_2,
            );
            pool.solver_delta_rows[index_b].vectors[2] = stores.body_b_2;
            observe_store(
                ordinal,
                SimulationRigidBodySolverDeltaStoreKind::BodyB2,
                stores.body_b_2,
            );
        }
        Ok(())
    }
}

fn fresh_pool_authority(
    source: &AtomicU64,
) -> Result<SimulationRigidBodyPoolAuthority, SimulationRigidBodyPoolError> {
    source
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map(SimulationRigidBodyPoolAuthority)
        .map_err(|_| SimulationRigidBodyPoolError::AuthorityExhausted)
}

#[cfg(test)]
#[path = "../../assembly/simulation_part_body_transform_tests.rs"]
mod part_transform_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    fn inertia(seed: f32) -> Arc<SimulationInertia> {
        Arc::new(SimulationInertia {
            inverse_tensor: [seed, seed + 1.0, seed + 2.0],
            inverse_mass: seed + 3.0,
            ..SimulationInertia::native_default()
        })
    }

    fn bits3(value: [f32; 3]) -> [u32; 3] {
        value.map(f32::to_bits)
    }

    fn bits4(value: [f32; 4]) -> [u32; 4] {
        value.map(f32::to_bits)
    }

    fn after_one_phase_identity_issue(
        mut snapshot: SimulationRigidBodyPoolSnapshot,
    ) -> SimulationRigidBodyPoolSnapshot {
        snapshot.next_dynamic_update_phase_identity = snapshot
            .next_dynamic_update_phase_identity
            .and_then(|identity| identity.checked_add(1));
        snapshot
    }

    #[test]
    fn dynamic_update_phase_identity_is_nonreused_and_drop_burns_the_issued_value() {
        let mut pool = SimulationRigidBodyPool::try_new(0).expect("pool");
        let first = pool
            .try_prepare_dynamic_update_phase()
            .expect("first phase");
        let first_identity = first.phase_identity();
        drop(first);
        let second = pool
            .try_prepare_dynamic_update_phase()
            .expect("second phase");
        let second_identity = second.phase_identity();
        assert!(first_identity != second_identity);
        drop(second);
        assert_eq!(pool.snapshot().next_dynamic_update_phase_identity, Some(3));
    }

    #[test]
    fn network_contact_snapshot_copies_its_complete_distinct_read_set() {
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let body = add(&mut pool, None, 1);
        let slot = &mut pool.slots[body.id().index() as usize];
        slot.center_of_mass = [1.0, 2.0, 3.0];
        slot.velocity = [4.0, 5.0, 6.0];
        slot.omega = [7.0, 8.0, 9.0];
        slot.right = [10.0, 11.0, 12.0];
        slot.up = [13.0, 14.0, 15.0];
        slot.at = [16.0, 17.0, 18.0];
        slot.inverse_inertia_full = [19.0, 20.0, 21.0];
        slot.inverse_mass = 22.0;
        slot.inverse_inertia_split = [23.0, 24.0, 25.0];
        slot.force = [26.0, 27.0, 28.0];
        slot.kinematic_value = 29.0;
        slot.torque = [30.0, 31.0, 32.0];
        slot.cooldown = 0x4142_4344;
        pool.begin_dynamic_update_frame(1.0 / 60.0);

        let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
        let snapshot = phase
            .try_network_contact_snapshot(body.observation())
            .expect("network Contact snapshot");
        assert_eq!(snapshot.observation, body.observation());
        assert_eq!(bits3(snapshot.center_of_mass), bits3([1.0, 2.0, 3.0]));
        assert_eq!(bits3(snapshot.velocity), bits3([4.0, 5.0, 6.0]));
        assert_eq!(bits3(snapshot.omega), bits3([7.0, 8.0, 9.0]));
        assert_eq!(bits3(snapshot.right), bits3([10.0, 11.0, 12.0]));
        assert_eq!(bits3(snapshot.up), bits3([13.0, 14.0, 15.0]));
        assert_eq!(bits3(snapshot.at), bits3([16.0, 17.0, 18.0]));
        assert_eq!(
            bits3(snapshot.inverse_inertia_full),
            bits3([19.0, 20.0, 21.0])
        );
        assert_eq!(snapshot.inverse_mass.to_bits(), 22.0_f32.to_bits());
        assert_eq!(
            bits3(snapshot.inverse_inertia_split),
            bits3([23.0, 24.0, 25.0])
        );
        assert_eq!(snapshot.state, 1);
        assert_eq!(bits3(snapshot.force), bits3([26.0, 27.0, 28.0]));
        assert_eq!(snapshot.kinematic_value.to_bits(), 29.0_f32.to_bits());
        assert_eq!(bits3(snapshot.torque), bits3([30.0, 31.0, 32.0]));
        assert_eq!(snapshot.cooldown, 0x4142_4344);
    }

    #[test]
    fn dynamic_update_phase_identity_exhaustion_is_last_preflight_and_pre_effect() {
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let body = add(&mut pool, None, 4);
        pool.next_dynamic_update_phase_identity = None;
        let Err(error) = pool.try_prepare_dynamic_update_phase() else {
            panic!("missing inertia must precede phase-identity exhaustion");
        };
        assert_eq!(
            error,
            SimulationRigidBodyDynamicUpdatePrepareError::Pool(
                SimulationRigidBodyPoolError::ActiveBodyMissingInertia {
                    slot: body.id().index(),
                },
            )
        );

        pool.try_remove_rigid_body(body).expect("remove body");
        let before = pool.snapshot();
        let Err(error) = pool.try_prepare_dynamic_update_phase_with_reservation(|_, _| Err(()))
        else {
            panic!("plan allocation must precede phase-identity exhaustion");
        };
        assert_eq!(
            error,
            SimulationRigidBodyDynamicUpdatePrepareError::AllocationFailed { active_count: 0 }
        );
        assert_eq!(pool.snapshot(), before);
        let Err(error) = pool.try_prepare_dynamic_update_phase() else {
            panic!("exhausted identity must reject the phase");
        };
        assert_eq!(
            error,
            SimulationRigidBodyDynamicUpdatePrepareError::PhaseIdentityExhausted
        );
        assert_eq!(
            error.into_pool_error(),
            SimulationRigidBodyPoolError::DynamicUpdatePhaseIdentityExhausted
        );
        assert_eq!(pool.snapshot(), before);
    }

    fn pool_with_state(
        capacity: usize,
        gravity: [f32; 3],
        cooldown: u32,
    ) -> SimulationRigidBodyPool {
        let mut pool = SimulationRigidBodyPool::try_new(capacity).expect("pool");
        pool.set_gravity(gravity);
        pool.set_cooldown(cooldown);
        pool
    }

    fn add(
        pool: &mut SimulationRigidBodyPool,
        inertia: Option<&Arc<SimulationInertia>>,
        state: u32,
    ) -> SimulationRigidBodyHandle {
        pool.try_add_rigid_body(identity(), inertia, state)
            .expect("valid pool")
            .expect("free slot")
    }

    #[test]
    fn solver_delta_transaction_reserves_and_validates_all_pairs_before_effects() {
        let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
        let first = add(&mut pool, None, 4);
        let second = add(&mut pool, None, 4);
        pool.try_with_solver_delta_row_mut(first.observation(), |row| {
            row[0] = [1.0, 2.0, 3.0, 4.0];
        })
        .expect("seed first");
        let before = pool
            .try_with_solver_delta_row(first.observation(), |row| *row)
            .expect("first row");

        let reservation = pool.try_begin_solver_delta_transaction_with_reservation(
            [(first.observation(), second.observation())].into_iter(),
            |plan, pair_count| {
                assert!(plan.is_empty());
                assert_eq!(pair_count, 1);
                Err(())
            },
        );
        assert!(matches!(
            reservation,
            Err(SimulationRigidBodySolverDeltaTransactionError::AllocationFailed { pair_count: 1 })
        ));
        assert_eq!(
            pool.try_with_solver_delta_row(first.observation(), |row| *row)
                .expect("unchanged first row"),
            before
        );

        pool.try_remove_rigid_body(second).expect("remove second");
        let stale = pool.try_begin_solver_delta_transaction([
            (first.observation(), first.observation()),
            (first.observation(), second.observation()),
        ]);
        assert!(matches!(
            stale,
            Err(SimulationRigidBodySolverDeltaTransactionError::Body {
                ordinal: 1,
                endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyB,
                source: SimulationRigidBodyPoolError::BodyIsFree { slot: 1 },
            })
        ));
        assert_eq!(
            pool.try_with_solver_delta_row(first.observation(), |row| *row)
                .expect("still unchanged first row"),
            before
        );
    }

    #[test]
    fn solver_delta_transaction_self_pair_stages_then_commits_a0_b0_a2_b2() {
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let body = add(&mut pool, None, 4);
        let original = [
            [1.0, 2.0, 3.0, 4.0],
            [11.0, 12.0, 13.0, 14.0],
            [21.0, 22.0, 23.0, 24.0],
            [31.0, 32.0, 33.0, 34.0],
        ];
        pool.try_with_solver_delta_row_mut(body.observation(), |row| *row = original)
            .expect("seed self row");
        let transaction = pool
            .try_begin_solver_delta_transaction([(body.observation(), body.observation())])
            .expect("self pair is valid");
        let mut item = [()];
        let mut stores = Vec::new();
        transaction
            .try_apply_all_with_observer(
                &mut item,
                |(), rows| {
                    let (body_a, body_b) = rows.into_rows();
                    assert_eq!(body_a, original);
                    assert_eq!(body_b, original);
                    SimulationRigidBodySolverDeltaStores::new(
                        [101.0; 4], [102.0; 4], [103.0; 4], [104.0; 4],
                    )
                },
                |ordinal, kind, value| stores.push((ordinal, kind, value)),
            )
            .expect("apply self pair");
        assert_eq!(
            stores,
            [
                (
                    0,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA0,
                    [101.0; 4],
                ),
                (
                    0,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB0,
                    [102.0; 4],
                ),
                (
                    0,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA2,
                    [103.0; 4],
                ),
                (
                    0,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB2,
                    [104.0; 4],
                ),
            ]
        );
        assert_eq!(
            pool.try_with_solver_delta_row(body.observation(), |row| *row)
                .expect("self result"),
            [[102.0; 4], original[1], [104.0; 4], original[3]]
        );
    }

    #[test]
    fn solver_delta_transaction_reads_shared_rows_after_prior_record_stores() {
        let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
        let body_p = add(&mut pool, None, 4);
        let body_x = add(&mut pool, None, 4);
        let body_q = add(&mut pool, None, 4);
        for (body, seed) in [(body_p, 1.0), (body_x, 10.0), (body_q, 20.0)] {
            pool.try_with_solver_delta_row_mut(body.observation(), |row| {
                row[0] = [seed; 4];
                row[2] = [seed + 1.0; 4];
            })
            .expect("seed row");
        }
        let transaction = pool
            .try_begin_solver_delta_transaction([
                (body_p.observation(), body_x.observation()),
                (body_x.observation(), body_q.observation()),
            ])
            .expect("shared plan");
        let mut ordinals = [0_u32, 1];
        transaction
            .try_apply_all(&mut ordinals, |ordinal, rows| {
                let (body_a, body_b) = rows.into_rows();
                if *ordinal == 0 {
                    assert_eq!(bits4(body_a[0]), [1.0_f32.to_bits(); 4]);
                    assert_eq!(bits4(body_b[0]), [10.0_f32.to_bits(); 4]);
                    SimulationRigidBodySolverDeltaStores::new(
                        [2.0; 4], [30.0; 4], [3.0; 4], [31.0; 4],
                    )
                } else {
                    assert_eq!(bits4(body_a[0]), [30.0_f32.to_bits(); 4]);
                    assert_eq!(bits4(body_a[2]), [31.0_f32.to_bits(); 4]);
                    assert_eq!(bits4(body_b[0]), [20.0_f32.to_bits(); 4]);
                    SimulationRigidBodySolverDeltaStores::new(
                        [40.0; 4], [50.0; 4], [41.0; 4], [51.0; 4],
                    )
                }
            })
            .expect("shared apply");
        assert_eq!(
            pool.try_with_solver_delta_row(body_x.observation(), |row| (row[0], row[2]))
                .expect("shared X"),
            ([40.0; 4], [41.0; 4])
        );
    }

    #[test]
    fn solver_delta_transaction_item_count_mismatch_precedes_callback_and_stores() {
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let body = add(&mut pool, None, 4);
        let before = pool
            .try_with_solver_delta_row(body.observation(), |row| *row)
            .expect("before");
        let transaction = pool
            .try_begin_solver_delta_transaction([(body.observation(), body.observation())])
            .expect("plan");
        let mut called = false;
        let result = transaction.try_apply_all(&mut [], |(): &mut (), _| {
            called = true;
            SimulationRigidBodySolverDeltaStores::new([1.0; 4], [2.0; 4], [3.0; 4], [4.0; 4])
        });
        assert_eq!(
            result,
            Err(
                SimulationRigidBodySolverDeltaTransactionError::ItemCountMismatch {
                    pair_count: 1,
                    item_count: 0,
                }
            )
        );
        assert!(!called);
        assert_eq!(
            pool.try_with_solver_delta_row(body.observation(), |row| *row)
                .expect("unchanged"),
            before
        );
    }

    #[test]
    #[allow(
        clippy::similar_names,
        reason = "A/B and record 0/1 roles are the behavior under test"
    )]
    fn solver_delta_transaction_panic_keeps_prefix_and_current_item_without_current_rf() {
        let mut pool = SimulationRigidBodyPool::try_new(4).expect("pool");
        let body_a0 = add(&mut pool, None, 4);
        let body_b0 = add(&mut pool, None, 4);
        let body_a1 = add(&mut pool, None, 4);
        let body_b1 = add(&mut pool, None, 4);
        for (body, seed) in [
            (body_a0, 1.0),
            (body_b0, 11.0),
            (body_a1, 21.0),
            (body_b1, 31.0),
        ] {
            pool.try_with_solver_delta_row_mut(body.observation(), |row| {
                row[0] = [seed; 4];
                row[2] = [seed + 1.0; 4];
            })
            .expect("seed panic row");
        }
        let before_a1 = pool
            .try_with_solver_delta_row(body_a1.observation(), |row| *row)
            .expect("A1 before");
        let before_b1 = pool
            .try_with_solver_delta_row(body_b1.observation(), |row| *row)
            .expect("B1 before");
        let transaction = pool
            .try_begin_solver_delta_transaction([
                (body_a0.observation(), body_b0.observation()),
                (body_a1.observation(), body_b1.observation()),
            ])
            .expect("panic plan");
        let mut items = [0_u32; 2];
        let mut calls = 0;
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = transaction.try_apply_all(&mut items, |item, _| {
                *item += 1;
                let ordinal = calls;
                calls += 1;
                assert_ne!(ordinal, 1, "injected second callback panic");
                SimulationRigidBodySolverDeltaStores::new(
                    [101.0; 4], [102.0; 4], [103.0; 4], [104.0; 4],
                )
            });
        }));
        assert!(unwind.is_err());
        assert_eq!(items, [1, 1]);
        assert_eq!(calls, 2);
        assert_eq!(
            pool.try_with_solver_delta_row(body_a0.observation(), |row| {
                (bits4(row[0]), bits4(row[2]))
            })
            .expect("A0 prefix"),
            ([101.0_f32.to_bits(); 4], [103.0_f32.to_bits(); 4])
        );
        assert_eq!(
            pool.try_with_solver_delta_row(body_b0.observation(), |row| {
                (bits4(row[0]), bits4(row[2]))
            })
            .expect("B0 prefix"),
            ([102.0_f32.to_bits(); 4], [104.0_f32.to_bits(); 4])
        );
        assert_eq!(
            pool.try_with_solver_delta_row(body_a1.observation(), |row| *row)
                .expect("A1 no RF"),
            before_a1
        );
        assert_eq!(
            pool.try_with_solver_delta_row(body_b1.observation(), |row| *row)
                .expect("B1 no RF"),
            before_b1
        );
    }

    #[test]
    fn initialization_builds_ascending_free_ring_and_empty_pool_is_inert() {
        let pool = SimulationRigidBodyPool::try_new(3).expect("pool");
        let snapshot = pool.snapshot();
        assert_eq!(snapshot.gravity.map(f32::to_bits), [0, 0xc416_0000, 0]);
        assert_eq!(snapshot.cooldown, 30);
        assert_eq!(snapshot.counts.free, 3);
        assert_eq!(
            snapshot.free_order,
            [0, 1, 2].map(SimulationRigidBodyId).to_vec()
        );
        assert!(snapshot.static_order.is_empty());
        assert!(snapshot.frozen_order.is_empty());
        assert!(snapshot.active_order.is_empty());
        assert!(snapshot.slots.iter().all(|slot| {
            slot.state == 0
                && slot.membership == SimulationRigidBodyList::Free
                && slot.stasis == snapshot.authority
        }));

        let mut empty = SimulationRigidBodyPool::try_new(0).expect("empty pool");
        let before = empty.snapshot();
        assert_eq!(
            empty
                .try_add_rigid_body(identity(), None, 4)
                .expect("empty is ordinary"),
            None
        );
        assert_eq!(empty.snapshot(), before);
    }

    #[test]
    fn add_reservation_is_drop_inert_and_freezes_exact_state_dispatch() {
        let mut pool = pool_with_state(1, [0.0; 3], 7);
        for (raw_state, expected_static) in [(1, true), (2, false), (0, false), (u32::MAX, false)] {
            let before = pool.snapshot();
            let reservation = pool
                .try_prepare_add_rigid_body(raw_state)
                .expect("valid Add preflight")
                .expect("one Free body");
            assert_eq!(reservation.is_static(), expected_static);
            drop(reservation);
            assert_eq!(pool.snapshot(), before);
        }

        let handle = pool
            .try_prepare_add_rigid_body(2)
            .expect("valid Frozen preflight")
            .expect("one Free body")
            .commit(identity(), None);
        assert_eq!(pool.try_raw_state(handle.observation()), Ok(2));
        let full_before = pool.snapshot();
        assert!(
            pool.try_prepare_add_rigid_body(1)
                .expect("empty Free list is not an error")
                .is_none()
        );
        assert_eq!(pool.snapshot(), full_before);

        pool.next_occurrence = None;
        pool.validated_topology.take(); // Test-only corruption bypasses ring mutators.
        pool.slots[0].links.right = SimulationRigidBodyNode::Body(handle.id());
        let poisoned_full = pool.snapshot();
        assert!(
            matches!(pool.try_prepare_add_rigid_body(u32::MAX), Ok(None)),
            "empty Free must precede occurrence and ring validation"
        );
        assert_eq!(pool.snapshot(), poisoned_full);
    }

    #[test]
    fn solver_delta_rows_have_native_stride_alignment_and_one_time_zero_bits() {
        assert_eq!(
            std::mem::size_of::<SimulationRigidBodySolverDeltaRow>(),
            0x40
        );
        assert_eq!(
            std::mem::align_of::<SimulationRigidBodySolverDeltaRow>(),
            0x40
        );

        let pool = SimulationRigidBodyPool::try_new(3).expect("pool");
        let base = pool.solver_delta_rows.as_ptr().addr();
        assert_eq!(base % 0x40, 0);
        assert_eq!(pool.solver_delta_rows[1..].as_ptr().addr() - base, 0x40);
        assert!(pool.solver_delta_rows.iter().all(|row| {
            row.vectors
                .iter()
                .flatten()
                .all(|value| value.to_bits() == 0)
        }));
    }

    #[test]
    fn active_solver_delta_visit_is_guarded_ordered_and_excludes_other_lists() {
        let mut empty = SimulationRigidBodyPool::try_new(0).expect("empty pool");
        let mut empty_calls = 0;
        empty
            .try_for_each_active_solver_delta_row(|_, _| empty_calls += 1)
            .expect("valid empty pool");
        assert_eq!(empty_calls, 0);

        let mut pool = pool_with_state(5, [0.0; 3], 0);
        let active_a = add(&mut pool, None, 4);
        let static_body = add(&mut pool, None, 1);
        let active_b = add(&mut pool, None, 0);
        let frozen_body = add(&mut pool, None, 2);
        let active_c = add(&mut pool, None, u32::MAX);
        let mut visited = Vec::new();
        pool.try_for_each_active_solver_delta_row(|observation, row| {
            let id = observation.identity().id();
            visited.push(id);
            row[0][0] = match id.index() {
                0 => 0.0,
                2 => 2.0,
                4 => 4.0,
                unexpected => panic!("unexpected Active slot {unexpected}"),
            };
        })
        .expect("valid retained rings");

        assert_eq!(visited, [active_a.id(), active_b.id(), active_c.id()]);
        assert!(!visited.contains(&static_body.id()));
        assert!(!visited.contains(&frozen_body.id()));
        assert_eq!(
            pool.solver_delta_rows[active_a.id().0 as usize].vectors[0][0].to_bits(),
            0.0_f32.to_bits()
        );
        assert_eq!(
            pool.solver_delta_rows[active_b.id().0 as usize].vectors[0][0].to_bits(),
            2.0_f32.to_bits()
        );
        assert_eq!(
            pool.solver_delta_rows[active_c.id().0 as usize].vectors[0][0].to_bits(),
            4.0_f32.to_bits()
        );
    }

    #[test]
    fn active_solver_delta_visit_validates_every_ring_before_callback_effects() {
        let mut pool = pool_with_state(3, [0.0; 3], 0);
        let active = add(&mut pool, None, 4);
        let static_body = add(&mut pool, None, 1);
        pool.validated_topology.take(); // Test-only corruption bypasses ring mutators.
        pool.slots[static_body.id().0 as usize].links.right =
            SimulationRigidBodyNode::Body(static_body.id());
        let rows_before = pool.solver_delta_rows.clone();
        let mut calls = 0;
        assert_eq!(
            pool.try_for_each_active_solver_delta_row(|_, row| {
                calls += 1;
                row[0][0] = 99.0;
            }),
            Err(SimulationRigidBodyPoolError::CorruptStructure)
        );
        assert_eq!(calls, 0);
        assert_eq!(pool.solver_delta_rows, rows_before);
        assert_eq!(active.id(), SimulationRigidBodyId(0));
    }

    #[test]
    fn solver_delta_access_rejects_foreign_free_and_stale_observations_atomically() {
        let mut first = pool_with_state(1, [0.0; 3], 0);
        let mut second = pool_with_state(1, [0.0; 3], 0);
        let old = add(&mut first, None, 4);
        let observation = old.observation();
        let mut calls = 0;
        assert_eq!(
            second.try_with_solver_delta_row_mut(observation, |_| calls += 1),
            Err(SimulationRigidBodyPoolError::ForeignHandle)
        );
        assert_eq!(calls, 0);

        first.try_remove_rigid_body(old).expect("remove");
        assert_eq!(
            first.try_with_solver_delta_row_mut(observation, |_| calls += 1),
            Err(SimulationRigidBodyPoolError::BodyIsFree { slot: 0 })
        );
        assert_eq!(calls, 0);

        let fresh = add(&mut first, None, 4);
        assert_eq!(fresh.id(), old.id());
        assert_eq!(
            first.try_with_solver_delta_row_mut(observation, |_| calls += 1),
            Err(SimulationRigidBodyPoolError::StaleHandle { slot: 0 })
        );
        assert_eq!(calls, 0);
        first
            .try_with_solver_delta_row_mut(fresh.observation(), |row| {
                calls += 1;
                row[3][3] = f32::from_bits(0x8000_0000);
            })
            .expect("current occurrence");
        assert_eq!(calls, 1);
    }

    #[test]
    fn solver_delta_row_bits_survive_remove_and_fifo_reuse_without_reset_clear() {
        let mut pool = pool_with_state(2, [0.0; 3], 0);
        let old = add(&mut pool, None, 4);
        let sentinel_bits = [
            [0x8000_0000, 0x3f80_0001, 0x7fc0_1234, 0xff80_0000],
            [0x0000_0001, 0x0080_0000, 0x7f7f_ffff, 0xff7f_ffff],
            [0x4120_0000, 0xc120_0000, 0x3eaa_aaab, 0xbeaa_aaab],
            [0xdead_beef, 0x1234_5678, 0x0000_0000, 0x8000_0000],
        ];
        pool.try_with_solver_delta_row_mut(old.observation(), |row| {
            *row = sentinel_bits.map(|vector| vector.map(f32::from_bits));
        })
        .expect("live row");

        pool.try_remove_rigid_body(old).expect("remove old");
        let intervening = add(&mut pool, None, 1);
        assert_ne!(intervening.id(), old.id(), "Free reuse is FIFO");
        let reused = add(&mut pool, None, 4);
        assert_eq!(reused.id(), old.id());
        let observed_bits = pool
            .try_with_solver_delta_row(reused.observation(), |row| {
                row.map(|vector| vector.map(f32::to_bits))
            })
            .expect("reused occurrence");
        assert_eq!(observed_bits, sentinel_bits);
    }

    #[test]
    fn raw_state_dispatch_and_current_dynamics_table_are_exact() {
        for raw in [0, 4, 7, 8, u32::MAX] {
            let source = inertia(1.0);
            let mut pool = pool_with_state(1, [1.0, f32::from_bits(0x8000_0000), 3.0], 17);
            let handle = add(&mut pool, Some(&source), raw);
            let slot = &pool.snapshot().slots[0];
            assert_eq!(slot.state, 4, "raw {raw}");
            assert_eq!(slot.membership, SimulationRigidBodyList::Active);
            assert_eq!(slot.cooldown, 0);
            assert_eq!(slot.kinematic_value.to_bits(), 0x7f7f_ffff);
            assert_eq!(
                slot.force.map(f32::to_bits),
                [0x3f80_0000, 0x8000_0000, 0x4040_0000]
            );
            assert!(Arc::ptr_eq(
                pool.try_inertia(handle).expect("handle").expect("inertia"),
                &source
            ));
        }

        let source = inertia(2.0);
        let mut frozen = pool_with_state(1, [4.0, 5.0, 6.0], u32::MAX);
        add(&mut frozen, Some(&source), 2);
        let slot = &frozen.snapshot().slots[0];
        assert_eq!(slot.state, 2);
        assert_eq!(slot.cooldown, u32::MAX);
        assert_eq!(bits3(slot.force), bits3([4.0, 5.0, 6.0]));

        let mut static_pool = pool_with_state(1, [4.0, 5.0, 6.0], 9);
        let static_handle = add(&mut static_pool, Some(&source), 1);
        let slot = &static_pool.snapshot().slots[0];
        assert_eq!(slot.state, 1);
        assert_eq!(slot.cooldown, 9);
        assert_eq!(slot.kinematic_value.to_bits(), 0);
        assert_eq!(slot.force.map(f32::to_bits), [0; 3]);
        assert_eq!(
            static_pool.try_inertia(static_handle).expect("handle"),
            None
        );
    }

    #[test]
    fn call_time_gravity_and_cooldown_are_sampled_only_by_the_selected_path() {
        let source = inertia(1.0);
        let mut pool = pool_with_state(3, [1.0, 2.0, 3.0], 4);
        add(&mut pool, Some(&source), 4);
        pool.set_gravity([-5.0, -6.0, -7.0]);
        pool.set_cooldown(0x8000_0008);
        add(&mut pool, Some(&source), 2);
        add(&mut pool, Some(&source), 1);
        let slots = &pool.snapshot().slots;
        assert_eq!(bits3(slots[0].force), bits3([1.0, 2.0, 3.0]));
        assert_eq!(slots[0].cooldown, 0);
        assert_eq!(bits3(slots[1].force), bits3([-5.0, -6.0, -7.0]));
        assert_eq!(slots[1].cooldown, 0x8000_0008);
        assert_eq!(bits3(slots[2].force), [0; 3]);
        assert_eq!(slots[2].cooldown, 0x8000_0008);
    }

    #[test]
    fn add_uses_free_head_and_destination_tail_in_stable_order() {
        let mut pool = pool_with_state(4, [0.0; 3], 0);
        let a = add(&mut pool, None, 4);
        let b = add(&mut pool, None, 1);
        let c = add(&mut pool, None, 4);
        let d = add(&mut pool, None, 2);
        let snapshot = pool.snapshot();
        assert_eq!(a.id().index(), 0);
        assert_eq!(b.id().index(), 1);
        assert_eq!(c.id().index(), 2);
        assert_eq!(d.id().index(), 3);
        assert_eq!(snapshot.active_order, [a.id(), c.id()]);
        assert_eq!(snapshot.static_order, [b.id()]);
        assert_eq!(snapshot.frozen_order, [d.id()]);
        assert_eq!(
            snapshot.counts,
            SimulationRigidBodyCounts {
                free: 0,
                static_bodies: 1,
                frozen: 1,
                active: 2
            }
        );
    }

    #[test]
    fn pose_quaternion_and_inertia_use_the_committed_operation_graphs() {
        let matrix = [
            0.36, 0.48, -0.8, 11.0, -0.8, 0.60, 0.0, 22.0, 0.48, 0.64, 0.60, 33.0, 5.0, -6.0, -0.0,
            44.0,
        ];
        let source = Arc::new(SimulationInertia {
            inverse_tensor: [
                f32::from_bits(0x3f33_3333),
                f32::from_bits(0x3fa6_6666),
                f32::from_bits(0x4006_6666),
            ],
            inverse_mass: f32::from_bits(0x8000_0000),
            ..SimulationInertia::native_default()
        });
        let mut pool = pool_with_state(1, [0.0; 3], 0);
        let handle = pool
            .try_add_rigid_body(matrix, Some(&source), 4)
            .expect("pool")
            .expect("body");
        let ortho = orthonormalize_collision_affine(matrix);
        let expected_quaternion = simulation_quaternion_from_orthonormalized_matrix_host(ortho);
        let expected_inertia = simulation_rotated_inverse_inertia_host(
            ortho,
            source.inverse_tensor,
            source.inverse_mass,
        );
        let slot = &pool.snapshot().slots[0];
        assert_eq!(
            slot.quaternion.map(f32::to_bits),
            expected_quaternion.map(f32::to_bits)
        );
        assert_eq!(
            slot.right.map(f32::to_bits),
            ortho[0..3]
                .try_into()
                .map(|v: [f32; 3]| v.map(f32::to_bits))
                .expect("three")
        );
        assert_eq!(
            slot.up.map(f32::to_bits),
            ortho[4..7]
                .try_into()
                .map(|v: [f32; 3]| v.map(f32::to_bits))
                .expect("three")
        );
        assert_eq!(
            slot.at.map(f32::to_bits),
            ortho[8..11]
                .try_into()
                .map(|v: [f32; 3]| v.map(f32::to_bits))
                .expect("three")
        );
        assert_eq!(
            slot.center_of_mass.map(f32::to_bits),
            ortho[12..15]
                .try_into()
                .map(|v: [f32; 3]| v.map(f32::to_bits))
                .expect("three")
        );
        assert_eq!(
            slot.inverse_inertia_full.map(f32::to_bits),
            expected_inertia.full.map(f32::to_bits)
        );
        assert_eq!(
            slot.inverse_inertia_split.map(f32::to_bits),
            expected_inertia.split.map(f32::to_bits)
        );
        assert_eq!(slot.inverse_mass.to_bits(), 0x8000_0000);
        assert_eq!(
            pool.try_rigid_body_pose(handle).expect("pose")[15].to_bits(),
            1.0_f32.to_bits()
        );
    }

    #[test]
    fn null_inertia_leaves_reset_positive_zero_derived_values() {
        let mut pool = pool_with_state(1, [0.0; 3], 0);
        let handle = add(&mut pool, None, 4);
        let slot = &pool.snapshot().slots[0];
        assert_eq!(slot.inverse_inertia_full.map(f32::to_bits), [0; 3]);
        assert_eq!(slot.inverse_inertia_split.map(f32::to_bits), [0; 3]);
        assert_eq!(slot.inverse_mass.to_bits(), 0);
        assert_eq!(pool.try_inertia(handle).expect("handle"), None);
    }

    #[test]
    fn remove_preserves_every_non_link_non_state_field_and_retains_arc() {
        let source = inertia(7.0);
        let mut pool = pool_with_state(1, [1.0, 2.0, 3.0], 4);
        let handle = add(&mut pool, Some(&source), 2);
        let before = pool.snapshot().slots[0].clone();
        assert_eq!(Arc::strong_count(&source), 2);
        pool.try_remove_rigid_body(handle).expect("remove");
        let after = pool.snapshot().slots[0].clone();
        assert_eq!(after.state, 0);
        assert_eq!(after.membership, SimulationRigidBodyList::Free);
        assert_eq!(after.occurrence, before.occurrence);
        assert_eq!(bits4(after.quaternion), bits4(before.quaternion));
        assert_eq!(bits3(after.center_of_mass), bits3(before.center_of_mass));
        assert_eq!(bits3(after.velocity), bits3(before.velocity));
        assert_eq!(bits3(after.omega), bits3(before.omega));
        assert_eq!(bits3(after.right), bits3(before.right));
        assert_eq!(after.stasis, before.stasis);
        assert_eq!(bits3(after.up), bits3(before.up));
        assert_eq!(after.inertia, before.inertia);
        assert_eq!(bits3(after.at), bits3(before.at));
        assert_eq!(after.tag, before.tag);
        assert_eq!(
            bits3(after.inverse_inertia_full),
            bits3(before.inverse_inertia_full)
        );
        assert_eq!(after.inverse_mass.to_bits(), before.inverse_mass.to_bits());
        assert_eq!(
            bits3(after.inverse_inertia_split),
            bits3(before.inverse_inertia_split)
        );
        assert_eq!(bits3(after.force), bits3(before.force));
        assert_eq!(
            after.kinematic_value.to_bits(),
            before.kinematic_value.to_bits()
        );
        assert_eq!(bits3(after.torque), bits3(before.torque));
        assert_eq!(after.cooldown, before.cooldown);
        assert_eq!(Arc::strong_count(&source), 2);
    }

    #[test]
    fn removal_appends_free_tail_and_reuse_is_fifo() {
        let mut pool = pool_with_state(4, [0.0; 3], 0);
        let a = add(&mut pool, None, 4);
        let b = add(&mut pool, None, 4);
        pool.try_remove_rigid_body(a).expect("remove a");
        assert_eq!(
            pool.snapshot().free_order,
            [SimulationRigidBodyId(2), SimulationRigidBodyId(3), a.id()]
        );
        let c = add(&mut pool, None, 4);
        assert_eq!(c.id(), SimulationRigidBodyId(2));
        pool.try_remove_rigid_body(b).expect("remove b");
        assert_eq!(
            pool.snapshot().free_order,
            [SimulationRigidBodyId(3), a.id(), b.id()]
        );
    }

    #[test]
    fn foreign_double_and_stale_handles_are_atomic() {
        let mut first = pool_with_state(1, [0.0; 3], 0);
        let mut second = pool_with_state(1, [0.0; 3], 0);
        let old = add(&mut first, None, 4);
        let second_before = second.snapshot();
        assert_eq!(
            second.try_remove_rigid_body(old),
            Err(SimulationRigidBodyPoolError::ForeignHandle)
        );
        assert_eq!(second.snapshot(), second_before);

        first.try_remove_rigid_body(old).expect("first remove");
        let removed = first.snapshot();
        assert_eq!(
            first.try_remove_rigid_body(old),
            Err(SimulationRigidBodyPoolError::BodyIsFree { slot: 0 })
        );
        assert_eq!(first.snapshot(), removed);
        let fresh = add(&mut first, None, 4);
        assert_eq!(fresh.id(), old.id());
        let reused = first.snapshot();
        assert_eq!(
            first.try_remove_rigid_body(old),
            Err(SimulationRigidBodyPoolError::StaleHandle { slot: 0 })
        );
        assert_eq!(first.snapshot(), reused);
        first.try_remove_rigid_body(fresh).expect("fresh remove");
    }

    #[test]
    fn contact_list_velocity_reads_current_raw_xyz_in_every_allocated_ring() {
        for state in [1, 2, 4, 9, 10, 12] {
            let mut pool = pool_with_state(1, [0.0; 3], 2);
            let body = add(&mut pool, None, state & 7);
            if state & 8 != 0 {
                pool.try_register_contact_spy(body).unwrap();
            }
            for words in [
                [0x8000_0000, 0x7fc0_1234, 0xff80_0000],
                [0x3f80_0000, 0xc040_0000, 0x0000_0001],
            ] {
                pool.slots[0].velocity = words.map(f32::from_bits);
                assert_eq!(
                    pool.try_contact_spy_linear_velocity(body.observation())
                        .unwrap()
                        .map(f32::to_bits),
                    words
                );
                assert_eq!(pool.slots[0].velocity.map(f32::to_bits), words);
                assert_eq!(pool.try_raw_state(body.observation()), Ok(state));
            }
        }
    }

    #[test]
    fn contact_list_velocity_refuses_foreign_free_and_reused_occurrences() {
        let mut pool = pool_with_state(1, [0.0; 3], 0);
        let foreign = pool_with_state(1, [0.0; 3], 0);
        let old = add(&mut pool, None, 4);
        assert_eq!(
            foreign.try_contact_spy_linear_velocity(old.observation()),
            Err(SimulationRigidBodyPoolError::ForeignHandle)
        );
        pool.try_remove_rigid_body(old).unwrap();
        assert_eq!(
            pool.try_contact_spy_linear_velocity(old.observation()),
            Err(SimulationRigidBodyPoolError::BodyIsFree { slot: 0 })
        );
        let fresh = add(&mut pool, None, 4);
        pool.slots[0].velocity = [1.0, 2.0, 3.0];
        assert_eq!(
            pool.try_contact_spy_linear_velocity(old.observation()),
            Err(SimulationRigidBodyPoolError::StaleHandle { slot: 0 })
        );
        assert_eq!(
            pool.try_contact_spy_linear_velocity(fresh.observation()),
            Ok([1.0, 2.0, 3.0])
        );
    }

    #[test]
    fn contact_response_reads_unmodified_cached_words_and_exact_state_in_each_ring() {
        for state in [1, 2, 4, 9, 10, 12] {
            let mut pool = pool_with_state(1, [0.0; 3], 2);
            let body = add(&mut pool, None, state & 7);
            if state & 8 != 0 {
                pool.try_register_contact_spy(body).unwrap();
            }
            for word in [
                0,
                0x8000_0000,
                0x7fc0_1234,
                0xff80_0000,
                0x0000_0001,
                0x3e80_0000,
            ] {
                // Controlled private cached-field discriminator, not a claim
                // that the source mass constructor produces every word here.
                pool.slots[0].inverse_mass = f32::from_bits(word);
                let mut before = pool.snapshot();
                before.slots[0].inverse_mass = 0.0;
                assert_eq!(pool.try_raw_state(body.observation()), Ok(state));
                assert_eq!(
                    pool.try_contact_response_cached_inverse_mass(body.observation())
                        .unwrap()
                        .to_bits(),
                    word
                );
                let mut after = pool.snapshot();
                assert_eq!(after.slots[0].inverse_mass.to_bits(), word);
                // Compare all other snapshot fields without NaN equality.
                after.slots[0].inverse_mass = 0.0;
                assert_eq!(after, before);
            }
        }
    }

    #[test]
    fn contact_response_uses_body_cache_not_mutated_or_poisoned_inertia_owner() {
        let mut pool = pool_with_state(1, [0.0; 3], 2);
        let original = SimulationInertia {
            inverse_mass: 0.25,
            ..SimulationInertia::native_default()
        };
        let owner = SimulationMutableInertia::new(original);
        let body = pool
            .try_add_rigid_body_mutable(identity(), Some(&owner), 4)
            .unwrap()
            .unwrap();
        pool.try_register_contact_spy(body).unwrap();
        let changed = SimulationInertia {
            inverse_mass: 7.0,
            ..original
        };
        pool.try_write_mutable_inertia(body, &owner, changed)
            .unwrap();
        assert_eq!(
            owner.try_read().unwrap().inverse_mass.to_bits(),
            7.0_f32.to_bits()
        );
        assert_eq!(
            pool.try_contact_response_cached_inverse_mass(body.observation())
                .unwrap()
                .to_bits(),
            0.25_f32.to_bits()
        );
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = owner.0.lock().unwrap();
            panic!("controlled retained-owner poison");
        }));
        assert!(poisoned.is_err());
        assert!(matches!(
            owner.try_read(),
            Err(SimulationRigidBodyPoolError::InertiaLockPoisoned)
        ));
        assert_eq!(pool.try_raw_state(body.observation()), Ok(12));
        assert_eq!(
            pool.try_contact_response_cached_inverse_mass(body.observation())
                .unwrap()
                .to_bits(),
            0.25_f32.to_bits()
        );
    }

    #[test]
    fn contact_response_cache_refuses_foreign_free_reused_and_corrupt_occurrences() {
        let mut pool = pool_with_state(1, [0.0; 3], 0);
        let foreign = pool_with_state(1, [0.0; 3], 0);
        let old = add(&mut pool, None, 4);
        assert!(matches!(
            foreign.try_contact_response_cached_inverse_mass(old.observation()),
            Err(SimulationRigidBodyPoolError::ForeignHandle)
        ));
        pool.try_remove_rigid_body(old).unwrap();
        assert!(matches!(
            pool.try_contact_response_cached_inverse_mass(old.observation()),
            Err(SimulationRigidBodyPoolError::BodyIsFree { slot: 0 })
        ));
        let fresh = add(&mut pool, None, 4);
        assert_eq!(fresh.id(), old.id());
        pool.slots[0].inverse_mass = f32::from_bits(0x8000_0000);
        assert!(matches!(
            pool.try_contact_response_cached_inverse_mass(old.observation()),
            Err(SimulationRigidBodyPoolError::StaleHandle { slot: 0 })
        ));
        assert_eq!(
            pool.try_contact_response_cached_inverse_mass(fresh.observation())
                .unwrap()
                .to_bits(),
            0x8000_0000
        );
        // Deliberately malformed private topology is refused before any readout;
        // restore its count so this remains a test-only corruption.
        pool.validated_topology.take(); // Test-only corruption bypasses ring mutators.
        pool.counts.active += 1;
        assert!(matches!(
            pool.try_contact_response_cached_inverse_mass(fresh.observation()),
            Err(SimulationRigidBodyPoolError::CorruptStructure)
        ));
        assert_eq!(pool.slots[0].inverse_mass.to_bits(), 0x8000_0000);
        pool.counts.active -= 1;
        pool.try_remove_rigid_body(fresh).unwrap();
    }

    #[test]
    fn contact_spy_state_prefix_preserves_rings_occurrences_and_dynamics() {
        for state in [1, 2, 4] {
            let source = inertia(7.0);
            let mut pool = pool_with_state(2, [1.0, 2.0, 3.0], 4);
            let handle = add(&mut pool, Some(&source), state);
            pool.try_with_solver_delta_row_mut(handle.observation(), |rows| {
                rows[0] = [1.0, 2.0, 3.0, 4.0];
            })
            .unwrap();
            let before = pool.snapshot();
            let rf = pool
                .try_with_solver_delta_row(handle.observation(), |rows| *rows)
                .unwrap();
            pool.try_register_contact_spy(handle).unwrap();
            pool.try_register_contact_spy(handle).unwrap();
            let mut expected = before.clone();
            expected.slots[handle.id().index() as usize].state = state | 8;
            assert_eq!(pool.snapshot(), expected);
            assert_eq!(pool.try_raw_state(handle.observation()), Ok(state | 8));
            assert_eq!(
                pool.try_with_solver_delta_row(handle.observation(), |rows| *rows)
                    .unwrap(),
                rf
            );
            pool.try_remove_contact_spy(handle).unwrap();
            pool.try_remove_contact_spy(handle).unwrap();
            assert_eq!(pool.snapshot(), before);
            assert_eq!(
                pool.try_with_solver_delta_row(handle.observation(), |rows| *rows)
                    .unwrap(),
                rf
            );
        }
    }

    #[test]
    fn contact_spy_state_prefix_rejects_foreign_free_and_reused_handles() {
        let mut pool = pool_with_state(1, [0.0; 3], 0);
        let mut foreign = pool_with_state(1, [0.0; 3], 0);
        let old = add(&mut pool, None, 4);
        let before = foreign.snapshot();
        assert_eq!(
            foreign.try_register_contact_spy(old),
            Err(SimulationRigidBodyPoolError::ForeignHandle)
        );
        assert_eq!(
            foreign.try_remove_contact_spy(old),
            Err(SimulationRigidBodyPoolError::ForeignHandle)
        );
        assert_eq!(foreign.snapshot(), before);
        pool.try_remove_rigid_body(old).unwrap();
        let before = pool.snapshot();
        assert_eq!(
            pool.try_register_contact_spy(old),
            Err(SimulationRigidBodyPoolError::BodyIsFree { slot: 0 })
        );
        assert_eq!(
            pool.try_remove_contact_spy(old),
            Err(SimulationRigidBodyPoolError::BodyIsFree { slot: 0 })
        );
        assert_eq!(pool.snapshot(), before);
        let _fresh = add(&mut pool, None, 2);
        let before = pool.snapshot();
        assert_eq!(
            pool.try_register_contact_spy(old),
            Err(SimulationRigidBodyPoolError::StaleHandle { slot: 0 })
        );
        assert_eq!(
            pool.try_remove_contact_spy(old),
            Err(SimulationRigidBodyPoolError::StaleHandle { slot: 0 })
        );
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn observations_preserve_occurrence_for_reads_without_granting_removal() {
        let mut first = pool_with_state(1, [0.0; 3], 0);
        let second = pool_with_state(1, [0.0; 3], 0);
        let old_handle = add(&mut first, None, 4);
        let old_observation = first.try_observe(old_handle).expect("live observation");
        assert_eq!(old_observation.identity(), old_handle.identity());
        assert_eq!(first.try_raw_state(old_observation), Ok(4));
        assert_eq!(
            second.try_raw_state(old_observation),
            Err(SimulationRigidBodyPoolError::ForeignHandle)
        );

        first
            .try_remove_rigid_body(old_handle)
            .expect("remove observed body");
        assert_eq!(
            first.try_raw_state(old_observation),
            Err(SimulationRigidBodyPoolError::BodyIsFree { slot: 0 })
        );
        let fresh_handle = add(&mut first, None, 2);
        assert_eq!(
            first.try_raw_state(old_observation),
            Err(SimulationRigidBodyPoolError::StaleHandle { slot: 0 })
        );
        assert_eq!(first.try_raw_state(fresh_handle.observation()), Ok(2));
    }

    #[test]
    fn packed_word_topology_snapshot_reads_live_rings_and_actual_inertia_presence() {
        let source = inertia(17.0);
        let mut pool = pool_with_state(5, [0.0; 3], 0);
        let static_body = add(&mut pool, Some(&source), 1);
        let frozen_body = add(&mut pool, Some(&source), 2);
        let active_a = add(&mut pool, Some(&source), 4);

        let active_before = pool
            .try_packed_word_topology_snapshot(active_a.observation())
            .expect("single Active body topology");
        assert_eq!(
            active_before.right_link,
            SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Active)
        );
        assert_eq!(active_before.left_link, active_before.right_link);

        let active_b = add(&mut pool, None, 99);
        let static_topology = pool
            .try_packed_word_topology_snapshot(static_body.observation())
            .expect("Static body topology");
        let frozen_topology = pool
            .try_packed_word_topology_snapshot(frozen_body.observation())
            .expect("Frozen body topology");
        let first_active_topology = pool
            .try_packed_word_topology_snapshot(active_a.observation())
            .expect("first Active body topology");
        let second_active_topology = pool
            .try_packed_word_topology_snapshot(active_b.observation())
            .expect("second Active body topology");

        assert_eq!(static_topology.observation, static_body.observation());
        assert_eq!(
            static_topology.right_link,
            SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Static)
        );
        assert_eq!(static_topology.left_link, static_topology.right_link);
        assert!(!static_topology.has_retained_inertia);
        assert_eq!(
            frozen_topology.right_link,
            SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Frozen)
        );
        assert_eq!(frozen_topology.left_link, frozen_topology.right_link);
        assert!(frozen_topology.has_retained_inertia);
        assert_eq!(
            first_active_topology.right_link,
            SimulationRigidBodyNode::Body(active_b.id())
        );
        assert_eq!(
            first_active_topology.left_link,
            SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Active)
        );
        assert!(first_active_topology.has_retained_inertia);
        assert_eq!(
            second_active_topology.right_link,
            SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Active)
        );
        assert_eq!(
            second_active_topology.left_link,
            SimulationRigidBodyNode::Body(active_a.id())
        );
        assert!(!second_active_topology.has_retained_inertia);
    }

    #[test]
    fn packed_word_topology_snapshot_rejects_foreign_free_stale_and_corrupt_inputs() {
        let mut first = pool_with_state(1, [0.0; 3], 0);
        let second = pool_with_state(1, [0.0; 3], 0);
        let old = add(&mut first, None, 4);
        let observation = old.observation();

        assert_eq!(
            second.try_packed_word_topology_snapshot(observation),
            Err(SimulationRigidBodyPoolError::ForeignHandle)
        );
        first.try_remove_rigid_body(old).expect("remove old body");
        assert_eq!(
            first.try_packed_word_topology_snapshot(observation),
            Err(SimulationRigidBodyPoolError::BodyIsFree { slot: 0 })
        );
        let fresh = add(&mut first, None, 2);
        assert_eq!(fresh.id(), old.id());
        assert_eq!(
            first.try_packed_word_topology_snapshot(observation),
            Err(SimulationRigidBodyPoolError::StaleHandle { slot: 0 })
        );

        first.validated_topology.take(); // Test-only corruption bypasses ring mutators.
        first.slots[0].links.right = SimulationRigidBodyNode::Body(fresh.id());
        assert_eq!(
            first.try_packed_word_topology_snapshot(fresh.observation()),
            Err(SimulationRigidBodyPoolError::CorruptStructure)
        );
    }

    #[test]
    fn drive_build_snapshot_copies_exact_numeric_fields_without_retaining_owner() {
        let retained_inertia = inertia(11.0);
        let mut pool = pool_with_state(1, [0.0; 3], 0);
        let handle = add(&mut pool, Some(&retained_inertia), 4);
        let value = |bits| f32::from_bits(bits);
        {
            let slot = &mut pool.slots[handle.id().0 as usize];
            slot.quaternion = [
                value(0x8000_0000),
                value(0x3f00_0001),
                value(0xbf80_0002),
                value(0x7fc1_2345),
            ];
            slot.center_of_mass = [value(0x3f80_0010), value(0xc000_0020), value(0x4040_0030)];
            slot.velocity = [value(0x4080_0040), value(0xc0a0_0050), value(0x40c0_0060)];
            slot.omega = [value(0xc0e0_0070), value(0x4100_0080), value(0xc110_0090)];
            slot.right = [value(0x4120_00a0), value(0xc130_00b0), value(0x4140_00c0)];
            slot.up = [value(0xc150_00d0), value(0x4160_00e0), value(0xc170_00f0)];
            slot.at = [value(0x4180_0100), value(0xc188_0110), value(0x4190_0120)];
            slot.force = [value(0xc198_0130), value(0x41a0_0140), value(0xc1a8_0150)];
            slot.torque = [value(0x41b0_0160), value(0xc1b8_0170), value(0x41c0_0180)];
            slot.inverse_inertia_full =
                [value(0x3d00_0190), value(0xbd80_01a0), value(0x3e00_01b0)];
            slot.inverse_mass = value(0xbe80_01c0);
            slot.inverse_inertia_split =
                [value(0x3f00_01d0), value(0xbf40_01e0), value(0x3f80_01f0)];
        }

        let owner_count = Arc::strong_count(&retained_inertia);
        let observation = handle.observation();
        let copied = pool
            .try_drive_build_snapshot(observation)
            .expect("live occurrence snapshot");

        assert_eq!(copied.observation(), observation);
        assert_eq!(copied.state, 4);
        assert_eq!(
            bits4(copied.quaternion),
            [0x8000_0000, 0x3f00_0001, 0xbf80_0002, 0x7fc1_2345]
        );
        assert_eq!(
            bits3(copied.center_of_mass),
            [0x3f80_0010, 0xc000_0020, 0x4040_0030]
        );
        assert_eq!(
            bits3(copied.velocity),
            [0x4080_0040, 0xc0a0_0050, 0x40c0_0060]
        );
        assert_eq!(bits3(copied.omega), [0xc0e0_0070, 0x4100_0080, 0xc110_0090]);
        assert_eq!(bits3(copied.right), [0x4120_00a0, 0xc130_00b0, 0x4140_00c0]);
        assert_eq!(bits3(copied.up), [0xc150_00d0, 0x4160_00e0, 0xc170_00f0]);
        assert_eq!(bits3(copied.at), [0x4180_0100, 0xc188_0110, 0x4190_0120]);
        assert_eq!(bits3(copied.force), [0xc198_0130, 0x41a0_0140, 0xc1a8_0150]);
        assert_eq!(
            bits3(copied.torque),
            [0x41b0_0160, 0xc1b8_0170, 0x41c0_0180]
        );
        assert_eq!(
            bits3(copied.inverse_inertia_full),
            [0x3d00_0190, 0xbd80_01a0, 0x3e00_01b0]
        );
        assert_eq!(copied.inverse_mass.to_bits(), 0xbe80_01c0);
        assert_eq!(
            bits3(copied.inverse_inertia_split),
            [0x3f00_01d0, 0xbf40_01e0, 0x3f80_01f0]
        );
        assert_eq!(Arc::strong_count(&retained_inertia), owner_count);
        assert!(!std::mem::needs_drop::<SimulationRigidBodyDriveBuildSnapshot>());

        pool.slots[handle.id().0 as usize].quaternion = [0.0; 4];
        assert_eq!(
            bits4(copied.quaternion),
            [0x8000_0000, 0x3f00_0001, 0xbf80_0002, 0x7fc1_2345],
            "the snapshot is a detached numeric copy"
        );
    }

    #[test]
    fn drive_build_snapshot_rejects_foreign_free_and_reused_occurrences() {
        let mut first = pool_with_state(1, [0.0; 3], 0);
        let second = pool_with_state(1, [0.0; 3], 0);
        let old = add(&mut first, None, 4);
        let observation = old.observation();

        assert_eq!(
            second.try_drive_build_snapshot(observation),
            Err(SimulationRigidBodyPoolError::ForeignHandle)
        );
        first.try_remove_rigid_body(old).expect("remove old body");
        assert_eq!(
            first.try_drive_build_snapshot(observation),
            Err(SimulationRigidBodyPoolError::BodyIsFree { slot: 0 })
        );

        let fresh = add(&mut first, None, 2);
        assert_eq!(fresh.id(), old.id());
        assert_eq!(
            first.try_drive_build_snapshot(observation),
            Err(SimulationRigidBodyPoolError::StaleHandle { slot: 0 })
        );
        assert_eq!(
            first
                .try_drive_build_snapshot(fresh.observation())
                .expect("fresh occurrence")
                .state,
            2
        );
    }

    #[test]
    fn final_occurrence_is_issued_once_then_exhaustion_has_no_pool_effect() {
        let mut pool = pool_with_state(1, [0.0; 3], 0);
        pool.next_occurrence = NonZeroU64::new(u64::MAX);
        let handle = add(&mut pool, None, 4);
        assert_eq!(handle.occurrence.get(), u64::MAX);
        assert_eq!(pool.snapshot().next_occurrence, None);
        pool.try_remove_rigid_body(handle).expect("remove");
        let before = pool.snapshot();
        assert_eq!(
            pool.try_add_rigid_body(identity(), None, 4),
            Err(SimulationRigidBodyPoolError::OccurrenceExhausted)
        );
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn corrupt_structure_is_rejected_before_add_or_remove_effects() {
        let mut add_pool = pool_with_state(2, [0.0; 3], 0);
        add_pool.validated_topology.take(); // Test-only corruption bypasses ring mutators.
        add_pool.slots[0].links.right = SimulationRigidBodyNode::Body(SimulationRigidBodyId(0));
        let before = add_pool.snapshot();
        assert_eq!(
            add_pool.try_add_rigid_body(identity(), None, 4),
            Err(SimulationRigidBodyPoolError::CorruptStructure)
        );
        assert_eq!(add_pool.snapshot(), before);

        let mut remove_pool = pool_with_state(2, [0.0; 3], 0);
        let handle = add(&mut remove_pool, None, 4);
        remove_pool.validated_topology.take(); // Test-only corruption bypasses ring mutators.
        remove_pool.slots[1].links.left = SimulationRigidBodyNode::Body(handle.id());
        let before = remove_pool.snapshot();
        assert_eq!(
            remove_pool.try_remove_rigid_body(handle),
            Err(SimulationRigidBodyPoolError::CorruptStructure)
        );
        assert_eq!(remove_pool.snapshot(), before);
    }

    #[test]
    fn public_remove_accepts_proven_spy_combined_states_and_uses_low_bits() {
        for (raw_state, source) in [
            (9, SimulationRigidBodyList::Static),
            (10, SimulationRigidBodyList::Frozen),
            (12, SimulationRigidBodyList::Active),
        ] {
            let mut pool = pool_with_state(1, [0.0; 3], 0);
            let handle = add(&mut pool, None, source.state());
            pool.slots[0].state = raw_state;
            pool.try_remove_rigid_body(handle)
                .expect("SPY bit is valid on an allocated body");
            assert_eq!(pool.counts.get(source), 0, "state {raw_state}");
            assert_eq!(pool.counts.free, 1);
            assert_eq!(pool.slots[0].state, 0);
            assert_eq!(pool.slots[0].occurrence, Some(handle.occurrence));
        }
    }

    #[test]
    fn raw_remove_mask_and_wrapping_counts_are_preserved_in_internal_primitive() {
        for raw_state in [0, 3, 5, 6, 7, 8, u32::MAX] {
            let mut pool = pool_with_state(1, [0.0; 3], 0);
            let handle = add(&mut pool, None, 4);
            pool.slots[0].state = raw_state;
            pool.counts.active = 0;
            pool.remove_validated_slot(0);
            assert_eq!(pool.counts.active, u32::MAX, "state {raw_state}");
            assert_eq!(pool.counts.free, 1);
            assert_eq!(pool.slots[0].state, 0);
            assert_eq!(pool.slots[0].occurrence, Some(handle.occurrence));
        }
    }

    #[test]
    fn reset_preserves_id_links_stasis_inertia_and_occurrence() {
        let old = inertia(1.0);
        let new = inertia(11.0);
        let mut pool = pool_with_state(1, [0.0; 3], 0);
        let first = add(&mut pool, Some(&old), 4);
        pool.try_remove_rigid_body(first).expect("remove");
        let free = pool.snapshot().slots[0].clone();
        let second = add(&mut pool, Some(&new), 2);
        let reused = pool.snapshot().slots[0].clone();
        assert_eq!(second.id(), first.id());
        assert_ne!(reused.occurrence, free.occurrence);
        assert_eq!(reused.id, free.id);
        assert_eq!(reused.stasis, free.stasis);
        assert_eq!(reused.inertia, Some(*new));
        assert_eq!(Arc::strong_count(&old), 1);
        assert_eq!(Arc::strong_count(&new), 2);
    }

    #[test]
    fn skeleton_motion_reset_changes_only_four_xyz_fields_and_reads_current_gravity() {
        let retained = inertia(3.0);
        let mut pool = pool_with_state(2, [1.0, 2.0, 3.0], 71);
        let first = add(&mut pool, Some(&retained), 4);
        let _second = add(&mut pool, Some(&retained), 4);
        let index = first.id.0 as usize;
        let slot = &mut pool.slots[index];
        slot.velocity = [11.0, 12.0, 13.0];
        slot.omega = [21.0, 22.0, 23.0];
        slot.force = [31.0, 32.0, 33.0];
        slot.torque = [41.0, 42.0, 43.0];
        slot.kinematic_value = -9.0;
        slot.cooldown = 83;
        pool.try_with_solver_delta_row_mut(first.observation(), |row| {
            row[0] = [51.0, 52.0, 53.0, 54.0];
        })
        .unwrap();
        pool.set_gravity([7.0, -8.0, 9.0]);
        let rf = pool
            .try_with_solver_delta_row(first.observation(), |row| *row)
            .unwrap();
        let mut expected = pool.snapshot();
        expected.slots[index].velocity = [0.0; 3];
        expected.slots[index].omega = [0.0; 3];
        expected.slots[index].force = [7.0, -8.0, 9.0];
        expected.slots[index].torque = [0.0; 3];
        pool.try_reset_part_body_motion_host(first).unwrap();
        assert_eq!(pool.snapshot(), expected);
        assert_eq!(
            pool.try_with_solver_delta_row(first.observation(), |row| *row)
                .unwrap(),
            rf
        );
    }

    struct DisplacementNumeric {
        reads: usize,
        fail_on: Option<usize>,
    }
    impl crate::reckoning_numeric::ReckoningNumeric for DisplacementNumeric {
        type Error = ();
        fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
            assert_eq!(value.to_bits(), 0x3c88_8889);
            self.reads += 1;
            if self.fail_on == Some(self.reads) {
                Err(())
            } else {
                Ok(value.recip())
            }
        }
        fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            panic!("displacement has no rsqrt");
        }
        fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
            panic!("displacement has no dot");
        }
        fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
            panic!("displacement has no conversion");
        }
    }

    #[test]
    fn displacement_updates_only_force_xyz_and_cooldown_using_cached_body_mass() {
        let retained = inertia(3.0);
        let mut pool = pool_with_state(2, [101.0, -202.0, 303.0], 71);
        let first = add(&mut pool, Some(&retained), 4);
        let _second = add(&mut pool, Some(&retained), 4);
        let index = first.id.0 as usize;
        let slot = &mut pool.slots[index];
        slot.inverse_mass = 0.25; // Deliberately distinct from retained inertia.
        slot.velocity = [11.0, 12.0, 13.0];
        slot.omega = [21.0, 22.0, 23.0];
        slot.force = [31.0, 32.0, 33.0];
        slot.torque = [41.0, 42.0, 43.0];
        slot.kinematic_value = -9.0;
        slot.cooldown = 83;
        pool.try_with_solver_delta_row_mut(first.observation(), |row| {
            row[0] = [51.0, 52.0, 53.0, 54.0];
        })
        .unwrap();
        let rf = pool
            .try_with_solver_delta_row(first.observation(), |row| *row)
            .unwrap();
        let mut expected = pool.snapshot();
        let displacement = [1.0, 2.0, -3.0, f32::NAN];
        // Arithmetic has independent frozen kernel tests. This test checks the
        // actual cached input authority and entire in-place pool write set.
        expected.slots[index].force = crate::skeleton_general_force::try_accumulate_force(
            displacement,
            0.25,
            [31.0, 32.0, 33.0],
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
        expected.slots[index].cooldown = 0;
        let mut numeric = DisplacementNumeric {
            reads: 0,
            fail_on: None,
        };
        pool.try_apply_part_body_displacement(first, displacement, &mut numeric)
            .unwrap();
        assert_eq!(numeric.reads, 1);
        assert_eq!(pool.snapshot(), expected);
        assert_eq!(
            pool.try_with_solver_delta_row(first.observation(), |row| *row)
                .unwrap(),
            rf
        );
    }

    #[test]
    fn displacement_refusal_preserves_current_body_and_previously_completed_body() {
        let mut pool = pool_with_state(2, [1.0, 2.0, 3.0], 71);
        let first = add(&mut pool, None, 4);
        let second = add(&mut pool, None, 4);
        for body in [first, second] {
            let slot = &mut pool.slots[body.id.0 as usize];
            slot.inverse_mass = 0.25;
            slot.force = [4.0, 5.0, 6.0];
            slot.cooldown = 99;
        }
        let mut numeric = DisplacementNumeric {
            reads: 0,
            fail_on: Some(2),
        };
        let displacement = [1.0, 2.0, 3.0, 0.0];
        pool.try_apply_part_body_displacement(first, displacement, &mut numeric)
            .unwrap();
        let completed_prefix = pool.snapshot();
        assert!(matches!(
            pool.try_apply_part_body_displacement(second, displacement, &mut numeric),
            Err(SimulationRigidBodyDisplacementError::Numeric(()))
        ));
        assert_eq!(numeric.reads, 2);
        assert_eq!(pool.snapshot(), completed_prefix);
        let mut foreign = pool_with_state(1, [0.0; 3], 0);
        assert!(matches!(
            foreign.try_apply_part_body_displacement(first, displacement, &mut numeric),
            Err(SimulationRigidBodyDisplacementError::Body(_))
        ));
        assert_eq!(numeric.reads, 2);
    }

    #[test]
    fn reset_write_set_is_positive_zero_identity_and_preserves_packed_fields() {
        let retained = inertia(3.0);
        let authority = SimulationRigidBodyPoolAuthority(77);
        let id = SimulationRigidBodyId(6);
        let occurrence = NonZeroU64::new(9);
        let links = Links {
            right: SimulationRigidBodyNode::Body(SimulationRigidBodyId(7)),
            left: SimulationRigidBodyNode::Body(SimulationRigidBodyId(5)),
        };
        let mut slot = RigidBodySlot::initialized(id, authority, links);
        slot.occurrence = occurrence;
        slot.quaternion = [-1.0; 4];
        slot.center_of_mass = [-1.0; 3];
        slot.velocity = [-1.0; 3];
        slot.omega = [-1.0; 3];
        slot.right = [-1.0; 3];
        slot.up = [-1.0; 3];
        slot.inertia = Some(RetainedInertia::Immutable(Arc::clone(&retained)));
        slot.at = [-1.0; 3];
        slot.tag = u32::MAX;
        slot.inverse_inertia_full = [-1.0; 3];
        slot.inverse_mass = -1.0;
        slot.inverse_inertia_split = [-1.0; 3];
        slot.state = u32::MAX;
        slot.force = [-1.0; 3];
        slot.kinematic_value = -1.0;
        slot.torque = [-1.0; 3];
        slot.cooldown = u32::MAX;

        slot.reset_native_fields();

        assert_eq!(slot.id, id);
        assert_eq!(slot.occurrence, occurrence);
        assert_eq!(slot.links, links);
        assert_eq!(slot.stasis, authority);
        assert!(
            matches!(slot.inertia.as_ref(), Some(RetainedInertia::Immutable(owner)) if Arc::ptr_eq(owner, &retained))
        );
        assert_eq!(bits4(slot.quaternion), [0, 0, 0, 0x3f80_0000]);
        assert_eq!(bits3(slot.center_of_mass), [0; 3]);
        assert_eq!(bits3(slot.velocity), [0; 3]);
        assert_eq!(bits3(slot.omega), [0; 3]);
        assert_eq!(bits3(slot.right), [0x3f80_0000, 0, 0]);
        assert_eq!(bits3(slot.up), [0, 0x3f80_0000, 0]);
        assert_eq!(bits3(slot.at), [0, 0, 0x3f80_0000]);
        assert_eq!(slot.tag, 0);
        assert_eq!(bits3(slot.inverse_inertia_full), [0; 3]);
        assert_eq!(slot.inverse_mass.to_bits(), 0);
        assert_eq!(bits3(slot.inverse_inertia_split), [0; 3]);
        assert_eq!(slot.state, 0);
        assert_eq!(bits3(slot.force), [0; 3]);
        assert_eq!(slot.kinematic_value.to_bits(), 0);
        assert_eq!(bits3(slot.torque), [0; 3]);
        assert_eq!(slot.cooldown, 0);
    }

    #[test]
    fn pose_has_explicit_host_canonical_w_lanes() {
        let matrix = [
            1.0,
            0.0,
            0.0,
            f32::from_bits(0x7fc0_1234),
            0.0,
            1.0,
            0.0,
            f32::from_bits(0xff80_0000),
            0.0,
            0.0,
            1.0,
            f32::from_bits(0x4f12_3456),
            7.0,
            8.0,
            9.0,
            f32::from_bits(0x8000_0000),
        ];
        let mut pool = pool_with_state(1, [0.0; 3], 0);
        let handle = pool
            .try_add_rigid_body(matrix, None, 4)
            .expect("pool")
            .expect("body");
        let pose = pool.try_rigid_body_pose(handle).expect("pose");
        assert_eq!(pose[3].to_bits(), 0);
        assert_eq!(pose[7].to_bits(), 0);
        assert_eq!(pose[11].to_bits(), 0);
        assert_eq!(pose[15].to_bits(), 1.0_f32.to_bits());
        assert_eq!(
            bits3([pose[12], pose[13], pose[14]]),
            bits3([7.0, 8.0, 9.0])
        );
    }

    #[test]
    fn dynamic_update_configuration_has_explicit_unstarted_policy_and_source_ordered_frame_values()
    {
        let mut pool = SimulationRigidBodyPool::try_new(0).expect("pool");
        let initial_frame = pool.frame_identity().expect("initial host epoch");
        {
            let phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("initial phase");
            assert_eq!(phase.frame_identity(), initial_frame);
        }
        assert_eq!(pool.timestep().to_bits(), 0);
        assert_eq!(pool.frequency().to_bits(), 0);
        assert_eq!(
            pool.minimum_energy().to_bits(),
            SIMULATION_INITIAL_MINIMUM_ENERGY.to_bits()
        );

        let timestep = f32::from_bits(0x3c88_8889);
        pool.begin_dynamic_update_frame(timestep);
        let started_frame = pool.frame_identity().expect("started host epoch");
        assert_ne!(started_frame, initial_frame);
        {
            let phase = pool
                .try_prepare_dynamic_update_phase()
                .expect("started phase");
            assert_eq!(phase.frame_identity(), started_frame);
        }
        pool.set_minimum_energy(f32::from_bits(0x3e80_0000));
        assert_eq!(pool.timestep().to_bits(), timestep.to_bits());
        assert_eq!(pool.frequency().to_bits(), 0x426f_ffff);
        assert_eq!(pool.minimum_energy().to_bits(), 0x3e80_0000);
        let snapshot = pool.snapshot();
        assert_eq!(snapshot.timestep.to_bits(), pool.timestep().to_bits());
        assert_eq!(snapshot.frequency.to_bits(), pool.frequency().to_bits());
        assert_eq!(
            snapshot.minimum_energy.to_bits(),
            pool.minimum_energy().to_bits()
        );
    }

    #[test]
    fn all_dynamic_inertia_controls_are_retained_bit_exact_without_changing_add_math() {
        let source = Arc::new(SimulationInertia {
            inverse_tensor: [0.75, 1.25, 2.5],
            inverse_mass: 0.125,
            spherical: f32::from_bits(0x3e91_2345),
            max_velocity: f32::from_bits(0x447a_4321),
            max_omega: f32::from_bits(0x42f1_3579),
            linear_drag: f32::from_bits(0x3dca_feba),
            angular_drag: f32::from_bits(0x3eab_cdef),
        });
        let comparison = Arc::new(SimulationInertia {
            inverse_tensor: source.inverse_tensor,
            inverse_mass: source.inverse_mass,
            spherical: 7.0,
            max_velocity: 8.0,
            max_omega: 9.0,
            linear_drag: 10.0,
            angular_drag: 11.0,
        });
        let mut pool = SimulationRigidBodyPool::try_new(4).expect("pool");
        let active = add(&mut pool, Some(&source), 4);
        let frozen = add(&mut pool, Some(&source), 2);
        let static_body = add(&mut pool, Some(&source), 1);
        let comparison_body = add(&mut pool, Some(&comparison), 4);
        let snapshot = pool.snapshot();

        for handle in [active, frozen] {
            let retained = snapshot.slots[handle.id().index() as usize]
                .inertia
                .expect("dynamic body retains inertia");
            assert_eq!(retained.spherical.to_bits(), 0x3e91_2345);
            assert_eq!(retained.max_velocity.to_bits(), 0x447a_4321);
            assert_eq!(retained.max_omega.to_bits(), 0x42f1_3579);
            assert_eq!(retained.linear_drag.to_bits(), 0x3dca_feba);
            assert_eq!(retained.angular_drag.to_bits(), 0x3eab_cdef);
        }
        assert!(
            snapshot.slots[static_body.id().index() as usize]
                .inertia
                .is_none()
        );

        let active_slot = &snapshot.slots[active.id().index() as usize];
        let comparison_slot = &snapshot.slots[comparison_body.id().index() as usize];
        assert_eq!(
            bits3(active_slot.inverse_inertia_full),
            bits3(comparison_slot.inverse_inertia_full)
        );
        assert_eq!(
            bits3(active_slot.inverse_inertia_split),
            bits3(comparison_slot.inverse_inertia_split)
        );
        assert_eq!(
            active_slot.inverse_mass.to_bits(),
            comparison_slot.inverse_mass.to_bits()
        );
    }

    #[test]
    fn dynamic_update_phase_reserves_and_checks_every_body_before_effects() {
        let retained = inertia(1.0);
        let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
        let first = add(&mut pool, Some(&retained), 4);
        let second = add(&mut pool, Some(&retained), 4);
        pool.begin_dynamic_update_frame(0.25);
        pool.try_with_solver_delta_row_mut(first.observation(), |row| {
            row[0] = [1.0, 2.0, 3.0, 4.0];
        })
        .expect("first row");
        pool.try_with_solver_delta_row_mut(second.observation(), |row| {
            row[3] = [5.0, 6.0, 7.0, 8.0];
        })
        .expect("second row");
        let before = pool.snapshot();
        let rows_before = pool.solver_delta_rows.clone();

        let mut requested = None;
        let result = pool.try_prepare_dynamic_update_phase_with_reservation(|_, active_count| {
            requested = Some(active_count);
            Err(())
        });
        assert_eq!(requested, Some(2));
        assert!(matches!(
            result,
            Err(SimulationRigidBodyDynamicUpdatePrepareError::AllocationFailed { active_count: 2 })
        ));
        assert_eq!(
            SimulationRigidBodyDynamicUpdatePrepareError::AllocationFailed { active_count: 2 }
                .into_pool_error(),
            SimulationRigidBodyPoolError::DynamicUpdatePlanAllocationFailed { active_count: 2 }
        );
        assert_eq!(pool.snapshot(), before);
        assert_eq!(pool.solver_delta_rows, rows_before);

        assert!(matches!(
            pool.try_prepare_dynamic_update_phase_with_reservation(|_, _| Ok(())),
            Err(SimulationRigidBodyDynamicUpdatePrepareError::AllocationFailed { active_count: 2 })
        ));
        assert_eq!(pool.snapshot(), before);
        assert_eq!(pool.solver_delta_rows, rows_before);

        pool.validated_topology.take(); // Test-only corruption bypasses ring mutators.
        pool.anchors[SimulationRigidBodyList::Static.index()].right =
            SimulationRigidBodyNode::Body(first.id());
        let mut reserve_called = false;
        let result = pool.try_prepare_dynamic_update_phase_with_reservation(|_, _| {
            reserve_called = true;
            Ok(())
        });
        assert!(matches!(
            result,
            Err(SimulationRigidBodyDynamicUpdatePrepareError::Pool(
                SimulationRigidBodyPoolError::CorruptStructure
            ))
        ));
        assert!(!reserve_called);
    }

    #[test]
    fn dynamic_update_phase_freezes_active_right_order_inertia_and_configuration() {
        let inertia_a = Arc::new(SimulationInertia {
            inverse_tensor: [1.0, 2.0, 3.0],
            inverse_mass: 0.25,
            ..SimulationInertia::native_default()
        });
        let inertia_b = Arc::new(SimulationInertia {
            inverse_tensor: [4.0, 5.0, 6.0],
            inverse_mass: 0.5,
            ..SimulationInertia::native_default()
        });
        let inertia_c = Arc::new(SimulationInertia {
            inverse_tensor: [7.0, 8.0, 9.0],
            inverse_mass: 0.75,
            ..SimulationInertia::native_default()
        });
        let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
        let first = add(&mut pool, Some(&inertia_a), 4);
        let recycled = add(&mut pool, Some(&inertia_b), 4);
        let third = add(&mut pool, Some(&inertia_c), 4);
        pool.try_remove_rigid_body(recycled).expect("remove middle");
        let recycled = add(&mut pool, Some(&inertia_b), 4);
        pool.begin_dynamic_update_frame(0.125);
        pool.set_gravity([9.0, -8.0, 7.0]);
        pool.set_cooldown(17);
        pool.set_minimum_energy(0.375);

        let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
        assert_eq!(
            phase
                .active
                .iter()
                .map(|entry| entry.index)
                .collect::<Vec<_>>(),
            [
                first.id().index() as usize,
                third.id().index() as usize,
                recycled.id().index() as usize,
            ]
        );
        assert_eq!(
            phase
                .active
                .iter()
                .map(|entry| entry.inertia.inverse_mass.to_bits())
                .collect::<Vec<_>>(),
            [0.25_f32.to_bits(), 0.75_f32.to_bits(), 0.5_f32.to_bits()]
        );
        assert_eq!(phase.configuration.timestep.to_bits(), 0.125_f32.to_bits());
        assert_eq!(phase.configuration.frequency.to_bits(), 8.0_f32.to_bits());
        assert_eq!(
            phase.configuration.gravity.map(f32::to_bits),
            bits3([9.0, -8.0, 7.0])
        );
        assert_eq!(phase.configuration.maximum_cooldown, 17);
        assert_eq!(
            phase.configuration.minimum_energy.to_bits(),
            0.375_f32.to_bits()
        );
    }

    #[test]
    fn dropping_dynamic_update_phase_after_rf_stores_never_integrates_or_clears() {
        let retained = inertia(1.0);
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let body = add(&mut pool, Some(&retained), 4);
        pool.begin_dynamic_update_frame(0.5);
        let body_before = pool.snapshot().slots[body.id().index() as usize].clone();

        {
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let transaction = phase
                .try_begin_solver_delta_transaction([(body.observation(), body.observation())])
                .expect("RF transaction");
            transaction
                .try_apply_all(&mut [()], |&mut (), _| {
                    SimulationRigidBodySolverDeltaStores::new(
                        [1.0, 2.0, 3.0, 4.0],
                        [5.0, 6.0, 7.0, 8.0],
                        [9.0, 10.0, 11.0, 12.0],
                        [13.0, 14.0, 15.0, 16.0],
                    )
                })
                .expect("ordered stores");
        }

        assert_eq!(
            pool.snapshot().slots[body.id().index() as usize],
            body_before
        );
        assert_eq!(
            pool.try_with_solver_delta_row(body.observation(), |row| *row)
                .expect("stored RF"),
            [
                [5.0, 6.0, 7.0, 8.0],
                [0.0; 4],
                [13.0, 14.0, 15.0, 16.0],
                [0.0; 4],
            ]
        );
    }

    #[test]
    fn dynamic_update_consume_uses_only_its_copied_plan_and_configuration() {
        let retained = Arc::new(SimulationInertia {
            max_velocity: 100.0,
            ..SimulationInertia::native_default()
        });
        let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
        let first = add(&mut pool, Some(&retained), 4);
        let second = add(&mut pool, Some(&retained), 4);
        let first_index = first.id().index() as usize;
        let second_index = second.id().index() as usize;
        pool.slots[first_index].velocity = [4.0, 0.0, 0.0];
        pool.slots[first_index].force = [0.0; 3];
        pool.slots[second_index].velocity = [7.0, 0.0, 0.0];
        pool.slots[second_index].force = [0.0; 3];
        pool.solver_delta_rows[second_index].vectors[1] = [3.0, 4.0, 5.0, 6.0];
        pool.begin_dynamic_update_frame(1.0);
        pool.set_gravity([10.0, 0.0, 0.0]);
        let second_before = pool.snapshot().slots[second_index].clone();
        let second_row_before = pool.solver_delta_rows[second_index];

        let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
        phase.configuration.gravity = [1.0, 0.0, 0.0];
        phase.active[0].inertia.linear_drag = 0.5;
        phase.active[1].index = first_index;
        phase.active[1].inertia.linear_drag = 0.75;
        phase.integrate_ordinary_finite();

        // First plan entry: 4 * (1 - 0.5) = 2. Second entry observes
        // the phase gravity left as force: (2 + 1) * (1 - 0.75) = 0.75.
        // Reversing the two copied entries yields 1.0 instead.
        assert_eq!(
            bits3(pool.slots[first_index].velocity),
            bits3([0.75, 0.0, 0.0])
        );
        assert_eq!(bits3(pool.slots[first_index].force), bits3([1.0, 0.0, 0.0]));
        assert_eq!(pool.solver_delta_rows[first_index].vectors, [[0.0; 4]; 4]);
        assert_eq!(pool.snapshot().slots[second_index], second_before);
        assert_eq!(pool.solver_delta_rows[second_index], second_row_before);
        assert_eq!(bits3(pool.gravity), bits3([10.0, 0.0, 0.0]));
    }

    #[test]
    fn dynamic_update_phase_rejects_later_stale_rf_pair_before_any_effect() {
        let retained = inertia(1.0);
        let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
        let first = add(&mut pool, Some(&retained), 4);
        let stale = add(&mut pool, Some(&retained), 4);
        pool.try_remove_rigid_body(stale).expect("remove second");
        let fresh = add(&mut pool, Some(&retained), 4);
        pool.try_with_solver_delta_row_mut(first.observation(), |row| {
            row[0] = [1.0, 2.0, 3.0, 4.0];
        })
        .expect("first row");
        pool.try_with_solver_delta_row_mut(fresh.observation(), |row| {
            row[2] = [5.0, 6.0, 7.0, 8.0];
        })
        .expect("fresh row");
        let before = after_one_phase_identity_issue(pool.snapshot());
        let rows_before = pool.solver_delta_rows.clone();

        {
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            assert!(matches!(
                phase.try_begin_solver_delta_transaction([
                    (first.observation(), fresh.observation()),
                    (fresh.observation(), stale.observation()),
                ]),
                Err(SimulationRigidBodySolverDeltaTransactionError::Body {
                    ordinal: 1,
                    endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyB,
                    source: SimulationRigidBodyPoolError::StaleHandle { slot },
                }) if slot == stale.id().index()
            ));
        }

        assert_eq!(pool.snapshot(), before);
        assert_eq!(pool.solver_delta_rows, rows_before);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "all indexed preflight error priorities share one unchanged-state fixture"
    )]
    fn indexed_rf_sequence_preflights_allocation_items_and_bodies_before_effects() {
        let retained = inertia(1.0);
        let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
        let first = add(&mut pool, Some(&retained), 4);
        let stale = add(&mut pool, Some(&retained), 4);
        pool.try_remove_rigid_body(stale).expect("remove second");
        let fresh = add(&mut pool, Some(&retained), 4);
        pool.solver_delta_rows[first.id().index() as usize].vectors[0] = [1.0; 4];
        pool.solver_delta_rows[fresh.id().index() as usize].vectors[2] = [2.0; 4];
        let pool_before = after_one_phase_identity_issue(pool.snapshot());
        let rows_before = pool.solver_delta_rows.clone();
        let mut items = [10_u32, 20_u32];
        let items_before = items;
        let mut calls = 0;

        {
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let out_of_range = phase.try_apply_indexed_solver_delta_sequence(
                &mut items,
                [
                    (0, first.observation(), fresh.observation()),
                    (2, fresh.observation(), first.observation()),
                ],
                |_, _| {
                    calls += 1;
                    SimulationRigidBodySolverDeltaStores::new(
                        [3.0; 4], [4.0; 4], [5.0; 4], [6.0; 4],
                    )
                },
            );
            assert_eq!(
                out_of_range,
                Err(SimulationRigidBodyIndexedSolverDeltaError::ItemOutOfRange {
                    ordinal: 1,
                    item_index: 2,
                    item_count: 2,
                })
            );

            let stale_body = phase.try_apply_indexed_solver_delta_sequence(
                &mut items,
                [
                    (0, first.observation(), fresh.observation()),
                    (1, fresh.observation(), stale.observation()),
                ],
                |_, _| {
                    calls += 1;
                    SimulationRigidBodySolverDeltaStores::new(
                        [3.0; 4], [4.0; 4], [5.0; 4], [6.0; 4],
                    )
                },
            );
            assert!(matches!(
                stale_body,
                Err(SimulationRigidBodyIndexedSolverDeltaError::Body {
                    ordinal: 1,
                    endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyB,
                    source: SimulationRigidBodyPoolError::StaleHandle { slot },
                }) if slot == stale.id().index()
            ));

            let allocation = phase.try_apply_indexed_solver_delta_sequence_with_reservation(
                &mut items,
                [(0, first.observation(), fresh.observation())].into_iter(),
                |_, _| {
                    calls += 1;
                    SimulationRigidBodySolverDeltaStores::new(
                        [3.0; 4], [4.0; 4], [5.0; 4], [6.0; 4],
                    )
                },
                |plan, entry_count| {
                    assert!(plan.is_empty());
                    assert_eq!(entry_count, 1);
                    Err(())
                },
                |_, _, _| {},
            );
            assert_eq!(
                allocation,
                Err(
                    SimulationRigidBodyIndexedSolverDeltaError::AllocationFailed { entry_count: 1 }
                )
            );

            let insufficient = phase.try_apply_indexed_solver_delta_sequence_with_reservation(
                &mut items,
                [(0, first.observation(), fresh.observation())].into_iter(),
                |_, _| {
                    calls += 1;
                    SimulationRigidBodySolverDeltaStores::new(
                        [3.0; 4], [4.0; 4], [5.0; 4], [6.0; 4],
                    )
                },
                |_, _| Ok(()),
                |_, _, _| {},
            );
            assert_eq!(
                insufficient,
                Err(
                    SimulationRigidBodyIndexedSolverDeltaError::AllocationFailed { entry_count: 1 }
                )
            );
        }

        assert_eq!(calls, 0);
        assert_eq!(items, items_before);
        assert_eq!(pool.snapshot(), pool_before);
        assert_eq!(pool.solver_delta_rows, rows_before);
    }

    #[test]
    fn indexed_rf_sequence_preserves_selector_duplicates_self_and_shared_visibility() {
        let retained = inertia(1.0);
        let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
        let body_p = add(&mut pool, Some(&retained), 4);
        let body_x = add(&mut pool, Some(&retained), 4);
        let body_q = add(&mut pool, Some(&retained), 4);
        for (body, seed) in [(body_p, 1.0), (body_x, 10.0), (body_q, 20.0)] {
            let row = &mut pool.solver_delta_rows[body.id().index() as usize].vectors;
            *row = [
                [seed; 4],
                [seed + 10.0; 4],
                [seed + 20.0; 4],
                [seed + 30.0; 4],
            ];
        }
        let untouched_p = [
            pool.solver_delta_rows[body_p.id().index() as usize].vectors[1],
            pool.solver_delta_rows[body_p.id().index() as usize].vectors[3],
        ];
        let mut items = [0_u32; 3];
        let mut seen = Vec::new();
        let mut stores = Vec::new();
        let mut call = 0.0_f32;

        {
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            phase
                .try_apply_indexed_solver_delta_sequence_with_reservation(
                    &mut items,
                    [
                        (2, body_p.observation(), body_x.observation()),
                        (0, body_x.observation(), body_q.observation()),
                        (2, body_q.observation(), body_p.observation()),
                        (1, body_p.observation(), body_p.observation()),
                    ]
                    .into_iter(),
                    |item, rows| {
                        let (body_a, body_b) = rows.into_rows();
                        seen.push((*item, body_a, body_b));
                        *item += 1;
                        call += 1.0;
                        let base = call * 100.0;
                        SimulationRigidBodySolverDeltaStores::new(
                            [base + 1.0; 4],
                            [base + 2.0; 4],
                            [base + 3.0; 4],
                            [base + 4.0; 4],
                        )
                    },
                    |plan, entry_count| plan.try_reserve_exact(entry_count).map_err(|_| ()),
                    |ordinal, kind, value| stores.push((ordinal, kind, value)),
                )
                .expect("indexed sequence");
        }

        assert_eq!(items, [1, 1, 2]);
        assert_eq!(
            seen.iter().map(|entry| entry.0).collect::<Vec<_>>(),
            [0, 0, 1, 0]
        );
        assert_eq!(bits4(seen[1].1[0]), bits4([102.0; 4]));
        assert_eq!(bits4(seen[1].1[2]), bits4([104.0; 4]));
        assert_eq!(bits4(seen[2].2[0]), bits4([101.0; 4]));
        assert_eq!(bits4(seen[2].2[2]), bits4([103.0; 4]));
        assert_eq!(bits4(seen[3].1[0]), bits4([302.0; 4]));
        assert_eq!(bits4(seen[3].2[0]), bits4([302.0; 4]));
        assert_eq!(bits4(seen[3].1[2]), bits4([304.0; 4]));
        assert_eq!(bits4(seen[3].2[2]), bits4([304.0; 4]));
        let row_p = pool.solver_delta_rows[body_p.id().index() as usize].vectors;
        assert_eq!(bits4(row_p[0]), bits4([402.0; 4]));
        assert_eq!(bits4(row_p[2]), bits4([404.0; 4]));
        assert_eq!([row_p[1], row_p[3]], untouched_p);
        assert_eq!(stores.len(), 16);
        for (ordinal, chunk) in stores.chunks_exact(4).enumerate() {
            assert_eq!(
                chunk.iter().map(|entry| entry.0).collect::<Vec<_>>(),
                [ordinal; 4]
            );
            assert_eq!(
                chunk.iter().map(|entry| entry.1).collect::<Vec<_>>(),
                [
                    SimulationRigidBodySolverDeltaStoreKind::BodyA0,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB0,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA2,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB2,
                ]
            );
        }
    }

    #[test]
    fn indexed_rf_sequence_panic_keeps_only_committed_prefix_and_current_item() {
        let retained = inertia(1.0);
        let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
        let first = add(&mut pool, Some(&retained), 4);
        let current = add(&mut pool, Some(&retained), 4);
        let later = add(&mut pool, Some(&retained), 4);
        for (body, seed) in [(first, 1.0), (current, 11.0), (later, 21.0)] {
            pool.solver_delta_rows[body.id().index() as usize].vectors =
                [[seed; 4], [seed + 1.0; 4], [seed + 2.0; 4], [seed + 3.0; 4]];
        }
        let body_before = after_one_phase_identity_issue(pool.snapshot());
        let rows_before = pool.solver_delta_rows.clone();
        let mut items = [0_u32; 3];
        let mut call = 0_u32;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let _ = phase.try_apply_indexed_solver_delta_sequence(
                &mut items,
                [
                    (0, first.observation(), first.observation()),
                    (1, current.observation(), current.observation()),
                    (2, later.observation(), later.observation()),
                ],
                |item, _| {
                    *item += 1;
                    call += 1;
                    assert_ne!(call, 2, "panic inside current indexed callback");
                    SimulationRigidBodySolverDeltaStores::new(
                        [101.0; 4], [102.0; 4], [103.0; 4], [104.0; 4],
                    )
                },
            );
        }));

        assert!(result.is_err());
        assert_eq!(items, [1, 1, 0]);
        assert_eq!(pool.snapshot(), body_before);
        assert_eq!(
            pool.solver_delta_rows[first.id().index() as usize]
                .vectors
                .map(bits4),
            [
                [102.0; 4],
                rows_before[first.id().index() as usize].vectors[1],
                [104.0; 4],
                rows_before[first.id().index() as usize].vectors[3],
            ]
            .map(bits4)
        );
        assert_eq!(
            pool.solver_delta_rows[current.id().index() as usize]
                .vectors
                .map(bits4),
            rows_before[current.id().index() as usize]
                .vectors
                .map(bits4)
        );
        assert_eq!(
            pool.solver_delta_rows[later.id().index() as usize]
                .vectors
                .map(bits4),
            rows_before[later.id().index() as usize].vectors.map(bits4)
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one Contact sequence fixture freezes selectors, live sharing, self-aliasing, and all eight stores"
    )]
    fn indexed_contact_rf_sequence_preserves_order_and_all_row_visibility() {
        let retained = inertia(1.0);
        let mut pool = SimulationRigidBodyPool::try_new(3).expect("pool");
        let body_p = add(&mut pool, Some(&retained), 4);
        let body_x = add(&mut pool, Some(&retained), 4);
        let body_q = add(&mut pool, Some(&retained), 4);
        for (body, seed) in [(body_p, 1.0), (body_x, 10.0), (body_q, 20.0)] {
            pool.solver_delta_rows[body.id().index() as usize].vectors = [
                [seed; 4],
                [seed + 10.0; 4],
                [seed + 20.0; 4],
                [seed + 30.0; 4],
            ];
        }
        let mut items = [0_u32; 3];
        let mut seen = Vec::new();
        let mut stores = Vec::new();
        let mut call = 0.0_f32;

        {
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            phase
                .try_apply_indexed_contact_solver_delta_sequence_with_reservation(
                    &mut items,
                    [
                        (2, body_p.observation(), body_x.observation()),
                        (0, body_x.observation(), body_q.observation()),
                        (2, body_q.observation(), body_p.observation()),
                        (1, body_p.observation(), body_p.observation()),
                    ]
                    .into_iter(),
                    |item, rows| {
                        let (body_a, body_b) = rows.into_rows();
                        seen.push((*item, body_a, body_b));
                        *item += 1;
                        call += 1.0;
                        let base = call * 100.0;
                        SimulationRigidBodyContactSolverDeltaStores::new(
                            [base + 1.0; 4],
                            [base + 2.0; 4],
                            [base + 3.0; 4],
                            [base + 4.0; 4],
                            [base + 5.0; 4],
                            [base + 6.0; 4],
                            [base + 7.0; 4],
                            [base + 8.0; 4],
                        )
                    },
                    |plan, entry_count| plan.try_reserve_exact(entry_count).map_err(|_| ()),
                    |ordinal, kind, value| stores.push((ordinal, kind, value)),
                )
                .expect("indexed Contact sequence");
        }

        assert_eq!(items, [1, 1, 2]);
        assert_eq!(
            seen.iter().map(|entry| entry.0).collect::<Vec<_>>(),
            [0, 0, 1, 0]
        );
        assert_eq!(
            seen[1].1.map(bits4),
            [[108.0; 4], [104.0; 4], [106.0; 4], [102.0; 4]].map(bits4)
        );
        assert_eq!(
            seen[2].2.map(bits4),
            [[107.0; 4], [103.0; 4], [105.0; 4], [101.0; 4]].map(bits4)
        );
        assert_eq!(
            seen[3].1.map(bits4),
            [[308.0; 4], [304.0; 4], [306.0; 4], [302.0; 4]].map(bits4)
        );
        assert_eq!(seen[3].2.map(bits4), seen[3].1.map(bits4));
        assert_eq!(
            pool.solver_delta_rows[body_p.id().index() as usize]
                .vectors
                .map(bits4),
            [[408.0; 4], [404.0; 4], [406.0; 4], [402.0; 4]].map(bits4)
        );
        assert_eq!(stores.len(), 32);
        for (ordinal, chunk) in stores.chunks_exact(8).enumerate() {
            assert_eq!(
                chunk.iter().map(|entry| entry.0).collect::<Vec<_>>(),
                [ordinal; 8]
            );
            assert_eq!(
                chunk.iter().map(|entry| entry.1).collect::<Vec<_>>(),
                [
                    SimulationRigidBodySolverDeltaStoreKind::BodyA3,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB3,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA1,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB1,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA2,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB2,
                    SimulationRigidBodySolverDeltaStoreKind::BodyA0,
                    SimulationRigidBodySolverDeltaStoreKind::BodyB0,
                ]
            );
        }
    }

    #[test]
    fn indexed_contact_rf_sequence_preflights_later_entries_and_allocation() {
        let retained = inertia(1.0);
        let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
        let first = add(&mut pool, Some(&retained), 4);
        let stale = add(&mut pool, Some(&retained), 4);
        pool.try_remove_rigid_body(stale).expect("remove second");
        let fresh = add(&mut pool, Some(&retained), 4);
        let pool_before = after_one_phase_identity_issue(pool.snapshot());
        let rows_before = pool.solver_delta_rows.clone();
        let mut items = [10_u32, 20_u32];
        let items_before = items;
        let mut calls = 0;
        let stores = || {
            SimulationRigidBodyContactSolverDeltaStores::new(
                [1.0; 4], [2.0; 4], [3.0; 4], [4.0; 4], [5.0; 4], [6.0; 4], [7.0; 4], [8.0; 4],
            )
        };

        {
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            assert_eq!(
                phase.try_apply_indexed_contact_solver_delta_sequence(
                    &mut items,
                    [
                        (0, first.observation(), fresh.observation()),
                        (2, fresh.observation(), first.observation()),
                    ],
                    |_, _| {
                        calls += 1;
                        stores()
                    },
                ),
                Err(SimulationRigidBodyIndexedSolverDeltaError::ItemOutOfRange {
                    ordinal: 1,
                    item_index: 2,
                    item_count: 2,
                })
            );
            assert!(matches!(
                phase.try_apply_indexed_contact_solver_delta_sequence(
                    &mut items,
                    [
                        (0, first.observation(), fresh.observation()),
                        (1, fresh.observation(), stale.observation()),
                    ],
                    |_, _| {
                        calls += 1;
                        stores()
                    },
                ),
                Err(SimulationRigidBodyIndexedSolverDeltaError::Body {
                    ordinal: 1,
                    endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyB,
                    source: SimulationRigidBodyPoolError::StaleHandle { slot },
                }) if slot == stale.id().index()
            ));
            assert_eq!(
                phase.try_apply_indexed_contact_solver_delta_sequence_with_reservation(
                    &mut items,
                    [(0, first.observation(), fresh.observation())].into_iter(),
                    |_, _| {
                        calls += 1;
                        stores()
                    },
                    |_, entry_count| {
                        assert_eq!(entry_count, 1);
                        Err(())
                    },
                    |_, _, _| {},
                ),
                Err(
                    SimulationRigidBodyIndexedSolverDeltaError::AllocationFailed { entry_count: 1 }
                )
            );
            assert_eq!(
                phase.try_apply_indexed_contact_solver_delta_sequence_with_reservation(
                    &mut items,
                    [(0, first.observation(), fresh.observation())].into_iter(),
                    |_, _| {
                        calls += 1;
                        stores()
                    },
                    |_, entry_count| {
                        assert_eq!(entry_count, 1);
                        Ok(())
                    },
                    |_, _, _| {},
                ),
                Err(
                    SimulationRigidBodyIndexedSolverDeltaError::AllocationFailed { entry_count: 1 }
                )
            );
        }

        assert_eq!(calls, 0);
        assert_eq!(items, items_before);
        assert_eq!(pool.snapshot(), pool_before);
        assert_eq!(pool.solver_delta_rows, rows_before);
    }

    #[test]
    fn indexed_contact_rf_store_observer_panic_keeps_exact_native_prefix() {
        let retained = inertia(1.0);
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let body = add(&mut pool, Some(&retained), 4);
        let original = [[11.0; 4], [12.0; 4], [13.0; 4], [14.0; 4]];
        pool.solver_delta_rows[body.id().index() as usize].vectors = original;
        let mut items = [0_u32];

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let _ = phase.try_apply_indexed_contact_solver_delta_sequence_with_reservation(
                &mut items,
                [(0, body.observation(), body.observation())].into_iter(),
                |item, _| {
                    *item += 1;
                    SimulationRigidBodyContactSolverDeltaStores::new(
                        [1.0; 4], [2.0; 4], [3.0; 4], [4.0; 4], [5.0; 4], [6.0; 4], [7.0; 4],
                        [8.0; 4],
                    )
                },
                |plan, entry_count| plan.try_reserve_exact(entry_count).map_err(|_| ()),
                |_, kind, _| {
                    assert_ne!(
                        kind,
                        SimulationRigidBodySolverDeltaStoreKind::BodyB1,
                        "panic after the live B1 store"
                    );
                },
            );
        }));

        assert!(result.is_err());
        assert_eq!(items, [1]);
        assert_eq!(
            pool.solver_delta_rows[body.id().index() as usize]
                .vectors
                .map(bits4),
            [[11.0; 4], [4.0; 4], [13.0; 4], [2.0; 4]].map(bits4)
        );
    }

    #[test]
    fn missing_active_inertia_rejects_entire_batch_before_body_or_rf_mutation() {
        let source = inertia(1.0);
        let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
        let retained = add(&mut pool, Some(&source), 4);
        let missing = add(&mut pool, None, 4);
        pool.begin_dynamic_update_frame(1.0 / 60.0);
        pool.try_with_solver_delta_row_mut(missing.observation(), |row| {
            row[0] = [1.0, 2.0, 3.0, 4.0];
        })
        .expect("missing row");
        pool.try_with_solver_delta_row_mut(retained.observation(), |row| {
            row[3] = [5.0, 6.0, 7.0, 8.0];
        })
        .expect("retained row");
        let before = pool.snapshot();
        let rows_before = pool.solver_delta_rows.clone();

        assert_eq!(
            pool.try_integrate_active_bodies_ordinary_finite(),
            Err(SimulationRigidBodyPoolError::ActiveBodyMissingInertia {
                slot: missing.id().index(),
            })
        );
        assert_eq!(pool.snapshot(), before);
        assert_eq!(pool.solver_delta_rows, rows_before);
    }

    #[test]
    fn active_batch_separates_four_delta_roles_and_clears_rows_in_place() {
        let source = Arc::new(SimulationInertia::native_default());
        let mut pool = pool_with_state(1, [9.0, 8.0, 7.0], 30);
        let handle = add(&mut pool, Some(&source), 4);
        pool.begin_dynamic_update_frame(1.0);
        let index = handle.id().index() as usize;
        pool.slots[index].force = [0.0; 3];
        pool.slots[index].torque = [0.0; 3];
        pool.slots[index].kinematic_value = 0.0;
        pool.solver_delta_rows[index].vectors = [
            [1.0, 0.0, 0.0, 11.0],
            [0.0, 2.0, 0.0, 12.0],
            [0.0, 0.0, 0.1, 13.0],
            [0.2, 0.0, 0.0, 14.0],
        ];
        let links = pool.slots[index].links;
        let occurrence = pool.slots[index].occurrence;
        let stasis = pool.slots[index].stasis;
        let tag = pool.slots[index].tag;
        let inverse_mass = pool.slots[index].inverse_mass;
        let raw_state = pool.slots[index].state;

        pool.try_integrate_active_bodies_ordinary_finite()
            .expect("ordinary-finite batch");
        let slot = &pool.slots[index];
        assert_eq!(bits3(slot.center_of_mass), bits3([1.0, 2.0, 0.0]));
        assert_eq!(bits3(slot.velocity), bits3([1.0, 0.0, 0.0]));
        assert_eq!(bits3(slot.omega), bits3([0.0, 0.0, 0.1]));
        assert_ne!(bits4(slot.quaternion), [0, 0, 0, 0x3f80_0000]);
        assert_eq!(bits3(slot.force), bits3([9.0, 8.0, 7.0]));
        assert_eq!(bits3(slot.torque), [0; 3]);
        assert_eq!(pool.solver_delta_rows[index].vectors, [[0.0; 4]; 4]);
        assert_eq!(slot.links, links);
        assert_eq!(slot.occurrence, occurrence);
        assert_eq!(slot.stasis, stasis);
        assert_eq!(slot.tag, tag);
        assert_eq!(slot.inverse_mass.to_bits(), inverse_mass.to_bits());
        assert_eq!(slot.state, raw_state);
        assert!(
            matches!(slot.inertia.as_ref(), Some(RetainedInertia::Immutable(owner)) if Arc::ptr_eq(owner, &source))
        );
    }

    #[test]
    #[cfg(feature = "skate3-runtime")]
    fn native_mode_publication_never_rewrites_a_second_physical_body() {
        let source = inertia(1.0);
        let mut pool = pool_with_state(1, [0.0, -9.8, 0.0], 9);
        let body = add(&mut pool, Some(&source), 4);
        let original = pool.snapshot().slots[body.id().index() as usize].clone();
        pool.try_set_externally_driven(&[body.observation()]).unwrap();
        let transform = [1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,9.,8.,7.,1.];
        let values = [(body.observation(), transform, [3.,2.,1.], [0.1,0.2,0.3])];
        pool.try_publish_mode_body_views(&values).unwrap();
        assert_eq!(pool.snapshot().slots[body.id().index() as usize], original,
            "a completed mode publication must not copy physical state into the dormant native body");
        assert_eq!(pool.try_rigid_body_pose(body).unwrap(), transform);
        let view = pool.try_drive_build_snapshot(body.observation()).unwrap();
        assert_eq!(view.center_of_mass, [9.,8.,7.]);
        assert_eq!(view.velocity, [3.,2.,1.]);
        assert_eq!(view.omega, [0.1,0.2,0.3]);
        assert!(pool.try_publish_mode_body_views(&[]).is_err());
        assert_eq!(pool.try_rigid_body_pose(body).unwrap(), transform);
        pool.try_remove_rigid_body(body).unwrap();
        let next = add(&mut pool, Some(&source), 4);
        assert_eq!(next.id(), body.id());
        assert!(pool.mode_body_view(next.observation()).is_none());
        assert!(pool.try_publish_mode_body_views(&values).is_err());
        pool.try_set_externally_driven(&[]).unwrap();
        assert!(pool.mode_body_views.is_empty());
    }

    #[test]
    fn alternate_solver_owns_only_exact_body_occurrences() {
        let source = inertia(1.0);
        let mut pool = pool_with_state(2, [0.0, -9.8, 0.0], 9);
        let external = add(&mut pool, Some(&source), 4);
        let native = add(&mut pool, Some(&source), 4);
        pool.slots[external.id().index() as usize].velocity = [1., 0., 0.];
        pool.slots[native.id().index() as usize].velocity = [1., 0., 0.];
        pool.try_set_externally_driven(&[external.observation()]).unwrap();
        let before = pool.snapshot().slots[external.id().index() as usize].clone();
        pool.begin_dynamic_update_frame(0.5);
        pool.try_integrate_active_bodies_ordinary_finite().unwrap();
        assert_eq!(pool.snapshot().slots[external.id().index() as usize].clone(), before);
        assert_ne!(pool.snapshot().slots[native.id().index() as usize].center_of_mass, before.center_of_mass);

        let foreign_pool = pool_with_state(1, [0.; 3], 9);
        let mut foreign_pool = foreign_pool;
        let foreign = add(&mut foreign_pool, Some(&source), 4);
        assert_eq!(pool.try_set_externally_driven(&[foreign.observation()]), Err(SimulationRigidBodyPoolError::ForeignHandle));
        assert!(pool.is_externally_driven(external.observation()));
        assert!(!pool.is_externally_driven(native.observation()));

        // Removing/reusing a slot must never transfer alternate ownership to
        // its next allocation occurrence, even before the driver refreshes.
        pool.try_remove_rigid_body(external).unwrap();
        let replacement = add(&mut pool, Some(&source), 4);
        assert_eq!(replacement.id(), external.id());
        assert!(!pool.is_externally_driven(replacement.observation()));
        assert!(pool.try_set_externally_driven(&[external.observation()]).is_err());
        pool.try_set_externally_driven(&[]).unwrap();
        assert!(pool.externally_driven.is_empty());
    }

    #[test]
    fn batch_walks_only_active_ring_and_preserves_static_and_frozen_payloads() {
        let source = inertia(1.0);
        let mut pool = pool_with_state(4, [3.0, -4.0, 5.0], 9);
        let first_active = add(&mut pool, Some(&source), 4);
        let static_body = add(&mut pool, Some(&source), 1);
        let frozen_body = add(&mut pool, Some(&source), 2);
        let second_active = add(&mut pool, Some(&source), 7);
        pool.begin_dynamic_update_frame(0.5);
        for (ordinal, handle) in [1.0, 2.0, 3.0, 4.0].into_iter().zip([
            first_active,
            static_body,
            frozen_body,
            second_active,
        ]) {
            let index = handle.id().index() as usize;
            pool.slots[index].velocity = [ordinal, 0.0, 0.0];
            pool.solver_delta_rows[index].vectors[0] = [ordinal + 4.0; 4];
        }
        let before = pool.snapshot();
        let rows_before = pool.solver_delta_rows.clone();

        pool.try_integrate_active_bodies_ordinary_finite()
            .expect("ordinary-finite batch");
        let after = pool.snapshot();
        assert_eq!(
            after.active_order,
            vec![first_active.id(), second_active.id()]
        );
        for handle in [static_body, frozen_body] {
            let index = handle.id().index() as usize;
            assert_eq!(after.slots[index], before.slots[index]);
            assert_eq!(pool.solver_delta_rows[index], rows_before[index]);
        }
        for handle in [first_active, second_active] {
            let index = handle.id().index() as usize;
            assert_eq!(pool.solver_delta_rows[index].vectors, [[0.0; 4]; 4]);
            assert_eq!(bits3(after.slots[index].force), bits3([3.0, -4.0, 5.0]));
        }
    }

    #[test]
    fn active_batch_samples_current_frame_energy_cooldown_and_gravity_configuration() {
        let source = Arc::new(SimulationInertia {
            linear_drag: 1.0,
            ..SimulationInertia::native_default()
        });
        let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
        let energetic = add(&mut pool, Some(&source), 4);
        let resting = add(&mut pool, Some(&source), 4);
        let energetic_index = energetic.id().index() as usize;
        let resting_index = resting.id().index() as usize;
        pool.slots[energetic_index].velocity = [1.0, 0.0, 0.0];
        pool.slots[energetic_index].force = [0.0; 3];
        pool.slots[energetic_index].cooldown = 4;
        pool.slots[resting_index].force = [0.0; 3];
        pool.slots[resting_index].kinematic_value = 0.0;
        pool.slots[resting_index].cooldown = 6;

        pool.begin_dynamic_update_frame(0.5);
        pool.set_minimum_energy(0.2);
        pool.set_cooldown(5);
        pool.set_gravity([9.0, -8.0, 7.0]);
        pool.try_integrate_active_bodies_ordinary_finite()
            .expect("ordinary-finite batch");

        assert_eq!(
            bits3(pool.slots[energetic_index].velocity),
            bits3([0.5, 0.0, 0.0])
        );
        assert_eq!(pool.slots[energetic_index].cooldown, 0);
        assert_eq!(pool.slots[resting_index].cooldown, 5);
        for index in [energetic_index, resting_index] {
            assert_eq!(bits3(pool.slots[index].force), bits3([9.0, -8.0, 7.0]));
        }
    }

    #[test]
    fn horus_transaction_reuses_snapshots_in_exact_contact_joint_drive_pass_order() {
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let retained = inertia(1.0);
        let body = add(&mut pool, Some(&retained), 4);
        pool.solver_delta_rows[body.id().index() as usize].vectors =
            [[0.0; 4], [10.0; 4], [20.0; 4], [30.0; 4]];
        let mut contacts = [(0_u8, 0_u32), (1, 0)];
        let mut joints = [(0_u8, 0_u32)];
        let mut drives = [(0_u8, 0_u32), (1, 0)];
        let visits = std::cell::RefCell::new(Vec::new());

        {
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let contact = phase
                .try_prepare_indexed_solver_delta_plan(
                    &mut contacts,
                    [
                        (0, body.observation(), body.observation()),
                        (1, body.observation(), body.observation()),
                    ],
                )
                .expect("Contact plan");
            let joint = phase
                .try_prepare_indexed_solver_delta_plan(
                    &mut joints,
                    [(0, body.observation(), body.observation())],
                )
                .expect("Joint plan");
            let drive = phase
                .try_prepare_indexed_solver_delta_plan(
                    &mut drives,
                    [
                        (0, body.observation(), body.observation()),
                        (1, body.observation(), body.observation()),
                    ],
                )
                .expect("Drive plan");

            phase
                .begin_horus_island_rf_transaction(contact, joint, drive)
                .execute_iterations(
                    2,
                    |pass, item, rows| {
                        let (_, count) = item;
                        *count += 1;
                        let (_, body_b) = rows.into_rows();
                        visits.borrow_mut().push((pass, 'C', item.0, body_b[0][0]));
                        let row_3 = body_b[3].map(|value| value + 1.0);
                        let row_1 = body_b[1].map(|value| value + 1.0);
                        let row_2 = body_b[2].map(|value| value + 1.0);
                        let row_0 = body_b[0].map(|value| value + 1.0);
                        SimulationRigidBodyContactSolverDeltaStores::new(
                            row_3, row_3, row_1, row_1, row_2, row_2, row_0, row_0,
                        )
                    },
                    |pass, item, rows| {
                        let (_, count) = item;
                        *count += 1;
                        let (_, body_b) = rows.into_rows();
                        visits.borrow_mut().push((pass, 'J', item.0, body_b[0][0]));
                        let row_0 = body_b[0].map(|value| value + 10.0);
                        let row_2 = body_b[2].map(|value| value + 10.0);
                        SimulationRigidBodySolverDeltaStores::new(row_0, row_0, row_2, row_2)
                    },
                    |pass, item, rows| {
                        let (_, count) = item;
                        *count += 1;
                        let (_, body_b) = rows.into_rows();
                        visits.borrow_mut().push((pass, 'D', item.0, body_b[0][0]));
                        let row_0 = body_b[0].map(|value| value + 100.0);
                        let row_2 = body_b[2].map(|value| value + 100.0);
                        SimulationRigidBodySolverDeltaStores::new(row_0, row_0, row_2, row_2)
                    },
                );
        }

        assert_eq!(
            visits.into_inner(),
            [
                (0, 'C', 0, 0.0),
                (0, 'C', 1, 1.0),
                (0, 'J', 0, 2.0),
                (0, 'D', 0, 12.0),
                (0, 'D', 1, 112.0),
                (1, 'C', 0, 212.0),
                (1, 'C', 1, 213.0),
                (1, 'J', 0, 214.0),
                (1, 'D', 0, 224.0),
                (1, 'D', 1, 324.0),
            ]
        );
        assert_eq!(contacts, [(0, 2), (1, 2)]);
        assert_eq!(joints, [(0, 2)]);
        assert_eq!(drives, [(0, 2), (1, 2)]);
        assert_eq!(
            pool.solver_delta_rows[body.id().index() as usize].vectors,
            [[424.0; 4], [14.0; 4], [444.0; 4], [34.0; 4]]
        );
    }

    #[test]
    fn horus_transaction_panic_retains_only_the_exact_cross_family_prefix() {
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let retained = inertia(1.0);
        let body = add(&mut pool, Some(&retained), 4);
        pool.solver_delta_rows[body.id().index() as usize].vectors =
            [[0.0; 4], [10.0; 4], [20.0; 4], [30.0; 4]];
        let mut contacts = [(0_u8, 0_u32), (1, 0)];
        let mut joints = [(0_u8, 0_u32)];
        let mut drives = [(0_u8, 0_u32), (1, 0)];
        let visits = std::cell::RefCell::new(Vec::new());

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let contact = phase
                .try_prepare_indexed_solver_delta_plan(
                    &mut contacts,
                    [
                        (0, body.observation(), body.observation()),
                        (1, body.observation(), body.observation()),
                    ],
                )
                .expect("Contact plan");
            let joint = phase
                .try_prepare_indexed_solver_delta_plan(
                    &mut joints,
                    [(0, body.observation(), body.observation())],
                )
                .expect("Joint plan");
            let drive = phase
                .try_prepare_indexed_solver_delta_plan(
                    &mut drives,
                    [
                        (0, body.observation(), body.observation()),
                        (1, body.observation(), body.observation()),
                    ],
                )
                .expect("Drive plan");

            phase
                .begin_horus_island_rf_transaction(contact, joint, drive)
                .execute_iterations(
                    2,
                    |pass, item, rows| {
                        let (_, count) = item;
                        *count += 1;
                        let (_, body_b) = rows.into_rows();
                        visits.borrow_mut().push((pass, 'C', item.0, body_b[0][0]));
                        let row_3 = body_b[3].map(|value| value + 1.0);
                        let row_1 = body_b[1].map(|value| value + 1.0);
                        let row_2 = body_b[2].map(|value| value + 1.0);
                        let row_0 = body_b[0].map(|value| value + 1.0);
                        SimulationRigidBodyContactSolverDeltaStores::new(
                            row_3, row_3, row_1, row_1, row_2, row_2, row_0, row_0,
                        )
                    },
                    |pass, item, rows| {
                        let (_, count) = item;
                        *count += 1;
                        let (_, body_b) = rows.into_rows();
                        visits.borrow_mut().push((pass, 'J', item.0, body_b[0][0]));
                        assert_ne!(pass, 1, "injected pass-one Joint panic");
                        let row_0 = body_b[0].map(|value| value + 10.0);
                        let row_2 = body_b[2].map(|value| value + 10.0);
                        SimulationRigidBodySolverDeltaStores::new(row_0, row_0, row_2, row_2)
                    },
                    |pass, item, rows| {
                        let (_, count) = item;
                        *count += 1;
                        let (_, body_b) = rows.into_rows();
                        visits.borrow_mut().push((pass, 'D', item.0, body_b[0][0]));
                        let row_0 = body_b[0].map(|value| value + 100.0);
                        let row_2 = body_b[2].map(|value| value + 100.0);
                        SimulationRigidBodySolverDeltaStores::new(row_0, row_0, row_2, row_2)
                    },
                );
        }));

        assert!(panic.is_err());
        assert_eq!(
            visits.into_inner(),
            [
                (0, 'C', 0, 0.0),
                (0, 'C', 1, 1.0),
                (0, 'J', 0, 2.0),
                (0, 'D', 0, 12.0),
                (0, 'D', 1, 112.0),
                (1, 'C', 0, 212.0),
                (1, 'C', 1, 213.0),
                (1, 'J', 0, 214.0),
            ]
        );
        assert_eq!(contacts, [(0, 2), (1, 2)]);
        assert_eq!(joints, [(0, 2)]);
        assert_eq!(drives, [(0, 1), (1, 1)]);
        assert_eq!(
            pool.solver_delta_rows[body.id().index() as usize].vectors,
            [[214.0; 4], [14.0; 4], [234.0; 4], [34.0; 4]]
        );
    }

    #[test]
    fn horus_plan_failure_after_earlier_plans_has_no_item_or_rf_effect() {
        let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
        let retained = inertia(1.0);
        let current = add(&mut pool, Some(&retained), 4);
        let stale = add(&mut pool, Some(&retained), 4);
        pool.try_remove_rigid_body(stale).expect("remove stale");
        let replacement = add(&mut pool, Some(&retained), 4);
        pool.solver_delta_rows[current.id().index() as usize].vectors = [[7.0; 4]; 4];
        pool.solver_delta_rows[replacement.id().index() as usize].vectors = [[9.0; 4]; 4];
        let rows_before = pool.solver_delta_rows.clone();
        let mut contacts = [11_u32];
        let mut joints = [22_u32];
        let mut drives = [33_u32];

        {
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let contact = phase
                .try_prepare_indexed_solver_delta_plan(
                    &mut contacts,
                    [(0, current.observation(), replacement.observation())],
                )
                .expect("earlier Contact plan");
            let joint_error = phase.try_prepare_indexed_solver_delta_plan(
                &mut joints,
                [(0, current.observation(), stale.observation())],
            );
            assert!(matches!(
                joint_error,
                Err(SimulationRigidBodyIndexedSolverDeltaError::Body {
                    ordinal: 0,
                    endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyB,
                    source: SimulationRigidBodyPoolError::StaleHandle { slot },
                }) if slot == stale.id().index()
            ));
            drop(contact);

            let contact = phase
                .try_prepare_indexed_solver_delta_plan(
                    &mut contacts,
                    [(0, current.observation(), replacement.observation())],
                )
                .expect("retry Contact plan");
            let joint = phase
                .try_prepare_indexed_solver_delta_plan(
                    &mut joints,
                    [(0, current.observation(), replacement.observation())],
                )
                .expect("Joint plan");
            let drive_error = phase.try_prepare_indexed_solver_delta_plan_with_reservation(
                &mut drives,
                [(0, replacement.observation(), current.observation())].into_iter(),
                |_, entry_count| {
                    assert_eq!(entry_count, 1);
                    Err(())
                },
            );
            assert_eq!(
                drive_error.map(|_| ()),
                Err(
                    SimulationRigidBodyIndexedSolverDeltaError::AllocationFailed { entry_count: 1 }
                )
            );
            drop((contact, joint));
        }

        assert_eq!(contacts, [11]);
        assert_eq!(joints, [22]);
        assert_eq!(drives, [33]);
        assert_eq!(pool.solver_delta_rows, rows_before);
    }

    #[test]
    fn counted_plan_ranges_reuse_one_exact_record_borrow_and_live_rf() {
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let retained = inertia(1.0);
        let body = add(&mut pool, Some(&retained), 4);
        pool.solver_delta_rows[body.id().index() as usize].vectors =
            [[0.0; 4], [10.0; 4], [20.0; 4], [30.0; 4]];
        let mut items = [(0_u8, 0_u32), (1, 0), (2, 0)];
        let visits = std::cell::RefCell::new(Vec::new());

        {
            let mut phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let mut source = [
                (0, body.observation(), body.observation()),
                (1, body.observation(), body.observation()),
                (2, body.observation(), body.observation()),
            ]
            .into_iter();
            let mut plan = phase
                .try_prepare_indexed_solver_delta_plan_counted(
                    &mut items,
                    3,
                    std::iter::from_fn(move || source.next()),
                )
                .expect("counted plan");

            phase.apply_indexed_solver_delta_plan_range(&mut plan, 2..3, |item, rows| {
                item.1 += 1;
                let (_, body_b) = rows.into_rows();
                visits.borrow_mut().push((item.0, body_b[0][0]));
                SimulationRigidBodySolverDeltaStores::new(
                    body_b[0].map(|value| value + 100.0),
                    body_b[0].map(|value| value + 100.0),
                    body_b[2].map(|value| value + 100.0),
                    body_b[2].map(|value| value + 100.0),
                )
            });
            phase.apply_indexed_contact_solver_delta_plan_range(&mut plan, 0..2, |item, rows| {
                item.1 += 1;
                let (_, body_b) = rows.into_rows();
                visits.borrow_mut().push((item.0, body_b[0][0]));
                SimulationRigidBodyContactSolverDeltaStores::new(
                    body_b[3].map(|value| value + 1.0),
                    body_b[3].map(|value| value + 1.0),
                    body_b[1].map(|value| value + 1.0),
                    body_b[1].map(|value| value + 1.0),
                    body_b[2].map(|value| value + 1.0),
                    body_b[2].map(|value| value + 1.0),
                    body_b[0].map(|value| value + 1.0),
                    body_b[0].map(|value| value + 1.0),
                )
            });
        }

        assert_eq!(items, [(0, 1), (1, 1), (2, 1)]);
        assert_eq!(visits.into_inner(), [(2, 0.0), (0, 100.0), (1, 101.0)]);
        assert_eq!(
            pool.solver_delta_rows[body.id().index() as usize].vectors,
            [[102.0; 4], [12.0; 4], [122.0; 4], [32.0; 4]]
        );
    }

    #[test]
    fn counted_plan_rejects_short_trailing_and_late_stale_sequences_before_effects() {
        let mut pool = SimulationRigidBodyPool::try_new(2).expect("pool");
        let retained = inertia(1.0);
        let current = add(&mut pool, Some(&retained), 4);
        let stale = add(&mut pool, Some(&retained), 4);
        pool.try_remove_rigid_body(stale).expect("remove stale");
        let replacement = add(&mut pool, Some(&retained), 4);
        pool.solver_delta_rows[current.id().index() as usize].vectors = [[7.0; 4]; 4];
        pool.solver_delta_rows[replacement.id().index() as usize].vectors = [[9.0; 4]; 4];
        let rows_before = pool.solver_delta_rows.clone();
        let mut items = [11_u32, 22];

        {
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let short = phase.try_prepare_indexed_solver_delta_plan_counted(
                &mut items,
                2,
                [(0, current.observation(), replacement.observation())],
            );
            assert_eq!(
                short.map(|_| ()),
                Err(
                    SimulationRigidBodyIndexedSolverDeltaError::SequenceEndedEarly {
                        expected: 2,
                        actual: 1,
                    }
                )
            );

            let trailing = phase.try_prepare_indexed_solver_delta_plan_counted(
                &mut items,
                1,
                [
                    (0, current.observation(), replacement.observation()),
                    (1, replacement.observation(), current.observation()),
                ],
            );
            assert_eq!(
                trailing.map(|_| ()),
                Err(
                    SimulationRigidBodyIndexedSolverDeltaError::SequenceHasTrailingEntry {
                        expected: 1,
                    }
                )
            );

            let stale_error = phase.try_prepare_indexed_solver_delta_plan_counted(
                &mut items,
                2,
                [
                    (0, current.observation(), replacement.observation()),
                    (1, current.observation(), stale.observation()),
                ],
            );
            assert!(matches!(
                stale_error,
                Err(SimulationRigidBodyIndexedSolverDeltaError::Body {
                    ordinal: 1,
                    endpoint: SimulationRigidBodySolverDeltaEndpoint::BodyB,
                    source: SimulationRigidBodyPoolError::StaleHandle { slot },
                }) if slot == stale.id().index()
            ));
        }

        assert_eq!(items, [11, 22]);
        assert_eq!(pool.solver_delta_rows, rows_before);
    }

    #[test]
    fn counted_plan_reserves_the_complete_flattened_count_before_iterator_reads() {
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let retained = inertia(1.0);
        let body = add(&mut pool, Some(&retained), 4);
        let rows_before = pool.solver_delta_rows.clone();
        let mut items = [17_u32];
        let reads = std::cell::Cell::new(0_usize);

        {
            let phase = pool.try_prepare_dynamic_update_phase().expect("phase");
            let source = std::iter::from_fn(|| {
                reads.set(reads.get() + 1);
                Some((0, body.observation(), body.observation()))
            });
            let result = phase.try_prepare_indexed_solver_delta_plan_counted_with_reservation(
                &mut items,
                9,
                source,
                |_, entry_count| {
                    assert_eq!(entry_count, 9);
                    Err(())
                },
            );
            assert_eq!(
                result.map(|_| ()),
                Err(
                    SimulationRigidBodyIndexedSolverDeltaError::AllocationFailed { entry_count: 9 }
                )
            );
        }

        assert_eq!(reads.get(), 0);
        assert_eq!(items, [17]);
        assert_eq!(pool.solver_delta_rows, rows_before);
    }

    #[test]
    fn counted_plan_range_rejects_replacement_phase_before_empty_range_callback() {
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let retained = inertia(1.0);
        let body = add(&mut pool, Some(&retained), 4);
        let mut items = [0_u32];
        let first_phase = pool
            .try_prepare_dynamic_update_phase()
            .expect("first phase");
        let mut plan = first_phase
            .try_prepare_indexed_solver_delta_plan_counted(
                &mut items,
                1,
                [(0, body.observation(), body.observation())],
            )
            .expect("plan");
        drop(first_phase);

        let mut replacement_phase = pool
            .try_prepare_dynamic_update_phase()
            .expect("replacement phase");
        let called = std::cell::Cell::new(false);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            replacement_phase.apply_indexed_solver_delta_plan_range(&mut plan, 0..0, |_, _| {
                called.set(true);
                SimulationRigidBodySolverDeltaStores::new([0.0; 4], [0.0; 4], [0.0; 4], [0.0; 4])
            });
        }));
        assert!(panic.is_err());
        assert!(!called.get());
    }

    #[test]
    fn dynamic_update_phase_reports_the_preflighted_active_ring_state() {
        let mut pool = SimulationRigidBodyPool::try_new(1).expect("pool");
        let empty = pool
            .try_prepare_dynamic_update_phase()
            .expect("empty phase");
        assert!(!empty.has_active_bodies());
        drop(empty);

        let retained = inertia(1.0);
        let _active = add(&mut pool, Some(&retained), 4);
        let active = pool
            .try_prepare_dynamic_update_phase()
            .expect("active phase");
        assert!(active.has_active_bodies());
    }

    #[test]
    fn local_authority_allocator_is_checked_and_never_wraps() {
        let source = AtomicU64::new(u64::MAX - 1);
        assert_eq!(
            fresh_pool_authority(&source),
            Ok(SimulationRigidBodyPoolAuthority(u64::MAX - 1))
        );
        assert_eq!(
            fresh_pool_authority(&source),
            Err(SimulationRigidBodyPoolError::AuthorityExhausted)
        );
        assert_eq!(source.load(Ordering::Relaxed), u64::MAX);
    }
}
