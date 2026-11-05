# Changes since Smokey's mod (generated)

Smokey's mod (a fork of this repository at 0.2.9) brought the Skate 3 Rust Engine's player
into Skate 2. This port took that code as its starting point; this file lists, per file, what
the port changed since then (line counts from a line diff, line endings ignored).

- **same**: identical to the mod
- **changed**: lines added/removed by the port
- **new**: not in the mod (written for the port)
- **removed**: in the mod, not in the port

## Skate 3 physics module (`crates/skate2-core/src/physics/modes/skate3`)

343 same, 82 changed, 3 new, 5 removed; +2161 / -1286 lines.

| File | Status | Lines |
|---|---|---|
| `animation_pose/authored_clips.rs` | changed | +1 / -1 |
| `camera/graph.rs` | changed | +1 / -1 |
| `camera/publication.rs` | changed | +1 / -1 |
| `camera/runtime.rs` | changed | +8 / -2 |
| `camera/shot_data.rs` | changed | +19 / -1 |
| `camera/tests/runtime.rs` | changed | +2 / -2 |
| `custom_difficulty.rs` | changed | +3 / -3 |
| `graph_host/action.rs` | changed | +1 / -1 |
| `graph_host/mod.rs` | changed | +1 / -0 |
| `graph_host/motion.rs` | changed | +4 / -2 |
| `graph_host/motion_channels.rs` | changed | +4 / -0 |
| `graph_host/motion_conditions.rs` | changed | +12 / -0 |
| `graph_host/motion_execute.rs` | changed | +3 / -0 |
| `graph_host/motion_execute_leaf.rs` | changed | +12 / -13 |
| `graph_host/motion_gameplay_conditions.rs` | changed | +20 / -3 |
| `graph_host/motion_hand_services.rs` | changed | +6 / -1 |
| `graph_host/motion_instances.rs` | changed | +4 / -0 |
| `graph_host/motion_moving_object.rs` | new | 397 lines |
| `graph_host/motion_shove.rs` | changed | +21 / -1 |
| `graph_host/motion_stock_gameplay.rs` | changed | +4 / -92 |
| `graph_host/pushing.rs` | changed | +14 / -0 |
| `graph_host/tests/motion_bump.rs` | changed | +2 / -2 |
| `graph_host/tests/motion_slide.rs` | changed | +2 / -2 |
| `graph_host/tests/motion_stock.rs` | changed | +2 / -2 |
| `graph_host/tests/pushing_metadata.rs` | changed | +1 / -1 |
| `input/controllers.rs` | changed | +4 / -2 |
| `input/gesture_input.rs` | changed | +1 / -1 |
| `mod.rs` | changed | +99 / -4 |
| `modding.rs` | changed | +29 / -0 |
| `modding/player_physics.rs` | changed | +7 / -7 |
| `physics.rs` | changed | +23 / -8 |
| `physics/air_phase.rs` | changed | +24 / -0 |
| `physics/air_trajectory/mod.rs` | changed | +61 / -3 |
| `physics/animation_phase.rs` | changed | +57 / -3 |
| `physics/biped_air.rs` | changed | +1 / -1 |
| `physics/biped_ground.rs` | changed | +139 / -24 |
| `physics/biped_ground/grab.rs` | changed | +17 / -1 |
| `physics/biped_ground/grab_runtime.rs` | changed | +9 / -0 |
| `physics/camera_output.rs` | changed | +2 / -2 |
| `physics/climbing/approach.rs` | changed | +1 / -1 |
| `physics/climbing/mod.rs` | changed | +5 / -4 |
| `physics/collision_groups.rs` | changed | +19 / -0 |
| `physics/controls.rs` | changed | +1 / -1 |
| `physics/foot_ik.rs` | changed | +7 / -3 |
| `physics/frame.rs` | changed | +42 / -60 |
| `physics/ground_animation.rs` | changed | +3 / -15 |
| `physics/ground_animation/board.rs` | changed | +9 / -0 |
| `physics/ground_animation/settings.rs` | changed | +0 / -2 |
| `physics/ground_animation/turning.rs` | removed | |
| `physics/ground_phase.rs` | changed | +0 / -3 |
| `physics/ground_runtime/entry.rs` | changed | +0 / -1 |
| `physics/ground_runtime/input.rs` | changed | +45 / -0 |
| `physics/ground_runtime/mod.rs` | changed | +0 / -1 |
| `physics/ground_runtime/mongo_push.rs` | removed | |
| `physics/ground_runtime/state.rs` | changed | +0 / -3 |
| `physics/ground_runtime/update.rs` | changed | +40 / -19 |
| `physics/handplant.rs` | changed | +4 / -4 |
| `physics/input_phase.rs` | changed | +1 / -0 |
| `physics/input_teleport.rs` | changed | +1 / -1 |
| `physics/network.rs` | removed | |
| `physics/offboard/air_selector.rs` | changed | +1 / -1 |
| `physics/offboard/air_trajectory.rs` | changed | +2 / -2 |
| `physics/offboard/board_manager/runtime.rs` | changed | +27 / -0 |
| `physics/offboard/contact_toolkit/world.rs` | changed | +1 / -1 |
| `physics/offboard/ground_state.rs` | changed | +4 / -4 |
| `physics/offboard/skeleton_ground.rs` | changed | +144 / -3 |
| `physics/player_state.rs` | changed | +3 / -0 |
| `physics/player_state/publication.rs` | changed | +20 / -0 |
| `physics/player_state/selection.rs` | changed | +10 / -0 |
| `physics/player_state/transition.rs` | changed | +6 / -2 |
| `physics/respawn.rs` | changed | +208 / -45 |
| `physics/revert_state.rs` | changed | +1 / -1 |
| `physics/riding_outputs.rs` | changed | +18 / -2 |
| `physics/shove_probe.rs` | new | 186 lines |
| `physics/skater.rs` | changed | +28 / -6 |
| `physics/skeleton_feedback.rs` | changed | +29 / -7 |
| `physics/skeleton_input_general.rs` | changed | +5 / -2 |
| `physics/skeleton_input_runtime.rs` | changed | +2 / -0 |
| `physics/solid_contacts.rs` | changed | +20 / -5 |
| `physics/solve.rs` | changed | +33 / -90 |
| `physics/solve/assembly_contacts.rs` | changed | +13 / -1 |
| `physics/wipeout.rs` | changed | +26 / -1 |
| `physics/wipeout_states/skeleton.rs` | changed | +1 / -1 |
| `physics/wipeout_states/update.rs` | changed | +12 / -0 |
| `physics/world_proxies.rs` | new | 119 lines |
| `scoring_runtime.rs` | changed | +2 / -807 |
| `scoring_runtime/display.rs` | removed | |
| `scoring_runtime/tests.rs` | removed | |
| `skater_animation.rs` | changed | +30 / -0 |
| `world_types.rs` | changed | +14 / -0 |

