# One tick, end to end

Physics runs at a fixed 1/60 s on the game's simulation thread. One Skate 2 frame advances the
Skate 3 player exactly once (no second accumulator inside the player: the game's simulation timer
owns it, as it owns every other player, car and score).

Addresses below: Skate 2 functions for the host frame, TU3 functions for the Skate 3 tick.

## A. The host frame (`session/selected_player_tick.rs`)

| # | Step | Skate 2 function | Skate 3 work |
|---|---|---|---|
| 1 | Input step (queued or immediate) | GameInputManager.Update | the pad sample is also captured for the Skate 3 player (`session/skate3_input_capture.rs`), after the port's stick settings |
| 2 | **Reconcile the physics mode** | | `try_reconcile_skate3` (`session/selected_player_skate3.rs`): create, switch or hand back the player; apply stance, style, equipment, profile, gestures, camera, focus, slow-motion option; `sync_teleport`; `sync_breadcrumbs` |
| 3 | AI manager | SimUpdate1 0x8289f558 | |
| 4 | Recipes | SimUpdate1 0x8289f580 | |
| 5 | LivingWorld Update1, BeginDMOUpdate | SimUpdate1 0x8289f5a8 / 0x8289f5e0 | |
| 6 | Actor input capture | GenerateActionGraphIntents 0x825FDA28 policy | `capture_actor_input`: pins, force brake, menus as Skate 2's Actor decides them (`physics/player/skate3_actor_input.rs`) |
| 7 | LivingWorld Update2; Actor Phase2 for every skater | UpdateAnimation2 0x825fa310, UpdateAnimation3 0x825fa4e0 | |
| 8 | **Skate 3 animation prepare** | | `set_graph_gestures`, `prepare_animation_tick` (section B), then the Actor takes the Skate 3 movement pose and body commands (`try_set_mode_body_pose`, `try_set_mode_body_commands`) for its hand/face streams |
| 9 | UpdatePhysics per skater | 0x825ffd98: ACS sync, SetUpPhysics, Actor::Update 0x825f10f0 | the Skate 3 skater skips ACS sync and the AI controller; `capture_actor_physics` reads its Actor physics policy |
| 10 | Grab messages; LivingWorld Update3 (animation) | SimUpdate1 0x8289f77c | |
| 11 | Shove response from the previous tick | TU3 0x8275F0D8 | a shove strike on a pedestrian reaches its bodies; one on another skater runs that skater's LivingWorld wipeout (ForceWipeOut 0x825edad0) along the shove |
| 12 | **Skate 2 world solve** | `try_step_authored_ground_with_locomotion`, SkaterManager::UpdateLocomotion 0x825f53b0 | everyone except the Skate 3 skater |
| 13 | **Skate 3 tick** | | `sync_world_bodies` (this tick's solved host bodies), `capture_composed_actor_pose`, shove probe query (TU3 0x82db3ce8: pedestrians and other skaters near the probe point), `advance` (sections B–D) |
| 14 | Breadcrumbs | | the Actor's reset reply, then PostUpdate's breadcrumb Update (TU3 0x82BFB1E0) with the host's network and living-world services; `mirror_breadcrumbs` |
| 15 | Contacts to pedestrians | | `take_host_contacts` → `LivingWorldGrindServices::skate3_contacts`; the pedestrians' OnEntityCollision reads them in this tick's ResolveContactSpyList, as TU3's island records the skater's contacts on each pedestrian body |
| 16 | Pose to Skate 2's skeleton | | `try_publish_mode_movement_pose` by bone name, root = Skate 3 root; `acknowledge_teleport` |
| 17 | Other players' Update4/Update5, conditioners | World::Update3 0x827af8f0 | |
| 18 | **Skate 3 gameplay publication** | | `try_finish_skate3_gameplay` → `try_publish_skate3_observers` (`physics/player/output/physical_player_phys_out_skate3.rs`): PhysOut for scoring, audio, HUD, career; then the scoring conditioners run on it |
| 19 | Camera | | `try_publish_skate3_camera` (`actors/skater/skater_manager_observers.rs`, `camera/frame/camera_skate3.rs`) |

## B. `Skate3Player::prepare_animation_tick` (`physics/skate3_player.rs`)

1. Online flag → solver passes (25 online, 50 offline), the skeleton's self-collision tables (every
   pair culled online), the world-contact cap (80 for the board and skeleton together online,
   none offline) and the turn bail limit (an online turn's
   wipeout limit, or `SKATE2_DIAGNOSTIC_TURN_BAIL_LIMIT` offline).
2. No resident collision → wait (no tick, no teleport consumed).
3. A finished background terrain build swaps in; a changed resident set starts a new one
   (inline during travel or with an empty world).
4. Grind refresh (`skate3_grinds.rs`), grab refresh (`skate3_grabs.rs`).
5. Pad sample → `RawController` → `collect_controller`.
6. A queued Actor teleport is applied (`host_teleport`) once the previous one was consumed,
   restoring the reset's requested leading foot first.
7. `sample_solver_input`, then `frame::prepare` (section C, first half).

## C. The Skate 3 tick (`physics/modes/skate3/physics/frame.rs`), TU3 order

`prepare`:

