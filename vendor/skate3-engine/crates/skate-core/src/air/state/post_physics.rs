//! TU3 PhysicsAir post-physics update (`0x82D34DD8`).

use super::{data::PhysicsAirState, runtime::PhysicsAirRuntime};

/// Latches the first strict downward board velocity, then always runs the air
/// wipeout check. Negative zero and NaN do not mark the apex.
pub fn update_post_physics(state: &mut PhysicsAirState, runtime: &mut impl PhysicsAirRuntime) {
    if !state.reached_apex {
        let board_velocity = runtime.board_body_velocity();
        if board_velocity[1] < 0.0 {
            state.reached_apex = true;
        }
    }
    runtime.check_for_air_wipeout(false);
}
