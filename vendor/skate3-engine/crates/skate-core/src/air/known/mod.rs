//! Readable TU3 `PhysState_KnownAir` (state 201).
//!
//! The routines in this module preserve the observed Skate 3 call, branch,
//! field, and constant order. Engine-owned objects and Xenon-specific vector
//! operations are mandatory adapter contracts; this module supplies no
//! guessed fallbacks.

mod body;
mod data;
mod footplant;
mod lifecycle;
mod output;
mod post_physics;
mod runtime;
mod trajectory;
pub use trajectory::perfect_body_flip_target;
mod update;

pub use body::{calculate_body_flip_speed, calculate_body_spin_speed};
pub use data::{
    AffineTransform, GrindFootBody, KnownAirFootplantInput, KnownAirFrame, KnownAirModeSettings,
    KnownAirOutput, KnownAirPrediction, KnownAirReckoningFields, KnownAirSettings, KnownAirState,
    KnownAirTrajectory, KnownAirWipeoutRequest, KnownAirWipeoutSettings, RestoreVelocityGeometry,
    Vector4,
};
pub use lifecycle::{enter, exit};
pub use output::fill_physics_output;
pub use post_physics::{check_upside_down_falling_wipeout, update_post_physics};
pub use runtime::{KnownAirMath, KnownAirRuntime};
pub use trajectory::{init_trajectory_info, restore_velocity, update_trajectory_follow};
pub use update::update;

#[cfg(test)]
mod tests;
