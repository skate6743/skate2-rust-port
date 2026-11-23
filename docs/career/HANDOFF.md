# Career work — handoff (updated 2026-10-05)

Start here when continuing the career/missions/challenges work. Read [README.md](README.md) (native new-career flow,
first mission) and [runtime.md](runtime.md) (native runtime facts) next; this file says where
the port stands and what to do next.

## Where we left off (2026-10-05)

Read this section first. §0 onward is the detailed record (goal, rules, per-update status, code
map, native facts, running).

### State of the tree

- Everything compiles: `cargo check --release -p skate2-pc -j 2` passes (2026-10-05). The
  release binary builds with `build-release.bat`, or `cargo build --release -p skate2-pc -j 2`.
- Nothing is committed. The career, NIS and post-fx work is uncommitted in the working tree, like
  the earlier reorganisation: never reset, clean or check out over it.
- The last live runs were on 2026-10-02 (New Career until `m_1a_01_goto_slappys` starts; Continue
  Career on a scratch save). **Nothing added since 2026-10-03 has run in game**: the Apt HUD screens
  and popups, the NIS (cutscene) system with its overlay movies and scene characters, the ICE post-fx
  and the save orientation are tested headlessly only.

### Done in the last session (2026-10-04/05)

| Area | What | Where |
| --- | --- | --- |
| NIS camera | ICE player space 0 (the camera mover's forward ease, lag, bungee, noroll, avatar 0's frame) and the avatar anchors | `crates/skate2-formats/src/ice_camera.rs`, `nis/camera.rs`, `nis/manager.rs` |
| NIS overlays | killed it (Apt masks as the native occluder rectangles), the three photo treatments and the photo outros' preload, the pro bio card (`NIS.GetProInfo`, the `NISBackEnd` pros, `GetFontByAptName`'s fallback), the GVR stance badges, SanVanTV; 2D (type 0) screens drawn flat | `nis/fe_overlays.rs`, `nis/back_end.rs`, PC `marker_ui.rs`, `marker_ui/*_movie.rs` |
| Photo flash | the bloom pulse `FireEvent` 5 starts, and the bloom channel's default rule | `nis/manager.rs` (`BloomAnim`) |
| Reda's car | Vehicle-category scene characters (`ColorizedSkaterPresEntity`) drawn with the traffic vehicles' shaders and the entity's own constants (`i_isLightOn` is unbound natively: lights off); the native lazy default-bone lookup (`RemapBones`/`FillInvBPBuffer`) for every character; RedaCarRide_01/05's cameras, anchored on the car, now resolve | `assets/character.rs`, `session/selected_session_scene_characters.rs`, PC `vehicle_render.rs` (`SceneVehicle`), `live_skating_view.rs`, `gpu_renderer/dynamic_object.rs` |
| ICE post-fx | tonemap, fisheye warp, scanline roll, radial (zoom) bloom, the near/far colour grade (contrast, saturation with the near value on both sides, tint through `XYZ2HSV`/`HSV2RGB`, brightness) blended per pixel by scene depth | formats `Playback::{tonemap, fisheye, ...}`, `nis/manager.rs` (`PostfxChannels`, `Grade`, `IcePostfx`), PC `render_postfx.rs` (`IceInputs`), `gpu_renderer/postfx.rs`, `postfx.wgsl`, `gpu_renderer.rs` (depth view, warp), `final_pass.rs` |
| Career save | `SaveProgressionOrientation`: in a career with completed challenges, the local skater's const `GetSuggestedRespawnPosition`; `GetLastResortBreadCrumb` (which also completes the bail and chase respawn selector) | `actors/actor_breadcrumb_manager.rs`, `actors/actor_breadcrumb_live_selection.rs`, `challenge/signpost_progress.rs`, `session/selected_session_career.rs` |

Researched and blocked (written up): freeze command 5's frozen skater needs replay-controlled NIS
playback (`docs/evidence/nis-frozen-skater-2026-10-05.md`); the ICE depth of field and camera motion
blur need more shader decoding (`docs/evidence/nis-ice-postfx-2026-10-05.md`, §4.4, §4.5, §8); the
NIS breadcrumb teleport needs `IsAreaLoaded` for the sim and the presentation foci
(`docs/evidence/nis-port-notes-2026-10-04.md`, "Breadcrumb teleport"; ported 2026-10-09,
`docs/evidence/new-career-start-position-2026-10-09.md`).

### Tests (2026-10-05, with the game data)

Pass: core `nis::` 34/34, `scene_characters` 4/4, `challenge::` 32/32, `suggestion_tests` 1/1;
`skate2-formats` `ice_camera` 18/18; PC `marker_ui::` 30/30, `live_skating_view` 3/3, `postfx::`
1/1 (GPU), `final_pass` 2/2, `vehicle_render` 1/1.

Fail, not from this work (re-run on 2026-10-05, as on 2026-10-04):

- Skating input/motion: `control_fade_samples_the_twist_range_then_blends_the_intent`
  (`motion_grinding_tests.rs:295`, unwraps `Err("intent")`),
  `original_selected_live_facade_fresh_ollie_motion_route` ("fresh Ollie never selected Motion
  FromAntic789"; it passed on 2026-09-23), `original_selected_live_facade_crouch_hold_release_routes`,
  and four `grab_bail_corpus` probes
  (`original_raw_left_stick_reaches_kickturn_through_shared_speed_conditions`,
  `original_raw_mixed_controls_move_trick_grab_bail_and_recover`,
  `original_raw_partial_left_trigger_holds_air_grab_through_action_and_motion`,
  `original_raw_partial_right_trigger_holds_air_grab_through_action_and_motion`).
- Living world: `later_unaligned_skater_replaces_speed_but_keeps_prior_skitch_flags` (a missing
  attribute definition), `populated_range_uses_existing_tree_and_preserves_unused_output_entries`
  (3 against 2).
- `render_postfx_math`'s neutral-cube test is ignored by default and needs `SKATE2_NEUTRAL_CUBE`.

Nobody has traced the skating failures yet. Skating parity is rule one, so they come first after
the live checks.

### How to continue (in order)

1. **Build and live-check** with a scratch save (§6 has the commands):
   - New Career: Edit Skater, "New To Skate?" (a real popup; Yes skips the unported tutorial FMV
     through the native media-off branch), then play `m_1a_01_goto_slappys` by hand to its
     autosave. Its tutorial NIS (Rob Dyrdek, the ollie gesture tip) plays at the start. The headless
     test sets the player facts directly, so this is the first real check of the objective feed
     from physics, the trick output and the map's trigger volumes.
   - Continue Career: it should now start at the last save's breadcrumb, not at `J_Gamestart`.
   - Sign up for `m_1a_03` at its challenge locator (D-pad up, poster, A), play it, then `m_1a_04`.
   - Run with `SKATE2_TRACE_CAREER=1` and collect any `career script error` lines.
   - Reda's car and most ICE post-fx only appear in later scenes (RedaCarRide, Own the Spot, photo
     and film takes); no direct NIS launcher exists. A debug launcher (`NISManager::RunBasicNIS` by
     name) would make those checks quick.
