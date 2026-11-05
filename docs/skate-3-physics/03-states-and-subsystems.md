# Physical states and subsystems

Paths without a prefix are under `crates/skate2-core/src/physics/modes/skate3/`. `core/...` is
`vendor/skate3-engine/crates/skate-core/src/...`. Addresses are TU3 unless marked "Skate 2".
Every file names its own TU3 functions in its first lines; `91-address-index.md` lists them all.

## Physical states (`core/player/state.rs`, `PhysicalStateId`)

The player is TU3's `PhysicalPlayer` (vtable 0x82328688, ctor 0x82DB3008). One state is current;
`CalcSuggestedState` 0x82D8ADE8 proposes, `SetPhysicsState` 0x82DB8540 switches
(`physics/player_state/transition.rs`), each state has Enter / Update / Exit / Post / FillPhysOut.
The per-state update dispatch is the `match` in `physics/frame.rs`.

| ID | State | Port | TU3 |
|---|---|---|---|
| 100 | PhysicsGround | `physics/ground_phase.rs`, `physics/ground_runtime/*`, `physics/ground_exit.rs` | vtable 0x82327244; Enter 0x82D37538, PreUpdate 0x82D37C50, Update 0x82D37C88, UpdateSkateboard 0x82D38800, Exit 0x82D37B20 |
| 101 | SlideGround (powerslide) | `physics/slide_state.rs`, `physics/slide_state/*` | vtable 0x823285B4; UpdateSkateboard 0x82D3A900 |
| 102 | RevertGround | `physics/revert_state.rs` | vtable 0x82327330; Enter 0x82D43488, Update 0x82D43518, Post 0x82D4C070, Fill 0x82D43B10 |
| 103 | GroundAnimation (animated pops) | `physics/ground_animation.rs`, `physics/ground_animation/*` | 0x82D33BF8 / 0x82D33DB8 / 0x82D33C70 / 0x82D34150; board 0x82D33E30; skeleton 0x82D33E00 |
| 104 | Skitching | not ported | |
| 105 | FollowPath | not ported | |
| 200 | PhysicsAir | `physics/air_phase.rs`, `physics/air_phase/*` | 0x82D34388 / 0x82D346C0 / 0x82D346A0; board 0x82D34AB0 |
| 201 | KnownAir (locked trajectory, big air) | `physics/known_air.rs`, `physics/known_air/*` | 0x82D352D0 / 0x82D35C58 / 0x82D35920 / 0x82D36590; Fill 0x82D36880 |
| 202 | PhysicsAirSecondary (grind trick) | `physics/grind_trick.rs` | vtable 0x82327174; Enter 0x82D335F8, Exit 0x82D33678, Update 0x82D336F8, board 0x82D33928, post 0x82D33A70 (Skate 2 has no 202) |
| 300 | WipeoutGround | `physics/wipeout_states/*` | Enter 0x82D3B5E8, Exit 0x82D3B990, Update 0x82D3B9C8, Fill 0x82D3EEE8 |
| 400 | GrindBoardslide | `physics/grind/*` | common ctor 0x82D3F210, Enter 0x82D3F318, Exit 0x82D3F430, Update 0x82D3F4E8, FillOut 0x82D40EA0 |
| 401 | GrindFiftyFifty | same | |
| 402 | GrindTipslide | same + `physics/grind/tip_nudge.rs` (0x82D421B0) | |
| 403 | GrindFiveO | same | |
| 404 | GrindBackslash | same | |
| 405 | GrindDarkslide | same (Skate 2 has no 405) | |
| 500 | BipedGround (on foot) | `physics/biped_ground.rs`, `physics/biped_ground/*`, `physics/offboard/*` | ctor 0x82D305D8, PreUpdate 0x82D30D30, Sync 0x82D324B0, Post 0x82D32AA0, Fill 0x82D32D38 / 0x82D785F8 |
| 501 | BipedAir | `physics/biped_air.rs`, `physics/biped_air/*`, `physics/offboard/air_*` | 0x82D2E5C0 / 0x82D2EF38 / 0x82D2FFF8 / 0x82D30300 / 0x82D2EF20 |
| 502 | OffBoardPushing (pushing/pulling a prop) | BipedGround path + `../../skate3_pushing.rs` + `graph_host/motion_moving_object.rs` | vtable 0x82327364; ctor 0x82d43b90, Reset 0x82d43d70, Enter 0x82d442d0, UpdateSpline 0x82d444a0, ComputePushNormal 0x82d448d8, Update 0x82D44A10, MoveObject 0x82d45318, ComputeTargetHandIK 0x82d45d30, Start/UpdateTargetHandIKBlend 0x82d46218 / 0x82d463d8, CalculateSkeletonPosition 0x82d46610, FillPhysOut 0x82d46fd0 |
| 503 | LandingOnDeck | `physics/landing_on_deck.rs`, `physics/landing_on_deck/*`, `physics/offboard/landing_deck*` | 0x82D4D418..0x82D4E100; Fill 0x82D4DFC8; Skeleton 0x82BDE428; manager 0x82D78B28..0x82D79E30 |
| 600 | HandPlant | `physics/handplant.rs`, `physics/handplant/*` | state vtable 0x823273CC; HandPlantManager 0x82D61040 |
| 601 | FootPlant | `physics/footplant.rs`, `physics/footplant/*` | vtable 0x82327400; manager 0x82D704B0 / 0x82D70E40 |
| 602 | Boneless (no-comply, boneless, fastplant) | `physics/boneless.rs` | vtable 0x82327434 |
| 700 | Sleeping | not reached | |
| 701 | Nonspecific (grind family) | `physics/grind/advance.rs`, `physics/grind/substate/nonspecific.rs` | 0x82D42DF0 |
| 702 | Teleporting | `physics/teleport_state.rs` | vtable 0x823272FC, slot 8 = 0x82D431F0 |

