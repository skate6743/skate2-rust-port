# What Skate 2 and Skate 3 share, and what differs

Skate 3 (TU3) is built on Skate 2's engine. Most of the game around the skater, and much of the
skater's own code, is the same in both executables, sometimes byte for byte, sometimes with
Skate 3 branches added. This port keeps one copy of each shared system and gives it the TU3
branches, instead of running two of them. A porter moving the physics elsewhere needs to know
which parts came from where.

## 1. Skate 2 systems the Skate 3 physics uses unchanged

| System | Where in the port | How Skate 3 physics uses it |
|---|---|---|
| World streaming and static collision | `static_ground_pool`, city streaming | its resident triangles become the Skate 3 `BoardWorld` (`physics/skate3_terrain.rs`) |
| Moving world bodies (props/DMOs, vehicles, pedestrians, other skaters) | Skate 2 body pool and solver | solved by Skate 2 first, then joined to the Skate 3 solve as finite-mass shadows; reactions come back (`physics/assembly/skate3_world.rs`) |
| Grind and grab splines | LivingWorld DMOs, vehicles, `skate2_formats::pegasus::Grab` | same tGrab family in both games: a direct field copy into the Skate 3 records (`physics/skate3_grinds.rs`, `physics/skate3_grabs.rs`) |
| World grab queue | `player/interaction/physical_player_grab_world.rs` (`GrabMessageKind::ApplyForce`) | prop pushing's forces |
| AI skaters, remote/online skaters | Skate 2 physics | they stay on Skate 2 physics; the Skate 3 skater collides with them through the shadows |
| Actor | `actors/*` | intents, pins, force brake, hand/face streams, teleports, breadcrumb consumers |
| Camera transport, HUD, career, challenges, online netcode, replays | everywhere | they read Skate 2's PhysOut and skeleton, which the Skate 3 player publishes into |

## 2. Skate 2 systems with TU3 branches (shared code, Skate 3 behaviour)

These run in both modes. Their Skate 3 parts sit behind a Skate 3 flag
(`skate3_enabled()`, `use_skate3`, a `skate3_physics` PhysOut flag, or a linked-data check).

| System | Files | What differs in TU3 |
|---|---|---|
| **Scoring** (ScoreModule) | `physics/player/output/conditioning/scoring/*` (`attributes.rs`, `skate3_catalog.rs`, collectors, holder, module) | Skate 3 values served at Skate 2 layout offsets (`ScoringAttributes`; never change these to TU3 offsets); holder byte +0x1C5D (locked trajectory, PhysOut_Air +452) gates 14 functions; manuals have no speed condition; hippy 180s from landing half turns. Per function: `20-scoring-audit.md` |
| **Skate 3 names in scoring/HUD** | `scoring/skate3_catalog.rs` | TU3 trick names resolved into Skate 2's pipeline; audio trick type/finish/scrape from the Skate 3 record (FillSkaterData 0x827A1F80..2014) |
| **Hall of Meat** | `scoring/skate3_hom.rs`, `scoring/wipeout_skate3.rs` | WipeoutScorer::Update 0x82DAE4F0: run-up scoring (air, drop, rotation), decaying bone hits, body-tweak spins, wipeout response; TU3 moved shared fields and added curves (translated by field hash). `21-hall-of-meat.md` |
| **Catch / landing audio** | `skate2-audio/src/player/contacts.rs`, `offboard.rs` | PlayFootCatch 0x824b9268 always uses Skate 3's aud_general/wheels thresholds and cues (0x824c0b90) through its sense-of-speed effect; foot-count types (0x824b9508) differ from Skate 2's |
| **Darkcatch effect buses** | `skate2-audio/src/global_effects.rs` | 8 voices (DistortionClip → IIR2 → IIR2), built only in Skate 3 mode; catch/land splices routed by the `aud_senseofspeed_default` index (0x8B0E030799CBDD00) |
| **Flip trick sounds** | `skate2-audio/src/player/tricks.rs` | Skate 3's own `Class_Flips` class (28 members, ctor 0x824afad8) and `Sk8_Air_Flip_Tricks.abk` from the linked data |
| **Reverb** | `skate2-audio/src/reverb.rs`, `world/emitter_system.rs` | SFXObj_Reverb::UpdateParams 0x824de548: emitter volume first, else Skate 3's painted layer 10, else default 0xA2782D75A971CC8C; district emitter files are per-district (not ported, city volumes off in districts) |
| **Breadcrumbs** | `actors/actor_breadcrumb_*`, `physics/modes/skate3/physics/respawn.rs` | TU3's history is the one store in Skate 3 mode, mirrored to Skate 2's manager |
| **Pedestrians** | `living_world/pedestrians/pedestrian_collision_shove.rs`, contacts | the Skate 3 solver's contacts reach the pedestrians' contact lists; TU3 shove |
| **Collision culling** | `physics/modes/skate3/physics/collision_groups.rs`, `simulation_island_culling_table.rs` | group numbering (below) |
| **PhysOut publication** | `physics/player/output/physical_player_phys_out_skate3.rs` | the Skate 3 frame filling Skate 2's PhysOut (state IDs below, filtered grind stores, frames-since-reset, state update count, requested teleport flag) |
| **Camera** | `camera/frame/camera_skate3.rs`, camera startup fields | the Skate 3 camera's field of view is restored to Skate 2's on switching back |

## 3. Translations

### Physical state IDs

Most IDs mean the same in both games. The off-board ones and two Skate 3-only states do not.

