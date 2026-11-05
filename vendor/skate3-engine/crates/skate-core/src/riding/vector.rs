use crate::physics::reciprocal_sqrt::estimate;

pub(super) fn dot3(a: [f32; 4], b: [f32; 4]) -> f32 {
    crate::physics::native_arithmetic::dot3(a, b)
}

/// Inline NormalizeSafe sequence in TU3 82C04F68 / 82D92CF8.
pub(super) fn normalize(v: [f32; 4], threshold: [f32; 4]) -> [f32; 4] {
    let squared = dot3(v, v);
    let mut inverse = estimate(squared);
    for _ in 0..2 {
        let correction = (-squared).mul_add(inverse * inverse, 1.0);
        inverse = (inverse * 0.5).mul_add(correction, inverse);
    }
    let length = if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    };
    core::array::from_fn(|i| {
        if length > threshold[i] {
            v[i] * inverse
        } else {
            0.0
        }
    })
}

pub(super) fn clamp(value: f32, lower: f32, upper: f32) -> f32 {
    let v = if lower - value >= 0.0 { lower } else { value };
    if upper - v >= 0.0 { v } else { upper }
}

pub(super) fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    core::array::from_fn(|i| {
        let j = if i == 3 { 3 } else { (i + 1) % 3 };
        let k = if i == 3 { 3 } else { (i + 2) % 3 };
        (-a[k]).mul_add(b[j], a[j] * b[k])
    })
}
