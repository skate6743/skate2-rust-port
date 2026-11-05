//! KnownAir trajectory setup, ground restoration, and follow update.

use super::{
    data::{
        KnownAirFrame, KnownAirModeSettings, KnownAirSettings, KnownAirState, KnownAirTrajectory,
        Vector4,
    },
    runtime::KnownAirRuntime,
};

const FIXED_STEP: f32 = f32::from_bits(0x3C88_8889); // 0x820849C8, 1/60.
const HALF: f32 = f32::from_bits(0x3F00_0000); // 0x8209975C.
const ONE: f32 = f32::from_bits(0x3F80_0000); // 0x8231A844.
const PERFECT_FLIP_NUMERATOR: f32 = f32::from_bits(0xC0F1_463B); // 0x822F9450.
const NEGATIVE_TWO_PI: f32 = f32::from_bits(0xC0C9_0FDB); // 0x822F9434.
const HEADING_LENGTH_SQUARED_MIN: f32 = f32::from_bits(0x3DCC_CCCD); // 0x820641A8.

/// Perfect-mode branch of InitTrajectoryInfo82D35508; consumed collision time.
pub fn perfect_body_flip_target(collision_time: f32) -> f32 {
    PERFECT_FLIP_NUMERATOR / collision_time
}

/// `PhysState_KnownAir::InitTrajectoryInfo`, TU3 `0x82D35508`.
pub fn init_trajectory_info(
    state: &mut KnownAirState,
    frame: &KnownAirFrame,
    settings: &KnownAirSettings,
    mode: &KnownAirModeSettings,
    runtime: &mut impl KnownAirRuntime,
) {
    state.landing_normal_64 = frame.selector_landing_normal_2656;

    let prediction = runtime.selected_prediction();
    let (apex, time_to_apex) = runtime.highest_trajectory_position(&prediction.trajectory);
    state.time_to_apex_200 = time_to_apex;
    state.trajectory_apex_96 = apex;

    state.collision_position_112 = runtime.selector_contact_position();
    //82D35580 reads r30+48; r30 is the complete winning result.
    state.collision_time_196 = prediction.collision_time_48;
    state.body_flip_target_speed_204 = if mode.perfect_body_flips_28 {
        perfect_body_flip_target(state.collision_time_196)
    } else {
        (settings.flip_scalar / state.collision_time_196) * NEGATIVE_TWO_PI
    };
    state.selector_vector_128 = runtime.selector_vector_2848();

    let mut heading = [0.0; 4];
    let mut heading_valid = false;
    state.collision_normal_speed_176 = 0.0;
    if state.collision_time_196 >= 0.0 {
        let collision_frame = prediction.collision_frame_128;
        let previous_time = collision_frame.wrapping_sub(2) as f32 * FIXED_STEP;
        let collision_time = collision_frame.wrapping_sub(1) as f32 * FIXED_STEP;
        let previous = trajectory_position(&prediction.trajectory, previous_time);
        let collision = trajectory_position(&prediction.trajectory, collision_time);
        let delta = sub(collision, previous);
        let velocity = runtime.divide_vector_by_scalar(delta, FIXED_STEP);

        state.collision_normal_speed_176 = runtime.dot3(state.landing_normal_64, velocity);
        let normal_velocity = mul_scalar(state.landing_normal_64, state.collision_normal_speed_176);
        let tangent = sub(velocity, normal_velocity);
        let (normalized, length) = runtime.normalize_safe_with_length(tangent);
        if !(settings.min_target_heading_velocity_420 > length) {
            heading = normalized;
            heading_valid = true;
        }
    }

    state.landing_heading_80 = heading;
    state.landing_heading_valid_209 = heading_valid;
    let heading_length_squared = runtime.dot3(heading, heading);
    if HEADING_LENGTH_SQUARED_MIN > heading_length_squared {
        state.landing_heading_valid_209 = false;
    }
}

