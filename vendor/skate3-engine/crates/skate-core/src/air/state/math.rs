//! Independent arithmetic for original PhysicsAir's jump-velocity helpers.
//! Retains original refinements; PC results are not certified Xenon bit-exact.
use super::PhysicsAirMath;
use crate::physics::{board_motion_output::inverse_length_squared, native_arithmetic};

pub struct AirMath;
impl PhysicsAirMath for AirMath {
    fn minimum_vminfp(&mut self, left: f32, right: f32) -> f32 {
        native_arithmetic::vector_min(left, right)
    }
    fn length_squared_vmsum3fp(&mut self, value: [f32; 4]) -> f32 {
        native_arithmetic::dot3(value, value)
    }
    fn length_vmsum3fp_vrsqrte(&mut self, value: [f32; 4]) -> f32 {
        let square = native_arithmetic::dot3(value, value);
        let length = square * inverse_length_squared(square, 2);
        if square == 0.0 { 0.0 } else { length }
    }
    fn clamp_vector_within_max_length(&mut self, value: [f32; 4], maximum: f32) -> [f32; 4] {
        //82BD3D90..3E74, including the original short-vector bge and fsel.
        let length = self.length_vmsum3fp_vrsqrte(value);
        if !(length >= f32::from_bits(0x3780_0000)) {
            return value;
        }
        let bound = if maximum - length >= -0.0 {
            length
        } else {
            maximum
        };
        let mut inverse = native_arithmetic::reciprocal_estimate(length);
        for _ in 0..2 {
            inverse = inverse.mul_add((-inverse).mul_add(length, 1.0), inverse);
        }
        value.map(|v| (v * bound) * inverse)
    }
}

///8296EBB0 uses one normalization refinement and a clamped dot product.
pub fn angle_between_vectors(left: [f32; 4], right: [f32; 4]) -> f32 {
    let xyz = |v: [f32; 4]| crate::math::Vector3::new(v[0], v[1], v[2]);
    crate::physics::board_ground::angle_between(xyz(left), xyz(right))
}

///Toolkit_ClampJumpVelocity82D93518. The output uses the CURRENT velocity's
///direction; the supplied reference only bounds its magnitude.
pub fn clamp_jump_velocity(reference: [f32; 4], current: [f32; 4]) -> [f32; 4] {
    let reference_speed = AirMath.length_vmsum3fp_vrsqrte(reference);
    let speed = AirMath.length_vmsum3fp_vrsqrte(current);
    let denominator = if speed - 1.0 >= -0.0 { speed } else { 1.0 };
    let scale = if denominator > reference_speed {
        reference_speed / denominator
    } else {
        1.0
    };
    current.map(|v| v * scale)
}
