//!82BD41B0 delta bound and82C1E170 one-sided projection, original raw audited.
use super::Vector;
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
fn inverse_squared(value: f32) -> f32 {
    // The portable estimate preserves subnormals. For a tiny positive squared
    // length, r*r can overflow even though both the length and inverse root
    // are finite. Normalize its exponent before the recovered refinements;
    // rsqrt(x * 2^24) * 2^12 = rsqrt(x). Normal inputs keep their old path.
    let (value, rescale) = if value > 0.0 && value.is_subnormal() {
        (value * 16_777_216.0, 4096.0)
    } else {
        (value, 1.0)
    };
    //PC seed is not bit-exact Xenon; retain both original refinement stages.
    let mut r = crate::physics::reciprocal_sqrt::estimate(value);
    for _ in 0..2 {
        let error = (-value).mul_add(r * r, 1.0);
        r = (r * 0.5).mul_add(error, r);
    }
    r * rescale
}
pub(super) fn length(v: Vector) -> f32 {
    let squared = dot(v, v);
    let value = squared * inverse_squared(squared);
    if squared == 0.0 { 0.0 } else { value }
}
pub(super) fn normalize_or(v: Vector, fallback: Vector) -> Vector {
    let squared = dot(v, v);
    let r = inverse_squared(squared);
    let magnitude = if squared == 0.0 { 0.0 } else { squared * r };
    if magnitude > f32::from_bits(0x3586_37bd) {
        scale(v, r)
    } else {
        fallback
    }
}
pub(super) fn limit_delta(target: Vector, current: Vector, maximum: f32) -> Vector {
    let delta = sub(target, current);
    let magnitude = length(delta);
    if maximum >= magnitude {
        target
    } else {
        madd(delta, maximum / magnitude, current)
    }
}
pub(super) fn remove_positive(value: Vector, direction: Vector) -> Vector {
    let projection = dot(value, normalize_or(direction, [0.0; 4]));
    //Native subtracts original direction, NOT the normalized direction.
    if projection > 0.0 {
        sub(value, scale(direction, projection))
    } else {
        value
    }
}
