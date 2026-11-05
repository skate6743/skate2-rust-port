//! Recovered leaf expressions. Ordinary PC estimates retain native refinements;
//! they are not claimed to be bit-exact Xenon arithmetic.
use super::Vector;
pub(in crate::player::offboard) use crate::player::wipeout_state::math::{
    add, clamp, cross, dot, length, madd, normalize, normalize_or, scale, sub, wrap_angle,
};
pub(in crate::player::offboard) const ZERO: Vector = [0.0; 4];
pub(in crate::player::offboard) const UP: Vector = [0.0, 1.0, 0.0, 0.0];
pub(in crate::player::offboard) const DT: f32 = f32::from_bits(0x3c88_8889);
pub(in crate::player::offboard) const RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
pub(in crate::player::offboard) fn select(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
pub(in crate::player::offboard) fn flatten(mut v: Vector) -> Vector {
    v[1] = 0.0;
    v
}
pub(in crate::player::offboard) fn xyz_finite(v: Vector) -> bool {
    v[..3].iter().all(|x| x.is_finite())
}
/// Preserve finite packed lanes; drop non-finite SIMD scratch in lane 3.
pub(in crate::player::offboard) fn sanitize_w_lane(v: Vector) -> Vector {
    if v[3].is_finite() {
        v
    } else {
        let mut v = v;
        v[3] = 0.0;
        v
    }
}
pub(in crate::player::offboard) fn magnitude(q: f32) -> f32 {
    let value = q * crate::physics::board_motion_output::inverse_length_squared(q, 2);
    if q == 0.0 { 0.0 } else { value }
}
///82E0A4E8: normalize axis safely, then separate multiplication/subtraction.
pub(in crate::player::offboard) fn reject(v: Vector, axis: Vector) -> Vector {
    let axis = normalize_or(axis, ZERO);
    sub(v, scale(axis, dot(v, axis)))
}
///82BD3E78, including its strict angle comparison and raw quaternion W insert.
pub(in crate::player::offboard) fn limit_angle(
    target: Vector,
    from: Vector,
    maximum: f32,
) -> Vector {
    let a = normalize_or(target, ZERO);
    let b = normalize_or(from, ZERO);
    let axis = cross(a, b);
    let angle = crate::trigonometry::acos(dot(a, b).max(-1.0).min(1.0));
    let turns = angle.mul_add(f32::from_bits(0x3e22_f983), 0.5).floor();
    let closest = (-turns).mul_add(f32::from_bits(0x40c9_0fdb), angle);
    if closest.abs() < maximum || dot(axis, axis) < f32::from_bits(0x3780_0000) {
        return target;
    }
    let (s, c) = crate::trigonometry::sin_cos(-maximum * 0.5);
    let mut q = scale(normalize(axis), s);
    q[3] = c;
    let first = cross(q, from);
    let middle = madd(from, c, first);
    scale(madd(cross(q, middle), 2.0, from), length(target))
}
///82D7C724..C7F4 XYZ Rodrigues rotation. Host representation adaptation:
///native permutation scratch lanes are not a fourth spatial velocity component.
///Keep the geometric vector W=0 at this producer, before trajectory feedback.
pub(in crate::player::offboard) fn rotate(v: Vector, axis: Vector, angle: f32) -> Vector {
    let [x, y, z, _] = axis;
    let (s, c) = crate::trigonometry::sin_cos(angle);
    let t = 1.0 - c;
    let (tx, ty, tz) = (t * x, t * y, t * z);
    let (sx, sy, sz) = (s * x, s * y, s * z);
    let xx = tx.mul_add(x, c);
    let yx = ty * x - sz;
    let zx = tz.mul_add(x, sy);
    let right = [xx, tx.mul_add(y, sz), tx * z - sy, 0.];
    let up = [yx, ty.mul_add(y, c), ty.mul_add(z, sx), 0.];
    let forward = [zx, tz * y - sx, tz.mul_add(z, c), 0.];
    madd(forward, v[2], madd(up, v[1], scale(right, v[0])))
}
