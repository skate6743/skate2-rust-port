//! Biped special-mode latch82D7C8E8 and sliding velocity82D7CC18.
use super::contact_correction::{clamp_length, inverse_length, magnitude};
use crate::{physics::native_arithmetic::dot3, point_graph::PointGraph};
type Vector = [f32; 4];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpecialMode {
    pub enabled_714: bool,
    pub elapsed_788: f32,
}
impl SpecialMode {
    pub fn update(&mut self, move_input_308: f32, contact_flags_176: u32, forward_y: f32) {
        if self.enabled_714 {
            self.elapsed_788 += f32::from_bits(0x3c88_8889);
            if !(move_input_308 >= 0.5)
                || (!(self.elapsed_788 <= 0.5) && contact_flags_176 & 0x200 == 0)
            {
                self.enabled_714 = false;
            }
        } else if !(self.elapsed_788 <= 0.0) {
            if f32::from_bits(0x3e19_999a) > forward_y {
                self.elapsed_788 = 0.0;
            }
        } else if contact_flags_176 & 0x200 != 0 {
            self.elapsed_788 = 0.0;
            self.enabled_714 = true;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sliding {
    pub velocity_528: Vector,
    pub active_710: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub special_mode_714: bool,
    pub surface_normal_560: Vector,
    pub movement_velocity_480: Vector,
    pub contact_direction_400: Option<Vector>,
}
impl Sliding {
    pub fn update(
        &mut self,
        input: Input,
        slide_vs_slope: &PointGraph<8>,
        slide_vs_speed: &PointGraph<8>,
    ) {
        //82D7CC44 returns without changing either retained field in this mode.
        if input.special_mode_714 {
            return;
        }
        let down = [0.0, -1.0, 0.0, 0.0];
        let normal = input.surface_normal_560;
        let projection = dot3(normal, down);
        let downhill: Vector = std::array::from_fn(|i| down[i] - normal[i] * projection);
        let tilt = -downhill[1];
        let lower = select(-tilt, 0.0, tilt);
        let slope = select(1.0 - lower, lower, 1.0);
        let uphill = safe_unit(downhill).map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
        let speed = dot3(input.movement_velocity_480, uphill);
        let factor = -(slide_vs_slope.evaluate(slope) * slide_vs_speed.evaluate(speed));
        //82F82588 initializes830BD400 by splatting822F8B40 (-9.8).
        let acceleration = clamp_length(
            downhill.map(|v| v * (f32::from_bits(0xc11c_cccd) * factor)),
            5.0,
        );
        self.velocity_528 = std::array::from_fn(|i| {
            acceleration[i].mul_add(f32::from_bits(0x3d4c_cccd), self.velocity_528[i] * 0.95)
        });
        let threshold = f32::from_bits(0x3c23_d70b);
        if threshold > dot3(acceleration, acceleration) {
            self.velocity_528 = self.velocity_528.map(|v| v * 0.9);
        }
        if let Some(direction) = input.contact_direction_400 {
            self.velocity_528 = reject_positive(self.velocity_528, direction.map(|v| -v));
        }
        self.active_710 = dot3(self.velocity_528, self.velocity_528) > threshold;
    }
}
fn select(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
fn safe_unit(vector: Vector) -> Vector {
    let square = dot3(vector, vector);
    let inverse = inverse_length(square);
    if magnitude(square) > f32::from_bits(0x3586_37bd) {
        vector.map(|v| v * inverse)
    } else {
        [0.0; 4]
    }
}
///82C1E170: project against safe unit, subtract the original supplied direction.
fn reject_positive(vector: Vector, direction: Vector) -> Vector {
    let projection = dot3(vector, safe_unit(direction));
    if projection > 0.0 {
        std::array::from_fn(|i| vector[i] - direction[i] * projection)
    } else {
        vector
    }
}

#[cfg(test)]
#[path = "slide/tests.rs"]
mod tests;
