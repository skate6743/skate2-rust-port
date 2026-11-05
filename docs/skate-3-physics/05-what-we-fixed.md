# What this port fixed and added on top of the engine

Each entry: what the engine or Smokey's mod did, the TU3 evidence, the fix and where it lives, and
how it was checked. "Confirmed" = the change was played and confirmed in game; "measured" =
compared numerically with the running TU3 game (see `07-verifying-against-tu3.md`); "scripted" =
checked in a scripted run of the port. TU3 addresses throughout.

## Integration (making it part of the game instead of an overlay)

| What | Before | Now |
|---|---|---|
| Assets | an embedded pack in the exe (Skate 3 files from an old pipeline) | read from the player's linked Skate 3 data, folder or ISO (`skate_data::asset_io` provider, `skate3_player::assets_root`); nothing embedded |
| Input | raw pad captured beside the game's input, bypassing its settings | the same hardware poll as Skate 2 after the stick settings; camera invert and angle from the profile; Actor policy for pins and force brake (`session/skate3_input_*.rs`, `physics/player/skate3_actor_input.rs`) |
| Cost in Skate 2 mode | an always-on trace journal (114 spans/events per tick, 16 MB ring, disk thread) | nothing runs in Skate 2 mode; traces only with `SKATE2_TRACE_*` |
| Dependencies | rapier3d, mlua, gltf, zip, sockets, Bevy in the core crate | `glam`, `serde`, `serde_json` and the three vendored crates |
| Settings | a feature-toggle mix (darkcatch/footplants/offboard on the Skate 2 solver) | one mode: Physics Settings → Skate 2 \| Skate 3, Difficulty |
| Grindable props | only the mod's Object Dropper props fed the Skate 3 grind world | every movable object with grind splines (`DmoObjectMan::grindable_objects`) |
| Streamed ground refresh | decoded and rebuilt on the simulation thread (12–51 ms decode + 10–39 ms build every few hundred ms while streaming: tick stalls seen as the skater "flickering back" on steep ramps) | built on a background thread, swapped in at a tick start; district arrival decode spread over cores (`skate3_player.rs`, `static_ground_pool`). The rest of the vert flicker was Skate 2-wide: the presentation restore snapped the camera on a light cut (camera brushing a vert wall) half a tick ahead of the blended skater (DoSimRestore, Skate 2 0x827f60e0); small cuts now blend. Confirmed |
| Mode switching | | hand-over in both directions with velocities kept (`skate3_exit.rs`, `PlayerEntry`), Party Play Actor changes, waiting for resident collision instead of failing |

## Input and steering

| What | TU3 evidence | Fix | Check |
|---|---|---|---|
| Stick values ~0.8% stronger than native | TU3 sends the local pad through `PlayerInputData::Pack` 0x828E1F98 / Unpack 0x828E2260 (sticks `(int)((pos-neg)*127)`, triggers `*255`) even offline | `core/input/player_input_data.rs` `round_trip`, applied in `input/controllers.rs` | measured: stick 0.3..0.9 → 9..117/127 as native; diagonal turn/hard-turn equal to native in every tested direction |
| Turning too sensitive at speed | CalculateTilt 0x82D92440 = port; the input was wrong: a new Skate 3 skater has trucks/wheels 0.7/0.7 (CASPlayerInitializer::InitializePlayer 0x8253D4BC, Actor reset 0x82593640 reads PlayersBackEnd +160/+164), the port fed Skate 2's 0.3 (Loose) | `PcSkate3Style` equipment leaf (`SK82_PC::Skate3Equipment`, default 0.7/0.7), Edit Skater sliders, `Skate3Player::configure_equipment` | measured: turn onset 10° at 283 ms vs native 287–298 ms; 98–110 °/s from 4 to 9.4 m/s in both |
| Mongo pushes weak | foot IK contact (0x82BEFC80) reads SkeletonCollision+0x480, the most vertical toe contact normal's y (copied by UpdateIKData 0x82BDC9E8); the engine read the hips height | `FootIk.toe_contact_normal` from the collision feedback's foot normal; push style published once per stroke (Update5 0x82DB7B68..7C14, PhysOut_Animation mPushMongo/mPushRegular) for sounds and stats | scripted (mongo 3.69/6.29 vs regular 4.52/7.08 m/s, shorter contact window, same cap) |
| Gestures blocked under force brake (online 3-2-1) | CharacterGesture 0x82BAA794..7E8 refuses a start under ForceBrake only inside front-end states 55/27 (0x82594AA8) | `gesture_force_brake_bypass` from the Skate 3 player's input focus | scripted (pin diagnostic) |
| RB during a held gesture only tilted the arms | Fill helper 0x8259BF50 (bumper → no gesture intents); Shove::Update 0x82bbcdf0 starts SkitchAntic; CharacterGesture stacks it above the gesture (0x82D1CF70) | Skate 3 gestures play inside the Skate 3 graph's channel stack (`set_graph_gestures`, CharacterGesture 0x82BAA848) | confirmed |
| Hands showed Skate 2 fists (after RB + tricks, then standing) | the hand buffer started from the Skate 2 Actor's own body-tree cues, and a stream missing from the Skate 3 result kept Skate 2's tokens; TU3's off-board stand hand streams are authored with 0 keys and fall back to pose 0 (Skate 3 palette index 61) | Skate 3 hand pose fallback; the 123-pose palette (Skate 2's 61 + Skate 3's 62) | scripted (standing hands blend Skate 3 poses 61–65) |
| Gesture fingers flat (regression of the above) | gesture clips' hand/face streams were only in the overlay's clip map | gesture clips added to the body tree (`animation/runtime/skate3_gesture_streams.rs`) | scripted |

