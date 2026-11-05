//! Reuse ordinary native-arithmetic leaves; no new physical solver.
use super::{DT, Vector};
use crate::air::trajectory::Trajectory;
pub(super) use crate::player::wipeout_state::math::{
    add, cross, dot, length, madd, normalize, normalize_or, reciprocal, scale, sub, wrap_angle,
};
pub(super) const UP: Vector = [0., 1., 0., 0.];
pub(super) fn flatten(mut v: Vector) -> Vector {
    v[1] = 0.;
    v
}
pub(super) fn shift(t: &mut Trajectory, time: f32) {
    let p = t.position_at(time);
    t.velocity = t.velocity_at(time);
    t.position = p;
}
///82D609E0: velocity correction only, bounded at the caller's maximum.
pub(super) fn adjust(t: &mut Trajectory, frame: i32, delta: Vector, maximum: f32) {
    if frame <= 0 {
        return;
    }
    let correction = scale(delta, reciprocal(frame as f32 * DT));
    let scalar = if dot(correction, correction) > maximum * maximum {
        maximum / length(correction)
    } else {
        1.
    };
    t.velocity = madd(correction, scalar, t.velocity);
}
