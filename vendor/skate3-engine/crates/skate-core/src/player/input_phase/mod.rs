//! Skate 3 TU3 `PhysicalPlayerHiLOD::Input` (`0x82DB4048`).
//!
//! The phase is split into direct field publication and an ordered runtime.
//! Separate native subsystems remain mandatory service calls; none have a
//! fallback implementation here.

mod publication;
mod special_surface;
pub use special_surface::publish_special_surface;
mod air_output;
mod grind_input;
mod grind_output;
mod motion_math;
mod pose_output;
mod requests;
mod runtime;
mod types;

pub use air_output::AirOutputFields;
pub use grind_input::GrindInvestigationFields;
pub use grind_output::GrindOutputFields;
pub use pose_output::{AnimationOutputFields, ScoringOutputFields, SkeletonOutputFields};
pub use requests::*;
pub use runtime::{
    InputContinuation, InputPhaseError, InputPhaseServices, finish_input, process_input,
    start_input,
};
pub use types::*;

#[cfg(test)]
mod tests;
