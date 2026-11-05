//! Original body adjustments used by WipeoutGround300.
use super::math::{self, V};
use crate::{math::Vector3, physics::skeleton_body::SkeletonBody};
pub fn set_velocity(body: &mut SkeletonBody, velocity: V) {
    for part in &mut body.bodies_mut()[1..24] {
        part.rates.linear_velocity = Vector3::new(velocity[0], velocity[1], velocity[2]);
    }
}
pub fn add_velocity(body: &mut SkeletonBody, velocity: V) {
    for part in &mut body.bodies_mut()[1..24] {
        part.rates.linear_velocity.x += velocity[0];
        part.rates.linear_velocity.y += velocity[1];
        part.rates.linear_velocity.z += velocity[2];
    }
}
///82BE74C8. Upper parts retain a tenth of the normal component; lower parts zero it.
pub fn remove_normal_velocity(body: &mut SkeletonBody, normal: V) {
    for i in 1..24 {
        let v = body.bodies()[i].rates.linear_velocity;
        let v = [v.x, v.y, v.z, 0.0];
        let projected = math::scale(normal, math::dot(normal, v));
        let next = math::madd(
            projected,
            if i < 15 { 0.1 } else { 0.0 },
            math::sub(v, projected),
        );
        let v = &mut body.bodies_mut()[i].rates.linear_velocity;
        *v = Vector3::new(next[0], next[1], next[2]);
    }
}
///82BE75B0 then82BD41B0: average with the requested velocity and limit distance2.
pub fn blend_velocity(body: &mut SkeletonBody, target: V) {
    for part in &mut body.bodies_mut()[1..24] {
        let v = part.rates.linear_velocity;
        let average = math::madd(target, 0.5, [v.x * 0.5, v.y * 0.5, v.z * 0.5, 0.0]);
        let delta = math::sub(average, target);
        let length = math::length(delta);
        let next = if length <= 2.0 {
            average
        } else {
            math::madd(delta, 2.0 / length, target)
        };
        part.rates.linear_velocity = Vector3::new(next[0], next[1], next[2]);
    }
}
///82BE76B0: length and reciprocal each use the original two refinements.
pub fn limit_velocity(body: &mut SkeletonBody, maximum: f32) {
    for part in &mut body.bodies_mut()[1..24] {
        let v = part.rates.linear_velocity;
        let v = [v.x, v.y, v.z, 0.0];
        if math::dot(v, v) > maximum * maximum {
            let next = math::scale(v, math::reciprocal(math::length(v)) * maximum);
            part.rates.linear_velocity = Vector3::new(next[0], next[1], next[2]);
        }
    }
}
///82D9CD60 and82D9CE58 publish per-second drag into the same live inertia.
pub fn set_drag(body: &mut SkeletonBody, linear: f32, angular: f32) {
    for part in body.bodies_mut() {
        part.inertia.linear_drag = linear * f32::from_bits(0x426F_FFFF);
        part.inertia.angular_drag = angular * f32::from_bits(0x426F_FFFF);
    }
}
///82BE32D0.
pub fn set_wipeout_drag(body: &mut SkeletonBody, drag: f32) {
    set_drag(
        body,
        math::clamp(drag, 0.0, 1.0),
        math::clamp((drag + 1.0) * 0.5, 0.0, 1.0),
    );
}
///82BE3998. Source uses cached physical-record positions before the live writes.
pub fn special_surface(body: &mut SkeletonBody, height: f32) {
    // 82BE3A70..3B38 tests cached part depth only; no world/floor-depth gate.
    const WEIGHTS: [f32; 23] = [
        1.7, 1.5, 1.2, 1.0, 1.0, 1.0, 1.2, 1.0, 1.0, 1.0, 1.2, 1.0, 1.0, 1.0, 0.5, 0.5, 0.5, 0.6,
        0.5, 0.5, 0.5, 0.6, 1.0,
    ];
    for i in 1..24 {
        let depth = (height + 0.1) - body.record.pose[i][3][1];
        let drag = if depth > 0.0 {
            body.apply_part_displacement(
                i,
                [0.0, depth.mul_add(0.6, 0.1) * WEIGHTS[i - 1], 0.0, 0.0],
            );
            0.1
        } else {
            0.005
        };
        body.bodies_mut()[i].inertia.angular_drag = drag * f32::from_bits(0x426F_FFFF);
        body.bodies_mut()[i].inertia.linear_drag = drag * f32::from_bits(0x426F_FFFF);
    }
}
