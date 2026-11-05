//! TU3 `PhysicalPlayerHiLOD::State` wrapper (`0x82DB6120`).
//!
//! State bodies, skateboard-controller modes, the board force queue, and the
//! board's fixed-step cache are separate recovered systems. This module owns
//! their exact wrapper order and Player+1344 accumulation.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatePhaseFields {
    /// PhysicalPlayer+1344.
    pub elapsed_1344: f32,
    /// ProcessedPhysIn+2604.
    pub timestep_2604: f32,
    /// SkateboardController+448.
    pub controller_state_448: u32,
    /// SkateboardController+452.
    pub controller_system_on_452: bool,
}

/// Calls made by `0x82DB6120`. Implementors must connect each method to the
/// corresponding recovered system; no fallback behavior is defined here.
pub trait StatePhaseServices {
    /// Current-state vtable slot `+8`.
    fn update_current_state_vtable_8(&mut self);

    fn update_controller_82d75f00(&mut self);
    fn update_controller_mode_1_82d751d8(&mut self);
    fn update_controller_mode_2_82d750f0(&mut self);
    fn update_controller_mode_3_82d75bb8(&mut self);
    fn update_controller_mode_4_82d75d58(&mut self);

    /// First Player vtable `+116` lookup at `0x82DB61C4`.
    fn touch_skateboard_vtable_116(&mut self);
    /// Second Player vtable `+116` lookup followed by `0x82C03718`.
    fn apply_skateboard_force_queue_82c03718(&mut self);
    /// Third Player vtable `+116` lookup and the fixed-`1/60` VMX cache update
    /// at `0x82DB61F0..0x82DB62D0`.
    fn update_skateboard_fixed_step_cache_82db61f0(&mut self);
}

pub fn run_state_phase(fields: &mut StatePhaseFields, services: &mut impl StatePhaseServices) {
    services.update_current_state_vtable_8();

    if fields.controller_system_on_452 {
        services.update_controller_82d75f00();
        match fields.controller_state_448 {
            1 => services.update_controller_mode_1_82d751d8(),
            2 => services.update_controller_mode_2_82d750f0(),
            3 => services.update_controller_mode_3_82d75bb8(),
            4 => services.update_controller_mode_4_82d75d58(),
            _ => {}
        }
    }

    services.touch_skateboard_vtable_116();
    services.apply_skateboard_force_queue_82c03718();
    services.update_skateboard_fixed_step_cache_82db61f0();
    fields.elapsed_1344 += fields.timestep_2604;
}

#[cfg(test)]
#[path = "tests/state_phase.rs"]
mod tests;
