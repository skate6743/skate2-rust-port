//! CameraMan shot-to-rig binding82DFF23C..82DFF864.
use super::manager_state::clamp;
use super::vector_tracker::length;
use super::{DropPredictor, LookInput, ManagerState, ManagerSubject, Rig, RigOrientation, Shot};

impl ManagerState {
    pub(super) fn bind_shot(
        &mut self,
        shot: Shot,
        subject: &ManagerSubject,
        rig: &mut Rig,
        orientation: &mut RigOrientation,
        drop: &DropPredictor,
        look: LookInput,
        previous_roll: f32,
        transitioning: bool,
    ) -> f32 {
        let reset = self.reset_requested();
        rig.fields.avoidance_mode = u32::from(self.options & 0x20 != 0);
        rig.fields.flags_516 = (rig.fields.flags_516 & !0x62)
            | (u8::from(transitioning) << 5)
            | ((shot.follow_subject_in_air << 6) & 0x40)
            | ((shot.avoidance_override << 1) & 2);
        rig.fields.flags_517 = (rig.fields.flags_517 & 0x7f) | (subject.stance_560 << 7);
        let [x, y, z, w] = shot.arm_orientation;
        let basis = crate::physics::rigid_body::basis_from_quaternion(
            crate::physics::rigid_body::RetailQuaternion { x, y, z, w },
        );
        let at = basis.columns[2];
        let angles = super::direction_to_angles([at[0], at[1], at[2], 0.0]);
        let mut elevation = -angles[0];
        let mut heading = angles[1];
        let mut distance = shot.distance;
        if subject.special_effect != 0 {
            elevation += f32::from_bits(0x3eb2b8c2);
            let time = (self.frames as f32) * f32::from_bits(0x3dd67750);
            let distance_wave = crate::trigonometry::sin(time * f32::from_bits(0x3d321643));
            let heading_wave = crate::trigonometry::sin(time * f32::from_bits(0x3c23d70a));
            heading = heading_wave.mul_add(f32::from_bits(0x40278d36), heading);
            distance = (distance_wave + distance) + 1.0;
        }
        let mut drop_elevation = 0.0;
        if self.options & 0x40 != 0 {
            let speed = length(self.anchor_velocity);
            let (minimum, maximum) = if self.options & 0x10 != 0 {
                (0.7, 0.8)
            } else {
                (0.9, 1.0)
            };
            let scale = clamp(
                -(speed - 4.0).mul_add(f32::from_bits(0x3cccccd0), -maximum),
                minimum,
                maximum,
            );
            drop_elevation = drop.elevation * scale;
            distance = drop.distance_offset + distance;
        }
        let collision_elevation = if reset {
            0.0
        } else {
            let maximum = if distance - 2.5 >= 0.0 { 2.5 } else { distance };
            let fraction = clamp(
                (maximum - rig.positioner.available_distance) / maximum,
                0.0,
                1.0,
            );
            let angle = (if subject.rig.off_board != 0 {
                60.0
            } else {
                50.0
            }) * f32::from_bits(0x3c8efa35);
            let target = angle.mul_add(fraction, -drop_elevation);
            let mut target = if target >= 0.0 { target } else { 0.0 };
            if subject.is_ground_camera(rig.fields.height_mode) && subject.ground_normal[1] < -0.2 {
                target *= -1.0;
            }
            let outward = (target > 0.0 && self.collision_elevation < target)
                || (target < 0.0 && self.collision_elevation > target);
            let step = angle
                * (if outward {
                    f32::from_bits(0x3d888889)
                } else {
                    f32::from_bits(0x3c23d70a)
                });
            self.collision_elevation = if (target - self.collision_elevation).abs() > step {
                if target >= self.collision_elevation {
                    self.collision_elevation + step
                } else {
                    self.collision_elevation - step
                }
            } else {
                target
            };
            clamp(
                self.collision_elevation,
                -f32::from_bits(0x3fb2b8c2),
                f32::from_bits(0x3fb2b8c2),
            )
        };
        rig.fields.distance = distance;
        let turns = heading.mul_add(f32::from_bits(0x3e22f983), 0.5).floor();
        let reference = -turns.mul_add(f32::from_bits(0x40c90fdb), -heading);
        rig.fields.heading_reference = reference;
        rig.fields.heading_target = if reset {
            reference
        } else {
            let from = rig.fields.avoidance_heading;
            let delta = super::rig_tracking::wrap_vmx(reference - from);
            super::rig_tracking::wrap_vmx(delta.mul_add(1.0 - rig.fields.heading_weight, from))
        };
        rig.set_elevation(elevation, !reset);
        rig.fields.elevation_offset = drop_elevation + look.elevation;
        rig.fields.additional_elevation = collision_elevation;
        rig.fields.flags_517 =
            (rig.fields.flags_517 & !0x20) | (u8::from(shot.compass_north != 5) << 5);
        self.blur = if reset {
            0.0
        } else if transitioning {
            shot.transition_blur
        } else {
            shot.blur
        };
        let heading_mirror = if shot.compass_north == 2 {
            1.0
        } else {
            self.heading_mirror.position
        };
        orientation.framing_mirror = if shot.compass_north == 2 {
            1.0
        } else {
            self.framing_mirror.position
        };
        rig.fields.collision_mode = shot.collision_hint;
        let h = 1.0 - shot.smoothing[0];
        let e = 1.0 - shot.smoothing[1];
        let pan = 1.0 - shot.smoothing[2];
        let tilt = 1.0 - shot.smoothing[3];
        rig.fields.heading_smoothing = 1.0 - (-h.mul_add(h, -1.0));
        rig.fields.elevation_smoothing = 1.0 - (-e.mul_add(e, -1.0));
        orientation.pan_smoothing = -pan.mul_add(pan, -1.0);
        orientation.tilt_smoothing = -tilt.mul_add(tilt, -1.0);
        orientation.roll = previous_roll;
        heading_mirror
    }
}
