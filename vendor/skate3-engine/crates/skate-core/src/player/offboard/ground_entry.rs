//! Numerical BipedGround Enter82D30808 and local reset fields82D30BD0.
//! The game lifecycle also resets toolkit/contact data, initializes the controller,
//! enters board manager and skeleton mode4, and resets trajectory helpers in order.
mod math;
use math::*;
pub type Vector = [f32; 4];
pub type Frame = [Vector; 4];

/// Ground physical-state fields, separate from the Biped controller.
#[derive(Clone, Debug)]
pub struct State {
    pub frame_80: Frame,
    pub flags_144_to_150: [bool; 7],
    pub counter_152: u32,
    pub counter_156: u32,
    pub elapsed_160: f32,
    pub distance_164: f32,
    pub elapsed_168: f32,
    pub angle_172: f32,
    pub angular_velocity_176: f32,
    pub duration_180: f32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            frame_80: IDENTITY,
            flags_144_to_150: [false; 7],
            counter_152: 0,
            counter_156: 0,
            elapsed_160: 0.0,
            distance_164: f32::from_bits(0x5015_02f9),
            elapsed_168: 0.0,
            angle_172: 0.0,
            angular_velocity_176: 0.0,
            duration_180: 0.0,
        }
    }
}
pub struct Input {
    /// Actual82BE3650 animation-to-world frame, before changing skeleton mode.
    pub animation_frame: Frame,
    pub processed_velocity_608: Vector,
    pub processed_flags_2484: u32,
    pub processed_flags_2476: u32,
    pub requested_angle_2936: f32,
    pub requested_duration_2896: f32,
    pub previous_state_2504: u32,
    pub previous_frame_up_208: Vector,
    /// Actual skeleton15872; feeds Biped initialization input80.
    pub body_position_15872: Vector,
}
pub struct Placement {
    pub frame: Frame,
    pub planar_velocity: Vector,
    pub body_position: Vector,
}
impl State {
    /// Call following toolkit/contact reset; pass returned fields to82D7B7A0.
    pub fn enter(&mut self, input: &Input) -> Placement {
        *self = Self::default();
        self.flags_144_to_150[4] = input.previous_state_2504 == 502;
        let up = input.animation_frame[1];
        let velocity = input.processed_velocity_608;
        let planar = sub(velocity, scale(up, dot(up, velocity)));
        let mut frame = input.animation_frame;
        if input.processed_flags_2484 & 0x4000 != 0 {
            self.flags_144_to_150[6] = true;
            self.duration_180 = input.requested_duration_2896;
            let mut angle = input.requested_angle_2936;
            if input.processed_flags_2476 & 4 != 0 {
                angle *= -1.0;
            }
            let direction = if length(planar) < f32::from_bits(0x3dcc_cccd) {
                input.animation_frame[2]
            } else {
                planar
            };
            let basis = super::controller::build_frame(up, direction);
            frame[0] = basis[0];
            frame[1] = basis[1];
            frame[2] = basis[2];
            let rotated = rotate(input.animation_frame[2], up, angle);
            self.angle_172 = wrap(signed_angle(direction, rotated, up));
            let mut original = wrap(signed_angle(input.animation_frame[2], direction, up));
            if !(angle.abs() <= f32::from_bits(0x3fc9_0fdb)) && !(original * angle >= 0.0) {
                original += if original > 0.0 { -TAU } else { TAU };
            }
            self.angular_velocity_176 = (self.angle_172 + original) / self.duration_180;
        }
        self.frame_80 = frame;
        // Source sets149 after board-manager entry and before skeleton mode4.
        self.flags_144_to_150[5] = true;
        if input.previous_state_2504 == 501 {
            self.flags_144_to_150[1] = f32::from_bits(0x3f35_c28f) > input.previous_frame_up_208[1];
        }
        Placement {
            frame,
            planar_velocity: planar,
            body_position: input.body_position_15872,
        }
    }
}
#[cfg(test)]
mod tests;
