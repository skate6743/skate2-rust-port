//! Original TU3 82D2EF38 first-contact orientation and 82D2E968/82D2E7C8.
//! S2 82D94158/82D88320 instead adjusts body spin; that algorithm is not used.
use super::{State, Vector, math::*};
use crate::player::wipeout_state::{math::limit_length, orientation::projected_angle};

impl State {
    /// Return the actual adjustment to submit to selector82D6DE08. This does
    /// not invent a completion or change selector state before its host call.
    pub fn animation_adjustment(&mut self, value_2880: Vector) -> Option<Vector> {
        if self.flags_544_550[1] {
            if value_2880[1] > 0.1 {
                let delta = sub(value_2880, self.adjustment_496);
                if dot(delta, delta) > 0.001 {
                    self.adjustment_496 = add(self.adjustment_496, limit_length(delta, 0.2));
                    return Some(self.adjustment_496);
                }
            }
        } else if self.flags_544_550[0] && value_2880[1] > 0.1 {
            self.flags_544_550[1] = true;
            self.adjustment_496 = value_2880;
            return Some(value_2880);
        }
        None
    }

    /// Called immediately after selector sampling; frames remain animation
    /// targets. Only the shared physical solver advances the physical bodies.
    pub fn orient_sample(&mut self, start_angle_2936: f32, flags_2476: u32) {
        if !self.flags_544_550[0] && self.result.valid_404 {
            self.frame_80 = self.frame_208;
            let mut up = UP;
            if self.result.normal_304[1] > 0.85 {
                up = limit_angle(
                    normalize_or(add(self.result.normal_304, UP), self.frame_80[1]),
                    self.frame_80[1],
                    (self.result.time_remaining_384 * 180.) * RADIANS,
                );
            } else if self.frame_80[1][1] < 0.71 {
                self.flags_544_550[5] = true;
            }
            self.landing_orientation(
                self.result.contact_velocity_320,
                up,
                self.result.time_remaining_384,
                start_angle_2936,
                flags_2476,
            );
            self.flags_544_550[0] = true;
            self.flags_544_550[2] = false;
        }
        self.update_times();
        self.frame_208 = crate::animation::foot_ik::interpolate_native(
            &self.frame_80,
            &self.frame_144,
            self.blend_440,
        )
        .0;
        // The shared interpolation helper preserves native SIMD scratch lanes.
        // Air publishes a geometric frame, not a register image. Convert here,
        // before COM/root construction and the eventual Ground placement.
        // Deliberately leave the shared helper and all XYZ calculations intact.
        self.frame_208 = self.frame_208.map(|v| [v[0], v[1], v[2], 0.]);
    }

    fn landing_orientation(
        &mut self,
        velocity: Vector,
        up: Vector,
        remaining: f32,
        start_angle: f32,
        flags_2476: u32,
    ) {
        if self.flags_544_550[6] {
            self.vector_512 = self.frame_80[2];
            let horizontal = flatten(velocity);
            if dot(horizontal, horizontal) > 0.01 {
                let angle = if flags_2476 & 4 != 0 {
                    -start_angle
                } else {
                    start_angle
                };
                self.vector_512 = normalize(rotate(normalize(horizontal), UP, -angle));
            }
            self.frame_144 = super::super::super::controller::build_frame(up, self.vector_512);
            return;
        }
        let degrees = (wrap_angle(projected_angle(self.frame_208[2], velocity, up))
            * f32::from_bits(0x4265_2ee1))
        .abs();
        if self.flags_544_550[2] {
            self.frame_80 = self.frame_208;
            if degrees > 90. {
                return;
            }
        }
        self.frame_144 = self.frame_208;
        if remaining < 0.033 {
            self.frame_80 = self.frame_208;
            return;
        }
        let tangent = sub(velocity, scale(up, dot(velocity, up)));
        let mut right = if length(tangent) > 1. {
            normalize_or(cross(up, velocity), self.frame_144[0])
        } else {
            normalize_or(cross(up, self.frame_208[2]), self.frame_144[0])
        };
        if length(tangent) > 1. && degrees > 90. && degrees / remaining > 540. {
            self.flags_544_550[5] = true;
            right = scale(right, -1.);
        }
        self.frame_144[0] = right;
        self.frame_144[1] = up;
        self.frame_144[2] = normalize_or(cross(right, up), self.frame_144[2]);
    }

    ///82D2F698..6CC: the larger early assist decays over the source interval.
    pub fn landing_assist_limit(elapsed_2664: f32) -> f32 {
        let time = select(-(elapsed_2664 - 0.034), 0., elapsed_2664 - 0.034);
        let blend = clamp(-(time * 6.25 - 1.), 0., 1.);
        (1. - blend) * 0.5 + blend * 3.5
    }

    pub fn finish_landing_latch(&mut self) {
        if self.time_remaining_444 <= 0. && self.result.valid_404 && self.result.normal_304[1] > 0.1
        {
            self.flags_544_550[4] = false;
        }
    }
}
