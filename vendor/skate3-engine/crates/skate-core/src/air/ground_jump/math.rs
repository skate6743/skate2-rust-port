//! Original arithmetic ordering and two estimate-refinement steps.
use crate::physics::{board_motion_output::inverse_length_squared, native_arithmetic};
type Vector = [f32; 4];
pub(super) const UP: Vector = [0.0, 1.0, 0.0, 0.0];
pub(super) fn dot(a: Vector, b: Vector) -> f32 {
    native_arithmetic::dot3(a, b)
}
pub(super) fn add(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] + b[i])
}
pub(super) fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn scale(a: Vector, b: f32) -> Vector {
    a.map(|v| v * b)
}
pub(super) fn madd(a: Vector, b: f32, c: Vector) -> Vector {
    std::array::from_fn(|i| a[i].mul_add(b, c[i]))
}
pub(super) fn planar(value: Vector, normal: Vector) -> Vector {
    sub(value, scale(normal, dot(value, normal)))
}
pub(super) fn maximum(a: f32, b: f32) -> f32 {
    if a - b >= -0.0 { a } else { b }
}
pub(super) fn clamp(v: f32, low: f32, high: f32) -> f32 {
    let lower = if low - v >= -0.0 { low } else { v };
    if high - lower >= -0.0 { lower } else { high }
}
pub(super) fn vector_clamp(v: f32, low: f32, high: f32) -> f32 {
    native_arithmetic::vector_min(high, native_arithmetic::vector_max(low, v))
}
pub(super) fn reciprocal(v: f32) -> f32 {
    let mut r = native_arithmetic::reciprocal_estimate(v);
    for _ in 0..2 {
        r = r.mul_add((-r).mul_add(v, 1.0), r);
    }
    r
}
pub(super) fn square_root(square: f32) -> f32 {
    let value = square * inverse_length_squared(square, 2);
    if square == 0.0 { 0.0 } else { value }
}
pub(super) fn length(value: Vector) -> f32 {
    square_root(dot(value, value))
}
pub(super) fn normalize(value: Vector) -> Vector {
    let square = dot(value, value);
    let inverse = inverse_length_squared(square, 2);
    let length = if square == 0.0 { 0.0 } else { square * inverse };
    //82F826F8 initializes830BD350 from82181A88 (1e-6), not its IDA BSS zero.
    if length > f32::from_bits(0x3586_37bd) {
        scale(value, inverse)
    } else {
        [0.0; 4]
    }
}
pub(super) fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.0,
    ]
}
