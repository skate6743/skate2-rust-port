//! Concrete safe owner for the three recovered `rw::physics::Simulation` pools.
//!
//! [`DynamicAssemblySession`](crate::dynamic_assembly_session::DynamicAssemblySession)
//! owns the proven Assembly transaction order. This module supplies only its
//! body, Joint, and Drive primitives by composing [`SimulationRigidBodyPool`],
//! [`SimulationJointPool`], and [`SimulationDrivePool`]. It does not add a
//! solver, integration, constraint, or collision response model.
//!
//! The backend's identity is its process-unique body-pool authority. Joint and
//! Drive pools cannot be replaced or independently exposed for mutation, so all
//! handles retained by a session remain in one ownership domain. Native
//! nonowning payload pointers are represented by retained [`Arc`] owners:
//! immutable or checked mutable owners for inertia and Joint limits, immutable
//! Joint frames, and `Arc<RwLock<_>>` for live Drive inputs. Those choices are explicit
//! safe-host policy.
//!
//! The existing backend trait has no typed error channel. A failure-atomic pool
//! error during an add is therefore returned as `None` and retained in
//! [`SimulationAssemblyBackend::last_add_failure`]; native-shaped capacity
//! exhaustion is `None` without a diagnostic. Pose and removal rely on the
//! trait's live, same-backend, exactly-once handle precondition. Violating that
//! precondition through direct trait calls panics rather than silently skipping
//! a required transaction step; `DynamicAssemblySession` keeps its handles
//! private and maintains the precondition.

#[cfg(feature = "skate3-runtime")]
#[path = "skate3_player_backend.rs"]
mod skate3_player_backend;

#[cfg(test)]
use crate::simulation_rigid_body_pool::SimulationInertia;
use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyPartAttachmentMetadata, DynamicAssemblyPartAttachmentTemplate,
        DynamicAssemblyPartAttachmentTemplateError, DynamicAssemblyPartPayload,
        DynamicAssemblySimulationBackend,
    },
    serialised_assembly_template::{SerialisedAssemblyRuntimeInertia, SerialisedAssemblyTemplate},
    simulation_assembly_payload::{SimulationAssemblyInertia, SimulationAssemblyJointLimits},
    simulation_drive_pool::{
        SimulationDriveDynamics, SimulationDriveFrames, SimulationDriveHandle, SimulationDrivePool,
        SimulationDrivePoolError, SimulationDrivePoolSnapshot,
    },
    simulation_joint_pool::{
        SimulationJointHandle, SimulationJointPool, SimulationJointPoolError,
        SimulationJointPoolSnapshot,
    },
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_rigid_body_pool::{
        SimulationRigidBodyHandle, SimulationRigidBodyPool, SimulationRigidBodyPoolAuthority,
        SimulationRigidBodyPoolError, SimulationRigidBodyPoolSnapshot,
    },
    simulation_xenon_address_image::{
        SimulationArenaXenonAddressBinding, SimulationPackedBodyWordImageDomain,
        SimulationPackedBodyWordImageDomainError, SimulationRigidBodyAddInertiaInput,
        SimulationRigidBodyAddInertiaOwner, SimulationRigidBodyAddInertiaOwnerError,
        SimulationXenonAddressImage,
    },
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{
    error::Error,
    fmt,
    marker::PhantomData,
    num::NonZeroU64,
    sync::{Arc, RwLock},
};

#[path = "simulation_assembly_constraint_session.rs"]
pub(crate) mod constraint_session;
#[path = "simulation_assembly_deck_torque.rs"]
mod deck_torque;

#[path = "../solver/simulation_skeleton_normal_backend.rs"]
mod skeleton_normal;

#[path = "simulation_assembly_source_backend.rs"]
pub(crate) mod source;
pub use source::SimulationAssemblySourceAddError;

/// Which semantic Joint endpoint failed safe-host body validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationAssemblyJointEndpoint {
    Parent,
    Child,
}

/// A non-capacity add refusal compressed to `None` by the Assembly backend trait.
///
/// Pool methods are failure-atomic, so recording this diagnostic does not leave
/// a partially added object. Ordinary native-shaped capacity exhaustion is
/// represented by `None` with no diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationAssemblyBackendAddFailure {
    RigidBody(SimulationRigidBodyPoolError),
    SourceRigidBody(SimulationAssemblySourceAddError),
    JointBody {
        endpoint: SimulationAssemblyJointEndpoint,
        source: SimulationRigidBodyPoolError,
    },
    Joint(SimulationJointPoolError),
    Drive(SimulationDrivePoolError),
}

impl fmt::Display for SimulationAssemblyBackendAddFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RigidBody(source) => write!(formatter, "rigid-body add failed: {source}"),
            Self::SourceRigidBody(source) => {
                write!(formatter, "source rigid-body add failed: {source}")
            }
            Self::JointBody { endpoint, source } => {
                write!(
                    formatter,
                    "Joint {endpoint:?} body validation failed: {source}"
                )
            }
            Self::Joint(source) => write!(formatter, "Joint add failed: {source}"),
            Self::Drive(source) => write!(formatter, "Drive add failed: {source}"),
        }
    }
}

impl Error for SimulationAssemblyBackendAddFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RigidBody(source) | Self::JointBody { source, .. } => Some(source),
            Self::SourceRigidBody(source) => Some(source),
            Self::Joint(source) => Some(source),
            Self::Drive(source) => Some(source),
        }
    }
}

/// Failure to construct one complete preallocated Simulation pool owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationAssemblyBackendInitError {
    RigidBodies(SimulationRigidBodyPoolError),
    Joints(SimulationJointPoolError),
    Drives(SimulationDrivePoolError),
}

impl fmt::Display for SimulationAssemblyBackendInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RigidBodies(source) => {
                write!(formatter, "could not initialize rigid-body pool: {source}")
            }
            Self::Joints(source) => write!(formatter, "could not initialize Joint pool: {source}"),
            Self::Drives(source) => write!(formatter, "could not initialize Drive pool: {source}"),
        }
    }
}

impl Error for SimulationAssemblyBackendInitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RigidBodies(source) => Some(source),
            Self::Joints(source) => Some(source),
            Self::Drives(source) => Some(source),
        }
    }
}

/// Construction refusal for the staged, source-aware Assembly body backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SerialisedAssemblySimulationBackendInitError {
    RigidBodies(SimulationRigidBodyPoolError),
    ImageDomain(SimulationPackedBodyWordImageDomainError),
    RouteStorageAllocation {
        requested: usize,
    },
    Route {
        ordinal: usize,
        source: SimulationRigidBodyAddInertiaOwnerError,
    },
    DuplicateBindingEpoch {
        first: usize,
        second: usize,
        epoch: NonZeroU64,
    },
    DuplicateArenaOwner {
        first: usize,
        second: usize,
    },
    BindingOwnerMismatch {
        ordinal: usize,
    },
}

impl fmt::Display for SerialisedAssemblySimulationBackendInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RigidBodies(source) => {
                write!(formatter, "could not initialize rigid-body pool: {source}")
            }
            Self::ImageDomain(source) => {
                write!(
                    formatter,
                    "could not bind Simulation image domain: {source}"
                )
            }
            Self::RouteStorageAllocation { requested } => {
                write!(formatter, "could not reserve {requested} arena routes")
            }
            Self::Route { ordinal, source } => {
                write!(
                    formatter,
                    "could not initialize arena route {ordinal}: {source}"
                )
            }
            Self::DuplicateBindingEpoch {
                first,
                second,
                epoch,
            } => write!(
                formatter,
                "arena routes {first} and {second} repeat binding epoch {epoch}"
            ),
            Self::DuplicateArenaOwner { first, second } => write!(
                formatter,
                "arena routes {first} and {second} refer to the same exact arena owner"
            ),
            Self::BindingOwnerMismatch { ordinal } => write!(
                formatter,
                "arena route {ordinal} binding belongs to a different arena owner"
            ),
        }
    }
}

impl Error for SerialisedAssemblySimulationBackendInitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RigidBodies(source) => Some(source),
            Self::ImageDomain(source) => Some(source),
            Self::Route { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Source-owner refusal before a Part payload is admitted to a body transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SerialisedAssemblySimulationBindError {
    ArenaNotRegistered,
}

impl fmt::Display for SerialisedAssemblySimulationBindError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArenaNotRegistered => {
                formatter.write_str("Definition Inertia arena owner is not registered")
            }
        }
    }
}

impl Error for SerialisedAssemblySimulationBindError {}

/// Safe-host refusal while sealing one source-aware Definition-order Part batch.
///
/// This validation chronology is host policy, not a recovered native error
/// surface: Joint mismatch, Drive mismatch, matched nonzero Joints, matched
/// nonzero Drives, both storage reservations, then route binding by ordinal.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the Definition-backed DynamicAssembly runtime-template bridge"
    )
)]
pub(crate) enum SerialisedAssemblyPartAttachmentBindError {
    JointCountMismatch {
        assembly: u32,
        definition: u32,
    },
    DriveCountMismatch {
        assembly: u32,
        definition: u32,
    },
    UnsupportedJoints {
        count: u32,
    },
    UnsupportedDrives {
        count: u32,
    },
    StorageAllocation {
        requested: usize,
    },
    Route {
        ordinal: usize,
        source: SerialisedAssemblySimulationBindError,
    },
    AttachmentTemplate(DynamicAssemblyPartAttachmentTemplateError),
}

impl fmt::Display for SerialisedAssemblyPartAttachmentBindError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JointCountMismatch {
                assembly,
                definition,
            } => write!(
                formatter,
                "SerialisedAssembly has {assembly} Joints but its Definition has {definition}"
            ),
            Self::DriveCountMismatch {
                assembly,
                definition,
            } => write!(
                formatter,
                "SerialisedAssembly has {assembly} Drives but its Definition has {definition}"
            ),
            Self::UnsupportedJoints { count } => {
                write!(
                    formatter,
                    "source-aware Part binding does not support {count} Joints"
                )
            }
            Self::UnsupportedDrives { count } => {
                write!(
                    formatter,
                    "source-aware Part binding does not support {count} Drives"
                )
            }
            Self::StorageAllocation { requested } => {
                write!(
                    formatter,
                    "could not reserve {requested} source-aware Part records"
                )
            }
            Self::Route { ordinal, source } => {
                write!(
                    formatter,
                    "could not bind Part {ordinal} source route: {source}"
                )
            }
            Self::AttachmentTemplate(source) => {
                write!(
                    formatter,
                    "could not seal source-aware Part batch: {source}"
                )
            }
        }
    }
}

impl Error for SerialisedAssemblyPartAttachmentBindError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Route { source, .. } => Some(source),
            Self::AttachmentTemplate(source) => Some(source),
            _ => None,
        }
    }
}

/// Non-capacity refusal compressed to `None` by the generic Assembly backend trait.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "reserved for the Definition-backed DynamicAssembly production bridge"
    )
)]
pub(crate) enum SerialisedAssemblySimulationAddFailure {
    ForeignRoute,
    MissingRoute,
    RigidBody(SimulationRigidBodyAddInertiaOwnerError),
    UnsupportedJoint,
    UnsupportedDrive,
}

/// Private, nonforgeable route identity retained by both Part input and body handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SerialisedAssemblyArenaRouteToken {
    pool: SimulationRigidBodyPoolAuthority,
    binding_epoch: NonZeroU64,
}