Skate 2 numbers differ for some states; see `04-shared-skate2-skate3.md`.

## The player tick skeleton

| Piece | Port | TU3 |
|---|---|---|
| Startup, PostInput, selection, publication | `physics/player_state/*` (`post_input.rs` 0x82DB5588, `pre_state.rs` 0x82DB6050, `selection.rs` 0x82D8ADE8, `transition.rs` 0x82DB8540, `publication.rs` 0x82DB7580..7790, `wipeout_output.rs` 0x82D3EEE8) | |
| Input phase | `physics/input_phase.rs`, `physics/player_input/*` (`services.rs` 0x82DB4048, `reset.rs` ProcessedPhysIn reset 0x82BF9EF0, `initial.rs` 0x82DB18A0, `pre_input.rs` 0x82D81610 / 0x82D2DDD0, `output_reset.rs` 0x82DE53F0 / 0x82DE4940, `ground_position.rs` CalculateGroundPosition 0x82C02840, `dynamic_normal.rs` 0x82C02388) | `core/player/input_phase*` |
| Exchange buffers | `physics.rs` `SimulationExchange` (commands, events, output snapshot) | `core/physics/phase.rs` |
| Settings | `physics/settings.rs` (stock skater vault values), `difficulty.rs`, `custom_difficulty.rs` | |

## Input and controls

- **Pad**: `input/controllers.rs`, `input/platform.rs`. TU3 pad conversion 0x8296D5F8 (0.25
  radial dead zone, then /0.7). Every sample goes through TU3's network pack/unpack quantization
  (`Sk8::PlayerInputData::Pack` 0x828E1F98 in PlayerUI::Update 0x82898920, Unpack 0x828E2260 in
  GameInputSK8::OnCommand 0x82695438): sticks `(int)((pos-neg)*127)`, triggers `(int)(v*255)`;
  `core/input/player_input_data.rs` `round_trip`. Measured on the running game (sticks 0.3..0.9 =
  9, 27, 45, 63, 81, 99, 117 /127).
- **Controls / intents**: `physics/controls.rs`, `physics/controls/object_settings.rs`
  (inputlistener_default 0x8259C650..C75C). TU3 Fill 0x825999F0 turn constants (0x822F9488:
  3.05, 2.35, 1/0.7, 1.39, -2.0833, 0.91; length 0.9); auto push gate (PhysOut+0x34 byte 58,
  not Actor+0x774 bit 11, |from forward| < 0.6, length > kOBLT_TrajEndLength).
