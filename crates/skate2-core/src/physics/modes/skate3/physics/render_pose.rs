//! Finish the actual physical pose and publish it to the single render skin.
//! Call after collision/error/COM output and before the next animation update.
use super::{Skate3PhysicsState, Skate3ControllerState, solve::deck_frame};
use skate3_core::animation::{foot_ik::post_physics, output};

pub(super) fn publish(
    physics: &Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    average_compression: [f32; 2],
) -> Result<(), String> {
    let deck = deck_frame(&physics.board);
    let p = &skater.player_input.processed;
    //82DB6DDC/6DF8 passes IsWipeout82DB9100 after state postphysics, not the
    //collision controller's ragdoll mode. Both correction and IK use this result.
    let wiping_out = skater.wipeout.requests_wipeout(p);
    //Consume the state producer's sole Skeleton16388 owner. Blocked branches
    //retain it; no per-tick copy may resurrect an already consumed request.
    skater.skeleton_output.correction.apply(
        &mut skater.skeleton,
        p.vectors_544_560_592_608[0].map(f32::from_bits),
        wiping_out,
        p.flags_2468,
        p.flags_2476,
    );
    let board = skater
        .skeleton_output
        .advance_wobble(&mut skater.skeleton, &deck);
    skater.foot_ik.post_physics(
        &mut skater.skeleton,
        post_physics::Input {
            state_id: p.state_2508,
            category_id: p.category_2512,
            board_body_flag_868: physics.riding.ground.part_contact_count != 0,
            wipeout: wiping_out,
            flags_2468: p.flags_2468,
            flags_2484: p.flags_2484,
            value_2664: p.state_timer_2664,
            state_2520: p.player_state_value_2520,
            world_to_animation: &skater.animated_skeleton.roots.world_to_animation,
            board: &board,
        },
    );
    let feet = skater.foot_physical.publish(
        &skater.skeleton.record,
        physics.settings.step.simulation.time_step,
    );
    let physical = &mut skater.player_input.physical;
    physical.skeleton.flag_597 = u8::from(skater.foot_ik.state.contacts.support_failed);
    physical.skeleton.flag_600 = u8::from(feet.within_deck_box[0]);
    physical.skeleton.flag_601 = u8::from(feet.within_deck_box[1]);
    physical.skeleton.anim_to_world_11920 = skater
        .animated_skeleton
        .roots
        .animation_to_world
        .map(|v| v.map(f32::to_bits));
    physical.reckoning.vector_16 = skater
        .animated_skeleton
        .board_frames
        .com_velocity
        .map(f32::to_bits);
    physical.reckoning.vector_64 = skater
        .animated_skeleton
        .board_frames
        .centre_of_mass
        .map(f32::to_bits);
    let up = physics.riding.reckoning.up;
    physical.reckoning.vector_96 = [up.x, up.y, up.z, 0.0].map(f32::to_bits);

    // Use this tick's evaluated animation as the base so unmapped bones keep
    // the real animation. The physical worker changes mapped bones and wheels.
    let mut globals=skater.animation.packet.hierarchy.clone();
    let mut locals=skater.animation.packet.local.clone();
    skater.skeleton_output.publish(
        &skater.animated_skeleton,
        &skater.skeleton,
        &physics.board,
        physics.settings.step.base_truck_transforms,
        skater.ground.steering.targets,
        average_compression,
        &mut globals,
        &mut locals,
    )?;
    // The renderer consumes render_pose as global matrices. Physical output
    // publishes final locals for mapped bones, so rebuild the complete global
    // hierarchy for every mode, including ragdoll.
    output::compose_hierarchy_in_place(
        locals.len() as i32,
        &skater.animation.evaluator.frames.parents,
        0,
        &mut locals,
    )
    .map_err(|e| format!("Physical render hierarchy: {e:?}"))?;
    skater.render_pose = locals;
    skater.pose_generation = skater.pose_generation.wrapping_add(1);
    Ok(())
}
