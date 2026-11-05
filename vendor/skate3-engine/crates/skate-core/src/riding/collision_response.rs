//! Complete collision response calculation, TU3 82D944E8 and its angle helpers.
use crate::{
    math::Vector3,
    physics::{
        board_ground::angle_between,
        board_motion_output::{dot, inverse_length_squared, length},
        native_arithmetic::reciprocal_estimate,
    },
    point_graph::PointGraph,
};
#[derive(Clone, Debug)]
pub struct CollisionResponseSettings {
    pub maximum_velocity_delta: f32,
    pub force_y_offset: f32,
    pub force_scalar: f32,
    pub target_displacement_velocity: f32,
    pub torque_vs_angle: PointGraph<8>,
}
#[derive(Clone, Copy, Debug)]
pub struct CollisionResponsePhysical {
    pub flags_2472: u32,
    /// Processed736,416,448,544 and Ground1216 respectively.
    pub collision_displacement: [f32; 4],
    pub velocity: [f32; 4],
    pub forward: [f32; 4],
    pub up: [f32; 4],
    pub ground_normal: [f32; 4],
    /// Processed2604 and2660.
    pub time_step: f32,
    pub mass: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionResponse {
    /// Native return value. False after the delta threshold still writes all
    /// three outputs, including the target velocity and angular correction.
    pub applied: bool,
    pub force: [f32; 4],
    pub point: [f32; 4],
    pub angular_displacement: [f32; 4],
    pub target_velocity: [f32; 4],
}
/// None represents the two early returns which write no outputs. The native
/// late false return must remain distinguishable from them.
pub fn collision_response(
    settings: &CollisionResponseSettings,
    input: CollisionResponsePhysical,
) -> Option<CollisionResponse> {
    if input.flags_2472 & 0x20000 == 0 {
        return None;
    }
    let projection = dot(xyz(input.collision_displacement), xyz(input.ground_normal));
    let displaced: [f32; 4] = std::array::from_fn(|i| {
        input.collision_displacement[i] - input.ground_normal[i] * projection
    });
    let distance = length(xyz(displaced));
    if distance < f32::from_bits(0x3780_0000) {
        return None;
    }
    let inverse_distance = 1.0 / distance; //scalar fdivs, not vector estimate.
    let direction: [f32; 4] = displaced.map(|v| v * inverse_distance);
    let speed_toward = dot(xyz(input.velocity), xyz(direction));
    let tangent: [f32; 4] =
        std::array::from_fn(|i| input.velocity[i] - direction[i] * speed_toward);
    let target_velocity = std::array::from_fn(|i| {
        direction[i].mul_add(settings.target_displacement_velocity, tangent[i])
    });
    let mut delta =
        std::array::from_fn(|i| (target_velocity[i] - input.velocity[i]) * settings.force_scalar);
    if speed_toward > settings.target_displacement_velocity {
        delta = [0.; 4];
    }
    let magnitude = length(xyz(delta));
    let inverse_dt = reciprocal_refined(input.time_step);
    let mut force = delta.map(|v| (v * input.mass) * inverse_dt);
    let applied = magnitude >= 0.01;
    if applied {
        let clamp = if magnitude <= settings.maximum_velocity_delta {
            1.0
        } else {
            settings.maximum_velocity_delta / magnitude
        };
        force = force.map(|v| v * clamp);
    } else {
        force = [0.; 4];
    }
    let angle = wrap_angle(signed_angle(
        xyz(input.forward),
        xyz(direction),
        xyz(input.up),
    ));
    let magnitude = settings
        .torque_vs_angle
        .evaluate(angle.abs() * f32::from_bits(0x3ea2_f983));
    let signed = if angle >= 0.0 { magnitude } else { -magnitude };
    Some(CollisionResponse {
        applied,
        force,
        point: [0.0, settings.force_y_offset, 0.0, 0.0],
        angular_displacement: input.up.map(|v| v * signed),
        target_velocity,
    })
}
///8296EC98, one-refinement normalization and signed cross about supplied up.
pub fn signed_angle(a: Vector3, b: Vector3, up: Vector3) -> f32 {
    let a_squared = dot(a, a);
    let b_squared = dot(b, b);
    if !(a_squared > 0.0001 && b_squared > 0.0001) {
        return 0.0;
    }
    let angle = angle_between(a, b);
    let a = scale(a, inverse_length_squared(a_squared, 1));
    let b = scale(b, inverse_length_squared(b_squared, 1));
    let cross = Vector3::new(
        (-a.z).mul_add(b.y, a.y * b.z),
        (-a.x).mul_add(b.z, a.z * b.x),
        (-a.y).mul_add(b.x, a.x * b.y),
    );
    if dot(cross, up) < 0.0 {
        f32::from_bits(0x40c9_0fdb) - angle
    } else {
        angle
    }
}
///8258DB98 scalar truncation, fused wrap subtraction, then single range repair.
fn wrap_angle(mut angle: f32) -> f32 {
    let pi = f32::from_bits(0x4049_0fdb);
    let tau = f32::from_bits(0x40c9_0fdb);
    if angle < -pi || !(angle < pi) {
        let turns = (angle * f32::from_bits(0x3e22_f983)) as i32;
        angle = (-(turns as f32)).mul_add(tau, angle);
        if !(angle < pi) {
            angle -= tau;
        } else if angle < -pi {
            angle += tau;
        }
    }
    angle
}
fn reciprocal_refined(value: f32) -> f32 {
    let estimate = reciprocal_estimate(value);
    let first = estimate.mul_add((-estimate).mul_add(value, 1.0), estimate);
    first.mul_add((-first).mul_add(value, 1.0), first)
}
fn scale(v: Vector3, k: f32) -> Vector3 {
    Vector3::new(v.x * k, v.y * k, v.z * k)
}
fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_false_still_publishes_collision_target_and_angle() {
        let settings = CollisionResponseSettings {
            maximum_velocity_delta: 8.,
            force_y_offset: -0.05,
            force_scalar: 1.,
            target_displacement_velocity: 1.,
            torque_vs_angle: PointGraph {
                x: [0., 1., 2., 3., 4., 5., 6., 7.],
                y: [0.03; 8],
            },
        };
        let mut input = CollisionResponsePhysical {
            flags_2472: 0x20000,
            collision_displacement: [1., 0., 0., 0.],
            velocity: [2., 0., 4., 0.],
            forward: [0., 0., 1., 0.],
            up: [0., 1., 0., 0.],
            ground_normal: [0., 1., 0., 0.],
            time_step: 1. / 60.,
            mass: 8.,
        };
        let late = collision_response(&settings, input).unwrap();
        assert!(!late.applied);
        assert_eq!(late.force, [0.; 4]);
        assert_eq!(late.target_velocity, [1., 0., 4., 0.]);
        assert!(late.angular_displacement[1] > 0.);
        input.velocity = [-20., 0., 4., 0.];
        let clamped = collision_response(&settings, input).unwrap();
        assert!(clamped.applied);
        assert!((clamped.force[0] - 3840.).abs() < 0.001);
        assert_eq!(clamped.point, [0., -0.05, 0., 0.]);
        input.collision_displacement = input.ground_normal;
        assert!(collision_response(&settings, input).is_none());
    }
}
