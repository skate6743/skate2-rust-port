//!82D7F458..FDD0 animation-directed movement and ground/obstacle approach.
use super::math::*;
use super::{Frame, GroundMotionInput, GroundMotionState, Vector};
use crate::trigonometry;

///82D80EB0. No state change when the job has no target frame.
fn refresh_target(s: &mut GroundMotionState, i: &GroundMotionInput) {
    if i.target_frame_present_352 {
        s.target_frame_608 = i.target_frame_368;
        s.target_frame_608[3] = madd(
            i.target_frame_368[0],
            f32::from_bits(0x3e4c_cccd),
            i.target_frame_368[3],
        );
    }
}

pub(super) fn calculate(
    s: &mut GroundMotionState,
    i: &GroundMotionInput,
    position: Vector,
    delta: Frame,
) -> (Vector, Vector) {
    let mut step = scale(s.velocity_480, DT);
    let mut height = ZERO;
    if i.animation_directed_304 {
        let animation_speed = length(i.animation_motion_224);
        let stationary = animation_speed < EPSILON;
        let mut difference = sub(s.target_frame_608[3], position);
        let mut distance = length(difference);
        if !(s.target_scale_784 >= 0.0) {
            refresh_target(s, i);
            difference = sub(s.target_frame_608[3], position);
            distance = length(difference);
            s.target_scale_784 = distance
                / if stationary {
                    i.requested_duration_288
                } else {
                    animation_speed
                };
        }
        step = ZERO;
        if !(distance <= EPSILON) {
            let mut step_length = s.target_scale_784 * DT;
            if !stationary {
                step_length = length(i.animation_motion_240) * step_length;
            }
            step = scale(scale(difference, reciprocal(distance)), step_length);
        }
        let sine = clamp_signed(dot(
            cross(s.target_frame_608[0], s.frame_0[2]),
            s.frame_0[1],
        ));
        let angle = trigonometry::asin(sine);
        let denominator = if stationary {
            i.requested_duration_288 * s.target_scale_784
        } else {
            s.target_scale_784 * animation_speed
        };
        s.angular_velocity_688 = 0.0;
        if !(denominator <= EPSILON) {
            s.angular_velocity_688 = ((1.0 - distance / denominator) * angle) * RATE;
        }
        s.target_frame_608 = compose(s.target_frame_608, delta);
        return (step, height);
    }
    s.target_scale_784 = -1.0;
    refresh_target(s, i);
    if i.contact_flags_176 & 2 == 0 && !i.obstacle_enabled_713 {
        if i.contact_flags_176 & 1 != 0 {
            height = height_adjustment(s, i, position);
        }
        return (step, height);
    }
    let mut target = if i.obstacle_enabled_713 {
        i.obstacle_target_672
    } else {
        let side = normalize_safe(cross(UP, s.frame_0[2]), s.frame_0[0]);
        let point = if s.correction_enabled_711 {
            i.correction_target_592
        } else {
            i.contact_target_96
        };
        sub(point, scale(side, dot(sub(point, position), side)))
    };
    let difference = sub(target, position);
    if dot(difference, difference) < f32::from_bits(0x3780_0000)
        || !(dot(difference, s.velocity_480) >= -0.5)
    {
        let predicted = madd(scale(s.velocity_480, DT), 2.0, position);
        target = [predicted[0], target[1], predicted[2], predicted[3]];
    }
    let difference = sub(target, position);
    let budget = length(step);
    let distance = length(difference);
    if !(distance <= budget) {
        step = scale(normalize_safe(difference, ZERO), budget);
    } else {
        let normal = if i.obstacle_enabled_713 {
            i.target_frame_368[1]
        } else {
            i.contact_normal_112
        };
        let angle = f32::from_bits(0x4234_0000) * f32::from_bits(0x3c8e_fa35);
        let limited = limit_angle(normal, i.reference_frame_128[1], angle);
        let normal = normalize_safe(limited, ZERO);
        let tangent = sub(s.velocity_480, scale(normal, dot(s.velocity_480, normal)));
        step = madd(normalize_safe(tangent, ZERO), budget - distance, difference);
    }
    if !(budget >= f32::from_bits(0x3c23_d70a)) {
        height = height_adjustment(s, i, position);
    }
    (step, height)
}
fn height_adjustment(s: &GroundMotionState, i: &GroundMotionInput, position: Vector) -> Vector {
    scale(
        s.frame_0[1],
        dot(sub(i.contact_position_0, position), s.frame_0[1]) * f32::from_bits(0x3d4c_cccd),
    )
}