## Pop, air and tricks

| What | TU3 evidence | Fix | Check |
|---|---|---|---|
| Hippy jump launched the board | Toolkit_CalcGroundJump 0x82D93618 has no hippy branch and never reads HippyJumpMin/MaxHeight; hippy only re-applies the animation's jump strength (+2624) in Skeleton::ProcessAnimAttributes | board launch removed | |
| Big air at speed off a vert lip gave a small jump (nollie at vy 17.3 → first air tick deck vy 8.9) | TU3 TrajectorySelector::Update 0x82D68800 completes only when the physics predictor query is ready, i.e. the NEXT tick; the engine completed it in the launch tick and the all-miss latch seeded the COM trajectory from a stale finite difference (Processed+608) | `AirTrajectoryRuntime` tick/submitted-tick bookkeeping, `trajectory.begin_tick` in `physics/frame.rs` prepare | confirmed |
| Scoring of air while the trajectory is locked | ScoreModule::Update 0x82DA3520 copies PhysOut_Air +452 into holder +0x1C5D; 14 TU3 functions gate on it (AirCollector 0x82DA8A70 etc.) | `skate3_air_reckoning` in scoring; every gate ported | per function in `20-scoring-audit.md` |
| Manuals: Skate 2's speed gates | TU3 UpdateManual 0x82daa8e0 has no speed condition; travel along the board velocity past 0.005 m | ported | `20-scoring-audit.md` |
| Hippy 180s | OffboardCollector::UpdateHippyJumps 0x82dabfb0 counts the physics' landing half turns (PhysOut_Ground +0x138) | ported | `20-scoring-audit.md` |
| Hall of Meat | WipeoutScorer::Update 0x82DAE4F0 (run-up, bone-hit decay, body-tweak spins, response) | `scoring/skate3_hom.rs`, `wipeout_skate3.rs` | `21-hall-of-meat.md` |

## Grinds

| What | TU3 evidence | Fix | Check |
|---|---|---|---|
| Grinds in Skate 3 physics never scored (career "grind the ledge" goals failed) | Skate 2's Physics conditioner (0x82e2f970) is dormant under Skate 3 physics; the Skate 3 publication set only the filtered state, never the filtered grind stores (IsGrinding, GrindType, names, score, crouch, pathed/local GUIDs) | `physical_player_phys_out_skate3.rs` stores them as the conditioner does, held through the filtered Grind tail | scripted (career) |
| Grind trick names | initializer 0x82DE6678 (384 records at 0x82F92AC0) | `physics/grind_names/*` checked entry for entry | |

## Off board, props and shoves

