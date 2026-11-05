//! Camera trajectory transitions82DFE0E0 and slope relation82DFE718.
use super::manager_state::clamp;
use super::vector_tracker::{dot, length};
use super::{ManagerState, ManagerSubject, Rig, direction_to_angles, normalize_angle};

impl ManagerState {
    /// Returns the exact normal/velocity pair passed to ShakeEffect82E055D8
    /// when the native ground-camera transition triggers a landing impulse.
    pub(super) fn update_air(
        &mut self,
        subject: &ManagerSubject,
        rig: &mut Rig,
    ) -> Option<([f32; 4], [f32; 4])> {
        let previous_delta = self.landing_height_delta;
        self.landing_height_delta = subject.landing_position[1] - subject.launch_position[1];
        self.apex_height = subject.apex_position[1] - subject.launch_position[1];
        self.apex_time = subject.apex_time;
        self.launch_position = subject.launch_position;
        self.apex_height_ratio = if self.trajectory_camera_distance != 0.0 {
            self.apex_height / self.trajectory_camera_distance
        } else {
            0.0
        };
        self.landing_normal = subject.landing_normal;
        self.launch_incline = crate::trigonometry::acos(clamp(subject.launch_normal[1], -1.0, 1.0));
        self.flags &= !0x10;
        let ground = subject.is_ground_camera(rig.fields.height_mode);
        if ground || subject.rig.trajectory_valid == 0 {
            self.flags &= !4;
        } else {
            if self.flags & 4 != 0 {
                if (self.landing_height_delta - previous_delta).abs() > f32::from_bits(0x3a83126f) {
                    rig.fields.flags_517 |= 8;
                }
            } else {
                let selected = subject.anchors[self.selected_anchor as usize].position;
                self.trajectory_camera_distance = length(core::array::from_fn(|i| {
                    rig.positioner.position[i] - selected[i]
                }));
                rig.fields.flags_517 &= !8;
            }
            self.flags |= 4;
            self.landing_incline =
                crate::trigonometry::acos(clamp(self.landing_normal[1], -1.0, 1.0));
            self.signed_landing_incline = self.landing_incline;
            if dot(subject.rig.transform[2], self.landing_normal) > 0.0 {
                self.signed_landing_incline = -self.landing_incline;
            }
            if self.launch_incline > f32::from_bits(0x3f9c61aa)
                && self.landing_incline > f32::from_bits(0x3f060a92)
            {
                self.flags |= 0x10;
            }
            self.update_slope_relation(subject);
        }
        rig.fields.height_transition_weight = if !ground && subject.rig.trajectory_valid != 0 {
            clamp(
                subject.trajectory_time / subject.trajectory_duration,
                0.0,
                1.0,
            )
        } else {
            0.0
        };
        let mut impulse = None;
        if ground {
            if self.flags & 0x40 == 0 && rig.positioner.flags & 0x80 == 0 {
                if dot(self.previous_velocity, self.ground_normal) < -0.0 {
                    impulse = Some((self.ground_normal, self.previous_velocity));
                }
                self.landing_height_delta = 0.0;
            }
        } else if rig.fields.height_transition_time >= rig.fields.height_transition_duration
            && subject.rig.trajectory_valid != 0
            && self.landing_height_delta > -3.0
            && subject.flag_684 == 0
        {
            if self.flags & 2 != 0 || subject.rig.air_flag_452 != 0 {
                rig.fields.flags_516 |= 0x40;
            } else if rig.fields.previous_height != f32::MAX
                && rig.fields.reference_height != f32::MAX
            {
                rig.fields.height_transition_start = rig.fields.reference_height;
                rig.fields.height_transition_end =
                    self.landing_height_delta + rig.fields.reference_height;
                rig.fields.height_transition_duration =
                    subject.trajectory_duration - subject.trajectory_time;
                rig.fields.height_transition_time = 0.0;
            }
        }
        self.flags = (self.flags & !0x40) | (u8::from(ground) << 6);
        self.previous_velocity = rig.fields.velocity;
        impulse
    }

    fn update_slope_relation(&mut self, subject: &ManagerSubject) {
        self.launch_landing_heading_delta = f32::from_bits(0x40490fdb);
        if subject.launch_normal[1] < f32::from_bits(0x3f7d70a4)
            && subject.landing_normal[1] < f32::from_bits(0x3f7d70a4)
        {
            let launch = normalize_angle(direction_to_angles(subject.launch_normal)[1]);
            let landing = normalize_angle(direction_to_angles(subject.landing_normal)[1]);
            self.launch_landing_heading_delta = normalize_angle(landing - launch).abs();
        }
    }
}
