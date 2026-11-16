//! The solver-result checks that open Skateboard::UpdatePostPhysics 82C02138, ahead of the
//! SkateboardBody update 82C07D20.
use super::{Skate3PhysicsState, ground_phase, skater::Skate3ControllerState};
use skate3_core::{
    math::{Basis3, Vector3},
    physics::{board::BodyId, drive_frames::RetailAffineTransform},
};

/// What Skateboard::UpdatePostPhysics returns to Update4 82DB6CF0.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SolverResult {
    /// 0: the board's own update follows.
    Accepted,
    /// 1: thrown upward by the solver and put back at rest.
    Exploded,
    /// 2: left without a transform, put back with a respawn asked for.
    BadData,
}

pub(super) fn check(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
) -> Result<SolverResult, String> {
    let toolkit = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Board solver-result check requires the current board toolkit")?;
    let (deck, effective) = (toolkit.deck, toolkit.effective);
    // 82C02160..82C02198. SkateboardBody::IsBadData 82C07B78 is a NaN anywhere in the deck
    // part's transform. The board asks for a respawn (Skateboard+0x121, which FillPhysOut
    // 82C03600..3618 turns into State+69) and goes back to this tick's deck transform
    // (Processed+64): InitializeTransforms, then TeleportReset 82C05F50.
    let solved = physics.board.part_transforms()[BodyId::Deck.index()];
    let translation = [solved.translation.x, solved.translation.y, solved.translation.z];
    if solved.basis.columns.iter().flatten().chain(&translation).any(|value| value.is_nan()) {
        skater.ground_lifecycle.board_wants_respawn_289 = true;
        teleport_reset(physics, skater, deck);
        return Ok(SolverResult::BadData);
    }
    // 82C021D0..82C0230C. A deck that leaves the solver rising faster than 25 m/s (0x8209994C)
    // after a tick whose published velocity (Processed+400) rose slower than 5 m/s (0x821F1790)
    // goes back to this tick's effective transform (Processed+128) at rest: InitializeTransforms,
    // TeleportReset 82C05F50, then every part's velocity and the deck's angular velocity cleared.
    // A board the animation drives (Skateboard+0x122) is exempt.
    let rise = physics.board.bodies()[BodyId::Deck.index()].rates.linear_velocity.y;
    let published = f32::from_bits(skater.player_input.processed.vectors_400_416[0][1]);
    if !(rise > 25.0 && 5.0 > published) || skater.ground_lifecycle.board_animated_290 != 0 {
        return Ok(SolverResult::Accepted);
    }
    teleport_reset(physics, skater, effective);
    Ok(SolverResult::Exploded)
}

/// InitializeTransforms and TeleportReset 82C05F50 with the board's own update 82C07D20 left
/// out: ProcessOutput 82C02A80 publishes the rates the reset left.
fn teleport_reset(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    transform: [[f32; 4]; 4],
) {
    let flags_2468 = skater.player_input.processed.flags_2468;
    physics.board.reset_physical(
        physics.settings.authored,
        RetailAffineTransform {
            basis: Basis3 {
                columns: std::array::from_fn(|i| [transform[i][0], transform[i][1], transform[i][2]]),
            },
            translation: Vector3::new(transform[3][0], transform[3][1], transform[3][2]),
        },
        flags_2468,
        physics.settings.step.simulation.gravity_acceleration,
    );
    ground_phase::reset_board_state(
        &mut skater.ground,
        &mut skater.ground_runtime,
        &mut skater.ground_lifecycle,
        &mut physics.board_wiping_out,
    );
    skater.skeleton_air.reset_board();
    physics.riding.reset_for_teleport();
    physics.riding.finish_post_physics_reset(&physics.board, flags_2468);
}