| What | TU3 evidence | Fix | Check |
|---|---|---|---|
| Props could not be grabbed and moved (career "move the ramp" goals) | CalcSuggestedState 0x82D8ADE8 picks 502 on ProcessedPhysIn 2476 bit 21, copied from PhysOut+0x48 byte 0x130, which Pushing's FillPhysOut 0x82d46fd0 writes from its own grab bit; MoveObject moves the prop by forces (ApplyForce 0x82c52b30) through the grab queue (GrabSplineQueryManager +0x24 0x8275ff00) | `physics/skate3_pushing.rs` (state 502 on the host: Enter/UpdateSpline/MoveObject/hand IK/follow), grab-scene registry from DMO and vehicle grab splines (`skate3_grabs.rs`), grab flag from the driver | confirmed |
| Prop-push legs restarted mid-stride | IsMovingObject (activation 0x82ba6038 = Skate 2 0x82c9b808) tests \|movement\| > 0.1 and atan2(−x, z) within the arc; the engine ignored the arc | `graph_host/motion_gameplay_conditions.rs`, `graph_host/motion_moving_object.rs` (MovingObject 0x82bb64b8 / 0x82bb6860 / 0x82bb6a20, ClampAngle 0x82bb79d8, AngleAdjustByAnimationEndLessThan 0x82ba7598) | confirmed |
| Shove did nothing to pedestrians or skaters | probe 0x82d7a440 from PhysicalPlayerHiLOD 0x82db3ce8 (state flag 86); hit in ProcessOutput 0x82db7e38; velocity 0x82db7fc0 = 7·normalize(target − system) + (P400 − P+0x720); 0x8275F0D8 routes it; struck skater: WipeoutGround::Enter 0x82d3b838 adds it to all 23 parts | `physics/shove_probe.rs`, host query and response in `selected_player_tick.rs`, `pedestrian_collision_shove.rs`; other skaters (on Skate 2 physics) take a LivingWorld wipeout along the shove | scripted (31 knockdowns) |
| Interactable object | CalculateInteractableObjectQueryResults 0x82760368 | grab runtime publishes object 2464 / 2480 bit 20 into Skate 2's interactable id | |

## Collisions and bails

| What | TU3 evidence | Fix | Check |
|---|---|---|---|
| Wrong collision groups (pedestrian world bodies hit the skater; vehicles and light props missed) | the TU3 group numbering (table in `04-shared-skate2-skate3.md`); culling 0x82765ef0 | `skate3_group` map, assembly and part culling (0x8277AF90 / 0x8277A6C0) in `solid_contacts.rs` / `collision_groups.rs` | scripted |
| Pedestrians did not react to the Skate 3 skater | TU3's island records the skater's contacts on each pedestrian body | the Skate 3 solver's host contacts go to the pedestrians' contact lists | scripted |
| Bailing too easily bumping AI skaters | Skateboard UpdatePostPhysics 0x82C07D20 classifies each contact by the other body's group (0x82C07F54): board end-on (\|cos\| > 0.92) → bit 30, light DMO → bit 29 (+ normal invalid), 12 → 28, 11 → 27; the ground check 0x82D8F9E0 scales limits (DeckAccPlayerScalar 2.0, WipeoutGroundLightDMOScalar 3.0); contacts between two bodies of one owner are never reported (0x82768418) | `core/physics/board_ground.rs` `update_with_groups`, own-skeleton filter (`set_owned_attached` in `solve.rs`) | measured in play: end-on hit at 9.17 m/s, no bail (before: bail) |
| Water spots and special surfaces | GroundPipeline 0x8277BC58 excludes groups 7 and 16; ProcessOutput's special-surface writer 0x82DB8120 | ported | |
| Online turn wipeout limit | Skate 2's limit was never read by the engine's recovery | `wipeout_states/update.rs` resets after it (Spot Battle 4 s, S.K.A.T.E. 5 s) | scripted |

## Respawn and breadcrumbs

