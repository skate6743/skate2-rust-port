//! TU3 KnownAir post-physics and upside-down wipeout handling.

use super::{
    data::{
        KnownAirFrame, KnownAirModeSettings, KnownAirState, KnownAirWipeoutRequest,
        KnownAirWipeoutSettings,
    },
    runtime::KnownAirRuntime,
};

/// `PhysState_KnownAir::UpdatePostPhysics`, TU3 `0x82D36590`.
pub fn update_post_physics(
    state: &mut KnownAirState,
    frame: &KnownAirFrame,
    mode: &KnownAirModeSettings,
    settings: &KnownAirWipeoutSettings,
    request: &mut KnownAirWipeoutRequest,
    runtime: &mut impl KnownAirRuntime,
) {
    if !state.reached_apex_208 {
        let velocity = runtime.board_body_velocity();
        if velocity[1] < 0.0 {
            state.reached_apex_208 = true;
        }
    }
    runtime.check_for_air_wipeout(true);
    check_upside_down_falling_wipeout(state, frame, mode, settings, request, runtime);
}

/// `PhysState_KnownAir::CheckUpsideDownFallingWipeout`, TU3 `0x82D36650`.
pub fn check_upside_down_falling_wipeout(
    state: &KnownAirState,
    frame: &KnownAirFrame,
    mode: &KnownAirModeSettings,
    settings: &KnownAirWipeoutSettings,
    request: &mut KnownAirWipeoutRequest,
    runtime: &mut impl KnownAirRuntime,
) {
    if !mode.upside_down_falling_wipeout_enabled_2 {
        return;
    }
    if frame.flags_2468 & 0x0000_0020 != 0 {
        return;
    }
    if !(frame.skater_up_544[1] < settings.air_falling_min_up_y_260) {
        return;
    }
    if runtime.dot3(state.landing_normal_64, frame.skater_up_544)
        < settings.air_falling_max_angle_264
    {
        request.requested_35 = true;
        request.scalar_116 = 0.0;
        request.counter_200 = request.counter_200.wrapping_add(1);
    }
}
