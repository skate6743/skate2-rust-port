//! Observed deck motion from TU3 Skateboard::FillPhysOut82C02A80.
//! These calculations consume the live solver body and the actual Reckoning
//! normal. They do not infer contact state from height or vertical velocity.
use super::{board::BodyId, board_runtime::BoardRuntime, native_arithmetic};
use crate::math::{Basis3, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoardMotionOutput {
    pub angular_velocity: Vector3,
    pub linear_velocity: Vector3,
    pub ground_velocity: Vector3,
    /// SkateboardMotion+160: magnitude of the unprojected deck velocity.
    pub speed: f32,
    /// SkateboardMotion+164: magnitude after rejecting the Reckoning normal.
    pub ground_speed: f32,
    /// SkateboardMotion+168: raw deck velocity projected onto effective Z.
    pub forward_speed: f32,
    pub effective_basis: Basis3,
}

impl BoardMotionOutput {
    pub fn from_board(
        board: &BoardRuntime,
        reckoning_ground_normal: Vector3,
        processed_flags_2468: u32,
    ) -> Self {
        let deck = board.bodies()[BodyId::Deck.index()].rates;
        let mut effective_basis = board.part_transforms()[BodyId::Deck.index()].basis;
        // Complete GetEffectiveTransform82C01BF8 axis adjustment; translation
        // and Y remain the physical part's values.
        if processed_flags_2468 & 0x0010_0000 != 0 {
            for axis in [0, 2] {
                effective_basis.columns[axis] = effective_basis.columns[axis].map(|v| -v);
            }
        }
        let normal_speed = dot(deck.linear_velocity, reckoning_ground_normal);
        let ground_velocity = subtract(
            deck.linear_velocity,
            scale(reckoning_ground_normal, normal_speed),
        );
        let z = effective_basis.columns[2];
        Self {
            angular_velocity: deck.angular_velocity,
            linear_velocity: deck.linear_velocity,
            ground_velocity,
            speed: length(deck.linear_velocity),
            ground_speed: length(ground_velocity),
            forward_speed: dot(deck.linear_velocity, Vector3::new(z[0], z[1], z[2])),
            effective_basis,
        }
    }
}

pub fn dot(a: Vector3, b: Vector3) -> f32 {
    native_arithmetic::dot3([a.x, a.y, a.z, 0.0], [b.x, b.y, b.z, 0.0])
}
pub(crate) fn scale(v: Vector3, s: f32) -> Vector3 {
    Vector3::new(v.x * s, v.y * s, v.z * s)
}
pub(crate) fn add(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
pub(crate) fn subtract(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
pub(crate) fn inverse_length_squared(squared: f32, refinements: usize) -> f32 {
    // The PC rsqrt seed preserves subnormals. Squaring its finite inverse
    // can overflow during refinement (landing residuals around 1e-22).
    // Rescale the exponent before refinement, then undo that scale:
    // rsqrt(x * 2^24) * 2^12 = rsqrt(x). Normal inputs keep the same path.
    let (squared, rescale) = if squared > 0.0 && squared.is_subnormal() {
        (squared * 16_777_216.0, 4096.0)
    } else {
        (squared, 1.0)
    };
    let mut inverse = native_arithmetic::reciprocal_square_root_estimate(squared);
    for _ in 0..refinements {
        let correction = (-squared).mul_add(inverse * inverse, 1.0);
        inverse = (inverse * 0.5).mul_add(correction, inverse);
    }
    inverse * rescale
}
pub fn length(v: Vector3) -> f32 {
    let squared = dot(v, v);
    let value = squared * inverse_length_squared(squared, 2);
    if squared == 0.0 { 0.0 } else { value }
}

#[cfg(test)]
mod normalization_tests {
    use super::*;
    #[test]
    fn tiny_finite_vector_has_finite_nonzero_length() {
        let v = Vector3::new(1.6165965e-22, 0.0, 4.2145673e-22);
        let actual = length(v);
        let expected = ((v.x as f64).powi(2) + (v.z as f64).powi(2)).sqrt() as f32;
        assert!(actual.is_finite() && actual > 0.0, "length={actual}");
        assert!((actual / expected - 1.0).abs() < 0.01);
    }
}

#[cfg(test)]
mod inverse_range_tests {
    use super::*;
    #[test]
    fn inverse_root_covers_subnormal_range_without_changing_normal_arithmetic() {
        let mut samples = vec![
            f32::from_bits(1),
            f32::from_bits(0x007f_ffff),
            f32::MIN_POSITIVE,
            f32::MAX,
        ];
        samples.extend((-149..=127).map(|e| 2.0_f64.powi(e) as f32));
        for x in samples {
            for steps in 0..=2 {
                let actual = inverse_length_squared(x, steps);
                let expected = (1.0 / (x as f64).sqrt()) as f32;
                assert!(actual.is_finite() && actual > 0.0, "x={x} steps={steps}");
                assert!(
                    (actual / expected - 1.0).abs() < 3e-7,
                    "x={x} steps={steps}"
                );
                if x.is_normal() {
                    let mut old = native_arithmetic::reciprocal_square_root_estimate(x);
                    for _ in 0..steps {
                        let correction = (-x).mul_add(old * old, 1.0);
                        old = (old * 0.5).mul_add(correction, old);
                    }
                    assert_eq!(actual.to_bits(), old.to_bits());
                }
            }
        }
        for x in [0.0, -1.0, f32::INFINITY, f32::NAN] {
            assert!(inverse_length_squared(x, 2).is_nan());
        }
    }
}