| Meaning | Skate 3 | Skate 2 |
|---|---|---|
| On foot (ground) | 500 BipedGround | 500 (plus 504 Locomotion, part of BipedGround in Skate 3) |
| On foot, in the air | 501 BipedAir | 502 (plus 503 Jump, same air family) |
| Pushing/pulling a prop | 502 OffBoardPushing | 501 |
| Landing on the deck | 503 LandingOnDeck | 505 |
| Grind trick (secondary air) | 202 | none: published as 200 |
| Darkslide | 405 | none: published as 400 |
| everything else (0, 100–105, 200, 201, 300, 400–404, 600–602, 700–702) | same | same |

Skate 3 → Skate 2: `state_id` in `physical_player_phys_out_skate3.rs`. Skate 2 → Skate 3 (an
off-board skater switching to Skate 3 physics): `try_reconcile_skate3` in
`session/selected_player_skate3.rs`. The filtered categories (Invalid, Ground, Air, Grind,
Wipeout, Teleport, OffBoard, OffBoardAir) map one to one.

### Collision groups

TU3 has no camera or phantom groups and adds its own (15, 16, 19, 20), so its living-world
groups sit two lower. The TU3 IDB's `CollisionGroup` enum was imported from Skate 2 and is wrong.
From the body-creation calls:

| Body | Skate 2 | TU3 |
|---|---|---|
| Pedestrian world / skater bodies | 11 / 13 (InitCollisionBodies 0x82e99df0) | 9 / 11 (0x82e268c8) |
| Vehicle | 10 (0x82d28eb8) | 8 (0x82c37458) |
| Dynamic object, light / heavy (≥ 5.5 kg) | 16 / 14 (0x82d3aa28) | 14 / 12 (0x82c4e7b8) |

Map Skate 2 → TU3 (`skate3_group` in `physics/assembly/skate3_world.rs`): 0–4, 7, 17, 18 same;
6 → 5; 10..16 → g − 2; 5 (camera), 8 and 9 (phantoms) → 1. Under this map the two
`InitCullingTable` pair tables (Skate 2 0x827b8c10, TU3 0x82765ef0) agree on every pair except
group 17's, which TU3 moved from the board to the ragdoll.

### Other values

| Value | Skate 2 | Skate 3 |
|---|---|---|
| Truck tightness / wheel hardness, new local skater | 0.3 / 0.7 (kTruckTightnessDefault) | 0.7 / 0.7 (CASPlayerInitializer::InitializePlayer 0x8253D4BC; Actor reset 0x82593640 reads PlayersBackEnd +160/+164 = CAC record +0x2004/+0x2008) |
| Solver passes | 50 single player, 25 online (`kMaxSinglePlayerIterations` / `kMaxOnlineIterations`) | measured native: world 25, skater 50; online 25 (Step_UpdatePrePhysics1 0x82763E00) |
| ICE camera elements | 146 (0x82a4e8b8) | 148 (0x82903840): adds LENS_USE_DYNAMIC (102) and RAWFOCUS_RELATIVE (106); later ids shift |
| Tick | 1/60 s | 1/60 s |

## 4. Identical in both games (checked)

- **Data**: `physics_steering`, `physics_heading`, `physicstrucks`, `physicstrucks_drives`,
  `physicswheels` and `physicsdeck` are byte-identical between Skate 2's `skater.vlt` and Skate 3's
  `skatercollections.vlt`. `physics_steering` layout: SteeringVsSpeed 0, SteeringVsInput 16,
  MaxSpeed 32, TiltBlending 33, TightTrucks 34, PushMin 35, PushInc 36, PushDec 37,
  HardTurnIncrease 38, General 39, Damping 40.
- **Functions found equal to Skate 2's** (TU3 address, behaviour the same):
  - Pad conversion 0x8296D5F8; Fill turn constants; FilterMotionGraphIntent 0x82BB12C8 / 0x82BB17D0.
  - CalculateTilt 0x82D92440 and its toolkit copy 0x82D94C10; wobble; truck targets; SetTruckDriveFrames.
  - UpdateSkateboard order 0x82D38800; heading.
  - MoveObject (= Skate 2 0x82d923d0 at default cvars).
  - IsMovingObject activation 0x82ba6038 (= Skate 2 0x82c9b808).
  - CheckForJumpsAndLands 0x824b90d8, TakeOffGround 0x824b9cc8, GetSpliceIndexForOllie 0x824b9ad8.
  - CheckForBoardFlips 0x824cbfb8, UpdateBoardFlips 0x824cc7d8.
  - TestForLWRespawnContraints 0x82BFB928; GetSceneContext 0x82673c60; PointGraph evaluation 0x82481E10.
  - VerbInit 0x824de140 (same 37 offsets); DoSimSubmit painted layers 8/7/10.
  - Body::SetVelocity 0x82d9c930.
- **Wheel material ids** via `(tag >> 7) & 31`: the same enum (smooth, rough, slow, slippery,
  very slow).

## 5. Traps

- **IDB names**: the TU3 IDB carries many Skate 2 names that are wrong at TU3 offsets.
  - `GlobalAttributeCollection` members: verify by the TU3 initializer 0x8289D5C8, e.g. +0x6C is
    aud_splicer 0xF1647BFB782BE97F and +0x8C is class 0x42AFE160E647167C "default".
  - `CollisionGroup`: use the table above.
  - SkateboardBody: `mBack`/`mFront` are swapped against the raw offsets.

  Trust offsets and initializers, not field names.
- **Vault layouts**: the shared Skate 2 code reads Skate 3 vault values at Skate 2 layout offsets
  on purpose (`ScoringAttributes`, `skate3_catalog`; Hall of Meat by field hash). Changing an
  offset to its TU3 value reads a different field. New Skate 3-only fields are read by key.
- **Mirrored stance**: on board with a mirrored stance the effective root negates X and Z
  (`FrameOutput::effective_root_transform`); markers and checkpoints go through the same Actor
  teleport (0x825926f8, no adjust) in TU3.