- **Gestures**: `input/gesture_input.rs` (GameInputManager 0x82696030 / 0x826962D8 → listener
  0x8259B878 / 0x8259B9D0 → Fill 0x8259B1F0..B7D8), `input/gesture_mapping.rs`
  (GestureTrickMapping 0x82BA07F0, CreateTrickIntentFromGesture 0x82BA1C30..2228),
  `input/gesture_catalog.rs` (0x82BA10C0), `input/gesture_mapping_data.rs` (0x82B98F70, all 270
  map inserts). Bumper held → no gesture Start/Held intents (Fill helper 0x8259BF50).
- **Skate 2 side**: `session/skate3_input_capture.rs` (same hardware poll as Skate 2),
  `session/skate3_input_focus.rs` (Net::ResumeGame masks pauses), `physics/player/skate3_actor_input.rs`
  (Skate 2's GenerateActionGraphIntents 0x825FDA28 and Actor.Update 0x825F10F0 own requests:
  pins, force brake).

## Animation and graphs

- **Graph runtime**: `graph_runtime.rs` binds the stock on-disk graphs to the recovered TU3
  dynamic-controller runtime (`core/graph/`).
- **ActionGraph host**: `graph_host/action*.rs` (BoardAdjust 0x82BA2BA8 / 0x82BA2BC8, conditions,
  intent nodes).
- **MotionGraph host**: `graph_host/motion*.rs`, 58 files, each a TU3 behaviour with its factory,
  Begin, Update and End addresses. Examples: CharacterGesture 0x82BAA320 (instance 0x82BAADD0,
  select_hands 0x82BAB258), BodySpin 0x82BAB640, PowerSliding 0x82BB28E8, Shove 0x82BBCCF0 /
  0x82BBCDD0 / 0x82BBCDF0 / 0x82BBD540, ToggleBoard 0x82BA8FD8 / 0x82BA8FE8 (retrieval 0x82BA9310 /
  0x82BA9E20, drop 0x82BA9990), MatchAirTime 0x82BB9F48 / 0x82BBA070, BipedCadence 0x82BB9E78,
  MatchCadence 0x82BBA938 / 0x82BBAA00, AddRunoutAttribs 0x82BBA4A8 / 0x82BBA770,
  OffboardBodyTweakBlend 0x82BBAB98.., HippyJumpAntic 0x82BBCB50 / 0x82BBE900, AirDismounting
  0x82BB93E0 / 0x82BB94C0, FingerFlipOut 0x82BB3198 / 0x82BB7C40, SetTurning 0x82BB3830, TurnRemap
  0x82BB4248, SetBumpCoefficients 0x82BB0660, ScoringTrick 0x82BCC030 / 0x82BBFA88, SetTrickAttr
  0x82BC7848, MovingObject 0x82bb64b8 / 0x82bb6860 / 0x82bb6a20 (+ InitMovingObjects 0x82bb63d8,
  ClampAngle 0x82bb79d8), IsMovingObject activation 0x82ba6038, HandBusy 0x82BAB4D0 / 0x82BAB588,
  GroundSlopeType 0x82BA7BD8 / 0x82BA7E80, wipeout 0x82BBFF88 / 0x82BC01C0, push settings
  0x82B953E8..0x82B9584C, InitPush 0x82bad5b8, SetPushCoefs::Ramp 0x82bacf50.
- **Pose**: `skater_animation.rs` (graph → tree → pose → physics packet), `animation_pose.rs`,
  `animation_pose/authored_clips.rs`, `core/animation/*` (playback tree, blend, mirror, foot IK,
  posture, crouching, pumping channel, landing quality).
- **Animation ↔ physics**: `physics/animation_phase.rs` (physical output → stock graphs → next
  physical input; Actor phases 0x82592FD8 / 0x82593128 / 0x82593230 / 0x82593640; publication
  0x825937EC in `animation_phase_packet.rs`), `physics/animation_input.rs` (Skeleton attribute
  dispatcher), `physics/animation_feedback*.rs` (PhysOutConditioner_Animation, global 204),
  `physics/animation_grind.rs`, `physics/animated_skeleton.rs`.
