//! TU3 PhysicsAir Update (`0x82D346C0`) and COM launch helper (`0x82D34570`).

use crate::point_graph::PointGraph;

use super::{
    board::update_skateboard,
    data::{
        AirTrajectory, PhysicsAirFrame, PhysicsAirReckoningFields, PhysicsAirSettings,
        PhysicsAirState, Vector4,
    },
    jump_velocity::calculate_velocity_from_jump,
    runtime::{PhysicsAirLaunchInfo, PhysicsAirRuntime},
};

const FIXED_STEP: f32 = f32::from_bits(0x3C88_8889); // 1/60, 0x82098D40
const HALF: f32 = f32::from_bits(0x3F00_0000);
const ONE: f32 = f32::from_bits(0x3F80_0000);
const TRAJECTORY_UNBOUNDED: f32 = f32::from_bits(0xBF80_0000);
const TWO: f32 = f32::from_bits(0x4000_0000);
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3C8E_FA35);
const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
const RECIPROCAL_TWO_PI: f32 = f32::from_bits(0x3E22_F983);
const HALF_TURN: f32 = f32::from_bits(0x3F00_0000);
const COM_START_Y_OFFSET: f32 = f32::from_bits(0xBF26_6666); // -0.65
const BOARD_START_Y_OFFSET: f32 = f32::from_bits(0xBF4C_CCCD); // -0.8
const CONE_SCALE: f32 = f32::from_bits(0x3ECC_CCCD); // 0.4

/// Full recovered outer update, retaining native effect order.
pub fn update(
    state: &mut PhysicsAirState,
    frame: &PhysicsAirFrame,
    settings: &PhysicsAirSettings,
    reckoning: &mut PhysicsAirReckoningFields,
    world_acceleration: Vector4,
    runtime: &mut impl PhysicsAirRuntime,
) {
    runtime.update_air_collision();

    state.trajectory_query_countdown = state.trajectory_query_countdown.wrapping_sub(1);
    if state.trajectory_query_countdown <= 0
        && frame.flags_2468 & 0x0000_8000 == 0
        && frame.previous_physics_state_2504 != 0
        && !runtime.trajectory_query_just_started()
    {
        launch_trajectory(state, frame, world_acceleration, runtime);
        state.trajectory_query_countdown = 7;
    }

    runtime.update_trajectory_selector();

    let current_normal = reckoning.current_landing_normal_1152;
    let landing_normal = runtime.selector_landing_normal().unwrap_or(current_normal);
    state.landing_normal = landing_normal;

    let raw_angle = runtime.angle_between_vectors(landing_normal, current_normal);
    let wrapped_angle = wrap_signed_angle(raw_angle);
    let mut normal_blend = if wrapped_angle <= settings.landing_normal_angle_limit_444 {
        settings.landing_normal_blend_388
    } else {
        0.0
    };

    if !state.use_centre_of_mass_velocity {
        if !state.selector_latch_174 {
            state.selector_latch_174 = runtime.selector_centre_of_mass_trajectory_ready();
        }
        if state.selector_latch_174 {
            state.use_centre_of_mass_velocity = true;
            state.centre_of_mass_trajectory = AirTrajectory {
                position: frame.trajectory_position_592,
                velocity: frame.trajectory_velocity_608,
                acceleration: world_acceleration,
                scalar_48: TRAJECTORY_UNBOUNDED,
            };
        }
    }

    if frame.state_timer_2664 <= 0.0 {
        normal_blend = 0.0;
    } else if frame.flags_2468 & 0x0000_4000 != 0 {
        state.use_centre_of_mass_velocity = false;
    }

    let scaled_spin_input = frame.body_spin_input_2640 * settings.body_spin_scale_428;
    let spin_radians = scaled_spin_input * DEGREES_TO_RADIANS;
    let curve = evaluate_spin_curve(&settings.body_spin_over_time_320, state.time_in_state * TWO);
    let target_body_spin = curve * spin_radians;
    *reckoning =
        runtime.update_reckoning_air_states(landing_normal, normal_blend, target_body_spin, 0.0);

    if state.use_centre_of_mass_velocity {
        integrate_trajectory_fixed_step(&mut state.centre_of_mass_trajectory);
        runtime.update_known_air_skeleton(state.centre_of_mass_trajectory.position);
    } else {
        runtime.update_animated_skateboard_skeleton(false);
    }

    update_skateboard(state, frame, reckoning, runtime);
    runtime.enable_skateboard_error_on_skeleton();

    let board_y = runtime.board_transform_height();
    // fcmpu+bgt keeps the old value only for an ordered old > new result;
    // unordered values take the board-height write.
    if !(state.max_y > board_y) {
        state.max_y = board_y;
    }
    state.time_in_state += frame.delta_time_2604;
}

