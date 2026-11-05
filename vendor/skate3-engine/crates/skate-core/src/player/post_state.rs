//! TU3 `PhysicalPlayerHiLOD::PostState` thunk (`0x82DB62F8`).
//!
//! Raw instructions load Player+1796 and tail-call the current physical
//! state's vtable slot `+12`. The function has no other side effects.

/// Mandatory current-state dispatch performed by `0x82DB62F8`.
pub trait PostStateServices {
    fn update_current_state_vtable_12(&mut self);
}

/// Runs the complete PhysicalPlayer PostState phase.
pub fn run_post_state(services: &mut impl PostStateServices) {
    services.update_current_state_vtable_12();
}

#[cfg(test)]
#[path = "tests/post_state.rs"]
mod tests;
