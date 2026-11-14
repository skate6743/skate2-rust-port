# AI skaters — native map and port plan

AI skaters are full skaters (Actor, physical pair, presentation) whose action-graph intents and
physics adjustments come from an `AIController` instead of a pad. They ride recorded paths:
ambient skaters around the city and behind the front-end wall, and the competitors, photographers
and filmers of AI challenges. Facts come from the debug executable `sk82_na_m.xex` (IDA,
read-only) and the Jan-2009 data. Addresses are debug-xex virtual addresses.

## 1. Pieces

| Native | Size | Role |
| --- | --- | --- |
| `AI::AISkaterManager` | 68 fns | ambient spawn/unspawn/model loading, challenge AI, distance callbacks, audio events |
| `AI::SkaterAssetManager` / `SkaterAsset` | 36 fns | the AI skater recipes (`characters_marquee`) loaded through `CharacterLoader` |
| `AI::SkaterProfile` | 11 fns | `ai_skater_profiles` vault record: abilities, trick weights, wipeout/celebrate odds |
| `AI::AIController` | 21 fns | one per AI Actor (`Actor::mAIController`); behaviour, move stack, motion controllers |
| `AI::FollowPathBehavior`, `WanderBehavior` | | push a `FollowPathMove`; Wander uses `PathManager::mAmbientFilter` and branch mode 1 |
| `AI::FollowPathMove` | | copies its settings into the shared `PathFollowing::Influence` each update |
| `AI::PathController` | 85 fns, 34 KB | path following: nodes, speed/turn/trick intents, physics input, wipeouts, respawn |
| `AI::NavMeshController` | 23 fns | the alternative motion controller (swap disabled: `kbEnableNavMeshControllerSwap` 0) |
| `AI::ObstacleAvoider`, `Obstacle` | 25 fns | avoidance of DMOs, pedestrians, vehicles and skaters along the path |
| `AI::OnBoardMotion`, `OffBoardMotion`, `ToggleOffboardState` | | intent helpers |
| `AIPath::PathManager` | 30 fns | the path database (arena kind 27 assets), queries and filters |
| `AIPath::Path`, `Node`, `AIPathID` | 91 fns | recorded path data |

## 2. Per-frame flow

