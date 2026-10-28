//! Actual same-pair `SkeletonController` and collision publication mutations.
//! Selected explicit stages, not a completed `PhysicsGround.Enter` or `HiLOD` ctor.
use crate::{
    dynamic_assembly_session::{
        skater_pool::{SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError},
        skeleton_normal::SkeletonNormalAssembly,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    skeleton_controller::{SkeletonControllerCollision, SkeletonControllerError},
    skeleton_runtime_state::SkeletonRuntimeStateError,
};
use std::fmt;

#[cfg(test)]
#[path = "../runtime/physical_player_ragdoll_corpus.rs"]
pub(crate) mod ragdoll_corpus;

#[path = "../runtime/physical_player_hip_hook_soft_session.rs"]
pub(crate) mod hip_hook_soft;

#[derive(Debug)]
pub(crate) enum PhysicalSkeletonControllerError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    Poisoned,
    Controller(SkeletonControllerError<SkeletonRuntimeStateError>),
    /// A collision state transition, `SetUpRagdoll` included.
    Transition(crate::dynamic_assembly_session::skater_pool::ragdoll::PairRagdollError<std::convert::Infallible>),
}
impl fmt::Display for PhysicalSkeletonControllerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPool => f.write_str("SkeletonController skater pool unavailable"),
            Self::Pair(e) => write!(f, "SkeletonController pair: {e}"),
            Self::Poisoned => f.write_str("SkeletonController lock poisoned"),
            Self::Controller(e) => write!(f, "{e}"),
            Self::Transition(e) => write!(f, "{e}"),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SkeletonCollisionTransition {
    Set(i32),
    InitAir,
    StartWorldOnly,
    StopWorldOnly,
}

// This is the actual retained normal assembly, not a successful placeholder.
impl SkeletonControllerCollision for SkeletonNormalAssembly {
    type Error = SkeletonRuntimeStateError;
    fn set_ground_collision(&mut self) -> Result<(), Self::Error> {
        self.try_set_ground_collision()
    }
    fn collide_with_world_only(&mut self) -> Result<(), Self::Error> {
        self.try_collide_with_world_only()
    }
    fn collide_with_world_only_offboard(
        &mut self,
    ) -> Result<(), SkeletonControllerError<Self::Error>> {
        self.try_collide_with_world_only_offboard();
        Ok(())
    }
    fn set_air_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        self.try_set_air_collision()
            .map_err(SkeletonControllerError::Collision)
    }
    fn set_offboard_air_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        self.try_set_offboard_air_collision()
            .map_err(SkeletonControllerError::Collision)
    }
}

impl PhysicalAssemblyWorld {
    /// A retained first skater starts a new game with freshly sampled native
    /// constructor attributes; use the same admitted board/skeleton pair.
    pub(crate) fn try_pair_set_spawn_bouncy(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        bouncy: bool,
    ) -> Result<(), PhysicalRagdollError<std::convert::Infallible>> {
        self.skaters
            .as_mut()
            .ok_or(PhysicalRagdollError::MissingPool)?
            .try_pair_set_spawn_bouncy(&self.backend, board, skeleton, bouncy)
            .map_err(PhysicalRagdollError::Pair)
    }

    /// Selected controller state8 with this pair's retained dynamics, material
    /// records and live Vault readers. This is not complete `WipeoutGround.Enter`.
    pub(crate) fn try_pair_skeleton_ragdoll_collision<C>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &crate::input_cvar_runtime::InputCvarReferences,
        cosine: &mut impl FnMut(f32) -> Result<f32, C>,
    ) -> Result<(), PhysicalRagdollError<C>> {
        self.skaters
            .as_mut()
            .ok_or(PhysicalRagdollError::MissingPool)?
            .try_pair_ragdoll_collision(&mut self.backend, board, skeleton, refs, cosine)
            .map_err(PhysicalRagdollError::Pair)
    }

    /// Inline44a0/44a4 links are guaranteed by the enclosing SAME paired owner.
    /// Caller schedules this after actual Skeleton initialization.
    pub(crate) fn try_construct_pair_skeleton_controller(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalSkeletonControllerError> {
        use PhysicalSkeletonControllerError as E;
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| {
                owner
                    .skeleton_controller()
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .try_initialize_constructor()
                    .map_err(E::Controller)
            })
            .map_err(E::Pair)?
    }

    /// Source raw controller stores survive a collision failure. Publication
    /// becomes stale before possible volume/group writes, including on failure.
    /// Any `EnterState` mode can run, `SetUpRagdoll` with the wipeout entry's live cosine.
    pub(crate) fn try_pair_skeleton_collision_transition(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        transition: SkeletonCollisionTransition,
    ) -> Result<(), PhysicalSkeletonControllerError> {
        use PhysicalSkeletonControllerError as E;
        let mut cosine = |angle: f32| Ok::<f32, std::convert::Infallible>(angle.cos());
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_pair_collision_transition(&mut self.backend, board, skeleton, &mut cosine, transition)
            .map_err(E::Transition)
    }
}

#[derive(Debug)]
pub(crate) enum PhysicalRagdollError<C> {
    MissingPool,
    Pair(crate::dynamic_assembly_session::skater_pool::ragdoll::PairRagdollError<C>),
}
impl<C: fmt::Debug> fmt::Display for PhysicalRagdollError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPool => f.write_str("ragdoll skater pool unavailable"),
            Self::Pair(e) => write!(f, "{e}"),
        }
    }
}
