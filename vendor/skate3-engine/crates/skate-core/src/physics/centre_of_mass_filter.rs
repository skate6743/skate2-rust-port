//! Physical-output COM conditioning: original TU3 82DE56B0/82DE5978.
//! Advances once after physical Skeleton publishes position64 and velocity16.
use super::native_arithmetic::{dot3, reciprocal_estimate};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CentreOfMassOutput {
    /// PhysOutSystemReckoning0, used by animation and camera consumers.
    pub velocity: [f32; 4],
    /// PhysOutSystemReckoning32.
    pub acceleration: [f32; 4],
    /// PhysOutSystemReckoning80.
    pub position: [f32; 4],
}

#[derive(Debug)]
pub struct CentreOfMassFilter {
    velocity: [f32; 4],
    position: [f32; 4],
    position_velocity: [f32; 4],
    accumulated_error: [f32; 4],
    position_valid: bool,
}

impl Default for CentreOfMassFilter {
    fn default() -> Self {
        // Original Reset82DE5588 clears vectors16/96/112/128 and byte191.
        Self {
            velocity: [0.0; 4],
            position: [0.0; 4],
            position_velocity: [0.0; 4],
            accumulated_error: [0.0; 4],
            position_valid: false,
        }
    }
}

impl CentreOfMassFilter {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, position: [f32; 4], velocity: [f32; 4]) -> CentreOfMassOutput {
        let dt = f32::from_bits(0x3c888889);
        let old_velocity = self.velocity;
        self.velocity = std::array::from_fn(|i| old_velocity[i].mul_add(0.75, velocity[i] * 0.25));
        let mut inverse_dt = reciprocal_estimate(dt);
        inverse_dt = inverse_dt.mul_add((-dt).mul_add(inverse_dt, 1.0), inverse_dt);
        inverse_dt = inverse_dt.mul_add((-dt).mul_add(inverse_dt, 1.0), inverse_dt);
        let acceleration =
            std::array::from_fn(|i| (self.velocity[i] - old_velocity[i]) * inverse_dt);
        if !self.position_valid {
            self.position = position;
            self.position_velocity = [0.0; 4];
            self.position_valid = true;
        } else {
            let coefficient: [f32; 4] = [0.05, 0.03, 0.05, 0.0];
            let unity: [f32; 4] = [1.0, 1.0, 1.0, 0.0];
            self.position_velocity = std::array::from_fn(|i| {
                (unity[i] - coefficient[i])
                    .mul_add(self.position_velocity[i], coefficient[i] * velocity[i])
            });
            let predicted: [f32; 4] =
                std::array::from_fn(|i| self.position_velocity[i].mul_add(dt, self.position[i]));
            self.position = std::array::from_fn(|i| {
                (unity[i] - coefficient[i]).mul_add(predicted[i], coefficient[i] * position[i])
            });
            let error = std::array::from_fn(|i| position[i] - self.position[i]);
            if dot3(error, self.accumulated_error) > 0.0 {
                self.position = std::array::from_fn(|i| {
                    self.accumulated_error[i].mul_add(0.08, self.position[i])
                });
            }
            self.accumulated_error = std::array::from_fn(|i| {
                self.accumulated_error[i].mul_add(0.95, (position[i] - self.position[i]) * 0.05)
            });
        }
        CentreOfMassOutput {
            velocity: self.velocity,
            acceleration,
            position: self.position,
        }
    }
}
