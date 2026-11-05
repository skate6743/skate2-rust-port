//! Footplant branch embedded in TU3 KnownAir update.

use super::{
    data::{KnownAirFootplantInput, KnownAirFrame, KnownAirState},
    runtime::KnownAirRuntime,
};

const MIN_FOOTPLANT_TIME: f32 = f32::from_bits(0x3DCC_CCCD); // 0x820641A8.

/// `0x82D36728`, preserving the reset gate and the three update-call order.
pub(crate) fn update_footplant(
    state: &KnownAirState,
    frame: &KnownAirFrame,
    runtime: &mut impl KnownAirRuntime,
) {
    if frame.flags_2480 & 0x0400_0000 == 0
        || frame.flags_2476 & 0x0400_0000 != 0
        || state.time_in_state_180 <= MIN_FOOTPLANT_TIME
    {
        runtime.reset_footplants();
        return;
    }

    let mut input = KnownAirFootplantInput {
        trajectory: runtime.selector_trajectory_2704(),
        remaining_collision_time: state.collision_time_196 - state.time_in_state_180,
        collision_position: state.collision_position_112,
        landing_normal: state.landing_normal_64,
    };
    runtime.shift_footplant_trajectory_to_index(&mut input.trajectory, state.trajectory_index_216);
    runtime.update_footplant_prediction(&input);
    runtime.update_footplant_lock(&input);
    runtime.update_footplant_pose(&input);
}
