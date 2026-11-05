//! Same live Skeleton owners as Ground; Air chooses its original update method.
use super::*;
use crate::skate3_native::physics::{input_phase, skeleton_input_runtime::SkeletonOwners};
pub(super) fn advance(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    com: Option<[f32; 4]>,
) -> Result<(), String> {
    let collision = input_phase::collision(skater);
    let mut owners = SkeletonOwners {
        animated: &mut skater.animated_skeleton,
        body: &mut skater.skeleton,
        drives: &mut skater.skeleton_drives,
        ik: &mut skater.foot_ik,
        animation_input: &mut skater.animation_input,
        correction: &mut skater.skeleton_output.correction,
        pose_errors: &mut skater.pose_errors,
    };
    let packet = &skater.animation.packet;
    if let Some(com) = com {
        skater.skeleton_input.update_known_air(
            &mut skater.skeleton_air,
            &mut physics.board,
            &physics.riding.reckoning_frames.system,
            com,
            packet,
            &mut skater.player_input.processed,
            &mut owners,
            &packet.hierarchy,
            &collision,
            physics.settings.step.simulation,
        )?;
    } else {
        skater.skeleton_input.update_animated(
            &mut skater.skeleton_air,
            &mut physics.board,
            &physics.riding.reckoning_frames.system,
            &mut skater.player_input.processed,
            &mut owners,
            &packet.hierarchy,
            &collision,
            physics.settings.step.simulation,
            false,
        )?;
    }
    Ok(())
}
