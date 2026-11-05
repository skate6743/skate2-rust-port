//! Selected components of the single `HiLOD` `PhysOut` owner (0x82e2f2a8).
//! Constructor/reset completion describes this projection and its collaborators,
//! never completion of a native frame or an implicit buffer swap.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "selected output awaits HiLOD phase wiring")
)]

use crate::{
    skateboard_processed_surface::{
        SkateboardProcessedSurface, SkateboardSurfaceDispatchError, SkateboardSurfacePublication,
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;
#[path = "physical_player_phys_out_action_conditions.rs"]
mod action_conditions;

#[path = "physical_player_phys_out_collision.rs"]
mod collision;
#[path = "physical_player_phys_out_com.rs"]
pub(crate) mod com;
#[path = "conditioning/physical_player_phys_out_conditioners.rs"]
pub(crate) mod conditioners;
#[path = "physical_player_phys_out_construct.rs"]
pub(crate) mod construct;
#[cfg(test)]
#[path = "physical_player_phys_out_construct_tests.rs"]
mod construct_tests;
#[path = "physical_player_phys_out_grinds.rs"]
mod grinds;
#[path = "physical_player_phys_out_mission.rs"]
mod mission;
#[path = "physical_player_phys_out_motion.rs"]
pub(crate) mod motion;
#[path = "physical_player_phys_out_offboard_fill.rs"]
pub(crate) mod offboard_fill;
#[path = "physical_player_phys_out_path.rs"]
mod path;
#[path = "physical_player_phys_out_scoring.rs"]
mod scoring;
#[path = "physical_player_phys_out_perception.rs"]
pub(crate) mod perception;
#[cfg(feature = "skate3-runtime")]
#[path = "physical_player_phys_out_skate3.rs"]
pub(crate) mod skate3;
#[path = "physical_player_phys_out_simple.rs"]
mod simple;
#[path = "physical_player_phys_out_surface_motion.rs"]
pub(crate) mod surface_motion;
#[path = "physical_player_phys_out_teleporting_fill.rs"]
pub(crate) mod teleporting_fill;
#[path = "physical_player_phys_out_trick.rs"]
pub(crate) mod trick;

const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);
const UP: SkeletonRootVector = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
const FORWARD: SkeletonRootVector = SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]);
const RIGHT: SkeletonRootVector = SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysOutOrigin {
    Constructor,
    Reset,
    Publication,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PhysOutValue<T> {
    pub(crate) value: T,
    pub(crate) origin: PhysOutOrigin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysOutComponent {
    SkateboardReckoning,
    Motion,
    Air,
    Physics,
    Grinds,
    Skeleton,
    Collision,
    State,
    Ground,
    SystemReckoning,
    Audio,
    Camera,
    Trick,
    Intents,
    Animation,
    Scoring2,
    Filtered,
    PathRecording,
    OffBoard,
}
const COMPONENTS: [PhysOutComponent; 19] = [
    PhysOutComponent::SkateboardReckoning,
    PhysOutComponent::Motion,
    PhysOutComponent::Air,
    PhysOutComponent::Physics,
    PhysOutComponent::Grinds,
    PhysOutComponent::Skeleton,
    PhysOutComponent::Collision,
    PhysOutComponent::State,
    PhysOutComponent::Ground,
    PhysOutComponent::SystemReckoning,
    PhysOutComponent::Audio,
    PhysOutComponent::Camera,
    PhysOutComponent::Trick,
    PhysOutComponent::Intents,
    PhysOutComponent::Animation,
    PhysOutComponent::Scoring2,
    PhysOutComponent::Filtered,
    PhysOutComponent::PathRecording,
    PhysOutComponent::OffBoard,
];

/// Collaborators own the omitted native storage and operations. In particular,
/// `PathRecording` reset includes its three ordered `Node::Reset` calls. There is
/// deliberately no production implementation that silently accepts these stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysOutRemainderStage {
    Allocate(PhysOutComponent),
    ConstructOmitted(PhysOutComponent),
    ResetOmitted(PhysOutComponent),
    CollisionInfoConstructor,
    CollisionInfoReset,
}
pub(crate) trait PhysOutRemainder {
    type Error;
    fn run(&mut self, stage: PhysOutRemainderStage) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysOutAccessError {
    NotConstructed,
    LockedForWrite,
    ForeignGuard,
}
#[derive(Debug, PartialEq)]
pub(crate) enum PhysOutLifecycleError<E> {
    AlreadyStarted,
    NotConstructed,
    Remainder(E),
}
#[derive(Debug, PartialEq)]
pub(crate) enum PhysOutPublicationError<E> {
    Access(PhysOutAccessError),
    Source(E),
}
#[derive(Debug, PartialEq)]
pub(crate) enum PhysOutSurfaceError<E> {
    Access(PhysOutAccessError),
    Dispatch(SkateboardSurfaceDispatchError<E>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::enum_variant_names,
    reason = "variants preserve the distinct source velocity field names"
)]
pub(crate) enum PhysOutMotionVector {
    AngularVelocity,
    DeckVelocity,
    SurfaceVelocity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysOutMotionScalar {
    DeckSpeed,
    SurfaceSpeed,
    ZSpeed,
    SurfaceAngle,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::enum_variant_names,
    reason = "variants preserve the distinct source up-vector field names"
)]
pub(crate) enum PhysOutGroundVector {
    DynamicUp,
    GroundUp,
    LastValidGroundUp,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysOutSystemVector {
    Up,
    Heading,
    Right,
    CentreOfMass,
    CentreOfMassVelocity,
}

/// The source sometimes samples the latch before numerical work and only takes
/// its assertion branch afterwards. Keep that sampled condition and owner.
pub(crate) struct PhysOutWriteGuard {
    occurrence: Arc<()>,
    locked: bool,
}

pub(crate) struct PhysicalPlayerPhysOut {
    concrete: construct::ConcretePhysOut,
    // Host provenance for the selected actual body motion producer, cleared
    // on reset/attempt. This is not a native field or full Fill certificate.
    motion_source_frame:
        Option<Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>>,
    occurrence: Arc<()>,
    construction_started: bool,
    selected_construction_complete: bool,
    selected_reset_complete: bool,
    locked: Option<bool>,
    motion_vectors: [Option<PhysOutValue<SkeletonRootVector>>; 3],
    motion_scalars: [Option<PhysOutValue<f32>>; 4],
    ground_vectors: [Option<PhysOutValue<SkeletonRootVector>>; 3],
    ground_transform: Option<PhysOutValue<SkeletonRootMatrix>>,
    system_vectors: [Option<PhysOutValue<SkeletonRootVector>>; 5],
    collision_type: Option<PhysOutValue<u32>>,
}

impl PhysicalPlayerPhysOut {
    /// Selected `FillPhysOut` copies30304..30370. Each source read follows its
    /// own latch sample and precedes that sampled latch's assertion branch.
    /// Both destination fields copy the retained last-valid normal.
    pub(crate) fn try_publish_last_ground_up<E>(
        &mut self,
        mut read: impl FnMut() -> Result<SkeletonRootVector, E>,
    ) -> Result<(), PhysOutPublicationError<E>> {
        for field in [
            PhysOutGroundVector::GroundUp,
            PhysOutGroundVector::LastValidGroundUp,
        ] {
            let guard = self
                .sample_write_guard()
                .map_err(PhysOutPublicationError::Access)?;
            let value = read().map_err(PhysOutPublicationError::Source)?;
            self.publish_ground_vector(&guard, field, value)
                .map_err(PhysOutPublicationError::Access)?;
        }
        Ok(())
    }
    pub(crate) fn occurrence(&self) -> &Arc<()> {
        &self.occurrence
    }
    pub(crate) fn motion_source_frame(
        &self,
    ) -> Option<&Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>>
    {
        self.motion_source_frame.as_ref()
    }
    pub(crate) fn set_motion_source_frame(
        &mut self,
        frame: Option<
            Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>,
        >,
    ) {
        self.motion_source_frame = frame;
    }

    pub(crate) fn new() -> Self {
        Self {
            concrete: construct::ConcretePhysOut::new(),
            motion_source_frame: None,
            occurrence: Arc::new(()),
            construction_started: false,
            selected_construction_complete: false,
            selected_reset_complete: false,
            locked: None,
            motion_vectors: [None; 3],
            motion_scalars: [None; 4],
            ground_vectors: [None; 3],
            ground_transform: None,
            system_vectors: [None; 5],
            collision_type: None,
        }
    }
    /// One-shot construction leaves all executed selected primitive stores in
    /// this same owner when a later allocation/constructor collaborator fails.
    pub(crate) fn try_construct<R: PhysOutRemainder>(
        &mut self,
        remainder: &mut R,
    ) -> Result<(), PhysOutLifecycleError<R::Error>> {
        use PhysOutLifecycleError::{AlreadyStarted, Remainder};
        if self.construction_started {
            return Err(AlreadyStarted);
        }
        self.construction_started = true;
        for component in COMPONENTS {
            remainder
                .run(PhysOutRemainderStage::Allocate(component))
                .map_err(Remainder)?;
            if component == PhysOutComponent::Collision {
                remainder
                    .run(PhysOutRemainderStage::CollisionInfoConstructor)
                    .map_err(Remainder)?;
            }
            if !self.reset_selected(component, PhysOutOrigin::Constructor) {
                remainder
                    .run(PhysOutRemainderStage::ConstructOmitted(component))
                    .map_err(Remainder)?;
            }
            if component == PhysOutComponent::Collision {
                remainder
                    .run(PhysOutRemainderStage::CollisionInfoReset)
                    .map_err(Remainder)?;
            }
        }
        // 0x82e2f730, after all component constructor operations.
        self.locked = Some(false);
        self.selected_construction_complete = true;
        Ok(())
    }
    /// 0x82e2f75c..2f8dc: exact component order. Omitted stages can fail after
    /// earlier selected fields reset; the old latch survives until final success.
    pub(crate) fn try_reset<R: PhysOutRemainder>(
        &mut self,
        remainder: &mut R,
    ) -> Result<(), PhysOutLifecycleError<R::Error>> {
        use PhysOutLifecycleError::{NotConstructed, Remainder};
        if !self.selected_construction_complete {
            return Err(NotConstructed);
        }
        self.selected_reset_complete = false;
        self.motion_source_frame = None;
        for component in COMPONENTS {
            if !self.reset_selected(component, PhysOutOrigin::Reset) {
                remainder
                    .run(PhysOutRemainderStage::ResetOmitted(component))
                    .map_err(Remainder)?;
            }
            if component == PhysOutComponent::Collision {
                remainder
                    .run(PhysOutRemainderStage::CollisionInfoReset)
                    .map_err(Remainder)?;
            }
        }
        self.locked = Some(false);
        self.selected_reset_complete = true;
        Ok(())
    }
    fn reset_selected(&mut self, component: PhysOutComponent, origin: PhysOutOrigin) -> bool {
        match component {
            PhysOutComponent::Motion => {
                self.motion_vectors = [Some(PhysOutValue {
                    value: ZERO,
                    origin,
                }); 3];
                self.motion_scalars = [Some(PhysOutValue { value: 0.0, origin }); 4];
            }
            PhysOutComponent::Collision => {
                // 0x82e24f6c, before nested SkeletonCollisionInfo::Reset.
                self.collision_type = Some(PhysOutValue { value: 0, origin });
            }
            PhysOutComponent::Ground => {
                self.ground_transform = Some(PhysOutValue {
                    value: SkeletonRootMatrix::from_columns([RIGHT, UP, FORWARD, ZERO]),
                    origin,
                });
                self.ground_vectors = [Some(PhysOutValue { value: UP, origin }); 3];
            }
            PhysOutComponent::SystemReckoning => {
                self.system_vectors = [UP, FORWARD, RIGHT, ZERO, ZERO]
                    .map(|value| Some(PhysOutValue { value, origin }));
            }
            _ => return false,
        }
        true
    }
    pub(crate) const fn selected_construction_complete(&self) -> bool {
        self.selected_construction_complete
    }
    pub(crate) const fn selected_reset_complete(&self) -> bool {
        self.selected_reset_complete
    }
    pub(crate) const fn locked_for_write(&self) -> Option<bool> {
        self.locked
    }
    /// Exact late Update1 store 0x82e03ae8; no partial-copy helper invokes this.
    pub(crate) fn lock_after_update1(&mut self) -> Result<(), PhysOutAccessError> {
        self.require_constructed()?;
        self.locked = Some(true);
        Ok(())
    }
    fn require_constructed(&self) -> Result<(), PhysOutAccessError> {
        if self.selected_construction_complete {
            Ok(())
        } else {
            Err(PhysOutAccessError::NotConstructed)
        }
    }
    pub(crate) fn sample_write_guard(&self) -> Result<PhysOutWriteGuard, PhysOutAccessError> {
        self.require_constructed()?;
        Ok(PhysOutWriteGuard {
            occurrence: self.occurrence.clone(),
            locked: self.locked.expect("constructed latch"),
        })
    }
    pub(crate) fn check_write_guard(
        &self,
        guard: &PhysOutWriteGuard,
    ) -> Result<(), PhysOutAccessError> {
        self.require_constructed()?;
        if !Arc::ptr_eq(&self.occurrence, &guard.occurrence) {
            return Err(PhysOutAccessError::ForeignGuard);
        }
        if guard.locked {
            Err(PhysOutAccessError::LockedForWrite)
        } else {
            Ok(())
        }
    }
    fn check_current_guard(&self) -> Result<(), PhysOutAccessError> {
        self.check_write_guard(&self.sample_write_guard()?)
    }
    pub(crate) fn motion_vector(
        &self,
        field: PhysOutMotionVector,
    ) -> Result<PhysOutValue<SkeletonRootVector>, PhysOutAccessError> {
        self.check_current_guard()?;
        self.motion_vectors[field as usize].ok_or(PhysOutAccessError::NotConstructed)
    }
    pub(crate) fn motion_scalar(
        &self,
        field: PhysOutMotionScalar,
    ) -> Result<PhysOutValue<f32>, PhysOutAccessError> {
        self.check_current_guard()?;
        self.motion_scalars[field as usize].ok_or(PhysOutAccessError::NotConstructed)
    }
    pub(crate) fn ground_vector(
        &self,
        field: PhysOutGroundVector,
    ) -> Result<PhysOutValue<SkeletonRootVector>, PhysOutAccessError> {
        self.check_current_guard()?;
        self.ground_vectors[field as usize].ok_or(PhysOutAccessError::NotConstructed)
    }
    pub(crate) fn system_vector(
        &self,
        field: PhysOutSystemVector,
    ) -> Result<PhysOutValue<SkeletonRootVector>, PhysOutAccessError> {
        self.check_current_guard()?;
        self.system_vectors[field as usize].ok_or(PhysOutAccessError::NotConstructed)
    }
    pub(crate) fn collision_physics_type(&self) -> Result<PhysOutValue<u32>, PhysOutAccessError> {
        self.check_current_guard()?;
        self.collision_type
            .ok_or(PhysOutAccessError::NotConstructed)
    }
    pub(crate) fn publish_motion_vector(
        &mut self,
        guard: &PhysOutWriteGuard,
        field: PhysOutMotionVector,
        value: SkeletonRootVector,
    ) -> Result<(), PhysOutAccessError> {
        self.check_write_guard(guard)?;
        self.motion_vectors[field as usize] = Some(PhysOutValue {
            value,
            origin: PhysOutOrigin::Publication,
        });
        Ok(())
    }
    pub(crate) fn publish_motion_scalar(
        &mut self,
        guard: &PhysOutWriteGuard,
        field: PhysOutMotionScalar,
        value: f32,
    ) -> Result<(), PhysOutAccessError> {
        self.check_write_guard(guard)?;
        self.motion_scalars[field as usize] = Some(PhysOutValue {
            value,
            origin: PhysOutOrigin::Publication,
        });
        Ok(())
    }
    pub(crate) fn publish_ground_vector(
        &mut self,
        guard: &PhysOutWriteGuard,
        field: PhysOutGroundVector,
        value: SkeletonRootVector,
    ) -> Result<(), PhysOutAccessError> {
        self.check_write_guard(guard)?;
        self.ground_vectors[field as usize] = Some(PhysOutValue {
            value,
            origin: PhysOutOrigin::Publication,
        });
        Ok(())
    }
    pub(crate) fn publish_system_vector(
        &mut self,
        guard: &PhysOutWriteGuard,
        field: PhysOutSystemVector,
        value: SkeletonRootVector,
    ) -> Result<(), PhysOutAccessError> {
        self.check_write_guard(guard)?;
        self.system_vectors[field as usize] = Some(PhysOutValue {
            value,
            origin: PhysOutOrigin::Publication,
        });
        Ok(())
    }
    /// 0x82b2fbf4..fc20: output guard before the actual retained source read.
    pub(crate) fn try_publish_collision_type<E>(
        &mut self,
        read: impl FnOnce() -> Result<u32, E>,
    ) -> Result<(), PhysOutPublicationError<E>> {
        self.check_current_guard()
            .map_err(PhysOutPublicationError::Access)?;
        let value = read().map_err(PhysOutPublicationError::Source)?;
        self.collision_type = Some(PhysOutValue {
            value,
            origin: PhysOutOrigin::Publication,
        });
        Ok(())
    }
    /// `SurfacePhysics` reads this exact owner's published enum before Dispatch.
    /// The selected attribute token belongs to processed state, not `PhysOut`.
    pub(crate) fn try_dispatch_surface<P: SkateboardSurfacePublication>(
        &self,
        surfaces: &SkateboardProcessedSurface,
        output: &mut P,
    ) -> Result<(), PhysOutSurfaceError<P::Error>> {
        let incoming = self
            .collision_physics_type()
            .map_err(PhysOutSurfaceError::Access)?;
        surfaces
            .dispatch_from_published_type(incoming.value, output)
            .map_err(PhysOutSurfaceError::Dispatch)
    }
}

#[cfg(test)]
#[path = "physical_player_phys_out_tests.rs"]
mod tests;
