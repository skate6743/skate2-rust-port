//! Refinement sequence used by S3 82D861D8/82D88518/82D886B8.
//! Seeds use existing independent PC math, not a Xenon bit-exact claim.
use crate::physics::native_arithmetic;

pub(super) fn reciprocal(value: f32) -> f32 {
    let mut result = native_arithmetic::reciprocal_estimate(value);
    for _ in 0..2 {
        let error = (-result).mul_add(value, 1.0);
        result = result.mul_add(error, result);
    }
    result
}

pub(super) fn inverse_square_root(value: f32) -> f32 {
    let mut result = native_arithmetic::reciprocal_square_root_estimate(value);
    for _ in 0..2 {
        let square = result * result;
        let half = result * 0.5;
        let error = (-value).mul_add(square, 1.0);
        result = half.mul_add(error, result);
    }
    result
}

pub(super) fn square_root(value: f32) -> f32 {
    let result = value * inverse_square_root(value);
    // The original vector select explicitly chooses zero for a zero radicand.
    if value == 0.0 { 0.0 } else { result }
}