## Vendored skate-core (`vendor/skate3-engine/crates/skate-core`)

721 same, 17 changed, 1 new, 0 removed; +367 / -48 lines.

| File | Status | Lines |
|---|---|---|
| `src/air/ground_jump/mod.rs` | changed | +6 / -10 |
| `src/air/ground_jump/types.rs` | changed | +0 / -4 |
| `src/animation/foot_ik/contact.rs` | changed | +4 / -2 |
| `src/animation/foot_ik/state.rs` | changed | +2 / -2 |
| `src/animation/foot_ik/tests.rs` | changed | +1 / -1 |
| `src/input/mod.rs` | changed | +1 / -0 |
| `src/input/player_input_data.rs` | new | 60 lines |
| `src/input/riding_intentions.rs` | changed | +38 / -3 |
| `src/physics/board_ground.rs` | changed | +68 / -0 |
| `src/physics/board_runtime.rs` | changed | +5 / -0 |
| `src/physics/board_step.rs` | changed | +5 / -0 |
| `src/physics/contact_feedback/board_reports.rs` | changed | +9 / -2 |
| `src/physics/skeleton_root.rs` | changed | +1 / -1 |
| `src/physics/tests/board_ground.rs` | changed | +26 / -0 |
| `src/physics/tests/contact_reports.rs` | changed | +2 / -2 |
| `src/player/offboard/grab_scene.rs` | changed | +35 / -0 |
| `src/player/respawn.rs` | changed | +73 / -21 |
| `src/player/tests/respawn.rs` | changed | +31 / -0 |