/// Scalar wrap at `0x82D347E8..0x82D34848`, after AngleBetweenVectors.
pub fn wrap_signed_angle(angle: f32) -> f32 {
    let turns = angle * RECIPROCAL_TWO_PI;
    let fraction = turns - turns.floor();
    let signed_fraction = if fraction > HALF_TURN {
        fraction - ONE
    } else {
        fraction
    };
    signed_fraction * TWO_PI
}

/// Fixed 1/60 ballistic step at `0x82D349B8..0x82D34A20`.
pub fn integrate_trajectory_fixed_step(trajectory: &mut AirTrajectory) {
    let step_squared = FIXED_STEP * FIXED_STEP;
    let linear_position: Vector4 = core::array::from_fn(|lane| {
        trajectory.velocity[lane].mul_add(FIXED_STEP, trajectory.position[lane])
    });
    let next_velocity: Vector4 = core::array::from_fn(|lane| {
        trajectory.acceleration[lane].mul_add(FIXED_STEP, trajectory.velocity[lane])
    });
    let half = ONE * HALF;
    let half_acceleration = trajectory.acceleration.map(|lane| lane * half);
    let next_position: Vector4 = core::array::from_fn(|lane| {
        half_acceleration[lane].mul_add(step_squared, linear_position[lane])
    });
    trajectory.position = next_position;
    trajectory.velocity = next_velocity;
}

fn launch_trajectory(
    state: &PhysicsAirState,
    frame: &PhysicsAirFrame,
    world_acceleration: Vector4,
    runtime: &mut impl PhysicsAirRuntime,
) {
    let mut info = runtime.construct_trajectory_launch_info();
    runtime.fill_skeleton_launch_info(&mut info);
    if frame.frames_since_jump_correction_2576 < 12 {
        let velocity =
            calculate_velocity_from_jump(frame, frame.frames_since_jump_correction_2576, runtime);
        info.set_start_velocity(velocity);
    } else if state.use_centre_of_mass_velocity {
        adjust_centre_of_mass_launch_info(state, world_acceleration, runtime, &mut info);
    }

    // The caller always clears +268, including after the COM helper sets it.
    info.set_player_jumped(false);
    runtime.launch_trajectory(&info);
}

fn adjust_centre_of_mass_launch_info<R: PhysicsAirRuntime>(
    state: &PhysicsAirState,
    world_acceleration: Vector4,
    runtime: &mut R,
    info: &mut R::LaunchInfo,
) {
    runtime.fill_skeleton_launch_info(info);

    let velocity = core::array::from_fn(|lane| {
        world_acceleration[lane].mul_add(FIXED_STEP, state.centre_of_mass_trajectory.velocity[lane])
    });
    info.set_start_velocity(velocity);
    info.set_player_jumped(true);

    let animation_position = info.centre_of_mass_animation_position();
    let trajectory_offset = [0.0, COM_START_Y_OFFSET, 0.0, 0.0];
    let board_offset = [0.0, BOARD_START_Y_OFFSET, 0.0, 0.0];
    info.set_trajectory_start_position_override(core::array::from_fn(|lane| {
        animation_position[lane] + trajectory_offset[lane]
    }));
    info.set_board_position_override(core::array::from_fn(|lane| {
        animation_position[lane] + board_offset[lane]
    }));
    info.set_use_trajectory_start_position_override(true);
    info.set_trajectory_count(5);

    let cone_angles = info.cone_angles();
    info.set_cone_angles([cone_angles[0] * CONE_SCALE, cone_angles[1] * CONE_SCALE]);
}

fn evaluate_spin_curve(graph: &PointGraph<8>, input: f32) -> f32 {
    graph.evaluate(input)
}
