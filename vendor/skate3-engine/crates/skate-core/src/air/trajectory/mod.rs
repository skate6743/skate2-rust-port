//! TU3 trajectory selection82D67848/82D682E8/82D68800.
//! Host world storage and batch ownership are independent of console layout.
pub mod grind;
pub mod grind_surface;
mod launch;
mod math;
mod prediction;
mod query;
mod scoring;
mod selector;
mod types;
pub use prediction::{Prediction, QueryRequest, QueryResult, Trajectory};
pub use query::{SurfaceHit, query_trajectory};
pub use selector::{Selection, TrajectorySelector, WorldWithoutGrindEdges};
pub use types::{LaunchInfo, SelectorInput, SelectorSettings};
