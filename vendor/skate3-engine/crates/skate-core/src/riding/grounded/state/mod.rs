//! Readable composition of the TU3 `PhysState_PhysicsGround` lifecycle.
//!
//! This module owns native state order. Collision queries, skeleton writes and
//! other engine objects remain required interfaces; no branch has a no-op
//! implementation.

pub mod board;
pub mod board_types;
pub mod contact_state;
pub mod corrections;
pub mod data;
pub mod lifecycle;
pub mod motion;
mod ordinary;
pub mod output;
pub mod post_physics;
pub mod update;

pub use data::{GroundContactHistory, GroundUpdateInput, PhysicsGroundState};
