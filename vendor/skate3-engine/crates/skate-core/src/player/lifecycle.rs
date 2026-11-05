//! Physical-player state changes recovered from TU3 `0x82DB8540`.
//!
//! The selected state objects remain owned by `PhysicalPlayer`; changing state
//! selects one of those objects, rather than constructing a replacement.

use super::state::{PhysicalStateId, UnknownPhysicalState};

/// The state object selected from one of `PhysicalPlayer`'s owned pointers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateBinding {
    pub state: PhysicalStateId,
    pub owner_offset: u32,
}

impl StateBinding {
    pub const fn new(state: PhysicalStateId) -> Self {
        Self {
            state,
            owner_offset: state.native_owner_offset(),
        }
    }
}

/// State-change fields owned directly by `PhysicalPlayer`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerStateChangeFields {
    /// Player+1312. `SetPhysicsState` clears it before publishing state history.
    pub word_1312: u32,
    /// Player+1336. Updated to the previous category only across a category edge.
    pub previous_category_latch_1336: u32,
    /// Player+1344. `SetPhysicsState` clears it on every accepted transition.
    pub scalar_1344: f32,
}

/// The exact `ProcessedPhysIn` fields read or written by `0x82DB8540`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcessedStateChangeFields {
    /// +2480; bits 7 and 8 choose free-board controller mode on off-board entry.
    pub flags_2480: u32,
    /// +2500; requested state.
    pub requested_state_2500: u32,
    /// +2504; type reported by the old state's vtable slot +28.
    pub previous_state_2504: u32,
    /// +2508; current concrete state published before old-state Exit.
    pub current_state_2508: u32,
    /// +2512; category of `current_state_2508`.
    pub current_category_2512: u32,
    /// +2516; category of `previous_state_2504`.
    pub previous_category_2516: u32,
    /// +2520; copy of Player+1336 after its conditional update.
    pub previous_category_latch_2520: u32,
    /// +2564; reset before the old state's Exit call.
    pub word_2564: u32,
    /// +2664; reset before the old state's Exit call.
    pub scalar_2664: f32,
}

/// Fields at `PhysicalPlayer+1844`'s `SkateboardController`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkateboardControllerFields {
    /// Controller+444. Its wider meaning is unresolved; every mode change here
    /// resets it with a 32-bit zero.
    pub word_444: u32,
    /// Controller+448. Values used by this path are stopped=0, held=1, free=2.
    pub state_448: u32,
    /// Controller+452, identified by `StopController` as the system-on byte.
    pub system_on_452: bool,
}

/// Complete mutable data touched by the state-change function itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateChangeData {
    pub player: PlayerStateChangeFields,
    pub processed: ProcessedStateChangeFields,
    pub skateboard_controller: SkateboardControllerFields,
}

/// Board and drive operations called by the controller branch. Their complete
/// implementations are `HoldSkateboard` `0x82D75370` and
/// `LetGoOfSkateboard` `0x82D75440`; this lifecycle never replaces them.
pub trait SkateboardControllerActions {
    fn hold_skateboard(&mut self);
    fn let_go_of_skateboard(&mut self);
}

/// Context supplied to an owned state's Enter or Exit implementation.
/// `active` makes the pointer-switch boundary observable without exposing a
/// native pointer in engine-independent code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateCall {
    pub state: StateBinding,
    pub active: StateBinding,
}

/// Calls corresponding to the recovered state vtable slots used here.
pub trait PhysicalStateCalls {
    /// Vtable +28 on the currently selected state.
    fn get_type(&mut self, state: StateBinding) -> PhysicalStateId;
    /// Vtable +20 on the old state.
    fn exit(&mut self, call: StateCall);
    /// Vtable +16 on the newly selected state.
    fn enter(&mut self, call: StateCall);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalPlayerStateLifecycle {
    active: StateBinding,
}

impl PhysicalPlayerStateLifecycle {
    pub const fn new(initial_state: PhysicalStateId) -> Self {
        Self {
            active: StateBinding::new(initial_state),
        }
    }

    pub const fn active(&self) -> StateBinding {
        self.active
    }

    /// Port of `PhysicalPlayerHiLOD::SetPhysicsState` `0x82DB8540`.
    ///
    /// Unknown numeric IDs are rejected before any callbacks or field writes.
    /// The native default branches exit and re-enter the existing pointer for
    /// such values, but no recovered caller contract establishes them as valid
    /// states; making that case an error prevents a silent false transition.
    pub fn set_physics_state<S, C>(
        &mut self,
        requested_raw: u32,
        data: &mut StateChangeData,
        states: &mut S,
        controller_actions: &mut C,
    ) -> Result<StateBinding, UnknownPhysicalState>
    where
        S: PhysicalStateCalls,
        C: SkateboardControllerActions,
    {
        let requested = PhysicalStateId::try_from(requested_raw)?;

        prepare_skateboard_controller(
            requested,
            &mut data.skateboard_controller,
            data.processed.flags_2480,
            controller_actions,
        );

        data.player.word_1312 = 0;
        data.player.scalar_1344 = 0.0;
        data.processed.scalar_2664 = 0.0;
        data.processed.word_2564 = 0;

        let previous = states.get_type(self.active);
        let previous_category = previous.category();
        let current_category = requested.category();
        data.processed.previous_state_2504 = previous as u32;
        data.processed.previous_category_2516 = previous_category;
        data.processed.requested_state_2500 = requested as u32;
        data.processed.current_state_2508 = requested as u32;
        data.processed.current_category_2512 = current_category;
        if current_category != previous_category {
            data.player.previous_category_latch_1336 = previous_category;
        }
        data.processed.previous_category_latch_2520 = data.player.previous_category_latch_1336;

        states.exit(StateCall {
            state: self.active,
            active: self.active,
        });
        self.active = StateBinding::new(requested);
        states.enter(StateCall {
            state: self.active,
            active: self.active,
        });
        Ok(self.active)
    }
}

fn prepare_skateboard_controller<C: SkateboardControllerActions>(
    requested: PhysicalStateId,
    controller: &mut SkateboardControllerFields,
    processed_flags_2480: u32,
    actions: &mut C,
) {
    if !matches!(
        requested,
        PhysicalStateId::BipedGround | PhysicalStateId::BipedAir | PhysicalStateId::OffBoardPushing
    ) {
        stop_skateboard_controller(controller, actions);
        return;
    }

    if controller.system_on_452 {
        return;
    }

    controller.word_444 = 0;
    if processed_flags_2480 & (0x80 | 0x100) != 0 {
        if controller.state_448 != 2 {
            actions.let_go_of_skateboard();
            controller.state_448 = 2;
        }
    } else if controller.state_448 != 1 {
        actions.hold_skateboard();
        controller.state_448 = 1;
    }
    controller.system_on_452 = true;
}

/// `StopController` `0x82D75EA0`, called directly by `SetPhysicsState` for
/// every requested state outside 500..=502.
fn stop_skateboard_controller<C: SkateboardControllerActions>(
    controller: &mut SkateboardControllerFields,
    actions: &mut C,
) {
    if !controller.system_on_452 {
        return;
    }
    let previous_state = controller.state_448;
    controller.word_444 = 0;
    if previous_state != 0 {
        actions.let_go_of_skateboard();
        controller.state_448 = 0;
    }
    controller.system_on_452 = false;
}

#[cfg(test)]
#[path = "tests/lifecycle.rs"]
mod tests;