- **Skate 2 side**: `animation/runtime/actor_skate3_gestures.rs`, `skate3_gesture_playback.rs`,
  `skate3_gesture_streams.rs` (gesture clips' hand/face streams in the body tree).

## Ground riding

- `physics/ground_phase.rs`, `physics/ground_runtime/*` (entry 0x82D37538, `input.rs` the inputs
  of UpdateSkateboard 0x82D38800, `pumping.rs` 0x82D37D34..7EAC, `settings.rs` toolkit copy
  0x82D94C10, `surface.rs` wheel surface vote 0x82C08818, `trajectory.rs` tail 0x82D749D0,
  `launch.rs` wall jump 0x82D33448 / 0x82BE33D0), `physics/riding_outputs.rs`.
- Engine: `core/riding/*`: speed model (UpdateSpeedModel 0x82C04F68, force tag 6), steering tilt
  (CalculateTilt 0x82D92440), heading (0x82D92CF8, 0x82DB5E10), speed wobble (0x82C03638 /
  0x82C0DBE8), push, pumping, braking, anti-flip, straighten, slide friction; `core/physics/board_*`
  (board runtime, step, toolkit, probes, world), truck targets (0x82C040F0), SetTruckDriveFrames
  (0x82C0B9C0), CreateDrives (0x82C0B770), Skateboard::FillPhysOut (0x82c02a80).
- The pop: Toolkit_CalcGroundJump 0x82D93618 with the `physics_jump` layout (adjust X 436, Z 432).

## Air

- `physics/air_phase.rs` (PhysicsAir), `physics/air_reckoning.rs` (Reckoning 0x82D8DBD8; classes
  physics_reckoning 7CE1B6B581709C6F, physics_bodyspin 05EC9CB03A5B2EEE), `physics/air_trajectory/*`
  (trajectory class 429A8F4385DD539D; collision leaf 0x82771D08, nearby triangles 0x82772028;
  UpdateTrajectorySelector 0x82d68800 sets processed 2468 bit 10 for KnownAir), `physics/known_air*`
  (ResetBodyFlip 0x82D8E9E0, BeginBodyFlip 0x82D8EAB0), `physics/skeleton_air.rs` (0x82BDDA10 /
  0x82BDE600), `physics/grind_trick.rs`, `physics/camera_tricks.rs`; `core/air/*`.

## Grind

- `physics/grind/*`: common update 0x82D3F4E8, substates 0x82D40AF8 (collision → pop → engagement
  → derived substate), six family FillOuts, AddWorldForce 0x82C03F98, SetVelocity 0x82C04168,
  involuntary (family substate 2), nonspecific 0x82D42DF0 via board helper 0x82D42F50.
- `physics/grind_chromosome*` (0x82DEE918 / 0x82DEF518 / 0x82DEE508; Skate 2 0x82E2E9E0 /
  0x82E2EBE8 / 0x82E2F9D0), `physics/grind_names*` (initializer 0x82DE6678, 384 records at
  0x82F92AC0), `physics/grind_camera.rs` (0x82DF0640; Skate 2 0x82E2D000),
  `physics/grind_materials.rs` (0x82D89DC0 / 0x82C090C0, Init 0x82C08C28),
  `physics/grind_air_settings.rs` (GrindAirAdjust 0x82D72B40), `physics/skeleton_grind_air.rs`
  (0x82BDCFB0).
- Acquisition: `physics/player_input/grind*` (manager 0x82D8A828 / 0x82D8AB08; line batches
  0x82C20728 / 0x82E0AAD0).
- World: `grind_world/*` (loose octree 0x82AD6830 / 0x82476158; spline 0x82C1E568, 0x82C1E6C0,
  0x82C1EEF0, 0x82C1F098), fed by `../../skate3_grinds.rs`.
- Graph: `graph_host/motion_grind*` (Begin 0x82BB0A30).

## Off board

- **BipedGround** (`physics/biped_ground*`): feet (0x82D77330 / 0x82D775D8), effective root
  (0x82BE3650), grab decision (Sync 0x82D324B0), grab runtime (PlayerGrabSpline 0x82D73FC0 /
  0x82D74270 / 0x82D740F8 / 0x82D749D0), carried board (0x82D31D1C..DB0).
- **BipedAir** (`physics/biped_air*`, `physics/offboard/air_*`): selector, trajectory, feet,
  Skeleton 0x82BDDD70.
- **Shared off-board owners** (`physics/offboard/*`, 55 files): SkateboardController (0x82DB6150,
  0x82D76D20), hand drives (0x82D74FD8 / 0x82D751D8; the factory 0x82AE64F8 reverses argument
  bodies), board possession and effects, Biped Skeleton update (0x82BDE060), contact toolkit
  (0x82770B40 / 0x82771D08 traversal, 0x82770E00 / 0x82772028 nearby), ground query (0x82764AB0 /
  0x82770650 / 0x82C1EAD8), LandingOnDeckManager (0x82D78B28..0x82D79E30), settings (0x82D7B630;
  metrics 0x82D7AFD8 / 0x82D16680), pose adjust (0x82BD8F10), post-physics (slot40 0x82DB6DCC),
  collision check (0x82D90148), grab scene.
- **Pushing props (502)**: `../../skate3_pushing.rs` runs Skate 2's pushing core (the same math:
  TU3 MoveObject = Skate 2 MoveObject 0x82d923d0 at default cvars) on Skate 3's pushing class
  (0x3eda5b140604613d) from the Skate 2 player's vault, sends the force through the world grab
  queue (TU3 DynamicObject::ApplyForce 0x82c52b30 → Body::AddForce), gives the skater the follow
  (TU3 Update 0x82D44A10; Skeleton::UpdateOffBoardPushing 0x82BDF268) and hand IK targets
  (SetLeft/RightHandIKTargetPosition 0x82bd9728 / 0x82bd97d0, within 0.65 of the shoulder).
