//! Normal camera state timing82DFEAA8 and stance tracking82E00108.
use super::manager_state::{STEP, mirror_parameters};
use super::{ManagerState, ManagerSubject, Rig, ShotManager};

impl ManagerState {
    pub(super) fn update_timing(&mut self, subject: &ManagerSubject, steering_threshold: f32) {
        self.time_without_trajectory = if subject.rig.trajectory_valid != 0 {
            0.0
        } else {
            self.time_without_trajectory + STEP
        };
        if subject.reset != 0 || self.reset_requested() {
            self.options |= 0x80;
            self.flags &= !4;
            self.trajectory_camera_distance = 0.0;
            self.centred_time = 0.0;
            self.steering_time = 0.0;
            self.collision_elevation = 0.0;
            self.instant_frames = 3;
            self.anchor_velocity = [0.0; 4];
        }
        self.steering_time = if subject.steering[0].abs() <= steering_threshold {
            0.0
        } else {
            self.steering_time + STEP
        };
        self.centred_time =
            if subject.steering_for_turn(self.mirrored()).abs() >= steering_threshold {
                0.0
            } else {
                self.centred_time + STEP
            };
        self.flags = (self.flags & !8) | (u8::from(subject.stance_592 != subject.stance_560) << 3);
    }

    pub(super) fn update_mirrors(
        &mut self,
        dt: f32,
        subject: &ManagerSubject,
        rig: &Rig,
        shots: &ShotManager,
    ) {
        if !self.reset_requested() && !subject.is_ground_camera(rig.fields.height_mode) {
            return;
        }
        let option = if self.reset_requested() {
            shots.first_leaf().mirror_for_stance
        } else {
            shots.current().shot.mirror_for_stance
        };
        let target = if option != 0 && self.mirrored() {
            -1.0
        } else {
            1.0
        };
        if self.reset_requested() {
            for tracker in [&mut self.heading_mirror, &mut self.framing_mirror] {
                tracker.target = target;
                tracker.position = target;
                tracker.velocity = 0.0;
                tracker.acceleration = 0.0;
            }
        } else {
            self.heading_mirror
                .update(dt, target, mirror_parameters(100.0, 500.0));
            self.framing_mirror
                .update(dt, target, mirror_parameters(15.0, 50.0));
        }
    }
}
