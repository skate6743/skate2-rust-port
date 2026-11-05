//! Original Wipeout contact control82D3D660, using the world-space animation hands.
use super::{
    State,
    math::{self, V},
    profiles::Profile,
    torque,
};
use crate::{
    physics::{skeleton_animation_record::AnimationPartTransform, skeleton_body::SkeletonBody},
    trigonometry,
};
pub fn update(
    state: &mut State,
    body: &mut SkeletonBody,
    physical_com: V,
    effective: &AnimationPartTransform,
    processed: &crate::player::input_phase::ProcessedPhysicsInput,
    profile: &Profile,
    animation_to_world: &AnimationPartTransform,
    animation_pose: &[AnimationPartTransform; 24],
) {
    // 82D3D734 reads ProcessedPhysIn+464 (v6), NOT WipeoutState+464.
    // The latter is a world position and reverses rolling at different locations.
    let ground_axis_464 = processed.vectors_464_480_496_512_528[0].map(f32::from_bits);
    state.angular_velocity =
        torque::angular_velocity(&body.record, &body.definition.animation_masses.fractional);
    let speed = math::length(state.velocity);
    let mut rolling_axis = math::normalize_or(
        std::array::from_fn(|i| {
            let x = effective[0][i] * profile.roll_axis[0];
            let y = effective[1][i].mul_add(profile.roll_axis[1], x);
            effective[2][i].mul_add(profile.roll_axis[2], y)
        }),
        [0.0; 4],
    );
    let hands = [3, 7].map(|i| {
        crate::physics::skeleton_animation_record::compose_affine(
            animation_to_world,
            &animation_pose[i],
        )
    });
    let hands_close = math::length(math::sub(hands[0][3], hands[1][3])) <= 0.6;
    let mut alignment = [0.0; 4];
    if profile.align_ground_with_velocity {
        let axis = math::normalize_or(math::cross(ground_axis_464, state.velocity), [0.0; 4]);
        if math::dot(rolling_axis, axis) < 0.0 {
            rolling_axis = math::scale(rolling_axis, -1.0);
        }
        let perpendicular = math::cross(rolling_axis, axis);
        let sine = math::clamp(math::length(perpendicular), -0.9999, 0.9999);
        let angle = math::wrap_angle(trigonometry::asin(sine));
        let magnitude = math::clamp(angle * 1.2, 0.0, 1.0);
        if speed > 2.0 {
            let axis = math::normalize_or(perpendicular, [0.0; 4]);
            let current = math::normalize_or(state.angular_velocity, [0.0; 4]);
            alignment = math::scale(
                math::sub(
                    math::scale(axis, magnitude),
                    math::scale(axis, math::dot(current, axis)),
                ),
                profile.ground_align_torque,
            );
        }
    }
    let mut rolling = [0.0; 4];
    if hands_close && profile.roll_on_ground && speed > 2.0 {
        let current = math::normalize_or(state.angular_velocity, [0.0; 4]);
        rolling = math::scale(
            math::sub(
                math::scale(math::scale(rolling_axis, 1.5), state.forward_input),
                math::scale(rolling_axis, math::dot(current, rolling_axis)),
            ),
            profile.ground_roll_torque,
        );
    }
    torque::apply(body, physical_com, math::add(alignment, rolling));
}
