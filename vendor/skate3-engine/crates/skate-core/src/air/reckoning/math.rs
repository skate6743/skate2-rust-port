//! Original TU3 angle limiter82BD3E78 and Air heading rotation82D8DFD4.
//! Basic arithmetic runs on the host; this is not bit-exact Xenon emulation.
use crate::{
    math::Vector3,
    physics::{
        board_ground::angle_between,
        board_motion_output::inverse_length_squared,
        native_arithmetic::{dot3, vector_max, vector_min},
    },
    trigonometry,
};
pub(super) type V = [f32; 4];
const EPSILON: f32 = f32::from_bits(0x3586_37bd);
const TAU: f32 = f32::from_bits(0x40c9_0fdb);
const INV_TAU: f32 = f32::from_bits(0x3e22_f983);

pub(super) fn angle(a: V, b: V) -> f32 {
    angle_between(
        Vector3::new(a[0], a[1], a[2]),
        Vector3::new(b[0], b[1], b[2]),
    )
}
pub(super) fn wrap(angle: f32) -> f32 {
    let turns = angle * INV_TAU;
    let fraction = turns - turns.floor();
    (fraction - if fraction > 0.5 { 1.0 } else { 0.0 }) * TAU
}
pub(super) fn length(v: V) -> f32 {
    let sq = dot3(v, v);
    let inverse = inverse_length_squared(sq, 2);
    if sq == 0.0 { 0.0 } else { sq * inverse }
}
pub(super) fn normalize(v: V) -> V {
    let inverse = inverse_length_squared(dot3(v, v), 2);
    v.map(|x| x * inverse)
}
pub(super) fn normalize_safe(v: V, fallback: V) -> V {
    let sq = dot3(v, v);
    let inverse = inverse_length_squared(sq, 2);
    let magnitude = if sq == 0.0 { 0.0 } else { sq * inverse };
    if magnitude > EPSILON {
        v.map(|x| x * inverse)
    } else {
        fallback
    }
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}

///82BD3E78. Retains the source's original (not normalized) endpoint operands
/// and final target magnitude. The parallel-axis branch returns target as-is.
pub fn limit_angle(target: V, from: V, maximum: f32) -> V {
    let normalized_from = normalize_safe(from, [0.0; 4]);
    let normalized_target = normalize_safe(target, [0.0; 4]);
    let axis = cross(normalized_target, normalized_from);
    let cosine = vector_min(
        vector_max(dot3(normalized_target, normalized_from), -1.0),
        1.0,
    );
    let angle = trigonometry::acos(cosine);
    //The limiter uses floor(angle/tau + .5), unlike its caller's fraction wrap.
    let turns = angle.mul_add(INV_TAU, 0.5).floor();
    let closest_angle = (-turns).mul_add(TAU, angle);
    if closest_angle.abs() < maximum || dot3(axis, axis) < f32::from_bits(0x3780_0000) {
        return target;
    }
    let axis = normalize(axis);
    let (sin, cos) = trigonometry::sin_cos(-maximum * 0.5);
    let q = axis.map(|v| v * sin);
    let first = cross(q, from);
    let intermediate = std::array::from_fn(|i| cos.mul_add(from[i], first[i]));
    let second = cross(q, intermediate);
    let rotated: V = std::array::from_fn(|i| second[i].mul_add(2.0, from[i]));
    let magnitude = length(target);
    rotated.map(|v| v * magnitude)
}

///82D8DFD4..E0B0: Rodrigues columns followed by X/Y/Z ordered FMAs.
///The native permutation repeats each column's X into its W lane.
pub(super) fn rotate_heading(heading: V, up: V, angle: f32) -> V {
    let [x, y, z, _] = up;
    let (s, c) = trigonometry::sin_cos(angle);
    let t = 1.0 - c;
    let (tx, ty, tz) = (t * x, t * y, t * z);
    let (sx, sy, sz) = (s * x, s * y, s * z);
    let xx = tx.mul_add(x, c);
    let yx = ty * x - sz;
    let zx = tz.mul_add(x, sy);
    let columns = [
        [xx, tx.mul_add(y, sz), tx * z - sy, xx],
        [yx, ty.mul_add(y, c), ty.mul_add(z, sx), yx],
        [zx, tz * y - sx, tz.mul_add(z, c), zx],
    ];
    std::array::from_fn(|i| {
        let first = columns[0][i] * heading[0];
        let second = columns[1][i].mul_add(heading[1], first);
        columns[2][i].mul_add(heading[2], second)
    })
}
