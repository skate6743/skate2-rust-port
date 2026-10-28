//! Platform-neutral contracts for reconstructed Skate 2 systems.
//!
//! Modules are added only when their behavior has an evidence record. This crate
//! intentionally contains no emulated CPU or console kernel.

pub mod ui;
#[path = "online/online.rs"]
pub mod online;
pub(crate) mod ai;
pub mod challenge;
#[path = "dlc/dlc.rs"]
pub mod dlc;
pub mod nis;
#[path = "session/edit_skater_profile.rs"]
pub mod edit_skater_profile;
#[path = "session/edit_skater_character.rs"]
pub mod edit_skater_character;
#[path = "session/edit_skater_record.rs"]
mod edit_skater_record;
#[path = "session/edit_skater_frontend.rs"]
mod edit_skater_frontend;
#[path = "session/edit_skater_shop.rs"]
mod edit_skater_shop;
#[path = "session/edit_skater_session.rs"]
mod edit_skater_session;
#[path = "rendering/character_morph.rs"]
mod character_morph;
#[path = "living_world/entities/living_world_execution_sort.rs"]
pub(crate) mod native_execution_sort;

#[macro_use]
#[path = "diagnostics/diagnostics.rs"]
pub mod diagnostics;

pub mod host_thread;

#[path = "input/action_graph_input_listener.rs"]
mod action_graph_input_listener;
#[path = "animation/runtime/actor_animation_construction.rs"]
mod actor_animation_construction;
#[cfg(test)]
#[path = "animation/runtime/actor_animation_mount_corpus.rs"]
mod actor_animation_mount_corpus;
#[path = "animation/trees/actor_animation_tree.rs"]
mod actor_animation_tree;
#[path = "actors/actor_construction.rs"]
mod actor_construction;
#[path = "animation/face/actor_face_playback.rs"]
mod actor_face_playback;
#[path = "animation/graphs/actor_graph_construction.rs"]
mod actor_graph_construction;
#[path = "animation/animation.rs"]
pub mod animation;
#[path = "living_world/vehicles/vehicle_animation.rs"]
pub mod vehicle_animation;
#[path = "animation/jobs/animation_job_owners.rs"]
mod animation_job_owners;
#[path = "streaming/arena_registration_ref_counter.rs"]
pub mod arena_registration_ref_counter;
#[path = "streaming/asset_activation_manager.rs"]
pub mod asset_activation_manager;
#[path = "assets/attributes/attribute_runtime.rs"]
mod attribute_runtime;
#[cfg(test)]
#[path = "assets/attributes/attribute_runtime_corpus.rs"]
mod attribute_runtime_corpus;
#[path = "physics/assembly/authored_collision_publication.rs"]
mod authored_collision_publication;
#[path = "animation/graphs/behaviour_graph_runtime.rs"]
pub mod behaviour_graph_runtime;
#[path = "streaming/catalog_resident_load_factory.rs"]
pub mod catalog_resident_load_factory;
#[path = "assets/character.rs"]
pub mod character;
#[path = "animation/face/character_face_manager.rs"]
mod character_face_manager;
#[path = "streaming/collection_load_pump.rs"]
pub mod collection_load_pump;
#[path = "streaming/collection_parent_manager.rs"]
pub mod collection_parent_manager;
#[path = "streaming/collection_resident_streamer.rs"]
pub mod collection_resident_streamer;
#[path = "streaming/collection_stream_scheduler.rs"]
pub mod collection_stream_scheduler;
#[path = "physics/collision/collision_line_collection.rs"]
mod collision_line_collection;
#[path = "physics/collision/collision_line_numeric.rs"]
mod collision_line_numeric;
#[path = "physics/collision/collision_mesh_line.rs"]
mod collision_mesh_line;
#[path = "physics/collision/collision_query_batches.rs"]
mod collision_query_batches;
#[path = "physics/collision/collision_triangle_line.rs"]
mod collision_triangle_line;
#[path = "input/devices/controller_input.rs"]
pub mod controller_input;
#[path = "diagnostics/debug_overlay.rs"]
mod debug_overlay;
#[path = "assets/decoded_arena_asset.rs"]
pub mod decoded_arena_asset;
#[path = "streaming/deferred_collection_reg_manager.rs"]
pub mod deferred_collection_reg_manager;
#[path = "streaming/dmo_bank_processed_asset.rs"]
pub mod dmo_bank_processed_asset;
#[path = "streaming/dmo_hull_acknowledgement.rs"]
pub mod dmo_hull_acknowledgement;
#[path = "physics/assembly/runtime/dynamic_assembly_session.rs"]
pub mod dynamic_assembly_session;
#[path = "streaming/dynamic_clustered_mesh_template.rs"]
pub mod dynamic_clustered_mesh_template;
#[path = "animation/graphs/dynamic_state_graph_controller.rs"]
pub mod dynamic_state_graph_controller;
#[path = "diagnostics/evidence.rs"]
pub mod evidence;
#[path = "session/game.rs"]
pub mod game;
#[path = "input/game_input_command.rs"]
mod game_input_command;
#[path = "input/game_input_frame_session.rs"]
mod game_input_frame_session;
#[path = "input/game_input_manager.rs"]
mod game_input_manager;
#[cfg(test)]
#[path = "input/game_input_original_corpus.rs"]
mod game_input_original_corpus;
#[path = "input/game_input_player.rs"]
mod game_input_player;
#[cfg(test)]
#[path = "input/recognition/game_input_recognizer_corpus.rs"]
mod game_input_recognizer_corpus;
#[path = "input/recognition/game_input_recognizer_runtime_services.rs"]
mod game_input_recognizer_runtime_services;
#[path = "streaming/game_stream.rs"]
pub mod game_stream;
#[path = "input/gameplay_input_update_manager.rs"]
mod gameplay_input_update_manager;
#[path = "animation/graphs/action/generated_action_load.rs"]
pub(crate) mod generated_action_load;
#[path = "animation/graphs/motion/generated_motion_load.rs"]
mod generated_motion_load;
#[path = "animation/graphs/action/generated_onboard_state_graph.rs"]
pub mod generated_onboard_state_graph;
#[path = "input/recognition/gesture_trick_mapping.rs"]
mod gesture_trick_mapping;
#[path = "streaming/global_location_activation.rs"]
pub mod global_location_activation;
#[path = "assets/graph_asset_cache.rs"]
pub mod graph_asset_cache;
#[path = "physics/grind.rs"]
pub mod grind;
#[path = "physics/grind_data_octree.rs"]
mod grind_data_octree;
#[path = "physics/grind_data_runtime.rs"]
mod grind_data_runtime;
#[path = "input/tuning/input_cvar_runtime.rs"]
mod input_cvar_runtime;
#[path = "input/input_map.rs"]
mod input_map;
#[path = "input/devices/input_pad.rs"]
mod input_pad;
#[path = "input/devices/input_pad_cache.rs"]
mod input_pad_cache;
#[path = "input/stick_settings.rs"]
pub mod stick_settings;
#[path = "input/recognition/input_pattern.rs"]
mod input_pattern;
#[path = "input/recognition/input_pattern_matcher.rs"]
mod input_pattern_matcher;
#[path = "input/recognition/juice_trick_input.rs"]
mod juice_trick_input;
#[path = "living_world/interaction/living_world_grind_services.rs"]
mod living_world_grind_services;
#[path = "living_world/pedestrians/living_world_pedestrian_memory.rs"]
mod living_world_pedestrian_memory;
#[path = "living_world/pedestrians/living_world_pedestrian_resources.rs"]
mod living_world_pedestrian_resources;
#[path = "living_world/pedestrians/living_world_pedestrian_entity.rs"]
mod living_world_pedestrian_entity;
#[path = "living_world/pedestrians/pedestrian_ground_batches.rs"]
mod pedestrian_ground_batches;
#[path = "living_world/pedestrians/pedestrian_execution_queries.rs"]
mod pedestrian_execution_queries;
#[path = "living_world/pedestrians/pedestrian_ground_dmo_batches.rs"]
mod pedestrian_ground_dmo_batches;
#[path = "animation/graphs/pedestrian_motion_delegate.rs"]
mod pedestrian_motion_delegate;
#[path = "living_world/pedestrians/pedestrian_volume_batches.rs"]
mod pedestrian_volume_batches;
#[path = "living_world/pedestrians/pedestrian_collision.rs"]
mod pedestrian_collision;
#[path = "living_world/pedestrians/pedestrian_adjust_owners.rs"]
mod pedestrian_adjust_owners;
#[path = "physics/collision/collision_volume_line.rs"]
mod collision_volume_line;
#[path = "physics/collision/collision_sphere_line.rs"]
mod collision_sphere_line;
#[path = "physics/collision/collision_box_line.rs"]
mod collision_box_line;
#[path = "living_world/pedestrians/living_world_pedestrian_state.rs"]
mod living_world_pedestrian_state;
#[path = "living_world/pedestrians/pedestrian_mood.rs"]
mod pedestrian_mood;
#[path = "living_world/pedestrians/pedestrian_graph_data.rs"]
mod pedestrian_graph_data;
#[path = "living_world/pedestrians/pedestrian_graph.rs"]
mod pedestrian_graph;
#[path = "animation/graphs/generated_graph_selection.rs"]
mod generated_graph_selection;
#[path = "living_world/pedestrians/pedestrian_navigation.rs"]
mod pedestrian_navigation;
#[path = "living_world/pedestrians/pedestrian_navigation_modifiers.rs"]
mod pedestrian_navigation_modifiers;
#[path = "living_world/pedestrians/pedestrian_animation.rs"]
mod pedestrian_animation;
#[path = "living_world/pedestrians/pedestrian_intent_packet.rs"]
mod pedestrian_intent_packet;
#[path = "session/living_world/selected_living_world_pedestrian_visual.rs"]
mod selected_living_world_pedestrian_visual;
#[path = "living_world/security/living_world_security_guard.rs"]
mod living_world_security_guard;
#[path = "living_world/security/living_world_guard_graph.rs"]
mod living_world_guard_graph;
#[path = "living_world/security/living_world_chase.rs"]
mod living_world_chase;
#[path = "living_world/assets/living_world_mesh_store.rs"]
mod living_world_mesh_store;
#[path = "living_world/navigation/living_world_rain.rs"]
mod living_world_rain;
#[path = "living_world/population/living_world_recipe_manager.rs"]
pub mod living_world_recipe_manager;
#[path = "living_world/assets/living_world_state_graph_preload.rs"]
pub mod living_world_state_graph_preload;
#[path = "streaming/location_arena_adapter.rs"]
pub mod location_arena_adapter;
#[path = "streaming/location_arena_plan.rs"]
pub mod location_arena_plan;
#[path = "animation/graphs/motion_graph_authority_session.rs"]
pub mod motion_graph_authority_session;
#[path = "animation/graphs/motion_graph_capability_preflight.rs"]
pub mod motion_graph_capability_preflight;
#[path = "animation/graphs/motion_graph_conditions.rs"]
pub mod motion_graph_conditions;
#[path = "animation/graphs/motion_graph_dynamic_backend.rs"]
pub mod motion_graph_dynamic_backend;
#[path = "animation/graphs/motion_graph_dynamic_session.rs"]
pub mod motion_graph_dynamic_session;
#[path = "animation/graphs/motion_graph_entity_authority.rs"]
pub mod motion_graph_entity_authority;
#[path = "animation/graphs/motion_graph_leaf_factory.rs"]
pub mod motion_graph_leaf_factory;
#[path = "input/motion_kickturn_steering.rs"]
pub(crate) mod motion_kickturn_steering;
#[path = "rendering/mounted_character_scene.rs"]
pub mod mounted_character_scene;
#[path = "streaming/mounted_collection_producer.rs"]
pub mod mounted_collection_producer;
#[path = "streaming/stream_reads.rs"]
pub(crate) mod stream_reads;
#[path = "streaming/mounted_stream_backend.rs"]
pub mod mounted_stream_backend;
#[path = "physics/collision/narrow_phase.rs"]
pub mod narrow_phase;
#[path = "math/native_skater_random.rs"]
mod native_skater_random;
#[path = "world/octree.rs"]
pub mod octree;
#[path = "streaming/owned_resident_collection.rs"]
pub mod owned_resident_collection;
#[path = "physics/player/air/physical_player_air_state.rs"]
mod physical_player_air_state;
#[path = "physics/player/output/physical_player_anim_out.rs"]
mod physical_player_anim_out;
#[path = "physics/player/lifecycle/physical_player_assembly.rs"]
mod physical_player_assembly;
#[path = "physics/player/footplant/physical_player_footplant.rs"]
mod physical_player_footplant;
#[path = "physics/player/interaction/physical_player_grab_spline.rs"]
mod physical_player_grab_spline;
#[path = "physics/player/grind/physical_player_grind_air_adjust.rs"]
mod physical_player_grind_air_adjust;
#[path = "physics/player/grind/physical_player_grind_manager.rs"]
mod physical_player_grind_manager;
#[path = "physics/player/ground/physical_player_ground_wipeout.rs"]
mod physical_player_ground_wipeout;
#[path = "physics/player/handplant/physical_player_handplant.rs"]
mod physical_player_handplant;
#[path = "physics/player/offboard/physical_player_landing_on_deck_math.rs"]
mod physical_player_landing_on_deck_math;
#[path = "physics/player/output/physical_player_phys_out.rs"]
mod physical_player_phys_out;
#[path = "physics/player/lifecycle/physical_player_physics_state.rs"]
mod physical_player_physics_state;
#[path = "physics/player/lifecycle/physics_state_ids.rs"]
mod physics_state_ids;
#[path = "physics/player/reckoning/physical_player_prepare_velocity.rs"]
mod physical_player_prepare_velocity;
#[path = "physics/player/board/physical_player_pumping.rs"]
mod physical_player_pumping;
#[path = "physics/player/reckoning/physical_player_reckoning_owner.rs"]
mod physical_player_reckoning_owner;
#[path = "physics/player/lifecycle/physical_player_remaining_constructor_owners.rs"]
mod physical_player_remaining_constructor_owners;
#[path = "physics/player/lifecycle/physical_player_state_changer.rs"]
mod physical_player_state_changer;
#[path = "physics/player/prediction/physical_player_trajectory_owners.rs"]
mod physical_player_trajectory_owners;
#[path = "physics/player/wipeout/physical_player_wipeout.rs"]
mod physical_player_wipeout;
#[path = "physics/player/wipeout/physical_player_wipeout_state.rs"]
mod physical_player_wipeout_state;
#[path = "physics/physics.rs"]
pub mod physics;
#[path = "animation/playback/play_animation.rs"]
pub mod play_animation;
#[path = "animation/playback/play_animation_lifecycle.rs"]
pub mod play_animation_lifecycle;
#[path = "animation/playback/play_remapped_animation.rs"]
pub mod play_remapped_animation;
#[path = "session/player.rs"]
pub mod player;
#[path = "session/players_backend_settings.rs"]
pub(crate) mod players_backend_settings;
#[path = "rendering/presentation.rs"]
pub mod presentation;
#[path = "rendering/presentation_shadow_culling.rs"]
pub mod presentation_shadow_culling;
#[path = "rendering/presentation_scene_culling.rs"]
pub mod presentation_scene_culling;
#[path = "rendering/presentation_anti_frustums.rs"]
pub mod presentation_anti_frustums;
#[path = "rendering/world_particles.rs"]
pub mod world_particles;
#[path = "rendering/presentation_restore.rs"]
pub mod presentation_restore;
#[path = "rendering/presentation_dmo_arena_adapter.rs"]
pub mod presentation_dmo_arena_adapter;
#[path = "rendering/presentation_dmo_deferred_producer.rs"]
pub mod presentation_dmo_deferred_producer;
#[path = "streaming/proxy_world_manager.rs"]
pub mod proxy_world_manager;
#[path = "physics/ragdoll.rs"]
pub mod ragdoll;
#[path = "physics/ragdoll_dynamics.rs"]
pub mod ragdoll_dynamics;
#[path = "physics/reckoning/reckoning.rs"]
mod reckoning;
#[path = "physics/reckoning/reckoning_attributes.rs"]
mod reckoning_attributes;
#[path = "physics/reckoning/reckoning_body_spin.rs"]
mod reckoning_body_spin;
#[path = "physics/reckoning/reckoning_numeric.rs"]
mod reckoning_numeric;
#[path = "physics/reckoning/reckoning_pid.rs"]
mod reckoning_pid;
#[path = "physics/reckoning/reckoning_sin_cos.rs"]
mod reckoning_sin_cos;
#[path = "rendering/render.rs"]
pub mod render;
#[path = "rendering/skater/skater_ropa_solver.rs"]
pub(crate) mod skater_ropa_solver;
#[path = "streaming/resident_collection_load_driver.rs"]
pub mod resident_collection_load_driver;
#[path = "diagnostics/scaffold.rs"]
pub mod scaffold;
#[path = "session/selected_sim_command_queue.rs"]
mod selected_sim_command_queue;
#[path = "physics/assembly/serialised_assembly_bbox.rs"]
mod serialised_assembly_bbox;
#[path = "physics/assembly/serialised_assembly_create.rs"]
mod serialised_assembly_create;
#[path = "physics/assembly/serialised_assembly_template.rs"]
mod serialised_assembly_template;
#[path = "session/session_marker.rs"]
mod session_marker;
#[path = "animation/jobs/simulation_animation_startup.rs"]
pub(crate) mod simulation_animation_startup;
#[path = "physics/assembly/simulation_assembly_backend.rs"]
pub mod simulation_assembly_backend;
#[path = "physics/collision/simulation_assembly_ground_run.rs"]
mod simulation_assembly_ground_run;
#[path = "physics/collision/simulation_assembly_ground_triangle_cache.rs"]
mod simulation_assembly_ground_triangle_cache;
#[path = "physics/collision/simulation_assembly_ground_triangle_rip.rs"]
mod simulation_assembly_ground_triangle_rip;
#[path = "physics/collision/simulation_assembly_ground_triangle_topology.rs"]
mod simulation_assembly_ground_triangle_topology;
#[path = "physics/assembly/simulation_assembly_payload.rs"]
pub mod simulation_assembly_payload;
#[path = "streaming/simulation/simulation_collection_adapter.rs"]
pub mod simulation_collection_adapter;
#[path = "streaming/simulation/simulation_collection_admission.rs"]
pub mod simulation_collection_admission;
#[path = "streaming/simulation/simulation_collection_notification_router.rs"]
pub mod simulation_collection_notification_router;
#[path = "streaming/simulation/simulation_collision_asset.rs"]
pub mod simulation_collision_asset;
#[path = "physics/solver/simulation_constraint_math.rs"]
mod simulation_constraint_math;
#[path = "physics/solver/contacts/simulation_contact_append.rs"]
mod simulation_contact_append;
#[path = "physics/solver/contacts/simulation_contact_jacobian.rs"]
pub mod simulation_contact_jacobian;
#[path = "physics/solver/contacts/simulation_contact_network_queue.rs"]
mod simulation_contact_network_queue;
#[path = "streaming/simulation/simulation_deferred_batch_policy.rs"]
pub mod simulation_deferred_batch_policy;
#[path = "streaming/simulation/simulation_deferred_message_fifo.rs"]
pub mod simulation_deferred_message_fifo;
#[path = "streaming/simulation/simulation_dmo_arena_adapter.rs"]
pub mod simulation_dmo_arena_adapter;
#[path = "streaming/simulation/simulation_dmo_arena_plan.rs"]
pub mod simulation_dmo_arena_plan;
#[path = "physics/assembly/simulation_dmo_assembly_part_session.rs"]
mod simulation_dmo_assembly_part_session;
#[path = "physics/board/control/skateboard_ai_path_following.rs"]
mod skateboard_ai_path_following;
#[path = "physics/board/body/skateboard_assembly.rs"]
mod skateboard_assembly;
#[path = "physics/board/runtime/skateboard_attributes.rs"]
mod skateboard_attributes;
#[path = "physics/board/body/skateboard_body_construction.rs"]
mod skateboard_body_construction;
#[path = "physics/board/body/skateboard_body_mass.rs"]
mod skateboard_body_mass;
#[path = "physics/board/forces/skateboard_brake_attributes.rs"]
mod skateboard_brake_attributes;
#[path = "physics/board/forces/skateboard_brake_drag.rs"]
mod skateboard_brake_drag;
#[path = "physics/board/runtime/skateboard_calculate_ground_position.rs"]
mod skateboard_calculate_ground_position;
#[path = "physics/board/collision/skateboard_contact_surface.rs"]
mod skateboard_contact_surface;
#[path = "physics/board/control/skateboard_controller.rs"]
mod skateboard_controller;
#[path = "physics/board/body/skateboard_deck.rs"]
mod skateboard_deck;
#[cfg(test)]
#[path = "physics/board/body/skateboard_definition_corpus.rs"]
mod skateboard_definition_corpus;
#[path = "physics/board/forces/skateboard_foot_attributes.rs"]
mod skateboard_foot_attributes;
#[path = "physics/board/forces/skateboard_foot_forces.rs"]
mod skateboard_foot_forces;
#[path = "physics/board/forces/skateboard_force_attributes.rs"]
mod skateboard_force_attributes;
#[path = "physics/board/forces/skateboard_force_queue.rs"]
mod skateboard_force_queue;
#[path = "physics/board/forces/skateboard_force_runtime.rs"]
mod skateboard_force_runtime;
#[path = "physics/board/runtime/skateboard_ground_force_tail.rs"]
mod skateboard_ground_force_tail;
#[path = "physics/board/body/skateboard_hook.rs"]
mod skateboard_hook;
#[path = "physics/board/body/skateboard_initial_transforms.rs"]
mod skateboard_initial_transforms;
#[path = "physics/board/body/skateboard_joints.rs"]
mod skateboard_joints;
#[path = "physics/board/forces/skateboard_linear_drag.rs"]
mod skateboard_linear_drag;
#[path = "physics/board/runtime/skateboard_processed_surface.rs"]
mod skateboard_processed_surface;
#[path = "physics/board/runtime/skateboard_set_standard.rs"]
mod skateboard_set_standard;
#[path = "physics/board/runtime/skateboard_set_velocity.rs"]
mod skateboard_set_velocity;
#[path = "physics/board/collision/skateboard_slide_collision.rs"]
mod skateboard_slide_collision;
#[path = "physics/board/collision/skateboard_slide_collision_attributes.rs"]
mod skateboard_slide_collision_attributes;
#[path = "physics/board/body/skateboard_spawn.rs"]
mod skateboard_spawn;
#[path = "physics/board/body/skateboard_spawn_math.rs"]
mod skateboard_spawn_math;
#[path = "physics/board/control/skateboard_steering.rs"]
mod skateboard_steering;
#[path = "physics/board/body/skateboard_truck_drives.rs"]
mod skateboard_truck_drives;
#[path = "physics/board/body/skateboard_trucks.rs"]
mod skateboard_trucks;
#[path = "physics/board/runtime/skateboard_update_up_attributes.rs"]
mod skateboard_update_up_attributes;
#[path = "physics/board/runtime/skateboard_update_up_vector.rs"]
mod skateboard_update_up_vector;
#[path = "physics/board/runtime/skateboard_wheels.rs"]
mod skateboard_wheels;
#[path = "physics/skeleton/pose/skeleton_affine_interpolation.rs"]
mod skeleton_affine_interpolation;
#[path = "physics/skeleton/pose/skeleton_animation_record.rs"]
mod skeleton_animation_record;
#[path = "physics/skeleton/pose/skeleton_animation_targets.rs"]
mod skeleton_animation_targets;
#[path = "physics/skeleton/body/skeleton_assembly.rs"]
mod skeleton_assembly;
#[path = "physics/skeleton/body/skeleton_assembly_construction.rs"]
mod skeleton_assembly_construction;
#[path = "physics/skeleton/movement/skeleton_attributes.rs"]
pub mod skeleton_attributes;
#[path = "physics/skeleton/body/skeleton_body_attributes.rs"]
mod skeleton_body_attributes;
#[path = "physics/skeleton/body/skeleton_body_definition.rs"]
mod skeleton_body_definition;
#[cfg(test)]
#[path = "physics/skeleton/body/skeleton_body_definition_corpus.rs"]
mod skeleton_body_definition_corpus;
#[path = "physics/skeleton/body/skeleton_body_joints.rs"]
mod skeleton_body_joints;
#[path = "physics/skeleton/body/skeleton_body_mass.rs"]
mod skeleton_body_mass;
#[path = "physics/skeleton/collision/skeleton_collision_attributes.rs"]
mod skeleton_collision_attributes;
#[path = "physics/skeleton/collision/skeleton_collision_data.rs"]
mod skeleton_collision_data;
#[path = "physics/skeleton/collision/skeleton_collision_numeric.rs"]
mod skeleton_collision_numeric;
#[path = "physics/skeleton/collision/skeleton_collision_update_attributes.rs"]
mod skeleton_collision_update_attributes;
#[path = "physics/skeleton/controller/skeleton_controller.rs"]
mod skeleton_controller;
#[path = "physics/skeleton/collision/skeleton_culling.rs"]
mod skeleton_culling;
#[path = "physics/skeleton/pose/skeleton_current_animation.rs"]
mod skeleton_current_animation;
#[path = "physics/skeleton/body/skeleton_data.rs"]
mod skeleton_data;
#[cfg(test)]
#[path = "physics/skeleton/body/skeleton_data_corpus.rs"]
mod skeleton_data_corpus;
#[cfg(test)]
#[path = "physics/skeleton/body/skeleton_data_tests.rs"]
mod skeleton_data_tests;
#[path = "physics/skeleton/drives/skeleton_drive_attributes.rs"]
mod skeleton_drive_attributes;
#[path = "physics/skeleton/drives/skeleton_drives.rs"]
mod skeleton_drives;
#[cfg(test)]
#[path = "physics/skeleton/drives/skeleton_drives_corpus.rs"]
mod skeleton_drives_corpus;
#[path = "physics/skeleton/controller/skeleton_general_force.rs"]
mod skeleton_general_force;
#[path = "physics/skeleton/controller/skeleton_general_update.rs"]
mod skeleton_general_update;
#[path = "physics/skeleton/movement/skeleton_hat.rs"]
mod skeleton_hat;
#[path = "physics/skeleton/movement/skeleton_head_tracking.rs"]
mod skeleton_head_tracking;
#[path = "physics/skeleton/ik/skeleton_ik.rs"]
mod skeleton_ik;
#[path = "physics/skeleton/ik/skeleton_ik_attributes.rs"]
mod skeleton_ik_attributes;
#[path = "physics/skeleton/ik/skeleton_ik_geometry.rs"]
mod skeleton_ik_geometry;
#[path = "physics/skeleton/ik/skeleton_ik_line_frame.rs"]
mod skeleton_ik_line_frame;
#[path = "physics/skeleton/ik/skeleton_ik_prephysics.rs"]
mod skeleton_ik_prephysics;
#[path = "physics/skeleton/ik/skeleton_ik_state.rs"]
mod skeleton_ik_state;
#[path = "physics/skeleton/ik/skeleton_ik_tangent.rs"]
mod skeleton_ik_tangent;
#[path = "physics/skeleton/ik/skeleton_ik_targets.rs"]
mod skeleton_ik_targets;
#[path = "physics/skeleton/drives/skeleton_joint_attributes.rs"]
mod skeleton_joint_attributes;
#[path = "physics/skeleton/movement/skeleton_jump_adjustments.rs"]
mod skeleton_jump_adjustments;
#[path = "physics/skeleton/movement/skeleton_landing_adjust.rs"]
mod skeleton_landing_adjust;
#[path = "physics/skeleton/movement/skeleton_landing_attributes.rs"]
mod skeleton_landing_attributes;
#[path = "physics/skeleton/movement/skeleton_landing_on_board.rs"]
mod skeleton_landing_on_board;
#[path = "physics/skeleton/drives/skeleton_normal_displacement.rs"]
mod skeleton_normal_displacement;
#[path = "physics/skeleton/drives/skeleton_normal_joint_limits.rs"]
mod skeleton_normal_joint_limits;
#[path = "physics/skeleton/movement/skeleton_offboard_adjust.rs"]
mod skeleton_offboard_adjust;
#[path = "physics/skeleton/movement/skeleton_offboard_reorient.rs"]
mod skeleton_offboard_reorient;
#[path = "physics/skeleton/postphysics/skeleton_physics_history.rs"]
mod skeleton_physics_history;
#[path = "physics/skeleton/postphysics/skeleton_post_physics_records.rs"]
mod skeleton_post_physics_records;
#[path = "physics/skeleton/processed/skeleton_processed_contact.rs"]
mod skeleton_processed_contact;
#[path = "physics/skeleton/movement/skeleton_query_rotation.rs"]
mod skeleton_query_rotation;
#[path = "physics/skeleton/pose/skeleton_root_transforms.rs"]
mod skeleton_root_transforms;
#[path = "physics/skeleton/controller/skeleton_runtime_state.rs"]
mod skeleton_runtime_state;
#[path = "physics/skeleton/movement/skeleton_skateboard_offset.rs"]
mod skeleton_skateboard_offset;
#[path = "physics/skeleton/movement/skeleton_skateboard_wobble.rs"]
mod skeleton_skateboard_wobble;
#[path = "physics/skeleton/controller/skeleton_state_constructor.rs"]
mod skeleton_state_constructor;
#[path = "physics/skeleton/ik/skeleton_two_bone.rs"]
mod skeleton_two_bone;
#[path = "physics/collision/step_selector_geometry.rs"]
mod step_selector_geometry;
#[path = "physics/collision/walking_step_query.rs"]
mod walking_step_query;
#[path = "physics/collision/walking_trajectory_distance.rs"]
mod walking_trajectory_distance;
pub use simulation_dmo_assembly_part_session::{
    DmoBankResidentModelCatalog, DmoBankResidentModelCatalogError,
    DmoBankResidentModelRoutePosition, MainDmoSimulationDeferredDeregistrationAcknowledgementError,
    MainDmoSimulationDeferredDeregistrationQueueError,
    MainDmoSimulationDeferredDeregistrationQueueFailure, MainDmoSimulationDeferredDispatch,
    MainDmoSimulationDeferredDispatchError, MainDmoSimulationDeferredMarkerKind,
    MainDmoSimulationDeferredProducer, MainDmoSimulationDeferredProducerFailure,
    MainDmoSimulationDeferredProducerFailureKind, MainDmoSimulationDeferredProducerUpdate,
    MainDmoSimulationDeferredRegistrationDriver,
};
#[path = "session/streaming/selected_arena_managers.rs"]
mod selected_arena_managers;
#[path = "session/streaming/selected_dmo_bank_startup.rs"]
mod selected_dmo_bank_startup;
#[path = "session/streaming/selected_hull_loader.rs"]
mod selected_hull_loader;
#[path = "session/living_world/selected_living_world_dmo.rs"]
mod selected_living_world_dmo;
#[path = "session/living_world/selected_living_world_recipes.rs"]
mod selected_living_world_recipes;
pub use selected_living_world_dmo::SelectedDynamicObjectVisual;
#[path = "session/living_world/selected_living_world_vehicle_visual.rs"]
mod selected_living_world_vehicle_visual;
pub use selected_living_world_vehicle_visual::SelectedVehicleVisual;
pub use selected_living_world_recipes::pedestrian_animation::presentation::{PedestrianPlacement, SelectedPedestrianVisual};
pub use selected_living_world_recipes::crowd::{
    CrowdTargetCamera, IMPOSTER_ATLAS, IMPOSTER_CELL, SelectedCrowdImposter, SelectedCrowdTarget, SelectedCrowdVisual,
    imposter_transform,
};
#[path = "living_world/assets/living_world_presentation_opacity.rs"]
mod living_world_presentation_opacity;
pub use living_world_presentation_opacity::{LivingWorldOpacityDistance, LivingWorldOpacityParameters, LivingWorldOpacitySettings, LivingWorldOpacityState};
#[path = "session/startup/selected_original_asset_classes.rs"]
mod selected_original_asset_classes;
#[path = "session/startup/selected_original_assets_bootstrap.rs"]
mod selected_original_assets_bootstrap;
#[path = "session/selected_pause_manager.rs"]
pub mod selected_pause_manager;
#[allow(
    dead_code,
    reason = "selected input owner awaits retained session frame caller"
)]
#[path = "session/selected_production_input_owner.rs"]
mod selected_production_input_owner;
#[path = "session/streaming/selected_resident_startup.rs"]
mod selected_resident_startup;
#[path = "session/startup/selected_session_boot_owner.rs"]
mod selected_session_boot_owner;
pub use physical_player_phys_out::construct::audio::{SkaterAudioCollision, SkaterAudioPhysOut};
pub use selected_session_boot_owner::public_facade::AudioVehicle;
pub use selected_session_boot_owner::public_facade::AudioDynamicObject;
pub use selected_session_boot_owner::public_facade::{AudioFrame, AudioFrontEnd, AudioSkater, BailVisual, SecurityPresentation};
pub use selected_session_boot_owner::public_facade::{SelectedSkaterVisual, SkaterNameTag};
pub use selected_session_boot_owner::public_facade::{
    AudioPedestrian, CareerHud, EndOptionsView, FrontendArguments, PhotoSelectView, PressADialog, SelectedBootProgress, SelectedCommandStartPolicy, SelectedControlledBootPolicy,
    SelectedFrameNumerics, SelectedGestureAllocation, SelectedGravityPolicy, SelectedLiveError,
    SelectedLivePending, SelectedLiveSession, SelectedLiveStage, SelectedQueuedFrameOutcome,
    SelectedQueuedFramePolicy, SelectedRootWPolicy, SelectedSceneCharacterVisual, SelectedSpawnPolicy,
    SelectedVisualSnapshot, WorldPainterKeys, TrickGuideView,
};
#[allow(dead_code, reason = "retained frame seam awaiting native app caller")]
#[path = "session/selected_session_frame.rs"]
mod selected_session_frame;
#[path = "session/selected_session_marker.rs"]
mod selected_session_marker;
#[path = "session/startup/selected_session_physical_pair_startup.rs"]
mod selected_session_physical_pair_startup;
#[path = "session/startup/selected_session_player_startup.rs"]
mod selected_session_player_startup;
#[allow(dead_code, reason = "selected terrain awaits native session caller")]
#[path = "session/selected_session_terrain.rs"]
mod selected_session_terrain;
#[path = "session/startup/selected_session_world_startup.rs"]
mod selected_session_world_startup;
#[path = "session/streaming/selected_simulation_asset_services.rs"]
mod selected_simulation_asset_services;
#[path = "session/streaming/selected_terrain_startup.rs"]
mod selected_terrain_startup;
#[path = "session/streaming/selected_world_stream.rs"]
mod selected_world_stream;
#[path = "physics/solver/drives/simulation_drive_jacobian.rs"]
pub mod simulation_drive_jacobian;
#[path = "physics/solver/drives/simulation_drive_pool.rs"]
pub mod simulation_drive_pool;
#[path = "physics/solver/bodies/simulation_dynamic_update.rs"]
mod simulation_dynamic_update;
#[path = "streaming/simulation/simulation_environment_unlock_router.rs"]
pub mod simulation_environment_unlock_router;
#[path = "physics/solver/simulation_horus_pipeline.rs"]
mod simulation_horus_pipeline;
#[path = "streaming/simulation/simulation_inner_registry_asset.rs"]
mod simulation_inner_registry_asset;
#[path = "physics/solver/bodies/simulation_island_map.rs"]
mod simulation_island_map;
#[path = "physics/solver/joints/simulation_joint_build_effective_mass_middle.rs"]
mod simulation_joint_build_effective_mass_middle;
#[path = "physics/solver/joints/simulation_joint_build_effective_mass_prefix.rs"]
mod simulation_joint_build_effective_mass_prefix;
#[path = "physics/solver/joints/simulation_joint_build_effective_mass_suffix.rs"]
mod simulation_joint_build_effective_mass_suffix;
#[path = "physics/solver/joints/simulation_joint_build_final_record.rs"]
mod simulation_joint_build_final_record;
#[path = "physics/solver/joints/simulation_joint_build_stage1.rs"]
mod simulation_joint_build_stage1;
#[path = "physics/solver/joints/simulation_joint_build_swing.rs"]
mod simulation_joint_build_swing;
#[path = "physics/solver/joints/simulation_joint_build_through_effective_mass.rs"]
mod simulation_joint_build_through_effective_mass;
#[path = "physics/solver/joints/simulation_joint_build_through_prediction.rs"]
mod simulation_joint_build_through_prediction;
#[path = "physics/solver/joints/simulation_joint_build_twist_prediction.rs"]
mod simulation_joint_build_twist_prediction;
#[path = "physics/solver/joints/simulation_joint_jacobian.rs"]
pub mod simulation_joint_jacobian;
#[path = "physics/solver/joints/simulation_joint_numeric.rs"]
mod simulation_joint_numeric;
#[path = "physics/solver/joints/simulation_joint_pool.rs"]
pub mod simulation_joint_pool;
#[path = "physics/solver/joints/simulation_joint_preparation.rs"]
mod simulation_joint_preparation;
#[path = "streaming/simulation/simulation_normal_arena_adapter.rs"]
pub mod simulation_normal_arena_adapter;
#[path = "streaming/simulation/simulation_normal_arena_plan.rs"]
pub mod simulation_normal_arena_plan;
#[path = "streaming/simulation/simulation_pointer_arena_adapter.rs"]
pub mod simulation_pointer_arena_adapter;
#[path = "streaming/simulation/simulation_registry_arena_adapter.rs"]
pub mod simulation_registry_arena_adapter;
#[path = "streaming/simulation/simulation_resident_collection.rs"]
pub mod simulation_resident_collection;
#[path = "streaming/simulation/simulation_resident_registry.rs"]
pub mod simulation_resident_registry;
#[path = "physics/solver/bodies/simulation_rigid_body_pool.rs"]
pub mod simulation_rigid_body_pool;
#[path = "streaming/simulation/simulation_static_arena_adapter.rs"]
pub mod simulation_static_arena_adapter;
#[path = "streaming/simulation/simulation_static_arena_plan.rs"]
pub mod simulation_static_arena_plan;
#[path = "physics/solver/bodies/simulation_volume_integral.rs"]
mod simulation_volume_integral;
#[path = "physics/solver/bodies/simulation_xenon_address_image.rs"]
mod simulation_xenon_address_image;
#[path = "actors/skater/skater_manager_adjust_spawn_position.rs"]
mod skater_manager_adjust_spawn_position;
#[path = "actors/skater/skater_manager_local_identity.rs"]
mod skater_manager_local_identity;
#[path = "actors/skater/skater_push_shared_attributes.rs"]
mod skater_push_shared_attributes;
#[path = "actors/skater/skater_wipeout_recovery_graph.rs"]
mod skater_wipeout_recovery_graph;
#[path = "animation/graphs/state_graph_context.rs"]
pub mod state_graph_context;
#[path = "animation/graphs/state_graph_controller_route.rs"]
pub mod state_graph_controller_route;
#[path = "animation/graphs/state_graph_runtime.rs"]
pub mod state_graph_runtime;
#[path = "physics/collision/static_ground_pool.rs"]
pub mod static_ground_pool;
#[path = "streaming/streaming.rs"]
pub mod streaming;
#[path = "rendering/proxy_shader_texture_table_draft.rs"]
pub mod shader_texture_table;
#[path = "rendering/texture.rs"]
pub mod texture;
#[path = "rendering/respawn_noise.rs"]
pub mod respawn_noise;
#[path = "physics/wipeout.rs"]
pub mod wipeout;
#[path = "world/world.rs"]
pub mod world;
#[path = "world/world_painter_layer.rs"]
mod world_painter_layer;
#[path = "world/world_painter_layer_asset.rs"]
mod world_painter_layer_asset;
#[path = "world/world_painter_palette.rs"]
mod world_painter_palette;
#[path = "streaming/world_stream_adapter.rs"]
pub mod world_stream_adapter;
#[path = "streaming/world_stream_catalog.rs"]
pub mod world_stream_catalog;
#[path = "streaming/world_stream_runtime.rs"]
pub mod world_stream_runtime;
#[path = "streaming/world_stream_store.rs"]
pub mod world_stream_store;