2. **Trace the failing skating tests** above.
3. **Pick the next system:**
   - AI skaters: the career blocker (`m_1a_05_photo_rail`, `m_1a_06_photo_puzzle` and every race,
     contest and competitor). Plan in "AI skaters" below.
   - Replay-controlled NIS playback (`RunReplayNIS` 0x828a3c70; `DataMan` restore ->
     `DoSimRestore` scrubbed by `SceneReplayTimeController` 0x828727e8): photo outros and best-trick
     replays, then freeze command 5's frozen skater.
   - ICE depth of field and camera motion blur: decode `tap9dofmotionblur_PS`, `quad_motionblur_VS`
     and what writes the stencil, then `GenerateDOFMotionBlurBuffers` and `ApplyUber`'s lerp.
   - The NIS breadcrumb teleport: done 2026-10-09
     (`docs/evidence/new-career-start-position-2026-10-09.md`).
   - Audio: tracked in `docs/audio/` (§0).
4. Keep §1's rules: IDA read-only, isolated saves, byte-exact line endings, and
   `cargo check --release -p skate2-pc -j 2` before integrating.

### Implemented vs missing (2026-10-05)

| Area | Implemented | Missing |
| --- | --- | --- |
| Skating | Board and skater physics, the animation graphs, trick input and recognition (flicks, grabs, flips), grinds, manuals, vert, off-board, bails and Hall of Meat bails (evidence in `docs/evidence/`) | The failing tests above |
| World | City streaming (collision and render), world materials, gameplay post-fx and colour cubes, shadows, map travel, session markers | The load-screen flow (`IsAreaLoaded`, `LoadingMan`) |
| Living world | Pedestrians, traffic vehicles, DMOs, security and chases | The two failing tests above |
| Front end | Main menu, Edit Skater (Create-a-Skater), Start/pause menu, map, display settings, the marker phone UI | Online menus |
| Career runtime | Lua 5.1 VM and compiler, activities, challenge state graphs, objectives (locomotion, trigger, session marker), progression state machines, sign-up, HUD screens, popups, unlock overlay, Skate 101, sign-post messages, save and load, Continue Career with the saved orientation | 628 of 773 natives unbound (`python tools/career/unbound_natives.py`): points, air, grind, race and Hall of Meat objectives, the `Trick` module, most of `HUD2`, `AI`, `ChallengeProgression`; results screen (FE state 8), betting, sponsors, `QuittingOverride` |
| Missions | `m_1a_01` end to end and `m_1a_03`'s sign-up (headless); `m_1a_04` binds only ported natives | `m_1a_05` onward (AI skaters, about 67 natives each for the photo family), the other challenge families (film, Own the Spot, contests, races, Create-a-Spot, throwdowns, services) |
| NIS (cutscenes) | Scene directory, `NISManager` state machine and load states, ICE camera (every space, lens, shake), freeze channel (but commands 5 and 8), the FE overlays above, subtitles, scene characters (skater, LivingWorld, Vehicle), visible types, ground snapping, stream focus, transitions, bloom, fade and the ICE post-fx above | Replay-controlled scenes, freeze commands 5 and 8, depth of field and camera motion blur, the light rig, shadow and sun channels, speech, the breadcrumb teleport, the sponsor logo and X Games screens |
| Audio | Nothing (research only, `docs/audio/`) | Everything |
| Video | Nothing (no VP6 decoder; FMVs take the media-off branch) | The intro and tutorial movies |
| Online, Party Play | Nothing | Everything |

## 0. The goal

A new save's start screen offers New Career and runs the real game's whole flow from there: the
first mission, the save on completing it, then the entire career progression (missions,
challenges) working 1:1. Party Play follows once that works.

Audio is tracked separately in `docs/audio/`.

## 1. Ground rules

- **1:1 with the native game.** Every behaviour is traced to the debug executable
  `sk82_na_m.xex` in IDA (read-only: never rename, retype, comment, patch or save the IDB).
  Cite native addresses in doc comments, as the existing code does. Never invent gameplay
  logic or tuning; when something is not ported, say so in the code comment and the report.
- **Never sacrifice skating physics parity.**
- **Never touch a real save** (`saves/SKATER.P` or wherever `SKATE2_SAVE_ROOT` normally
  points). Every run or test that can save must set `SKATE2_SAVE_ROOT` to a scratch folder
  holding a *copy* of a save. Tests that write use unique temp dirs and delete only those.
- **Do not disturb the working tree.** Never reset, clean, check out over or reformat the
  workspace.
- **Mixed line endings.** Source files mix CRLF and LF, sometimes within one file. Never
  normalise. Edit byte-exact (Python with `newline=''`, converting the pattern only when the
  surrounding lines use CRLF), or with an editor that keeps each line's ending.
- **Building/running:** runs use isolated saves only. A full rebuild takes long and needs
  several GB of disk.
- Keep code style: doc comments state native facts; no narrative commentary in code.

## 2. Status

### Update 2026-10-04: NIS (in-game cutscenes)

Core (`crates/skate2-core/src/nis/`, `crates/skate2-formats/src/ice_camera*.rs`), all traced in IDA:

- **Scene directory, NIS state machine and challenge hookup** (from 2026-10-03/04): `RunBasicNIS`, the
  presentation load states, `kPAUSE_STATE_NIS`, bloom-to-white transitions (the original's 1 s each
  way, the port's 0.2 s),
  `SkipNIS`/`IsNISSkippable`, subtitles (`SubTitler` -> `cMsgSetSubtitle` -> the front end's
  `FESubtitle`: cleared by `FrontEndState_NIS::OnEnter`, and on the first front-end update after
  `OnLeave` turns the time-out back on), sign-post type-2 messages (12-update delay for
  four scenes), presentation event `NIS` ("True" skips the breadcrumb teleport), `cMsgNISChallengeNotify`.
- **ICE camera:** assembly takes and cuts, `TIME_SCALE`, the lens channel
  (`ICEUserChannelLens::UpdateFrameEnd` 0x826cd7e8: final FOV with `LENS_OVERRIDE`/aux lens, near plane
  from `CLIPPING_NEAR`), `GetFieldOfView`'s float round trip for the shake, anchor space (skater root,
  `User_ICECameraAnchor::GetPosition` 0x826cfc78), scene space 3 (`ICEMatrixFromSkatePointAndNormal`,
  `TransformToWorld` 0x82a37540) with the scene's up (`CreateWorldCameraMatrix` 0x82a371b8, eye in
  space 3), procedural shake (`ICE::Noise`, `ICE::ASin` + `ASinTable`, `RotateY/Z`), recorded shake.
  Exact native float order: fused `ICE::Sin`/`SinCos`, `MulMatrix` per-element fma chains, fused
  lengths/cross products. `PLAYER_SPACE_LAG` is a zero offset in this game (anchor acceleration is
  always zero). The first ICE frame is `Scene::ComputeTransform` of the time-zero eye/look
  (`GetCameraComponentsAtTimeZero` 0x828647f0); FOV/near plane keep their last values (constructor
  30/0.2). `kNISFreezesStreamer` (default 1) freezes the stream focus at that first camera.
