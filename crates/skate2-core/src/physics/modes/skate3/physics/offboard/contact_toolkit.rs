//! Static canonical-world adapter for OffboardGroundAnalyzer.
//! The runtime must supply actual mesh metadata; neither a board contact nor
//! a synthetic ground ray can provide it.
#[cfg(any())]
mod tests;
mod world;
pub use skate3_core::player::offboard::contact_toolkit::Owner;
pub use world::StaticScene;
