//! Independent host math for the operations used by recovered TU3 callers.
//!
//! Uses Rust floating-point operations without external estimate tables or
//! translator-specific special cases. Callers retain their recovered refinement
//! steps. This is working PC math, not bit-exact Xenon instruction emulation.

pub(crate) fn reciprocal_estimate(value: f32) -> f32 {
    value.recip()
}

pub(crate) fn reciprocal_square_root_estimate(value: f32) -> f32 {
    value.sqrt().recip()
}

pub(crate) fn dot3(left: [f32; 4], right: [f32; 4]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

pub(crate) fn dot4(left: [f32; 4], right: [f32; 4]) -> f32 {
    dot3(left, right) + left[3] * right[3]
}

pub(crate) fn vector_min(left: f32, right: f32) -> f32 {
    left.min(right)
}

pub(crate) fn vector_max(left: f32, right: f32) -> f32 {
    left.max(right)
}

#[cfg(test)]
#[path = "tests/native_arithmetic.rs"]
mod tests;
