//! Complete normal-shot blend callback82E07708 and air blends82DFE898/E998.
use super::manager_state::{DEGREES, clamp};
use super::vector_tracker::dot;
use super::{ManagerState, ManagerSubject, Rig, ShotEnvironment};

pub(super) struct BlendEnvironment<'a> {
    pub state: &'a ManagerState,
    pub subject: &'a ManagerSubject,
    pub rig: &'a Rig,
    pub rig_forward: [f32; 4],
    pub angular_velocity: [f32; 4],
    /// Rig424, including the compass2 override from the previous binding.
    pub heading_multiplier: f32,
    /// ShotManager992+60: previous shot distance.
    pub previous_distance: f32,
}
impl ShotEnvironment for BlendEnvironment<'_> {
    fn heading_mirror(&self) -> f32 {
        self.heading_multiplier
    }
    fn compass_north(&self, entry: u32) -> f32 {
        self.subject.compass[entry as usize]
    }
    fn special_camera_flag(&self) -> bool {
        self.state.flags & 0x10 != 0
    }
    fn raw_blend_value(&mut self, kind: u32, authored: f32, dt: f32) -> f32 {
        let s = self.subject;
        let m = self.state;
        match kind {
            0 => authored,
            1 => dot(self.rig.fields.velocity, s.rig.transform[2]).abs(),
            2 => dot(self.rig.fields.acceleration, self.rig_forward),
            3 => self.heading_multiplier * (-self.angular_velocity[1]),
            4 => m.apex_height,
            5 => self.air_progress(),
            6 => m.velocity_incline * DEGREES,
            7 => self.previous_distance,
            8 => self.apex_progress(),
            9 => m.absolute_velocity_incline * DEGREES,
            10 => m.landing_incline * DEGREES,
            11 => {
                if m.mirrored() {
                    -s.steering[1]
                } else {
                    s.steering[1]
                }
            }
            12 => s.steering_for_blend(m.mirrored()),
            13 => {
                if dt == 0.0 {
                    0.0
                } else {
                    s.steering_for_turn(m.mirrored())
                }
            }
            14 => {
                if dt == 0.0 {
                    0.0
                } else {
                    f32::MAX
                }
            }
            15 => m.frontside_angle,
            16 => s.valid_trajectory_duration(),
            17 => s.look[0],
            18 => s.look[1],
            19 => s.look_heading(),
            20 => m.direction_incline * DEGREES,
            21 => s.value_512,
            22 => -s.value_516,
            23 => m.effect_weight,
            _ => 0.5,
        }
    }
}
impl BlendEnvironment<'_> {
    fn air_progress(&self) -> f32 {
        let s = self.subject;
        if s.is_ground_camera(self.rig.fields.height_mode) {
            return 1.0;
        }
        if self.state.apex_height <= 0.0 {
            return 0.0;
        }
        let remaining = s.trajectory_duration - s.trajectory_time;
        clamp(
            if s.trajectory_duration <= 0.0 {
                1.0
            } else {
                (s.trajectory_duration - remaining) / s.trajectory_duration
            },
            0.0,
            1.0,
        )
    }
    fn apex_progress(&self) -> f32 {
        let s = self.subject;
        let m = self.state;
        if s.is_ground_camera(self.rig.fields.height_mode) {
            return if m.flags & 0x40 != 0 { 0.0 } else { 1.0 };
        }
        if m.apex_height <= 0.0 || s.valid_trajectory_duration() < m.apex_time {
            return 0.0;
        }
        let twice = m.apex_time * 2.0;
        let remaining = s.valid_trajectory_duration() - twice;
        if s.trajectory_time > twice {
            clamp((s.trajectory_time - twice) / remaining + 1.0, 1.0, 2.0)
        } else {
            s.trajectory_time / twice
        }
    }
}