## Vendored skate-data (`vendor/skate3-engine/crates/skate-data`)

48 same, 6 changed, 1 new, 4 removed; +561 / -268 lines.

| File | Status | Lines |
|---|---|---|
| `examples/apt_data.rs` | removed | |
| `examples/apt_movie_data.rs` | removed | |
| `examples/hud_data.rs` | removed | |
| `examples/scoring_flow_data.rs` | removed | |
| `src/asset_io.rs` | changed | +101 / -197 |
| `src/collections.rs` | changed | +40 / -3 |
| `src/lib.rs` | changed | +1 / -0 |
| `src/manifest.rs` | changed | +6 / -29 |
| `src/physics_skeleton.rs` | changed | +86 / -3 |
| `src/vault.rs` | new | 313 lines |
| `tests/manifest.rs` | changed | +14 / -36 |

## Vendored skate-dynamics (`vendor/skate3-engine/crates/skate-dynamics`)

6 same, 0 changed, 0 new, 0 removed; +0 / -0 lines.

## Integration layer

+2176 / -395 lines over these files.

| File | Status | Lines |
|---|---|---|
| `crates/skate2-core/src/physics/skate3_player.rs` | changed | +385 / -126 |
| `crates/skate2-core/src/physics/skate3_bridge.rs` | same | |
| `crates/skate2-core/src/physics/skate3_terrain.rs` | same | |
| `crates/skate2-core/src/physics/skate3_grabs.rs` | changed | +19 / -0 |
| `crates/skate2-core/src/physics/skate3_grinds.rs` | changed | +2 / -2 |
| `crates/skate2-core/src/physics/skate3_pushing.rs` | new | 356 lines |
| `crates/skate2-core/src/physics/assembly/skate3_world.rs` | changed | +21 / -92 |
| `crates/skate2-core/src/physics/assembly/skate3_player_backend.rs` | same | |
| `crates/skate2-core/src/physics/assembly/parts/skate3_part_mirror.rs` | same | |
| `crates/skate2-core/src/physics/solver/bodies/external_player_driver.rs` | same | |
| `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs` | changed | +88 / -2 |
| `crates/skate2-core/src/physics/player/skate3_actor_input.rs` | same | |
| `crates/skate2-core/src/physics/player/skate3_camera_rate.rs` | changed | +1 / -1 |
| `crates/skate2-core/src/physics/player/skate3_exit.rs` | same | |
| `crates/skate2-core/src/physics/player/skate3_spawn.rs` | same | |
| `crates/skate2-core/src/physics/player/skate3_tuning.rs` | removed | |
| `crates/skate2-core/src/session/selected_player_skate3.rs` | changed | +46 / -57 |
| `crates/skate2-core/src/session/selected_player_tick.rs` | changed | +113 / -31 |
| `crates/skate2-core/src/session/skate3_input_capture.rs` | same | |
| `crates/skate2-core/src/session/skate3_input_focus.rs` | changed | +13 / -0 |
| `crates/skate2-core/src/camera/frame/camera_skate3.rs` | changed | +15 / -2 |
| `crates/skate2-core/src/animation/runtime/actor_skate3_gestures.rs` | changed | +41 / -6 |
| `crates/skate2-core/src/animation/runtime/skate3_gesture_playback.rs` | changed | +65 / -2 |
| `crates/skate2-core/src/animation/runtime/skate3_gesture_streams.rs` | changed | +127 / -49 |
| `crates/skate2-core/src/actors/skate3_auto_push.rs` | removed | |
| `crates/skate2-core/src/physics/player/output/conditioning/scoring/skate3_catalog.rs` | changed | +53 / -7 |
| `crates/skate2-core/src/physics/player/output/conditioning/scoring/skate3_hom.rs` | changed | +67 / -13 |
| `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs` | new | 650 lines |
| `crates/skate2-core/src/physics/player/output/conditioning/scoring/attributes.rs` | changed | +5 / -5 |
| `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_shove.rs` | new | 109 lines |

