//! Compatibility entry point for recovered callers that use `vrsqrtefp`.

pub(crate) fn estimate(value: f32) -> f32 {
    super::native_arithmetic::reciprocal_square_root_estimate(value)
}
