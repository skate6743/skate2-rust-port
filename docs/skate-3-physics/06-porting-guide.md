# Porting the Skate 3 physics to another host in one pass

## 0. Pick the shape of the target

- **A Skate 2-based codebase** (another fork of this port, Smokey's mod): copy everything listed
  in step 1, including the integration layer and the shared Skate 2 changes, then build. The
  integration files assume this port's Skate 2 types (`PhysicalAssemblyWorld`,
  `StaticGroundPool`, `ActorConstruction`, PhysOut), so a fork that diverged needs those seams
  adapted, not rewritten.
- **A different host** (a Skate 3 engine, a standalone game): take the vendored crates and the
  module as they are, stub six small things (step 2), and write your own driver in place of
  `crates/skate2-core/src/physics/skate3_player.rs` (steps 3–5). Use `skate3_player.rs` and the
  other integration files as the reference for every call.

Either way, keep the module's internals untouched at first: they are TU3-checked. Get it running
unchanged, then compare.

## 1. Copy list

| What | Path | Notes |
|---|---|---|
| Engine crates | `vendor/skate3-engine/` (whole: `crates/skate-core`, `crates/skate-data`, `crates/skate-dynamics`, `LICENSE`, `SOURCE-SNAPSHOT.json`, `UPSTREAM-SYNC.json`, `maps/format-demo.skate`) | GPL-3.0-only; 25 files changed or added since Smokey's copy (`92-changes-since-smokeys-mod.md`) |
| The player | `crates/skate2-core/src/physics/modes/skate3/` (whole, 428 files, plus `SOURCE_PORT.json`) | `SOURCE_PORT.json` holds each captured file's upstream and ported hash |
| Integration driver | `crates/skate2-core/src/physics/skate3_player.rs` and its children `skate3_terrain.rs`, `skate3_grinds.rs`, `skate3_grabs.rs`, `skate3_pushing.rs`, `player/skate3_exit.rs`, `player/skate3_actor_input.rs`, `player/skate3_camera_rate.rs`, `player/skate3_spawn.rs` | mounted as `skate3_native::physics::player` |
| Integration seams in Skate 2 | `90-file-index.md` section 2 (42 files) | world bodies, solver ownership, PhysOut, camera, gestures, input, session tick |
| Shared Skate 2 systems with TU3 branches | `90-file-index.md` section 3 (84 files) | scoring, Hall of Meat, audio, breadcrumbs, pedestrians; only needed if the host uses Skate 2's versions of them |

Cargo (`crates/skate2-core/Cargo.toml`):

```toml
[features]
default = ["skate3-runtime"]
skate3-runtime = ["skate3-source"]
skate3-source = []

[dependencies]
glam = "=0.30.10"
serde = { version = "=1.0.229", features = ["derive"] }
serde_json = "=1.0.154"
skate3-core = { package = "skate-core", path = "../../vendor/skate3-engine/crates/skate-core" }
skate_data = { package = "skate-data", path = "../../vendor/skate3-engine/crates/skate-data" }
skate_dynamics = { package = "skate-dynamics", path = "../../vendor/skate3-engine/crates/skate-dynamics" }
```

`lib.rs`: `#[cfg(feature = "skate3-source")] mod skate3_native;` (the module directory, with its
`mod.rs`). The engine crates resolve as workspace members (path dependencies under the root).

## 2. Stub the six Skate 2 couplings in the module

| Item | Stub |
|---|---|
| `crate::diagnostics::test_log` (`event`, `span`, `timed_span`, `anomaly`, `enabled`) | no-ops returning a unit guard; `enabled()` false |
| `crate::ui::puddle_physics` (`Response`, `Decision`, `contact_at`, `grip_scale`) | `contact_at` → `None`, `grip_scale` → 1.0, `Decision::default()` (drag 0, no wipeout); this is the PC rain feature |
| `crate::reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, wrap_turns_to_radians}` | copy from `crates/skate2-core/src/physics/reckoning/reckoning_numeric.rs` |
| `crate::camera_compass::runtime::math::{unit_fraction, camera_atan2_pending}` | copy from `crates/skate2-core/src/camera/camera_compass.rs` (its `runtime::math`) |
| `crate::simulation_drive_jacobian::refine_drive_reciprocal_sqrt` | copy from `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs` |
| `crate::actor_construction::motion_runtime_session::trick_conditions::moving_object_in_sector` | copy (|movement| > 0.1 and atan2(−x, z) within [start, end]) |

