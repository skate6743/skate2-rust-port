//! Rigid-body kernels and connected-board simulation, separate from collision
//! queries and player behavior.
pub mod assembly;
pub mod board;
pub mod board_animation;
pub mod board_dynamic_normal;
pub mod board_ground;
pub mod board_input_output;
pub mod board_motion_output;
pub mod board_pose;
pub mod board_runtime;
pub mod board_step;
pub mod board_toolkit;
pub mod board_world;
pub mod centre_of_mass_filter;
pub mod collision;
pub mod constraint_batch;
mod constraint_frames;
pub mod contact;
pub mod contact_feedback;
mod contact_layout;
pub mod contact_solver;
pub mod drive_frames;
pub mod drive_parameters;
pub mod drive_preparation;
pub mod drive_solver;
pub mod foot_physical_output;
pub mod force_queue;
pub mod grind_air;
pub mod grind_contact;
pub mod grind_forces;
pub mod hook_drive;
pub mod joint_builder;
pub mod joint_records;
pub mod joint_solver;
pub mod manual;
pub mod mass;
pub(crate) mod native_arithmetic;
pub mod phase;
pub mod point_force;
pub(crate) mod reciprocal_sqrt;
pub mod rigid_body;
pub mod solver;
pub mod startup;
mod triangle_closest;
pub mod triangle_query;
mod triangle_sweep;
mod triangle_sweep_roots;
mod triangle_sweep_walk;
pub mod truck_frames;
pub mod world_contact;

#[cfg(test)]
#[path = "tests/world_dispatch.rs"]
mod world_dispatch_tests;

pub mod board_probes;
pub mod deck_angular_correction;
pub mod filtered_state;
pub mod ground_hang_geometry;
#[cfg(test)]
#[path = "tests/primitive_query.rs"]
mod primitive_query_tests;
pub mod skateboard_controller;
pub mod skeleton_air_frames;
pub mod skeleton_animation_record;
pub mod skeleton_biped_air;
pub mod skeleton_board_frames;
pub mod skeleton_board_offset;
pub mod skeleton_body;
pub mod skeleton_general;
pub mod skeleton_landing;
pub mod skeleton_landing_on_board;
pub mod skeleton_motion;
pub mod skeleton_output;
pub mod skeleton_root;