| What | TU3 evidence | Fix | Check |
|---|---|---|---|
| A-press respawn "in the middle of nowhere" | the engine's history was created at load and never restarted for a new Actor (SkaterManager::OnCommand case 3); Skate 2's manager got no crumbs (its Update 0x82ce2230 gates on mFramesSinceReset ≥ 120 and mStateUpdateCount, never published) | TU3 history (Update 0x82BFB1E0, Pop 0x82BFC9B0, …) is the Actor's one store, restarted at each host spawn, answered by the host's network/living-world services, mirrored to Skate 2's manager; PhysOut gets state update count, frames since reset, requested teleport (flag 69) | confirmed (airborne A reset) |
| Off-board reset height | Skate 2 AdjustSpawnPosition 0x825F1B8C adds 0.15 m, then TU3 board reset 0x82DB8A10 adds 0.1 m | `physics/player/skate3_spawn.rs`: a standing biped starts on its actual support | |

## Audio (shared Skate 2 audio, Skate 3 branches)

| What | TU3 evidence | Fix | Check |
|---|---|---|---|
| Darkcatch catches played Skate 2's footcatch | PlayFootCatch 0x824b9268 always takes Skate 3's aud_general/wheels thresholds and cue (0x824c0b90), bank 0, through its sense-of-speed effect; foot types from 0x824b9508 | `play_skate3_foot_catch`, `skate3_physics` PhysOut flag | scripted (cues 1112–1162 by speed) |
| Catch/land effects dry | 8 GlobalEffects voices (DistortionClip → IIR2 → IIR2) activated per catch with randomized attributes (0x824916E8, 0x82491DF0) | `global_effects.rs` `skate3_effect`, routed by the `aud_senseofspeed_default` index | scripted |
| Darkcatch start played a heelflip pop | Skate 3's own `Class_Flips` (28 members, ctor 0x824afad8; member 16 = not rewinding) and `Sk8_Air_Flip_Tricks.abk` | Skate 3's csi/abk loaded from the linked data for Skate 3 tricks | scripted |
| Darkcatch scrape | PlayFootScrapeSound 0x824b97c8 (aud_contacts feetscraping keys, inverted latch) | ported | |
| Jumps on Skate 3 maps echoed like a tunnel | SFXObj_Reverb::UpdateParams 0x824de548: emitter volume first, then Skate 3's painted layer 10, else 0xA2782D75A971CC8C | Skate 3 layer and default key; Skate 2 city emitter volumes off in districts | confirmed (first part); district emitters not ported |

## Camera and time

| What | TU3 evidence | Fix | Check |
|---|---|---|---|
| Camera stayed "further away" after switching back to Skate 2 | Skate 2's path never writes the frame field of view | the replaced value is kept and restored | |
| Slow motion | OnRequestSetSimRate 0x82857EC8 sets a wall-clock timer, never the integration step | camera slow motion only through Skate 2's SetSimRate, only with the airtime option, never online | |
| Online physics | Island::Step_UpdatePrePhysics1 0x82763E00: 25 passes off the single-player island | `set_online` | |

## Checked and found equal (no change needed)

Coasting speed model (UpdateSpeedModel 0x82C04F68, term for term), pop (Toolkit_CalcGroundJump),
turning chain (Fill, FilterMotionGraphIntent, CalculateTilt, wobble, truck targets, drives,
heading), pushing (InitPush, SetPushCoefs::Ramp, BreakOutOfPush 0x82BA6258), running and sprint
speeds (measured 6.0 / ~9.4 m/s, as native), caveman (PhysicsAir enter 0x82D34388 seed), mongo
coefficients (0x82BAD340), PointGraph evaluation, ICE camera scene context, grind contact per
family (Update_Contact 0x82d41848 / 0x82d41d70 / 0x82d42030 / 0x82d42808 / 0x82d429f8: friction
triples, align 800/3000; ReorientSkateboard 0x82d40890 noise 0.09), wallride manual spin
(TargetAngularDisplacement 0x82C07000, AttachIntent 0x82BB5AE0), skater lighting (SkaterPresEntity
light probes, same 160-byte record in both games).

Do not "smooth" these, they are native: the pushing root moves in ~1 cm steps (TU3 minimum delta,
kSimilarThresh 1e-4 against |d|², 0x82d46f0c); TU3's CalculateSkeletonPosition adds no collision
offset (Skate 2's does; Skate 3's value is 0 anyway); Bumped has an unconditional transition to
Riding.Turning.Push in TU3's graph.