- **Shove**: `graph_host/motion_shove.rs`, `physics/shove_probe.rs` (probe 0x82d7a440 /
  0x82d7a670; ProcessOutput 0x82db7fc0: velocity = 7·normalize(target − system) + (P400 − P+0x720));
  hit type 1 (skater) / 2 (pedestrian) → 0x8275F0D8 → vslot +8 0x82db3c50 → Processed+0x740..0x758 →
  WipeoutGround::Enter 0x82d3b838 adds it to all 23 parts.

## Plants and boneless

- HandPlant (`physics/handplant*`): PositionSelector 0x82D63D40, velocity window 0x82D64420, edge
  clipping 0x82D651E0, IK 0x82D633B0 + Skeleton 0x82BD9728 / 0x82BD97D0, launch 0x82D613B0,
  trajectory selector 0x82D66318 / 0x82D66948, COM 0x82D625C0, rotation 0x82D61B60 / 0x82BD3150.
- FootPlant (`physics/footplant*`): FootPlantManager ctor 0x82DB217C, Reset 0x82D6F440, update
  0x82D6FD60 → 0x82D6F5F0 → 0x82D6F910 (from 0x82D36728), Footplant 0x82D70070.
- `physics/plant_skeleton.rs`: anchored foot 0x82BDEC48, plant COM 0x82BDF090.
- `physics/boneless.rs` (602): AnimSkateboard + FootJump launch.

## Wipeout and ragdoll

- Requests (Ground/Air): `physics/wipeout.rs` (0x82D8F9E0 / 0x82D90358), settings from
  physics_wipeout (Globals264 → 0x8289D270, class EB844CFDBF928EFD).
- WipeoutGround (`physics/wipeout_states/*`): prediction 0x82D3C02C..C288, contact corrections
  0x82D3EC08, Skeleton::UpdateWipeout 0x82BDFA88, ragdoll selector 0x82D913C0 (requests 8..11) and
  0x82BE7280, normal 0x82BE58A8, ragdoll 0x82BE59A8 / 0x82BE6D60, per-part drives 0x82BE7860 /
  0x82BEB0C8 / 0x82BEB790.
- Board contact classification by group (Skateboard UpdatePostPhysics 0x82C07D20 / 0x82C07F54;
  `core/physics/board_ground.rs` `update_with_groups`): bits 26..30, see `05-what-we-fixed.md`.
- Recovery and the online turn bail limit: vendor recovery + `wipeout_states/update.rs`.

