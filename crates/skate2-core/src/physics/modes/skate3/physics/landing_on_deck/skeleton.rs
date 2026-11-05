//! Skeleton82BDE428: old-root COM, new root, shared drives, shared reckoning.
use super::{Skate3PhysicsState, Skate3ControllerState};
use crate::skate3_native::physics::{
    input_phase, offboard::skeleton_ground::ReckoningUpdate, skeleton_input_runtime::SkeletonOwners,
};
use skate3_core::physics::{
    skeleton_animation_record::{IDENTITY, compose_affine},
    skeleton_landing_on_board as native,
};
pub(super) fn update(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    position: [f32; 4],
) -> Result<(), String> {
    let p = &mut skater.player_input.processed;
    let board = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Landing skeleton requires the completed board toolkit")?
        .deck;
    let s = &mut skater.animated_skeleton;
    let old = s.roots.animation_to_world;
    s.roots.initialize_heading = true;
    s.board_frames.skate_root = native::skate_root(
        old,
        skater.skeleton.record.centre_of_mass,
        p.flags_2480,
        f32::from_bits(p.vectors_880_896_912_928_944[0][1]),
        skater.landing_on_deck.root_settings,
    );
    s.board_frames
        .update_com_lift(&old, s.board_frames.centre_of_mass, 0.);
    s.motion.trajectory = IDENTITY;
    //82BE090C..0950 resets only packet bone0; inverse trajectory12112 survives.
    let packet = &mut skater.animation.packet;
    *packet
        .hierarchy
        .first_mut()
        .ok_or("Landing packet has no trajectory bone")? = IDENTITY;
    *packet
        .local
        .first_mut()
        .ok_or("Landing packet has no local trajectory bone")? = IDENTITY;
    let revert =
        (packet.flags & 0x10000000 != 0).then_some(packet.air_dismount_revert_frames as u32);
    native::update_root(
        &mut s.roots,
        position,
        s.record.centre_of_mass,
        skater.landing_on_deck.state.applied_spin,
        revert,
        p.flags_2476 & 4 != 0,
    );
    let target = compose_affine(
        &s.roots.animation_to_world,
        &skater.skeleton_input.drive_frames[0],
    );
    s.board_frames.animation_target = target;
    p.flags_2468 |= 0x80000;
    if p.flags_2480 & 0x8000 != 0 {
        s.board_frames.physical_board =
            skater
                .skeleton_air
                .apply_board(&mut physics.board, &target, true);
    }
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
    skater.skeleton_input.general_update(
        &skater.player_input.processed,
        &mut owners,
        &skater.animation.packet.hierarchy,
        &collision,
        physics.settings.step.simulation,
    )?;
    skater.biped_ground.finish_reckoning(
        ReckoningUpdate {
            up: owners.animated.roots.animation_to_world[1],
            forward: board[2],
            blend: 0.5,
        },
        &mut physics.riding,
        &mut skater.air_reckoning,
        &skater.player_input.processed,
        owners.animation_input.extra.physical_body_spin,
    );
    owners.animated.finish_ground();
    owners.animation_input.fields.flags2468 = skater.player_input.processed.flags_2468;
    Ok(())
}
