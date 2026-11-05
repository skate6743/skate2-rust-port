//! Original82BD8F10 writes the existing AnimatedSkeleton board-offset owner.
use crate::skate3_native::physics::animated_skeleton::AnimatedSkeleton;
use skate3_core::{
    physics::skeleton_animation_record::AnimationPartTransform,
    player::{input_phase::ProcessedPhysicsInput, offboard::pose_adjust},
};
/// Reparented indices are the existing target_bones[2] and[3] loaded by82BED688.
/// No state is changed when category500/state503 eligibility is false.
pub(crate) fn update(
    animated: &mut AnimatedSkeleton,
    globals: &[AnimationPartTransform],
    reparented_hands: [usize; 2],
    processed: &ProcessedPhysicsInput,
) -> Result<(), String> {
    if processed.category_2512 != 500 || processed.state_2508 == 503 {
        return Ok(());
    }
    let hand = pose_adjust::selected_hand(processed.flags_2476);
    let actual_index = animated.bone_indices[[3, 7][hand]];
    let actual = globals
        .get(actual_index)
        .ok_or("Off-board pose missing actual animation hand")?;
    let reparented = globals
        .get(reparented_hands[hand])
        .ok_or("Off-board pose missing board-parented animation hand")?;
    animated
        .board_offset
        .refresh_transform(pose_adjust::adjustment(actual, reparented));
    Ok(())
}
