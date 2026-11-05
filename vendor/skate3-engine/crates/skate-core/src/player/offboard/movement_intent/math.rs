//! Address-audited scalar forms; host sqrt seed plus original refinements.
//! Ordinary PC arithmetic is not bit-exact Xenon emulation.
use super::Vector;
pub(super) const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
const EPSILON: f32 = f32::from_bits(0x3586_37bd); //82F826F8 ->830BD350
pub(super) fn select(test: f32, yes: f32, no: f32) -> f32 {
    if test >= 0.0 { yes } else { no }
}
pub(super) fn clamp(value: f32, low: f32, high: f32) -> f32 {
    let lower = select(low - value, low, value);
    select(high - lower, lower, high)
}
pub(super) fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(super) fn scale(a: Vector, s: f32) -> Vector {
    a.map(|v| v * s)
}
pub(super) fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn madd(a: Vector, s: f32, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i].mul_add(s, b[i]))
}
fn inverse_squared(value: f32, refinements: usize) -> f32 {
    let mut r = crate::physics::reciprocal_sqrt::estimate(value);
    for _ in 0..refinements {
        let error = (-value).mul_add(r * r, 1.0);
        r = (r * 0.5).mul_add(error, r);
    }
    r
}
pub(super) fn length(v: Vector) -> f32 {
    let squared = dot(v, v);
    let value = squared * inverse_squared(squared, 2);
    if squared == 0.0 { 0.0 } else { value }
}
pub(super) fn normalize_or(v: Vector, fallback: Vector) -> Vector {
    let squared = dot(v, v);
    let r = inverse_squared(squared, 2);
    let length = if squared == 0.0 { 0.0 } else { squared * r };
    if length > EPSILON {
        scale(v, r)
    } else {
        fallback
    }
}
///8269A4F0 permute [x,zero,x,x], then vrlimi mask2 restores Z.
///W is X, not zero: preserve the native lane even though the caller dots XYZ.
pub(super) fn horizontal(v: Vector) -> Vector {
    normalize_or([v[0], 0.0, v[2], v[0]], [0.0; 4])
}
pub(super) fn wrap_angle(angle: f32) -> f32 {
    let turns = angle * f32::from_bits(0x3e22_f983);
    let fraction = turns - turns.floor();
    (fraction - if fraction > 0.5 { 1.0 } else { 0.0 }) * f32::from_bits(0x40c9_0fdb)
}
pub(super) fn signed_angle(a: Vector, b: Vector, axis: Vector) -> f32 {
    let aa = dot(a, a);
    let bb = dot(b, b);
    let threshold = f32::from_bits(0x38d1_b717);
    if !(aa > threshold && bb > threshold) {
        return 0.0;
    }
    let a = scale(a, inverse_squared(aa, 1));
    let b = scale(b, inverse_squared(bb, 1));
    let angle = crate::trigonometry::acos(dot(a, b).max(-1.0).min(1.0));
    let cross = [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ];
    if dot(cross, axis) < 0.0 {
        f32::from_bits(0x40c9_0fdb) - angle
    } else {
        angle
    }
}
pub(super) fn projected_angle(a: Vector, b: Vector, axis: Vector) -> f32 {
    if !(dot(a, a) * dot(b, b) > f32::from_bits(0x3780_0000)) {
        return 0.0;
    }
    signed_angle(
        sub(a, scale(axis, dot(axis, a))),
        sub(b, scale(axis, dot(axis, b))),
        axis,
    )
}
