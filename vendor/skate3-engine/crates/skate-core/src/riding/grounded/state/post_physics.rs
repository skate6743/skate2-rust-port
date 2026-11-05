//! `PhysState_PhysicsGround::UpdatePostPhysics` (`82D387A8`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundPostPhysicsStage {
    CheckForGroundWipeout,
    UpdateBoardPath,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundPostPhysicsError<E> {
    pub stage: GroundPostPhysicsStage,
    pub source: E,
}

/// Wipeout::CheckForGroundWipeout82D8F9E0 and board path82C05EC0.
/// There is deliberately no empty implementation.
pub trait GroundPostPhysicsServices {
    type Error;

    fn check_for_ground_wipeout_82d8f9e0(&mut self) -> Result<(), Self::Error>;
    fn update_board_path_82c05ec0(&mut self) -> Result<(), Self::Error>;
}

pub fn update_post_physics<S: GroundPostPhysicsServices>(
    offboard_trajectory_state_1776: i32,
    services: &mut S,
) -> Result<(), GroundPostPhysicsError<S::Error>> {
    services
        .check_for_ground_wipeout_82d8f9e0()
        .map_err(|source| GroundPostPhysicsError {
            stage: GroundPostPhysicsStage::CheckForGroundWipeout,
            source,
        })?;
    if offboard_trajectory_state_1776 < 0 {
        services
            .update_board_path_82c05ec0()
            .map_err(|source| GroundPostPhysicsError {
                stage: GroundPostPhysicsStage::UpdateBoardPath,
                source,
            })?;
    }
    Ok(())
}
