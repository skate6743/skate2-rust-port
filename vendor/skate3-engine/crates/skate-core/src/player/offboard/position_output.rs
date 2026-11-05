//! Biped frame/position output stages82D80360 and82D80548.
//! The retained state is part of the actual Biped owner; no inferred initialization.
use crate::physics::{
    native_arithmetic::dot3, reciprocal_sqrt::estimate, skeleton_root::orthonormalize,
};
type Vector = [f32; 4];
type Matrix = [Vector; 4];
const DT: f32 = f32::from_bits(0x3c88_8889);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameOutput {
    /// Biped128..176. Common cadence reads this frame's up vector144.
    pub frame: Matrix,
    /// Biped512. Common physical publication copies this to OffBoard64.
    pub velocity: Vector,
}

impl FrameOutput {
    ///82D80360 after movement, before82D80548 and cadence82D80720.
    pub fn update(
        &mut self,
        forward_32: Vector,
        frame_up_448: Vector,
        projection_axis_416: Vector,
        position_48: Vector,
    ) {
        let right = normalize(cross(frame_up_448, forward_32));
        let forward = normalize(cross(right, projection_axis_416));
        self.frame = orthonormalize([right, frame_up_448, forward, self.frame[3]]);
        let mut change: Vector =
            std::array::from_fn(|i| (position_48[i] - self.frame[3][i]) * 60.0 - self.velocity[i]);
        change[1] = select(change[1] - -1.0, change[1], -1.0);
        self.velocity = std::array::from_fn(|i| self.velocity[i] + change[i]);
        self.frame[3] = std::array::from_fn(|i| self.velocity[i].mul_add(DT, self.frame[3][i]));
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PositionInput {
    pub previous_origin_112: Vector,
    pub override_origin_592: Vector,
    pub use_override_711: bool,
    pub projection_axis_416: Vector,
    pub contact_flags_176: u32,
    pub contact_origin_0: Vector,
    pub contact_origin_96: Vector,
    pub frame_position_176: Vector,
    pub animation_position_272: Vector,
}

///82D80548 updates retained Biped368 using actual ground-job contact/pose data.
pub fn update_position(position_368: &mut Vector, input: PositionInput) {
    let up = input.projection_axis_416;
    let mut origin = input.previous_origin_112;
    if input.use_override_711 {
        origin = input.override_origin_592;
    } else if input.contact_flags_176 & 1 != 0 {
        if dot3(subtract(input.contact_origin_0, origin), up) > 0.0 {
            origin = input.contact_origin_0;
        }
    }
    if input.contact_flags_176 & 2 != 0 {
        let contact_height = dot3(subtract(input.contact_origin_96, origin), up);
        let frame_height = dot3(subtract(input.frame_position_176, origin), up);
        let height = select(contact_height - frame_height, contact_height, frame_height);
        let height = select(height - 0.3, 0.3, height);
        //82D80638 ble: unordered takes the update branch.
        if !(height <= 0.0) {
            origin = std::array::from_fn(|i| up[i].mul_add(height, origin[i]));
        }
    }
    let delta = subtract(input.animation_position_272, origin);
    let projection = dot3(up, delta);
    let planar: Vector = std::array::from_fn(|i| delta[i] - up[i] * projection);
    let target: Vector = std::array::from_fn(|i| up[i].mul_add(0.95, origin[i] + planar[i]));
    let delta = subtract(target, *position_368);
    let height = dot3(delta, up);
    let lower = select(-0.1 - height, -0.1, height);
    let bounded = select(0.1 - lower, lower, 0.1);
    let correction = bounded - height;
    *position_368 = std::array::from_fn(|i| position_368[i] + up[i].mul_add(correction, delta[i]));
}

fn subtract(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
fn select(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn normalize(vector: Vector) -> Vector {
    let squared = dot3(vector, vector);
    let mut inverse = estimate(squared);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-squared).mul_add(inverse * inverse, 1.0), inverse);
    }
    vector.map(|value| value * inverse)
}

#[cfg(test)]
#[path = "position_output/tests.rs"]
mod tests;
