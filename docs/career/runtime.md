# Challenge and progression runtime (native behaviour)

All addresses are debug-xex (`sk82_na_m.xex`) virtual addresses.

## Frame placement

| Step | Native | Notes |
| --- | --- | --- |
| Pre-update | `Simulation::SimUpdate1` → `Sk8ChallengeSystem::PreUpdateSubmit` (`0x825a2ba8`) | activity sub-controllers `PreUpdateSubmit` |
| Update | `Sk8ChallengeSystem::Update` (`0x825a3f10`), from SimUpdate1 when `kRunChallengeUpdateDuringPhysics`, else SimUpdate2 | activity manager (`ProcessPendingResetList`, `PerformBBoxTests`, sub-controller `Update`), online, voting, race, invisible boundaries, collision, effects, FTL, then `cSignPostManager::Update` |
| Progression toggle | end of `Sk8ChallengeSystem::Update` | when `ProgressionEnabled` differs from cvar `kEnableProgression`: copy it, `ReevaluatePrereqActivities`; in career (`GameModeFlags == 4`) set `StartSignpostNextFrame`. Next frame: `cSignPostManager::StartGameSession` + `ReevaluatePrereqActivities` |
| Post-update | `Simulation::PostUpdate` (`0x8289ff24`) | challenge `AssetManager::Update`; then, unless paused (`PauseMan` flag 4), `EventSys::Flush(queue 7)` delivers every queued Challenge message and event, including ones queued during the flush; then `HighlightUpdate` |

## Message kinds

A Hermes message kind is `stringhash32(class name)` (lookup2, seed `0xABCDEF00`,
`0x82985a40`), e.g. `cMsgStateEnter` = `4040627377`, `cMsgHUDYes` = `3836229679`,
`cMsgTerminateStateMachine` = `0x86D1A5E1`, `cMsgInitAutosave` = `1200268441`.

## Progression (`cSignPostManager`, `packages/sk8/signpost`)

* Lua state: `cLuaStateManager::CreateState` (`0x82aae998`) — `luaL_openlibs`, `print`
  replaced, `PerformLuaCBinding` (`0x825e4d58`) binds `Progression` (63 functions),
  `Achievement`, `Award`, `Unlock`, `ChallengeAsset`, `GameMode`, `DynamicHull`,
  `ChallengeSpeech`, `NIS`, `Service`, `Teleport`, `SessionMarker`, and the globals
  `ChangeState`, `Terminate`, `TerminateWithoutSave`; then metatable accessors and the
  `cMsg*` posting functions (`BindAllMessageExportersToLua`).
* Natives read the manager from upvalue 2 (`lua_upvalueindex(2)`); the running machine
  is the state's closure data (`SetClosureData`).
* `StartGameSession` (`0x825e4b48`): every `progression_statemachines` collection with
  `Starting` set is queued (`QueueTriggeredStateMachine` only queues when not
  `DuringInit` and `GameTypeFlags == 4`).
* `Update` (`0x825e2ce8`): flushes cached load-screen messages when not loading; when
  the flow manager is not in `STATE_Loading`, pops **one** triggered machine and
  `RunStateMachine`s it.
* `RunStateMachine` (`0x825e15c0`): terminates a running instance with the same key,
  creates `cSignPostStateMachine`, and for every state in `States` builds a
  `cSignPostState` from up to nine handlers, found by handler `Name`
  (`GetMessageHandlerKeyFromName`): `cMsgStateEnter`, `cMsgNISChallengeNotify`,
  `cMsgHUDComplete`, `cMsgHUDContinue`, `cMsgHUDNo`, `cMsgHUDYes`, `cMsgSpeechEnded`,
  `cMsgTimerExpired`, `cMsgFMVFinishedPlaying`. The `InitialState` becomes `nextState`,
  then `Execute(StateEnter)`.
* `cSignPostStateMachine::Execute` (`0x825d0b78`): while `nextState` and not
  terminating: if `nextState` differs from `currentState` it becomes current and the
  script type becomes StateEnter; clear `nextState`; run the current state's handler
  for the type. `ChangeState(name)` (`SetNextState`, `0x825d0a20`) finds the state by
  `Name` and sets `nextState`; more than 500 changes terminate the machine.
* Message receivers (`ReceiveHUDYesMessage` etc.): for every running machine, set
  `nextState = currentState` when empty, then `Execute(type)`.
* `Terminate()` / `TerminateWithoutSave()` set the terminate flag (autosave on/off).
  When the handler returns, `cMsgTerminateStateMachine` is sent; its receiver deletes
  the machine and, for `Terminate()`, calls `InitiateAutoSave` (`0x825a66c0`), which
  sends `cMsgInitAutosave` to the front end (career mode, not demo). **This is how the
  career saves.**
