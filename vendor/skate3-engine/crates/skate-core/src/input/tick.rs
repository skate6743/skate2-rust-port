//! Immutable fixed-tick input boundary.
//!
//! The platform adapter owns device polling. Once a `TickInput` exists, all
//! simulation phases consume this value; they do not access a platform device,
//! pad history, or rebuild the gameplay map.

use super::gameplay_map::GameplayActions;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TickInput {
    tick: u64,
    actions: GameplayActions,
    controller_available: bool,
}

impl TickInput {
    pub const fn new(tick: u64, actions: GameplayActions, controller_available: bool) -> Self {
        Self {
            tick,
            actions,
            controller_available,
        }
    }

    pub const fn tick(self) -> u64 {
        self.tick
    }

    pub const fn actions(self) -> GameplayActions {
        self.actions
    }

    pub const fn controller_available(self) -> bool {
        self.controller_available
    }
}
