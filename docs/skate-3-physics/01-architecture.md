# Architecture

## Layers

```
 Skate 2 game (host)                                      Skate 3 physics
 ─────────────────────────────────────────────            ──────────────────────────────────────────
 session/selected_player_tick.rs  (Skate 2 frame)  ──►    physics/skate3_player.rs  (Skate3Player)
   input, AI, LivingWorld, Actor phases                     terrain, grinds, grabs, input, teleports
   Skate 2 world solve (everyone else)                      │
   ▲ pose, PhysOut, camera, reactions                       ▼
   │                                                      physics/modes/skate3/  (crate::skate3_native)
   │                                                        physics/frame.rs  one TU3 tick
   │                                                        physics/*         states, solve, output
   │                                                        graph_host/*      Action/Motion graphs
   │                                                        camera/*          camera graph + CameraMan
   │                                                        input/*           pad, gestures
   │                                                        │
   │                                                        ▼
   │                                                      vendor/skate3-engine/crates/
   │                                                        skate-core     recovered calculations
   │                                                        skate-data     abin/graph/vault readers
   │                                                        skate-dynamics solid helpers
   │
 shared Skate 2 systems with Skate 3 branches:
   scoring (ScoreModule on Skate 3 data), wipeout scorer (Hall of Meat), audio (darkcatch, catch
   cues, reverb), breadcrumbs, pedestrians (contacts, shove), collision groups, camera transport
```

- **`vendor/skate3-engine`**: the engine's calculation crates, kept as crates. `skate-core` has
  no I/O and no engine startup: board runtime, contact/solver, skeleton bodies and drives,
  riding/speed models, air/trajectory, grind kernels, camera kernels, the dynamic graph runtime,
  scoring helpers. `skate-data` reads Skate 3 files (abin animation banks, state graphs, vault
  collections, physics skeletons, input config, gesture patterns), all through `asset_io`, whose
  reserved root `__skate3_linked__/assets` maps `private/stock/<path>` to the linked game's
  `<path>`.
- **`crates/skate2-core/src/physics/modes/skate3`** (module path `crate::skate3_native`): the
  engine's player, the part that used to live in `skate-game`: the physical player and its states,
  the input phase, the ActionGraph/MotionGraph host, the animation pose owner, the camera runtime,
  input/gesture mapping, difficulty. It compiles only with the `skate3-runtime` feature (default
  on; `crates/skate2-core/Cargo.toml`), which turns on `skate3-source`
  (`#[cfg(feature = "skate3-source")] mod skate3_native;` in `lib.rs`).
- **Integration layer**: the files named `skate3_*` plus the hooks in Skate 2 files (list in
  `90-file-index.md`, section 2). They create the player, feed it the Skate 2 world, run it in the
  right place of Skate 2's frame and publish its results back.
- **Shared Skate 2 systems**: Skate 2 code that runs in both modes and has Skate 3 branches or
  TU3 values (section 3 of the file index). See `04-shared-skate2-skate3.md`.

The module depends on almost nothing in Skate 2. Outside the vendored crates and `glam`,
`serde`, `serde_json`, it uses:

| Skate 2 item | Used by | What a new host does |
|---|---|---|
| `crate::diagnostics::test_log` | trace events (16 sites) | stub (no-op) or route to your logger |
| `crate::ui::puddle_physics` | `physics.rs`, `physics/frame.rs`, `physics/solve.rs` | PC rain feature: stub `contact_at` to `None`, `grip_scale` to 1 |
| `crate::reckoning_numeric`, `crate::camera_compass::runtime::math`, `crate::simulation_drive_jacobian::refine_drive_reciprocal_sqrt`, `crate::actor_construction::motion_runtime_session::trick_conditions::moving_object_in_sector` | `graph_host/motion_moving_object.rs`, `graph_host/motion_gameplay_conditions.rs` (our TU3 MovingObject port) | copy these small math helpers (they are the same TU3/Skate 2 functions) |