/// Definition Inertia prebound to exactly one registered arena outcome owner.
#[derive(Clone, Debug)]
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "reserved for the Definition-backed DynamicAssembly production bridge"
    )
)]
pub(crate) struct SerialisedAssemblyRoutedInertia {
    route: SerialisedAssemblyArenaRouteToken,
    inertia: SerialisedAssemblyRuntimeInertia,
}

/// Body capability retaining the exact route which published its Add outcome.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "reserved for the Definition-backed DynamicAssembly production bridge"
    )
)]
pub(crate) struct SerialisedAssemblyRoutedBodyHandle {
    route: SerialisedAssemblyArenaRouteToken,
    body: SimulationRigidBodyHandle,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "reserved for the Definition-backed DynamicAssembly production bridge"
    )
)]
pub(crate) enum UnsupportedSerialisedAssemblyHandle {}

struct SerialisedAssemblyArenaRoute {
    token: SerialisedAssemblyArenaRouteToken,
    owner: Arc<SimulationNormalArenaPlan>,
    inertia: SimulationRigidBodyAddInertiaOwner,
}

/// Exclusive body-pool owner for Definition-backed `SerialisedAssembly` Parts.
///
/// This body-only checkpoint accepts already typed Simulation-image and arena
/// bindings. It never derives an address or claims the allocator leases behind
/// those values. Routes are immutable for the owner's lifetime, so a prebound
/// route token cannot be detached or reused while a body occurrence survives.
/// Joint and Drive adds are explicitly refused until their source-aware
/// production payload bridge exists.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "reserved for the Definition-backed DynamicAssembly production bridge"
    )
)]
pub(crate) struct SerialisedAssemblySimulationBackend<M> {
    rigid_bodies: SimulationRigidBodyPool,
    image_domain: SimulationPackedBodyWordImageDomain,
    routes: Box<[SerialisedAssemblyArenaRoute]>,
    last_add_failure: Option<SerialisedAssemblySimulationAddFailure>,
    material_type: PhantomData<fn() -> M>,
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "reserved for the Definition-backed DynamicAssembly production bridge"
    )
)]
impl<M> SerialisedAssemblySimulationBackend<M> {
    /// Builds one pool-wide image domain and one preallocated outcome table per
    /// caller-supplied typed arena binding before publishing this owner.
    pub(crate) fn try_new(
        rigid_body_capacity: usize,
        image: SimulationXenonAddressImage,
        arena_bindings: Vec<(
            Arc<SimulationNormalArenaPlan>,
            Arc<SimulationArenaXenonAddressBinding>,
        )>,
    ) -> Result<Self, SerialisedAssemblySimulationBackendInitError> {
        let rigid_bodies = SimulationRigidBodyPool::try_new(rigid_body_capacity)
            .map_err(SerialisedAssemblySimulationBackendInitError::RigidBodies)?;
        let image_domain = SimulationPackedBodyWordImageDomain::try_new(&rigid_bodies, image)
            .map_err(SerialisedAssemblySimulationBackendInitError::ImageDomain)?;
        let routes = source::try_build_routes(&rigid_bodies, arena_bindings)?;
        Ok(Self {
            rigid_bodies,
            image_domain,
            routes,
            last_add_failure: None,
            material_type: PhantomData,
        })
    }

    /// Prebinds one immutable Definition payload by exact arena-owner identity.
    ///
    /// This method has no pool or route-table effects. Two independently owned,
    /// byte-equal arena plans remain distinct. Construction has already
    /// rejected multiple bindings for the same exact owner.
    fn try_bind_inertia(
        &self,
        inertia: &SerialisedAssemblyRuntimeInertia,
    ) -> Result<SerialisedAssemblyRoutedInertia, SerialisedAssemblySimulationBindError> {
        debug_assert!(self.image_domain.is_same_pool(&self.rigid_bodies));
        let owner = inertia.source().owner();
        let ordinal = self
            .routes
            .iter()
            .position(|route| route.inertia.is_same_arena_owner(owner))
            .ok_or(SerialisedAssemblySimulationBindError::ArenaNotRegistered)?;
        Ok(SerialisedAssemblyRoutedInertia {
            route: self.routes[ordinal].token,
            inertia: inertia.clone(),
        })
    }

    pub(crate) const fn last_add_failure(&self) -> Option<&SerialisedAssemblySimulationAddFailure> {
        self.last_add_failure.as_ref()
    }

    fn route_index(&self, token: SerialisedAssemblyArenaRouteToken) -> Option<usize> {
        (token.pool == self.rigid_bodies.authority())
            .then(|| self.routes.iter().position(|route| route.token == token))?
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the Definition-backed DynamicAssembly runtime-template bridge"
    )
)]
impl SerialisedAssemblySimulationBackend<AssemblyPartMaterial> {
    /// Seals one backend-state-effect-free, source-aware Part batch in
    /// template-preserved Definition order.
    ///
    /// Exact arena-owner route lookup is safe-host policy and runs for every
    /// Part, including Static. This does not validate the routed Xenon
    /// Definition-Inertia address; that validation remains at body Add, where
    /// native Static selection does not consume the address. The returned batch
    /// retains routed Inertia owners and copied Part values, but not the selected
    /// DMO asset/template and does not create a live Assembly session or body.
    pub(crate) fn try_bind_part_attachment_template(
        &self,
        template: &SerialisedAssemblyTemplate,
    ) -> Result<
        Arc<
            DynamicAssemblyPartAttachmentTemplate<
                SerialisedAssemblyRoutedInertia,
                AssemblyPartMaterial,
            >,
        >,
        SerialisedAssemblyPartAttachmentBindError,
    > {
        source::try_bind_attachment(template, |inertia| self.try_bind_inertia(inertia))
    }
}

impl<M> DynamicAssemblySimulationBackend for SerialisedAssemblySimulationBackend<M> {
    type Inertia = SerialisedAssemblyRoutedInertia;
    type Material = M;
    type JointFrames = ();
    type JointLimits = ();
    type DriveFrames = ();
    type DriveDynamics = ();
    type Authority = SimulationRigidBodyPoolAuthority;
    type RigidBodyHandle = SerialisedAssemblyRoutedBodyHandle;
    type JointHandle = UnsupportedSerialisedAssemblyHandle;
    type DriveHandle = UnsupportedSerialisedAssemblyHandle;

    fn authority(&self) -> Self::Authority {
        self.rigid_bodies.authority()
    }

    fn add_rigid_body(
        &mut self,
        transform: &[f32; 16],
        inertia: &Self::Inertia,
        body_state: u32,
    ) -> Option<Self::RigidBodyHandle> {
        self.last_add_failure = None;
        if inertia.route.pool != self.rigid_bodies.authority() {
            self.last_add_failure = Some(SerialisedAssemblySimulationAddFailure::ForeignRoute);
            return None;
        }
        let Some(route_index) = self.route_index(inertia.route) else {
            self.last_add_failure = Some(SerialisedAssemblySimulationAddFailure::MissingRoute);
            return None;
        };
        let route = &mut self.routes[route_index];
        match route.inertia.try_add(
            &mut self.rigid_bodies,
            *transform,
            SimulationRigidBodyAddInertiaInput::Definition(&inertia.inertia),
            body_state,
        ) {
            Ok(Some(body)) => Some(SerialisedAssemblyRoutedBodyHandle {
                route: inertia.route,
                body,
            }),
            Ok(None) => None,
            Err(source) => {
                self.last_add_failure =
                    Some(SerialisedAssemblySimulationAddFailure::RigidBody(source));
                None
            }
        }
    }

    fn rigid_body_pose(&self, body: &Self::RigidBodyHandle) -> [f32; 16] {
        assert_eq!(
            self.route_index(body.route)
                .map(|index| self.routes[index].token),
            Some(body.route),
            "session-owned rigid body retains a live source route"
        );
        self.rigid_bodies
            .try_rigid_body_pose(body.body)
            .expect("session-owned rigid-body handle remains live through pose copyback")
    }

    fn remove_rigid_body(&mut self, body: &Self::RigidBodyHandle) {
        let route_index = self
            .route_index(body.route)
            .expect("session-owned rigid body retains a live source route");
        self.routes[route_index]
            .inertia
            .try_remove(&mut self.rigid_bodies, body.body)
            .expect("session-owned rigid-body occurrence and outcome are removed exactly once");
    }

    fn add_joint(
        &mut self,
        _parent: &Self::RigidBodyHandle,
        _child: &Self::RigidBodyHandle,
        _frames: &Self::JointFrames,
        _limits: &Self::JointLimits,
    ) -> Option<Self::JointHandle> {
        self.last_add_failure = Some(SerialisedAssemblySimulationAddFailure::UnsupportedJoint);
        None
    }

    fn remove_joint(&mut self, joint: &Self::JointHandle) {
        match *joint {}
    }

    fn add_drive(
        &mut self,
        _parent: &Self::RigidBodyHandle,
        _child: &Self::RigidBodyHandle,
        _frames: &Self::DriveFrames,
        _dynamics: &Self::DriveDynamics,
    ) -> Option<Self::DriveHandle> {
        self.last_add_failure = Some(SerialisedAssemblySimulationAddFailure::UnsupportedDrive);
        None
    }

    fn remove_drive(&mut self, drive: &Self::DriveHandle) {
        match *drive {}
    }
}

/// Concrete owner joining the recovered body, Joint, and Drive pools.
///
/// `M` is retained by the session template rather than the Simulation pools.
/// `JF` and `JL` are Joint frame/limit values retained in the Joint pool; limits
/// may use checked mutable storage. All lifecycle mutation stays through this
/// owner and the private handles held by `DynamicAssemblySession`.
#[derive(Debug)]
pub struct SimulationAssemblyBackend<M, JF, JL> {
    rigid_bodies: SimulationRigidBodyPool,
    joints: SimulationJointPool<JF, JL>,
    drives: SimulationDrivePool,
    source_routes: Option<source::SimulationAssemblySourceRoutes>,
    last_add_failure: Option<SimulationAssemblyBackendAddFailure>,
    material_type: PhantomData<fn() -> M>,
}

impl<M, JF: Copy, JL: Copy> SimulationAssemblyBackend<M, JF, JL> {
    /// Allocates and publishes all three pools as one authority domain.
    ///
    /// # Errors
    ///
    /// Returns the first typed pool-initialization error. Any earlier pool is
    /// still local and contains no live object or externally issued handle.
    pub fn try_new(
        rigid_body_capacity: usize,
        joint_capacity: usize,
        drive_capacity: usize,
    ) -> Result<Self, SimulationAssemblyBackendInitError> {
        let rigid_bodies = SimulationRigidBodyPool::try_new(rigid_body_capacity)
            .map_err(SimulationAssemblyBackendInitError::RigidBodies)?;
        let joints = SimulationJointPool::try_new(joint_capacity)
            .map_err(SimulationAssemblyBackendInitError::Joints)?;
        let drives = SimulationDrivePool::try_new(drive_capacity, &rigid_bodies)
            .map_err(SimulationAssemblyBackendInitError::Drives)?;
        Ok(Self {
            rigid_bodies,
            joints,
            drives,
            source_routes: None,
            last_add_failure: None,
            material_type: PhantomData,
        })
    }

    /// Process-unique authority used by attached Assembly sessions.
    #[must_use]
    pub const fn authority(&self) -> SimulationRigidBodyPoolAuthority {
        self.rigid_bodies.authority()
    }

