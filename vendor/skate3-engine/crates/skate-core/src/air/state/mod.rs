//! Readable TU3 `PhysState_PhysicsAir` lifecycle and update composition.
//!
//! The state owns the recovered branch/order semantics. Calls into the board,
//! skeleton, trajectory selector, wipeout controller, and unresolved Xenon
//! vector math remain mandatory runtime interfaces with no fallback.

mod board;
mod data;
mod jump_velocity;
mod lifecycle;
mod math;
mod post_physics;
mod runtime;
mod update;

pub use board::update_skateboard;
pub use data::{
    AirBoardForce, AirTrajectory, PhysicsAirFrame, PhysicsAirOutput, PhysicsAirReckoningFields,
    PhysicsAirSettings, PhysicsAirState,
};
pub use jump_velocity::{PhysicsAirMath, calculate_velocity_from_jump};
pub use lifecycle::{enter, exit, fill_physics_output};
pub use math::{AirMath, angle_between_vectors, clamp_jump_velocity};
pub use post_physics::update_post_physics;
pub use runtime::{PhysicsAirLaunchInfo, PhysicsAirRuntime};
pub use update::{integrate_trajectory_fixed_step, update, wrap_signed_angle};

#[cfg(test)]
mod tests;