- `Simulation::SimUpdate1` calls `AISkaterManager::Update` 0x824d0188 (between input recognition and
  `RecipeMan.Update`): `PreUpdate` (clears each AI's `AIOut.mAudio`), `++mUpdateCount`,
  `SkaterAssetManager::Update`, `UpdateAmbientSkaters`, test skaters, distance callbacks, audio events.
  Nothing runs in an online game (`IsInAnOnlineGame`).
- `UpdateAmbientSkaters` 0x824cfe08: `ShouldPreventAmbientSkatersCurrently` → `UnspawnAllAmbientSkaters`;
  else `EvaluateMaxAmbientSkaters`, then on `mUpdateCount % 60`: 0 → `UpdateAmbientSkatersUnspawning`;
  with `kbEnableAmbientSkaters`: 15 → `UpdateAmbientModelLoading`, 30 → `UpdateAmbientSkatersSpawning`,
  3..12, 18..27, 33..57 → `CheckForBannedAmbientSkaterUsage`; `UpdateAmbientSkatersBehaviors` every frame.
- `Actor::Update` 0x825f10f0 calls `AIController::Update` first: `UpdateIsLocalWorldLoaded`, the
  behaviour's `OnUpdate`, `UpdateMove` (begin the top move, `OnUpdate`, pop on false).
- `Actor::GenerateActionGraphIntents` 0x825fda28: with an AI controller, `AIController::GenerateActionGraphIntents`
  (top move → `FollowPathMove::OnUpdateActionGraphIntent` → active motion controller) replaces the input
  listener's `Fill`.
- `PathController::GenerateActionGraphIntents` 0x824d3e00: ambient AI clear the pin bit; `PreUpdate`,
  `HandleWipeouts`; settled and not wiping out → `Update` 0x824d3998, else a cached-deck
  `Actor::ApplyAIPhysicsInput`; `GenerateGestureIntents`; `mPath` is a per-frame cache cleared at the end.
- `Actor::CreateAiController` 0x825f40b8 also disables the skater animation's accel/decel.

## 3. Front end

- Boot: `CommonBackEnd` seeds location `Z_TrainingStart`; `GameModule::OneOffLoad` registers
  `Z_TrainingStart_Fixed` at `[-2646.865967, 129.720886, -101.560486]`, initializes the stream focus there
  and calls `AISkaterManager::LoadGame`, waiting for `AreAllAssetsLoaded`. Its `ResetGame(1, 1, 0)`
  makes the first player's `PlayerUI`, whose `OnReset` 0x828aadd0 asks for a spawn.
- `PlayerUI::Update` 0x828af9f8, called by `DoGameUpdate` 0x82730418 on every frame the sim is not paused
  (the gate of its sync command too), sends that spawn: `ResetPlayer` 0x828ad280, then
  `SkaterManager::SpawnPlayer` 0x82600630, which needs no loaded character. The first player stands at
  the location as `GetLocalSimIndex`. Unfocused, its input unpacks with Brake held
  (`PlayerInputData::Unpack` 0x82a8c330). `StateMainMenu::FilterImpl` 0x82770510 also runs
  `PlayerLoader::Load` in the front end (message 0xF, a profile with a valid character).
- `GameModule::Update` 0x8273edc8 runs `SimController::Update1/2` while `FEBackEnd.mFEState` is IN_FE or
  IN_GAME and the load state is In_Game, so the simulation ticks behind the menu with the first player
  and the AI skaters.
- `FrontEndState_MainMenu::OnEnter` 0x82659fd8 calls `EnableAmbientSkaterSpawn(1)`.
- `GetCameraReferenceMatrix` 0x824ba008: in the front end the presented camera
  (`Presentation::mCameraPresentation`), otherwise the game camera.
- The menu camera (ICE take ENTER end) is at `(-2656.08, 107.14, -37.08)`, in Sim tile `cSim_-2700_-100`,
  whose path arena holds 88 ambient paths around `(-2650, 130, -84..-54)`.

## 4. Path data

- Ambient paths stream with the world: Sim tile arenas hold a `tAIPathData` object, TOC type `0x00eb0014`,
  registry kind 27 (`ProcessAIPathData` 0x82d5f498 → `PathManager::AddAsset`/`RemoveAsset`). 101 BAM
  tiles carry one. Challenge paths are `data\content\aipaths\AIPATH_Challenge_*.rx2` (`aipaths.big`,
  125 files, `LoadBinaryPathPackage` 0x824b8f28). `am_*.xml` loading is dev-only (`kbAmbientPathStreaming` 1).
- Self-relative pointers (`tAIPathData::Fixup` 0x829c8428, `tAIPath::Fixup` 0x829c7e40):
  - `tAIPathData`: `u32 count`, `Path*` (relative to the object), 96-byte paths;
  - `Path`: `m_pNodes` +0x30, `m_pExtraNodeData` +0x38 and `m_pBranchGroupVector.mData` +0x3c relative
    to the path; each node's `mExtendedNodeData` +0x20 relative to the node; each branch group's
    `mBranches.mData` relative to the group.
- `Path` (96): `m_BBox` (two Vector4), `m_ID` (16), nodes/count, extended data, branch groups
  (pointer, count), `m_BitFlags` u32, `m_AllowedSkaters` u64, `m_SkillLevel`, `m_ExtraData1..3`.
- `Node` (44): `mPos` and `mDirection` (3 f32 each), `mBoardOrientation`/`mSkaterOrientation`
  (`CompressedQuaternion`: byte × 1/127.5 − 1 per lane, then normalized), extended data pointer,
  `mFrameCountSinceLastNode`, `mNodeWidthLeft/Right`, `mEventType`, `mBitFlags`, `mExtraData1..3`.
- `ExtendedNodeData` (40): trajectory start position, start velocity and offset (3 f32 each),
  `mTrickScorable` i16, `mAirSpin180Count` i8, `mBitFlags` u8.
- `BranchGroup` (12): branches (pointer, count), `mSourceNodeIndex`. `Branch` (24): destination
  `AIPathID`, destination node index, `mSourceNodeRatio`.
- `AIPathID` (16 bytes): byte 0 is the progression branch, 0 for ambient. Ambient filters compare the
  words at 4 and 12 (`CompareAgainstFilter` 0x824c22d8); challenge IDs hold stage, part, challenge type,
  challenge index, a description from byte 5 and a 16-bit suffix at 14.

## 5. Port status (2026-10-08, recovery updated 2026-10-11)

October11 recovery continuation: `Actor::TeleportSomewhereAppropriate`
`0x825f0e18` now calls the live AI owner through
`AIController::GetAppropriateRespawnTransform` `0x824cc208` and
`PathController::GetAppropriateRespawnTransform` `0x824ca430`. Challenge AI
selects an authored locator or a path node, temporarily resolving a cleared
path cache; a missing path continues to breadcrumbs. Ambient unspawn uses
both native client/backend type gates and runs at the reached call point.
The four respawn policy cvars are live. See
[recovery evidence](../evidence/deathrace-ai-respawn-2026-10-11.md) for source
branches, validation and limits. The following older list records October8
coverage; later challenge/UI work is tracked in STATUS and October11 evidence.

Ported, and run in the front end and in Free Skate:

- PC port addition: Free Skate's options have an AI Skaters row (On or Off, on by default) above
  MongoCorp Zones. Off, `ShouldPreventAmbientSkatersCurrently` also holds for that free skate
  (`AmbientFrame::ambient_off`), so no ambient skater loads or spawns. The setting lasts while the
  game runs, as the screen's other settings do, and binds only the free skate it starts: every
  game start sets it again (`set_game_mode`, profile free skate only), and the front end's world
  never takes it, so the main menu, the career, party play and online keep their skaters.
- `AISkaterManager`'s ambient update (spawning, unspawning, model loading, banned usage,
  behaviours), `SkaterAssetManager`, `SkaterProfile`, `AIController` with `WanderBehavior` and
  `FollowPathMove`, `PathController` (`ai/path_controller.rs`), `PathManager` and the path data
  (`ai/path_manager.rs`, `ai/path.rs`).