    /// Last non-capacity, failure-atomic add refusal, if any.
    ///
    /// A subsequent add attempt clears the previous diagnostic before it starts.
    #[must_use]
    pub const fn last_add_failure(&self) -> Option<&SimulationAssemblyBackendAddFailure> {
        self.last_add_failure.as_ref()
    }

    /// Clears and returns the last non-capacity add diagnostic.
    pub fn take_last_add_failure(&mut self) -> Option<SimulationAssemblyBackendAddFailure> {
        self.last_add_failure.take()
    }

    #[must_use]
    pub fn rigid_body_snapshot(&self) -> SimulationRigidBodyPoolSnapshot {
        self.rigid_bodies.snapshot()
    }

    #[must_use]
    pub fn joint_snapshot(&self) -> SimulationJointPoolSnapshot {
        self.joints.snapshot()
    }

    #[must_use]
    pub fn drive_snapshot(&self) -> SimulationDrivePoolSnapshot {
        self.drives.snapshot()
    }

    /// Test observation reads the actual retained pool inputs, never the
    /// constructor's own payload copies. This is not a Jacobian/solver tick.
    #[cfg(test)]
    pub(crate) fn active_drive_payloads_for_test(
        &self,
    ) -> Result<Vec<(SimulationDriveFrames, SimulationDriveDynamics)>, SimulationDrivePoolError>
    {
        self.drives
            .try_active_build_order(&self.rigid_bodies)?
            .into_iter()
            .map(|identity| {
                Ok((
                    self.drives.try_frames(identity)?,
                    self.drives.try_dynamics(identity)?,
                ))
            })
            .collect()
    }

    pub fn set_gravity(&mut self, gravity: [f32; 3]) {
        self.rigid_bodies.set_gravity(gravity);
    }

    #[cfg(test)]
    pub(crate) fn try_manager_for_test(&mut self, maximum_contacts: u32)
        -> Result<crate::simulation_contact_jacobian::network_preparation::SimulationManagerContactDomain,
    crate::simulation_contact_jacobian::network_preparation::SimulationManagerContactDomainInitError>{
        crate::simulation_contact_jacobian::network_preparation::SimulationManagerContactDomain::try_new(
            &mut self.rigid_bodies, maximum_contacts)
    }

    #[cfg(test)]
    pub(crate) fn rigid_body_pool_for_test(&self) -> &SimulationRigidBodyPool {
        &self.rigid_bodies
    }

    pub fn set_cooldown(&mut self, cooldown: u32) {
        self.rigid_bodies.set_cooldown(cooldown);
    }

    /// Installs the body-integration timestep and derived frequency for the
    /// current frame.
    pub fn begin_dynamic_update_frame(&mut self, timestep: f32) {
        self.rigid_bodies.begin_dynamic_update_frame(timestep);
    }

    /// See `SimulationRigidBodyPool::ensure_publication_frame`.
    pub fn ensure_publication_frame(&mut self) {
        self.rigid_bodies.ensure_publication_frame();
    }

    pub fn set_minimum_energy(&mut self, minimum_energy: f32) {
        self.rigid_bodies.set_minimum_energy(minimum_energy);
    }

    /// Runs only the recovered ordinary-finite Active-body integration seam.
    ///
    /// This does not build Jacobians, invoke Horus, or constitute a complete
    /// constrained Simulation tick.
    ///
    /// # Errors
    ///
    /// Returns the body pool's failure-atomic structural, Active-plan
    /// allocation, missing-Inertia, or frame-identity-exhaustion diagnostic
    /// before any body or solver-row mutation.
    pub fn try_integrate_active_bodies_ordinary_finite(
        &mut self,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies
            .try_integrate_active_bodies_ordinary_finite()
    }

    /// Session-owned attached Part update, without exposing mutable pool access.
    pub(crate) fn try_set_part_body_transform_host(
        &mut self,
        body: SimulationRigidBodyHandle,
        transform: [f32; 16],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies
            .try_set_part_body_transform_host(body, transform)
    }

    pub(crate) fn try_reset_part_body_motion_host(
        &mut self,
        body: SimulationRigidBodyHandle,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies.try_reset_part_body_motion_host(body)
    }

    pub(crate) fn try_apply_part_body_displacement<
        N: crate::reckoning_numeric::ReckoningNumeric,
    >(
        &mut self,
        body: SimulationRigidBodyHandle,
        displacement: [f32; 4],
        numeric: &mut N,
    ) -> Result<(), crate::simulation_rigid_body_pool::SimulationRigidBodyDisplacementError<N::Error>>
    {
        self.rigid_bodies
            .try_apply_part_body_displacement(body, displacement, numeric)
    }

    pub(crate) fn try_apply_skateboard_queued_force(
        &mut self,
        body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        force: crate::skeleton_root_transforms::SkeletonRootVector,
        local_position: crate::skeleton_root_transforms::SkeletonRootVector,
        local_y_offset: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies.try_apply_skateboard_queued_force(
            body,
            force,
            local_position,
            local_y_offset,
        )
    }

    pub(crate) fn try_set_part_group_linear_drag(
        &mut self,
        bodies: &[crate::simulation_rigid_body_pool::SimulationRigidBodyObservation],
        scaled_drag: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies
            .try_set_part_group_linear_drag(bodies, scaled_drag)
    }

