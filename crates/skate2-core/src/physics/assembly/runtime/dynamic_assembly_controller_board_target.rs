//! Actual board setters used by Controller retrieval/hiding, without new bodies.
use super::{SimulationSkaterAssemblyHandle as Handle, SimulationSkaterPoolError};
use crate::{
    dynamic_assembly_session::{
        hook_pool::SimulationHookPoolError, part_transform::DynamicAssemblyPartTransformError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    skateboard_spawn::{
        FallibleSkateboardSpawnServices, SkateboardSpawnError, TypedSkateboardSpawnParts,
    },
    skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
    skeleton_root_transforms::{SkeletonRootMatrix as M, SkeletonRootVector as V},
};

#[derive(Debug)]
pub(crate) enum ControllerBoardTargetError {
    Pool(SimulationSkaterPoolError),
    Missing(&'static str),
    Poisoned,
    Part(DynamicAssemblyPartTransformError),
    Hook(SimulationHookPoolError),
    Cvar(InputCvarError),
    Velocity(crate::skateboard_set_velocity::SkateboardSetVelocityError),
    DeckVelocity(crate::dynamic_assembly_session::skater_pool::board_entry::SkateboardEntrySessionError),
    Spy(crate::dynamic_assembly_session::part_contact_spy::DynamicAssemblyPartContactSpyError),
    Body(crate::simulation_rigid_body_pool::SimulationRigidBodyPoolError),
}
impl std::fmt::Display for ControllerBoardTargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "controller board target: {self:?}")
    }
}
type E = ControllerBoardTargetError;
struct SpawnServices<'a>(&'a InputCvarReferences);
impl FallibleSkateboardSpawnServices for SpawnServices<'_> {
    fn try_current_orthogonal_tolerance(&mut self) -> Result<f32, InputCvarError> {
        self.0.try_orthogonal_tolerance()
    }
    fn orthogonality_assertion_failed(&mut self) {
        panic!("native SkateboardBody::SetTransform orthogonality assertion")
    }
}
pub(crate) struct BoardTargetAccess<'a> {
    pub(crate) skaters: Option<&'a mut super::SimulationSkaterAssemblyPool>,
    pub(crate) backend: &'a mut crate::skateboard_assembly::SkateboardSimulationBackend,
    pub(crate) hooks: &'a mut super::SimulationHookAssemblyPool<
        super::AssemblyPartMaterial,
        crate::skateboard_hook::SkateboardHookDefinition,
    >,
}
struct Parts<'a, 'b> {
    world: &'a mut BoardTargetAccess<'b>,
    index: usize,
}
impl TypedSkateboardSpawnParts for Parts<'_, '_> {
    type Error = E;
    fn live_board_part(&mut self, part: usize) -> Result<M, E> {
        self.world
            .skaters
            .as_ref()
            .ok_or(E::Missing("skater pool"))?
            .members[self.index]
            .content
            .board()
            .map_err(E::Pool)?
            .0
            .session
            .try_live_part_transform_typed(&self.world.backend, part)
            .map_err(E::Part)
    }
    fn set_board_part(&mut self, part: usize, matrix: M) -> Result<(), E> {
        self.world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .members[self.index]
            .content
            .board_mut()
            .map_err(E::Pool)?
            .session
            .try_set_part_transform_typed(&mut self.world.backend, part, &matrix)
            .map_err(E::Part)
    }
    fn set_hook_part(&mut self, matrix: M) -> Result<(), E> {
        let hook = self
            .world
            .skaters
            .as_ref()
            .ok_or(E::Missing("skater pool"))?
            .members[self.index]
            .content
            .board()
            .map_err(E::Pool)?
            .0
            .hook
            .as_ref()
            .ok_or(E::Missing("board hook"))?;
        self.world
            .hooks
            .try_set_board_hook_part_typed(&mut self.world.backend, hook, &matrix)
            .map_err(E::Hook)
    }
}
impl BoardTargetAccess<'_> {
    fn controller_board_target_index(&self, board: &Handle) -> Result<usize, E> {
        let pool = self.skaters.as_ref().ok_or(E::Missing("skater pool"))?;
        pool.check_backend(&self.backend).map_err(E::Pool)?;
        let index = pool.member_index(board).map_err(E::Pool)?;
        pool.members[index].content.board().map_err(E::Pool)?;
        Ok(index)
    }
    /// SetTargetOrientation82b32ac0 writes only the retained hook Part0.
    pub(crate) fn try_set_controller_board_target_orientation(
        &mut self,
        board: &Handle,
        orientation: &M,
    ) -> Result<(), E> {
        let index = self.controller_board_target_index(board)?;
        Parts { world: self, index }.set_hook_part(*orientation)
    }
    /// SetPosition82b35a80: copy current deck Part transform, replace P, then
    /// execute the SAME SetTransform used by relocation, including all 7 Parts.
    pub(crate) fn try_set_controller_board_position(
        &mut self,
        board: &Handle,
        position: &V,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        let index = self.controller_board_target_index(board)?;
        let mut parts = Parts { world: self, index };
        let mut requested = parts.live_board_part(6)?;
        requested.set_column(3, *position);
        parts
            .world
            .try_set_board_transform_typed(board, requested, refs)
    }
    /// SetTransform82b34a90 shared by relocation and controller positioning.
    pub(crate) fn try_set_board_transform_typed(
        &mut self,
        board: &Handle,
        requested: M,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        let index = self.controller_board_target_index(board)?;
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .invalidate_refresh();
        crate::skateboard_spawn::try_set_skateboard_transform_typed(
            &mut Parts { world: self, index },
            requested,
            &mut SpawnServices(refs),
            &mut HostSkateboardSpawnMathNumeric,
        )
        .map_err(|e| match e {
            SkateboardSpawnError::Cvar(e) => E::Cvar(e),
            SkateboardSpawnError::Parts(e) => e,
        })
    }
    /// `Body::GetPosition(6)` 0x82de7b20: the deck Part's live transform.
    pub(crate) fn try_deck_part_transform(&mut self, board: &Handle) -> Result<M, E> {
        let index = self.controller_board_target_index(board)?;
        Parts { world: self, index }.live_board_part(6)
    }
    /// `Body::SetPosition(.., 6)` 0x82de8090: the deck Part alone, its body woken.
    pub(crate) fn try_set_deck_part_position(&mut self, board: &Handle, matrix: &M) -> Result<(), E> {
        let index = self.controller_board_target_index(board)?;
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        pool.invalidate_refresh();
        pool.members[index]
            .content
            .board_mut()
            .map_err(E::Pool)?
            .session
            .try_set_body_position_typed(self.backend, 6, matrix)
            .map_err(E::Part)
    }
    /// `Skateboard::GetDeckVelocity` 0x82b25ff8.
    pub(crate) fn try_deck_velocity(&mut self, board: &Handle) -> Result<[f32; 3], E> {
        let index = self.controller_board_target_index(board)?;
        let candidate = self
            .skaters
            .as_ref()
            .ok_or(E::Missing("skater pool"))?
            .members[index]
            .content
            .board()
            .map_err(E::Pool)?
            .0;
        super::board_entry::try_current_board_deck_velocity(&candidate.session, self.backend)
            .map(|velocity| velocity.xyz)
            .map_err(E::DeckVelocity)
    }
    /// The deck body's `mVel` store `FollowPath_NoGeometry` 0x82b2aee0 makes before the
    /// solve, without waking the body.
    pub(crate) fn try_set_deck_body_velocity(&mut self, board: &Handle, velocity: [f32; 3]) -> Result<(), E> {
        self.try_store_deck_body_velocity(board, velocity, false)
    }
    /// Diagnostics: the deck body of the pool's `index`th skater given `velocity` before the solve.
    pub(crate) fn try_diagnostic_deck_velocity(&mut self, index: usize, velocity: [f32; 3]) -> Result<(), E> {
        let observation = self
            .skaters
            .as_ref()
            .ok_or(E::Missing("skater pool"))?
            .members[index]
            .content
            .board()
            .map_err(E::Pool)?
            .0
            .session
            .try_part_contact_spy_observation(self.backend, 6)
            .map_err(E::Spy)?;
        self.backend.try_set_part_linear_velocity(observation, velocity).map_err(E::Body)
    }
    /// `PostPhysics_AdjustVelocity` 0x82b32b28's store after the solve.
    pub(crate) fn try_store_post_solve_deck_velocity(&mut self, board: &Handle, velocity: [f32; 3]) -> Result<(), E> {
        self.try_store_deck_body_velocity(board, velocity, true)
    }
    fn try_store_deck_body_velocity(&mut self, board: &Handle, velocity: [f32; 3], post_solve: bool) -> Result<(), E> {
        let index = self.controller_board_target_index(board)?;
        let observation = self
            .skaters
            .as_ref()
            .ok_or(E::Missing("skater pool"))?
            .members[index]
            .content
            .board()
            .map_err(E::Pool)?
            .0
            .session
            .try_part_contact_spy_observation(self.backend, 6)
            .map_err(E::Spy)?;
        if post_solve {
            self.backend.try_store_post_solve_linear_velocity(observation, velocity)
        } else {
            self.backend.try_set_part_linear_velocity(observation, velocity)
        }
        .map_err(E::Body)
    }
    /// TargetPositionWithVelocity82b2a038 reads body (not composed Part) P;
    /// scalar reciprocal precedes subtraction and the existing all-Part setter.
    pub(crate) fn try_target_controller_board_position_with_velocity(
        &mut self,
        board: &Handle,
        position: &V,
    ) -> Result<(), E> {
        let index = self.controller_board_target_index(board)?;
        let candidate = self
            .skaters
            .as_ref()
            .ok_or(E::Missing("skater pool"))?
            .members[index]
            .content
            .board()
            .map_err(E::Pool)?
            .0;
        let owner = candidate
            .construction
            .as_ref()
            .and_then(|c| c.physical_owner())
            .ok_or(E::Missing("physical owner"))?;
        let processed = std::sync::Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let dt = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(E::Missing("Processed DT"))?;
        crate::skateboard_set_velocity::try_target_position_with_velocity(
            &candidate.session,
            &mut self.backend,
            dt,
            position,
        )
        .map_err(|e| match e {
            crate::skateboard_set_velocity::TargetPositionError::Transform(e) => E::Part(e),
            crate::skateboard_set_velocity::TargetPositionError::Velocity(e) => E::Velocity(e),
        })
    }
}
