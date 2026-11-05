//! Complete Wipeout300 update order82D3B9C8, on the game body/pose owners.
use super::*;
use crate::skate3_native::physics::{Skate3PhysicsState, Skate3ControllerState, skeleton_input_runtime::SkeletonOwners};
use skate3_core::{
    math::Vector3,
    physics::board::BodyId,
    player::wipeout_state::{
        body, control_air, control_ground, control_profile_air, math, profiles, response, weights,
    },
    riding::ground_correction_math::apply_world_force,
};
const DT: f32 = f32::from_bits(0x3C88_8889);

pub(crate) fn advance(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let p = &skater.player_input.processed;
    let runtime = &mut skater.wipeout_state;
    let state = &mut runtime.state;
    let contact = skater.collision_feedback.flags.any || state.below_surface;
    state.airborne_frames = if contact {
        -1
    } else {
        state.airborne_frames.wrapping_add(1)
    };
    if !state.special_surface && p.flags_2488 & 0x4000_0000 != 0 {
        state.special_surface = true;
        state.surface_height = p.collision_scalar_2924;
        runtime.ragdoll.request(
            &mut skater.ground_lifecycle.skeleton_controller,
            10,
            &mut skater.skeleton,
            &mut skater.skeleton_joints,
            &mut skater.skeleton_collision,
        )?;
    }
    let velocity = p.vectors_544_560_592_608[3].map(f32::from_bits);
    if velocity[1] > 5.0 && velocity[1] - state.velocity[1] > 5.0 {
        body::limit_velocity(&mut skater.skeleton, 5.0);
    }
    state.velocity = velocity;
    let speed = math::length(velocity);
    response::retained_velocity(
        state,
        &mut skater.skeleton,
        &mut skater.skeleton_drives,
        &runtime.drives,
        contact,
        p.flags_2468,
        velocity,
    );
    let position = p.vectors_544_560_592_608[2].map(f32::from_bits);
    if state.special_surface {
        state.below_surface = position[1] - state.surface_height < 0.0;
        body::special_surface(&mut skater.skeleton, state.surface_height);
    }
    let effective = p
        .effective_anim_transform_192
        .map(|row| row.map(f32::from_bits));
    let controls = skater.animation_input.extra.wipeout_control;
    let gesture = skater.animation_input.extra.wipeout_gesture;
    if !state.below_surface {
        contact::update(
            state,
            &mut runtime.contact,
            &mut skater.skeleton,
            &skater.collision_feedback,
            &mut skater.ground_lifecycle.skeleton_controller,
            &mut skater.skeleton_collision,
            &mut skater.skeleton_joints,
            &runtime.ragdoll,
            contact,
            velocity,
            effective[2],
            controls,
        )?;
        let drag = if state.slow && contact {
            if state.over {
                math::clamp(state.settled_time * 0.15 + 0.05, 0.0, 0.5)
            } else {
                0.05
            }
        } else {
            0.0
        };
        body::set_wipeout_drag(&mut skater.skeleton, drag);
    }
    state.time += DT;
    state.response_time += DT;
    state.extra_weight_zero_time += DT;
    if state.extra_weight > 0.0 {
        state.extra_weight_zero_time = 0.0;
    }
    state.response_scalar = 0.0;
    state.slow_time += DT;
    state.maximum_speed = if speed - state.maximum_speed >= 0.0 {
        speed
    } else {
        state.maximum_speed
    };
    if speed > 2.0 {
        state.slow_time = 0.0;
    }
    response::update_counter(state, contact);
    let com = skater.animated_skeleton.board_frames.centre_of_mass;
    let mut control_applied = false;
    if state.below_surface || state.imminent_surface_twelve {
        control_air::update(state, &mut skater.skeleton, com, controls, true);
    } else if p.flags_2484 & 0x100 != 0 && state.response_time >= 0.1 {
        let normal = response::response_normal(
            skater
                .collision_feedback
                .flags
                .compliant
                .then_some(skater.collision_feedback.highest_normal),
            runtime
                .prediction
                .result
                .valid()
                .then_some(runtime.prediction.result.landing_normal),
        );
        response::trigger(state, &mut skater.skeleton, normal, controls);
    } else if state.airborne_frames > 15 && !state.over {
        profiles::prepare(state, &runtime.profiles, gesture, controls);
        let profile = &runtime.profiles[state.profile];
        control_profile_air::update(
            state,
            &mut skater.skeleton,
            com,
            &effective,
            controls,
            profile,
        );
        profiles::drift(state, &mut skater.skeleton, profile);
        control_applied = true;
    } else if (!contact && state.airborne_frames > 15) || state.over {
        control_applied = control_air::update(state, &mut skater.skeleton, com, controls, contact);
    } else {
        profiles::prepare(state, &runtime.profiles, gesture, controls);
        control_ground::update(
            state,
            &mut skater.skeleton,
            com,
            &effective,
            p,
            &runtime.profiles[state.profile],
            &skater.animated_skeleton.roots.animation_to_world,
            &skater.animated_skeleton.record.pose,
        );
        control_applied = true;
    }
    let weight = weights::update(
        state,
        weights::Settings {
            remove_target_time: runtime.settings.remove_target_time,
            remove_drives_time: runtime.settings.remove_drives_time,
            collision_step: runtime.settings.collision_weight_step,
            controlled_step: runtime.settings.controlled_weight_step,
        },
        contact,
        control_applied,
        p.flags_2472,
        gesture,
    );
    skater.ground.steering.update(
        0.0,
        skater.air_settings.steering_blend,
        p.flags_2468,
        p.flags_2472,
    );
    drives::set_angular_root(
        &mut skater.skeleton_drives,
        &runtime.drives,
        weight.target_weight,
    );
    let mut owners = SkeletonOwners {
        animated: &mut skater.animated_skeleton,
        body: &mut skater.skeleton,
        drives: &mut skater.skeleton_drives,
        ik: &mut skater.foot_ik,
        animation_input: &mut skater.animation_input,
        correction: &mut skater.skeleton_output.correction,
        pose_errors: &mut skater.pose_errors,
    };
    skeleton::update(
        &mut skater.skeleton_input,
        &mut owners,
        p,
        &skater.animation.packet.hierarchy,
        &runtime.drives,
        weight.start,
        weight.end,
        weight.controlled,
        weight.extra,
        state.retained_velocity_active,
    )?;
    if state.move_board && (state.board_move_frames as i32) < 21 {
        let deck = physics.board.part_transforms()[BodyId::Deck.index()].translation;
        let point = skater
            .player_input
            .toolkit
            .as_ref()
            .ok_or("Wipeout board force requires the current BoardToolkit")?
            .deck[3];
        apply_world_force(
            &mut physics.board.bodies_mut()[BodyId::Deck.index()],
            deck,
            vector(state.board_offset),
            vector(point),
        );
        state.board_move_frames = state.board_move_frames.wrapping_add(1);
    }
    state.manage_recovery(
        p.flags_2468,
        p.flags_2472,
        p.flags_2484,
        p.timestep_2604,
        &runtime.settings.recovery,
    );
    // PC port: in an online turn the wipeout ends by the game's limit at the latest, however long
    // Hall of Meat moves keep the body responding, as Skate 2's WipeoutGround tail takes it.
    if let Some(limit) = physics.turn_bail_limit
        && !state.prevent_manual
        && !state.teleport_pending
        && state.time > limit
    {
        state.teleport_pending = true;
        state.time_until_teleport = runtime.settings.recovery.fade_time;
        crate::diagnostics::test_log::event("skate3_turn_bail_limit", format_args!(
            "wipeout time {:.3} passed limit {limit:.3}; reset in {:.3}", state.time, state.time_until_teleport));
    }
    let gravity = physics.settings.step.simulation.gravity_acceleration;
    runtime.prediction.advance(
        state,
        physics.world(),
        position,
        [gravity.x, gravity.y, gravity.z, 0.0],
    )
}
fn vector(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
