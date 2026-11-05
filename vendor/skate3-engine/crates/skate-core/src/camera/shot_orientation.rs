//! Quaternion interpolation82E07540, with the native sign choice and near-angle
//! branch. Shared Xenon arithmetic remains approximation-derived; no generated
//! implementation is linked or imported.
use super::{shot::sine_blend, vector_tracker::refined_reciprocal};

pub(super) fn interpolate_orientation(from: [f32; 4], to: [f32; 4], fraction: f32) -> [f32; 4] {
    slerp(from, to, sine_blend(fraction))
}

/// FrameComposer82E01740 uses the same interpolation without sine-easing its
/// input. Native negates the first quaternion when the dot product is negative.
pub(super) fn slerp(from: [f32; 4], to: [f32; 4], weight: f32) -> [f32; 4] {
    let dot = crate::physics::native_arithmetic::dot4(from, to);
    let signed_from = if 0.0 > dot { from.map(|v| -v) } else { from };
    let absolute_dot = if 0.0 > dot { -dot } else { dot };
    if absolute_dot > f32::from_bits(0x3f7f069e) {
        let same_direction = crate::physics::native_arithmetic::dot4(signed_from, to) > 0.0;
        let value = core::array::from_fn(|i| {
            if same_direction {
                (to[i] - signed_from[i]).mul_add(weight, signed_from[i])
            } else {
                (-(to[i] + signed_from[i])).mul_add(weight, signed_from[i])
            }
        });
        let square = crate::physics::native_arithmetic::dot4(value, value);
        let mut inverse = crate::physics::reciprocal_sqrt::estimate(square);
        for _ in 0..2 {
            inverse = (inverse * 0.5).mul_add((-square).mul_add(inverse * inverse, 1.0), inverse);
        }
        value.map(|v| v * inverse)
    } else {
        let angle = crate::trigonometry::acos(absolute_dot);
        let inverse_sine = refined_reciprocal(crate::trigonometry::sin(angle));
        let first = crate::trigonometry::sin((1.0 - weight) * angle) * inverse_sine;
        let second = crate::trigonometry::sin(weight * angle) * inverse_sine;
        core::array::from_fn(|i| to[i].mul_add(second, signed_from[i] * first))
    }
}
