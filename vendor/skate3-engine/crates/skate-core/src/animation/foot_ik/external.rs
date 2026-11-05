//! SkeletonIK::CalculateExternalIKOffset82BEE390.
use super::{
    math::{axis_rotation, cross, length, limit_length, reciprocal},
    status::{LimbStatus, Mode},
    transforms::{LimbBinding, LimbFrames, inverse_rigid},
};
use crate::{
    physics::skeleton_animation_record::{
        AnimationPartTransform as Transform, IDENTITY, compose_affine, transform_point,
    },
    trigonometry::asin,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct ExternalTarget {
    /// SkeletonIK544 /608; setters distinguish the supplied coordinate space.
    pub world_position: [f32; 4],
    pub animation_position: [f32; 4],
    /// SkeletonIK736,800,828. The normal is measured against world up.
    pub normal: [f32; 4],
    pub normal_set: bool,
    pub normal_blend: f32,
}

pub fn update(
    target: &mut ExternalTarget,
    status: &mut LimbStatus,
    frames: &mut LimbFrames,
    binding: LimbBinding,
    animation: &[Transform; 24],
    animation_to_world: &Transform,
    world_to_animation: &Transform,
) {
    let animated = &animation[binding.part];
    let original = compose_affine(animation_to_world, animated);
    let mut rotation = IDENTITY;
    if target.normal_blend > 0.0 {
        // Native v16 is the fixed Y basis82139A20, not the animated limb's up.
        let axis = cross([0.0, 1.0, 0.0, 0.0], target.normal);
        let magnitude = length(axis);
        if magnitude > 0.05 {
            let inverse = reciprocal(magnitude, 2);
            let axis = axis.map(|v| inverse * v);
            let angle = asin(magnitude);
            let angle = if -0.71 - angle >= 0.0 { -0.71 } else { angle };
            let angle = if 0.71 - angle >= 0.0 { angle } else { 0.71 };
            rotation = axis_rotation(axis, angle * target.normal_blend);
        }
        if !target.normal_set {
            let next = target.normal_blend - 0.2;
            target.normal_blend = if -next >= 0.0 { 0.0 } else { next };
        }
    }
    if status.mode == Mode::Local {
        let weight = status.target_blend;
        let desired: [f32; 4] = core::array::from_fn(|lane| {
            (target.animation_position[lane] - animated[3][lane]).mul_add(
                weight,
                status.external_target_local_delta[lane] * (1.0 - weight),
            )
        });
        let change = limit_length(
            core::array::from_fn(|lane| desired[lane] - status.external_target_local_delta[lane]),
            0.025,
        );
        for lane in 0..4 {
            status.external_target_local_delta[lane] += change[lane];
            // The source publishes desired before its history limit. Do not
            // replace this with the separately limited retained delta.
            target.animation_position[lane] = animated[3][lane] + desired[lane];
        }
        target.world_position = transform_point(animation_to_world, target.animation_position);
    } else {
        target.animation_position = transform_point(world_to_animation, target.world_position);
    }
    let offset = limit_length(
        core::array::from_fn(|lane| target.world_position[lane] - original[3][lane]),
        0.7,
    );
    let mut from_origin = IDENTITY;
    from_origin[3] = original[3].map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
    let rotated = compose_affine(&rotation, &from_origin);
    let mut to_origin = IDENTITY;
    to_origin[3] = core::array::from_fn(|lane| original[3][lane] + offset[lane]);
    let adjustment = compose_affine(&to_origin, &rotated);
    frames.external_world = compose_affine(&adjustment, &original);
    if let Some(parent) = binding.parent_part {
        let relative = compose_affine(&inverse_rigid(animated), &animation[parent]);
        frames.external_parent_world = compose_affine(&frames.external_world, &relative);
    }
}
