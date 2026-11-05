//! Persistent ToggleBoard instance and shared animation-owner dispatch.
use super::*;
impl MotionHost {
    pub(super) fn execute_toggle_board(&mut self, behavior: BehaviorId, phase: u8) -> Result<(), String> {
        let Instance::ToggleBoard(state) = self.instances.get_mut(behavior)
            .ok_or("Unallocated ToggleBoard behavior")? else {
                return Err("ToggleBoard operation/instance mismatch".into());
            };
        let mirrored = self.animation.skater_animation_flags
            .map(|flags| flags & 0x4000_0000 != 0);
        //82BA9060/9080 reads Actor+20 virtual16, not raw AG input.
        //8259176C binds Actor+20 to IMotionGraph (8258F180); vtable
        //823006D0+16 ->8258F330 returns MG+12, its authored intention map.
        //The stock OffBoard AG state controls when these requests exist.
        let intents = &self.animation.motion_intents;
        let board = super::super::outputs::BoardControls {
            drop_requested: intents.contains_key("OB_DropBoard"),
            throw_requested: intents.contains_key("OB_ThrowBoard"),
            retrieve_requested: intents.contains_key("OB_RetrieveBoard"),
        };
        super::super::motion_toggle_board::execute(
            state, &mut self.animation, board,
            self.toggle_board_physical, mirrored, phase,
        )
    }
}
