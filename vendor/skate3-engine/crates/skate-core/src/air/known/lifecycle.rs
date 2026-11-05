//! TU3 KnownAir entry and exit lifecycle.

use super::{
    data::{
        KnownAirFrame, KnownAirModeSettings, KnownAirReckoningFields, KnownAirSettings,
        KnownAirState,
    },
    runtime::KnownAirRuntime,
    trajectory::{init_trajectory_info, restore_velocity},
};

const ZERO_VECTOR: [f32; 4] = [0.0; 4]; // 0x830BD350.

/// `PhysState_KnownAir::Enter`, TU3 `0x82D352D0`.
pub fn enter(
    state: &mut KnownAirState,
    frame: &KnownAirFrame,
    settings: &KnownAirSettings,
    mode: &KnownAirModeSettings,
    runtime: &mut impl KnownAirRuntime,
) {
    runtime.enable_board_angular_drive_only();
    runtime.set_air_collision_update_enabled(true);
    runtime.set_skeleton_collision_state(7);
    runtime.set_skeleton_physics_truck_tilt_enabled(false);
    runtime.set_skeleton_inverse_kinematics_enabled(true);

    state.body_flipping_211 = false;
    state.grind_air_adjust_activated_212 = false;
    state.targeting_grind_213 = false;
    runtime.reset_reckoning_flipping();
    runtime.set_footplant_flag_240(false);
    runtime.reset_footplants();

    state.reached_apex_208 = false;
    state.time_in_state_180 = 0.0;
    state.start_y_184 = frame.start_height_484;
    state.max_y_188 = runtime.board_transform_height();
    state.com_max_y_192 = runtime.skeleton_com_position()[1];

    init_trajectory_info(state, frame, settings, mode, runtime);

    if runtime.selector_targeting_grind() {
        runtime.start_grind_air_adjust_from_selector();
        state.targeting_grind_213 = true;
    }

    let skeleton_com = runtime.skeleton_com_position();
    let (index, offset) = runtime.selector_closest_trajectory_point(skeleton_com, ZERO_VECTOR);
    state.trajectory_index_216 = index;
    state.trajectory_follow_offset_144 = offset;

    state.start_flipped_210 = 0.0 > runtime.dot3(frame.velocity_400, frame.start_flip_reference_96);
}

/// `PhysState_KnownAir::Exit`, TU3 `0x82D35920`.
pub fn exit(
    state: &mut KnownAirState,
    frame: &mut KnownAirFrame,
    settings: &KnownAirSettings,
    reckoning: &mut KnownAirReckoningFields,
    runtime: &mut impl KnownAirRuntime,
) {
    runtime.reset_reckoning_flipping();
    if frame.next_physics_state_2500 == 100 {
        restore_velocity(state, frame, settings, runtime);
    }
    runtime.reset_trajectory_selector();
    reckoning.body_spin_speed_1572 = 0.0;
    reckoning.body_spin_angle_1568 = 0.0;
    runtime.set_grind_air_adjust_started(false);
    runtime.set_grind_air_adjust_activated(false);
}
