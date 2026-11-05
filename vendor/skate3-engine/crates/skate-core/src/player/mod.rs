//! Playable-skater orchestration recovered from the TU3 player and world loops.
//!
//! State behavior lives in the riding, air, animation and physics modules. This
//! module owns the one ordering contract that connects those systems.

pub mod adjust;
pub mod frame;
pub mod input_phase;
pub mod lifecycle;
pub mod logic;
pub mod offboard;
pub mod post_input;
pub mod post_output;
pub mod post_state;
pub mod pre_state;
pub mod selector;
pub mod slide_state;
pub mod state;
pub mod state_phase;
pub mod wipeout;
pub mod wipeout_state;

pub mod conditioner_capabilities;
pub mod respawn;
pub mod teleport_state;
