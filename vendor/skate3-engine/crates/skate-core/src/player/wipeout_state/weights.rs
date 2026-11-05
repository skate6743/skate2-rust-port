//! Wipeout Update82D3BE04..82D3BFE8, after the selected physical control.
use super::{State, math};

#[derive(Clone, Copy)]
pub struct Settings {
    pub remove_target_time: f32,
    pub remove_drives_time: f32,
    /// Native352 is a per-update increment, not a per-second velocity.
    pub collision_step: f32,
    /// Native348 is also a per-update increment.
    pub controlled_step: f32,
}

#[derive(Clone, Copy)]
pub struct Output {
    pub target_weight: f32,
    pub start: f32,
    pub end: f32,
    pub controlled: f32,
    pub extra: f32,
}

pub fn update(
    state: &mut State,
    settings: Settings,
    contact: bool,
    control_applied: bool,
    flags_2472: u32,
    gesture: [f32; 2],
) -> Output {
    let target_weight = fade(settings.remove_target_time, state.time);
    if state.special_surface || (gesture[0].abs() < 0.2 && gesture[1].abs() < 0.2) {
        let step = if contact {
            settings.collision_step
        } else {
            -settings.collision_step
        };
        state.collision_weight = math::clamp(state.collision_weight + step, 0.0, 1.0);
    } else {
        state.collision_weight = 0.0;
    }
    let drive_fade = fade(settings.remove_drives_time, state.time);
    let drive_fade_squared = drive_fade * drive_fade;
    let enabled = (control_applied || flags_2472 & 0x0008_0000 != 0) && !state.special_surface;
    let step = if enabled {
        settings.controlled_step
    } else {
        -settings.controlled_step
    };
    state.controlled_weight = math::clamp(state.controlled_weight + step, 0.0, 1.0);
    let available = 1.0 - state.collision_weight;
    let start = available * drive_fade_squared;
    Output {
        target_weight,
        start,
        end: state.collision_weight,
        controlled: (available - start) * state.controlled_weight,
        extra: state.extra_weight * 0.03,
    }
}

fn fade(duration: f32, time: f32) -> f32 {
    let divisor = if duration == 0.0 { 0.01 } else { duration };
    math::clamp((duration - time) / divisor, 0.0, 1.0)
}
