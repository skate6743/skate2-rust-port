//! Compile as a child of motion_animation, NOT motion_grind, so the real
//! owner's private current tree stays private. See the module line in handoff.
use super::MotionAnimation;
use crate::skate3_native::graph_host::motion_grind::animation::Animation;
use skate3_core::animation::output::attributes::{AnimationAttribute, AttributeName};

impl Animation for MotionAnimation {
    fn apply_parameters(&mut self) -> Result<(), String> {
        MotionAnimation::apply_parameters(self)
    }

    fn query_current_attribute(
        &self,
        name: AttributeName,
        mask: u32,
        output: &mut AnimationAttribute,
    ) -> Result<bool, String> {
        // S3 virtual48 ->825310E0 returns the current root at owner+13524.
        // Query that root, retaining transition routing and partial results.
        self.current
            .as_ref()
            .ok_or("GrindControlFade requires a current animation tree")?
            .query_attribute(name, mask, output)
    }

    fn emit_packet(&mut self, name: AttributeName, value: f32) {
        MotionAnimation::emit_packet(self, name, value);
    }

    fn intent(&self, name: &str) -> Option<f32> {
        self.motion_intents.get(name).copied()
    }
}

#[cfg(any())]
#[path = "animation_owner_tests.rs"]
mod tests;
