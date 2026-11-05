//! TU3 Skateboard::UpdateSpeedModel 82C04F68, including its force-enable gate.
//! The caller supplies GetEffectiveTransform and GetMass outputs, not a guessed
//! direction or mass. Ground submits this result as force tag 6.
use super::vector::{clamp, dot3, normalize};
use crate::point_graph::PointGraph;

#[derive(Clone)]
pub struct SpeedModelSettings {
    /// physics_speedmodel layout +0,+8,+16,+20,+24.
    pub negative_gain: f32,
    pub maximum_gravity_acceleration: f32,
    pub gravity: f32,
    pub positive_gain: f32,
    pub coffin_acceleration: f32,
    /// Surface layout +260 and curve +96/+128.
    pub speed_error_bound: f32,
    pub surface_friction: PointGraph<8>,
    /// physics_manual layout +180,+212,+220.
    pub manual_acceleration: f32,
    pub negative_manual_angle_limit: f32,
    pub manual_angle_limit: f32,
    /// Global +184 layout +356 and curves +144/+176,+224/+256.
    pub no_input_delay: f32,
    pub no_input_friction: PointGraph<8>,
    pub manual_friction: PointGraph<8>,
    /// Selected-mode layout +44 and +40.
    pub override_enabled: bool,
    pub override_speed: f32,
    /// Live constants 830BD350 and 830BD380 (four native lanes each).
    pub normal_threshold: [f32; 4],
    pub override_direction_threshold: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct SpeedModelState {
    pub target_speed: f32, // Board +260
    pub flags_1360: u32,
}

pub struct SpeedModelInput {
    pub flags_2468: u32,
    pub flags_2476: u32,
    pub wheel_contact_count: i32, // +2556
    pub frames_2580: i32,
    pub timestep: f32,     // +2604
    pub signed_speed: f32, // +2612
    pub angle_2652: f32,
    pub surface_speed: f32, // +2656
    pub turn_2672: f32,
    pub balance: f32,               // +2720
    pub elapsed_without_input: f32, // +2748
    pub manual_state_276: f32,      // Board +276
    pub vector_160: [f32; 4],
    pub vector_352: [f32; 4],
    pub velocity_416: [f32; 4],
    pub normal_464: [f32; 4],
    /// Z column of 82C01BF8's result; stance correction has already happened.
    pub effective_forward: [f32; 4],
    /// 82C06F58 result, whose two-accumulator summation is implemented in mass.
    pub mass: f32,
}

/// Force vector followed by zeroed local application point, all four lanes.
pub fn update(
    state: &mut SpeedModelState,
    settings: &SpeedModelSettings,
    input: &SpeedModelInput,
) -> [f32; 8] {
    let speed = input.signed_speed.abs();
    let dt = input.timestep;
    let bound = settings.speed_error_bound;
    let lower = speed - bound;
    let lower = if -lower >= 0.0 { 0.0 } else { lower };
    let mut target = if state.flags_1360 & 0x8000_0000 != 0 {
        state.flags_1360 &= 0x7fff_ffff;
        speed
    } else {
        clamp(state.target_speed, lower, bound + speed)
    };
    let coffin = if input.flags_2476 & 0x4000_0000 != 0 {
        settings.coffin_acceleration * dt
    } else {
        0.0
    };
    let projection = dot3(input.velocity_416, input.effective_forward);
    let projected = input.effective_forward.map(|v| v * projection);
    let normal_speed = dot3(projected, input.normal_464);
    let tangent = normalize(
        core::array::from_fn(|i| projected[i] - input.normal_464[i] * normal_speed),
        settings.normal_threshold,
    );
    let gravity = clamp(
        -tangent[1] * settings.gravity,
        -settings.maximum_gravity_acceleration,
        settings.maximum_gravity_acceleration,
    ) * dt;

    if input.frames_2580 < 30 {
        let active = if input.manual_state_276 == 0.0 {
            0.0
        } else {
            1.0
        };
        let mut direction = if input.balance > 0.0 { 1.0 } else { -1.0 };
        if 0.0 > dot3(input.vector_352, tangent) {
            direction *= -1.0;
        }
        let angle = input.angle_2652.abs();
        let within = if angle > settings.manual_angle_limit {
            0.0
        } else {
            1.0
        };
        let negative_within = if angle > settings.negative_manual_angle_limit {
            0.0
        } else {
            1.0
        };
        let mut acceleration =
            ((((input.turn_2672 * input.turn_2672) * settings.manual_acceleration) * within)
                * direction)
                * active;
        if acceleration < 0.0 {
            acceleration *= negative_within;
        }
        target = acceleration + target;
    }
    let friction = -settings.surface_friction.evaluate(input.surface_speed) * dt;
    let surface_delta = if gravity > 0.0 {
        friction + gravity
    } else if friction - gravity >= 0.0 {
        gravity
    } else {
        friction
    };
    target = (surface_delta + coffin) + target;
    if input.elapsed_without_input > settings.no_input_delay {
        target = -settings
            .no_input_friction
            .evaluate(input.surface_speed)
            .mul_add(dt, -target);
    }
    if input.balance != 0.0 {
        target = -settings
            .manual_friction
            .evaluate(input.surface_speed)
            .mul_add(dt, -target);
    }
    state.target_speed = if target >= 0.0 { target } else { 0.0 };
    let mut error = clamp(state.target_speed - speed, -bound, bound);
    if settings.override_enabled
        && input.flags_2476 & 0x0040_0000 != 0
        && input.normal_464[1] > 0.65
    {
        let mut goal = settings.override_speed * f32::from_bits(0x3E8E_38E4);
        let along = dot3(input.velocity_416, input.vector_160);
        if settings
            .override_direction_threshold
            .iter()
            .all(|t| *t > along)
            && 1.0 > input.surface_speed
        {
            goal *= -1.0;
        }
        error = clamp(goal - speed, -0.4, 0.4);
    }
    let enabled = input.flags_2468 & 0x6201_0000 == 0
        && input.flags_2476 & 0x0800_0000 == 0
        && input.wheel_contact_count >= 2;
    let gain = if !enabled {
        0.0
    } else if error < 0.0 {
        settings.negative_gain
    } else {
        settings.positive_gain
    };
    let scalar = ((input.mass * gain) * error) / dt;
    let forward = dot3(input.effective_forward, tangent) > 0.0;
    let mut output = [0.0; 8];
    for i in 0..4 {
        let axis = if forward {
            input.effective_forward[i]
        } else {
            -input.effective_forward[i]
        };
        output[i] = axis * scalar;
    }
    output
}
