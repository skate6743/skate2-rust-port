//! CURRENT Skate3PhysicsState/Skate3ControllerState adapters. Shared scheduling belongs to main.
use super::super::{Skate3PhysicsState, Skate3ControllerState};
use super::{Family, lifecycle};

pub(crate) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    if skater.player_state.current() as u32 == 701 {
        // Original82D42D80: no feet/drag changes; reset the shared spin only here.
        skater.grind.nonspecific_active = true;
        skater.grind.nonspecific_jumped = false;
        skater.grind.nonspecific_jump_velocity = [0.; 4];
        skater.ground_lifecycle.skeleton_elapsed_16505 = true;
        skater.air_reckoning.state.spin_angle = 0.;
        skater.air_reckoning.state.spin_speed = 0.;
        physics
            .board
            .hook_mut()
            .drive
            .disable_animation(&mut skater.ground_lifecycle.board_animated_290);
        return Ok(());
    }
    let family = Family::from_physical_state(skater.player_state.current() as u32)
        .ok_or("Grind Enter requires selected state400..405")?;
    if skater.grind.active.is_some() {
        return Err("Grind Enter requires prior Exit".into());
    }
    let frame = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Grind Enter requires current board toolkit")?
        .deck;
    lifecycle::enter(
        &mut physics.board,
        &mut skater.ground_lifecycle.board_animated_290,
        &mut skater.ground_lifecycle.skeleton_elapsed_16505,
        &mut skater.skeleton_collision,
    );
    skater.grind.states[family as usize].enter(family, frame);
    skater.grind.active = Some(family);
    Ok(())
}

pub(crate) fn exit(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    if skater.grind.nonspecific_active {
        // Vtable8232861C+20 is82B61BB8 (empty), not common grind Exit.
        skater.grind.nonspecific_active = false;
        return Ok(());
    }
    let family = skater
        .grind
        .active
        .ok_or("Grind Exit requires active physical state")?;
    let collisions = [19, 15, 20, 16].map(|i| skater.skeleton.definition.bones[i].has_collision);
    lifecycle::exit(
        &mut physics.board,
        &mut skater.ground_lifecycle.board_animated_290,
        &mut skater.skeleton_collision,
        collisions,
        skater.grind.settings.standard_angular_drag,
    );
    // Common Exit82D3F448 clears the retained leaving byte96.
    skater.grind.states[family as usize].output.leaving = false;
    skater.grind.active = None;
    // Camera/chromosome history is updated by the ordinary output phase,
    // not reset by a physical-state transition.
    Ok(())
}

pub(crate) fn fill(_physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    if skater.grind.nonspecific_active {
        // Original82D430A0 only writes Air442/128 on this retained latch.
        if skater.grind.nonspecific_jumped {
            skater.player_input.physical.air.launch_velocity_128 =
                skater.grind.nonspecific_jump_velocity.map(f32::to_bits);
            skater.player_input.physical.air.launched_442 = 1;
        }
        return Ok(());
    }
    let manager = skater
        .grind
        .manager
        .ok_or("Grind Fill requires completed manager observation")?;
    let side_effects = skater
        .grind
        .fill_physical(&manager, &mut skater.player_input.physical.grinds)
        .ok_or("Grind Fill requires active state")?;
    if let Some(velocity) = side_effects.air_jump_velocity_128 {
        skater.player_input.physical.air.launch_velocity_128 = velocity.map(f32::to_bits);
        skater.player_input.physical.air.launched_442 = 1;
    }
    skater.grind.pending_wipeout_impulse = side_effects.wipeout_impulse_16;
    super::wipeout_output::publish(skater);
    Ok(())
}

pub(crate) fn post(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let observations = super::super::wipeout::Observations {
        processed: &skater.player_input.processed,
        board: &physics.riding.ground,
        collision: &skater.collision_feedback,
        deck: super::super::solve::deck_frame(&physics.board),
        input_board: skater.animated_skeleton.board_frames.animation_target,
        world_to_animation: skater.animated_skeleton.roots.world_to_animation,
        pose_error: skater.collision_pose_error,
        maximum_pose_error: skater.collision_maximum_error,
        jump_fix_frames: skater.player_state.post.jump_fix_frames,
        air: &skater.air_reckoning.state,
        system_up_y: physics.riding.reckoning_frames.system[1][1],
        grind_locked_to_middle: skater.trajectory.selector.grind_locked_to_middle(),
        grind_normal: skater.trajectory.selector.grind_normal(),
    };
    super::post::check(
        &mut skater.wipeout.state,
        skater.grind.settings.post,
        &observations,
    )
}
