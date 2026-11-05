//! Dynamic wheel-response normal, original TU3 82C02388, with acceleration
//! history from82C082AC..8348. This is Ground64, distinct from wheel normals.
use super::{
    board_ground::BoardGroundState,
    board_motion_output::{add, dot, inverse_length_squared, scale, subtract},
};
use crate::{math::Vector3, point_graph::PointGraph};
const UP: Vector3 = Vector3::new(0., 1., 0.);
const ZERO: Vector3 = Vector3::ZERO;
#[derive(Clone, Debug)]
pub struct DynamicNormalSettings {
    pub speed_damping: f32,
    pub up_vector_damping: f32,
    pub maximum_delta: f32,
    pub speed_scale: f32,
    pub maximum_delta_vs_speed: PointGraph<8>,
}
#[derive(Clone, Debug)]
pub struct BoardDynamicNormal {
    /// Skateboard112, published to Ground64.
    pub normal: Vector3,
    /// Skateboard128. The constructor really initializes this to UP.
    pub delta: Vector3,
    /// Skateboard144 and160, retained independently by the source.
    pub acceleration: Vector3,
    pub last_contact_normal: Vector3,
}
impl BoardDynamicNormal {
    ///82C00ED0 initializes112/128 to UP and144/160 to zero. The full board
    ///reset82C0D680 ->82C00D68 clears all seven previous velocities.
    pub fn new() -> Self {
        Self {
            normal: UP,
            delta: UP,
            acceleration: ZERO,
            last_contact_normal: ZERO,
        }
    }
    pub fn update(
        &mut self,
        contacts: &BoardGroundState,
        gravity: Vector3,
        previous_ground_speed: f32,
        settings: &DynamicNormalSettings,
    ) {
        //82C02344 runs Body::UpdatePostPhysics before82C0234C calls this
        //filter.82C02444/60/7C read the existing Body432/448/464 array.
        //Paired S2:82B37E5C/64 calls Body::UpdatePostPhysics then
        //UpdateUpVector82B26158, which reads CollisionInfo acceleration.
        //No live body, timestep, duplicate history or mirror belongs here.
        if contacts.wheel_contact_count == 0 {
            self.acceleration = ZERO;
            return;
        }
        let mut sum = ZERO;
        //The original body tests780,781,782, then normalizes. There is no
        //fourth-wheel accumulation in this function; retain that distinction.
        for i in 0..3 {
            if contacts.parts[i].in_contact {
                sum = subtract(add(sum, contacts.accelerations[i]), gravity);
            }
        }
        let target = normalize(sum);
        self.delta = scale(self.delta, settings.speed_damping);
        let error = scale(subtract(target, self.normal), settings.up_vector_damping);
        let (direction, magnitude) = normalize_length(error);
        let maximum = settings
            .maximum_delta_vs_speed
            .evaluate(previous_ground_speed.abs() / settings.speed_scale)
            * settings.maximum_delta;
        let nonnegative = if -magnitude >= -0.0 { 0.0 } else { magnitude };
        let amount = if maximum - nonnegative >= -0.0 {
            nonnegative
        } else {
            maximum
        };
        self.delta = Vector3::new(
            direction.x.mul_add(amount, self.delta.x),
            direction.y.mul_add(amount, self.delta.y),
            direction.z.mul_add(amount, self.delta.z),
        );
        self.normal = normalize(add(self.normal, self.delta));
        self.last_contact_normal = contacts.overall_normal;
    }
}

#[cfg(test)]
#[path = "tests/board_dynamic_normal_history.rs"]
mod tests;
fn normalize(v: Vector3) -> Vector3 {
    normalize_length(v).0
}
fn normalize_length(v: Vector3) -> (Vector3, f32) {
    let squared = dot(v, v);
    let inverse = inverse_length_squared(squared, 2);
    let length = if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    };
    //830BD350's verified initializer is float1e-6. Source compares length.
    let normal = if length > f32::from_bits(0x3586_37bd) {
        scale(v, inverse)
    } else {
        ZERO
    };
    (normal, length)
}