The module's log macros (`trace::info!`, `debug!`, `warn!`, `error!` in `mod.rs`) print only
with `SKATE2_TRACE_SKATE3=1` (info/debug); warnings and errors always print.

## 3. Load (what `Skate3Player::load` does)

1. **Link the data.** `skate_data::asset_io::link(Arc<Provider>)` with a provider that returns
   the bytes of a Skate 3 game path (`data/anim/OnBoard.abin`, ...). The root is then
   `skate_data::asset_io::ROOT`. Files needed: see `01-architecture.md`, "Data".
2. **Build the scene.** `Scene { spawn, heading, triangles, rails }` from your static collision.
   Each `SceneTriangle` needs its RenderWare packed surface (audio 7, physics 5, pattern 4 bits);
   pass native edge flags/features when you have them (they keep TU3's edge contacts exact).
3. **Difficulty.** `difficulty::Difficulty::parse("easy" | "normal" | "hardcore" | "motorized" |
   "custom")`.
4. **Graphs and animation.** `skate_data::GameAssets::load_physics(root)` →
   `graph_runtime::StockGraphs::load(root, &manifest)`; `skater_animation::AnimationSource::load(root)`.
   Share these between players; everything else is per player.
5. **Physics state.** `physics::Skate3PhysicsState::load_with_host_scene(root, &scene, difficulty)`.
6. **Skater.** `physics::Skate3ControllerState::load_for_world(root, &graphs, &physics,
   difficulty.profile_key(), Some(animation))`.
7. **Controls and camera.** `physics::PlayerControls::load(root)`, `camera::CameraRuntime::load(root)`.
8. **Entry.** Place the skater with a teleport: `spawn::target(world, transform, on_board)`, then
   `skater.player_input.request_teleport_ex(target, Some(velocity))` and
   `skater.teleport_state.request_manual(target, on_board)`. Entering off board also seeds the
   graph's first input with the off-board state and filtered state (see `load`, lines after
   `if !entry.on_board`).
9. **Optional world feeds.** Grind assets (`HostGrindAsset`s, `grind_world`), grab objects
   (`set_grab_objects`), moving bodies (`set_moving_bodies` / `set_moving_mesh_bodies`), external
   queries (`set_external_queries`).
10. **Profile.** `set_profile` (or `set_skate2_profile` / `set_motorized_profile`), then
    `configure_character_stance`, `configure_animation_style`, `configure_equipment` (trucks and
    wheels 0.7/0.7 for a new Skate 3 skater), `set_graph_gestures`, camera options.

In this port the Skate 3 rig drives Skate 2's 7-part board and 24-part skeleton. Bodies are paired
by bone name, never by position, and velocities are transported between the two rigs' mass frames
(`velocity_at`). A host with its own rig does the same mapping or renders the Skate 3 output
directly.

## 4. Each tick (fixed 1/60 s, once per host tick)

```
host input poll ─► RawController ─► collect_controller
host world step (everything else)
sync moving bodies (set_moving_*), shove query answers (set_shove_*)
prepare  = skate3_native::physics::frame::prepare(physics, skater, controls, graphs, camera)
           (input sample: controls.simulation_actions / sample_solver_input)
finish   = skate3_native::physics::frame::finish(physics, skater, controls, actions, input_available, camera, prepared)
output   = export_frame(physics, skater, camera, hips_body)   → FrameOutput
host: apply take_body_reactions(), take_host_contacts(), take_shove_hit(), camera rate requests,
      teleport completion, breadcrumbs
```

`frame::advance` does prepare and finish back to back when nothing has to run in between. This
port splits them so Skate 2's Actor can take the Skate 3 pose between the animation graphs and
the solve.