and `physics.rs` mounts the integration driver `crates/skate2-core/src/physics/skate3_player.rs`
as `skate3_native::physics::player` (`#[path = "../../skate3_player.rs"]`). That driver is the
part a new host rewrites.

## Ownership rules

These are the rules the integration keeps; a new host should keep them too.

1. **One solver per skater per tick.** With Skate 3 physics selected, the local skater's Skate 2
   board and skeleton bodies stay in Skate 2's body pool (allocation, list membership and handles
   unchanged) but are *externally driven* (`physics/solver/bodies/external_player_driver.rs`,
   `try_set_externally_driven`). Skate 2 skips that skater's physics sync, locomotion and solve
   (`native_skater(index)` in `session/selected_player_tick.rs`). The Skate 3 solve moves them, and
   `Skate3Player::publish_body_views` writes the result back as "mode body views" so Skate 2's
   readers (pose, camera, other skaters' collisions) see Skate 3's bodies.
2. **The Skate 2 world joins the Skate 3 solve as shadows.** After Skate 2's own solve,
   `Skate3Player::sync_world_bodies` takes this tick's moving bodies from Skate 2's pool
   (`physics/assembly/skate3_world.rs`, `try_skate3_bodies`): props (DMOs), vehicles,
   pedestrians, other skaters. Each joins at its centre of mass as a finite-mass body
   (`physics/modes/skate3/physics/world_proxies.rs`). A prop with an authored collision mesh
   answers board, wheel, skeleton and line queries with its native triangles at its current
   placement (`Skate3MeshBody`; re-placed only when it moved more than a millimetre,
   `same_prop_placement`). Groups are translated to TU3's numbering (`skate3_group`, see
   `04-shared-skate2-skate3.md`).
3. **Reactions go back once.** The Skate 3 solve's corrections to a host body come out as
   `BodyReaction` (velocity, angular velocity, position and orientation deltas;
   `world_types.rs`), applied to the matching Skate 2 body before its next publication
   (`try_apply_skate3_reactions`). Only bodies Skate 2 itself moves keep a reaction; a resting
   body with a zero reaction stays asleep.
4. **The static world is Skate 2's.** Skate 2 streams the city; the Skate 3 world
   (`skate3_core::physics::board_world::BoardWorld`) is rebuilt from the same resident triangles
   (`physics/skate3_terrain.rs`), on a background thread ("skate 3 terrain"), swapped in at the
   start of a tick. Travel and an empty world build inline. Without resident collision the player
   waits instead of running on an empty world.
5. **Grinds and grabs come from Skate 2's data.** Skate 2's grind splines (city and props,
   `DmoObjectMan::grindable_objects`) become `HostGrindAsset`s for the Skate 3 loose octree
   (`physics/skate3_grinds.rs`, `physics/modes/skate3/grind_world/`). DMO and vehicle grab splines
   become the off-board grab-scene registry (`physics/skate3_grabs.rs`), rebuilt every tick with
   assembly identities, so a validated grab knows the prop's body.
6. **Animation and pose.** Skate 3's stock graphs (`ActionGraph_OnBoard`, `MotionGraph_OnBoard`)
   evaluate the skater. The pose is retargeted onto Skate 2's skeleton by bone name
   (`try_publish_mode_movement_pose`). Skate 2's Actor still runs its hand/face streams from the
   Skate 3 body commands (`try_set_mode_body_pose`, `try_set_mode_body_commands`). Skate 3
   gestures play inside the Skate 3 graph's channel stack (`CharacterGesture` TU3 0x82BAA848).
7. **Camera.** The Skate 3 camera graph (`data/script/camera/Default_cameragraph.stategraph`)
   and CameraMan compute the gameplay camera (`physics/modes/skate3/camera/`), published to
   Skate 2's camera transport (`camera/frame/camera_skate3.rs`). Its slow-motion requests go
   through Skate 2's `SetSimRate` (`physics/player/skate3_camera_rate.rs`), only with the
   "airtime slow motion" option and never online.
