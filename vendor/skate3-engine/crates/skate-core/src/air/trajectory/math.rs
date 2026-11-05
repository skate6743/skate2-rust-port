//! Independent PC arithmetic retaining the recovered refinement sequences.
use crate::physics::native_arithmetic;
pub(super) type Vector = [f32; 4];
pub(super) type Transform = [Vector; 4];
pub(super) const ZERO: Vector = [0.0; 4];
pub(super) const UP: Vector = [0.0, 1.0, 0.0, 0.0];
pub(super) const STEP: f32 = f32::from_bits(0x3c88_8889);
pub(super) const IDENTITY: Transform = [[1.0, 0.0, 0.0, 0.0], UP, [0.0, 0.0, 1.0, 0.0], ZERO];
pub(super) fn dot(a: Vector, b: Vector) -> f32 {
    native_arithmetic::dot3(a, b)
}
pub(super) fn add(a: Vector, b: Vector) -> Vector {
    core::array::from_fn(|i| a[i] + b[i])
}
pub(super) fn sub(a: Vector, b: Vector) -> Vector {
    core::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn scale(a: Vector, s: f32) -> Vector {
    a.map(|v| v * s)
}
pub(super) fn madd(a: Vector, s: f32, b: Vector) -> Vector {
    core::array::from_fn(|i| a[i].mul_add(s, b[i]))
}
pub(super) fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.0,
    ]
}
pub(super) fn inverse_length(square: f32) -> f32 {
    let mut v = native_arithmetic::reciprocal_square_root_estimate(square);
    for _ in 0..2 {
        v = (v * 0.5).mul_add((-square).mul_add(v * v, 1.0), v);
    }
    v
}
pub(super) fn reciprocal(value: f32) -> f32 {
    let mut inverse = native_arithmetic::reciprocal_estimate(value);
    for _ in 0..2 {
        inverse = inverse.mul_add((-inverse).mul_add(value, 1.0), inverse);
    }
    inverse
}
///8296EBB0: squared-length gate, ONE refinement, clamped dot, native acos.
pub(super) fn angle_between(a: Vector, b: Vector) -> f32 {
    let aa = dot(a, a);
    let bb = dot(b, b);
    if !(aa > f32::from_bits(0x38d1_b717) && bb > f32::from_bits(0x38d1_b717)) {
        return 0.0;
    }
    let unit = |v: Vector, square: f32| {
        let r = native_arithmetic::reciprocal_square_root_estimate(square);
        scale(v, (r * 0.5).mul_add((-square).mul_add(r * r, 1.0), r))
    };
    crate::trigonometry::acos(dot(unit(a, aa), unit(b, bb)).max(-1.0).min(1.0))
}
pub(super) fn length(a: Vector) -> f32 {
    let square = dot(a, a);
    if square == 0.0 {
        0.0
    } else {
        square * inverse_length(square)
    }
}
pub(super) fn normalize(a: Vector) -> Vector {
    let square = dot(a, a);
    let inverse = inverse_length(square);
    let length = if square == 0.0 { 0.0 } else { square * inverse };
    //830BD350's initializer82F826F8 broadcasts82181A88 (1e-6).
    if length > f32::from_bits(0x3586_37bd) {
        scale(a, inverse)
    } else {
        ZERO
    }
}
pub(super) fn transform(m: Transform, a: Vector) -> Vector {
    madd(m[2], a[2], madd(m[1], a[1], madd(m[0], a[0], m[3])))
}
