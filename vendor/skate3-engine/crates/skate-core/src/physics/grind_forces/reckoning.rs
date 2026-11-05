//! UpdateGrindStates82D8E3E0, compared with S2 named82DDD2A8.
//! Maintains CURRENT's shared filters/frames and BodySpin history; no second
//! reckoning owner and no ground/air-state substitute.
use super::{V, length};
use crate::{
    air::{
        body_spin::{self, BodySpinState},
        reckoning::AirState,
    },
    math::Vector3,
    physics::{board_motion_output::inverse_length_squared, native_arithmetic::dot3},
    point_graph::PointGraph,
    riding::{ground_orientation::GroundOrientation, reckoning_frames::ReckoningFrames},
};

pub struct Settings {
    pub ground_normal_smoothing: V,
    /// Stock schema: layout800 TiltVsRotGround,672 TiltVsSlopeGround.
    pub tilt_vs_rotation: PointGraph<8>,
    pub tilt_vs_slope: PointGraph<8>,
}

pub fn update(
    orientation: &mut GroundOrientation,
    frames: &mut ReckoningFrames,
    body_spin: &mut BodySpinState,
    air: &mut AirState,
    settings: &Settings,
    normal: V,
    heading: V,
    smoothing: f32,
    reverse_stance: bool,
    physical_body_spin: f32,
) {
    let old_up = lanes(orientation.up);
    orientation.dynamic_up = orientation.up;
    orientation.up_velocity = Vector3::ZERO;
    frames.target_lean_angle = 0.;
    air.secondary_lean_angle = 0.;
    orientation.ground_blend = 0.;
    let ground = orientation
        .ground_filter
        .update(settings.ground_normal_smoothing, normal);
    orientation.ground_normal = xyz(ground);
    frames.heading = heading;
    let candidate: V =
        core::array::from_fn(|i| old_up[i].mul_add(smoothing, normal[i] * (1. - smoothing)));
    // Native NormalizeSafe threshold is the original830BD350 splat1e-6;
    // zero/short candidates retain the preceding up, not world-up.
    let up = if length(candidate) > f32::from_bits(0x3586_37bd) {
        let inverse = inverse_length_squared(dot3(candidate, candidate), 2);
        candidate.map(|v| v * inverse)
    } else {
        old_up
    };
    orientation.up = xyz(up);
    orientation.target = orientation.up;
    orientation.slow_filter.filter(up);
    orientation.fast_filter.filter(up);
    orientation.slow_filter.publish_current(up);
    orientation.fast_filter.publish_current(up);
    frames.calculate_transform(up, ground);
    frames.calculate_tilt(
        reverse_stance,
        &settings.tilt_vs_rotation,
        &settings.tilt_vs_slope,
    );
    // Caller82D3F4E8 passes updateBodySpin=false, hence0. Other native callers
    // may supply their actual PhysBodySpin; this is still the ground branch.
    body_spin::update_ground(body_spin, physical_body_spin);
    // No ResetSpin: Air's spin_angle/spin_speed survive this function.
}

fn xyz(v: V) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn lanes(v: Vector3) -> V {
    [v.x, v.y, v.z, 0.]
}