8. **Everything else reads Skate 2's PhysOut.** `FrameOutput` is translated into Skate 2's
   PhysOut (`physics/player/output/physical_player_phys_out_skate3.rs`): state IDs, filtered
   state, grind stores, collision/audio facts, scoring facts. Scoring, audio, HUD, career,
   challenges, replays and online all keep reading PhysOut as they do in Skate 2 mode.
9. **Breadcrumbs have one owner.** In Skate 3 mode the TU3 respawn history is the Actor's only
   breadcrumb store, mirrored into Skate 2's manager for Skate 2's readers (`05-what-we-fixed.md`).
10. **Zero cost in Skate 2 mode.** `try_reconcile_skate3` returns at once when Skate 2 physics is
    picked and no Skate 3 player is left to hand back; all other hooks test that the player exists.

## The host boundary (`physics/modes/skate3/world_types.rs`)

| Type | Fields (summary) | Direction |
|---|---|---|
| `RawController` | `buttons: u16`, `triggers: [u8; 2]`, `left/right: [i16; 2]` | host → player, one pad sample |
| `Scene` | `spawn`, `heading`, `triangles: Vec<SceneTriangle>`, `rails: Vec<SceneRail>` | host → player world |
| `SceneTriangle` | 3 points, `surface`, `packed_surface` (RenderWare audio 7 / physics 5 / pattern 4 bits), optional native edges and features (flags, edge cosines) | |
| `HostGrindSegment` / `HostGrindAsset` | 4×4 segment coefficients, bounds, owner, two spline GUIDs, segment index | host → grind world |
| `BodyOutput` | transform, centre of mass, velocity, omega | player → host |
| `SkeletonPartOutput` | physics-to-animation bone name, body, bone transform | player → host |
| `FrameOutput` | tick, state, category, offboard, wiping_out, state_frames, frames_since_reset, board transform/velocity/7 parts, skeleton parts, root transform/velocity/omega, camera frame, bone names + world bones, pumping state, `physical` (the whole TU3 PhysicalPlayerInput publication), `gameplay` | player → host |
| `GameplayOutput` | state bytes 52..88, interactable object, landing quality, jump strength, hips/deck ground probes, foot surfaces, deck tilt, wheel hardness, trick name/flags/tweak, wipeout gesture, stance (fakie, mirrored, switch, regular), push style, board ground and wheel lines, 24 bone contacts and velocity changes, contact regions/flags, wipeout times, entity impacts (vehicle, skater, pedestrian), feet, heavy landing | player → host audio/scoring/career |
| `BodyReaction` | body id, linear/angular velocity deltas, position/orientation deltas | player → host world |

`FrameOutput::effective_root_transform` gives the world facing Actor queries and markers use: on
board with a mirrored stance it negates the X and Z axes, as TU3's processed-input publication
unmirrors animation.

The driver's API (`physics/skate3_player.rs`, `Skate3Player`): `load`, `sync_world_bodies`,
`prepare_animation_tick`, `advance`, `set_moving_bodies` / `set_moving_mesh_bodies`,
`set_external_queries`, `set_grab_objects`, `take_body_reactions`, `queue_teleport` /
`host_teleport` / `sync_teleport` / `acknowledge_teleport`, `travel`, `replace_scene` /
`replace_collision_scene`, `set_profile` / `set_skate2_profile` / `set_motorized_profile`,
`configure_character_stance`, `configure_animation_style`, `configure_equipment`,
`set_graph_gestures`, `set_camera_type`, `set_invert_look`, `set_airtime_slow_motion`,
`set_input_focus`, shove (`shove_query_point`, `set_shove_pedestrians`, `set_shove_skaters`,
`take_shove_hit`), `take_host_contacts`, breadcrumbs (`sync_breadcrumbs`,
`post_update_breadcrumbs`, `mirror_breadcrumbs`, `suggested_breadcrumb`, `pop_breadcrumb`,
`breadcrumb_position_ok`), `return_to_skate2`, `release`, `output` / `output_frame`.

