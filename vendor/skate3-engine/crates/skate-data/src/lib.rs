//! Runtime stock data, validation and bank storage. Stateless codec math is in core.
#![forbid(unsafe_code)]
pub mod asset_io;
mod manifest;
pub use manifest::{AssetError, GameAssets};
pub mod abin;
pub mod animation_banks;
pub mod animation_frames;
pub mod animation_metadata;
pub mod attrib_hash;
pub mod collections;
pub mod gesture_patterns;
pub mod input_config;
pub mod input_recording;
pub mod menu_tables;
pub mod ocean_pca;
pub mod physics_skeleton;
pub mod retail_collision;
pub mod scoring;
mod scoring_fields;
mod sha256;
pub mod skate_map;
pub mod state_graph;
pub mod trigger_volumes;
mod vault;
pub mod xex;
