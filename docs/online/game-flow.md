# Game flow (`Sk8::Flow::StateOnlineGame`)

The client side of an online session, from "join this game" to leaving it. `StateOnlineGame` is
a flow state whose sub-states are member functions (`Enter`/`Update`/`SubstateExit`/
`RequestExit`/`Exit` events). It listens to three streams: server messages
(`OnMessage` 0x8276b060, [messages.md](messages.md)), Fesl GameManager events (`OnEvent`
0x8276ac88) and flow messages from the rest of the game (`FilterImpl` 0x827711a0).
Addresses are `sk82_na_m.xex`.

Hex-Rays note: `OnMessage` and `OnEvent` run on sub-objects at +0x34 and +0x2C, so the field
names it prints there are shifted; the facts below use the true offsets (read from the store
instructions), e.g. `mRecipeRequested` +0x5C, `mConnectedToGameServer` +0x52, `mBeKicked`
+0x14E.

## 1. The states

```
DoJoinOnlineGame ─► [GetUserProfile] ─► JoinGameState ─► UploadRecipeState ─► LoadLocalRecipe
                                                                                   │
           ┌───────────────────────────────────────────────────────────────────────┘
           ▼
       SetupGame ─► [LoadGame] ─► LobbySkate ◄──────────────────────────────────────┐
                                     │  GameReset(LoadChallenge)                    │
                                     ▼                                              │
                              ChallengeLoadState  ── GameReset(Challenge) ──►  ChallengeSkate
                                                                                    │
                       WaitingForGameToFinish ◄── local complete / AllPlayersComplete
                                     │ AllPlayersComplete
                                     ▼
                              SendPlayerResults ── FinalResults ──► DisplayFinalResults
                                                                          │ ExitPostChallenge
                                                                          ▼
                                              WaitForDisplayExitAndCheckAchievements
                                                                          │ GameAttributes
                                                                          ▼
                                                                      SetupGame (loop)
   any state: lost connection / error ─► LeaveGame / HandleErrorState ─► LeaveGameFinal ─► exit
```

## 2. Entering

`StateOnlineGame::OnEnterState` 0x8276a590: loading screen on, release the front-end pause,
attach the net listener, push presence "StateOnlineGame", `FrontEndManager::StartInGame`, open
the online HUDs, attach the online hub menu as a message listener, and create the online HUD
group (component list from an attrib collection).

`DoJoinOnlineGame` 0x82779d58 (the flow message that started it): reset; abandon any running
main challenge (`cMsgAbandonChallenge`, quitting); require a ready Fesl connection and a local
user id (else internal error → `HandleErrorState`). If the cached skill or overall rank expired,
`StateOnlineGetProfile` refreshes them first (`GetUserProfile` sub-state), then
`JoinGameState`.

## 3. Joining (`JoinGameState` 0x82779898)

`JoinGame` 0x8274cc88 posts a `Sk8::Net` input event by how the game was chosen:

| Flow message | Input event | Use |
| --- | --- | --- |
| `MESSAGE_JoinGame` | `IE_JoinGame` (13) with lobby + game id | PlayNow result, game browser |
| `MESSAGE_JoinGameByUserId` | `IE_JoinGame_UserId` (14) with a user id | join a friend |
| `MESSAGE_JoinGameInvite` | `IE_JoinGame_GameInvite` (29) with the invite | accepted invite |

`OnEvent(OE_GameJoined)` sets `mConnectedToGameServer`, queues every other player's peer id in
`mRecipesToLoad`, sends `GamePlayerAttributes` (overall rank, challenge skill, overall skill),
and works out the game leader (§8). `OE_GameJoinFailed` sets `mLeftGame` (→ `LeaveGame`).

Once connected, the state waits for the RB's `GameRecipeRequest` (`mRecipeRequested`):
`StateOnlineRecipe::DoUploadRecipe` → **UploadRecipeState** → `DoDownLoadRecipe(local peer)`
→ **LoadLocalRecipe** (sim paused, `Net::PauseGame`): the client uploads its appearance and
then downloads it back from the RB, so it renders itself from the same data its peers use.
See recipes.md.

## 4. Setting up a game (`SetupGame` 0x82778530)

Enter: close popups, pop front-end states, online timer HUD mode 4, loading screen, pause the
sim, drop the challenge HUD group, `mNewGameReset = 1`.

Update, once the RB's `GameAttributes` is in:

