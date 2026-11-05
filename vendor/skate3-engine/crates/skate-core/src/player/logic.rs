//! TU3 PhysicalPlayer Logic phase (`0x82DB5FA8`).

use super::{
    lifecycle::{
        PhysicalPlayerStateLifecycle, PhysicalStateCalls, SkateboardControllerActions,
        StateBinding, StateChangeData,
    },
    selector::{StateSelectionInput, StateSelector},
    state::UnknownPhysicalState,
};

/// Player+1876 staging data cleared at the end of every Logic phase.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsOutputStaging {
    /// Bytes +20 through +53.
    pub bytes_20_through_53: [u8; 34],
    /// Floats +56 through +188.
    pub scalars_56_through_188: [f32; 34],
    /// Word +200.
    pub word_200: u32,
}

impl PhysicsOutputStaging {
    pub fn clear_like_82db6000(&mut self) {
        self.word_200 = 0;
        self.bytes_20_through_53.fill(0);
        self.scalars_56_through_188.fill(0.0);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogicResult {
    pub previous: StateBinding,
    pub selected: StateBinding,
    pub changed: bool,
}

/// State owned by the exact transition portion of PhysicalPlayer's Logic phase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalPlayerLogic {
    pub selector: StateSelector,
    pub lifecycle: PhysicalPlayerStateLifecycle,
    /// PhysicalPlayer+1304; native writes 100 only when the concrete state changes.
    pub frames_since_jump_1304: u32,
}

impl PhysicalPlayerLogic {
    pub fn new(initial_state: super::state::PhysicalStateId) -> Self {
        Self {
            selector: StateSelector::default(),
            lifecycle: PhysicalPlayerStateLifecycle::new(initial_state),
            frames_since_jump_1304: 0,
        }
    }

    /// Calls current-state GetType, calculates the suggested state, performs
    /// SetPhysicsState only on an edge, then clears the output staging block.
    pub fn run<S, C>(
        &mut self,
        selection_input: &StateSelectionInput,
        state_change_data: &mut StateChangeData,
        output_staging: &mut PhysicsOutputStaging,
        states: &mut S,
        controller_actions: &mut C,
    ) -> Result<LogicResult, UnknownPhysicalState>
    where
        S: PhysicalStateCalls,
        C: SkateboardControllerActions,
    {
        let previous = self.lifecycle.active();
        let previous_type = states.get_type(previous);
        let suggested = self.selector.calculate(previous_type, selection_input);
        let changed = suggested != previous_type;
        let selected = if changed {
            self.frames_since_jump_1304 = 100;
            self.lifecycle.set_physics_state(
                suggested as u32,
                state_change_data,
                states,
                controller_actions,
            )?
        } else {
            previous
        };
        output_staging.clear_like_82db6000();
        Ok(LogicResult {
            previous,
            selected,
            changed,
        })
    }
}

#[cfg(test)]
#[path = "tests/logic.rs"]
mod tests;
