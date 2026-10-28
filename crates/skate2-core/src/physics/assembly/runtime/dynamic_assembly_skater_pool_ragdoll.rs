//! Same admitted pair and controller state8, with retained constructor inputs.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    dynamic_assembly_session::{
        skater_pool::SimulationSkeletonJumpPairError,
        skeleton_normal::{
            SkeletonNormalAssembly,
            ragdoll_owner::{RagdollInputs, RagdollSetupError},
        },
    },
    input_cvar_runtime::InputCvarReferences,
    simulation_assembly_backend::constraint_session::physical_skeleton_controller::SkeletonCollisionTransition,
    skeleton_controller::{SkeletonControllerCollision, SkeletonControllerError},
    skeleton_runtime_state::SkeletonRuntimeStateError,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum RagdollCollisionError<C> {
    Runtime(SkeletonRuntimeStateError),
    Setup(RagdollSetupError<C>),
    /// `SetUpRagdoll` on a Skeleton whose ragdoll constructor inputs are not retained.
    MissingInputs,
}
impl<C: std::fmt::Debug> std::fmt::Display for RagdollCollisionError<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(e) => write!(f, "{e}"),
            Self::Setup(e) => write!(f, "{e}"),
            Self::MissingInputs => f.write_str("SetUpRagdoll without the Skeleton's ragdoll constructor inputs"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum PairRagdollError<C> {
    Pair(SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    Poisoned,
    Controller(SkeletonControllerError<RagdollCollisionError<C>>),
}
impl<C: std::fmt::Debug> std::fmt::Display for PairRagdollError<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(e) => write!(f, "ragdoll pair: {e}"),
            Self::Pool(e) => write!(f, "ragdoll pool: {e}"),
            Self::Poisoned => f.write_str("ragdoll controller lock poisoned"),
            Self::Controller(e) => write!(f, "{e}"),
        }
    }
}

/// Every `SkeletonController::EnterState` (0x82dd488c) mode on the same pair's Skeleton,
/// `SetUpRagdoll` (states 3 and 8) included, with the owner's live cvar references that
/// `SetOffBoardCollision` samples.
struct Collision<'a, C, F> {
    normal: &'a mut SkeletonNormalAssembly,
    backend: &'a mut SkateboardSimulationBackend,
    /// The Skeleton's retained ragdoll constructor inputs, which one `SetUpRagdoll` takes.
    inputs: Option<RagdollInputs<'a>>,
    cvars: &'a InputCvarReferences,
    cosine: &'a mut F,
    marker: std::marker::PhantomData<C>,
}
impl<C, F: FnMut(f32) -> Result<f32, C>> SkeletonControllerCollision for Collision<'_, C, F> {
    type Error = RagdollCollisionError<C>;
    fn set_ground_collision(&mut self) -> Result<(), Self::Error> {
        self.normal
            .try_set_ground_collision()
            .map_err(RagdollCollisionError::Runtime)
    }
    fn collide_with_world_only(&mut self) -> Result<(), Self::Error> {
        self.normal
            .try_collide_with_world_only()
            .map_err(RagdollCollisionError::Runtime)
    }
    fn collide_with_world_only_offboard(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        self.normal.try_collide_with_world_only_offboard();
        Ok(())
    }
    fn set_air_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        self.normal
            .try_set_air_collision()
            .map_err(|e| SkeletonControllerError::Collision(RagdollCollisionError::Runtime(e)))
    }
    fn set_offboard_air_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        self.normal
            .try_set_offboard_air_collision()
            .map_err(|e| SkeletonControllerError::Collision(RagdollCollisionError::Runtime(e)))
    }
    fn set_offboard_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        self.normal.try_set_offboard_collision(self.cvars).map_err(runtime_controller_error)
    }
    fn set_ragdoll_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        let inputs = self
            .inputs
            .take()
            .ok_or(SkeletonControllerError::Collision(RagdollCollisionError::MissingInputs))?;
        self.normal
            .try_set_up_ragdoll(self.backend, inputs, self.cosine)
            .map_err(RagdollCollisionError::Setup)
            .map_err(SkeletonControllerError::Collision)
    }
}

