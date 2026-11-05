//! Original profile-driven Wipeout air rotation82D3CB50.
use super::{
    State,
    math::{self, V},
    orientation,
    profiles::Profile,
    torque,
};
use crate::{
    physics::{skeleton_animation_record::AnimationPartTransform, skeleton_body::SkeletonBody},
    trigonometry::sin_cos,
};
const UP: V = [0.0, 1.0, 0.0, 0.0];

pub fn update(
    state: &mut State,
    body: &mut SkeletonBody,
    physical_com: V,
    effective: &AnimationPartTransform,
    input: [f32; 2],
    profile: &Profile,
) {
    state.angular_velocity =
        torque::angular_velocity(&body.record, &body.definition.animation_masses.fractional);
    let radians = f32::from_bits(0x3C8E_FA35);
    let tilt_input = [
        (input[1] * profile.tilt_degrees) * radians,
        0.0,
        (input[0] * profile.tilt_degrees) * -radians,
        0.0,
    ];
    state.retained_tilt = math::madd(tilt_input, 0.05, math::scale(state.retained_tilt, 0.95));
    let tilt_length = math::length(state.retained_tilt);
    let mut tilt_rotation = [0.0, 0.0, 0.0, 1.0];
    if tilt_length > 0.01 {
        let (sin, cos) = sin_cos(tilt_length * 0.5);
        tilt_rotation = math::scale(
            math::scale(state.retained_tilt, math::reciprocal(tilt_length)),
            sin,
        );
        tilt_rotation[3] = cos;
    }
    let desired = transform(
        effective,
        orientation::alignment_up(profile.align_euler[0], profile.align_euler[1]),
    );
    let side = math::normalize_or(state.right, [0.0; 4]);
    let projected_velocity = math::sub(
        state.velocity,
        math::scale(side, math::dot(state.velocity, side)),
    );
    let direction = if math::dot(state.forward, projected_velocity) <= 0.0 {
        math::normalize(math::add(desired, [0.0, -100.0, 0.0, 0.0]))
    } else {
        math::normalize_or(projected_velocity, desired)
    };
    let tilted_direction = orientation::rotate(tilt_rotation, direction);
    let axis = math::normalize_or(math::cross(tilted_direction, desired), desired);
    let angle = math::wrap_angle(orientation::signed_angle(tilted_direction, desired, axis));
    let distance_torque = if profile.align_with_velocity {
        math::scale(math::scale(axis, angle), profile.torque_distance)
    } else {
        [0.0; 4]
    };

    let input_side_axis = orientation::rotate(tilt_rotation, desired);
    let input_forward_axis = orientation::rotate(
        tilt_rotation,
        math::normalize_or(math::cross(direction, UP), [0.0; 4]),
    );
    state.retained_sideways_input = retain_input(
        state.sideways_input,
        state.retained_sideways_input,
        profile.spin_inertia,
    );
    state.retained_forward_input = retain_input(
        state.forward_input,
        state.retained_forward_input,
        profile.spin_inertia,
    );
    let side_spin = profile.sideways_spin * state.retained_sideways_input;
    let forward_spin = profile.forward_spin * state.retained_forward_input;
    let spin = math::madd(
        input_forward_axis,
        forward_spin,
        math::madd(input_side_axis, side_spin, [0.0; 4]),
    );
    if math::dot(spin, state.angular_velocity) <= 0.0 {
        state.control_time = 0.0;
    }
    let spin = math::scale(spin, profile.spin_vs_time.evaluate(state.control_time));
    let horizontal = if side_spin.abs() <= 0.1 {
        horizontal_alignment(profile, effective, direction, tilt_rotation)
    } else {
        [0.0; 4]
    };
    let control = math::add(
        math::madd(
            math::sub(spin, state.angular_velocity),
            profile.torque_velocity,
            distance_torque,
        ),
        horizontal,
    );
    torque::apply(body, physical_com, control);
}

fn retain_input(input: f32, previous: f32, inertia: f32) -> f32 {
    if input.abs() >= previous.abs() {
        input
    } else {
        (1.0 - inertia) * input + previous * inertia
    }
}

fn horizontal_alignment(
    profile: &Profile,
    effective: &AnimationPartTransform,
    direction: V,
    tilt: V,
) -> V {
    let local = math::normalize_or(profile.horizontal_axis, [1.0, 0.0, 0.0, 0.0]);
    let world = transform(effective, local);
    let mut target = orientation::rotate(
        tilt,
        math::normalize_or(math::cross(direction, UP), [0.0; 4]),
    );
    if !profile.horizontal_directed {
        target = math::scale(
            target,
            if math::dot(world, target) > 0.0 {
                1.0
            } else {
                -1.0
            },
        );
    }
    //82D3D4F0 uses the normalized authored axis here, before world composition;
    //the angle query itself receives world/target. Preserve that distinction.
    let axis = math::normalize_or(math::cross(local, target), [0.0; 4]);
    if !(math::dot(axis, axis) > 0.9) {
        return [0.0; 4];
    }
    let angle = math::wrap_angle(orientation::projected_angle(world, target, axis));
    let limit = f32::from_bits(0x3F49_0FDB);
    math::scale(axis, math::clamp(angle, -limit, limit) * 0.8)
}

fn transform(frame: &AnimationPartTransform, vector: V) -> V {
    math::madd(
        frame[2],
        vector[2],
        math::madd(frame[1], vector[1], math::scale(frame[0], vector[0])),
    )
}