- **ICE post-fx channels:** bloom (`ICEUserChannelBloom` 0x826c88e0, defaults 1.4/0.5) and fade
  (`ICEUserChannelFade` 0x826c80f8 -> `g_FinalMult`). The ICE letterbox height is written but never read
  in this build (no bars to draw).

PC (`apps/skate2-pc`): while a NIS holds the sim paused, `finish_redraw` draws the retained world
through the ICE camera (`try_nis_visual_snapshot`: ICE matrix/FOV/near plane, `RenderMan::UpdateCamera`
0x82848978); the HUD hides (`FrontEndState_NIS`); bloom-to-white lowers the bloom threshold by 2x
(`VisualFX_Animation` 0x82848718 / `SetParameters` 0x8284c170), moves the colour matrices to identity
(`PostFX::Apply` 0x8284c92c) and lifts the vignette (`Blit_ToDeviceRT` 0x82831d5c); with PostFX off
(outside the cutscene, whose post-fx turns PostFX on) a white cover of the bloom's value stands for it
over the scene, so the fades show either way; the ICE path uses
the cinematic post-fx with the ICE bloom and `g_FinalMult`. The subtitle movie (`subtitle.apt`,
main.swf's `gSubtitle` layer; `marker_ui/subtitle_movie.rs`) shows the line with the profile's
subtitles option: band and centred text, `_y = (3 - maxscroll) * 12`.

Coverage: 91% of the shipped NIS camera samples resolved before the avatar anchors (17110/18777).
Anchors on scene avatars now resolve (`GetAvatarPosition` 0x826d1608: local hips, scene entities'
object matrices; RedaCarRide_07's take resolves on Haslam, RedaCarRide_01/05 on Reda's car).
Player space 0 now
resolves too (`ice_camera::Mover`: the camera mover's `mForward` ease and bungee, kept by the NIS
manager for the session; the anchor orientation from the published skater root; avatar 0's
upright heading; NOROLL), every shipped player-space sample included. Unresolved cameras
fall back to the frozen game camera.

- **Scene characters** (`nis/characters.rs`, `session/selected_session_scene_characters.rs`): one
  `SceneCharacter` per record (`Scene::AddEntities` 0x82877838: "dynamic" skipped, 32-character
  limit), its `.abin` clip played by `SceneCharacterAnim` (absolute trajectory fetch; `EvalCurrAnim`
  0x82866cd0 bit-exact: hips split out, scene twist bones, object matrix `H·(T·Traj)`, `mHipsPos`),
  user transform from the record (orientation flattened, basis in columns) times the scene transform.
  A record whose recipe is missing is no scene entity (`InstanceCharacter` destroys it).
  Each record gets its own renderable (`CreateCharacterRenderable` 0x82871150: recipe
  `data\content\recipe\<category>\<schema>.recipe`, clone type 2, cloth disabled, LOD 0, type
  0x822). `current_player` borrows the local skater instead (`ControlCharacterRenderable`
  0x82870fb0): face pose by `InitFace`, the clip poses the skater's own palette
  (`publish_borrowed`), opacity 1, not ghosted, type 0x822. The NIS pause skips `DoSimRestore`, so the
  scene alone places it. The `nis` technique is replaced by `default` before any draw (see the port
  notes); scene characters draw with `default`. LivingWorld records are
  `ColorizedSkaterPresEntity`s: the pedestrian effects (PC `pedestrian_render::ColorizedObjects`)
  with `mCSColours` from the living world models class, opacity 1, the character's skater SH, and
  `i_material_lod` 0 (unbound in the native: the register keeps the previous draw's value). Every
  scene character casts by its own hips and `m_BB`.
- **Visible types:** `RenderMan::UpdateVisibleTypes` 0x82839be0 while a scene exists; the PC hides the
  skater (by its type in the snapshot), DMOs, vehicles and pedestrians the mask leaves out
  (`DetermineVisibilityFromGameState` 0x82829698: `mask & type == type`); they keep updating.
- **Ground snapping and the stream focus:** load states 1-4 (`UpdateLoadState` 0x82875bc0) put a
  snapping scene on the ground under it (`cMsgNISWorldCollisionQuery`; the orientation laid on the
  ground's normal) and its flagged records on the ground under theirs (`SceneLocationSnapper`:
  out through the scene transform, back through its inverse); the simulation answers with
  `World::LineCollisionTest` from one unit above the point to 499 below it. The NIS freezes the
  stream focus at the scene (state 1), then at the camera (`Scene::Init`); while it pauses the
  simulation the session moves the terrain's focus there (else to the Actor, as
  `Sim_LocalPlayerStreamerController`) and the renderer streams there. States 2 and 6 (scenes with
  bit 1) wait for the terrain. `ComputeTransform` now keeps the native signed zeros (`vmx_cross`).
- **FE NIS overlays:** `nis::fe_overlays` ports `FrontEndState_NIS` (preload list from the
  camera track's overlay channel, `OnEnterState`/`OnLeaveState`, `PopNIS`) and the AS screen
  manager queue (one screen per nine Apt frames, one Apt frame per front end update); load
  state 5 waits for `IsFEPreloaded`. The PC draws the ollie gesture tip (`gestollie`, asset 37)
  from `CareerHud::nis_screens`; the tutorial NIS opens it during the load and steps it
  olliedown/ollieup three times. The spot banners (`ownthespot`, `spotfound` with
  `Mission.GetTitle()` through `CareerHud::mission_title`) are drawn too; the APT parser takes
  their 1:6:4 format (the game's loader reads no version). Killed it draws too, with Apt masks as the
  native occluder rectangles (`docs/evidence/apt-mask-layers-2026-10-04.md`: the game's masks hide
  what is inside a rectangle; they do not clip like Flash). Type 0 (2D) screens now draw flat as
  the game's camera 8 does: the sign-up HUD and poster, Skate101, subtitles and the NIS overlays
  had been going through the tilted HUD projection; worth a look in game.
- **ICE freeze channel (34):** run after the overlay channel each frame (registration order);
  every crossed interval fires once. Timeout/Owned/Permanent/PhotoShot freeze the scene clock
  through the scene player's freeze mults, Jump sets the scene time, Killable posts
  `NisChallengeNotify{3}`. Fp_u1_ownspot_01_outro holds a second on its Owned interval.

Vehicle scene characters (Reda's car, 8 records) draw through the traffic vehicles'
dynamic-object path with the entity's own constants (`i_isLightOn` unbound natively: lights off).
Not done for NIS: speech, the overlay movies
other than gestollie, the spot banners, killed it, the three photo treatments (photo outros now
preload the fixed or non-fixed one), the pro bio card, the GVR
stance badges and SanVanTV (sponsor logo, pro name, the photo-size and X Games states: logged per
template), FE state 8 for overlay 79, freeze commands 5 (fixed-frame photo: its frustum test
and frozen skater read the replayed skater, so it waits for replay-controlled NIS playback, which
the port lacks: all 64 photo outros and the best-trick replays are such scenes; the bloom pulse it
starts is ported) and 8 (load screen anim),
breadcrumb teleport application, the presentation stream focus's load state (the renderer's; not
waited for), the ICE DOF (Focus/RawFocus, 445 takes) and the light rig, shadow and sun channels'
readers (tonemap, fisheye, scanline, radial blur and the near/far grade from brightness, contrast,
saturate and tint are ported; motion blur and noise reach no shader). The Colorized and NIS
vehicle draw paths and the depth-blended NIS grade have not yet been run in game (need a full
build).

Tests: core `nis::` 34/34 (game data: 07_Cuz_Bowl_Series' snapping records on a flat ground;
tutorial_m_1a_goto_slappys' gesture tip commands; Fp_u1_ownspot_01_outro's freeze;
replay_c_1a_05_besttrick_street_01's player-space look; the photo outros' preload;
`a_photo_flash_dips_the_bloom_threshold_and_returns`; photo_killed_m_1a_05_photo_rail's photo
treatment channels),
`scene_characters` 4/4 (game data: Rob Dyrdek's renderable in
`tutorial_m_1a_goto_slappys`; `01_Welcome_Back`'s current_player on `tutorial_male`;
RedaCarRide_07's camera on Haslam, its car and its two pedestrians' colours; RedaCarRide_01/05's
cameras on the car), `skate2-formats` `ice_camera`
18/18 (every shipped player-space sample resolves; the post-fx override census), `challenge::` 32/32 (with
`the_pro_bio_reads_the_main_challenges_pro`), PC
`live_skating_view` 3/3, `the_nis_car_takes_only_the_vehicle_effects`, `postfx` (GPU),
`a_nis_lowers_the_bloom_threshold_and_takes_the_ice_bloom_and_fade`, `marker_ui::` 30/30 (with
`original_fixed_and_non_fixed_photo_treatments_snap_with_their_own_sound`,
`original_stance_badges_wait_unseen_then_scale_in_and_fade_out`,
`original_san_van_tv_logo_holds_its_first_frame_until_cleared`,
`original_pro_bio_card_fills_from_get_pro_info_and_plays_a_side`,
`apt_fonts_match_without_case_and_fall_back_to_the_first_non_debug_font`,
`original_photo_treatment_flashes_its_frame_on_snapshot_and_fades`,
`original_killed_it_banner_plays_a_corner_through_its_flourish_and_outro`,
`a_mask_rectangle_cuts_its_area_out_of_the_content`,
`original_gesture_tip_shows_its_backer_and_fades_in_the_stick_over_its_intro`,
`original_spot_owned_banner_plays_a_corner_intro_holds_and_outros`,
`original_spot_found_banner_names_the_spot_and_plays_its_corner`,
`original_subtitle_shows_a_nis_line_over_its_band_and_clears` and
`original_nis_subtitle_lines_format_without_arguments`). Pre-existing failure, not from this work
(files last changed 2026-09-28): `motion_grinding_tests.rs:295`
`control_fade_samples_the_twist_range_then_blends_the_intent` unwraps `Err("intent")`. Also failing
on 2026-10-04, on the input/motion path the NIS work does not touch (the ollie route passed on
2026-09-23 per `docs/evidence/ollie-live-board-tilt-2026-09-23.md`): the facade corpus
`original_selected_live_facade_fresh_ollie_motion_route` ("fresh Ollie never selected Motion
FromAntic789"), `original_selected_live_facade_crouch_hold_release_routes` and four
`grab_bail_corpus` probes (kickturn exit, mixed controls, partial left/right trigger grabs); the
other 16 facade tests pass.

Evidence: `docs/evidence/nis-port-notes-2026-10-04.md` (verified while porting) and the agent
research `nis-playback-flow-`, `nis-scene-characters-`, `nis-postfx-letterbox-2026-10-04.md`.

IDA note: the PPC module misprints the VA operand of VX128 instructions (bit 5 of the word taken as
+64; it is +32). Decode raw words; `tools/ida/vmx128_emu.py` runs straight-line VMX128 code and
`tools/ida/vmx128_symbolic_fp.py` prints the exact fused order of scalar float code.

### Update 2026-10-03 (laptop session; details and addresses in `docs/evidence/career-tutorial-natives-and-unlockables-2026-10-03.md`)

All of the 2026-10-02 "written but not compiled" edits below have since been compiled and tested. Added and
verified against IDA since then (core `challenge::` tests 31/31 and PC `marker_ui` tests 17/17 with the game data):

- Real Apt HUD screens instead of text stand-ins: objective HUD (`hudobjective2`), tutorial callout
  (`hudtutorial` + `gesture_item`, presentation events 20/21, `TutorialInstructionHUD`), sign-up prompt
  (`actionoverlay2`, `SignUpHUD` component in its `cChallengeSignUpManager-DPadUp` group).
- The "New To Skate?" / progression dialog is a real sk8popup (No/Yes, default No) answered by the player;
  No executes `m_1a_01` without the FMV (it used to be auto-answered Yes).
- `FrontEndManager::StartInGame` HUD groups (`main`, `phonelist`), presentation central messages (event 23),
  `SignUpData::SortChallenges`, localizer formatting (`FormatStringCustom`, number formats), the trick
  display's distance precision, Apt case-insensitive child lookup, `attachMovie` depth semantics.
- Sign-up poster: `challenge/challenge_signup` with `challenge_info`'s sign-up mode, the entry fee, the
  half-speed world after its intro, A/B.
- Unlock overlay (`FrontEndState_Unlock`, `unlock/unlockoverlay`): the real screen with its reward steps,
  sponsor items and 30-update cash count, drawn through the native spin projection; the state pauses the
  game (FRONTEND pause) and takes A only after `Cash_SetCashAnimDone`. Its data: `AddCompletedChallengeToResults`,
  message type 6 (throwdown phone numbers, sponsor-me tape), `Award.GrantAward`,
  `ChallengeProgression.UpdateCashAward` (contest purse, medal), `CheckChallengeResults`, the CAS unlock-part
  table (`cas_db_000.unlocks`).
- Every signpost message type now takes its native branch: 4 NPC speech (speech matched by
  `eSk8Characters`, as the presentation director does), 5 Skate 101 (FE state 79 with the real
  `tutorials/skate101` screen; pauses, A once the screen is ready), 7 cell-phone setting, 11 (empty in this
  build), 16 BLOG messages and user news (add/remove with the native order shuffle); 8/13/17 have no case.
- Front-end input waits for the top state's screen to be ready (`IsScreenReady`), for the unlock overlay too.
- Progression natives that ended their handlers with a Lua error are ported: `PlayProgressionReminder`/
  `PlayCameramanSpeech`, `DynamicHull.SetupDynamicHullState`/`TestForDynamicHullState` (states saved in the
  `Sk8ChallengeSystem` savable, `cChallengeDynamicHullManager::Data` v3), `Service.DrainPoolViaProgression`,
  `SessionMarker.SetSessionMarker`, `Achievement.GrantAchievement`, `NIS.IgnoreTeleportToBreadcrumbOnNextNIS`
  (the last three and the hull states are outputs the session logs; no hulls are loaded).
- Presentation events 26 (complete), 27 (revert), 30/31 (challenge map icons: m_1a_01's `mini_teleport` now
  shows on the minimap) and 35 (point pools); `Progression.RevertChallengeByTitle` now does the native
  `RevertCompletedChallengeMap` (order shuffle, count, re-activation) instead of only dropping the entry.
- Next, in career order: `m_1a_04` (manuals) uses only ported natives; `m_1a_05_photo_rail` and
  `m_1a_06_photo_puzzle` need ~67 unported natives each (the photo-challenge family: AI photographer paths,
  photo NIS, highlight reel, results/continue screens, `ChallengeSave`, timers HUD). Census:
  `python mission_natives.py <challenge>` (scratch tool reading `tools/career`). Also pending: betting
  (`SetPlayersBet`/`DidPlayerBetMax`, the poster's bet panel), the several-challenges choice popup,
  presentation events 28 (achievements), 30/31 (map icons), 33 (dynamic hull state), stats back end.

### AI skaters (the next career blocker; plan as of 2026-10-03)

`m_1a_05_photo_rail` (20-state photo-challenge graph: `spawn_player` → `AI.EnterAiBasedChallenge`,
`AI.SpawnAIAtLocation`, `AI.PinAI`; `check_ai` polls `Entity.DoesSkaterExist`; `countdown` → `AI.UnpinAI`,
`AI.StartPath`; `complete` → `AI.UnspawnAI`, `AI.UnloadPaths`) cannot advance without an AI photographer, and
races, contests, competitors and the city's ambient skaters need the same system.

Native pieces:
- `AI.SpawnAIAtLocation(id, location)` `0x8288c620`: `cLocationManager::GetSpawnLocation`, then
  `AISkaterManager::SpawnAISkater(transform, id, AC_CHALLENGE, ChallengeAIManager::GetCharacterProfile(id))`
  `0x824cba18`: `PlayersBackEnd::CreatePlayer`, `SetupAssetForSkater` (CAS recipe), `SetPlayerCharacter`, then a
  `cMsgSpawnAi` (kind `0x3405243E`) on the simulation broadcaster; `PlayersBackEnd::SetManualsAreConnectors`.
- `AI.StartPath(id, branch)` `0x8288ce68`: the skater's `AIController` (actor +532) gets a `FollowPathBehavior`
  with `ChallengeAIManager::mAIPathID`, the branch mode and, when filtered, the filter and trick-swap mode;
  `FollowPathMove::OnUpdateActionGraphIntent` writes the action-graph intents.
- `AI.LoadPaths(activity)` `0x8288c8b8`: `ChallengeAIManager::LoadPaths` unless
  `AIPath::PathManager::GetStaticPathBinaryLoadingEnabled`; returns the paths added.
- `AISkaterManager::Update` `0x824d0188` runs in SimUpdate1 between input recognition and `RecipeMan.Update`;
  `SkaterManager::UpdateAnimation1/2/3/UpdatePhysics` then loop over every skater.

Port state (survey of the session): session startup, the per-frame tick (`SelectedPlayerTick`,
`SelectedSessionFrame`), the skater rosters (`try_publish_first_actor`, `append_first`), the visual snapshot and
`LiveSkatingView` assume one local skater, and every AI-controller branch is stubbed (`ai_controller`,
`AnimationAiInput`). Already multi-skater: input dispatch (9 `GameInputPlayer`s, receiver ids 10–18), the
physics pool (8 board/skeleton pairs), the GPU renderer (the pedestrian path draws N characters),
`PlayersBackendSettings` (9 rows).

Order: (1) `PlayersBackEnd` rows and `AISkaterManager` spawn bookkeeping in core, `DoesSkaterExist`,
`Pin/UnpinAI`; (2) a second actor built with the shared global services, admitted to the rosters; (3) the
multi-skater update order; (4) `AIController`/`FollowPathBehavior`/`AIPath` path data; (5) drawing a second
skater; (6) the rest of the photo family (prefilm/DPO/ABD HUDs, countdown and film timers, highlight reel,
photo NIS, photo selection, end options/results). Skating physics must stay identical for the local player.

### Verified (tests and live runs on 2026-10-02)

| Area | State | Evidence |
| --- | --- | --- |
| New Career on a fresh profile | Main menu (new profile) → Edit Skater (new-career substate, opened once the player is registered) → `FrontEndState_CaC::Shutdown` equivalent starts the career → "Start State Machine" → "New To Skate?" dialog (auto-answered Yes, no popup UI yet) → `m_1a_01_goto_slappys` runs | live run (objective `ID_CHALLENGE_M_1A_01_GOTO_SLAPPYS_BACKENTRANCE` shown, teleport to `J_Gamestart`) |
| First mission end to end | 12 objectives, outro, `Trigger_m_1a_01_goto_slappys`, autosave; afterwards `m_1a_03_tutorial_fliptricks` available (new) | test `first_mission_completes_and_autosaves` (scripted player) |
| Save on completion | `Output::AutoSave` → `SKATER_PROFILE` (profile data with `mNewProfile` cleared) + `SignPost` savable (`cSignPost::Data` v14, `cSignPost::Position` v1, `cSignPostMessageMan::Data` v1), one atomic commit with read-back validation | tests `first_mission_completes_and_autosaves` (save → reload into a fresh system → identical progression; second save byte-stable), `career_autosave_round_trips_through_the_profile_store` (real file store) |
| Continue Career | profile no longer new → main menu takes the continue path → SignPost loaded (`LoadProgressionData`) → player placed per `FrontEndState_MainMenu::EnterGame` (saved orientation, else `fe_locations` start `J_Gamestart`) → Start State Machine runs its post-mission branch (Lock Hom / Pool Draining / Security / Uncapping, minimap) | live run against a scratch save made from the scripted mission; test `continued_career_start_location_is_authored` |
| Challenge sign-up (how every later mission starts) | `cChallengeSignUpManager`: prerequisite evaluation, boxes at challenge locators, D-pad-up prompt, select operations, teleport to `MapStartLocation`, poster, accept/decline operations, fade, `StartMainChallenge` | test `sign_up_starts_the_next_mission`: after mission one, standing at `m_1a_03_tutorial_fliptricks_challengelocator01` → prompt → D-pad up → poster + teleport → accept → `m_1a_03` becomes the main challenge, no script errors |

### Written but not compiled

None. The 2026-10-02 session and PC edits once listed here were compiled and tested on
2026-10-03.

### Not done (the long tail)

- **Presentation:** intro FMVs (VP6, no decoder; the media-off branch skips them), speech (ends at
  once; no audio), the results screen (FE state 8), most visual indicators. Ported since this list
  was first written: the objective HUD, tutorial callouts, sign-up prompt and poster, popups,
  unlock overlay, Skate 101, map icons, and the NIS (camera, scene characters, subtitles,
  overlays, most post-fx); see the updates above.
- **Progression features:** sponsors (`cSignPostSponsorMan`: board sponsor, sponsor bonus on cash
  pool 43), stats back end (cash credit), unlockables (`UNLOCKABLES` savable), achievements,
  awards, services (pool draining, skatestopper uncapping, security), dynamic hulls
  (`Sk8ChallengeSystem` savable), pinch points, photo articles/BLOG captions, betting/entry
  fees.
- **Natives:** see `python tools/career/unbound_natives.py`; only those the shipped scripts call
  matter (`tools/career/luacalls.py`).
- **Challenge families** needing new systems: AI competitors (races, contests), film/photo
  challenges, Own the Spot, Hall of Meat, Create-a-Spot, throwdowns, global activities
  (`Global` 1/2: services, OTS discovery).
- **Party Play** (`PartyPlayBackEnd`, module 19): not started.

Rough estimate (2026-10-02): the career's skeleton (mission runtime, objectives, progression
machines, save/load, sign-up) is in place — perhaps a quarter to a third of a fully playable
career. Presentation has grown since; the missing systems above are what remain.

## 3. Code map

`crates/skate2-core/src/challenge/` (the whole directory is new, written for this work):

| File | Native counterpart |
| --- | --- |
| `system.rs` | `Sk8ChallengeSystem`: `World`, event queue (queue 7), `Output` requests to the session, `update`/`update_frontend`/`flush`, `game_load`, `loading_reset`, sign-up input entry points |
| `activity.rs` | `cActivityManager`: activities (leaf `challenges`), main challenge, pending resets, `game_load`, `reevaluate_prereq_activities` |
| `runtime.rs` | `ChallengeRuntime` state graphs, post office, challenge handlers |
| `signpost.rs` | `cSignPostManager`: progression state machines, challenge availability/completion, messages (`HandleCaughtMessage`), natives |
| `signpost_progress.rs` | groups, group exits, point pools, rankings, challenge save values, BLOG, news, orientation; natives; **memcard save/load** |
| `signup.rs` | `cChallengeSignUpManager` + `SignUpHUD` input + the sign-up hooks of the other sub-controllers |
| `objectives.rs`, `objective_natives.rs` | `cChallengeObjectivesManager` and the objective condition natives |
| `mission.rs` | `PhysOutModule_MissionSystem` (messages from physics: tricks, grinds, wipeouts, …) |
| `trigger_tester.rs` | `GenerateTesterVolume`, enter/exit |
| `presentation.rs`, `hud.rs`, `effects.rs`, `timers.rs`, `assets.rs`, `dmo.rs`, `tricks.rs` | presentation director, HUD2, effects (fades), timers, challenge asset loads, DMO proxies, trick groups |
| `natives.rs`, `binder.rs`, `binding.rs`, `binding_table.rs` | Lua bindings; `binding_table.rs` lists every native table of the executable |
| `lua/` | Lua 5.1 VM **and compiler** (progression handler sources are compiled at load) |
| `attrib_db.rs`, `attrib_lua.rs`, `keys.rs` | Vault database, Lua metatables, hashes and class keys |
| `tests.rs` | headless career tests (game data required) |

`crates/skate2-core/src/nis/` (in-game cutscenes):

| File | Native counterpart |
| --- | --- |
| `directory.rs` | `SceneBinDirectory` (`scene_000.bin`) |
| `manager.rs` | `NISManager` + `SceneManagerImpl` load states + `ScenePlayer`/`Scene` + `CameraPresentation`'s ICE fields + `PostfxPresentation`'s ICE data (bloom, pulse, fade, grades, tonemap, fisheye, scanline, radial blur) + the freeze channel |
| `camera.rs` | `ICEManager::Update` over `ice_camera.rs` playback; anchor, player and scene spaces; the overlay preload list |
| `characters.rs` | `SceneCharacterManager`, `SceneCharacterAnim` (`EvalCurrAnim`), the records and recipes |
| `fe_overlays.rs` | `FrontEndState_NIS` and the AS screen manager queue (template numbers per overlay) |
| `back_end.rs` | `NISBackEnd`'s pro table (`GetProInfo`) |
| `transition.rs` | `PresTransitions`, `BloomToWhiteTransitionEffect` |
| `subtitles.rs` | `subtitles.big`, `SubTitler` |

`crates/skate2-formats/src/ice_camera.rs` (+ `ice_camera_math.rs`): ICE 2.96 camera data, take
playback, `ICECameraMover` (transform, lens, shake), the lens/bloom/fade user channels, ICE math.

Integration:

| File | Role |
| --- | --- |
| `session/selected_session_career.rs` | `CareerSession` (system + mission system + trigger tester), `start_career(continued)` (vaults, `global.lua`, `BAM_Sim.loc` locations, SignPost load, progression on, `game_load`, `loading_reset`), `try_update_career` (one completed sim tick in native order), `apply_career_outputs`, `save_career`, `place_continued_career`, `career_input`, `teleport_local_player` |
| `session/selected_session_public_facade.rs` | main-menu leave: new profile → Edit Skater then career; otherwise `career_start_pending` → `start_career(true)`; per-tick `try_update_career`; `career_input` |
| `session/selected_session_edit_skater.rs` | `finish_edit_skater` → `start_career(false)` for New Career |
| `session/edit_skater_profile.rs` | `EditSkaterProfileStore::load_leaf`, `save_career` (profile data + savable leaves, atomic) |
| `physics/player/output/physical_player_phys_out_mission.rs` | `try_mission_frame`: the PhysOut facts the mission system and trigger tester need |
| `apps/skate2-pc/src/live_skating_client.rs` | `--live-controls-demo career` (A at tick 600 on the main menu, Start every 60 ticks while the editor is up, 2400 ticks) |
| `apps/skate2-pc/src/marker_ui.rs`, `marker_ui/*_movie.rs` | the Apt screens: objective HUD, tutorial, sign-up prompt and poster, popups, unlock overlay, Skate 101, central messages, subtitles and the NIS overlays (gestollie, ownthespot, spotfound, killedit, phototreatment and the fixed/non-fixed photos, pro bio, GVR, SanVanTV) |
| `apps/skate2-pc/src/live_skating_view.rs` | scene characters (skater, colorized, vehicle draws), visible types, `nis_postfx` (the ICE inputs) |
| `apps/skate2-pc/src/vehicle_render.rs` | traffic vehicles and the NIS `SceneVehicle` |
| `apps/skate2-pc/src/render_postfx.rs`, `gpu_renderer/postfx.rs`, `postfx.wgsl`, `final_pass.rs` | PostFX: bloom chain (radial blur), ghosts, the gameplay grade and the ICE near/far grade (scene depth), the scanline roll, the colour cube, the final blit with the fisheye warp |

Tools: `tools/ida/career_ida.py` (IDA py_eval helpers: `dec`, `disasm`, `keys64`, `find`,
`show_type`), `tools/career/attrib_hash.py` (StringHash64, name search),
`tools/career/dump_signpost_save.py` (inspect a SKATER.P's savables and SignPost tables),
`tools/career/unbound_natives.py`, plus the older readers in `tools/career/README.md`.

## 4. Native facts used (reference)

Addresses are debug-xex VAs. More in [runtime.md](runtime.md).

### 4.1 Frame order

`PhysOutModule_MissionSystem::Update` (0x82e313a0) queues physics messages → `Sk8ChallengeSystem::Update`
in `SimUpdate2` (`kRunChallengeUpdateDuringPhysics` = 0): activity manager (pending resets,
sub-controllers in registration order: assets, …, **sign-up**, …, timers, …, objectives, …,
effects), then `cSignPostManager::Update` → `cTriggerVolumeManager::Update` queues trigger
enter/exit → `EventSys::Flush` in `Simulation::PostUpdate` (0x8289ff24). Trigger events must be
queued after that tick's challenge update (the stairs objective resets every frame otherwise).

### 4.2 Saving

- Savable registration order (save order): `BackEndManager::Init` → `UNLOCKABLES`, `CAC`,
  `SKATER_PROFILE`, `online`, `STATISTICS`, `DYNAMICOBJECTS`; then `Sk8ChallengeSystem`
  (ctor, saves the dynamic-hull manager), then `SignPost` (`cSignPostManager` ctor via
  `AllocateSubSystems`). The port inserts `SignPost` after the last of those present.
- Autosave: a progression machine's `Terminate()`, or `ProcessChallengeResults` with no
  triggered machine → `InitiateAutoSave` (0x825a66c0) → memcard save of every savable.
- `mNewProfile` (`ProfileBackEnd+0x68`) is set only by the ctor/`ResetProfile`; New Career's
  `FrontEndState_CaC::Shutdown` clears it in memory; the next save persists it. `IsNewProfile`
  (Apt) reads it. The port writes `ProfileData` with it cleared on every career autosave,
  creating it with the localized `ID_DEFAULT_PLAYER_NAME`/`_NICKNAME` when absent.
- `cSignPost::Data` v14 (`SaveData` 0x825cdef8): fixed-size tables in key order, padded with
  zero entries — available 2048×(Int id, Bool new); groups 80×Int; completed 2048×(id, order,
  photo caption); group exits 80×(group id, **point-pool lookup of the exit number** — native
  bug, always -1); group counts 80×(id, count); point pools 80×(id, points or -1); rankings
  32×(group id, ranking id); challenge save values 512×(challenge id, type id, value);
  256×(UInt64 0, Int 0); BLOG 512×(id, type, order, Bool new); news 32×Int. Ids are layout
  words (progression_challenges word 12, groups word 2, pools word 1, …; see
  `signpost_progress.rs`). Position v1: Bool valid + 4 rows × 3 floats. MessageMan v1: UInt 160,
  UInt count, count×(UInt64 key, UInt value), then (160−count)×(0, 0xDEADBEEF).
- `LoadData` (0x825e53d8) → `LoadProgressionData(14)` (0x825e41d0): `DeInit` + `Init`, then
  re-applies each table (exact conditions in `load_progression_data`), clears `DuringInit`
  before the BLOG; `LoadProgressionOrientation`; `MessageMan::LoadData`.

### 4.3 Deliberately mirrored native quirks (do not "fix")

- Completed group exits do not survive a reload (they come back as one key-0 entry) because of
  the save bug above.
- BLOG entries created during `Init` (Starting challenges) keep their Init order/`new` flag on
  load; saved values for them are ignored.
- Starting challenges and start-active groups deactivated during play come back after a reload
  (Init re-activates; load never deactivates).
- `RunStateMachine` does **not** touch `intActivatedStateMachineCount`; only the sign-up hooks set
  it (to the number of machines `CheckStateTriggers` queued) and `TerminateStateMachineNOW`
  lowers it.
- The save orientation is only valid when the native would set it: `SaveProgressionOrientation`
  (0x825c7448), in a career (`GameTypeFlags & 4`) with completed challenges, saves the local
  player's breadcrumb `GetSuggestedRespawnPosition` (0x82ce2888 -> 0x82ce2698, ported
  2026-10-05: the wipeout-spot crumb if `kEnableWipeoutSpotBreadCrumb`, else the newest crumb
  passing the living-world respawn test, kept in the ring (`GetBreadCrumb` 0x82ce2428), else
  `GetLastResortBreadCrumb` 0x82ce2598: the newest crumb, or with an empty ring the spawn
  position with the fakie foot forward) and marks it valid for every later save. Without
  completed challenges Continue Career still starts at `J_Gamestart`. `QuittingOverride` (the
  StartHub quit popup, 0x825c4688) is not ported. The last-resort crumb also completes the
  popping respawn selector (bails, chases).

### 4.4 Sign-up manager (`cChallengeSignUpManager`)

- Activities: `CreateActivity` (0x82cdbca8) adds every non-debug-only activity to each
  sub-controller; sign-up keeps `Global` == 3 (204 activities). Classification by `Global`
  (`challenge_global_data`, key 0x122db7b0f026dbec): 1/2 global (41/83), 3 sign-up, 4
  progression-launched (5: `m_1a_02_open_up`, `m_6a_01_follow_spillway`,
  `m_6a_01_filmfriend_70s`, `m_6c_01_profilm_soho`, `m_6d_01_profilm_classic`) — those never
  auto-start in the shipped config (`kEndableProgressionLaunchedChallenges` default 0); scripts
  start them.
- `GameLoad(4)` (0x82daec00) → `Reset`, `ReevaluateActivityPrereqs(4)`; state stays `Disabled`
  until `LoadingReset` (0x82da1a58, from `StateLoading::WorldPrepareState`).
- Prerequisites (`EvaluateActivitiesForPrereqs` 0x82cd9e88): `IgnorePrereqForSignup` on the
  challenge, else `Sk8ChallengeSystem::IsChallengeActive` (0x825a2c78: everything without
  progression, else the signpost's availability of the activity's challenge).
- Box: `Location` (global data) position ± radius (challenges key 0x8022ba64cb8e431d, else
  `kDefaultLocationTrackingRadius` 40). Tester box (`GenerateTesterBBox` 0x82e20ee0):
  `Actor::GetBasePosition` ± 0.29 on X/Z, +0..1.74 on Y (`kTriggerAABBCharacterWidth` 0.58,
  `…Height` 1.74). Overlap is inclusive on X/Y/Z; collisions are ascending slot indices;
  freed slots are reused last-in-first-out.
- States (`eSignUpState` 0–10) and `Update` (0x82db44a0) are ported 1:1 in `signup.rs`.
  High-priority sub-controllers: signpost (select/accept/decline → `CheckStateTriggers` 4/5/6),
  BE (pause menu/backpack off on select, on at accept). Low: telemetry, service, HOM, teleport
  (teleport to `MapStartLocation` else `Location` on select unless type 9/35/throwdown), audio,
  asset (load on select, hulls-loaded gate, unload on decline), DMO, living world, OTS
  discovery, effects (0.5 s fade-out on accept; gate on its completion), Create-a-Spot, system.
  Only the ones affecting logic are ported; the others are presentation or unported systems.
- `SignUpHUD` input: 15-frame press delay after the prompt shows (`Show`), counting only while
  D-pad up is not held; one challenge → `PlayerTriggerSignUp`; several → popup (up to 4).
  Poster (`FrontEndState_SignUpPoster::ReceiveInput` 0x82657bf8): code 2 accept (entry fee
  checks, `cMsgHUDYes`, `PlayerAcceptChallenge`), code 3 decline (`cMsgHUDNo`,
  `PlayerDeclineChallenge`).

### 4.5 Message handling

`HandleCaughtMessage` (0x825de4e0) by message type: 1 unlocks screen, 2 NIS, 3 execute
challenge, 4 speech, 5 Skate 101, 6 throwdown events, 7 cellphone moment, 9 teleport moment,
10 HUD component, 11 debug text, 12 Big Black (message id 256 spawn / 257 despawn; other
type-12 messages only update settings), 14 skater events, 15 FMV, 16 BLOG news, 18 dialog.
Ported: 2, 3, 10, 12, 15, 18. Each message first updates its message settings.

## 5. Next steps (in order)

The current order is in "How to continue" at the top. The steps below are kept for their
commands and fixtures.

1. **Build** (a full rebuild takes long and needs several GB): `build-release.bat`, or
   `cargo build --release -p skate2-pc -j 2`.
2. **Run the career tests** (game data required; non-incremental saves disk):
   ```
   set SKATE2_GAME_ROOT=<game folder>
   set CARGO_INCREMENTAL=0
   cargo test --release -p skate2-core --lib challenge::tests:: -j 4 -- --include-ignored --nocapture
   ```
   Expected PASS: compiler_reproduces_every_shipped_progression_handler,
   new_career_start_state_machine_reaches_first_mission, first_mission_state_graph_runs,
   first_mission_objectives_progress, first_mission_completes_and_autosaves,
   career_autosave_round_trips_through_the_profile_store,
   continued_career_start_location_is_authored, sign_up_starts_the_next_mission (and
   `challenge::attrib_db::tests::reads_first_mission_and_start_state_machine`).
3. **Live-check the sign-up** with a scratch save that has mission one done and a valid
   orientation at `m_1a_03_tutorial_fliptricks_challengelocator01`, plus a controls demo that
   presses D-pad up after the prompt (≥15 frames) and A on the poster. (A scratch fixture can be
   produced by a temporary test calling `complete_first_mission`, then
   `EditSkaterProfileStore::open(<scratch>/SKATER.P).save_career(..)`, as done on 2026-10-02.)
4. `Output::PinPlayer` (`cMsgPinPlayer`): hold the local player during sign-up.
5. Breadcrumb orientation for saves (§4.3): ported; live-check that Continue Career resumes at
   the last save's crumb.
6. The real HUDs: `hudobjective2` (ObjectiveHUD `Refresh` 0x826bc800), the sign-up prompt
   (`SignUpHUD`, Apt `eSk8FEScreenState_SignUpHUD`), the poster (`FrontEndState_SignUpPoster`,
   screen 19/state 32) and popups (`sk8popup`) are done (2026-10-03); the results screen (FE
   state 8) is not.
7. Play through `m_1a_03` and the following missions; port each missing native the scripts hit
   (check `Output::ScriptError` / "career script error" lines), sponsors, unlockables, awards.
8. Later: AI competitors, film/photo, services, global activities, then Party Play.

## 6. Running the game against a scratch save

Without `SKATE2_SAVE_ROOT` the game reads and writes `saves\SKATER.P` under the folder it is
started from, and `play-quiet.bat` does not set it: always set it for career runs. A missing
`SKATER.P` is a first-use state (`EditSkaterProfileStore::open`), so an empty scratch folder
also starts a new profile. Without `--live-controls-demo` the game starts at the main menu with
the controller.

```
copy <a PC-created SKATER.P with only CAC> <scratch>\SKATER.P     (new profile → New Career)
set SKATE2_SAVE_ROOT=<scratch>
set SKATE2_GAME_ROOT=<game root>
set SKATE2_TRACE_CAREER=1        (career outputs, objective changes, SignPost load, continue spot)
target\release\skate2-pc.exe --live-skating --live-controls-demo career   (scripted menu/editor demo)
target\release\skate2-pc.exe --live-skating                               (play by hand)
```

`SKATE2_TRACE_CAREER_LUA=1` prints Lua `print` output. `SKATE2_DIAGNOSTIC_SPAWN_LOCATION=<BAM_Sim.loc
name>` picks the boot spawn. Inspect a save with `python tools/career/dump_signpost_save.py <SKATER.P>`.
A store commit replaces `SKATER.P` in one rename and leaves nothing beside it
(`docs/evidence/save-replacement-2026-11-22.md`); builds before 2026-11-22 left a
`SKATER.P.<pid>.<n>.bak` per commit.
