//! Live BipedAir501 Enter/Update/Exit and FillPhysOut.
use crate::skate3_native::physics::{Skate3PhysicsState, Skate3ControllerState};
use skate3_core::{
    physics::skeleton_animation_record::{AnimationPartTransform as Frame, compose_affine},
    player::offboard::{air_state::State, controller::PlacementInput},
};
type V = [f32; 4];
const DT: f32 = f32::from_bits(0x3c888889);
fn effective(skater: &Skate3ControllerState) -> Frame {
    let mut frame = skater.animated_skeleton.roots.animation_to_world;
    if skater.player_input.processed.flags_2476 & 4 != 0 {
        for axis in [0, 2] {
            frame[axis] = frame[axis].map(|v| -v);
        }
    }
    frame
}
pub(crate) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    skater.offboard.air_diagnostics.clear();
    let frame = effective(skater);
    let p = &skater.player_input.processed;
    let body = skater.animated_skeleton.board_frames.com_frame[3];
    skater.offboard.air_state = State::enter(
        frame,
        p.flags_2484,
        body,
        skater.animated_skeleton.board_frames.lifted_com_frame[3],
        p.vectors_544_560_592_608[2].map(f32::from_bits),
        p.vectors_544_560_592_608[0].map(f32::from_bits),
    );
    if !matches!(p.state_2504, 500..=502) {
        skater.offboard.feet.reset();
        skater.foot_ik.state.enable_feet(false);
        for limb in &mut skater.foot_ik.state.limbs {
            limb.board_blend = 0.;
            limb.external_blend = 0.;
            limb.mode = skate3_core::animation::foot_ik::status::Mode::Disabled;
        }
    }
    skater.offboard.feet.flags_304_to_307[0] = false;
    skater
        .ground_lifecycle
        .skeleton_controller
        .request(4, &mut skater.skeleton_collision)?;
    if skater.offboard.air_prediction.is_none() {
        let launch = super::air_trajectory::prepare(skater, false, 1.0)?;
        super::air_trajectory::launch(physics, skater, launch)?;
    }
    let p = &skater.player_input.processed;
    skater.offboard.controller.place(PlacementInput {
        frame,
        velocity: p.vectors_544_560_592_608[3].map(f32::from_bits),
        body_position: body,
        current_state: 501,
        previous_state: p.state_2504,
        previous_frame: p
            .effective_anim_transform_192
            .map(|v| v.map(f32::from_bits)),
    });
    Ok(())
}
pub(crate) fn exit(_physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) {
    skater.offboard.air_prediction = None;
}
pub(crate) fn update(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let kind = if skater.player_input.processed.flags_2484 & 2 != 0 {
        7
    } else {
        4
    };
    skater
        .ground_lifecycle
        .skeleton_controller
        .request(kind, &mut skater.skeleton_collision)?;
    super::feet::update_air(skater);
    let p = &skater.player_input.processed;
    skater.ground.steering.update(
        0.,
        skater.air_settings.steering_blend,
        p.flags_2468,
        p.flags_2472,
    );
    let mut state = std::mem::take(&mut skater.offboard.air_state);
    state.tick += 1;
    let prediction = skater
        .offboard
        .air_prediction
        .as_mut()
        .ok_or("BipedAir has no launched trajectory")?;
    let animation = skater.animation_input.fields.animation_end_com;
    if animation[1] > 0.1 {
        if state.animation_adjusted {
            let delta = sub(animation, state.animation_offset);
            if dot(delta, delta) > 0.001 {
                let length = dot(delta, delta).sqrt();
                state.animation_offset = madd(
                    delta,
                    if length > 0.2 { 0.2 / length } else { 1. },
                    state.animation_offset,
                );
                prediction.adjust_animation(
                    state.tick - 1,
                    state.animation_offset,
                    state.target,
                    skater.offboard.air_query_settings.sphere_radius,
                );
            }
        } else if state.aligned {
            prediction.adjust_animation(
                state.tick - 1,
                animation,
                state.target,
                skater.offboard.air_query_settings.sphere_radius,
            );
            state.animation_adjusted = true;
            state.animation_offset = animation;
        }
    }
    let packet = prediction.packet(
        state.tick,
        p.timestep_2604,
        &skater.offboard.air_blend_curve,
    );
    let can_requery = prediction.can_requery;
    state.consume(
        packet,
        skater.animation_input.extra.biped_start_angle,
        p.flags_2476 & 4 != 0,
    );
    let up = p.vectors_544_560_592_608[0].map(f32::from_bits);
    let position = p.vectors_544_560_592_608[2].map(f32::from_bits);
    if let Some(relaunch) = skate3_core::player::offboard::air_collision::response(
        &mut state,
        skater.collision_extra_displacements,
        up,
        can_requery,
        &mut skater.wipeout.state,
    ) {
        let mut launch = super::air_trajectory::prepare(skater, false, 1.0)?;
        launch.velocity = relaunch.velocity;
        launch.has_target = true;
        let feet = feet_world(skater);
        let height = feet.map(|v| dot(sub(v, position), up));
        launch.target = madd(up, height[0].min(height[1]), position);
        launch.position = madd(relaunch.velocity, DT, position);
        state.collision_normal = relaunch.normal;
        super::air_trajectory::launch(physics, skater, launch)?;
        state.tick = 0;
        state.aligned = false;
        state.animation_adjusted = false;
        state.start = effective(skater);
        state.launch_up = relaunch.normal;
    }
    if state.collision_adjusted {
        let delta = state.packet.velocity.map(|v| v * DT);
        let normal = state.collision_normal.map(|v| -v);
        let amount = dot(delta, normal).max(0.);
        state.packet.position = sub(state.packet.position, normal.map(|v| v * amount));
    }
    let p = &skater.player_input.processed;
    let mapped = compose_affine(
        &skater.animated_skeleton.roots.animation_to_world,
        &skater.skeleton_input.drive_frames[1],
    )[3];
    state.update_body(
        up,
        p.vectors_544_560_592_608[3].map(f32::from_bits),
        feet_world(skater),
        mapped,
    );
    super::air_trajectory::requery(physics, skater)?;
    super::skeleton_air::update(
        physics,
        skater,
        state.frame,
        state.packet.position,
        state.body_position,
        state.lift,
    )?;
    if state.packet.contact {
        let c = &mut skater.offboard.controller.state;
        c.correction_target_592 = state.packet.target;
        let phase = skater.animation_input.fields.cadence_end_percent;
        if phase >= 0. && state.remaining > 0. {
            let delta = phase - c.cadence.phase.phase;
            c.cadence.phase.forward_target = true;
            c.cadence.phase.target = phase;
            c.cadence.phase.duration = Some(state.remaining);
            c.cadence.phase.rate = (if delta < 0. { delta + 1. } else { delta }) / state.remaining;
        }
        c.cadence.phase.advance();
        let delta = sub(state.packet.target, c.motion.frame_0[3]);
        state.landing_direction = [
            dot(c.motion.frame_0[0], delta),
            dot(c.motion.frame_0[1], delta),
            dot(c.motion.frame_0[2], delta),
            0.,
        ];
    } else {
        state.landing_direction = [0., 0., 1., 0.];
    }
    state.finish();
    skater.offboard.air_state = state;
    Ok(())
}
fn feet_world(skater: &Skate3ControllerState) -> [V; 2] {
    [15, 19].map(|i| {
        compose_affine(
            &skater.animated_skeleton.roots.animation_to_world,
            &skater.animated_skeleton.record.pose[i],
        )[3]
    })
}
fn dot(a: V, b: V) -> f32 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn madd(a: V, s: f32, b: V) -> V {
    std::array::from_fn(|i| a[i].mul_add(s, b[i]))
}
pub(crate) fn publish(skater: &mut Skate3ControllerState) {
    let state = &skater.offboard.air_state;
    let out = &mut skater.player_input.physical.off_board;
    out.vector_64 = state.packet.velocity.map(f32::to_bits);
    out.scalar_32 = state.remaining;
    out.flag_328 = u8::from(state.airborne);
    out.landed_320 = u8::from(state.remaining <= DT);
    out.flag_331 = 1;
    out.air_duration_92 = state.duration;
    out.landing_direction_96 = state.landing_direction;
    out.air_surface_144 = state.packet.surface_kind;
    out.impact_y_148 = state.packet.impact_y;
    out.air_time_152 = state.tick as f32 * DT;
    out.apex_time_156 = state.packet.apex_time;
    out.launch_up_160 = state.launch_up;
    out.launch_position_176 = state.start[3];
    out.landing_normal_192 = state.packet.normal;
    out.landing_position_208 = state.packet.target;
    out.landing_forward_224 = state.target[2];
    out.apex_position_240 = state.packet.apex;
    out.flags_306_307 = skater.offboard.feet.hands.map(|f| {
        u8::from((f.flags_104_to_107[1] && !f.flags_104_to_107[2]) || f.flags_104_to_107[3])
    });
}
