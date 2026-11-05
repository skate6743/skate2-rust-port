//! Original vector arithmetic used by82BD4A30 and82BD5A00.
use super::collision_feedback::V;
use crate::physics::{
    native_arithmetic::dot3, reciprocal_sqrt::estimate,
    skeleton_animation_record::AnimationPartTransform,
};
pub(super) fn dot(a: V, b: V) -> f32 {
    dot3(a, b)
}
pub(super) fn scale(v: V, s: f32) -> V {
    v.map(|v| v * s)
}
pub(super) fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
pub(super) fn inverse_length(square: f32) -> f32 {
    let mut inverse = estimate(square);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-square).mul_add(inverse * inverse, 1.0), inverse);
    }
    inverse
}
pub(super) fn length(v: V) -> f32 {
    let square = dot(v, v);
    let result = square * inverse_length(square);
    if square == 0.0 { 0.0 } else { result }
}
pub(super) fn normalize(v: V) -> V {
    let square = dot(v, v);
    let inverse = inverse_length(square);
    let size = if square == 0.0 { 0.0 } else { square * inverse };
    if size > 1.0e-6 {
        scale(v, inverse)
    } else {
        [0.0; 4]
    }
}
///82BD3D90 keeps all stored lanes, including the unused fourth velocity lane.
pub(super) fn clamp_length(v: V, maximum: f32) -> V {
    let size = length(v);
    if size < f32::from_bits(0x3780_0000) {
        return v;
    }
    let bounded = if maximum - size >= 0.0 { size } else { maximum };
    let mut inverse = crate::physics::native_arithmetic::reciprocal_estimate(size);
    for _ in 0..2 {
        inverse = inverse.mul_add((-inverse).mul_add(size, 1.0), inverse);
    }
    scale(scale(v, bounded), inverse)
}
pub(super) fn transform(m: AnimationPartTransform, v: V) -> V {
    std::array::from_fn(|i| {
        m[2][i].mul_add(v[2], m[1][i].mul_add(v[1], m[0][i].mul_add(v[0], m[3][i])))
    })
}
pub(super) fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