Order rules that matter (each one fixed a real bug):
- A trajectory query completes the tick AFTER it was submitted (`trajectory.begin_tick`).
- Camera rate requests from tick N apply at the start of N+1, and never online.
- Body reactions apply to the host body before its next publication, once.
- No tick without resident collision; a queued teleport waits for the previous one to be consumed.
- Set the online flag before the tick (`set_online`: 25 passes).

## 5. What to read out

- **Pose**: `FrameOutput.bone_names` + `world_bones` (world-space bone transforms), `root_transform`
  (use `effective_root_transform()` for facing), `board_transform`, `board_parts`.
- **Camera**: `FrameOutput.camera` (`skate3_core::camera::CameraFrame`).
- **Gameplay facts** for scoring/audio: `FrameOutput.gameplay`, `physical` (the TU3 PhysOut
  publication), `state`, `category`, `wiping_out`, `teleported`. To feed a Skate 2-style scorer,
  copy `physical_player_phys_out_skate3.rs` (state ID map, filtered grind stores, frames since
  reset, state update count).

## 6. Acceptance checks (all reproduced on this port)

| Check | Expect |
|---|---|
| Stick quantization | stick X 0.3..0.9 → 9, 27, 45, 63, 81, 99, 117 / 127 at Processed+2712 |
| Turning | 10° heading change ~285 ms after full stick; 98–110 °/s at 4–9.4 m/s on flat ground; steering halves (push scalar 0.5) for ~0.75–1 s after a push |
| Running / sprint | ~6.0 m/s steady, sprint ~9.4 m/s decaying to 6 |
| Big air | a nollie at a vert lip keeps its deck vy into the first air tick |
| AI-skater bump | end-on board hit at ~9 m/s: board contact flag bit 30, Bumped, no wipeout |
| Prop push | grab a ramp, pull ~2 m; the legs cycle without restarting each tick |
| Breadcrumbs | entries every 1.5 m (score 100); an airborne A reset returns to the last good crumb |
| Online | 25 passes; turn wipeout limit resets after 4 s (Spot Battle) / 5 s (S.K.A.T.E.) |
| Grind scoring | grind exits score with valid scorables and spline GUIDs |

Tests in the repo: `crates/skate2-core` unit tests under `physics/modes/skate3/**/tests*`
(several `#[ignore]`: they need converted stock assets), the corpus tests
(`session/skate3_*_corpus.rs`; set `SKATE2_GAME_ROOT` and `SKATE3_DATA`), the engine's own tests
(`cargo test -p skate-core`).

## 7. Debugging switches

Traces (stderr, any build): `SKATE2_TRACE_SKATE3` (general, focus, gestures), `_AIR`, `_LAUNCH`,
`_STEER` (per-tick speed, turn, tilt, trucks: the same fields read live from TU3), `_WIPEOUT`
(reasons, forces, board group flags), `_GRIND`, `_GRAB`/`_GRABS`, `_BREADCRUMBS`, `_DARKCATCH`,
`_HANDS`, `_PUSH_COEFS`, `_PUSH_TICKS`, `_MANUAL_TURN`, `_RIDER_CONTACTS`, `_VISUAL_ROOT`,
`_WATER`, `_COAST`, `_FOOT_FORCE`; also `SKATE2_TRACE_SHOVE`, `SKATE2_TRACE_SCORE_SEQUENCE`,
`SKATE2_TRACE_AUDIO`, `SKATE2_TRACE_TRAJECTORY`, `SKATE2_TRACE_KNOWN_AIR`, `SKATE2_TRACE_OFFBOARD_AIR`.

Scripted input (`SKATE2_DIAGNOSTIC_*`): `PHYSICS` (e.g. `Skate 3:Normal`), `HOLD`, `LEFT_STICK`,
`PUSH_OLLIE`, `TRICK_CYCLE`, `GESTURE_RB`, `PIN`, `TURN_BAIL_LIMIT`, `SPAWN_SKATER` +
`SPAWN_SKATER_AT`, `TRAJECTORY_MAX_TIME`, `PUSHING_FOLLOW=0`, `PUSHING_HAND_IK=0`.