1. `IsGameSameVersion(GameVersion)`, else `SER_GAME_NOT_FOUND`;
2. `CheckChallenge(ChallengeKey)` (the challenge exists locally), else `SER_GAME_DLC_REQUIRED`;
3. a ranked game joined through an invite → `SER_GAME_INVITE_TO_RANKED_GAME`;
4. `SetupInitalGameInfo` 0x8275db08 (network game on, sync level, player 0 = local, game mode
   and challenge key from the attributes). It returns whether the challenge key changed:
   - changed: online free skate also sends flow message 39; `StateLoading::DoLoading(GameLoad)`
     → **LoadGame** (sim paused; on exit: local player index, streamer position,
     `InitPresStreams`) → `LobbySkate`;
   - unchanged: straight to `LobbySkate`.

## 5. Lobby skate (`LobbySkate` 0x82777820)

Enter: open the lobby HUDs, `EnterLobbySkate` (HUD on, lobby HUD, `mLobbySkate`,
`mFreeSkate` = challenge type is online free skate), presence.

Each update, in this order:

1. not connected → `LeaveGame`;
2. a new `GameAttributes` → `SetupGame` (the location or challenge changed);
3. a peer in `mRecipesToLoad` → `DoDownLoadRecipe(peer)` as a sub-state;
4. `mCurrentLoadSequenceNumber` (set by the RB's `GameLoadRequest`) differs from the last one
   sent → `SendLoadRequest` echoes it;
5. otherwise, with no front-end screen loading, a pending `GameReset` →
   `HandleLobbySkateReset`.

Steps 3–5 are the **join barrier**. A newcomer's recipe has to be loaded by everyone, each
client confirms with the load sequence number, and only then does the RB send the reset that
puts the newcomer into the lockstep. A reset is never applied while recipes are still loading.

`HandleLobbySkateReset` 0x82776f90 applies the reset ([lockstep.md](lockstep.md) §10):
`UpdateGameInfo` 0x827538c8 rebuilds the players (one `PT_Human` per peer id of the 8-slot
table, empty = -1, recipe attributes applied per slot, local index found by user id; quality
`GQ_Low` above 4 players), then by reset type:

| Reset | Action |
| --- | --- |
| 0 LobbySkate | `GameModule::ResetGame(newGame, ..., seed)`, release the online pause, `Net::ResumeGame`, wait for the streamer on a new game → `LobbySkate` |
| 1 LoadChallenge | → `ChallengeLoadState` (no sim reset) |
| 2 Challenge | close popups, `ResetGame(1, 1, seed)`, `StartMainChallenge(challenge key)` in an online challenge → `ChallengeSkate` |

`UpdateChyron` shows "waiting for players" while the lockstep stalls ([lockstep.md](lockstep.md)
§8); `CheckPendingGameInvite` runs in lobby and challenge states.

## 6. A challenge

**ChallengeLoadState** 0x82771a28. Enter: pause the sim (`PauseGame`), wait for the streamer
at the activity, build the challenge HUD group, clear `mMyGameResult`. When the challenge assets
and HUD group are loaded and the loading screen is done: send `GameChallengeLoaded` once. The
RB's challenge reset then moves everyone on together.

**ChallengeSkate** 0x82776a78. Enter: resume, reset the round state, log the session id.
Update:

- a *sync point* (flow message 17 from the challenge script, `FilterImpl`) sets `mInSyncPoint`,
  the NIS to preload and the respawn transform. When the loading screen, challenge hulls and NIS
  preload are ready, send `GameSyncPoint` once ("We are loaded, waiting for everyone else to
  sync up"); the RB's `GameSyncPoint` releases it (unpause; `cMsgSpawnPlayer` at the round's
  start location, which `PlayerUI::ReceiveSpawnMsg` turns into spawn command 3, so
  `SkaterManager::OnCommand` destroys the local skater and spawns a new one; `cMsgTeleport` to the
  respawn transform, off the board; `cMsgPostChallengeDisplayExit`);
- local completion (flow message 16 carries the result into `mMyGameResult`) or an
  `AllPlayersComplete` from the RB → `WaitingForGameToFinish`;
- an NIS start (flow message 4) runs `StateLoading` as a sub-state.

**WaitingForGameToFinish** 0x82775a78: send `GameComplete`, disable the pause menu, status
"COMPLETE"; on `AllPlayersComplete` → **SendPlayerResults** 0x82774f08: send `GameResults`
(`ReportLocalPlayerResults`); on `FinalResults` → **DisplayFinalResults** 0x82772e58.

`mMyGameResult` is only written while `mChallengeSkate` is set, so a client that leaves
`ChallengeSkate` early (an early `AllPlayersComplete`) reports whatever it had (Arcadia's
"wait for every GameComplete" rule exists for this).

## 7. Results and the next challenge

**DisplayFinalResults**: pause, push front-end state 8 (`EndChallengeOptions`, online display,
quit disabled), `SetNextChallenge`, and start the HTTP request
`SkateProfileUpdateAndGetNewBadgeAndRank(local peer, challenge type)`. When that request and the
save are done the quit button is enabled; a disconnect or exit request leaves. On the RB's
`ExitPostChallenge`: `UpdateNextChallenge` (the game host peer sends `GameResetAttributes` for
the next challenge plus its session id; ranked picks a random key of the same type),
`cMsgPostChallengeDisplayExit`, `cMsgHUDContinue` → **WaitForDisplayExitAndCheckAchievements**
0x82771e98: on the next `GameAttributes` (and challenge clean-up done) → `SetupGame`; if the
leader changed meanwhile, the new leader runs `UpdateNextChallenge`.

## 8. The game leader ("host" player)

`Net::GetGameHostPeerID` 0x82641098 = the first player in the GameManager list whose type is
participant (not observer); `Net::IsHost` compares it with the local user. The leader alone:
starts challenges (`GameRequest` 1), requests changes (`GameRequestChange`), toggles privacy,
kicks (`GameRequest` 4 to the target), picks the next challenge (`GameResetAttributes`) and sends
the session id. `DetermineGameLeader` 0x827548e0 runs on join, player join and player leave; a
change writes "host changed" to the event log and makes the new leader re-pick the next challenge
if results are showing.

## 9. Players joining and leaving

- `OE_PlayerJoin`: queue the joiner's peer id in `mRecipesToLoad` (once), re-determine the
  leader, rebuild the lobby HUD. The joiner enters the lockstep at the next reset (§5).
  `OnlineEventLog` prints "joined as skater/observer" at that reset.
- `OE_PlayerLeave` and `GameRemovePlayer` → `HandlePlayerLeave` 0x8276aa70: find the slot by
  peer id, issue a `PlayerUnSpawn` command (disconnected) through `SimController::DoCommand`,
  mark the backend player disconnected (its row stays for results), drop it from
  `mRecipesToLoad` and queue its recipe for removal, re-determine the leader. Because the
  unspawn goes into the local command stream it is applied at the same aggregated frame on every
  console, whichever console's command arrives first.

## 10. Leaving and errors

**LeaveGame** 0x8276f060: post `IE_LeaveGame`, pause. Kicked (`GameRequest` 4 received:
`mBeKicked`, `mShowDisconnectError`) → `SER_GAME_KICKED` dialog; dropped by the server
(`OE_GameLeft` with reason lost connection or game destroyed: `mDisconnectedByGameServer`) →
`SER_GAME_DISCONNECTED`; accepting the dialog returns to the front end. Otherwise →
**LeaveGameFinal** 0x8276a920: unload all recipes and the game (`UnloadGame`) if a load had
started, log the session end, exit.

**HandleErrorState** 0x8276f258 (flow error messages 6/11 and internal errors): leave the game,
close the chyron and loading popup, show the error (`SER_DISCONNECTED_FROM_EA_SERVR` for internal
errors), and return to the front end when accepted and no NIS is playing.

Errors raised along the way (`Sk8AppMan::ThrowError`, `SE_ONL_GAME_ERROR`):
`SER_GAME_FULL` (-612), `SER_GAME_REJECTED` (-605), `SER_GAME_NOT_FOUND` (-603) and
`SER_GAME_SERVER_ERROR` from `OnGameJoined` 0x827877b8; `SER_GAME_DLC_REQUIRED`,
`SER_GAME_INVITE_TO_RANKED_GAME`, `SER_GAME_KICKED`, `SER_GAME_DISCONNECTED`; PlayNow:
`SE_ONL_PLAYNOW_FAILED` with `SER_PLAYNOW_NO_RB`, `SER_PLAYNOW_NO_MATCH`, `SER_PLAYNOW_CANCELED`.

## 11. Flow messages `StateOnlineGame` filters

| Id | Meaning |
| ---: | --- |
| 3 | teleport/respawn: unspawn AI; if the presentation collection is not loaded, wait for the streamer |
| 4 | an NIS starts loading |
| 12–14 | join another game while in one (`mEnterNewOnlineGame`) |
| 15 | leader: send `GameRequestChange` |
| 16 | local challenge complete, with the result |
| 17 | challenge sync point (NIS to preload, respawn transform) |
| 18 | challenge clean-up done |
| 87 | leader: `GameRequest(ChangePrivateStatus)` |
| 112 / 113 | replay entered / left |
| 122 | an error |
