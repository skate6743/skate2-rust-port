//! Selected SAME-pair Skeleton stores within `PhysicsGround::Enter`.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
};

#[derive(Debug)]
pub(crate) enum GroundEntrySkeletonError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    MissingIk,
}
impl std::fmt::Display for GroundEntrySkeletonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("ground entry skater pool unavailable"),
            Self::Pair(error) => write!(f, "ground entry pair: {error}"),
            Self::MissingIk => f.write_str("ground entry Skeleton IK not constructed"),
        }
    }
}
impl PhysicalAssemblyWorld {
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_return_native_skater_root(&mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        root: crate::skeleton_root_transforms::SkeletonRootMatrix,
        expected: &std::sync::Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
    ) -> Result<(), String> {
        let effective = crate::dynamic_assembly_session::skater_pool::board_entry::try_current_board_effective_transform(
            self.board(board).map_err(|error|error.to_string())?, &self.backend, expected)
            .map_err(|error|error.to_string())?;
        self.skaters.as_mut().ok_or("native return skater pool")?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, owner| {
                if !std::sync::Arc::ptr_eq(owner, expected) { return Err("Foreign native return root".to_string()); }
                normal.accept_mode_return_root(root, effective);
                Ok(())
            }).map_err(|error|error.to_string())?
    }
    /// Source 0x82d99c64..9c74, between actual board `SetStandard` and actual
    /// `SkeletonController` transition. No volume/body/pose publication changes.
    pub(crate) fn try_ground_entry_skeleton_ik_and_tilt(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), GroundEntrySkeletonError> {
        use GroundEntrySkeletonError as E;
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _owner| {
                normal
                    .try_ground_entry_ik_and_tilt()
                    .map_err(|_| E::MissingIk)
            })
            .map_err(E::Pair)?
    }
}
