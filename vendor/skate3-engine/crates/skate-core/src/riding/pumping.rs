//! Pumping lifecycle, native geometry and CalcPumpForce82D933D0 publication.
//! Ground supplies the current stock mode and queues the resulting board force.
pub mod controller;
pub mod geometry;
pub mod settings;
pub mod state;

use super::vector::normalize;
pub struct PumpForceInput {
    pub flags_2476: u32,
    pub mode_multiplier: f32, // selected-mode layout+8
    pub pumping_scalar: f32,  // Pumping+48 argument f1
    pub input_scalar_2660: f32,
    pub timestep: f32, // Processed+2604
    pub direction_432: [f32; 4],
    pub normal_threshold: [f32; 4],
}
pub fn calculate(i: &PumpForceInput) -> [f32; 8] {
    let pumping = if i.flags_2476 & 2 == 0 {
        i.mode_multiplier * i.pumping_scalar
    } else {
        i.pumping_scalar
    };
    let scalar = (i.input_scalar_2660 * pumping) / i.timestep;
    let direction = normalize(i.direction_432, i.normal_threshold);
    let mut result = [0.0; 8];
    for n in 0..4 {
        result[n] = direction[n] * scalar;
    }
    result
}
