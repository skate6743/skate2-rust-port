//! Skeleton82BDEC48 (anchored foot) and82BDF090 (plant COM).
use super::{Skate3PhysicsState, Skate3ControllerState, input_phase, skeleton_input_runtime::SkeletonOwners};
use skate3_core::{
    math::Vector3,
    physics::{
        board::BodyId,
        board_animation::target_velocity,
        skeleton_air_frames::update_plant_roots,
        skeleton_animation_record::{IDENTITY, compose_affine},
    },
};

pub(super) fn advance(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    anchor: [f32; 4],
    bone: Option<usize>,
) -> Result<(), String> {
    let collision = input_phase::collision(skater);
    let p = &mut skater.player_input.processed;
    let s = &mut skater.animated_skeleton;
    let local = bone.map_or(s.record.centre_of_mass, |i| {
        skater.skeleton_input.drive_frames[i][3]
    });
    update_plant_roots(
        &mut s.roots,
        &physics.riding.reckoning_frames.system,
        anchor,
        local,
    );
    s.motion.trajectory = IDENTITY;
    s.motion.inverse_trajectory = IDENTITY;
    let local_board = if bone.is_some() {
        &s.record.pose[0]
    } else {
        &skater.skeleton_input.drive_frames[0]
    };
    let target = compose_affine(&s.roots.animation_to_world, local_board);
    p.flags_2468 |= 1 << 19;
    s.board_frames.animation_target = target;
    let effective = skater
        .skeleton_air
        .apply_board(&mut physics.board, &target, true);
    s.board_frames.physical_board = effective;
    s.board_frames.skate_root = effective;
    if bone.is_none() {
        let pos = physics.board.bodies()[BodyId::Deck.index()].rates.position;
        let velocity = target_velocity(effective[3], [pos.x, pos.y, pos.z, 0.0], p.timestep_2604);
        for body in physics.board.bodies_mut() {
            body.rates.linear_velocity = Vector3::new(velocity[0], velocity[1], velocity[2]);
        }
    }
    s.board_frames.update_com_lift(
        &s.roots.animation_to_world,
        s.board_frames.centre_of_mass,
        0.0,
    );
    let mut owners = SkeletonOwners {
        animated: s,
        body: &mut skater.skeleton,
        drives: &mut skater.skeleton_drives,
        ik: &mut skater.foot_ik,
        animation_input: &mut skater.animation_input,
        correction: &mut skater.skeleton_output.correction,
        pose_errors: &mut skater.pose_errors,
    };
    skater.skeleton_input.general_update(
        p,
        &mut owners,
        &skater.animation.packet.hierarchy,
        &collision,
        physics.settings.step.simulation,
    )?;
    owners.animated.finish_ground();
    owners.animation_input.fields.flags2468 = p.flags_2468;
    Ok(())
}

pub(super) fn hold_foot(skater: &mut Skate3ControllerState, right: bool, position: [f32; 4], frames: u32) {
    let side = usize::from(right);
    skater.foot_ik.state.external_targets[side].world_position = position;
    skater.foot_ik.state.limbs[side].external_target_set = true;
    skater.foot_ik.state.limbs[side].target_blend = 1.0;
    if frames != 0 {
        for part in if right { [19, 20, 21] } else { [15, 16, 17] } {
            skater.skeleton_collision.pending_reenable = true;
            skater.skeleton_collision.disable_count[part] = frames;
            skater.skeleton_collision.parts[part].enabled = false;
        }
    }
}