    pub(crate) fn try_add_controller_drive(
        &mut self,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<Option<SimulationDriveHandle>, SimulationDrivePoolError> {
        self.drives
            .try_add_drive(&self.rigid_bodies, parent, child, frames, dynamics)
    }

    pub(crate) fn try_remove_controller_drive(
        &mut self,
        handle: &SimulationDriveHandle,
    ) -> Result<(), SimulationDrivePoolError> {
        self.drives.try_remove_drive(*handle)
    }

    pub(crate) fn try_set_part_linear_velocity(
        &mut self,
        body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        velocity: [f32; 3],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies
            .try_set_part_linear_velocity(body, velocity)
    }

    pub(crate) fn try_store_post_solve_linear_velocity(
        &mut self,
        body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        velocity: [f32; 3],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies
            .try_store_post_solve_linear_velocity(body, velocity)
    }

    pub(crate) fn try_publish_living_world_part_drive(
        &mut self,
        body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        velocity: [f32; 3],
        omega: [f32; 3],
        ignore_gravity: bool,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies
            .try_publish_living_world_part_drive(body, velocity, omega, ignore_gravity)
    }

    pub(crate) fn try_write_living_world_definition_inertia(
        &mut self,
        owner: &crate::simulation_rigid_body_pool::SimulationMutableInertia,
        value: crate::simulation_rigid_body_pool::SimulationInertia,
        w: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies
            .try_write_living_world_definition_inertia(owner, value, w)
    }

    pub(crate) fn try_set_part_angular_drag(
        &mut self,
        body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        scaled_drag: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies
            .try_set_part_angular_drag(body, scaled_drag)
    }

    pub(crate) fn try_set_part_angular_velocity(
        &mut self,
        body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        omega: [f32; 3],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies.try_set_part_angular_velocity(body, omega)
    }

    pub(crate) fn try_store_post_solve_angular_velocity(
        &mut self,
        body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        omega: [f32; 3],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies.try_store_post_solve_angular_velocity(body, omega)
    }

    pub(crate) fn try_capture_part_inertia(
        &self,
        body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
    ) -> Result<
        crate::simulation_rigid_body_pool::SimulationMutableInertia,
        SimulationRigidBodyPoolError,
    > {
        self.rigid_bodies.try_capture_part_inertia(body)
    }

    pub(crate) fn try_set_captured_angular_drag(
        &mut self,
        body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        inertia: &crate::simulation_rigid_body_pool::SimulationMutableInertia,
        value: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies
            .try_set_captured_angular_drag(body, inertia, value)
    }

    pub(crate) fn try_set_owned_drive_spy(
        &mut self,
        drive: SimulationDriveHandle,
        spy: u32,
    ) -> Result<(), SimulationDrivePoolError> {
        self.drives.try_set_spy(drive, spy)
    }

    /// Session-owned no-lock validation. A nullable source Add result is not
    /// promoted to a Drive, and no pool/endpoint mutation capability escapes.
    pub(crate) fn try_validate_owned_drive_payloads(
        &self,
        drive: Option<SimulationDriveHandle>,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, SimulationDrivePoolError> {
        self.drives.try_validate_owned_payloads(
            &self.rigid_bodies,
            drive,
            parent,
            child,
            frames,
            dynamics,
        )
    }

    pub(crate) fn try_part_body_pose(
        &self,
        body: SimulationRigidBodyHandle,
    ) -> Result<[f32; 16], SimulationRigidBodyPoolError> {
        self.rigid_bodies.try_rigid_body_pose(body)
    }

    /// Actual Part-owned capability validation; no writable handle is returned.
    pub(crate) fn try_owned_body_observation(
        &self,
        body: SimulationRigidBodyHandle,
    ) -> Result<
        crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        SimulationRigidBodyPoolError,
    > {
        self.rigid_bodies
            .try_validate_observation(body.observation())?;
        Ok(body.observation())
    }

    /// Island `RegisterSpyTarget`'s actual state-store prefix, before the caller
    /// publishes its Body/Part-tag target mapping (0x827b7874/0x827b7880).
    pub(crate) fn try_register_owned_body_contact_spy(
        &mut self,
        body: SimulationRigidBodyHandle,
    ) -> Result<
        crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        SimulationRigidBodyPoolError,
    > {
        self.rigid_bodies.try_register_contact_spy(body)?;
        Ok(body.observation())
    }

    /// Actual state &=7 prefix, before target-map erasure (0x827b5660).
    pub(crate) fn try_remove_owned_body_contact_spy(
        &mut self,
        body: SimulationRigidBodyHandle,
    ) -> Result<
        crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        SimulationRigidBodyPoolError,
    > {
        self.rigid_bodies.try_remove_contact_spy(body)?;
        Ok(body.observation())
    }

    /// Immutable occurrence-checked source access for authored collision
    /// publication. No mutable body pool or fabricated packed image escapes.
    pub(crate) fn collision_body_pool(&self) -> &SimulationRigidBodyPool {
        &self.rigid_bodies
    }

    fn record_failure(&mut self, failure: SimulationAssemblyBackendAddFailure) {
        self.last_add_failure = Some(failure);
    }
}

impl<M, JF: Copy, JL: Copy> DynamicAssemblySimulationBackend
    for SimulationAssemblyBackend<M, JF, JL>
{
    type Inertia = SimulationAssemblyInertia;
    type Material = M;
    type JointFrames = Arc<JF>;
    type JointLimits = SimulationAssemblyJointLimits<JL>;
    type DriveFrames = Arc<RwLock<SimulationDriveFrames>>;
    type DriveDynamics = Arc<RwLock<SimulationDriveDynamics>>;
    type Authority = SimulationRigidBodyPoolAuthority;
    type RigidBodyHandle = SimulationRigidBodyHandle;
    type JointHandle = SimulationJointHandle;
    type DriveHandle = SimulationDriveHandle;

    fn authority(&self) -> Self::Authority {
        self.authority()
    }

    fn add_rigid_body(
        &mut self,
        transform: &[f32; 16],
        inertia: &Self::Inertia,
        body_state: u32,
    ) -> Option<Self::RigidBodyHandle> {
        self.last_add_failure = None;
        if let Some(source) = inertia.as_source_routed() {
            return self.add_source_body(*transform, source, body_state);
        }
        match inertia.try_add(&mut self.rigid_bodies, *transform, body_state) {
            Ok(handle) => handle,
            Err(source) => {
                self.record_failure(SimulationAssemblyBackendAddFailure::RigidBody(source));
                None
            }
        }
    }

    fn rigid_body_pose(&self, body: &Self::RigidBodyHandle) -> [f32; 16] {
        if let Some(pose) = self
            .source_copyback_pose(*body)
            .expect("session-owned source body keeps its route and packed transform authority")
        {
            return pose;
        }
        self.rigid_bodies
            .try_rigid_body_pose(*body)
            .expect("session-owned rigid-body handle remains live through pose copyback")
    }

    fn rigid_body_part_transform(
        &self,
        body: &Self::RigidBodyHandle,
        inverse: Option<&[f32; 16]>,
    ) -> [f32; 16] {
        if let Some(pose) = self
            .source_copyback_pose(*body)
            .expect("session-owned source body keeps packed transform authority")
        {
            return source::source_part_transform(pose, inverse);
        }
        let pose = self
            .rigid_bodies
            .try_rigid_body_pose(*body)
            .expect("session-owned ordinary body remains live through copyback");
        inverse.map_or(pose, |inverse| {
            crate::world::compose_collision_affine(pose, *inverse)
        })
    }
    fn remove_rigid_body(&mut self, body: &Self::RigidBodyHandle) {
        if self
            .remove_source_body(*body)
            .expect("session-owned source body and outcome are removed exactly once")
        {
            return;
        }
        self.rigid_bodies
            .try_remove_rigid_body(*body)
            .expect("session-owned rigid-body handle is removed exactly once");
    }

    fn add_joint(
        &mut self,
        parent: &Self::RigidBodyHandle,
        child: &Self::RigidBodyHandle,
        frames: &Self::JointFrames,
        limits: &Self::JointLimits,
    ) -> Option<Self::JointHandle> {
        self.last_add_failure = None;
        if self.joints.counts().free == 0 {
            return None;
        }
        if let Err(source) = self.rigid_bodies.try_observe(*parent) {
            self.record_failure(SimulationAssemblyBackendAddFailure::JointBody {
                endpoint: SimulationAssemblyJointEndpoint::Parent,
                source,
            });
            return None;
        }
        if let Err(source) = self.rigid_bodies.try_observe(*child) {
            self.record_failure(SimulationAssemblyBackendAddFailure::JointBody {
                endpoint: SimulationAssemblyJointEndpoint::Child,
                source,
            });
            return None;
        }
        match limits.try_add(&mut self.joints, *parent, *child, frames) {
            Ok(handle) => handle,
            Err(source) => {
                self.record_failure(SimulationAssemblyBackendAddFailure::Joint(source));
                None
            }
        }
    }

    fn remove_joint(&mut self, joint: &Self::JointHandle) {
        // Native Assembly teardown removes bodies first. Joint removal therefore
        // deliberately consults only the Joint occurrence and ring.
        self.joints
            .try_remove_joint(*joint)
            .expect("session-owned Joint handle is removed exactly once");
    }

    fn add_drive(
        &mut self,
        parent: &Self::RigidBodyHandle,
        child: &Self::RigidBodyHandle,
        frames: &Self::DriveFrames,
        dynamics: &Self::DriveDynamics,
    ) -> Option<Self::DriveHandle> {
        self.last_add_failure = None;
        match self
            .drives
            .try_add_drive(&self.rigid_bodies, *parent, *child, frames, dynamics)
        {
            Ok(handle) => handle,
            Err(source) => {
                self.record_failure(SimulationAssemblyBackendAddFailure::Drive(source));
                None
            }
        }
    }

    fn remove_drive(&mut self, drive: &Self::DriveHandle) {
        // Drive removal, like Joint removal, never dereferences bodies retained
        // by the slot and is safe after body-first teardown.
        self.drives
            .try_remove_drive(*drive)
            .expect("session-owned Drive handle is removed exactly once");
    }
}

#[cfg(test)]
impl<M>
    SimulationAssemblyBackend<
        M,
        crate::simulation_joint_pool::SimulationJointFrames,
        crate::simulation_joint_pool::SimulationJointLimits,
    >
{
    /// Reads actual retained Active inputs, not constructor copies or compiled rows.
    pub(crate) fn active_joint_payloads_for_test(
        &self,
    ) -> Result<
        Vec<(
            crate::simulation_joint_pool::SimulationJointFrames,
            crate::simulation_joint_pool::SimulationJointLimits,
        )>,
        crate::simulation_joint_pool::SimulationJointActiveBuildVisitError<
            SimulationJointPoolError,
        >,
    > {
        self.joints
            .try_with_active_build_occurrences_and_reservation(
                |occurrences| {
                    occurrences
                        .map(|(_, occurrence)| Ok((occurrence.frames(), occurrence.try_limits()?)))
                        .collect()
                },
                |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
            )
    }
}

#[cfg(test)]
#[path = "simulation_assembly_payload_tests.rs"]
mod payload_tests;

impl<M, JF: Copy, JL: Copy>
    crate::dynamic_assembly_session::part_activation::DynamicAssemblyActivationBackend
    for SimulationAssemblyBackend<M, JF, JL>
{
    type ActivationError = SimulationRigidBodyPoolError;
    fn try_activation_state(
        &mut self,
        body: &Self::RigidBodyHandle,
    ) -> Result<u32, Self::ActivationError> {
        self.rigid_bodies.try_raw_state(body.observation())
    }
    fn try_activate_frozen(
        &mut self,
        body: &Self::RigidBodyHandle,
    ) -> Result<(), Self::ActivationError> {
        self.rigid_bodies
            .try_activate_frozen_body(body.observation())
    }
    fn try_clear_cooldown(
        &mut self,
        body: &Self::RigidBodyHandle,
    ) -> Result<(), Self::ActivationError> {
        self.rigid_bodies
            .try_clear_body_cooldown(body.observation())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        dynamic_assembly_session::{
            DynamicAssemblyAttachError, DynamicAssemblyDriveDefinition,
            DynamicAssemblyJointDefinition, DynamicAssemblyPartPayload,
            DynamicAssemblyRuntimeTemplate, DynamicAssemblySession,
        },
        dynamic_clustered_mesh_template::{
            DynamicClusteredMeshTemplate, DynamicClusteredMeshTemplatePart,
        },
        serialised_assembly_template::{
            SerialisedAssemblyPartInertiaSource, SerialisedAssemblyTemplatePart,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_drive_pool::{SimulationDriveDynamicsParams, SimulationDriveList},
        simulation_joint_pool::SimulationJointList,
        simulation_normal_arena_plan::SimulationNormalArenaPlan,
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::{CollisionArena, CollisionPrimitiveSet, CollisionRoot},
    };
    use skate2_formats::{
        collision::{AssemblyPartInertia, Volume},
        pegasus::ObjectId,
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::{ArenaResources, BaseResourceDescriptor},
    };

    pub(super) const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];

    pub(super) type Backend = SimulationAssemblyBackend<(), u32, u32>;
    pub(super) type Template = DynamicAssemblyRuntimeTemplate<
        SimulationAssemblyInertia,
        (),
        Arc<u32>,
        SimulationAssemblyJointLimits<u32>,
        Arc<RwLock<SimulationDriveFrames>>,
        Arc<RwLock<SimulationDriveDynamics>>,
    >;

    pub(super) fn collision_template(part_count: usize) -> Arc<DynamicClusteredMeshTemplate> {
        let roots = (0..part_count)
            .map(|index| CollisionRoot {
                dictionary_index: u32::try_from(index).expect("test index fits u32"),
                volume: Volume {
                    transform: IDENTITY,
                    vtable_offset: 1,
                    shape_words: [0; 3],
                    radius: 1.0,
                    group_id: 0,
                    surface_id: 0,
                    flags: 1,
                },
                aggregate: None,
            })
            .collect();
        let arena = Arc::new(CollisionArena {
            id: 1,
            grind_assets: Vec::new(),
            aggregate_roots: roots,
            primitive_sets: Vec::new(),
            simple_primitive_sets: Vec::new(),
            meshes: Vec::new(),
            collision_models: Vec::new(),
            instances: Vec::new(),
        });
        let parts = (0..part_count)
            .map(|root_index| DynamicClusteredMeshTemplatePart {
                root_index,
                inverse_body_transform: IDENTITY,
                has_inverse_body_transform: 0,
                is_static_body: 0,
            })
            .collect();
        Arc::new(DynamicClusteredMeshTemplate::try_new(arena, parts, false).expect("template"))
    }

    pub(super) fn serialised_arena_owner() -> Arc<SimulationNormalArenaPlan> {
        let resources = [
            vec![0; 0x100],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        let embedded_descriptors = std::array::from_fn(|index| ResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 0x10,
        });
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
            id: 0x8646_a1f7,
            entry_count: 0,
            used_entry_count: 0,
            alignment: 16,
            virtual_size: 0x100,
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: embedded_descriptors,
            resources_used: embedded_descriptors,
            resource_offsets: [u32::MAX; 5],
            arena_group_offset: 0,
            dictionary: Vec::new(),
        };
        let asset_resource_descriptors = std::array::from_fn(|index| BaseResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 0x10,
        });
        let decoded = DecodedArenaAsset {
            asset_id: 0x0123_4567_89ab_cdef,
            asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
            asset_resource_descriptors,
            resources: ArenaResources { resources },
            arena,
        };
        let collision = SimulationCollisionAsset {
            decoded,
            collision: CollisionArena {
                id: 0x8646_a1f7,
                grind_assets: Vec::new(),
                aggregate_roots: Vec::new(),
                primitive_sets: Vec::<CollisionPrimitiveSet>::new(),
                simple_primitive_sets: Vec::new(),
                meshes: Vec::new(),
                collision_models: Vec::new(),
                instances: Vec::new(),
            },
        };
        let static_plan = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
            collision,
            Vec::new(),
        ));
        Arc::new(SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap())
    }

    pub(super) fn serialised_binding(
        owner: Arc<SimulationNormalArenaPlan>,
        resource_zero_base: u32,
    ) -> Arc<SimulationArenaXenonAddressBinding> {
        Arc::new(
            SimulationArenaXenonAddressBinding::try_new(owner, [resource_zero_base, 0, 0, 0, 0])
                .unwrap(),
        )
    }

    pub(super) fn serialised_runtime_inertia(
        owner: Arc<SimulationNormalArenaPlan>,
        resource_offset: u32,
    ) -> SerialisedAssemblyRuntimeInertia {
        let source = crate::serialised_assembly_template::SerialisedAssemblyPartInertiaSource::from_parts_for_test(
            owner,
            0,
            resource_offset,
        );
        SerialisedAssemblyRuntimeInertia::from_parts_for_test(
            Arc::new(SimulationInertia {
                inverse_tensor: [1.0, 2.0, 3.0],
                inverse_mass: 0.25,
                ..SimulationInertia::native_default()
            }),
            source,
        )
    }

    fn serialised_backend(
        capacity: usize,
        arena_bindings: Vec<(
            Arc<SimulationNormalArenaPlan>,
            Arc<SimulationArenaXenonAddressBinding>,
        )>,
    ) -> SerialisedAssemblySimulationBackend<()> {
        let image = SimulationXenonAddressImage::try_new(
            0x1000_0080,
            u32::try_from(capacity).unwrap(),
            0,
            0,
        )
        .unwrap();
        SerialisedAssemblySimulationBackend::try_new(capacity, image, arena_bindings).unwrap()
    }

    fn serialised_material_backend(
        capacity: usize,
        arena_bindings: Vec<(
            Arc<SimulationNormalArenaPlan>,
            Arc<SimulationArenaXenonAddressBinding>,
        )>,
    ) -> SerialisedAssemblySimulationBackend<AssemblyPartMaterial> {
        let image = SimulationXenonAddressImage::try_new(
            0x1000_0080,
            u32::try_from(capacity).unwrap(),
            0,
            0,
        )
        .unwrap();
        SerialisedAssemblySimulationBackend::try_new(capacity, image, arena_bindings).unwrap()
    }

    fn noncommuting_transform(seed: f32) -> [f32; 16] {
        [
            0.0,
            seed,
            seed + 1.0,
            0.0,
            seed + 2.0,
            0.0,
            seed + 3.0,
            0.0,
            seed + 4.0,
            seed + 5.0,
            0.0,
            0.0,
            seed + 6.0,
            seed + 7.0,
            seed + 8.0,
            1.0,
        ]
    }

    pub(super) fn serialised_template_part(
        owner: Arc<SimulationNormalArenaPlan>,
        root_index: usize,
        inertia_offset: u32,
        seed: f32,
        has_inverse_body_transform: u32,
        is_static_body: u32,
    ) -> SerialisedAssemblyTemplatePart {
        SerialisedAssemblyTemplatePart {
            volume_object_id: ObjectId(u32::try_from(root_index).unwrap() + 1),
            root_index,
            inertia_source: SerialisedAssemblyPartInertiaSource::from_parts_for_test(
                owner,
                0,
                inertia_offset,
            ),
            inertia: AssemblyPartInertia {
                inverse_tensor: [seed, seed + 1.0, seed + 2.0, seed + 3.0],
                inverse_mass: seed + 4.0,
                spherical: seed + 5.0,
                max_velocity: seed + 6.0,
                max_omega: seed + 7.0,
                linear_drag: seed + 8.0,
                angular_drag: seed + 9.0,
            },
            inverse_body_transform: noncommuting_transform(seed + 10.0),
            material: AssemblyPartMaterial {
                static_friction: seed + 20.0,
                dynamic_friction: seed + 21.0,
                restitution: seed + 22.0,
            },
            has_inverse_body_transform,
            is_static_body,
        }
    }

    pub(super) fn serialised_template_for_binding(
        owner: Arc<SimulationNormalArenaPlan>,
        counts: [u32; 4],
        parts: Vec<SerialisedAssemblyTemplatePart>,
    ) -> SerialisedAssemblyTemplate {
        SerialisedAssemblyTemplate::from_parts_and_constraint_counts_for_test(
            owner,
            ObjectId(0),
            counts[0],
            counts[1],
            counts[2],
            counts[3],
            parts,
        )
    }

    fn source_runtime_template(
        inertias: Vec<SerialisedAssemblyRoutedInertia>,
    ) -> Arc<DynamicAssemblyRuntimeTemplate<SerialisedAssemblyRoutedInertia, (), (), (), (), ()>>
    {
        let collision = collision_template(inertias.len());
        let parts = inertias
            .into_iter()
            .map(|inertia| DynamicAssemblyPartPayload {
                inertia,
                material: (),
            })
            .collect();
        Arc::new(
            DynamicAssemblyRuntimeTemplate::try_new(collision, parts, Vec::new(), Vec::new())
                .unwrap(),
        )
    }

    pub(super) fn frames(seed: f32) -> Arc<RwLock<SimulationDriveFrames>> {
        Arc::new(RwLock::new(SimulationDriveFrames {
            quaternion_a: [seed, 0.0, 0.0, 1.0],
            position_a: [seed + 1.0, 0.0, 0.0, 0.0],
            quaternion_b: [0.0, seed, 0.0, 1.0],
            position_b: [0.0, seed + 2.0, 0.0, 0.0],
        }))
    }

    pub(super) fn dynamics(seed: f32) -> Arc<RwLock<SimulationDriveDynamics>> {
        Arc::new(RwLock::new(SimulationDriveDynamics {
            linear: SimulationDriveDynamicsParams {
                spring: seed,
                damping: seed + 1.0,
                strength: seed + 2.0,
                raw_type: 1,
            },
            angular: SimulationDriveDynamicsParams {
                spring: seed + 3.0,
                damping: seed + 4.0,
                strength: seed + 5.0,
                raw_type: 2,
            },
        }))
    }

    fn ordinal_f32(ordinal: usize) -> f32 {
        f32::from(u16::try_from(ordinal).expect("small test ordinal fits u16"))
    }

    pub(super) fn template(
        part_count: usize,
        joint_pairs: &[(i32, i32)],
        drive_pairs: &[(i32, i32)],
    ) -> Arc<Template> {
        let collision = collision_template(part_count);
        let parts = (0..part_count)
            .map(|index| DynamicAssemblyPartPayload {
                inertia: Arc::new(SimulationInertia {
                    inverse_tensor: [1.0 + ordinal_f32(index), 2.0, 3.0],
                    inverse_mass: 0.25,
                    ..SimulationInertia::native_default()
                })
                .into(),
                material: (),
            })
            .collect();
        let joints = joint_pairs
            .iter()
            .enumerate()
            .map(
                |(ordinal, &(parent_part, child_part))| DynamicAssemblyJointDefinition {
                    parent_part,
                    child_part,
                    frames: Arc::new(u32::try_from(ordinal).expect("ordinal") + 10),
                    limits: Arc::new(u32::try_from(ordinal).expect("ordinal") + 20).into(),
                },
            )
            .collect();
        let drives = drive_pairs
            .iter()
            .enumerate()
            .map(
                |(ordinal, &(parent_part, child_part))| DynamicAssemblyDriveDefinition {
                    parent_part,
                    child_part,
                    frames: frames(ordinal_f32(ordinal) + 1.0),
                    dynamics: dynamics(ordinal_f32(ordinal) + 10.0),
                },
            )
            .collect();
        Arc::new(
            DynamicAssemblyRuntimeTemplate::try_new(collision, parts, joints, drives)
                .expect("runtime template"),
        )
    }

    pub(super) fn transforms(count: usize) -> Vec<[f32; 16]> {
        (0..count)
            .map(|index| {
                let mut transform = IDENTITY;
                transform[12] = ordinal_f32(index) * 4.0;
                transform
            })
            .collect()
    }

    pub(super) fn assert_detached(backend: &Backend) {
        let bodies = backend.rigid_body_snapshot();
        let joints = backend.joint_snapshot();
        let drives = backend.drive_snapshot();
        assert_eq!(bodies.counts.free as usize, bodies.slots.len());
        assert_eq!(bodies.counts.static_bodies, 0);
        assert_eq!(bodies.counts.frozen, 0);
        assert_eq!(bodies.counts.active, 0);
        assert_eq!(joints.counts.free as usize, joints.slots.len());
        assert_eq!(joints.counts.active, 0);
        assert_eq!(drives.counts.free as usize, drives.slots.len());
        assert_eq!(drives.counts.active, 0);
    }

    #[test]
    fn end_to_end_success_binds_payloads_and_body_first_detach_is_safe() {
        let runtime = template(2, &[(0, 1)], &[(0, 1)]);
        let mut session =
            DynamicAssemblySession::<Backend>::try_new(runtime, transforms(2)).expect("session");
        let mut backend = Backend::try_new(2, 1, 1).expect("backend");

        session.attach(&mut backend, 4).expect("attach");
        let bodies = backend.rigid_body_snapshot();
        let joints = backend.joint_snapshot();
        let drives = backend.drive_snapshot();
        assert_eq!(bodies.active_order.len(), 2);
        assert_eq!(joints.active_order.len(), 1);
        assert_eq!(drives.active_order.len(), 1);
        assert_eq!(joints.slots[0].membership, SimulationJointList::Active);
        assert_eq!(
            joints.slots[0].body_b.expect("parent body").id(),
            bodies.slots[0].id
        );
        assert_eq!(drives.slots[0].membership, SimulationDriveList::Active);
        assert_eq!(drives.slots[0].body_b, joints.slots[0].body_b);
        assert_eq!(drives.slots[0].body_a, joints.slots[0].body_a);

        session.detach(&mut backend).expect("body-first detach");
        assert_detached(&backend);
        assert!(
            session
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .all(|transform| transform[15].to_bits() == 1.0_f32.to_bits())
        );
        assert_eq!(backend.last_add_failure(), None);
    }

    #[test]
    fn attached_session_can_run_the_bounded_body_integration_seam() {
        let runtime = template(1, &[], &[]);
        let mut session =
            DynamicAssemblySession::<Backend>::try_new(runtime, transforms(1)).expect("session");
        let mut backend = Backend::try_new(1, 0, 0).expect("backend");
        backend.set_gravity([0.0, -2.0, 0.0]);
        session.attach(&mut backend, 4).expect("attach");
        backend.begin_dynamic_update_frame(0.5);
        backend
            .try_integrate_active_bodies_ordinary_finite()
            .expect("bounded integration");

        let snapshot = backend.rigid_body_snapshot();
        assert_eq!(
            snapshot.slots[0].center_of_mass[1].to_bits(),
            (-0.5_f32).to_bits()
        );
        assert_eq!(
            snapshot.slots[0].velocity[1].to_bits(),
            (-1.0_f32).to_bits()
        );
        assert_eq!(
            snapshot.slots[0].force.map(f32::to_bits),
            [0, 0xc000_0000, 0]
        );

        session
            .detach(&mut backend)
            .expect("detach after integration");
        assert_eq!(
            session
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .next()
                .expect("part")[13]
                .to_bits(),
            (-0.5_f32).to_bits()
        );
    }

    #[test]
    fn part_capacity_failure_rolls_back_bodies_in_reverse_free_order() {
        let runtime = template(3, &[], &[]);
        let mut session =
            DynamicAssemblySession::<Backend>::try_new(runtime, transforms(3)).expect("session");
        let mut backend = Backend::try_new(2, 0, 0).expect("backend");
        assert_eq!(
            session.attach(&mut backend, 4),
            Err(DynamicAssemblyAttachError::PartUnavailable { ordinal: 2 })
        );
        assert_eq!(
            backend.rigid_body_snapshot().free_order,
            [
                backend.rigid_body_snapshot().slots[1].id,
                backend.rigid_body_snapshot().slots[0].id,
            ]
        );
        assert_detached(&backend);
        assert_eq!(backend.last_add_failure(), None);
    }

    #[test]
    fn joint_capacity_failure_rolls_joints_reverse_then_bodies_forward() {
        let pairs = [(0, 1), (1, 2), (2, 0)];
        let runtime = template(3, &pairs, &[]);
        let mut session =
            DynamicAssemblySession::<Backend>::try_new(runtime, transforms(3)).expect("session");
        let mut backend = Backend::try_new(3, 2, 0).expect("backend");
        assert_eq!(
            session.attach(&mut backend, 4),
            Err(DynamicAssemblyAttachError::JointUnavailable { ordinal: 2 })
        );
        let bodies = backend.rigid_body_snapshot();
        let joints = backend.joint_snapshot();
        assert_eq!(
            bodies.free_order,
            [bodies.slots[0].id, bodies.slots[1].id, bodies.slots[2].id]
        );
        assert_eq!(joints.free_order, [joints.slots[1].id, joints.slots[0].id]);
        assert_detached(&backend);
    }

    #[test]
    fn drive_capacity_failure_rolls_drives_reverse_bodies_then_joints_forward() {
        let pairs = [(0, 1), (1, 2), (2, 0)];
        let runtime = template(3, &pairs, &pairs);
        let mut session =
            DynamicAssemblySession::<Backend>::try_new(runtime, transforms(3)).expect("session");
        let mut backend = Backend::try_new(3, 3, 2).expect("backend");
        assert_eq!(
            session.attach(&mut backend, 4),
            Err(DynamicAssemblyAttachError::DriveUnavailable { ordinal: 2 })
        );
        let bodies = backend.rigid_body_snapshot();
        let joints = backend.joint_snapshot();
        let drives = backend.drive_snapshot();
        assert_eq!(
            bodies.free_order,
            [bodies.slots[0].id, bodies.slots[1].id, bodies.slots[2].id]
        );
        assert_eq!(
            joints.free_order,
            [joints.slots[0].id, joints.slots[1].id, joints.slots[2].id]
        );
        assert_eq!(drives.free_order, [drives.slots[1].id, drives.slots[0].id]);
        assert_detached(&backend);
    }

    #[test]
    fn successful_detach_can_reattach_with_fresh_occurrences_and_fifo_slots() {
        let runtime = template(2, &[(0, 1)], &[(0, 1)]);
        let mut session =
            DynamicAssemblySession::<Backend>::try_new(runtime, transforms(2)).expect("session");
        let mut backend = Backend::try_new(2, 1, 1).expect("backend");
        session.attach(&mut backend, 2).expect("first attach");
        let first_body_occurrences = backend
            .rigid_body_snapshot()
            .slots
            .iter()
            .map(|slot| slot.occurrence)
            .collect::<Vec<_>>();
        let first_joint_occurrence = backend.joint_snapshot().slots[0].occurrence;
        let first_drive_occurrence = backend.drive_snapshot().slots[0].occurrence;
        session.detach(&mut backend).expect("first detach");

        session.attach(&mut backend, 2).expect("reattach");
        let bodies = backend.rigid_body_snapshot();
        assert_eq!(
            bodies.frozen_order,
            [bodies.slots[0].id, bodies.slots[1].id]
        );
        assert!(
            bodies
                .slots
                .iter()
                .zip(first_body_occurrences)
                .all(|(slot, old)| slot.occurrence > old)
        );
        assert!(backend.joint_snapshot().slots[0].occurrence > first_joint_occurrence);
        assert!(backend.drive_snapshot().slots[0].occurrence > first_drive_occurrence);
        session.detach(&mut backend).expect("second detach");
        assert_detached(&backend);
    }

    #[test]
    fn direct_joint_adapter_preserves_parent_child_and_payload_mapping() {
        let mut backend = Backend::try_new(2, 1, 0).expect("backend");
        let inertia = SimulationAssemblyInertia::from(Arc::new(SimulationInertia {
            inverse_tensor: [1.0, 2.0, 3.0],
            inverse_mass: 0.25,
            ..SimulationInertia::native_default()
        }));
        let parent = backend
            .add_rigid_body(&IDENTITY, &inertia, 4)
            .expect("parent");
        let child = backend
            .add_rigid_body(&IDENTITY, &inertia, 4)
            .expect("child");
        let frames = Arc::new(0x1122_3344);
        let limits = SimulationAssemblyJointLimits::from(Arc::new(0x5566_7788));
        let joint = backend
            .add_joint(&parent, &child, &frames, &limits)
            .expect("joint");

        let body_slots = backend.rigid_body_snapshot().slots;
        let binding = backend.joints.try_binding(joint).expect("binding");
        assert_eq!(**binding.frames, 0x1122_3344);
        assert_eq!(**binding.limits, 0x5566_7788);
        assert_eq!(binding.body_b.id(), body_slots[0].id);
        assert_eq!(binding.body_a.id(), body_slots[1].id);

        backend.remove_rigid_body(&parent);
        backend.remove_rigid_body(&child);
        backend.remove_joint(&joint);
        assert_detached(&backend);
    }

    #[test]
    fn noncapacity_joint_failure_diagnostic_survives_rollback_removals() {
        let mut backend = Backend::try_new(2, 1, 0).expect("backend");
        let inertia = SimulationAssemblyInertia::from(Arc::new(SimulationInertia {
            inverse_tensor: [1.0, 1.0, 1.0],
            inverse_mass: 1.0,
            ..SimulationInertia::native_default()
        }));
        let parent = backend
            .add_rigid_body(&IDENTITY, &inertia, 4)
            .expect("parent");
        let child = backend
            .add_rigid_body(&IDENTITY, &inertia, 4)
            .expect("child");
        backend.remove_rigid_body(&child);

        assert_eq!(
            backend.add_joint(&parent, &child, &Arc::new(1), &Arc::new(2).into()),
            None
        );
        let expected = SimulationAssemblyBackendAddFailure::JointBody {
            endpoint: SimulationAssemblyJointEndpoint::Child,
            source: SimulationRigidBodyPoolError::BodyIsFree { slot: 1 },
        };
        assert_eq!(backend.last_add_failure(), Some(&expected));

        // Session rollback invokes removals after the failed add. Those calls
        // must not erase the diagnostic which explains the optional failure.
        backend.remove_rigid_body(&parent);
        assert_eq!(backend.last_add_failure(), Some(&expected));
        assert_detached(&backend);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one ordered fixture checks every field of the sealed three-Part batch"
    )]
    fn source_batch_preserves_three_definition_ordered_part_payloads() {
        let owner = serialised_arena_owner();
        let mut parts = vec![
            serialised_template_part(Arc::clone(&owner), 2, 0x10, 1.0, 0x8000_0000, 0),
            serialised_template_part(Arc::clone(&owner), 0, 0x40, 31.0, 0x4000_0000, 0x8000_0000),
            serialised_template_part(Arc::clone(&owner), 1, 0x70, 61.0, 0x2000_0000, 0),
        ];
        parts[0].material.static_friction = f32::from_bits(0x8000_0000);
        parts[1].material.dynamic_friction = f32::from_bits(0x7fc1_2345);
        parts[2].material.restitution = f32::from_bits(0x0000_0001);
        let template =
            serialised_template_for_binding(Arc::clone(&owner), [0, 0, 0, 0], parts.clone());
        assert_eq!(
            template
                .parts()
                .iter()
                .map(|part| part.root_index)
                .collect::<Vec<_>>(),
            [2, 0, 1]
        );
        let mut backend = serialised_material_backend(
            3,
            vec![(
                Arc::clone(&owner),
                serialised_binding(Arc::clone(&owner), 0x2000),
            )],
        );
        let before = backend.rigid_bodies.snapshot();
        let batch = backend
            .try_bind_part_attachment_template(&template)
            .unwrap();
        assert_eq!(backend.rigid_bodies.snapshot(), before);
        assert_eq!(backend.last_add_failure(), None);

        for (ordinal, part) in parts.iter().enumerate() {
            let metadata = batch.metadata_for_test()[ordinal];
            assert_eq!(
                metadata.inverse_body_transform.map(f32::to_bits),
                part.inverse_body_transform.map(f32::to_bits)
            );
            assert_eq!(
                (metadata.has_inverse_body_transform, metadata.is_static_body,),
                (part.has_inverse_body_transform, part.is_static_body)
            );
            let payload = &batch.payloads_for_test()[ordinal];
            assert_eq!(
                [
                    payload.material.static_friction,
                    payload.material.dynamic_friction,
                    payload.material.restitution,
                ]
                .map(f32::to_bits),
                [
                    part.material.static_friction,
                    part.material.dynamic_friction,
                    part.material.restitution,
                ]
                .map(f32::to_bits)
            );
            let semantic = payload.inertia.inertia.semantic();
            assert_eq!(
                [
                    semantic.inverse_tensor[0],
                    semantic.inverse_tensor[1],
                    semantic.inverse_tensor[2],
                    semantic.inverse_mass,
                    semantic.spherical,
                    semantic.max_velocity,
                    semantic.max_omega,
                    semantic.linear_drag,
                    semantic.angular_drag,
                ]
                .map(f32::to_bits),
                [
                    part.inertia.inverse_tensor[0],
                    part.inertia.inverse_tensor[1],
                    part.inertia.inverse_tensor[2],
                    part.inertia.inverse_mass,
                    part.inertia.spherical,
                    part.inertia.max_velocity,
                    part.inertia.max_omega,
                    part.inertia.linear_drag,
                    part.inertia.angular_drag,
                ]
                .map(f32::to_bits)
            );
            assert!(
                payload
                    .inertia
                    .inertia
                    .source()
                    .is_same_backing_inertia(&part.inertia_source)
            );
        }

        let mut bodies = Vec::new();
        for (ordinal, payload) in batch.payloads_for_test().iter().enumerate() {
            let metadata = batch.metadata_for_test()[ordinal];
            let body_state = if metadata.is_static_body == 0 { 4 } else { 1 };
            bodies.push(
                backend
                    .add_rigid_body(
                        &metadata.inverse_body_transform,
                        &payload.inertia,
                        body_state,
                    )
                    .expect("ordered Part Add"),
            );
        }
        assert_eq!(
            bodies
                .iter()
                .map(|body| body.body.id().index())
                .collect::<Vec<_>>(),
            [0_u32, 1_u32, 2_u32]
        );
        for body in &bodies {
            backend.remove_rigid_body(body);
        }
    }

    #[test]
    fn empty_source_batch_does_not_retain_an_unregistered_template_owner() {
        let owner = serialised_arena_owner();
        let weak_owner = Arc::downgrade(&owner);
        let template =
            serialised_template_for_binding(Arc::clone(&owner), [0, 0, 0, 0], Vec::new());
        let backend = serialised_material_backend(0, Vec::new());
        let before = backend.rigid_bodies.snapshot();

        let batch = backend
            .try_bind_part_attachment_template(&template)
            .expect("empty Definition order needs no registered route");
        assert!(batch.metadata_for_test().is_empty());
        assert!(batch.payloads_for_test().is_empty());
        assert_eq!(backend.rigid_bodies.snapshot(), before);
        assert_eq!(backend.last_add_failure(), None);

        drop(template);
        drop(owner);
        assert!(weak_owner.upgrade().is_none());
    }

    #[test]
    fn nonempty_source_batch_is_the_final_strong_arena_owner() {
        let owner = serialised_arena_owner();
        let weak_owner = Arc::downgrade(&owner);
        let template = serialised_template_for_binding(
            Arc::clone(&owner),
            [0, 0, 0, 0],
            vec![serialised_template_part(
                Arc::clone(&owner),
                0,
                0x10,
                1.0,
                0,
                0,
            )],
        );
        let binding = serialised_binding(Arc::clone(&owner), 0x2000);
        let backend =
            serialised_material_backend(1, vec![(Arc::clone(&owner), Arc::clone(&binding))]);
        let batch = backend
            .try_bind_part_attachment_template(&template)
            .expect("registered source route");

        drop(template);
        drop(binding);
        drop(backend);
        drop(owner);
        assert!(weak_owner.upgrade().is_some());
        drop(batch);
        assert!(weak_owner.upgrade().is_none());
    }

    #[test]
    fn source_batch_constraint_preflight_precedes_foreign_route_lookup() {
        let registered = serialised_arena_owner();
        let foreign = serialised_arena_owner();
        let part = serialised_template_part(Arc::clone(&foreign), 0, 0x10, 1.0, 0, 0);
        let backend = serialised_material_backend(
            1,
            vec![(
                Arc::clone(&registered),
                serialised_binding(registered, 0x2000),
            )],
        );
        let cases = [
            (
                [1, 2, 3, 4],
                SerialisedAssemblyPartAttachmentBindError::JointCountMismatch {
                    assembly: 1,
                    definition: 2,
                },
            ),
            (
                [1, 1, 3, 4],
                SerialisedAssemblyPartAttachmentBindError::DriveCountMismatch {
                    assembly: 3,
                    definition: 4,
                },
            ),
            (
                [1, 1, 0, 0],
                SerialisedAssemblyPartAttachmentBindError::UnsupportedJoints { count: 1 },
            ),
            (
                [0, 0, 1, 1],
                SerialisedAssemblyPartAttachmentBindError::UnsupportedDrives { count: 1 },
            ),
        ];
        let before = backend.rigid_bodies.snapshot();
        for (counts, expected) in cases {
            let template =
                serialised_template_for_binding(Arc::clone(&foreign), counts, vec![part.clone()]);
            assert_eq!(
                backend
                    .try_bind_part_attachment_template(&template)
                    .unwrap_err(),
                expected
            );
        }
        assert_eq!(backend.rigid_bodies.snapshot(), before);
        assert_eq!(backend.last_add_failure(), None);
    }

    #[test]
    fn byte_equal_foreign_owner_refuses_the_whole_batch_without_effects() {
        let registered = serialised_arena_owner();
        let foreign = serialised_arena_owner();
        assert!(!Arc::ptr_eq(&registered, &foreign));
        assert_eq!(
            registered.static_plan().asset().decoded.resources.resources,
            foreign.static_plan().asset().decoded.resources.resources
        );
        let template = serialised_template_for_binding(
            Arc::clone(&foreign),
            [0, 0, 0, 0],
            vec![serialised_template_part(foreign, 0, 0x10, 1.0, 0, 0)],
        );
        let backend = serialised_material_backend(
            1,
            vec![(
                Arc::clone(&registered),
                serialised_binding(registered, 0x2000),
            )],
        );
        let before = backend.rigid_bodies.snapshot();

        assert_eq!(
            backend
                .try_bind_part_attachment_template(&template)
                .unwrap_err(),
            SerialisedAssemblyPartAttachmentBindError::Route {
                ordinal: 0,
                source: SerialisedAssemblySimulationBindError::ArenaNotRegistered,
            }
        );
        assert_eq!(backend.rigid_bodies.snapshot(), before);
        assert_eq!(backend.last_add_failure(), None);
    }

    #[test]
    fn source_batch_retains_backend_affinity_without_body_effects() {
        let owner = serialised_arena_owner();
        let template = serialised_template_for_binding(
            Arc::clone(&owner),
            [0, 0, 0, 0],
            vec![serialised_template_part(
                Arc::clone(&owner),
                0,
                0x10,
                1.0,
                0,
                0,
            )],
        );
        let backend_a = serialised_material_backend(
            1,
            vec![(
                Arc::clone(&owner),
                serialised_binding(Arc::clone(&owner), 0x2000),
            )],
        );
        let mut backend_b = serialised_material_backend(
            1,
            vec![(Arc::clone(&owner), serialised_binding(owner, 0x4000))],
        );
        let before_a = backend_a.rigid_bodies.snapshot();
        let before_b = backend_b.rigid_bodies.snapshot();
        let batch = backend_a
            .try_bind_part_attachment_template(&template)
            .unwrap();
        assert_eq!(backend_a.rigid_bodies.snapshot(), before_a);
        let inertia = &batch.payloads_for_test()[0].inertia;

        assert!(backend_b.add_rigid_body(&IDENTITY, inertia, 4).is_none());
        assert_eq!(
            backend_b.last_add_failure(),
            Some(&SerialisedAssemblySimulationAddFailure::ForeignRoute)
        );
        assert_eq!(backend_a.rigid_bodies.snapshot(), before_a);
        assert_eq!(backend_b.rigid_bodies.snapshot(), before_b);
    }

    #[test]
    fn source_batch_defers_invalid_inertia_address_until_body_state_selects_it() {
        let owner = serialised_arena_owner();
        let mut backend = serialised_material_backend(
            1,
            vec![(
                Arc::clone(&owner),
                serialised_binding(Arc::clone(&owner), 0x2000),
            )],
        );
        let nonstatic = serialised_template_for_binding(
            Arc::clone(&owner),
            [0, 0, 0, 0],
            vec![serialised_template_part(
                Arc::clone(&owner),
                0,
                0x11,
                1.0,
                0,
                0,
            )],
        );
        let nonstatic_batch = backend
            .try_bind_part_attachment_template(&nonstatic)
            .expect("binding does not inspect the address");
        assert_eq!(nonstatic_batch.metadata_for_test()[0].is_static_body, 0);
        let before = backend.rigid_bodies.snapshot();
        assert!(
            backend
                .add_rigid_body(
                    &IDENTITY,
                    &nonstatic_batch.payloads_for_test()[0].inertia,
                    4,
                )
                .is_none()
        );
        assert_eq!(backend.rigid_bodies.snapshot(), before);
        assert_eq!(
            backend.last_add_failure(),
            Some(&SerialisedAssemblySimulationAddFailure::RigidBody(
                SimulationRigidBodyAddInertiaOwnerError::Binding(
                    crate::simulation_xenon_address_image::SimulationArenaXenonAddressBindingError::MisalignedInertiaAddress {
                        address: 0x2011,
                        required_alignment: 0x10,
                    },
                )
            ))
        );

        let static_template = serialised_template_for_binding(
            Arc::clone(&owner),
            [0, 0, 0, 0],
            vec![serialised_template_part(
                owner,
                0,
                0x11,
                2.0,
                0,
                0x8000_0000,
            )],
        );
        let static_batch = backend
            .try_bind_part_attachment_template(&static_template)
            .expect("Static source also binds without address inspection");
        let static_raw = static_batch.metadata_for_test()[0].is_static_body;
        assert_eq!(static_raw, 0x8000_0000);
        let selected_state = if static_raw == 0 { 4 } else { 1 };
        let static_body = backend
            .add_rigid_body(
                &IDENTITY,
                &static_batch.payloads_for_test()[0].inertia,
                selected_state,
            )
            .expect("Static Add skips the invalid Definition address");
        assert_eq!(backend.last_add_failure(), None);
        assert_eq!(backend.rigid_bodies.snapshot().counts.static_bodies, 1);
        backend.remove_rigid_body(&static_body);
    }

    #[test]
    fn source_routes_distinguish_two_byte_equal_arena_owners() {
        let owner_a = serialised_arena_owner();
        let owner_b = serialised_arena_owner();
        assert!(!Arc::ptr_eq(&owner_a, &owner_b));
        assert_eq!(
            owner_a.static_plan().asset().decoded.resources.resources,
            owner_b.static_plan().asset().decoded.resources.resources
        );
        let inertia_a = serialised_runtime_inertia(Arc::clone(&owner_a), 0x10);
        let inertia_b = serialised_runtime_inertia(Arc::clone(&owner_b), 0x10);
        let mut backend = serialised_backend(
            2,
            vec![
                (Arc::clone(&owner_a), serialised_binding(owner_a, 0x2000)),
                (Arc::clone(&owner_b), serialised_binding(owner_b, 0x4000)),
            ],
        );
        let routed_a = backend.try_bind_inertia(&inertia_a).unwrap();
        let routed_b = backend.try_bind_inertia(&inertia_b).unwrap();
        assert_ne!(routed_a.route, routed_b.route);

        let body_a = backend
            .add_rigid_body(&IDENTITY, &routed_a, 4)
            .expect("owner A body");
        let body_b = backend
            .add_rigid_body(&IDENTITY, &routed_b, 4)
            .expect("owner B body");
        for (body, expected_address, expected_source) in [
            (body_a, 0x2010, inertia_a.source()),
            (body_b, 0x4010, inertia_b.source()),
        ] {
            let route = &backend.routes[backend.route_index(body.route).unwrap()];
            let outcome = route
                .inertia
                .try_outcome(&backend.rigid_bodies, body.body.observation())
                .unwrap();
            match outcome {
                crate::simulation_xenon_address_image::SimulationRigidBodyAddInertiaOutcome::Definition {
                    address,
                    source,
                } => {
                    assert_eq!(address, expected_address);
                    assert!(source.is_same_backing_inertia(expected_source));
                }
                other => panic!("unexpected active outcome {other:?}"),
            }
        }

        backend.remove_rigid_body(&body_a);
        backend.remove_rigid_body(&body_b);
        assert_eq!(backend.rigid_bodies.counts().free, 2);
    }

    #[test]
    fn supplied_static_state_skips_even_invalid_definition_address_binding() {
        let owner = serialised_arena_owner();
        let inertia = serialised_runtime_inertia(Arc::clone(&owner), 0x11);
        let mut backend = serialised_backend(
            1,
            vec![(Arc::clone(&owner), serialised_binding(owner, 0x2000))],
        );
        let routed = backend.try_bind_inertia(&inertia).unwrap();

        let body = backend
            .add_rigid_body(&IDENTITY, &routed, 1)
            .expect("Static ignores Definition address alignment");
        let snapshot = backend.rigid_bodies.snapshot();
        assert_eq!(snapshot.counts.static_bodies, 1);
        assert_eq!(snapshot.slots[0].state, 1);
        assert_eq!(snapshot.slots[0].inertia, None);
        let route = &backend.routes[backend.route_index(body.route).unwrap()];
        assert_eq!(
            route
                .inertia
                .try_outcome(&backend.rigid_bodies, body.body.observation()),
            Ok(crate::simulation_xenon_address_image::SimulationRigidBodyAddInertiaOutcome::StaticNull)
        );
        backend.remove_rigid_body(&body);
    }

    #[test]
    fn unregistered_owner_prebind_has_no_backend_effects() {
        let registered = serialised_arena_owner();
        let foreign = serialised_arena_owner();
        let backend = serialised_backend(
            1,
            vec![(
                Arc::clone(&registered),
                serialised_binding(registered, 0x2000),
            )],
        );
        let before = backend.rigid_bodies.snapshot();
        let inertia = serialised_runtime_inertia(foreign, 0x10);

        assert!(matches!(
            backend.try_bind_inertia(&inertia),
            Err(SerialisedAssemblySimulationBindError::ArenaNotRegistered)
        ));
        assert_eq!(backend.rigid_bodies.snapshot(), before);
        assert_eq!(backend.last_add_failure(), None);
    }

    #[test]
    fn routed_inertia_from_foreign_backend_is_zero_effect() {
        let owner = serialised_arena_owner();
        let inertia = serialised_runtime_inertia(Arc::clone(&owner), 0x10);
        let backend_a = serialised_backend(
            1,
            vec![(
                Arc::clone(&owner),
                serialised_binding(Arc::clone(&owner), 0x2000),
            )],
        );
        let mut backend_b = serialised_backend(
            1,
            vec![(Arc::clone(&owner), serialised_binding(owner, 0x4000))],
        );
        let routed_a = backend_a.try_bind_inertia(&inertia).unwrap();
        let before_a = backend_a.rigid_bodies.snapshot();
        let before_b = backend_b.rigid_bodies.snapshot();

        assert!(backend_b.add_rigid_body(&IDENTITY, &routed_a, 4).is_none());
        assert_eq!(
            backend_b.last_add_failure(),
            Some(&SerialisedAssemblySimulationAddFailure::ForeignRoute)
        );
        assert_eq!(backend_a.rigid_bodies.snapshot(), before_a);
        assert_eq!(backend_b.rigid_bodies.snapshot(), before_b);
    }

    #[test]
    fn invalid_nonstatic_definition_binding_is_zero_body_effect() {
        let owner = serialised_arena_owner();
        let inertia = serialised_runtime_inertia(Arc::clone(&owner), 0x11);
        let mut backend = serialised_backend(
            1,
            vec![(Arc::clone(&owner), serialised_binding(owner, 0x2000))],
        );
        let routed = backend.try_bind_inertia(&inertia).unwrap();
        let before = backend.rigid_bodies.snapshot();

        assert!(backend.add_rigid_body(&IDENTITY, &routed, 4).is_none());
        assert_eq!(
            backend.last_add_failure(),
            Some(&SerialisedAssemblySimulationAddFailure::RigidBody(
                SimulationRigidBodyAddInertiaOwnerError::Binding(
                    crate::simulation_xenon_address_image::SimulationArenaXenonAddressBindingError::MisalignedInertiaAddress {
                        address: 0x2011,
                        required_alignment: 0x10,
                    },
                ),
            ))
        );
        assert_eq!(backend.rigid_bodies.snapshot(), before);
    }

    #[test]
    fn later_part_binding_failure_rolls_back_prior_outcome_and_keeps_diagnostic() {
        let owner = serialised_arena_owner();
        let valid = serialised_runtime_inertia(Arc::clone(&owner), 0x10);
        let invalid = serialised_runtime_inertia(Arc::clone(&owner), 0x11);
        let mut backend = serialised_backend(
            2,
            vec![(Arc::clone(&owner), serialised_binding(owner, 0x2000))],
        );
        let routed_valid = backend.try_bind_inertia(&valid).unwrap();
        let routed_invalid = backend.try_bind_inertia(&invalid).unwrap();
        let mut session = DynamicAssemblySession::try_new(
            source_runtime_template(vec![routed_valid.clone(), routed_invalid]),
            transforms(2),
        )
        .unwrap();

        assert_eq!(
            session.attach(&mut backend, 4),
            Err(DynamicAssemblyAttachError::PartUnavailable { ordinal: 1 })
        );
        let rolled_back = backend.rigid_bodies.snapshot();
        assert_eq!(rolled_back.counts.free, 2);
        assert_eq!(
            backend.last_add_failure(),
            Some(&SerialisedAssemblySimulationAddFailure::RigidBody(
                SimulationRigidBodyAddInertiaOwnerError::Binding(
                    crate::simulation_xenon_address_image::SimulationArenaXenonAddressBindingError::MisalignedInertiaAddress {
                        address: 0x2011,
                        required_alignment: 0x10,
                    },
                ),
            ))
        );

        let first = backend
            .add_rigid_body(&IDENTITY, &routed_valid, 4)
            .expect("FIFO slot 1 outcome is vacant");
        let second = backend
            .add_rigid_body(&IDENTITY, &routed_valid, 4)
            .expect("previously populated slot 0 outcome was retired");
        assert_eq!(
            [first.body.id(), second.body.id()],
            [rolled_back.free_order[0], rolled_back.free_order[1]]
        );
        let rebound = backend.rigid_bodies.snapshot();
        let occurrence_floor = rolled_back.next_occurrence.expect("unexhausted test pool");
        let rebound_occurrences = [first, second].map(|body| {
            rebound.slots[usize::try_from(body.body.id().index()).unwrap()]
                .occurrence
                .expect("re-added live occurrence")
        });
        assert!(
            rebound_occurrences
                .iter()
                .all(|&value| value >= occurrence_floor)
        );
        assert_ne!(rebound_occurrences[0], rebound_occurrences[1]);
        for body in [first, second] {
            let route = &backend.routes[backend.route_index(body.route).unwrap()];
            assert!(matches!(
                route
                    .inertia
                    .try_outcome(&backend.rigid_bodies, body.body.observation()),
                Ok(crate::simulation_xenon_address_image::SimulationRigidBodyAddInertiaOutcome::Definition {
                    address: 0x2010,
                    ..
                })
            ));
        }
        backend.remove_rigid_body(&first);
        backend.remove_rigid_body(&second);
    }

    #[test]
    fn unsupported_joint_rolls_back_bodies_and_keeps_diagnostic() {
        let owner = serialised_arena_owner();
        let inertia = serialised_runtime_inertia(Arc::clone(&owner), 0x10);
        let mut backend = serialised_backend(
            2,
            vec![(Arc::clone(&owner), serialised_binding(owner, 0x2000))],
        );
        let routed = backend.try_bind_inertia(&inertia).unwrap();
        let routed_after_rollback = routed.clone();
        let parts = vec![
            DynamicAssemblyPartPayload {
                inertia: routed.clone(),
                material: (),
            },
            DynamicAssemblyPartPayload {
                inertia: routed,
                material: (),
            },
        ];
        let joints = vec![DynamicAssemblyJointDefinition {
            parent_part: 0,
            child_part: 1,
            frames: (),
            limits: (),
        }];
        let template = Arc::new(
            DynamicAssemblyRuntimeTemplate::try_new(
                collision_template(2),
                parts,
                joints,
                Vec::new(),
            )
            .unwrap(),
        );
        let mut session = DynamicAssemblySession::try_new(template, transforms(2)).unwrap();

        assert_eq!(
            session.attach(&mut backend, 4),
            Err(DynamicAssemblyAttachError::JointUnavailable { ordinal: 0 })
        );
        assert_eq!(backend.rigid_bodies.counts().free, 2);
        assert_eq!(
            backend.last_add_failure(),
            Some(&SerialisedAssemblySimulationAddFailure::UnsupportedJoint)
        );
        let first = backend
            .add_rigid_body(&IDENTITY, &routed_after_rollback, 4)
            .expect("first Joint-rollback outcome was retired");
        let second = backend
            .add_rigid_body(&IDENTITY, &routed_after_rollback, 4)
            .expect("second Joint-rollback outcome was retired");
        backend.remove_rigid_body(&first);
        backend.remove_rigid_body(&second);
    }

    #[test]
    fn constructor_rejects_two_bindings_for_one_exact_arena_owner() {
        let owner = serialised_arena_owner();
        let image = SimulationXenonAddressImage::try_new(0x1000_0080, 1, 0, 0).unwrap();
        let result = SerialisedAssemblySimulationBackend::<()>::try_new(
            1,
            image,
            vec![
                (
                    Arc::clone(&owner),
                    serialised_binding(Arc::clone(&owner), 0x2000),
                ),
                (Arc::clone(&owner), serialised_binding(owner, 0x4000)),
            ],
        );
        assert!(matches!(
            result,
            Err(
                SerialisedAssemblySimulationBackendInitError::DuplicateArenaOwner {
                    first: 0,
                    second: 1,
                }
            )
        ));
    }

    #[test]
    fn binding_owner_mismatch_precedes_duplicate_claimed_owner() {
        let claimed_owner = serialised_arena_owner();
        let actual_owner = serialised_arena_owner();
        let image = SimulationXenonAddressImage::try_new(0x1000_0080, 1, 0, 0).unwrap();
        let result = SerialisedAssemblySimulationBackend::<()>::try_new(
            1,
            image,
            vec![
                (
                    Arc::clone(&claimed_owner),
                    serialised_binding(Arc::clone(&claimed_owner), 0x2000),
                ),
                (claimed_owner, serialised_binding(actual_owner, 0x4000)),
            ],
        );
        assert!(matches!(
            result,
            Err(SerialisedAssemblySimulationBackendInitError::BindingOwnerMismatch { ordinal: 1 })
        ));
    }

    #[test]
    fn session_capacity_rollback_retires_outcome_before_route_reuse() {
        let owner = serialised_arena_owner();
        let inertia = serialised_runtime_inertia(Arc::clone(&owner), 0x10);
        let mut backend = serialised_backend(
            1,
            vec![(Arc::clone(&owner), serialised_binding(owner, 0x2000))],
        );
        let routed = backend.try_bind_inertia(&inertia).unwrap();
        let mut failing = DynamicAssemblySession::try_new(
            source_runtime_template(vec![routed.clone(), routed.clone()]),
            transforms(2),
        )
        .unwrap();
        assert_eq!(
            failing.attach(&mut backend, 4),
            Err(DynamicAssemblyAttachError::PartUnavailable { ordinal: 1 })
        );
        let rolled_back = backend.rigid_bodies.snapshot();
        assert_eq!(rolled_back.counts.free, 1);
        assert_eq!(rolled_back.counts.active, 0);
        assert_eq!(backend.last_add_failure(), None);

        let mut reused =
            DynamicAssemblySession::try_new(source_runtime_template(vec![routed]), transforms(1))
                .unwrap();
        reused.attach(&mut backend, 4).expect("route reused");
        let attached = backend.rigid_bodies.snapshot();
        assert_eq!(attached.counts.active, 1);
        assert!(attached.slots[0].occurrence > rolled_back.slots[0].occurrence);
        reused.detach(&mut backend).unwrap();
        assert_eq!(backend.rigid_bodies.counts().free, 1);
    }

    #[test]
    fn capacity_failure_is_effect_free_and_removed_slot_reuses_exact_route() {
        let owner = serialised_arena_owner();
        let inertia = serialised_runtime_inertia(Arc::clone(&owner), 0x10);
        let mut backend = serialised_backend(
            1,
            vec![(Arc::clone(&owner), serialised_binding(owner, 0x2000))],
        );
        let routed = backend.try_bind_inertia(&inertia).unwrap();
        let first = backend
            .add_rigid_body(&IDENTITY, &routed, 4)
            .expect("first body");
        let full = backend.rigid_bodies.snapshot();

        assert!(backend.add_rigid_body(&IDENTITY, &routed, 4).is_none());
        assert_eq!(backend.last_add_failure(), None);
        assert_eq!(backend.rigid_bodies.snapshot(), full);

        backend.remove_rigid_body(&first);
        let reused = backend
            .add_rigid_body(&IDENTITY, &routed, 4)
            .expect("reused body");
        assert_eq!(reused.body.id(), first.body.id());
        assert_ne!(reused.body.observation(), first.body.observation());
        assert_eq!(reused.route, first.route);
        let route = &backend.routes[backend.route_index(reused.route).unwrap()];
        assert!(matches!(
            route
                .inertia
                .try_outcome(&backend.rigid_bodies, reused.body.observation()),
            Ok(crate::simulation_xenon_address_image::SimulationRigidBodyAddInertiaOutcome::Definition {
                address: 0x2010,
                ..
            })
        ));
        backend.remove_rigid_body(&reused);
        assert_eq!(backend.rigid_bodies.counts().free, 1);
    }
}

#[path = "simulation_assembly_apply_angular.rs"]
pub(crate) mod apply_angular;

impl<M, JF: Copy, JL: Copy> SimulationAssemblyBackend<M, JF, JL> {
    pub(crate) fn try_apply_skateboard_world_force(
        &mut self,
        body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        force: crate::skeleton_root_transforms::SkeletonRootVector,
        point: crate::skeleton_root_transforms::SkeletonRootVector,
        part_position: crate::skeleton_root_transforms::SkeletonRootVector,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies
            .try_apply_skateboard_world_force(body, force, point, part_position)
    }
}