## Skeleton, collision and solve

- `physics/skater.rs` (the single skater; bodies from the animator's first pose),
  `physics/skeleton_body*.rs` (collision 0x82BE7860 / 0x82BE7280, BoneDrives 0x82BEC4A0 /
  0x82BEC7B0), `physics/skeleton_colliders.rs`, `physics/skeleton_input_runtime.rs`
  (Skeleton::ProcessData 0x82BD8918), `physics/skeleton_input_teleport.rs` (0x82BE3508,
  0x82BDF7C0), `physics/skeleton_output.rs` (0x82DB6698 / 0x82DB95E8),
  `physics/skeleton_feedback.rs`, `physics/skeleton_controller.rs` (requests 0x82D911A8,
  ascending/falling 0x82D91660), `physics/foot_ik*.rs` (StartSkeletonLineTests 0x82DB63C0 /
  0x82DB6580), `physics/foot_physical_output.rs` (Skeleton::FillPhysOut 0x82BE2138),
  `physics/render_pose.rs`.
- Solve: `physics/solve.rs`, `physics/solve/assembly_contacts.rs` (SkaterSkaterCollisionPipeline
  0x8276556C; board before skeleton, 0x82DB18A0), `physics/solid_contacts.rs` (host convex solids),
  `physics/collision_groups.rs` (assembly then part culling 0x8277AF90 / 0x8277A6C0; culling table
  0x82765EF0), `physics/world_proxies.rs`. Engine: `core/physics/*` (rigid bodies, contacts,
  drives, joints, solver, triangle queries/sweeps).

## Teleport, respawn, breadcrumbs

- `physics/input_teleport.rs` (TeleportPlayerNow 0x82DB8998), `physics/teleport_state.rs` (Actor
  reset callback 0x82592518), `physics/respawn.rs` over `core/player/respawn.rs` (Update
  0x82BFB1E0, GetSuggestedRespawnPosition 0x82BFC828, Pop 0x82BFC9B0, IsPositionOk 0x82BFAFF0,
  GetBreadCrumb 0x82BFC1E8, last resort 0x82BFC718, physics constraints 0x82BFB3F8 with surface
  table 0x82BFB698, network 0x82BFBB48, grind edge 0x82BFB6F8, ground probe 0x82BFBC18, best score
  0x82BFC550, living-world constraints 0x82BFB928), `../player/skate3_spawn.rs` (Skate 2
  AdjustSpawnPosition 0x825F1B8C +0.15 m, then TU3 board reset 0x82DB8A10 +0.1 m).

## Camera

- `camera/*`: stock camera graph (conditions 0x82DF4A90..0x82DF61DC), subject 0x82DF69C0,
  settings 0x8252E288 / 0x8252EDD0, shots 0x82CA4660, trajectory requests 0x82DF9270 / 0x82DF9338 /
  0x82DF9340, world query leaf 0x82771D08, shakes 0x82E05DC0; `physics/camera_output.rs` (reset
  0x82DE53F0, producers 0x82DB6EC0 / 0x82D3A388); `physics/clock.rs` (OnRequestSetSimRate
  0x82857EC8 sets a wall timer, never the integration step); `core/camera/*`.

## Scoring, Hall of Meat, audio

Skate 2's systems, run on Skate 3 data with TU3 branches: see `04-shared-skate2-skate3.md`,
`20-scoring-audit.md`, `21-hall-of-meat.md`. In the module only `scoring_runtime/rotation.rs`
(RotationTracker 0x82DAC880 / 0x82DAC9D0 as UpdateSpinMetric 0x82DA8BE0 drives it).

## Not TU3: PC additions in the module

Port them only if wanted; they are not Skate 3 behaviour.

- `physics/climbing/*`: hybrid ledge hang/mantle (authored clips, two-bone IK).
- `modding/*`: dual-world moving queries, temporary joint overrides.
- Puddle physics (`crate::ui::puddle_physics` calls in `frame.rs` / `solve.rs`): rain makes wheels
  slip and can bail.
- `physics/startup_check.rs`, `physics/dev_trace*.rs`, `physics/offboard_audit_trace.rs`,
  `crash_context.rs`: diagnostics.
