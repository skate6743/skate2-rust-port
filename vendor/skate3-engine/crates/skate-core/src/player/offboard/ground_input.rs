//! Original TU3 ground control input82D310F8; S3 SHA256
//!431b8eba23565affdc10d137df19b06fe286244cefb3e1a13f32693e9600395a.
//! Pure calculation: the existing BipedGround STATE owns the inputs and outputs.
//! Stock curves must be present and validated at the data boundary.

mod math;
#[cfg(test)]
mod tests;

use crate::point_graph::PointGraph;
use math::{cross, dot, inverse_frame, length, normalize, signed_angle};

#[derive(Clone, Copy, Debug)]
pub struct GroundInput {
    pub processed_flags_2472: u32,
    pub processed_direct_2684: f32,
    pub processed_direct_2680: f32,
    pub processed_stick_2692: f32,
    pub processed_stick_2688: f32,
    pub processed_scale_2912: f32,
    pub processed_scale_2908: f32,
    /// Forward row of the actual Biped frame, state112.
    pub frame_forward_112: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Ground STATE offsets:708/712 become job-input308/312 (job base400).
/// These are not Biped controller708, which is a separate correction boolean.
pub struct GroundInputOutput {
    pub state_708: f32,
    pub state_712: f32,
    /// Original untransformed input, NOT its coordinates in the flattened frame.
    pub state_656: [f32; 4],
}

///82D310F8. Curve A key DF759B46440F16E9; curve B key2DD95B399BAE313E
/// (TurnVsStickAngle). Leading PointNegGraphData8 bounds are not inputs here.
pub fn calculate(
    input: &GroundInput,
    curve_a: &PointGraph<8>,
    turn_vs_stick_angle: &PointGraph<8>,
) -> GroundInputOutput {
    if input.processed_flags_2472 & 0x1000_0000 != 0 {
        return GroundInputOutput {
            state_708: input.processed_direct_2684,
            state_712: input.processed_direct_2680,
            state_656: [0.0; 4],
        };
    }
    let raw = [input.processed_stick_2692, 0.0, input.processed_stick_2688];
    let up = [0.0, 1.0, 0.0];
    let mut right = [1.0, 0.0, 0.0];
    let mut forward = [0.0, 0.0, 1.0];
    // Signed comparison: negative vertical does not receive an identity fallback.
    if dot(input.frame_forward_112, up) < f32::from_bits(0x3f7f_be77) {
        right = normalize(cross(up, input.frame_forward_112));
        forward = normalize(cross(right, up));
    }
    let local = inverse_frame(right, up, forward, raw);
    let angle = signed_angle(local[0], -local[2]) * f32::from_bits(0x3ea2_f983);
    // Scalar fsel accepts either signed zero; unordered input selects -1.
    let sign = if angle >= 0.0 { 1.0 } else { -1.0 };
    let absolute_angle = angle * sign;
    let magnitude = length(local);
    GroundInputOutput {
        state_708: (curve_a.evaluate(absolute_angle) * magnitude) * input.processed_scale_2912,
        state_712: ((turn_vs_stick_angle.evaluate(absolute_angle) * magnitude) * sign)
            * input.processed_scale_2908,
        state_656: [raw[0], raw[1], raw[2], 0.0],
    }
}
