//! Complete Skeleton::UpdateSkateboardOffsetTransform82BDD630 and
//! SkeletonIK::AddOffsetTransform82BF1C68. Counters are animation updates.
use super::{
    board_motion_output::inverse_length_squared,
    native_arithmetic::dot3,
    skeleton_animation_record::{AnimationPartTransform, IDENTITY, compose_affine},
};

#[derive(Clone, Debug)]
pub struct SkateboardOffset {
    pub transform: AnimationPartTransform,
    pub orientation_frames: f32,
    pub height_frames: f32,
    pub orientation_refreshed: bool,
    pub height_refreshed: bool,
}
impl Default for SkateboardOffset {
    fn default() -> Self {
        // Skeleton constructor82BD76E8..7708 and82BD79A0..79BC.
        Self {
            transform: IDENTITY,
            orientation_frames: 0.0,
            height_frames: 0.0,
            orientation_refreshed: false,
            height_refreshed: false,
        }
    }
}
impl SkateboardOffset {
    /// The offboard, landing-on-board and grind-air producers refresh both
    /// channels for15 updates after writing their independently calculated frame.
    pub fn refresh_transform(&mut self, transform: AnimationPartTransform) {
        self.transform = transform;
        self.orientation_frames = 15.0;
        self.height_frames = 15.0;
        self.orientation_refreshed = true;
        self.height_refreshed = true;
    }

    /// LandingAdjust writes only Y and its own duration, preserving X/Z/basis.
    pub fn refresh_height(&mut self, height: f32, frames: f32) {
        self.transform[3][1] = height;
        self.height_frames = frames;
        self.height_refreshed = true;
    }

    /// Update decay first, then apply the resulting offset to the board and
    /// four IK targets. Positive counters select application before decrement.
    pub fn update(
        &mut self,
        board_pose: &mut AnimationPartTransform,
        targets: &mut [AnimationPartTransform; 4],
    ) {
        let mut apply = false;
        if self.orientation_frames > 0.0 {
            apply = true;
            if !self.orientation_refreshed {
                let ratio = (self.orientation_frames - 1.0) / self.orientation_frames;
                let weight = ratio * ratio;
                let identity_weight = 1.0 - weight;
                for (axis, identity) in IDENTITY.iter().enumerate().take(3) {
                    for (lane, value) in self.transform[axis].iter_mut().enumerate() {
                        *value = value.mul_add(weight, identity[lane] * identity_weight);
                    }
                }
                // vrlimi mask4 retains Y. X/Z/W belong to orientation decay.
                for lane in [0, 2, 3] {
                    self.transform[3][lane] *= weight;
                }
                let unnormalized = self.transform;
                // Native Gram-Schmidt traverses At,Up,Ri in that order.
                for axis in (0..3).rev() {
                    let mut vector = unnormalized[axis];
                    for prior in ((axis + 1)..3).rev() {
                        let projection = dot3(self.transform[prior], unnormalized[axis]);
                        for (lane, value) in vector.iter_mut().enumerate() {
                            *value -= self.transform[prior][lane] * projection;
                        }
                    }
                    let reciprocal = inverse_length_squared(dot3(vector, vector), 2);
                    self.transform[axis] = vector.map(|v| v * reciprocal);
                }
                self.orientation_frames -= 1.0;
            }
        }
        if self.height_frames > 0.0 {
            apply = true;
            if !self.height_refreshed {
                let ratio = (self.height_frames - 1.0) / self.height_frames;
                self.transform[3][1] *= ratio * ratio;
                self.height_frames -= 1.0;
            }
        }
        if apply {
            *board_pose = compose_affine(&self.transform, board_pose);
            for target in targets {
                *target = compose_affine(&self.transform, target);
            }
        }
        self.orientation_refreshed = false;
        self.height_refreshed = false;
    }
}

#[cfg(test)]
#[path = "tests/skeleton_board_offset.rs"]
mod tests;
