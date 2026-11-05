use super::{Frame, Requests, Settings};
use crate::physics::{board_motion_output::inverse_length_squared, native_arithmetic::dot3};
pub(super) fn length(v: [f32; 4]) -> f32 {
    let sq = dot3(v, v);
    let inv = inverse_length_squared(sq, 2);
    if sq == 0.0 { 0.0 } else { sq * inv }
}
fn selected_max(a: f32, b: f32) -> f32 {
    if a - b >= 0.0 { a } else { b }
}
///82BD88A0 reads unweighted regional forces in this exact pair order.
pub(crate) fn force(frame: &Frame, body: f32, arms: f32) -> bool {
    force_values(frame.regions_force, body, arms)
}
pub(super) fn force_values(f: [f32; 8], body: f32, arms: f32) -> bool {
    let a = selected_max(f[0], f[1]);
    let b = selected_max(f[5], f[4]);
    let c = selected_max(f[7], f[6]);
    selected_max(selected_max(a, b), c) > body || selected_max(f[2], f[3]) > arms
}
pub(super) fn vehicle(state: &mut Requests, s: &Settings, f: &Frame) {
    let limit = if f.flags_2476 & 8 != 0 {
        s.ground.skitch_contact
    } else {
        s.ground.vehicle_contact
    };
    if f.vehicle_force > limit {
        state.request(7, 0.0);
    }
}
///82D90AB8 rotates closing velocity656 by the actual WorldToAnim basis.
pub(super) fn closing(state: &mut Requests, f: &Frame, xz: f32, y: f32) {
    if !f.board_contact {
        return;
    }
    closing_values(
        f.closing_velocity,
        f.world_to_animation,
        f.regions_force,
        xz,
        y,
        &mut |reason, value| state.request(reason, value),
    );
}
pub(super) fn closing_values(
    v: [f32; 4],
    m: [[f32; 4]; 4],
    forces: [f32; 8],
    xz: f32,
    y: f32,
    request: &mut impl FnMut(usize, f32),
) {
    let local: [f32; 4] =
        std::array::from_fn(|i| m[2][i].mul_add(v[2], m[1][i].mul_add(v[1], m[0][i] * v[0])));
    if length([local[0], 0.0, local[2], local[3]]) > xz || local[1].abs() > y {
        request(2, 0.0);
        if force_values(forces, 1.0, 20.0) {
            request(0, 0.0);
        }
    }
}
pub(super) fn leaning(state: &mut Requests, s: &Settings, f: &Frame) {
    if f.animation_up[1] < s.lean_contact_y
        && f.compliant
        && f.highest_normal[1] > f32::from_bits(0x3f34_fdf4)
    {
        state.request(19, 0.0);
    }
}
