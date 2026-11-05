//! TU3 PhysicsAir Enter/Exit/output lifecycle.

use super::{
    data::{
        AirTrajectory, PhysicsAirFrame, PhysicsAirOutput, PhysicsAirReckoningFields,
        PhysicsAirState, Vector4,
    },
    jump_velocity::calculate_velocity_from_jump,
    runtime::PhysicsAirRuntime,
};

const TRAJECTORY_UNBOUNDED: f32 = f32::from_bits(0xBF80_0000); // -1.0
const OFFBOARD_VERTICAL_SCALE: f32 = f32::from_bits(0x3F26_6666); // 0.65
const OFFBOARD_VERTICAL_LIMIT: f32 = f32::from_bits(0x4040_0000); // 3.0
const OUTPUT_SCALAR_184: f32 = f32::from_bits(0x4080_0000); // 4.0

/// Full TU3 `PhysState_PhysicsAir::Enter`, `0x82D34388`.
pub fn enter(
    state: &mut PhysicsAirState,
    frame: &PhysicsAirFrame,
    world_acceleration: Vector4,
    runtime: &mut impl PhysicsAirRuntime,
) {
    runtime.set_air_collision_update_enabled(true);
    runtime.set_skeleton_collision_state(7);
    runtime.set_skeleton_inverse_kinematics_enabled(true);
    runtime.set_skeleton_collision_state(6);
    runtime.enable_board_angular_drive_only();
    runtime.set_footplant_flag_240(false);
    runtime.reset_footplants();

    state.selector_latch_174 = false;
    state.reached_apex = false;
    state.trajectory_query_countdown = 0;
    state.landing_normal = [0.0, 1.0, 0.0, 0.0];
    state.time_in_state = 0.0;
    state.max_y = runtime.board_transform_height();
    state.start_y = frame.ground_position_y_500;

    let (use_centre_of_mass_velocity, request_heading_update) = initial_centre_of_mass_mode(frame);
    state.use_centre_of_mass_velocity = use_centre_of_mass_velocity;

    if use_centre_of_mass_velocity {
        let mut velocity = frame.trajectory_velocity_608;
        if frame.previous_physics_state_2504 == 500 {
            let scaled_y = velocity[1] * OFFBOARD_VERTICAL_SCALE;
            velocity[1] = runtime.minimum_vminfp(scaled_y, OFFBOARD_VERTICAL_LIMIT);
        }
        if frame.frames_since_jump_correction_2576 < 12 {
            velocity = calculate_velocity_from_jump(
                frame,
                frame.frames_since_jump_correction_2576,
                runtime,
            );
        }
        state.centre_of_mass_trajectory = AirTrajectory {
            position: frame.trajectory_position_592,
            velocity,
            acceleration: world_acceleration,
            scalar_48: TRAJECTORY_UNBOUNDED,
        };
    }

    // This branch-local flag deliberately differs from the COM selection in
    // the grind/category-400 path.
    if request_heading_update {
        runtime.request_skeleton_heading_update();
    }
}

/// TU3 `PhysState_PhysicsAir::Exit`, `0x82D346A0`.
pub fn exit(state: &mut PhysicsAirState, reckoning: &mut PhysicsAirReckoningFields) {
    state.use_centre_of_mass_velocity = false;
    reckoning.body_spin_speed_1572 = 0.0;
    reckoning.body_spin_angle_1568 = 0.0;
}

/// TU3 `PhysState_PhysicsAir::FillPhysOut`, `0x82D34E90`.
pub fn fill_physics_output(state: &PhysicsAirState) -> PhysicsAirOutput {
    PhysicsAirOutput {
        is_at_apex: state.reached_apex,
        jump_height: state.max_y - state.start_y,
        landing_normal: state.landing_normal,
        scalar_184_write: state.selector_latch_174.then_some(OUTPUT_SCALAR_184),
    }
}

fn initial_centre_of_mass_mode(frame: &PhysicsAirFrame) -> (bool, bool) {
    if frame.previous_physics_category_2516 == 400 || frame.previous_physics_state_2504 == 701 {
        (frame.flags_2468 & 0x0000_4000 == 0, false)
    } else {
        let selected = !matches!(frame.previous_physics_state_2504, 100 | 103 | 201);
        (selected, selected)
    }
}
