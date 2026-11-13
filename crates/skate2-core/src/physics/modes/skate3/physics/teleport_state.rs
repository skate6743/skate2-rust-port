//! Host checkpoint response to original actor reset callback82592518.
//! Shared scheduling preserves request -> next input reply ->702 output -> reset.
use skate3_core::{
    animation::output::actor_packet::ExternalReset,
    physics::skeleton_animation_record::AnimationPartTransform,
    player::{
        input_phase::{PhysicalPlayerInput, ProcessedPhysicsInput},
        teleport_state::{Output, Target, TeleportState, Update},
    },
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Checkpoint {
    pub transform: AnimationPartTransform,
    /// Original actor82592518 inverts checkpoint byte68. Its ground validator
    ///82BFBC18 sets that byte for surface category8. Authored rideable spawn
    ///surfaces supply true; an off-board checkpoint must explicitly supply false.
    pub on_board: bool,
}

pub(crate) struct Runtime {
    state: TeleportState,
    #[cfg(any())]
    checkpoint: Checkpoint,
    pending_reply: Option<Target>,
    manual_on_board: Option<bool>,
    /// An off-board target's dismount request (State77) has been published since the reset.
    dismount_window: bool,
}
impl Runtime {
    /// Explicit manual return uses the actor-reset publication without replacing
    /// the automatic recovery checkpoint owned by the respawn system.
    pub fn request_manual(&mut self, transform: AnimationPartTransform, on_board: bool) {
        self.pending_reply = Some(Target { transform: transform.map(|r| r.map(f32::to_bits)), on_board });
        self.manual_on_board = Some(on_board);
        self.dismount_window = false;
    }
    pub(crate) fn manual_on_board(&self) -> Option<bool> { self.manual_on_board }
    /// The next animation dispatch must observe Teleport before the reply is published.
    pub(crate) fn has_pending_reply(&self) -> bool { self.pending_reply.is_some() }
    /// `dismount_requested` is this publication's State77; `dismounting` whether the ActionGraph
    /// is in OnBoardGeneral/TeleportOffBoard.
    pub(crate) fn finish_manual_reset(
        &mut self,
        state: skate3_core::player::state::PhysicalStateId,
        dismount_requested: bool,
        dismounting: bool,
    ) {
        use skate3_core::player::state::PhysicalStateId;
        //The reset first enters100, including offboard requests. Keep the
        //target until the native graph has actually selected its board family.
        let target_reached = self.manual_on_board.is_none_or(|on_board|
            (state.category() == 500) != on_board);
        if state != PhysicalStateId::Teleporting && target_reached {
            self.manual_on_board = None;
            self.dismount_window = false;
        } else if state != PhysicalStateId::Teleporting && self.manual_on_board == Some(false) {
            // Natively an off-board target is only the reset's dismount request: State77 for the
            // three publications after state702's (ProcessOutput 82DB78A8..78DC, the PhysOut
            // template cleared each tick by 82DE53F0). The ActionGraph takes it through
            // OnGround -> TeleportOffBoard, and OnGround is inactive while ToggleOffBoardState is
            // held (Y), or off the ground. Once the request lapses without that dismount, the
            // native skater simply rides on: the host target goes with it, rather than keep
            // telling the graph and controls it is off board.
            if dismount_requested {
                self.dismount_window = true;
            } else if self.dismount_window && !dismounting {
                self.manual_on_board = None;
                self.dismount_window = false;
            }
        }
    }
    /// Keep the host's offboard target visible to animation until the reset
    /// continuation consumes it; state702 itself remains an ordinary teleport.
    pub fn new(#[cfg(any())] checkpoint: Checkpoint) -> Self {
        Self {
            state: TeleportState::default(),
            #[cfg(any())]
            checkpoint,
            pending_reply: None,
            manual_on_board: None,
            dismount_window: false,
        }
    }
    pub fn enter(&mut self) {
        self.state.enter();
    }
    /// Update82D431F0 calls the actor only without Processed2468 bit1.
    ///825926F8 stores the reply and marks it for the next input publication.
    pub fn update(&mut self, input: &ProcessedPhysicsInput) -> bool {
        self
            .state
            .update(input.flags_2468, input.matrix_1536, input.byte_1600)
            == Update::RequestCheckpoint
    }
    /// Host response for an independently verified actor-reset request.
    #[cfg(any())]
    pub fn request_checkpoint(&mut self) {
        self.reply(self.checkpoint);
    }
    pub fn reply(&mut self, checkpoint: Checkpoint) {
        self.pending_reply = Some(Target {
            transform: checkpoint.transform.map(|row| row.map(f32::to_bits)),
            on_board: checkpoint.on_board,
        });
    }
    /// Consume at the next actor-reset publication boundary. Publish this as
    ///ExternalReset plus Actor1904 bit29, then use the ordinary animation-packet
    ///mapper to produce1536/1600 and2468 bit1. Never reset physical bodies here.
    pub fn take_reply(&mut self) -> Option<ExternalReset> {
        self.pending_reply.take().map(|reply| ExternalReset {
            transform: reply.transform,
            byte64: u8::from(reply.on_board),
        })
    }
    /// Call after ordinary physical publication so702 owns State8/61 and
    ///the returned board0 transform/board272 publication for this tick.
    pub fn publish_output(&self, physical: &mut PhysicalPlayerInput) -> Option<Output> {
        let output = self.state.output()?;
        physical.teleport_output = Some(output);
        physical.state.identifier_8 = output.next_state;
        physical.state.flag_61 = output.state_61;
        Some(output)
    }
}

#[cfg(test)]
mod marker_reset_tests {
    use super::*;
    use skate3_core::{physics::skeleton_animation_record::IDENTITY,player::state::PhysicalStateId as State};
    #[test]
    fn offboard_target_survives_ground_reset_until_the_native_offboard_publication() {
        let mut runtime=Runtime::new();
        runtime.request_manual(IDENTITY,false);
        let reply=runtime.take_reply().unwrap();
        assert_eq!(reply.byte64,0);
        runtime.finish_manual_reset(State::Teleporting,false,false);
        runtime.finish_manual_reset(State::PhysicsGround,false,false);
        runtime.finish_manual_reset(State::PhysicsGround,true,false);
        assert_eq!(runtime.manual_on_board(),Some(false));
        runtime.finish_manual_reset(State::BipedGround,true,true);
        assert_eq!(runtime.manual_on_board(),None);
        assert!(runtime.take_reply().is_none(),"completion must not replay the external reset");
    }
    #[test]
    fn onboard_target_waits_for_the_actual_onboard_continuation() {
        let mut runtime=Runtime::new();runtime.request_manual(IDENTITY,true);
        runtime.finish_manual_reset(State::Teleporting,false,false);
        runtime.finish_manual_reset(State::BipedAir,false,false);
        assert_eq!(runtime.manual_on_board(),Some(true));
        runtime.finish_manual_reset(State::PhysicsGround,false,false);
        assert_eq!(runtime.manual_on_board(),None);
    }
    #[test]
    fn offboard_target_lapses_with_the_native_dismount_request() {
        // Y held through the reset: OnGround stays inactive, TeleportOffBoard never starts and
        // the skater rides on, as natively, without the host target left behind.
        let mut runtime=Runtime::new();runtime.request_manual(IDENTITY,false);
        runtime.take_reply();
        runtime.finish_manual_reset(State::Teleporting,false,false);
        for _ in 0..3 {
            runtime.finish_manual_reset(State::PhysicsGround,true,false);
            assert_eq!(runtime.manual_on_board(),Some(false));
        }
        runtime.finish_manual_reset(State::PhysicsGround,false,false);
        assert_eq!(runtime.manual_on_board(),None);
        assert!(runtime.take_reply().is_none(),"lapse must not replay the external reset");
    }
    #[test]
    fn a_dismount_taken_on_the_last_request_keeps_its_target() {
        let mut runtime=Runtime::new();runtime.request_manual(IDENTITY,false);
        runtime.finish_manual_reset(State::Teleporting,false,false);
        runtime.finish_manual_reset(State::PhysicsGround,true,false);
        runtime.finish_manual_reset(State::PhysicsGround,false,true);
        assert_eq!(runtime.manual_on_board(),Some(false));
        runtime.finish_manual_reset(State::BipedGround,false,true);
        assert_eq!(runtime.manual_on_board(),None);
    }
    #[test]
    fn the_target_waits_for_the_request_to_open() {
        // The first Ground publication can precede State77; nothing lapses before it opens.
        let mut runtime=Runtime::new();runtime.request_manual(IDENTITY,false);
        runtime.finish_manual_reset(State::Teleporting,false,false);
        runtime.finish_manual_reset(State::PhysicsGround,false,false);
        assert_eq!(runtime.manual_on_board(),Some(false));
    }
}
