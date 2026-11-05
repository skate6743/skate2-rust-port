//! Original Reckoning ResetBodyFlip82D8E9E0 / BeginBodyFlip82D8EAB0.
use super::{math::*, runtime::Runtime};
use skate3_core::physics::skeleton_animation_record::IDENTITY;
impl Runtime<'_> {
    pub(super) fn reset_flip(&mut self) {
        let frames = &mut self.physics.riding.reckoning_frames;
        let n = self.physics.riding.reckoning.up;
        let up = rotate(&frames.body_flip, [n.x, n.y, n.z, 0.0]);
        self.physics.riding.reckoning.up = xyz(up);
        frames.heading = rotate(&frames.body_flip, frames.heading);
        frames.body_flip = IDENTITY;
        let state = &mut self.skater.air_reckoning.state;
        state.flip_active = false;
        state.flip_speed = 0.0;
        state.flip_side = false;
        state.flip_angle = 0.0;
    }
    pub(super) fn begin_flip(&mut self, side: bool) {
        let state = &mut self.skater.air_reckoning.state;
        if state.flip_active {
            return;
        }
        self.physics.riding.reckoning_frames.body_flip = IDENTITY;
        state.flip_speed = 0.0;
        state.flip_angle = 0.0;
        let mut basis = self.physics.riding.reckoning_frames.system;
        if self.toolkit.control_sign < 0.0 {
            basis[0] = scale(basis[0], -1.0);
            basis[2] = scale(basis[2], -1.0);
        }
        let v = self.skater.animation_input.fields.body_spin * 4.0;
        let lo = if -v >= 0.0 { 0.0 } else { v };
        let blend = if 1.0 - lo >= 0.0 { lo } else { 1.0 };
        let adjustment = scale(self.skater.known_air.flip_axis_adjustment, blend);
        let axis = std::array::from_fn(|i| {
            basis[2][i].mul_add(
                adjustment[2],
                basis[1][i].mul_add(adjustment[1], basis[0][i]),
            )
        });
        state.flip_axis = normalized(axis).0;
        if side {
            state.flip_axis = scale(state.flip_axis, -1.0);
        }
        state.flip_side = side;
        state.flip_active = true;
    }
}
