//!82D7C818 ordering. Every shared writeback precedes its next reader.
use super::super::{
    cadence, contact_correction, ground_branch, ground_motion, movement_intent, movement_velocity,
    position_output, slide, surface_frame,
};
use super::{GroundJob, Settings, State, Vector};
fn xyz(v: Vector) -> [f32; 3] {
    [v[0], v[1], v[2]]
}
pub(super) fn update(s: &mut State, c: &Settings, j: &GroundJob) {
    s.intent.speed = 0.0;
    s.intent.steering = 0.0;
    s.special
        .update(j.movement, j.flags, s.motion.frame_0[2][1]);
    s.contact.update(contact_correction::Input {
        collision_displacements: j.collision_displacements,
        projection_axis_416: s.surface.spring_normal,
        up_axis_16: s.motion.frame_0[1],
    });
    s.sliding.update(
        slide::Input {
            special_mode_714: s.special.enabled_714,
            surface_normal_560: s.surface.source_normal,
            movement_velocity_480: s.motion.velocity_480,
            contact_direction_400: s.contact.active.then_some(s.contact.direction),
        },
        &c.slide_vs_slope,
        &c.slide_vs_speed,
    );
    s.intent.update(
        &c.movement_intent,
        &movement_intent::Input {
            flags: j.flags,
            suppress_minimum: j.suppress_minimum,
            magnitude: j.movement,
            steering: j.steering,
            sprint_pressed: j.sprint_pressed,
            edge_active: j.edge_active,
            ignore_obstacle: j.ignore_obstacle,
            direction: j.desired_direction,
            edge_tangent: j.target_frame[2],
            edge_point: j.target_frame[3],
            right: s.motion.frame_0[0],
            up: s.motion.frame_0[1],
            forward: s.motion.frame_0[2],
            position: s.motion.frame_0[3],
            obstacle: s.contact.active,
            obstacle_normal: s.contact.direction,
            sliding: s.sliding.active_710,
            slide_velocity: s.sliding.velocity_528,
        },
    );
    s.alternate_709 = ground_branch::select_alternate(ground_branch::BranchInput {
        movement_velocity_480: s.motion.velocity_480,
        output_velocity_512: s.frame_output.velocity,
        right_0: s.motion.frame_0[0],
        position_48: s.motion.frame_0[3],
        contact_point_96: j.target_position,
        contact_flags_176: j.flags,
        suppressed_317: j.suppress_lean,
    });
    if s.alternate_709 {
        let forward = s.motion.frame_0[2];
        ground_branch::integrate_alternate(
            &mut s.motion.velocity_480,
            &mut s.motion.speed_704,
            &mut s.motion.frame_0[3],
            &mut s.frame_output,
            forward,
            s.surface.final_up,
            s.surface.spring_normal,
        );
        return;
    }
    velocity(s, c, j);
    s.surface.update_surface(&surface_frame::SurfaceInput {
        flags: j.flags,
        normal: j.contact_normal,
        origin: j.contact_position,
        edge_point: j.edge_position,
        edge_normal: j.edge_normal,
    });
    s.surface.update_spring(&surface_frame::SpringInput {
        velocity: s.motion.velocity_480,
        up: s.motion.frame_0[1],
        right: s.motion.frame_0[0],
        forward: s.motion.frame_0[2],
        spring_right: s.frame_output.frame[0],
        spring_forward: s.frame_output.frame[2],
        right_delta: s.right_delta_696,
        forward_delta: s.forward_delta_692,
        suppress_lean: j.suppress_lean,
    });
    ground_motion::update(
        &mut s.motion,
        &ground_motion::GroundMotionInput {
            contact_position_0: j.contact_position,
            contact_frame_32: j.support_frame,
            contact_target_96: j.target_position,
            contact_normal_112: j.target_normal,
            contact_flags_176: j.flags,
            contact_id_180: j.support_id,
            animation_motion_224: j.animation_motion,
            animation_motion_240: j.animation_velocity,
            requested_duration_288: j.requested_duration,
            mirrored_292: j.mirrored,
            animation_directed_304: j.animation_directed,
            target_frame_present_352: j.target_frame_present,
            target_frame_368: j.target_frame,
            reference_frame_128: s.frame_output.frame,
            contact_displacement_384: s.contact.displacement,
            velocity_addition_528: s.sliding.velocity_528,
            desired_up_544: s.surface.surface_normal,
            correction_target_592: s.correction_target_592,
            obstacle_target_672: s.intent.edge_target,
            obstacle_enabled_713: s.intent.edge_aligned,
        },
    );
    s.frame_output.update(
        s.motion.frame_0[2],
        s.surface.final_up,
        s.surface.spring_normal,
        s.motion.frame_0[3],
    );
    position_output::update_position(
        &mut s.position_368,
        position_output::PositionInput {
            previous_origin_112: s.motion.published_frame_64[3],
            override_origin_592: s.correction_target_592,
            use_override_711: s.motion.correction_enabled_711,
            projection_axis_416: s.surface.spring_normal,
            contact_flags_176: j.flags,
            contact_origin_0: j.contact_position,
            contact_origin_96: j.target_position,
            frame_position_176: s.frame_output.frame[3],
            animation_position_272: j.animation_position,
        },
    );
    s.cadence.update(
        &cadence::CadenceInput {
            motion_512: xyz(s.frame_output.velocity),
            motion_reference_272: xyz(s.motion.predicted_support_velocity_272),
            reject_axis_400: xyz(s.contact.direction),
            reject_enabled_708: s.contact.active,
            up_144: xyz(s.frame_output.frame[1]),
            frame_rows_0_16_32: [
                xyz(s.motion.frame_0[0]),
                xyz(s.motion.frame_0[1]),
                xyz(s.motion.frame_0[2]),
            ],
            frame_position_48: xyz(s.motion.frame_0[3]),
            animation_motion_224: xyz(j.animation_motion),
            requested_duration_288: j.requested_duration,
            requested_phase_296: j.requested_phase,
            suppress_adjustment_353: j.edge_active,
            contact_flags_176: j.flags,
            contact_point_96: xyz(j.target_position),
        },
        s.thresholds,
    );
}
fn velocity(s: &mut State, c: &Settings, j: &GroundJob) {
    let mut state = movement_velocity::State {
        velocity: s.motion.velocity_480,
        turn: s.motion.angular_velocity_688,
        forward_delta: s.forward_delta_692,
        right_delta: s.right_delta_696,
        speed: s.motion.speed_704,
        override_remaining: s.velocity_override_remaining_772,
    };
    state.update(
        &c.movement_velocity,
        &movement_velocity::Input {
            forward: s.motion.frame_0[2],
            right: s.motion.frame_0[0],
            plane_normal: s.surface.surface_normal,
            desired_speed: s.intent.speed,
            steering: s.intent.steering,
            slope_mode: s.special.enabled_714,
            obstacle: s.contact.active,
            obstacle_normal: s.contact.direction,
            override_gate: j.requested_phase,
            override_duration: j.override_duration,
            override_velocity: j.animation_velocity,
            flags: j.flags,
        },
    );
    s.motion.velocity_480 = state.velocity;
    s.motion.angular_velocity_688 = state.turn;
    s.motion.speed_704 = state.speed;
    s.forward_delta_692 = state.forward_delta;
    s.right_delta_696 = state.right_delta;
    s.velocity_override_remaining_772 = state.override_remaining;
}
