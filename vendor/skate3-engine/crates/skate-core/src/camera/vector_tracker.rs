//! TU3 vector-tracker candidates.
use super::{ScalarTrackerParameters, tracker::interpolate};

/// Four lanes are retained because native vector stores update all four.
/// Magnitudes and direction tests use xyz only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorTracker {
    pub target: [f32; 4],
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub acceleration: [f32; 4],
    pub acceleration_clamp: f32,
    pub smoothing: f32,
}

impl VectorTracker {
    pub fn update(&mut self, dt: f32, target: [f32; 4], p: ScalarTrackerParameters) {
        self.target = target;
        let old = self.position;
        let delta = core::array::from_fn(|i| target[i] - old[i]);
        if p.overshoot_zeroes_velocity && dot(delta, self.velocity) < 0.0 {
            self.velocity = self.velocity.map(|v| v * 0.0);
        }
        let inverse_dt = refined_reciprocal(dt);
        self.acceleration = core::array::from_fn(|i| {
            // Separate VMX multiply/subtract/multiply, unlike scalar FMSUBS.
            (inverse_dt * delta[i] - self.velocity[i]) * inverse_dt
        });
        let magnitude = length(delta);
        let fraction = if magnitude < p.delta_umbra {
            1.0
        } else if magnitude < p.delta_penumbra {
            1.0 - (magnitude - p.delta_umbra) / (p.delta_penumbra - p.delta_umbra)
        } else {
            0.0
        };
        if dot(self.acceleration, delta) > 0.0 {
            self.acceleration_clamp =
                interpolate(p.acceleration_clamp_min, p.acceleration_clamp_max, fraction);
            self.acceleration = clamp_magnitude(self.acceleration, self.acceleration_clamp);
        }
        self.velocity =
            core::array::from_fn(|i| self.acceleration[i].mul_add(dt, self.velocity[i]));
        self.velocity = clamp_magnitude(self.velocity, p.speed_clamp);
        let next: [f32; 4] = core::array::from_fn(|i| self.velocity[i].mul_add(dt, old[i]));
        self.smoothing = interpolate(p.smoothing_min, p.smoothing_max, fraction);
        self.position =
            core::array::from_fn(|i| (next[i] - old[i]).mul_add(1.0 - self.smoothing, old[i]));
    }
}

/// TU3 82E08D18 preserves the fourth lane unless the xyz magnitude is capped;
/// then every lane receives the same multiplier.
pub fn clamp_magnitude(value: [f32; 4], limit: f32) -> [f32; 4] {
    let magnitude = length(value);
    if magnitude > limit {
        let scale = limit / magnitude;
        value.map(|v| v * scale)
    } else {
        value
    }
}

pub(super) fn refined_reciprocal(value: f32) -> f32 {
    let mut estimate = reciprocal_estimate(value);
    for _ in 0..2 {
        estimate = estimate.mul_add((-estimate).mul_add(value, 1.0), estimate);
    }
    estimate
}

fn reciprocal_estimate(value: f32) -> f32 {
    crate::physics::native_arithmetic::reciprocal_estimate(value)
}

pub(super) fn length(value: [f32; 4]) -> f32 {
    let square = dot(value, value);
    let mut estimate = crate::physics::reciprocal_sqrt::estimate(square);
    for _ in 0..2 {
        let residual = (-square).mul_add(estimate * estimate, 1.0);
        estimate = (estimate * 0.5).mul_add(residual, estimate);
    }
    if square == 0.0 {
        0.0
    } else {
        square * estimate
    }
}

pub(super) fn dot(a: [f32; 4], b: [f32; 4]) -> f32 {
    crate::physics::native_arithmetic::dot3(a, b)
}
