//! Camera-observable part of the TU3 trajectory batch. The variable-step walk
//! is 82770910, step sizing 8276C940 and collision time 8276C558. World storage
//! and scheduling are host-owned; the native segment sequence is retained.
use super::vector_tracker::{dot, length, refined_reciprocal};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrajectoryQuery {
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub gravity: [f32; 4],
    pub duration: f32,
    pub radius: f32,
    pub start_error: f32,
    pub end_error: f32,
}

impl TrajectoryQuery {
    /// Return the first trajectory-segment collision's reconstructed native
    /// time, or -1. The callback returns the closest swept-line contact point.
    /// The extra normal/mesh/frame fields of the native batch are not consumed
    /// by the gameplay camera request's GetCollisionTime accessor82DF9340.
    pub fn collision_time<E>(
        self,
        mut line: impl FnMut([f32; 4], [f32; 4], f32) -> Result<Option<[f32; 4]>, E>,
    ) -> Result<f32, E> {
        let mut start_step = self.duration;
        let mut end_step = self.duration;
        if self.gravity[1] < 0.0 {
            start_step = self.step_size(self.start_error * self.radius);
            end_step = self.step_size(self.end_error * self.radius);
        }
        let minimum_square = ((self.start_error * self.radius) * self.start_error) * self.radius;
        let mut time = 0.0;
        let mut start = self.evaluate(time);
        let mut end = self.evaluate(time + start_step);
        while self.duration > time {
            let delta = core::array::from_fn(|i| end[i] - start[i]);
            if dot(delta, delta) > minimum_square {
                // 82770B40 rejects only segments with every XYZ component at
                // or below this native threshold, before world enumeration.
                let visible = delta[..3]
                    .iter()
                    .any(|v| v.abs() > f32::from_bits(0x3780_0000));
                if visible {
                    if let Some(position) = line(start, end, self.radius)? {
                        return Ok(self.time_at_contact(position, time));
                    }
                }
                start = end;
            }
            let fraction = refined_reciprocal(self.duration) * time;
            let step = end_step.mul_add(fraction, (1.0 - fraction) * start_step);
            time += step;
            // The stock walk evaluates one step ahead and deliberately does
            // not clamp that point to duration. Do not replace with a uniform
            // sample count or an end-clamped parabola.
            end = self.evaluate(time + step);
        }
        Ok(-1.0)
    }

    fn evaluate(self, time: f32) -> [f32; 4] {
        let square = time * time;
        core::array::from_fn(|i| {
            (self.gravity[i] * 0.5)
                .mul_add(square, self.velocity[i].mul_add(time, self.position[i]))
        })
    }

    fn step_size(self, error_radius: f32) -> f32 {
        let square = refined_reciprocal(self.gravity[1]) * (-8.0 * error_radius);
        let mut inverse = crate::physics::reciprocal_sqrt::estimate(square);
        for _ in 0..2 {
            inverse = (inverse * 0.5).mul_add((-square).mul_add(inverse * inverse, 1.0), inverse);
        }
        if square == 0.0 { 0.0 } else { square * inverse }
    }

    fn time_at_contact(self, position: [f32; 4], mut time: f32) -> f32 {
        const FRAME: f32 = f32::from_bits(0x3c88_8889);
        const EPSILON: f32 = f32::from_bits(0x38d1_b717);
        if EPSILON >= (refined_reciprocal(2.0) * self.gravity[1]).abs() {
            let distance = length(core::array::from_fn(|i| position[i] - self.position[i]));
            if !(distance.abs() > EPSILON) {
                return 0.0;
            }
            let speed = length(self.velocity);
            if !(speed.abs() > EPSILON) {
                return 0.0;
            }
            // Preserve8276C794/7B4, including its frame factor. This branch
            // does not use the segment's fractional intersection parameter.
            return (refined_reciprocal(speed) * distance) * FRAME;
        }
        let mut closest_square = f32::MAX;
        while self.duration >= time {
            let point = self.evaluate(time);
            let delta = core::array::from_fn(|i| position[i] - point[i]);
            let square = dot(delta, delta);
            if closest_square >= square {
                closest_square = square;
            } else if square > closest_square {
                return time - FRAME;
            }
            time += FRAME;
        }
        self.duration
    }
}
