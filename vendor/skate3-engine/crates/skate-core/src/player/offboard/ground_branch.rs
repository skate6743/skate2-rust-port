//! Original82D7D868 branch selector and82D7E1A8 alternate integration.
use super::{
    contact_correction::{inverse_length, magnitude},
    position_output::FrameOutput,
};
use crate::physics::native_arithmetic::dot3;
type Vector = [f32; 4];
const DT: f32 = f32::from_bits(0x3c88_8889);

pub struct BranchInput {
    pub movement_velocity_480: Vector,
    pub output_velocity_512: Vector,
    pub right_0: Vector,
    pub position_48: Vector,
    pub contact_point_96: Vector,
    pub contact_flags_176: u32,
    pub suppressed_317: bool,
}
/// Recomputed Biped709. This selects a ground-controller branch, not a physical state.
pub fn select_alternate(input: BranchInput) -> bool {
    let velocity = if input.movement_velocity_480[1] > input.output_velocity_512[1] {
        input.output_velocity_512
    } else {
        input.movement_velocity_480
    };
    if input.suppressed_317 || input.contact_flags_176 & 2 == 0 {
        return false;
    }
    let delta: Vector = std::array::from_fn(|i| input.contact_point_96[i] - input.position_48[i]);
    if dot3(delta, delta) <= f32::from_bits(0x3b23_d70b) {
        return false;
    }
    // Native permuted multiply/subtract computes delta cross right.
    let right = input.right_0;
    let normal = [
        (-delta[2]).mul_add(right[1], delta[1] * right[2]),
        (-delta[0]).mul_add(right[2], delta[2] * right[0]),
        (-delta[1]).mul_add(right[0], delta[0] * right[1]),
        (-delta[3]).mul_add(right[3], delta[3] * right[3]),
    ];
    let square = dot3(normal, normal);
    let inverse = inverse_length(square);
    let unit = if magnitude(square) > f32::from_bits(0x3586_37bd) {
        normal.map(|v| v * inverse)
    } else {
        [0.0; 4]
    };
    dot3(velocity, unit) > 4.0
}

///82D7E1A8 writes480/704/48 and then calls82D80360. The caller skips
///82D80548 and cadence on this branch, but still publishes82D80F48 afterward.
pub fn integrate_alternate(
    velocity_480: &mut Vector,
    speed_704: &mut f32,
    position_48: &mut Vector,
    output: &mut FrameOutput,
    forward_32: Vector,
    frame_up_448: Vector,
    projection_axis_416: Vector,
) {
    // Original830BD330 initializer82F826B0 stores [0,-9.8,0,0], using
    //82165A10 zero and822F8B40 C11CCCCD. It is not the splatted830BD400.
    let gravity: Vector = [0.0, f32::from_bits(0xc11c_cccd), 0.0, 0.0];
    *velocity_480 = std::array::from_fn(|i| gravity[i].mul_add(DT, velocity_480[i]));
    *speed_704 = magnitude(dot3(*velocity_480, *velocity_480));
    *position_48 = std::array::from_fn(|i| velocity_480[i].mul_add(DT, position_48[i]));
    output.update(forward_32, frame_up_448, projection_axis_416, *position_48);
}

#[cfg(test)]
mod tests;
