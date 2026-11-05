//! Full outer TU3 KnownAir update in recovered call order.

use super::{
    body::{calculate_body_flip_speed, calculate_body_spin_speed},
    data::{
        GrindFootBody, KnownAirFrame, KnownAirModeSettings, KnownAirReckoningFields,
        KnownAirSettings, KnownAirState,
    },
    footplant::update_footplant,
    runtime::KnownAirRuntime,
    trajectory::{init_trajectory_info, select, update_trajectory_follow},
};

const ALIGNMENT_LEAD_FRAMES: f32 = f32::from_bits(0x4040_0000); // 0x82063B08, 3.
const MIN_ALIGNMENT_FRAMES: f32 = f32::from_bits(0x4000_0000); // 0x82060C50, 2.
const ONE: f32 = f32::from_bits(0x3F80_0000); // 0x8231A844.
const BOARD_ERROR_WINDOW: f32 = f32::from_bits(0x3D75_C28F); // 0x82099460, 0.06.
const ZERO: f32 = 0.0; // 0x82165A10.

const GRIND_FOOT_BODIES: [GrindFootBody; 6] = [
    GrindFootBody::Body3152,
    GrindFootBody::Body3156,
    GrindFootBody::Body3160,
    GrindFootBody::Body3168,
    GrindFootBody::Body3172,
    GrindFootBody::Body3176,
];

/// `PhysState_KnownAir::Update`, TU3 `0x82D35C58`.
pub fn update(
    state: &mut KnownAirState,
    frame: &KnownAirFrame,
    settings: &KnownAirSettings,
    mode: &KnownAirModeSettings,
    reckoning: &KnownAirReckoningFields,
    runtime: &mut impl KnownAirRuntime,
) {
    update_footplant(state, frame, runtime);
    if runtime.selector_has_just_changed() {
        init_trajectory_info(state, frame, settings, mode, runtime);
    }
    runtime.update_air_collision();

    let body_flip_speed =
        calculate_body_flip_speed(state, frame, settings, mode, reckoning, runtime);
    let body_spin_speed =
        calculate_body_spin_speed(state, frame, settings, mode, reckoning, runtime);

    let remaining_frames =
        (state.collision_time_196 - state.time_in_state_180) / frame.delta_time_2604;
    let after_lead = remaining_frames - ALIGNMENT_LEAD_FRAMES;
    let alignment_frames = select(
        after_lead - MIN_ALIGNMENT_FRAMES,
        after_lead,
        MIN_ALIGNMENT_FRAMES,
    );

    if !state.grind_air_adjust_activated_212
        && (state.reached_apex_208 || alignment_frames < settings.frames_for_grind_air_assist_436)
    {
        let started = runtime.grind_air_adjust_started();
        runtime.set_grind_air_adjust_activated(started);
        state.grind_air_adjust_activated_212 = true;
    }

    if state.targeting_grind_213
        && state.grind_air_adjust_activated_212
        && runtime.grind_air_adjust_adjusting()
    {
        for body in GRIND_FOOT_BODIES {
            runtime.disable_grind_foot_collision(body);
        }
    }

    runtime.update_reckoning_air_states(
        state.landing_normal_64,
        ONE / alignment_frames,
        body_spin_speed,
        body_flip_speed,
    );
    update_trajectory_follow(state, frame, settings, runtime);
    runtime.update_known_air_skeleton(state.target_com_position_160);

    if state.time_in_state_180 < BOARD_ERROR_WINDOW {
        runtime.set_skeleton_add_skateboard_error(true);
    }

    let head_target = if frame.flags_2472 & 0x0040_0000 != 0 {
        frame.alternate_head_target_112
    } else {
        state.collision_position_112
    };
    runtime.set_head_tracking_target(head_target, true);
    runtime.update_board_steering_tilt(ZERO);

    let board_y = runtime.board_transform_height();
    if !(state.max_y_188 > board_y) {
        state.max_y_188 = board_y;
    }
    if frame.flags_2468 & 0x0000_0008 == 0 {
        state.time_in_state_180 += frame.delta_time_2604;
    }

    let com_y = runtime.skeleton_com_position()[1];
    state.com_max_y_192 = select(state.com_max_y_192 - com_y, state.com_max_y_192, com_y);
}