pub use input_pad::device_manager::selected_runtime::{
    SelectedDeviceInputEvents, SelectedInputGroup,
};
pub use juice_trick_input::boot::{
    JuiceBootPlatform, JuiceConnection, JuiceConsoleName, JuiceEndpoint, JuiceHostOption,
};

/// Typed platform calls for the retained native pad device owner.
pub use input_pad::{
    XboxPadSample,
    device::{PadCapabilitiesResult, PadPlatform, PadStateResult, PadVibration},
};

#[path = "physics/solver/simulation_entity_roster.rs"]
mod simulation_entity_roster;
#[path = "physics/solver/simulation_sim_rate.rs"]
mod simulation_sim_rate;

#[path = "assets/attributes/attribute_notifications.rs"]
mod attribute_notifications;
#[path = "camera/camera_attribute_read.rs"]
mod camera_attribute_read;
#[path = "camera/camera_attributes.rs"]
mod camera_attributes;
#[path = "camera/camera_collision.rs"]
mod camera_collision;
#[path = "camera/camera_compass.rs"]
mod camera_compass;
#[path = "camera/camera_drop.rs"]
mod camera_drop;
#[path = "camera/frame/camera_frame.rs"]
mod camera_frame;
#[path = "camera/graph/camera_graph.rs"]
mod camera_graph;
#[path = "camera/graph/camera_graph_factory.rs"]
mod camera_graph_factory;
#[path = "camera/graph/camera_man.rs"]
mod camera_man;
#[path = "camera/camera_parameters.rs"]
mod camera_parameters;
#[path = "camera/camera_positioner.rs"]
mod camera_positioner;
#[path = "camera/camera_prediction.rs"]
mod camera_prediction;
#[path = "camera/rig/camera_rig.rs"]
mod camera_rig;
#[path = "camera/frame/camera_session.rs"]
mod camera_session;
#[path = "camera/camera_shake.rs"]
mod camera_shake;
#[path = "camera/shots/camera_shot.rs"]
mod camera_shot;
#[path = "camera/shots/camera_shot_manager.rs"]
mod camera_shot_manager;
#[path = "camera/camera_subject.rs"]
mod camera_subject;
#[path = "camera/camera_tracker.rs"]
mod camera_tracker;
#[path = "math/token_file_decimal.rs"]
mod token_file_decimal;
#[path = "camera/graph/trigger_camera_group.rs"]
mod trigger_camera_group;

#[path = "session/hermes_challenge_queue.rs"]
mod hermes_challenge_queue;
#[path = "session/hermes_game_sim_queue.rs"]
mod hermes_game_sim_queue;

#[cfg(test)]
#[path = "streaming/proxy_asset_census.rs"]
mod proxy_asset_census;

#[path = "physics/collision/walking_step_results.rs"]
mod walking_step_results;
#[path = "physics/collision/walking_step_selection.rs"]
mod walking_step_selection;
#[path = "physics/collision/walking_step_tuning.rs"]
mod walking_step_tuning;
#[path = "rendering/skater/skater_ropa_runtime.rs"]
mod skater_ropa_runtime;
