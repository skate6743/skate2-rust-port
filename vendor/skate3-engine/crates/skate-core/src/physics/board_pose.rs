//! Complete TU3 board SetTransform 82C0B2C8 and its part/pose dependencies.
//! This changes poses only; it does not infer startup, settle joints or reset rates.
mod arithmetic;
mod normalization;
mod part;
use arithmetic::*;
pub use normalization::{orthonormalize_part_basis, orthonormalize_rotation};
pub use part::{PartPose, part_transform, set_part_transform};

/// Guest-order Ri/Up/At/translation vectors, including all packed fourth lanes.
pub type PoseMatrix = [u32; 16];

/// Set deck part 6 to the orthonormalized request, then move parts 0..5 by its
/// common rigid delta. The separate hook receives the ORIGINAL requested pose.
pub fn set_board_transform(parts: &mut [PartPose; 7], hook: &mut PartPose, requested: PoseMatrix) {
    let old_deck = matrix(part_transform(&parts[6]));
    let normalized = orthonormalize_rotation(requested);
    let delta = compose(matrix(normalized), inverse_rigid(old_deck));
    set_part_transform(&mut parts[6], normalized);
    for part in &mut parts[..6] {
        let moved = compose(delta, matrix(part_transform(part)));
        set_part_transform(part, orthonormalize_rotation(words(moved)));
    }
    set_part_transform(hook, requested);
}
