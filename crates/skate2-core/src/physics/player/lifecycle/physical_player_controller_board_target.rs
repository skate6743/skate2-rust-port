//! Borrow the actual World board owners for native retrieval setters.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle as Handle,
        controller_target::{BoardTargetAccess, ControllerBoardTargetError as E},
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    reckoning_numeric::ReckoningNumeric,
    skateboard_ai_path_following::{
        AiPathFollowingBoard, AiPathFollowingError, AiPathFollowingInput, PhysicsAiAttributes,
        try_update_ai_path_following,
    },
    skeleton_root_transforms::{SkeletonRootMatrix as M, SkeletonRootVector as V},
};
use std::sync::Arc;
impl PhysicalAssemblyWorld {
    pub(crate) fn try_controller_board_angular_exact<
        N: crate::reckoning_numeric::ReckoningNumeric,
    >(
        &mut self,
        board: &Handle,
        displacement: V,
        numeric: &mut N,
    ) -> Result<
        (),
        crate::dynamic_assembly_session::skater_pool::target_angular::TargetAngularOwnerError<
            N::Error,
        >,
    > {
        use crate::dynamic_assembly_session::skater_pool::{
            SimulationSkaterPoolError, target_angular::TargetAngularOwnerError,
        };
        self.skaters
            .as_ref()
            .ok_or(TargetAngularOwnerError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_target_board_angular_displacement_exact(
                &mut self.backend,
                board,
                displacement,
                numeric,
            )
    }
    fn controller_board_access(&mut self) -> BoardTargetAccess<'_> {
        BoardTargetAccess {
            skaters: self.skaters.as_mut(),
            backend: &mut self.backend,
            hooks: &mut self.hooks,
        }
    }
    pub(crate) fn try_set_board_transform_typed(
        &mut self,
        board: &Handle,
        transform: M,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        self.controller_board_access()
            .try_set_board_transform_typed(board, transform, refs)
    }
    pub(crate) fn try_set_controller_board_target_orientation(
        &mut self,
        board: &Handle,
        orientation: &M,
    ) -> Result<(), E> {
        self.controller_board_access()
            .try_set_controller_board_target_orientation(board, orientation)
    }
    pub(crate) fn try_set_controller_board_position(
        &mut self,
        board: &Handle,
        position: &V,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        self.controller_board_access()
            .try_set_controller_board_position(board, position, refs)
    }
    pub(crate) fn try_target_controller_board_position_with_velocity(
        &mut self,
        board: &Handle,
        position: &V,
    ) -> Result<(), E> {
        self.controller_board_access()
            .try_target_controller_board_position_with_velocity(board, position)
    }
    pub(crate) fn try_deck_part_transform(&mut self, board: &Handle) -> Result<M, E> {
        self.controller_board_access().try_deck_part_transform(board)
    }
    pub(crate) fn try_set_deck_part_position(&mut self, board: &Handle, matrix: &M) -> Result<(), E> {
        self.controller_board_access()
            .try_set_deck_part_position(board, matrix)
    }
    pub(crate) fn try_deck_velocity(&mut self, board: &Handle) -> Result<[f32; 3], E> {
        self.controller_board_access().try_deck_velocity(board)
    }
    pub(crate) fn try_set_deck_body_velocity(&mut self, board: &Handle, velocity: [f32; 3]) -> Result<(), E> {
        self.controller_board_access()
            .try_set_deck_body_velocity(board, velocity)
    }
    pub(crate) fn try_diagnostic_deck_velocity(&mut self, index: usize, velocity: [f32; 3]) -> Result<(), E> {
        self.controller_board_access().try_diagnostic_deck_velocity(index, velocity)
    }
    pub(crate) fn try_store_post_solve_deck_velocity(&mut self, board: &Handle, velocity: [f32; 3]) -> Result<(), E> {
        self.controller_board_access()
            .try_store_post_solve_deck_velocity(board, velocity)
    }
    /// `Skateboard::UpdateAIPathFollowing` 0x82b35fe8 on this board, from its owner's processed
    /// `AIPhysicsInput` and the global `physics_ai.default`.
    pub(crate) fn try_update_ai_path_following<N: ReckoningNumeric>(
        &mut self,
        board: &Handle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
    ) -> Result<(), AiPathFollowingError<E, N::Error>> {
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| AiPathFollowingError::Board(E::Poisoned))?
                .processed(),
        );
        let input = {
            let processed = processed
                .lock()
                .map_err(|_| AiPathFollowingError::Board(E::Poisoned))?;
            let vectors = &processed.input_controls().ai_vectors;
            let vector = |index: usize| {
                vectors[index].ok_or(AiPathFollowingError::Board(E::Missing("Processed AI vectors")))
            };
            AiPathFollowingInput {
                desired_deck_transform: [vector(0)?, vector(1)?, vector(2)?, vector(3)?],
                desired_deck_velocity: vector(4)?,
                flags: processed.physics_state_fields().ai_flags.0,
            }
        };
        let attributes = PhysicsAiAttributes::bind(&owner.inputs().vault);
        try_update_ai_path_following(
            &mut AiPathFollowingWorld { world: self, board, owner },
            &attributes,
            &input,
            numeric,
        )
    }
}

/// The board an AI's ground post-physics adjust moves, with its owner's reckoning and cvars.
struct AiPathFollowingWorld<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a Handle,
    owner: &'a PhysicalPlayerAssemblyOwner,
}
impl AiPathFollowingBoard for AiPathFollowingWorld<'_> {
    type Error = E;
    fn deck_part_transform(&mut self) -> Result<M, E> {
        self.world.try_deck_part_transform(self.board)
    }
    fn set_deck_part_position(&mut self, matrix: &M) -> Result<(), E> {
        self.world.try_set_deck_part_position(self.board, matrix)
    }
    fn deck_velocity(&mut self) -> Result<[f32; 3], E> {
        self.world.try_deck_velocity(self.board)
    }
    fn set_deck_body_velocity(&mut self, velocity: [f32; 3]) -> Result<(), E> {
        self.world.try_store_post_solve_deck_velocity(self.board, velocity)
    }
    fn ground_transform(&mut self) -> Result<M, E> {
        let reckoning = self.owner.reckoning().lock().map_err(|_| E::Poisoned)?;
        Ok(*reckoning
            .reckoning()
            .ok_or(E::Missing("actual Reckoning"))?
            .ground_transform())
    }
    fn set_board_transform(&mut self, matrix: M) -> Result<(), E> {
        self.world
            .try_set_board_transform_typed(self.board, matrix, &self.owner.inputs().cvars)
    }
}
