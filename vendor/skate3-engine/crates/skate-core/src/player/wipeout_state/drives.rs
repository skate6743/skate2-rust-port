//! Original SkeletonDrives::UpdateRagdoll82BEB0C8 and root softness82BEB790.
use crate::{
    animation::foot_ik::inverse_affine,
    physics::{
        board_motion_output::inverse_length_squared,
        drive_parameters::{RetailDriveParams, RetailDriveType},
        skeleton_animation_record::{AnimationPartTransform, IDENTITY},
        skeleton_body::{SkeletonDrives, bone_drive_frames},
    },
};
const FREQUENCY_SQUARED: f32 = f32::from_bits(0x4560_FFFE);
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    ///Original SkeletonBody748+36*part: start, normal, controlled, end, extra.
    pub bone: [[f32; 5]; 24],
    ///The four full64 root_drive_* stock scalars, in the same first-four order.
    pub root: [f32; 4],
    ///animation/default: layout424 DriveStrengthLocal,420 DriveStrengthRootLocal.
    pub strength: [f32; 2],
    ///physics_animation/default HookSoftDsp888, HookSoftStr884, HookSoftDmp892.
    pub hook_spring: f32,
    pub hook_strength: f32,
    pub hook_damping: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Weights {
    pub start: f32,
    pub end: f32,
    pub controlled: f32,
    ///Processed2484 bits5 and4 select the two independently supplied weights.
    pub upper_extra: f32,
    pub lower_extra: f32,
}
///Returns original retained residual16. It is not clamped or renormalized.
pub fn update(
    drives: &mut SkeletonDrives,
    pose: &[AnimationPartTransform; 24],
    settings: &Settings,
    weights: Weights,
) -> f32 {
    let mut inverses = [IDENTITY; 24];
    for part in 1..24 {
        inverses[part] = inverse_affine(&pose[part]);
    }
    let residual = ((1.0 - weights.start) - weights.end) - weights.controlled;
    for part in 1..23 {
        let bone = drives.bones[part]
            .as_mut()
            .expect("Original22 bone drives exist");
        let coefficients = settings.bone[part];
        let upper = (3..=10).contains(&part);
        let lower = (15..=22).contains(&part);
        let strengths = if upper && weights.upper_extra > 0.0 || lower && weights.lower_extra > 0.0
        {
            bone.dynamics.mode = 5;
            let extra = if upper {
                weights.upper_extra
            } else {
                weights.lower_extra
            };
            let value = extra * coefficients[4];
            [value * settings.strength[0], value * settings.strength[1]]
        } else {
            bone.dynamics.mode = 4;
            let [start, normal, controlled, end, _] = coefficients;
            //82BEB424..444 preserves two distinct FMA accumulation orders.
            let local = controlled.mul_add(
                weights.controlled,
                start.mul_add(weights.start, normal.mul_add(residual, end * weights.end)),
            );
            let root_start = settings.root[0] * start;
            let root_normal = settings.root[1] * normal;
            let root_controlled = settings.root[2] * controlled;
            let root_end = settings.root[3] * end;
            let root = root_controlled.mul_add(
                weights.controlled,
                root_start.mul_add(
                    weights.start,
                    root_end.mul_add(weights.end, root_normal * residual),
                ),
            );
            [local * settings.strength[0], root * settings.strength[1]]
        };
        bone.dynamics.strengths = strengths;
        for channel in 0..2 {
            if !bone.active[channel] {
                continue;
            }
            bone.frames[channel] = bone_drive_frames(
                &pose[part],
                &inverses[part],
                &inverses[bone.parent[channel]],
            );
            bone.dynamics
                .enable(channel, strengths[channel], drives.settings.bone);
        }
    }
    residual
}
///82BEB790 changes only target0's linear drive; damping uses the unscaled spring.
pub fn set_linear_root(drives: &mut SkeletonDrives, s: &Settings, weight: f32) {
    let weight = clamp(weight);
    let spring = s.hook_spring;
    let root = if spring == 0.0 {
        0.0
    } else {
        spring * inverse_length_squared(spring, 2)
    };
    drives.targets.dynamics[0].linear = RetailDriveParams {
        spring_or_max_velocity: spring * weight,
        damping: root * 2.0 - spring * f32::from_bits(0x3C83_126F),
        max_strength: (s.hook_strength * weight) * FREQUENCY_SQUARED,
        drive_type: RetailDriveType::SoftDrive,
    };
}
///82D3BE98..BF24 uses the same target0 angular fields set during Enter.
///Enter uses spring*0.5 and strength*1799.9998; Update uses the supplied weight.
pub fn set_angular_root(drives: &mut SkeletonDrives, s: &Settings, weight: f32) {
    let weight = clamp(weight);
    drives.targets.dynamics[0].angular = RetailDriveParams {
        spring_or_max_velocity: s.hook_spring * weight,
        damping: s.hook_damping,
        max_strength: (s.hook_strength * weight) * FREQUENCY_SQUARED,
        drive_type: RetailDriveType::SoftDrive,
    };
}
fn clamp(v: f32) -> f32 {
    let lower = if -v >= 0.0 { 0.0 } else { v };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}