* Handler code: the handler layout's `tLuaScript` (offset 12) is Lua *source*, wrapped as
  `function h_<key>() \n<source>\n end`, compiled with `luaL_loadstring` and called with
  no arguments (`cSignPostScriptHandler`, `0x825c6320`/`0x825ca6c0`). `<key>` is read
  from the `cSignPostState` before its constructor stores it (an uninitialised read), so
  all of a state's handlers share one name. The Vault also ships each handler compiled
  (`Attrib::Blob`, defining `h_<StringHash64(Name)>`); all 910 blobs match their source
  literals. The port compiles the source with its own Lua compiler, as the native does
  (`compiler_reproduces_every_shipped_progression_handler` checks the output against every
  shipped blob), and names each function `h_<handler key>`, keeping handlers distinct; it
  does not reproduce the shared-name aliasing.

## Challenges (`ChallengeRuntime`, `packages/sk8/challenge`)

* Handlers are compiled blobs defining `h_<collection key>`; `Attach` runs the chunk
  once, then each delivery calls `h(data, message, vars)`:
  `data` is a `MultiDataProviderMetaTable` userdata over the activity's Vault data,
  `message` is the message table, `vars` is the per-activity table in the registry
  (created with `kActivityID` = the collection key as a float).
* `cLuaPostOffice::RouteMessage` (`0x82ce5a98`): message kind → subscribed state
  graphs (registration order). `HandleMessage` runs the current state's handlers for
  the kind, then the `global` state's.
* `ChangeState` (`ChangeStateFromScript`, `0x82ce4028`) allocates a `cEventChangeState`
  on queue 7 — the change happens when the queue is flushed. `EnterState`
  (`0x82ce67a8`): local `cMsgStateExit` to the old state, re-register handlers, local
  `cMsgStateEnter` to the new one. `Run` queues `ChangeState("initial")`.
* Attribute arrays are userdata (`LuaAttribArrayInfoMetaTable`): `arr[i]` (1-based)
  and `arr.Count`.

## Session marker around challenges

* `cChallengeTeleportManager::TeleportPlayer` (`0x82dad8e0`) with `repositionMarker` sends
  `cMsgSetSessionMarker` at the location's spawn point as the local player arrives
  (`TeleportPlayerNow` `0x82da6a00`). Scripts ask for it through
  `Teleport.TeleportLocalPlayerToLocationName` and `AI.RepositionAI`; Create-a-Spot's
  `TeleportToSpawnPoint` always does. The sign-post, map-select, presentation and chase
  teleports never do. `SessionMarker.SetSessionMarker` (`0x82da92d0`) puts the marker at the
  local skater's respawn transform. `PlayerUI::ReceiveSetSessionMarkerMsg` (`0x828aab38`)
  forces both, on the board.
* A new game (`GameModule::ResetGame` `0x8273a680` with `newGame`) makes a new marker, and
  `PlayerUI::ResetPlayer` (`0x828ad280`) puts it at the spawn while it is not valid.
* `cMsgClearSessionMarker` clears it. It is sent by `MapBackEndManager::TeleportToChallenge`
  (`0x8256dc10`) after its teleport, `cTeleportManager::TeleportLocalPlayer` (`0x828a9f40`)
  behind a load screen outside a network game, `TeleportLocalToWorldLocation` (`0x828a9cf0`)
  with an additional-world operation, and `SessionMarker.ClearSessionMarker` (`0x82da9350`).
  `cChallengeSimManager::CompleteChallenge` (`0x82da78e8`) also sends it for a main challenge
  other than a Flow (type 44) whose `challenge_local_data.ClearSessionMarkerOnExit` is set.

## Saving (`SKATER.P`)

`MemcardProfile::GenerateSaveData` (`0x82569f48`) writes, under `MEMCARD_ROOT`, the save
version inside the first savable's nested chunk, then one nested chunk per savable in
registration order. Leaf chunks are `StartWriteChunk(name, version)` + tuples
(`u32 size` + bytes) + a five-byte zero terminator. Chunk ids are `ICELITE::StringHash`
(`h = h * 33 + c` from `0xFFFFFFFF`).

| Savable | Leaves |
| --- | --- |
| `SKATER_PROFILE` | `ProfileBackEnd::Settings` v3, `ProfileBackEnd::ProfileData` v3 (name[32], nickname[32], `mNewProfile` int, peer id 8 bytes), `ProfileBackEnd::MusicPlayer` v2 |
| `SignPost` | `cSignPost::Data` v14 (`SaveData`, `0x825cdef8`), `cSignPost::Position` (`SaveProgressionOrientation`), `cSignPostMessageMan::Data` |
| `UNLOCKABLES` | `UnlockablesBackEnd::Data` |
| `STATISTICS` | `StatsBackEnd::Data` |
| `CAC`, `online`, `DYNAMICOBJECTS`, `Sk8ChallengeSystem` | |

The port writes `ProfileBackEnd::Settings` (native defaults) and `ProfileData`, and on
each career autosave the `SignPost` savable (format and load order in
[HANDOFF.md](HANDOFF.md) §4.2). It leaves `MusicPlayer` out, because playlist contents
need the music database. The native loader tolerates a missing leaf
(`ReadHashSkipUnknownHashValues`).