- `ObstacleAvoider` (`ai/obstacle_avoider.rs`): `AddSkaterObstacles`, then the Living World
  obstacles the session gathers for each AI skater before its update (`ai/living_world_obstacles.rs`:
  `AddPedestrianObstacles` 0x824c4ab8, `AddVehicleObstacles` 0x824c4f00 and `AddDynamicObstacles`
  0x824c52a0 with `IsObjectInDefaultLocation` 0x824bc8a8), `TestObstacle`, `Obstacle::Set` with
  `FindOnPath`, `CalculatePathObstruction` (skaters and pedestrians) or `CalculatePathObstructionBox`
  0x824c3a70 (vehicles and dynamic objects), `TestIfApproaching` with its time to collision,
  `TestIfSkitchable` 0x824bc100 and `CalculateMinMaxAvoidanceSpeeds`, then `CalculateSpeedAdjust`,
  `CalculateSkitching`, `CalculateSafePosition`, `PickAvoidanceStrategy` (with the dynamic object
  run-over, jump-over and give-up decisions) and `PickOptimumAvoidanceWidth`.
- The obstacle gathering per kind: up to 20 pedestrians within the gathering radius (`ai_skater`
  0x7c1d006c) with an initialized core, boxes on their model's width and length,
  `kPedestrianHeightInMeters` (1.6) tall; up to 20 vehicles within 2.5 radii, their collision
  model's box; up to 20 dynamic objects within 2 radii weighing at least `physics_skeleton`
  `SmallObjectMassThreshold`, their model's box placed at its centre, run over or jumped per the
  characteristics' `CanAISkatersRunOver`/`CanAISkaterJumpOver`. Challenge skaters
  (`kChallengeSkatersIgnoreDMOsAtDefaultLocation` 1) pass objects still at their locators; ambient
  ones (the ambient cvar is 0) do not.
- Physics: `Skateboard::UpdateAIPathFollowing` 0x82b35fe8, `PhysState_FollowPath` (105) and the AI
  branch of `TrajectorySelector::Launch` (`AIPathIsUseable` 0x82db6100, `LaunchAIPath`
  0x82db94f8).
