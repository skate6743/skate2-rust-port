//! Offboard state slot40 at82DB6DCC, after Skeleton feedback82DB6DB8 and
//! before PostWipeoutCheck82DB6DF8 consumes the same request owner.
use crate::skate3_native::physics::{Skate3PhysicsState, Skate3ControllerState, biped_air, biped_ground, wipeout};
use skate3_core::player::{offboard::ground_lifecycle::CollisionInput, state::PhysicalStateId};

pub(crate) fn advance(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let state = skater.player_state.current();
    if !matches!(
        state,
        PhysicalStateId::BipedGround
            | PhysicalStateId::OffBoardPushing
            | PhysicalStateId::BipedAir
            | PhysicalStateId::LandingOnDeck
    ) {
        return Ok(());
    }
    //Borrow the already completed reports and pose errors. This produces no
    //contacts, default support, replacement dynamics, or second request owner.
    let frame = wipeout::Observations {
        processed: &skater.player_input.processed,
        board: &physics.riding.ground,
        collision: &skater.collision_feedback,
        deck: crate::skate3_native::physics::solve::deck_frame(&physics.board),
        input_board: skater.animated_skeleton.board_frames.animation_target,
        world_to_animation: skater.animated_skeleton.roots.world_to_animation,
        pose_error: skater.collision_pose_error,
        maximum_pose_error: skater.collision_maximum_error,
        jump_fix_frames: skater.player_state.post.jump_fix_frames,
        air: &skater.air_reckoning.state,
        system_up_y: physics.riding.reckoning_frames.system[1][1],
        grind_locked_to_middle: skater.trajectory.selector.grind_locked_to_middle(),
        grind_normal: skater.trajectory.selector.grind_normal(),
    }
    .frame()?;
    match state {
        PhysicalStateId::LandingOnDeck => crate::skate3_native::physics::landing_on_deck::post(skater, frame),
        PhysicalStateId::BipedAir => biped_air::post(skater, frame),
        PhysicalStateId::BipedGround | PhysicalStateId::OffBoardPushing => {
            let collision = CollisionInput {
                shared: &frame,
                skeleton_velocity_16336: skater.skeleton_input.root_velocity,
                contact_flag_4072: skater.collision_feedback.flags.group_8,
                contact_force_4056: skater.collision_feedback.maximum_group_8_force,
            };
            biped_ground::post(skater, collision);
            Ok(())
        }
        _ => unreachable!("offboard state checked before borrowing its observations"),
    }
}
