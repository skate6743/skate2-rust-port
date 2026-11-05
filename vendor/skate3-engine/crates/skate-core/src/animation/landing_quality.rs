//! Original TU3 PhysicsConditioner::CalcLandingQuality, 0x82DE61D0.
use crate::{
    physics::{board_motion_output::inverse_length_squared, native_arithmetic},
    point_graph::PointGraph,
};

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub twist_spin: PointGraph<4>,
    pub side_speed: PointGraph<4>,
}

#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// PhysicsConditioner +144 and +148, after its filtered-state update.
    pub previous_filtered_state: u32,
    pub filtered_state: u32,
    /// PhysOut_Collision +32. Do not substitute a predicted landing normal.
    pub ground_normal: [f32; 4],
    /// PhysOut_SkateboardMotion +80 and +273.
    pub deck_velocity: [f32; 4],
    pub flipped: bool,
    /// PhysOut_SkateboardReckoning +32: physical deck PART world Z from
    /// 82C02AD8/82C02B1C. This is not the conditioned Reckoning ground frame.
    pub reckoning_forward: [f32; 4],
    /// PhysOut_Air +204.
    pub air_spin: f32,
}

/// Reset values are explicitly written by 0x82DE3F38.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Output {
    pub landing_adjust_80: f32,
    pub sideways_speed_84: f32,
    pub forward_speed_88: f32,
    pub spin_92: f32,
    pub landing_type_96: u32,
    pub landing_data_167: bool,
}

impl Output {
    /// Nonlanding frames preserve every field, including the valid-data latch.
    pub fn update(&mut self, input: Input, settings: &Settings) {
        if input.previous_filtered_state == 1 || input.filtered_state != 1 {
            return;
        }
        let velocity = planar(input.deck_velocity, input.ground_normal);
        let forward = planar(input.reckoning_forward, input.ground_normal);
        let speed = length(velocity);
        let forward_length = length(forward);
        let mut forward_speed = dot(velocity, forward).abs();
        let mut side_speed = dot(cross(forward, input.ground_normal), input.deck_velocity).abs();
        let mut kind = 0;
        let mut spin = 0.0;
        if forward_length * speed < f32::from_bits(0x3727_c5ac) {
            forward_speed = 0.0;
            side_speed = 0.0;
        } else {
            let normalized_velocity = scale(velocity, reciprocal(speed));
            let normalized_forward = scale(forward, reciprocal(forward_length));
            let mut orientation = cross(normalized_forward, normalized_velocity)[1];
            if input.flipped {
                orientation = -orientation;
            }
            let angular_speed = -input.air_spin;
            let spin_input = (angular_speed.abs() - 0.1) * 0.14492753;
            let spin_curve = settings.twist_spin.evaluate(upper_one(spin_input));
            let side_input = (side_speed - 0.1) * 0.1010101;
            let side_curve = settings.side_speed.evaluate(upper_one(side_input));
            if speed >= 2.0 {
                let heading = if input.flipped {
                    scale(normalized_forward, -1.0)
                } else {
                    normalized_forward
                };
                let toward = dot(heading, normalized_velocity) >= 0.0;
                let magnitude;
                if angular_speed.abs() <= 0.1 {
                    kind = 1;
                    magnitude = side_curve;
                } else {
                    let product = angular_speed * orientation;
                    kind = if (toward && product <= 0.0) || (!toward && product > 0.0) {
                        2
                    } else {
                        1
                    };
                    magnitude = if spin_curve - side_curve >= -0.0 {
                        spin_curve
                    } else {
                        side_curve
                    };
                }
                spin = if angular_speed >= -0.0 {
                    magnitude
                } else {
                    -magnitude
                };
                if orientation.abs() < 0.2 && (kind != 2 || !(spin.abs() > 0.3)) {
                    kind = if orientation.abs() >= 0.05 { 3 } else { 0 };
                    spin = 0.0;
                }
            }
        }
        *self = Self {
            landing_adjust_80: spin,
            sideways_speed_84: side_speed,
            forward_speed_88: forward_speed,
            spin_92: spin,
            landing_type_96: kind,
            landing_data_167: true,
        };
    }
}

fn dot(a: [f32; 4], b: [f32; 4]) -> f32 {
    native_arithmetic::dot3(a, b)
}
fn scale(a: [f32; 4], b: f32) -> [f32; 4] {
    a.map(|x| x * b)
}
fn planar(a: [f32; 4], normal: [f32; 4]) -> [f32; 4] {
    let projected = scale(normal, dot(normal, a));
    std::array::from_fn(|i| a[i] - projected[i])
}
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.0,
    ]
}
fn length(a: [f32; 4]) -> f32 {
    let square = dot(a, a);
    let value = square * inverse_length_squared(square, 2);
    if square == 0.0 { 0.0 } else { value }
}
fn reciprocal(value: f32) -> f32 {
    let mut estimate = native_arithmetic::reciprocal_estimate(value);
    for _ in 0..2 {
        estimate = estimate.mul_add((-estimate).mul_add(value, 1.0), estimate);
    }
    estimate
}
fn upper_one(value: f32) -> f32 {
    if 1.0 - value >= -0.0 { value } else { 1.0 }
}
