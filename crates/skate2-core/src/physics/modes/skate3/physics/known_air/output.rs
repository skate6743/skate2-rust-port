//! Fill82D36880 overwrites every field below except the explicitly retained pair.
use skate3_core::{
    air::known::{KnownAirOutput, KnownAirTrajectory},
    player::input_phase::AirOutputFields,
};
pub(super) fn storage(previous: &AirOutputFields) -> KnownAirOutput {
    KnownAirOutput {
        trajectory_apex_0: [0.0; 4],
        collision_position_16: [0.0; 4],
        landing_normal_32: [0.0; 4],
        selector_vector_48: [0.0; 4],
        trajectory_position_64: [0.0; 4],
        landing_heading_80: [0.0; 4],
        selector_com_position_96: [0.0; 4],
        landing_normal_copy_144: [0.0; 4],
        locked_trajectory_velocity_160: previous.vector_160.map(f32::from_bits),
        time_in_state_176: 0.0,
        collision_time_180: 0.0,
        time_until_collision_184: 0.0,
        collision_normal_speed_188: 0.0,
        time_to_apex_196: 0.0,
        jump_height_200: 0.0,
        trajectory_index_220: 0,
        selected_trajectory_240: KnownAirTrajectory {
            position: [0.0; 4],
            velocity: [0.0; 4],
            acceleration: [0.0; 4],
            scalar_48: 0.0,
            word_52: 0,
            word_56: 0,
            word_60: 0,
        },
        trajectory_plane_samples_336: [0.0; 25],
        reached_apex_436: false,
        known_air_valid_437: false,
        locked_trajectory_velocity_valid_452: previous.use_air_reckoning_452 != 0,
    }
}
pub(super) fn publish(out: &KnownAirOutput, physical: &mut AirOutputFields) {
    physical.trajectory_apex_0 = out.trajectory_apex_0.map(f32::to_bits);
    physical.collision_position_16 = out.collision_position_16.map(f32::to_bits);
    physical.landing_normal_32 = out.landing_normal_32.map(f32::to_bits);
    physical.selector_vector_48 = out.selector_vector_48.map(f32::to_bits);
    physical.trajectory_position_64 = out.trajectory_position_64.map(f32::to_bits);
    physical.landing_heading_80 = out.landing_heading_80.map(f32::to_bits);
    physical.selector_com_position_96 = out.selector_com_position_96.map(f32::to_bits);
    physical.time_in_state_176 = out.time_in_state_176;
    physical.collision_time_180 = out.collision_time_180;
    physical.collision_normal_speed_188 = out.collision_normal_speed_188;
    physical.time_to_apex_196 = out.time_to_apex_196;
    physical.trajectory_index_220 = out.trajectory_index_220;
    physical.trajectory_plane_samples_336 = out.trajectory_plane_samples_336;
    physical.known_air_valid_437 = u8::from(out.known_air_valid_437);
    let t = out.selected_trajectory_240;
    physical.selected_trajectory_240 = [
        t.position.map(f32::to_bits),
        t.velocity.map(f32::to_bits),
        t.acceleration.map(f32::to_bits),
        [t.scalar_48.to_bits(), t.word_52, t.word_56, t.word_60],
    ];

    physical.vector_160 = out.locked_trajectory_velocity_160.map(f32::to_bits);
    physical.use_air_reckoning_452 = u8::from(out.locked_trajectory_velocity_valid_452);
    physical.scalar_184 = out.time_until_collision_184;
    physical.landing_normal_144 = out.landing_normal_copy_144.map(f32::to_bits);
    physical.jump_height_200 = out.jump_height_200;
    physical.reached_apex_436 = u8::from(out.reached_apex_436);
}
