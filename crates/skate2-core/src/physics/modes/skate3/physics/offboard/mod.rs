//! Off-board game adapters over the shared physical and scene owners.
//!
//! These modules are the source-backed implementation units for the off-board
//! owner. The coordinator calls them through the selected physical state.
#![allow(dead_code)]
pub(crate) mod air_feet;
pub(crate) mod air_selector;
pub(crate) mod board_manager;
pub(crate) mod board_possession;
pub(crate) mod contact_toolkit;
pub(crate) mod grab_scene;
pub(crate) mod ground_geometry;
pub(crate) mod ground_query;
pub(crate) mod ground_sync;
pub(crate) mod mod_solid_ground;
pub(crate) mod landing_deck;
pub(crate) mod pose_adjust;
pub(crate) mod post_physics;
pub(crate) mod settings;
pub(crate) mod skeleton_air;
pub(crate) mod skeleton_ground;

pub(crate) mod contact_queries;
