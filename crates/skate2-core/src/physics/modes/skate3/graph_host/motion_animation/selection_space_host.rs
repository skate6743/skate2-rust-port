//! Deferred raw-tree callback in SelectionSpace::SetAttributes.
use super::*;

impl MotionAnimation {
    pub(super) fn prepare_selection_spaces(
        &mut self,
        tree: &mut PlaybackTree,
        attributes: &[SettableAttribute],
    ) -> Result<(), String> {
        match tree {
            PlaybackTree::SelectionSpace(space) => {
                space.select(attributes)?;
                //82D26F88 calls IAnimatable+76, not SkaterAnim::GetAnimTree.
                //TU3 vtable8231E050+76 ->82E32328 ->82D19648 constructs a
                //raw child. The enclosing main/channel tree already owns its
                //bind pose and mirroring; applying those here doubles bone
                //translations and rotations (visible on offboard jump clips).
                self.prepare_selection_spaces(space.current_mut().unwrap(), attributes)?;
            }
            PlaybackTree::BlendSpace(space) => {
                // Type6 SetAttributes82D24078 consumes its own parameters only.
                let _ = space;
            }
            PlaybackTree::PhaseBlend(space) => {
                for child in &mut space.children {
                    self.prepare_selection_spaces(child, attributes)?;
                }
            }
            PlaybackTree::Transition(transition) => {
                // PlaybackTransition::set_attributes follows this same to/from order.
                self.prepare_selection_spaces(&mut transition.to, attributes)?;
                if transition.settings.kind == 4 && !transition.complete() {
                    self.prepare_selection_spaces(&mut transition.from, attributes)?;
                }
            }
            PlaybackTree::BindPose { motion, .. } => {
                self.prepare_selection_spaces(motion, attributes)?
            }
            PlaybackTree::Clip { .. } => {}
        }
        Ok(())
    }
}