| Step | TU3 |
|---|---|
| Drain an unanswered respawn request from a tick the host did not follow | |
| Trajectory runtime begins the tick: a query submitted last tick completes now, never in its own tick | TrajectorySelector::Update 0x82D68800 |
| New exchange buffers (commands, events, output) for this tick | |
| Camera slow-motion requests from the last tick (ignored online; clock reset online) | SimController 0x8285C968 |
| First tick: `player_state::initialize` (Ground without ProcessOutput) | Player ctor 0x82DB3008; State/FilteredState zeros 0x82DE3BE8/0x82DE5588 |
| Climbing extension (PC addition: ledge hang/mantle; returns early while attached) | |
| Consume the off-board air selector with retained input | 0x8285A06C/A080, Player slot16 0x82DB3C78 (vtable 0x82328688) |
| Clear board forces; start wheel queries; start skeleton line tests | World 0x8275EA20 |
| **Animation graphs**: ActionGraph and MotionGraph advance on last tick's physical output | Actor phases 0x82592FD8 / 0x82593128 / 0x82593230 / 0x82593640 |

`finish`:

| Step | TU3 |
|---|---|
| Publish the animation packet, external reset reply, ForcePhysics mode | ForcePhysics Begin 0x82BB2868 |
| Simulation actions from this tick's mapping | PlayerUI 0x82898920 |
| Shove probe update | |
| **Input phase** (ProcessInput); on teleport: reset contact history, Biped cadence, state, COM filter, feedback, landing quality, board possession, ragdoll back to normal | Update1_ProcessInput 0x82DB4048 / 0x82db4d64; teleport 0x82DB8998, 0x82DB93B0..CC, Reset 0x82DE3F38, SetUpNormal via 0x82DB8E40 |
| Ground profile selection by state variant and surface mode | |
| Finish wheel queries; execute off-board grab queries | World 0x8275EC0C |
| Puddle response (PC rain feature) | |
| **PostInput and state selection** | PostInput 0x82DB5588/0x82DB573C, CalcSuggestedState 0x82D8ADE8, SetPhysicsState 0x82DB8540 |
| **PreState** | Update3_State 0x82DB6050 / 0x82DB60EC |
| **State update** (by state: Revert, HandPlant, FootPlant, Boneless, Ground [reckoning, PreUpdate 0x82D37C50, ground phase, handplant query, ground input], GrindTrick, Air, KnownAir, BipedAir, BipedGround/OffBoardPushing [+ contact submit, geometry submit], GroundAnimation, LandingOnDeck, Slide, Wipeout, Teleporting, grinds/Nonspecific) | see `03-states-and-subsystems.md` |
| State timer += timestep | State 0x82DB6120 |
| Board possession owner update | 0x82DB6150 |
| Skeleton line tests end; hips ground start | World 0x8275ECA4 |
| Puddle drag force (PC rain feature, tag 20) | |
| **Collision and solve** (`physics/solve.rs`): board + skeleton + drive targets, one island | |
| Puddle wipeout request (PC) | |
| Reset the PhysOut packet | ProcessOutput 0x82DB6EE8 |
| **Post-physics** (`Skate3PhysicsState::finish_skater`, `physics.rs`), below | |
| Dynamic normal, board publication, deck angles, grind graph outputs | |
| **State publication** (FillPhysOut) | 0x82DB7580..7790 |
| Grind camera conditioning, COM filter | 0x82DF0640 |
| Animation feedback publication, respawn observation | |
| Output snapshot and events (landing, state change) | |
| Camera output (after teleport reset) | camera subject 0x82DF69C0 |
| Climb approach, output publication end, update count, shove probe capture, clock end | |

## D. Post-physics (`finish_skater`, `physics/modes/skate3/physics.rs`)

| Step | TU3 |
|---|---|
| Wall probe for the next StartBoard | Skateboard::UpdatePostPhysics 0x82C02158 |
| Riding post-physics (board contact reports, classified by the other body's group) | Skateboard UpdatePostPhysics 0x82C07D20 / 0x82C07F54 |
| Skeleton feedback (contact reports, physical/animation errors) | |
| WipeoutGround post | |
| PhysicsAir apex | |
| KnownAir post | |
| Grind post (grinds and Nonspecific) | |
| Wipeout checks after physics | 0x82D8F9E0 / 0x82D90358 |
| GrindTrick post velocity | 0x82D33A70 |
| Off-board post-physics | slot40 0x82DB6DCC |
| Render pose publication (average compressions) | |
| Non-finite check of every board and skeleton body (fails the tick with a report) | |

## E. `Skate3Player::advance` after the tick (`physics/skate3_player.rs`)

1. `export_frame` → `FrameOutput` (the interactable object is held 30 ticks past a world grab).
2. TU3 off-board pushing on the host (`physics/skate3_pushing.rs`): with state 502 and a world
   grab, Skate 2's pushing core runs on Skate 3's attributes; it moves the prop by forces through
   the world grab queue, gives the skater the follow and the hand IK targets, and publishes the
   grab flag (FillPhysOut 0x82d46fd0) for the next ProcessInput.
3. Teleport completion; body reactions to the host world (`try_apply_skate3_reactions`);
   camera rate requests (`skate3_camera_rate::submit`).
4. Trace events (only with the test log enabled), then `publish_body_views`.
