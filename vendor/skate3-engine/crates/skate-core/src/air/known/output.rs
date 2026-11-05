//! TU3 KnownAir PhysOut publication.

use super::{
    data::{KnownAirFrame, KnownAirOutput, KnownAirState},
    runtime::KnownAirRuntime,
    trajectory::{add, sub, trajectory_position, trajectory_velocity},
};

const FIXED_STEP: f32 = f32::from_bits(0x3C88_8889); // 0x820849C8, 1/60.
const MAX_FINITE: f32 = f32::from_bits(0x7F7F_FFFF); // 0x8206D108.

/// `PhysState_KnownAir::FillPhysOut`, TU3 `0x82D36880`.
pub fn fill_physics_output(
    state: &KnownAirState,
    frame: &KnownAirFrame,
    output: &mut KnownAirOutput,
    runtime: &mut impl KnownAirRuntime,
) {
    let prediction = runtime.selected_prediction();
    if frame.flags_2468 & 0x0000_0008 != 0 {
        output.locked_trajectory_velocity_valid_452 = true;
        let time = state.trajectory_index_216 as f32 * FIXED_STEP;
        output.locked_trajectory_velocity_160 = trajectory_velocity(&prediction.trajectory, time);
    }

    output.reached_apex_436 = state.reached_apex_208;
    output.jump_height_200 = state.max_y_188 - state.start_y_184;
    output.trajectory_apex_0 = state.trajectory_apex_96;
    output.time_to_apex_196 = state.time_to_apex_200;
    output.collision_position_16 = state.collision_position_112;
    output.landing_normal_32 = state.landing_normal_64;
    output.landing_normal_copy_144 = state.landing_normal_64;
    output.time_until_collision_184 = if frame.flags_2468 & 0x0000_0008 != 0 {
        MAX_FINITE
    } else {
        state.collision_time_196 - state.time_in_state_180
    };
    output.time_in_state_176 = state.time_in_state_180;
    output.selector_vector_48 = state.selector_vector_128;

    let output_index = state.trajectory_index_216.max(1);
    output.trajectory_position_64 =
        trajectory_position(&prediction.trajectory, output_index as f32 * FIXED_STEP);
    output.landing_heading_80 = state.landing_heading_80;
    output.collision_normal_speed_188 = state.collision_normal_speed_176;
    output.known_air_valid_437 = true;
    output.selected_trajectory_240 = prediction.trajectory;
    output.trajectory_index_220 = state.trajectory_index_216;

    let selector_com = runtime.selector_com_position_2784();
    output.selector_com_position_96 = selector_com;
    output.collision_time_180 = state.collision_time_196;
    let com_transform = runtime.reckoning_com_transform_816();
    let local_com = runtime.transform_point_82d36880(com_transform, selector_com);
    let first_sample = state.trajectory_index_216.wrapping_sub(1).max(0);
    for (sample_offset, destination) in output.trajectory_plane_samples_336.iter_mut().enumerate() {
        let sample_index = first_sample.wrapping_add(sample_offset as i32);
        let sample = trajectory_position(&prediction.trajectory, sample_index as f32 * FIXED_STEP);
        let relative = sub(add(sample, local_com), state.collision_position_112);
        *destination = runtime.dot3(relative, state.landing_normal_64);
    }
}
