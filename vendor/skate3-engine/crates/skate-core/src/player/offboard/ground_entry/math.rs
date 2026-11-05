//! Original82C1E220 quaternion vector rotation and8296EC98 signed angle.
use super::{Frame, Vector};
pub(super) const TAU: f32 = f32::from_bits(0x40c9_0fdb);
pub(super) const IDENTITY: Frame = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0; 4],
];
pub(super) fn dot(a: Vector, b: Vector) -> f32 {
    crate::physics::native_arithmetic::dot3(a, b)
}
pub(super) fn scale(v: Vector, s: f32) -> Vector {
    v.map(|x| x * s)
}
pub(super) fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn inverse_length(q: f32, refinements: usize) -> f32 {
    let mut r = crate::physics::reciprocal_sqrt::estimate(q);
    for _ in 0..refinements {
        r = (r * 0.5).mul_add((-q).mul_add(r * r, 1.0), r);
    }
    r
}
pub(super) fn length(v: Vector) -> f32 {
    let q = dot(v, v);
    let l = q * inverse_length(q, 2);
    if q == 0.0 { 0.0 } else { l }
}
pub(super) fn wrap(angle: f32) -> f32 {
    let turns = angle * f32::from_bits(0x3e22_f983);
    let fraction = turns - turns.floor();
    (fraction - if fraction > 0.5 { 1.0 } else { 0.0 }) * TAU
}
pub(super) fn signed_angle(a: Vector, b: Vector, axis: Vector) -> f32 {
    let qa = dot(a, a);
    let qb = dot(b, b);
    if !(qa > f32::from_bits(0x38d1_b717)) || !(qb > f32::from_bits(0x38d1_b717)) {
        return 0.0;
    }
    let a = scale(a, inverse_length(qa, 1));
    let b = scale(b, inverse_length(qb, 1));
    let cosine = dot(a, b).max(-1.0).min(1.0);
    let angle = crate::trigonometry::acos(cosine);
    if dot(cross(a, b), axis) < 0.0 {
        TAU - angle
    } else {
        angle
    }
}
pub(super) fn rotate(v: Vector, axis: Vector, angle: f32) -> Vector {
    let (s, c) = crate::trigonometry::sin_cos(angle * 0.5);
    let mut q = scale(axis, s);
    q[3] = c;
    let first = cross(q, v);
    let intermediate = std::array::from_fn(|i| v[i].mul_add(c, first[i]));
    let second = cross(q, intermediate);
    std::array::from_fn(|i| second[i].mul_add(2.0, v[i]))
}
