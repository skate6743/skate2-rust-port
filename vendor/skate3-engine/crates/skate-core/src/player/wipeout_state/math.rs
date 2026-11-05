//! Scalar/vector expressions recovered from original Wipeout control kernels.
use crate::physics::{board_motion_output::inverse_length_squared, native_arithmetic};
pub type V = [f32; 4];
pub fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
pub fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
pub fn scale(a: V, b: f32) -> V {
    a.map(|v| v * b)
}
pub fn madd(a: V, b: f32, c: V) -> V {
    std::array::from_fn(|i| a[i].mul_add(b, c[i]))
}
pub fn dot(a: V, b: V) -> f32 {
    native_arithmetic::dot3(a, b)
}
pub fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
pub fn length(a: V) -> f32 {
    let square = dot(a, a);
    if square == 0.0 {
        0.0
    } else {
        square * inverse_length_squared(square, 2)
    }
}
pub fn normalize(a: V) -> V {
    scale(a, inverse_length_squared(dot(a, a), 2))
}
///Original830BD350 is initialized to splatted358637BD, not a fitted tolerance.
pub fn normalize_or(a: V, fallback: V) -> V {
    if length(a) > f32::from_bits(0x3586_37BD) {
        normalize(a)
    } else {
        fallback
    }
}
pub fn reciprocal(v: f32) -> f32 {
    let mut inv = native_arithmetic::reciprocal_estimate(v);
    for _ in 0..2 {
        inv = inv.mul_add((-inv).mul_add(v, 1.0), inv);
    }
    inv
}
pub fn clamp(v: f32, low: f32, high: f32) -> f32 {
    let v = if low - v >= 0.0 { low } else { v };
    if high - v >= 0.0 { v } else { high }
}

///Original82BD3D90: short vectors retain all four lanes.
pub fn limit_length(v: V, maximum: f32) -> V {
    let len = length(v);
    if !(len >= f32::from_bits(0x3780_0000)) {
        return v;
    }
    let cap = if maximum - len >= 0.0 { len } else { maximum };
    scale(scale(v, cap), reciprocal(len))
}

pub fn wrap_angle(angle: f32) -> f32 {
    let turns = angle * f32::from_bits(0x3E22_F983);
    let fraction = turns - turns.floor();
    let centered = if fraction > 0.5 {
        fraction - 1.0
    } else {
        fraction
    };
    centered * f32::from_bits(0x40C9_0FDB)
}
