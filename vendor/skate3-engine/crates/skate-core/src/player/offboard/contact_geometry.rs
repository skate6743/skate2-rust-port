//! Native classifier math82D85360/85488/855A8/85C48.
use super::{
    contact_queries::{Input, V},
    contact_segments::{length_inverse, reciprocal},
};
use crate::physics::native_arithmetic::dot3;
pub fn slope_limit(input: Input, height: f32, rising: bool) -> f32 {
    let large = height > f32::from_bits(0x3ecc_cccd);
    let end = if large {
        if rising { 35. } else { 30. }
    } else {
        15.
    };
    let start = if large { 50. } else { 30. };
    let speed = length_inverse(input.velocity).0;
    // Scalar fsel clamp order retains native unordered behavior.
    let lower = if 2. - speed >= 0. { 2. } else { speed };
    let clamped = if 6. - lower >= 0. { lower } else { 6. };
    let degrees = ((clamped - 2.) * (end - start)).mul_add(0.25, start);
    crate::animation::foot_ik::post_contact::tangent(degrees * f32::from_bits(0x3c8e_fa35))
}
pub fn slope_between(input: Input, start: V, end: V) -> f32 {
    let delta = std::array::from_fn(|i| end[i] - start[i]);
    let distance = dot3(input.surface_forward, delta);
    if f32::from_bits(0x3a83_126f) > distance {
        1000.
    } else {
        dot3(input.surface_up, delta) * reciprocal(distance)
    }
}
/// Infinite 2D line intersection parameter along a..b. Parallel lines return
///the native finite sentinel; callers perform their own [0,1] interval tests.
pub fn intersect(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let cd = [d[0] - c[0], d[1] - c[1]];
    let determinant = cd[1] * ab[0] - cd[0] * ab[1];
    if !(determinant.abs() >= f32::from_bits(0x3727_c5ac)) {
        return f32::from_bits(0x5015_02f9);
    }
    let ac = [a[0] - c[0], a[1] - c[1]];
    (1. / determinant) * (cd[0] * ac[1] - cd[1] * ac[0])
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input(speed: f32) -> Input {
        Input {
            position: [0.; 4],
            surface_forward: [0., 0., 1., 0.],
            surface_up: [0., 1., 0., 0.],
            surface_right: [1., 0., 0., 0.],
            velocity: [0., 0., speed, 0.],
            animation_up: [0., 1., 0., 0.],
            animation_right: [1., 0., 0., 0.],
        }
    }
    #[test]
    fn stock_speed_clamps_and_height_branch_select_expected_angles() {
        for (speed, height, rising, angle) in [
            (0., 0.4, true, 30.0f64),
            (6., 0.4, true, 15.),
            (2., 0.5, true, 50.),
            (6., 0.5, true, 35.),
            (8., 0.5, false, 30.),
        ] {
            assert!(
                (slope_limit(input(speed), height, rising) - angle.to_radians().tan() as f32).abs()
                    < 1e-5
            );
        }
    }
    #[test]
    fn line_parameters_and_parallel_sentinel() {
        assert_eq!(intersect([0., 0.], [2., 0.], [1., -1.], [1., 1.]), 0.5);
        assert_eq!(intersect([0., 0.], [1., 0.], [2., -1.], [2., 1.]), 2.);
        assert_eq!(
            intersect([0., 0.], [1., 0.], [0., 1.], [1., 1.]).to_bits(),
            0x501502f9
        );
        assert_eq!(slope_between(input(0.), [0.; 4], [0., 1., 2., 0.]), 0.5);
        assert_eq!(slope_between(input(0.), [0.; 4], [0., 1., 0., 0.]), 1000.);
    }
}
