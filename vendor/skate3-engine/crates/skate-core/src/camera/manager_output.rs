//! Final output/shake ordering from82DFFAA4..82DFFF28.
use super::vector_tracker::length;
use super::{CameraMan, ManagerSubject, ShakeSamples, ShakeSettings};

impl CameraMan {
    pub(super) fn finish_frame(
        &mut self,
        dt: f32,
        subject: &ManagerSubject,
        settings: ShakeSettings,
        samples: [&ShakeSamples; 2],
        reset: bool,
    ) {
        let old_basis = self.frame.basis;
        let old_position = self.frame.position;
        let basis = self.orientation.basis;
        let position = self.rig.positioner.position;
        self.frame.motion(dt, basis, position);
        let enabled = subject.rig.wiping_out == 0
            || self.rig.positioner.flags & 0x80 != 0
            || subject.rig.broken_bone_slowmo != 0;
        let velocity = self.rig.fields.velocity;
        let speed = length([velocity[0], 0.0, velocity[2], 0.0])
            * if subject.flag_652 != 0 {
                f32::from_bits(0x3f6147ae)
            } else {
                1.0
            };
        let distance = length(core::array::from_fn(|i| {
            self.rig.fields.anchor[i] - position[i]
        }));
        let first = self.shake[0].update(dt, speed, distance, basis, enabled, samples[0], settings);
        let second = self.shake[1].update(
            dt,
            if subject.shake_variant != 0 {
                speed * 50.0
            } else {
                speed
            },
            distance,
            basis,
            enabled,
            samples[1],
            settings,
        );
        let selected = usize::from(subject.shake_variant != 0);
        self.frame.basis = [first, second][selected];
        self.frame.position = position;
        self.frame.shake_translation = self.shake[selected].translation;
        //82DFFEC8..82DFFECC publishes ShakeEffect+16 into Rig+320. The
        // positioner consumes this world-space offset on the following tick;
        // it is not an extra translation added by the renderer to this frame.
        self.rig.fields.offset = self.frame.shake_translation;
        self.frame.discontinuity = self.rig.positioner.flags & 0x40 != 0 || reset;
        if self.frame.discontinuity {
            self.frame.previous_basis = self.frame.basis;
            self.frame.previous_position = position;
        } else {
            self.frame.previous_basis = old_basis;
            self.frame.previous_position = old_position;
        }
        self.frame.field_of_view_degrees = self.state.field_of_view;
        self.frame.opacity = self.state.opacity;
        self.frame.blur = self.state.blur;
    }
}