- Actor: `ApplyAIPhysicsInput`, the AI's intents in `GenerateActionGraphIntents`,
  `TeleportSomewhereAppropriate` unspawning an ambient skater, `SpawnAi`/`UnspawnAi`. Destroying a
  skater runs `SkaterAnim::PurgeFacePose` 0x82ca9b38 and, for its presentation,
  `SkaterPresentation::UnInitAnimation` 0x827e2d90.
- `UpdateAudioEvents` 0x824c3668: a skater's wipeout, respawn or big trick reaches the AI skaters
  within 15 m (`Actor::CanSeeSkater` 0x825ed758) as `AIOut_Audio` events, which drive the AI's
  celebration gesture and, through `AudioPresentation::FillSkaterData`, its audio record (with
  `can_see_player` against the local skater).
- Front end: the world ticks at `Z_TrainingStart_Fixed` behind the main menu. The first player stands
  there pinned (braking, as the native's unfocused input does) and hidden; the AI skaters draw as later
  players and see it like any skater. `DoSimSubmit` gives every skater to the audio with the first
  player local, so `MicrophoneMan::UpdateCameras` 0x824f2728, which needs a local skater, moves the
  camera microphone to the menu camera and the AI within `MaxSkaterDistance` (30 m) are heard. The
  WorldPainter audio keys come from the first player's hips.

The ambient spawn window (4 to 30 m from the camera) admits rooftop path starts that
`UpdateAmbientSkatersUnspawning`'s 20 m height limit removes at once; the native does the same.

Ported for missions (`m_1a_07_follow_slappy` is the end-to-end case):

- `ChallengeAIManager` with the AI half of `cChallengeCompetitorsManager` (`challenge/ai.rs`): the
  competitors and their `ai_characters`, the static challenge path packages (`aipaths.big`), the
  round path filter, the respawn location, conversion to ambient and `UnloadExcessSkaterModels`
  once the main challenge completes. Every Lua `AI` native the shipped scripts call but
  `FollowTrickPattern`.
- `PathController`'s follow and race rubber banding, `AISkaterManager`'s distance callbacks
  (`cMsgPlayerRadiusEnter`/`Exit`), and the Challenge port events the career drains.
- `ChallengeFTLManager` (`challenge/follow_the_leader.rs`), `cChallengeRaceManager`
  (`challenge/race.rs`), `cChallengeTeleportManager` with AI teleports through
  `Actor::SetTransform` (`challenge/teleport.rs`) and `cVisualIndicatorManager`'s groups and banks
  (`challenge/indicators.rs`).
- The race's end: `FrontEndState_LeaderBoard`'s three leaderboards (`challenge/results.rs`), and
  the screens `FrontEndState_EndChallengeOptions` opens without several players (42
  `endchallenge_small`, 45 `challengefailed_small`) beside 41 and 47.
- The `DistanceMeter` HUD component (`challenge/turn_hud.rs`) and its `hud2/huddistance` movie.

Not ported:

- `AI.FollowTrickPattern`, and 140 other natives of other challenge types
  (`tools/career/luacalls.py` lists each handler's calls; nested functions need a recursive walk).
- The visual indicator renderer (`GetVisible*Indicators`: ribbons, the follow arrow), the end
  screens with several players for anything but the place/name/int layout (41 with race times,
  43, 44), the results pages' Next and Previous, mission speech and music audio, and
  `ChallengeManager.SwitchChallengeCamera`'s camera.
- `AISkaterManager`'s test skaters, `NavMeshController`
  (`kbEnableNavMeshControllerSwap` is 0) and `PickOptimumBlockedWidth`
  (`kbDoBlockedNavmeshTest` is 0).

`HUD2.ShowChallengeCompleteMessage` and `ShowChallengeFailedMessage` show nothing in the native
either: their screen states 6 and 7 reach hudcentral's `mcCongrats`, which the shipped movie lacks.

Port choices:

- A skater mesh with the pedestrian `defaultlivingworld` effect (`ai_skater_06`'s
  `character.livingworld_pedestrians`) draws with colorize (0.5, 0.5, 0.5, 1).
  `SkaterPresEntity::BindConstants` 0x827dcf50 binds no colorize and `cScope::DispatchParamsHelper`
  0x82e14aa0 uploads only bound params, so the native register keeps the previous draw's value.