## Data: everything comes from the linked Skate 3 data

Nothing is embedded. `skate3_player::assets_root` links `skate_data::asset_io` to the linked
Skate 3 folder or ISO (`crate::skate3::linked()`), and each loader reads through it once
(`linked_once`; a failure is not cached, so linking later works):

| Game path | Read by | What |
|---|---|---|
| `data/state/ActionGraph_OnBoard.stategraph`, `data/state/MotionGraph_OnBoard.stategraph` | `graph_runtime.rs` (`StockGraphs::load`) | the stock graphs (`private/game.json` is a built-in manifest naming them) |
| `data/anim/OnBoard.abin`, `OffBoard.abin`, `SkaterLeftHand.abin`, `SkaterRightHand.abin` | `skater_animation.rs` (`AnimationSource::load`), `skate-data` `physics_skeleton` | animation banks; OnBoard's kind-5 records are the physics skeletons |
| `data/db/skaterschema.vlt`, `data/db/skatercollections.vlt` | `skate-data` `collections.rs` (`Collections::load`) | every physics/animation setting (the vault collections), per physics mode |
| `data/config/input.cfg`, `data/joystick/*.pat` | `input/` | input config and gesture patterns |
| `data/script/camera/Default_cameragraph.stategraph`, `data/camera/*.shk` | `camera/` | camera graph and shakes |

Skate 3's audio (darkcatch cues, `Sk8_Air_Flip_Tricks.abk`), HUD movies and challenge data are
read by the shared Skate 2 systems from the same linked data.

## Settings

- **Physics Settings** (`ui/gameplay.rs`, `GameplaySettings { skate3, difficulty,
  airtime_slow_motion }`), shown only with Skate 3 data linked. Difficulty → Skate 3
  `physics_mode` collection (`session/selected_player_skate3.rs`):

  | UI difficulty | Profile key | Driver call |
  |---|---|---|
  | Easy | `easy` | `set_profile` |
  | Easy Motorized | `easy` | `set_motorized_profile` |
  | Normal | `normal` | `set_profile` |
  | Hardcore | `hardcore` | `set_profile` |
  | Motorized | `motorized` | `set_profile` |
  | Skate 2 Default | `custom` | `set_skate2_profile`: Skate 2's own vault tuning on the Skate 3 solver |

  The module's own `difficulty.rs` lists the native modes `easy, normal, hardcore, motorized, test`;
  `custom` takes the unused `test` slot.
- **Edit Skater > Style > Skate 3 Physics**: Skate 3 Style and Posture (Skate 3's own strings),
  Truck Tightness and Wheel Stiffness (default 0.7/0.7 as TU3's new-skater record; profile leaf
  `SK82_PC::Skate3Equipment`), Gesture Set. Skate 3 physics plays the saved Skate 3 gestures.
- **Online**: the solver runs 25 passes (`Skate3PhysicsState::set_online`, TU3
  `Island::Step_UpdatePrePhysics1` 0x82763E00) and the clock ignores camera slow motion.

## Physics mode changes

- **Entering** (`try_reconcile_skate3`): an on-board skater starts at its Actor's transform; an
  off-board one carries its state over (`PlayerEntry`: transform, velocity, state, holding board,
  filtered state, camera basis), with Skate 2 off-board states translated (501→502, 502/503→501,
  505→503, 500/504→500).
- **Leaving** (`return_to_skate2`, `physics/player/skate3_exit.rs`): the Skate 3 rates are handed
  to the Skate 2 bodies once, applied by the matching Actor reset after the real body reset and
  before Ground Enter samples deck speed. Actor reset generations and body occurrences keep a
  mode change from leaking into a later marker or career reset.
- **Party Play / Actor change**: the previous Actor is returned to its own solver before another
  is selected.
