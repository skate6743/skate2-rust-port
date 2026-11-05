//! SetBumpCoefficients Begin82BB0660, ctor82BC7318/vtable823200C8+48.
use crate::physics::native_arithmetic;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub scale_x_acc: f32,
    pub min_bump_mag: f32,
    pub min_bump_blend_value: f32,
    pub max_bump_mag: f32,
}
///Input is PhysOutAnimation112 (already ground-conditioned), not raw world acceleration.
pub fn coefficients(mut acceleration: [f32; 4], mirrored: bool, settings: &Settings) -> [f32; 2] {
    acceleration[0] *= settings.scale_x_acc;
    let squared = native_arithmetic::dot3(acceleration, acceleration);
    let mut inverse = native_arithmetic::reciprocal_square_root_estimate(squared);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-squared).mul_add(inverse * inverse, 1.), inverse);
    }
    let magnitude = if squared == 0. { 0. } else { squared * inverse };
    let t = ((magnitude - settings.min_bump_mag) / (settings.max_bump_mag - settings.min_bump_mag))
        .clamp(0., 1.);
    let weight = t.mul_add(
        1. - settings.min_bump_blend_value,
        settings.min_bump_blend_value,
    );
    //Initializer82F826F8 broadcasts82181A88 into830BD350: length threshold1e-6.
    let direction = if magnitude > f32::from_bits(0x3586_37bd) {
        [acceleration[0] * inverse, acceleration[2] * inverse]
    } else {
        [0.; 2]
    };
    direction.map(|v| {
        let value = v * weight;
        if mirrored { -value } else { value }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bump_direction_scaling_thresholds_and_stance() {
        let s = Settings {
            scale_x_acc: 2.,
            min_bump_mag: 10.,
            min_bump_blend_value: 0.2,
            max_bump_mag: 30.,
        };
        assert_eq!(coefficients([0.; 4], false, &s), [0.; 2]);
        for (input, expected) in [
            ([5., 0., 0., 99.], [0.2, 0.]),
            ([10., 0., 0., 0.], [0.6, 0.]),
            ([0., 0., -40., 0.], [0., -1.]),
            ([0.0000001, 0., 0., 0.], [0., 0.]),
        ] {
            let result = coefficients(input, false, &s);
            let mirrored = coefficients(input, true, &s);
            for i in 0..2 {
                assert!(
                    (result[i] - expected[i]).abs() < 1e-6,
                    "{result:?} != {expected:?}"
                );
                assert_eq!(mirrored[i], -result[i]);
            }
        }
        let diagonal = coefficients([12., 0., 18., 0.], false, &s);
        assert!((diagonal[0] - 0.8).abs() < 1e-6 && (diagonal[1] - 0.6).abs() < 1e-6);
    }
}
