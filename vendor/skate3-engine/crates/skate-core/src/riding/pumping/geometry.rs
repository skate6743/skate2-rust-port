//! TU3 Pumping geometry82D8F280 and82D8F4A0..6AC. The reciprocal and
//! reciprocal-square-root estimate primitives retain their documented host
//! approximation boundary; source refinements, fused crosses and gates remain.
use super::controller::{PumpingGeometry, PumpingSample};
use crate::physics::{
    board_motion_output::inverse_length_squared,
    native_arithmetic::{dot3, reciprocal_estimate},
};
pub struct NativePumpingGeometry;
impl PumpingGeometry for NativePumpingGeometry {
    type Error = core::convert::Infallible;
    fn height(&mut self, normal: [f32; 4], com: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(dot3(normal, com))
    }
    fn speed(
        &mut self,
        previous: [f32; 4],
        current: [f32; 4],
        dt: f32,
    ) -> Result<f32, Self::Error> {
        let inverse = reciprocal(dt);
        let velocity = std::array::from_fn(|i| (current[i] - previous[i]) * inverse);
        let squared = dot3(velocity, velocity);
        let length = squared * inverse_length_squared(squared, 2);
        Ok(if squared == 0.0 { 0.0 } else { length })
    }
    fn angular_speed(
        &mut self,
        previous_position: [f32; 4],
        previous_normal: [f32; 4],
        sample: &PumpingSample,
        dt: f32,
    ) -> Result<f32, Self::Error> {
        let displacement = std::array::from_fn(|i| sample.position[i] - previous_position[i]);
        let axis = cross(displacement, sample.normal);
        let squared = dot3(axis, axis);
        let inverse = inverse_length_squared(squared, 2);
        let length = if squared == 0.0 {
            0.0
        } else {
            squared * inverse
        };
        let axis = if length > f32::from_bits(0x3586_37bd) {
            axis.map(|v| v * inverse)
        } else {
            [0.0; 4]
        };
        let inverse_dt = reciprocal(dt);
        let angular = cross(previous_normal, sample.normal).map(|v| v * inverse_dt);
        Ok(dot3(angular, axis))
    }
    fn inclination_radians(&mut self, y: f32) -> Result<f32, Self::Error> {
        Ok(crate::trigonometry::acos(y))
    }
}
fn reciprocal(value: f32) -> f32 {
    let mut inverse = reciprocal_estimate(value);
    for _ in 0..2 {
        let error = (-inverse).mul_add(value, 1.0);
        inverse = inverse.mul_add(error, inverse);
    }
    inverse
}
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let cyclic = [1, 2, 0, 3];
    let product: [f32; 4] =
        std::array::from_fn(|i| (-a[cyclic[i]]).mul_add(b[i], a[i] * b[cyclic[i]]));
    cyclic.map(|i| product[i])
}
