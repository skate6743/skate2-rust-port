//! Original Reckoning::UpdateBiped82D8E3E0, using the existing shared owners.
//! 82D8E458 explicitly clears velocity; riding acceleration/damping is absent.
//! This adapter follows the shared XYZ orientation representation; all input
//! frame/filter math uses four lanes and returns complete computed vectors.
use crate::{
    air::{
        body_spin::{self, BodySpinState},
        reckoning::AirState,
    },
    math::Vector3,
    point_graph::PointGraph,
    riding::{ground_orientation::GroundOrientation, reckoning_frames::ReckoningFrames},
};
type Vector = [f32; 4];
pub struct Settings<'a> {
    pub ground_normal_smoothing: [f32; 4],
    pub tilt_vs_rotation: &'a PointGraph<8>,
    pub tilt_vs_slope: &'a PointGraph<8>,
}
pub struct Input {
    /// Actual current reckoning1152, supplied as all four original lanes.
    pub previous_up: Vector,
    pub requested_up: Vector,
    pub requested_forward: Vector,
    pub blend: f32,
    pub reverse_stance: bool,
    pub enable_body_spin_input: bool,
    pub physical_body_spin_2812: f32,
}
/// Four-lane publications let the canonical owner retain every computed lane.
/// No separate persistent orientation state is introduced here.
pub struct Output {
    pub dynamic_up_1136: Vector,
    pub up_1152: Vector,
    pub target_1168: Vector,
    pub velocity_1184: Vector,
    pub ground_normal_1216: Vector,
}
pub fn update(
    orientation: &mut GroundOrientation,
    frames: &mut ReckoningFrames,
    body_spin: &mut BodySpinState,
    air: &mut AirState,
    settings: Settings<'_>,
    input: Input,
) -> Output {
    orientation.dynamic_up = xyz(input.previous_up);
    frames.target_lean_angle = 0.0;
    air.secondary_lean_angle = 0.0;
    orientation.up_velocity = Vector3::ZERO;
    let ground = orientation
        .ground_filter
        .update(settings.ground_normal_smoothing, input.requested_up);
    orientation.ground_normal = xyz(ground);
    frames.heading = input.requested_forward;
    let retained = 1.0 - input.blend;
    let candidate: Vector = std::array::from_fn(|i| {
        input.previous_up[i].mul_add(input.blend, input.requested_up[i] * retained)
    });
    let square = crate::physics::native_arithmetic::dot3(candidate, candidate);
    let inverse = super::contact_correction::inverse_length(square);
    let length = if square == 0.0 { 0.0 } else { square * inverse };
    let up = if length > f32::from_bits(0x3586_37bd) {
        candidate.map(|v| v * inverse)
    } else {
        input.previous_up
    };
    orientation.up = xyz(up);
    orientation.target = xyz(up);
    orientation.slow_filter.filter(up);
    orientation.fast_filter.filter(up);
    orientation.slow_filter.publish_current(up);
    orientation.fast_filter.publish_current(up);
    frames.calculate_transform(up, ground);
    frames.calculate_tilt(
        input.reverse_stance,
        settings.tilt_vs_rotation,
        settings.tilt_vs_slope,
    );
    body_spin::update_ground(
        body_spin,
        if input.enable_body_spin_input {
            input.physical_body_spin_2812
        } else {
            0.0
        },
    );
    Output {
        dynamic_up_1136: input.previous_up,
        up_1152: up,
        target_1168: up,
        velocity_1184: [0.0; 4],
        ground_normal_1216: ground,
    }
}
fn xyz(v: Vector) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}

#[cfg(test)]
mod tests;