/// Ground transition velocity restoration, TU3 `0x82D35998`.
pub fn restore_velocity(
    state: &mut KnownAirState,
    frame: &mut KnownAirFrame,
    settings: &KnownAirSettings,
    runtime: &mut impl KnownAirRuntime,
) {
    let prediction = runtime.selected_prediction();
    if state.trajectory_index_216 == 0 {
        state.trajectory_index_216 = 1;
    }

    let sample_time = state.trajectory_index_216 as f32 * FIXED_STEP;
    let trajectory_velocity = trajectory_velocity(&prediction.trajectory, sample_time);
    let geometry =
        runtime.restore_velocity_geometry_82d35998(trajectory_velocity, frame.ground_normal_464);
    let curve_value = settings
        .landing_speed_scalar_vs_ground_normal_y_240
        .evaluate(geometry.landing_speed_curve_input);
    let lower_clamped = select(
        -geometry.landing_speed_blend_source,
        0.0,
        geometry.landing_speed_blend_source,
    );
    let blend_weight = select(ONE - lower_clamped, lower_clamped, ONE);
    let speed_scale = (curve_value - ONE).mul_add(blend_weight, ONE);
    let scaled_tangent = mul_scalar(geometry.tangential_velocity, speed_scale);
    let current_board_velocity = runtime.board_body_velocity();
    let preserved_normal_speed = runtime.dot3(current_board_velocity, frame.ground_normal_464);
    let restored = core::array::from_fn(|lane| {
        frame.ground_normal_464[lane].mul_add(preserved_normal_speed, scaled_tangent[lane])
    });
    runtime.set_board_velocity(restored);

    let forward = runtime.board_forward_axis();
    frame.forward_speed_2612 = runtime.dot3(restored, forward);
}

/// `PhysState_KnownAir::UpdateTrajectoryFollow`, TU3 `0x82D36460`.
pub fn update_trajectory_follow(
    state: &mut KnownAirState,
    frame: &KnownAirFrame,
    settings: &KnownAirSettings,
    runtime: &impl KnownAirRuntime,
) {
    state.trajectory_index_216 = state.trajectory_index_216.wrapping_add(1);

    let ratio = (settings.trajectory_error_blend_away_time_384 - state.time_in_state_180)
        / settings.trajectory_error_blend_away_time_384;
    let lower_clamped = select(-ratio, 0.0, ratio);
    let blend = select(ONE - lower_clamped, lower_clamped, ONE);
    let sample_time = state.trajectory_index_216 as f32 * FIXED_STEP;
    let trajectory = runtime.selector_trajectory_2704();
    let trajectory_position = trajectory_position(&trajectory, sample_time);
    state.target_com_position_160 = core::array::from_fn(|lane| {
        state.trajectory_follow_offset_144[lane].mul_add(blend, trajectory_position[lane])
    });

    if frame.flags_2468 & 0x0000_0008 != 0 {
        state.trajectory_index_216 = state.trajectory_index_216.wrapping_sub(1);
    }
}

pub(crate) fn trajectory_position(trajectory: &KnownAirTrajectory, time: f32) -> Vector4 {
    let time_squared = time * time;
    core::array::from_fn(|lane| {
        let linear = trajectory.velocity[lane].mul_add(time, trajectory.position[lane]);
        let half_acceleration = trajectory.acceleration[lane] * HALF;
        half_acceleration.mul_add(time_squared, linear)
    })
}

pub(crate) fn trajectory_velocity(trajectory: &KnownAirTrajectory, time: f32) -> Vector4 {
    core::array::from_fn(|lane| {
        trajectory.acceleration[lane].mul_add(time, trajectory.velocity[lane])
    })
}

pub(crate) fn add(left: Vector4, right: Vector4) -> Vector4 {
    core::array::from_fn(|lane| left[lane] + right[lane])
}

pub(crate) fn sub(left: Vector4, right: Vector4) -> Vector4 {
    core::array::from_fn(|lane| left[lane] - right[lane])
}

pub(crate) fn mul_scalar(vector: Vector4, scalar: f32) -> Vector4 {
    vector.map(|lane| lane * scalar)
}

pub(crate) fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}
