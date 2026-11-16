//! The solver-result check that opens Skeleton::UpdatePostPhysics 82BD80D8, ahead of its
//! records and contact reports.
use super::{
    Skate3PhysicsState, skater::Skate3ControllerState, skeleton_input_runtime::SkeletonOwners,
    solve::deck_frame,
};
use skate3_core::animation::foot_ik::status::Mode;

/// 82BD80F8..82BD8154. SkeletonBody::IsBadData 82BE6B78 adds up the transforms of the 24
/// animation parts and reports a NaN in the sum. The skeleton then goes back to its animation
/// pose at rest (ResetSkeleton 82BE2798), the four IK limbs lose their mode and both blends with
/// the feet enabled, and Skeleton+0x407A asks for a respawn (FillPhysOut 82BE22E0..22FC makes it
/// State+69). The records 82BD9FB0 are taken from the reset parts.
pub(super) fn reset(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) -> bool {
    let mut sum = [[0.0f32; 4]; 4];
    for part in &skater.skeleton.part_transforms()[..24] {
        for (total, row) in sum.iter_mut().zip(part) {
            for (total, value) in total.iter_mut().zip(row) {
                *total += value;
            }
        }
    }
    if !sum.iter().any(|row| row[..3].iter().any(|value| value.is_nan())) {
        return false;
    }
    let mut owners = SkeletonOwners {
        animated: &mut skater.animated_skeleton,
        body: &mut skater.skeleton,
        drives: &mut skater.skeleton_drives,
        ik: &mut skater.foot_ik,
        animation_input: &mut skater.animation_input,
        correction: &mut skater.skeleton_output.correction,
        pose_errors: &mut skater.pose_errors,
    };
    skater
        .skeleton_input
        .reset_physical_pose(&mut owners, physics.settings.step.simulation);
    for limb in &mut skater.foot_ik.state.limbs {
        limb.board_blend = 0.0;
        limb.mode = Mode::Disabled;
        limb.external_blend = 0.0;
    }
    skater.foot_ik.state.enable_feet(true);
    skater.skeleton_input.invalid_target_reset = true;
    skater.skeleton.publish_physical_record(deck_frame(&physics.board));
    true
}