/// A Skeleton mode's own controller error as this implementor's.
fn runtime_controller_error<C>(
    error: SkeletonControllerError<SkeletonRuntimeStateError>,
) -> SkeletonControllerError<RagdollCollisionError<C>> {
    use SkeletonControllerError as E;
    match error {
        E::AlreadyConstructed => E::AlreadyConstructed,
        E::Unconstructed => E::Unconstructed,
        E::Collision(e) => E::Collision(RagdollCollisionError::Runtime(e)),
        E::CurrentWorldOnlyState(state) => E::CurrentWorldOnlyState(state),
        E::InvalidWorldOnlyRequest(state) => E::InvalidWorldOnlyRequest(state),
        E::InvalidCollisionState(state) => E::InvalidCollisionState(state),
        E::PendingCollisionMode(state) => E::PendingCollisionMode(state),
        E::Cvar(e) => E::Cvar(e),
    }
}

impl SimulationSkaterAssemblyPool {
    /// Restore SkeletonBody's constructor byte (82b62594) when a new game
    /// reuses a host Actor. Ordinary teleports and ragdoll entries retain it.
    pub(crate) fn try_pair_set_spawn_bouncy(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        bouncy: bool,
    ) -> Result<(), PairRagdollError<std::convert::Infallible>> {
        use PairRagdollError as E;
        self.try_with_jump_pair(backend, board, skeleton, |_, _| ())
            .map_err(E::Pair)?;
        let index = self.skeleton_mutation_index(backend, skeleton).map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("validated Skeleton")
        };
        let stored = candidate.construction.as_mut()
            .and_then(|construction| construction.constructor.bindings.as_mut())
            .and_then(|bindings| bindings.bouncy_ragdoll.as_mut())
            .ok_or(E::Pool(SimulationSkaterPoolError::MissingPhysicalConstruction))?;
        *stored = u8::from(bouncy);
        Ok(())
    }

    pub(crate) fn try_pair_ragdoll_collision<C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        cosine: &mut impl FnMut(f32) -> Result<f32, C>,
    ) -> Result<(), PairRagdollError<C>> {
        use PairRagdollError as E;
        let owner = self
            .try_with_jump_pair(backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("validated Skeleton")
        };
        let (bouncy, culling, body_reader, joint_reader) = candidate
            .construction
            .as_mut()
            .and_then(|construction| construction.ragdoll_inputs_mut())
            .ok_or(E::Pool(
                SimulationSkaterPoolError::MissingPhysicalConstruction,
            ))?;
        let mut collision = Collision {
            normal: &mut candidate.owner,
            backend,
            inputs: Some(RagdollInputs {
                bouncy,
                culling,
                body_reader,
                joint_reader,
                cvars: refs,
            }),
            cvars: refs,
            cosine,
            marker: std::marker::PhantomData,
        };
        owner
            .skeleton_controller()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_set_collision_state(8, &mut collision)
            .map_err(E::Controller)
    }

    /// `SetSkeletonCollisionState`, `InitAirCollisionUpdate`, `StartCollideWithWorldOnly` or
    /// `StopCollideWithWorldOnly` of the same pair, whose `EnterState` may set the ragdoll up
    /// again: world-only collision ending while the skater bails (`UpdateCollisionStatus` near a
    /// living world object) re-enters the stored ragdoll state 8, `SetUpRagdoll`.
    pub(crate) fn try_pair_collision_transition<C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cosine: &mut impl FnMut(f32) -> Result<f32, C>,
        transition: SkeletonCollisionTransition,
    ) -> Result<(), PairRagdollError<C>> {
        use PairRagdollError as E;
        let owner = self
            .try_with_jump_pair(backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("validated Skeleton")
        };
        let cvars = &owner.inputs().cvars;
        let inputs = candidate
            .construction
            .as_mut()
            .and_then(|construction| construction.ragdoll_inputs_mut())
            .map(|(bouncy, culling, body_reader, joint_reader)| RagdollInputs { bouncy, culling, body_reader, joint_reader, cvars });
        let mut collision = Collision {
            normal: &mut candidate.owner,
            backend,
            inputs,
            cvars,
            cosine,
            marker: std::marker::PhantomData,
        };
        let mut controller = owner.skeleton_controller().lock().map_err(|_| E::Poisoned)?;
        match transition {
            SkeletonCollisionTransition::Set(state) => controller.try_set_collision_state(state, &mut collision),
            SkeletonCollisionTransition::InitAir => controller.try_init_air_collision(&mut collision),
            SkeletonCollisionTransition::StartWorldOnly => controller.try_start_world_only(&mut collision),
            SkeletonCollisionTransition::StopWorldOnly => controller.try_stop_world_only(&mut collision),
        }
        .map_err(E::Controller)
    }
}
