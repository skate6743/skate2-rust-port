//! Ordered82D2EF38, including actual skeleton/selector/landing consumers.
use super::{Skate3PhysicsState, Skate3ControllerState, input};
use skate3_core::player::offboard::biped_air::recovered::{DT, State, height::projected_height};
pub(crate) fn update(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    _controls: &super::super::PlayerControls,
) -> Result<(), String> {
    let p = skater.player_input.processed;
    skater.ground_lifecycle.skeleton_controller.request(
        if p.flags_2484 & 2 != 0 { 7 } else { 4 },
        &mut skater.skeleton_collision,
    )?;
    let frame = skater.biped_air.state.begin_update();
    let mut feet = std::mem::take(&mut skater.offboard_feet);
    super::super::offboard::air_feet::update_air(skater, &mut feet);
    skater.offboard_feet = feet;
    //82C040F0(0): shared board steering helper, not Ground locomotion.
    skater.ground.steering.update(
        0.,
        skater.air_settings.steering_blend,
        p.flags_2468,
        p.flags_2472,
    );
    //82D2EFF0/82D2F014 load processed+0xB40 (AnimEndCOM), not
    //AnimTrans at0xB30. These are distinct stock animation attributes.
    if let Some(adjustment) = skater
        .biped_air
        .state
        .animation_adjustment(skater.animation_input.fields.animation_end_com)
    {
        skater.offboard_air_selector.adjust_animation(
            frame.wrapping_sub(1),
            adjustment,
            skater.biped_air.state.frame_144,
        );
    }
    skater.biped_air.state.result.reset();
    skater
        .offboard_air_selector
        .sample(frame, DT, &mut skater.biped_air.state.result);
    skater
        .biped_air
        .state
        .orient_sample(skater.animation_input.extra.biped_start_angle, p.flags_2476);
    let up = p.vectors_544_560_592_608[0].map(f32::from_bits);
    let position = p.vectors_544_560_592_608[2].map(f32::from_bits);
    let response = skater.biped_air.state.collision_response(
        skater.collision_extra_errors,
        up,
        skater
            .offboard_air_selector
            .core
            .sampling
            .restart_allowed_8493,
    );
    if response.request_52 {
        skater.wipeout.state.request(32, 0.);
    }
    if let Some(normal) = response.restart {
        let packet = input::launch_packet(skater)?;
        let feet = input::height(skater);
        let height = projected_height(feet.bone15, feet.bone19, position, up);
        let packet = skater
            .biped_air
            .state
            .restart_packet(packet, normal, position, up, height);
        input::submit(physics, skater, packet)?;
        let effective = input::effective_frame(skater);
        skater.biped_air.state.finish_restart(effective, normal);
    }
    skater.biped_air.state.correct_restarted_sample();
    let height = input::height(skater);
    skater.biped_air.state.correct_height(height);
    let context = input::context(skater);
    skater
        .offboard_air_selector
        .requery(&physics.world, context, p.flags_2472, p.flags_2488)
        .map_err(str::to_owned)?;
    skeleton(physics, skater)?;
    skater.biped_air.state.update_cadence(
        &mut skater.biped_ground.controller.state,
        skater.animation_input.fields.cadence_end_percent,
    );
    let scene = super::super::offboard::contact_toolkit::StaticScene::new(&physics.world)
        .map_err(str::to_owned)?;
    skater.landing_deck.assist(
        &scene,
        &skater.player_input,
        State::landing_assist_limit(p.state_timer_2664),
    )?;
    skater.biped_air.state.finish_landing_latch();
    Ok(())
}
fn skeleton(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    use super::super::{
        input_phase, offboard::skeleton_air, skeleton_input_runtime::SkeletonOwners,
    };
    let board_forward = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("BipedAir Skeleton requires the completed BoardToolkit")?
        .deck[2];
    let collision = input_phase::collision(skater);
    let state = &skater.biped_air.state;
    let input = skeleton_air::Input {
        frame_208: state.frame_208,
        trajectory_position_272: state.result.position_272,
        body_target_416: state.body_target_416,
        lift_436: state.body_offset_436,
        board_forward_96: board_forward,
    };
    let mut owners = SkeletonOwners {
        animated: &mut skater.animated_skeleton,
        body: &mut skater.skeleton,
        drives: &mut skater.skeleton_drives,
        ik: &mut skater.foot_ik,
        animation_input: &mut skater.animation_input,
        correction: &mut skater.skeleton_output.correction,
        pose_errors: &mut skater.pose_errors,
    };
    //82BDDFFC..82BDE010 calls the SAME82D8E3E0 as Ground.
    //Only this common helper is borrowed; Air owns all movement/root updates.
    let reckoning = &skater.biped_ground;
    let air_reckoning = &mut skater.air_reckoning;
    let riding = &mut physics.riding;
    skater.skeleton_input.update_biped_air(
        &mut skater.skeleton_air,
        &mut physics.board,
        input,
        &mut skater.player_input.processed,
        &mut owners,
        &skater.animation.packet.hierarchy,
        &collision,
        physics.settings.step.simulation,
        |update, processed, body_spin| {
            reckoning.finish_reckoning(update, riding, air_reckoning, processed, body_spin);
            Ok(())
        },
    )?;
    Ok(())
}
