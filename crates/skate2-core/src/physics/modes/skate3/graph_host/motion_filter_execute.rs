//! Live stock intent filtering shares the existing MotionAnimation maps.
use super::*;
impl MotionHost {
    pub(super) fn execute_intent_filter(
        &mut self,
        behavior: BehaviorId,
        operation: super::super::motion_intent_filter::Operation,
        frame: &Frame,
        phase: u8,
    ) -> Result<(), String> {
        let Instance::IntentFilter(state) = self
            .instances
            .get_mut(behavior)
            .ok_or("Unallocated intent-filter behavior")?
        else {
            return Err("Intent-filter operation/instance mismatch".into());
        };
        match phase {
            0 => operation.begin(state, &mut self.animation.filtered_intents),
            1 => {
                //82BB17D0 reads live ISkaterAnim v12/v28; earlier graph
                //operations can change these flags during the same traversal.
                let flags = self
                    .animation
                    .skater_animation_flags
                    .ok_or("FilterMotionGraphIntent requires live animation stance flags")?;
                operation.update(
                    state,
                    &self.animation.motion_intents,
                    &mut self.animation.filtered_intents,
                    frame.dt,
                    (flags & 0x2000_0000 != 0, flags & 0x4000_0000 != 0),
                );
            }
            _ => operation.end(&mut self.animation.filtered_intents),
        }
        Ok(())
    }
}
