//! Original Wipeout response82D3E678 and retained-velocity82D3E348/82D3E8C0.
use super::{
    State, body, drives,
    math::{self, V},
};
use crate::physics::skeleton_body::{SkeletonBody, SkeletonDrives};
///82D3C2B0: only upward collision/prediction normals are accepted.
pub fn response_normal(support: Option<V>, prediction: Option<V>) -> V {
    if let Some(normal) = support.filter(|n| n[1] > 0.0) {
        return normal;
    }
    prediction
        .filter(|n| n[1] > 0.0)
        .unwrap_or([0.0, 1.0, 0.0, 0.0])
}
pub fn trigger(state: &mut State, body: &mut SkeletonBody, normal: V, input: [f32; 2]) {
    state.response_frames = 0;
    state.response_start_speed = math::length(state.velocity);
    let strength = state.response_strength();
    if state.maximum_speed * 0.1 < 1.0 {
        state.response_count = state.response_count.wrapping_add(1);
    } else {
        state.response_count = 0;
    }
    state.maximum_speed = 0.0;
    state.extra_weight = 0.0;
    state.response_scalar = 0.0;
    if strength == 0.0 {
        return;
    }
    state.response_time = 0.0;
    let strength = if strength - 1.0 >= 0.0 { 1.0 } else { strength };
    state.response_scalar = strength;
    state.extra_weight = strength;
    let mut control = [input[0], 0.0, input[1], 0.0];
    let projection = math::dot(normal, control);
    if projection < 0.0 {
        control = math::sub(control, math::scale(normal, projection));
    }
    //82D3E868: raw vmadd A=normal,C=zero,B=control*1.25.
    let velocity = math::scale(
        math::madd(normal, 0.0, math::scale(control, 1.25)),
        strength,
    );
    body::add_velocity(body, velocity);
}
pub fn update_counter(state: &mut State, contact: bool) {
    state.response_finished = false;
    if state.response_frames < 0 {
        return;
    }
    state.response_frames = state.response_frames.wrapping_add(1);
    if state.response_frames == 3 {
        let factor = if state.response_start_speed < 1.0 {
            1.0 - state.response_start_speed
        } else {
            state.response_start_speed * 0.2
        };
        state.response_change =
            (math::length(state.velocity) - state.response_start_speed) * (factor + 1.0);
    }
    if state.response_frames == 8 {
        state.response_frames = -1;
        state.response_finished = true;
        if !contact {
            state.response_change = state.response_change.abs() * 2.0;
        }
        state.response_change = math::clamp((state.response_change - 0.1) * 0.2, 0.0, 1.0);
    }
}
pub fn retained_velocity(
    state: &mut State,
    body: &mut SkeletonBody,
    targets: &mut SkeletonDrives,
    settings: &drives::Settings,
    contact: bool,
    flags2468: u32,
    com_velocity: V,
) {
    let requested = flags2468 & 8 != 0;
    if state.retained_velocity_active || requested {
        if contact {
            state.allow_retained_velocity = false;
            release(state, body, targets, settings);
        } else if !requested {
            release(state, body, targets, settings);
        } else if !state.retained_velocity_active
            && state.allow_retained_velocity
            && com_velocity[1] < 2.0
        {
            drives::set_linear_root(targets, settings, 1.0);
            state.retained_velocity_active = true;
            state.retained_velocity = state.velocity;
        }
    }
    if state.retained_velocity_active {
        state.velocity = state.retained_velocity;
        state.time -= f32::from_bits(0x3C88_8889);
    }
}
fn release(
    state: &mut State,
    body: &mut SkeletonBody,
    targets: &mut SkeletonDrives,
    settings: &drives::Settings,
) {
    if !state.retained_velocity_active {
        return;
    }
    drives::set_linear_root(targets, settings, 0.0);
    body::set_velocity(body, state.retained_velocity);
    state.retained_velocity_active = false;
    state.velocity = [0.0; 4];
}
