//! Movement compass82DF3AC0 and its two-dimensional dead-zone82DF9AD0.
use super::compass::{blend, horizontal, sub};
use super::manager_state::clamp;
use super::vector_tracker::{length, refined_reciprocal};
use super::{Compass, CompassInputs, CompassSettings, direction_to_angles};

impl Compass {
    pub(super) fn update_movement(&mut self, dt: f32, input: CompassInputs, s: CompassSettings) {
        if !input.no_trajectory {
            return;
        }
        let reset = dt == 0.0 || self.previous_reset;
        if reset {
            self.velocity = [0.0; 4];
            self.damped_velocity = [0.0; 4];
        } else {
            let inverse = refined_reciprocal(dt);
            self.velocity = sub(input.transform[3], self.previous_position).map(|v| v * inverse);
            let speed = length(horizontal(self.velocity));
            let fraction = clamp(
                (speed - s.maximum_deadzone_speed)
                    / (s.minimum_deadzone_speed - s.maximum_deadzone_speed),
                0.0,
                1.0,
            );
            self.deadzone_size = fraction
                .mul_add(s.maximum_deadzone_size, -self.deadzone_size)
                .mul_add(1.0 - s.deadzone_smoothing, self.deadzone_size);
            self.update_deadzone(dt, [input.transform[3][0], input.transform[3][2]]);
            self.damped_velocity = [
                self.deadzone_velocity[0],
                0.0,
                self.deadzone_velocity[1],
                self.deadzone_velocity[0],
            ];
            self.previous_position = input.transform[3];
        }
        let speed = length(horizontal(self.velocity));
        if reset {
            self.movement_heading = -direction_to_angles(input.skeleton_direction)[1];
        } else if input.grinding {
            self.movement_heading = -direction_to_angles(input.grind_direction)[1];
        } else {
            let mut target = self.movement_heading;
            let mut weight = 1.0;
            self.stopped_time = if input.wiping_out || speed > 0.5 {
                0.0
            } else {
                self.stopped_time + dt
            };
            if speed > 0.5 {
                weight = clamp(s.heading_response, 0.0, 1.0);
                target =
                    -direction_to_angles(super::orientation_math::normalize(self.damped_velocity))
                        [1];
            } else if !input.state_103 && self.stopped_time >= s.time_before_lineup {
                weight = s.lineup_speed;
                target = -direction_to_angles(input.transform[2])[1];
            }
            self.movement_heading = blend(self.movement_heading, target, weight);
        }
    }

    fn update_deadzone(&mut self, dt: f32, target: [f32; 2]) {
        let old = self.deadzone_position;
        let difference: [f32; 2] = core::array::from_fn(|i| target[i] - old[i]);
        // The source explicitly sums two squared lanes, not vmsum3fp.
        let square = difference[0] * difference[0] + difference[1] * difference[1];
        let mut inverse = crate::physics::reciprocal_sqrt::estimate(square);
        for _ in 0..2 {
            inverse = (inverse * 0.5).mul_add((-square).mul_add(inverse * inverse, 1.0), inverse);
        }
        let distance = if square == 0.0 { 0.0 } else { square * inverse };
        if distance > self.deadzone_size {
            self.deadzone_position = core::array::from_fn(|i| {
                (difference[i] * inverse).mul_add(distance - self.deadzone_size, old[i])
            });
        }
        self.deadzone_velocity = if dt == 0.0 {
            [0.0; 2]
        } else {
            core::array::from_fn(|i| (self.deadzone_position[i] - old[i]) * (1.0 / dt))
        };
    }
}
