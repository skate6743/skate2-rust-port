//! Animated portion of CalculateInitialPartTransforms82BEDF08.
use super::status::{LimbStatus, Mode};
use crate::physics::skeleton_animation_record::{
    AnimationPartTransform as Transform, IDENTITY, compose_affine,
};

#[derive(Clone, Copy, Debug)]
pub struct LimbBinding {
    /// SkeletonIK1120: the physics volume driven by this target.
    pub part: usize,
    /// SkeletonIK1136: optional adjacent volume receiving the same adjustment.
    pub parent_part: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
pub struct LimbFrames {
    /// SkeletonIK864, from the actual reparented animation target.
    pub target: Transform,
    /// SkeletonIK1152; animated world frame, then blended/contact-adjusted.
    pub world: Transform,
    /// SkeletonIK1408: externally supplied target in world space.
    pub external_world: Transform,
    /// SkeletonIK1664: target relative to the animated board.
    pub board: Transform,
    /// SkeletonIK2240, optional adjacent volume in world space.
    pub parent_world: Transform,
    /// SkeletonIK2496: adjacent volume's external target.
    pub external_parent_world: Transform,
    /// SkeletonIK2752: adjacent volume relative to the animated board.
    pub parent_board: Transform,
    /// SkeletonIK3120: animated target lies within the cached contact box.
    pub within_contact_bounds: bool,
}
impl Default for LimbFrames {
    fn default() -> Self {
        //82BED780 initializes every retained affine frame to identity.
        Self {
            target: IDENTITY,
            world: IDENTITY,
            external_world: IDENTITY,
            board: IDENTITY,
            parent_world: IDENTITY,
            external_parent_world: IDENTITY,
            parent_board: IDENTITY,
            within_contact_bounds: false,
        }
    }
}

/// Prepare the animated frames after this limb's CalculateExternalIKOffset
/// stage. External/local modes retain their separately calculated frames.
/// The full controller calls both stages in native limb order.
pub fn prepare_animation_target(
    status: &mut LimbStatus,
    frames: &mut LimbFrames,
    binding: LimbBinding,
    animation: &[Transform; 24],
    animation_to_world: &Transform,
    inverse_animation_board: &Transform,
    contact_bounds: [f32; 4],
) {
    if status.mode != Mode::OnDeck && !(status.board_blend > 0.0) && status.external_blend >= 1.0 {
        return;
    }
    let animated = &animation[binding.part];
    frames.board = compose_affine(inverse_animation_board, &frames.target);
    status.part_position = frames.board[3];
    frames.world = compose_affine(animation_to_world, animated);
    // Native comparisons reject only a strict bound violation; NaNs do not
    // become a host-side exclusion. The fourth stored lane is excluded.
    frames.within_contact_bounds = (0..3).all(|i| {
        !(status.part_position[i] > contact_bounds[i]
            || -contact_bounds[i] > status.part_position[i])
    });
    if let Some(parent) = binding.parent_part {
        let relative = compose_affine(&inverse_rigid(animated), &animation[parent]);
        frames.parent_board = compose_affine(&frames.board, &relative);
        frames.parent_world = compose_affine(&frames.world, &relative);
    }
}

///82BEDF5C..DFEC and E1B4..E260: rigid inverse, transpose XYZ and zero W;
/// translation uses negative position with Z multiply, Y FMA, then X FMA.
/// This deliberately does not introduce a general scale/shear inverse.
pub fn inverse_rigid(source: &Transform) -> Transform {
    let mut result = IDENTITY;
    for axis in 0..3 {
        result[axis] = [source[0][axis], source[1][axis], source[2][axis], 0.0];
    }
    let translation = source[3].map(|v| 0.0 - v);
    result[3] = core::array::from_fn(|i| {
        let z = translation[2] * result[2][i];
        let y = translation[1].mul_add(result[1][i], z);
        translation[0].mul_add(result[0][i], y)
    });
    result
}
