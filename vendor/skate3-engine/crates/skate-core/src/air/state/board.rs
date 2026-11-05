//! TU3 PhysicsAir board update (`0x82D34AB0`).

use super::{
    data::{AirBoardForce, PhysicsAirFrame, PhysicsAirReckoningFields, PhysicsAirState},
    jump_velocity::calculate_velocity_from_jump,
    runtime::PhysicsAirRuntime,
};

const AIR_COLLISION_FORCE_TAG: u32 = 15;

pub fn update_skateboard(
    state: &PhysicsAirState,
    frame: &PhysicsAirFrame,
    reckoning: &PhysicsAirReckoningFields,
    runtime: &mut impl PhysicsAirRuntime,
) {
    runtime.update_board_steering_tilt(0.0);

    let mut force = AirBoardForce::ZERO;
    let collision_force_created = runtime
        .calculate_air_collision_force(reckoning.collision_reference_normal_1216, &mut force);
    if collision_force_created {
        runtime.enqueue_board_force(AIR_COLLISION_FORCE_TAG, force);
        return;
    }

    if !state.use_centre_of_mass_velocity && frame.frames_since_jump_correction_2576 < 12 {
        let velocity =
            calculate_velocity_from_jump(frame, frame.frames_since_jump_correction_2576, runtime);
        runtime.set_board_velocity(velocity);
    }
}
