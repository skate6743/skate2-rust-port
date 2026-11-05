//! Complete scalar tracker update at TU3 0x82E08640 (0x190 bytes).
//! Callers supply native state, parameters and timestep; no camera tuning or
//! initialization policy is inferred here. See docs/agents/camera/CAM-01.md.

/// The six scalar words written/read at tracker offsets +0 through +20.
/// Acceleration clamp retains its prior value when acceleration opposes error.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarTracker {
    pub target: f32,
    pub position: f32,
    pub velocity: f32,
    pub acceleration: f32,
    pub acceleration_clamp: f32,
    pub smoothing: f32,
}

/// Effective tracker fields +28 through +56, after the native caller binds
/// stock settings and applies its dynamic overrides. These are not defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarTrackerParameters {
    pub delta_umbra: f32,
    pub delta_penumbra: f32,
    pub speed_clamp: f32,
    pub acceleration_clamp_min: f32,
    pub acceleration_clamp_max: f32,
    pub smoothing_min: f32,
    pub smoothing_max: f32,
    pub overshoot_zeroes_velocity: bool,
}

impl ScalarTracker {
    /// Advance the complete native update. TU3 does not clamp dt, sanitize
    /// parameters or special-case dt == 0 in this function.
    pub fn update(&mut self, dt: f32, target: f32, parameters: ScalarTrackerParameters) {
        let position = self.position;
        let delta = target - position;
        self.target = target;
        if parameters.overshoot_zeroes_velocity && self.velocity * delta < 0.0 {
            // Multiplication preserves negative zero; assignment to +0 does not.
            self.velocity *= 0.0;
        }
        let velocity = self.velocity;
        let reciprocal_dt = 1.0 / dt;
        self.acceleration = reciprocal_dt.mul_add(delta, -velocity) * reciprocal_dt;

        let absolute_delta = delta.abs();
        let fraction = if absolute_delta < parameters.delta_umbra {
            1.0
        } else if absolute_delta < parameters.delta_penumbra {
            1.0 - (absolute_delta - parameters.delta_umbra)
                / (parameters.delta_penumbra - parameters.delta_umbra)
        } else {
            0.0
        };

        if self.acceleration * delta > 0.0 {
            let limit = interpolate(
                parameters.acceleration_clamp_min,
                parameters.acceleration_clamp_max,
                fraction,
            );
            self.acceleration_clamp = limit;
            let absolute_acceleration = self.acceleration.abs();
            if absolute_acceleration > limit {
                self.acceleration = (self.acceleration / absolute_acceleration) * limit;
            }
        }

        self.velocity = self.acceleration.mul_add(dt, velocity);
        let absolute_velocity = self.velocity.abs();
        if absolute_velocity > parameters.speed_clamp {
            self.velocity = (self.velocity / absolute_velocity) * parameters.speed_clamp;
        }

        let next_position = self.velocity.mul_add(dt, position);
        self.smoothing = interpolate(parameters.smoothing_min, parameters.smoothing_max, fraction);
        // Smoothing changes position only. Do not derive velocity back from it.
        self.position = (next_position - position).mul_add(1.0 - self.smoothing, position);
    }
}

pub(super) fn interpolate(minimum: f32, maximum: f32, fraction: f32) -> f32 {
    // The equal-parameter branch avoids a multiply/add, including for NaNs in
    // the unused fraction. Native otherwise uses a VMX fused multiply-add.
    if minimum == maximum {
        maximum
    } else {
        (maximum - minimum).mul_add(fraction, minimum)
    }
}
