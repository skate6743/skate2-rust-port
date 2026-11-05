//! Ground carried-board Update82D31D1C..DB0. State60 is SkateboardController;
//! state52 in Sync82D324B0 is a DIFFERENT grab-spline manager.
use super::{Skate3PhysicsState, Skate3ControllerState};

/// Executes original Stop/Hold effects on the existing possession/solver owner.
/// Does not run its common per-frame Update a second time.
pub(crate) fn update_possession(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) {
    let flags = skater.player_input.processed.flags_2484;
    let state = skater.skateboard_controller.fields.state_448;
    let target = if flags & 1 != 0 {
        if matches!(state, 2 | 3 | 4) {
            return;
        }
        5
    } else {
        if state != 5 {
            return;
        }
        1
    };
    skater.skateboard_controller.fields.word_444 = 0;
    if state == target {
        return;
    }
    let observation = super::super::offboard::board_manager::runtime::observe(physics, skater);
    let mut effects = skater.board_possession_live.effects(
        physics,
        &mut skater.ground_lifecycle.board_animated_290,
        skater.player_input.processed.timestep_2604,
    );
    if target == 5 {
        skater.board_possession.stop(
            &mut skater.skateboard_controller.fields,
            &observation,
            &mut effects,
        );
    } else {
        skater.board_possession.hold(
            &mut skater.skateboard_controller.fields,
            &observation,
            &mut effects,
        );
    }
    //82D31D80/DB0 assigns state only AFTER the physical action.
    skater.skateboard_controller.fields.state_448 = target;
    skater.board_possession_live.publish_volumes(physics);
}
