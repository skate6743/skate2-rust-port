# Online — challenge facts and pacing

How a challenge round's per-tick "facts" are scoped, coded and paced (protocol 6, unchanged in
7, 2026-10-16). Code: `crates/skate2-core/src/online/facts.rs` (scope and coding),
`crates/skate2-core/src/session/online/online_challenge.rs` (`Round`: pacing, notes, standing
frames), `crates/skate2-net/src/{protocol,server}.rs` (`FactsNotes`, `Vote`, `Busy`, the
inactivity kick).

## What a facts frame carries

A facts frame is the part of a skater's `MissionFrame` (`PhysOutModule_MissionSystem`'s
output) the challenge reads of *another* player. What only the owner's console reads of its own
skater is never sent:

- the objective natives' player state (`mission::player_state`: `ObjectiveLocomotion`,
  `ObjectiveTrigger`, `Tricks`, `Manual`, `Wipeout` and the rest read `world.player`, built from
  the local frame only), the stance flip, the HoM and location keys;
- `MissionSystem::Update`'s local branch (`cMsgSkaterPushed`, `cMsgSkaterAutoPumped`);
- fields no consumer reads at all (`cMsgLineTimerExpired`, `cMsgLineTimerStarted`,
  `cMsgSkaterManualPumped` and `cMsgWipeoutBodyTweakPerformed` have no handler in any online
  challenge and no internal consumer);
- trick names (`cMsgTrickComplete`'s consumers take `TrickAttribKey`), the bail's 1,264-byte save
  image (only the career's HoM statistics read it, game type 4 only), and the root position (only
  the map icons read another skater's; receivers use the tester's base).

Always sent: the trigger tester's three points and validity (every console runs every player's
trigger volume tests), `physics_requested_teleport`, `teleporting`, `is_regular_stance`,
`just_wiped_out`, `hom_bail_in_progress`, `hom_bail_completed` (`cMsgWipeoutOver` ends Spot
Battle and S.K.A.T.E. turns in every mode) and `is_teleport_finished` (see pacing).

## Scope by challenge type

The rest is grouped by its readers (`facts::group`). A challenge type sends a group only if one
of its online challenges reads it: their script handlers' messages and natives, and their
objective conditions' natives (a census of every online challenge's handler bytecode and
objective sources in the game data, 2026-10-16), plus the objective system's own message
consumers (`objectives.rs`: grind, air, trick, gesture, sequence, points, on/off board and
grabbing messages, per player's objective group).

| Type | Groups beyond the core | Why |
| --- | --- | --- |
| S.K.A.T.E. (4) | deck speed, air, air state, sequence, in sequence, sequence score, patterns | `aircompleted`; `Skate.MatchedPattern`, `Skate.GetPlayerSpeed`, `Trick.GetCompletedAir180Count`, `Trick.IsInAirWithValidLaunchTrick`, `Trick.GetCompletedSequenceScore`; no objectives |
| Best Trick (8), Jam (15) | objectives, surface speed, sequence types, HoM score | `Entity.GetSpeed`, `Trick.GetNumTrickType`, `Trick.GetHOMScore`, `Trick.GetCompletedAir*` |
| Spot Battle (9) | objectives, wipeout almost over | `wipeoutalmostover`; the bonus objective |
| Death Race (19) | objectives, grind distance, trick active, airborne | `Trick.GetCompletedGrindDistance`, `Trick.IsDoingTrick`, `Trick.IsSkaterInAir` |
| Hall of Meat (31) | objectives, wipeout almost over, HoM score | `Trick.GetHOMScore`; no `ObjectiveHOM` getters |
| 1-Up (32) | sequence, air state, airborne, grind state | `Trick.IsInAirWithValidLaunchTrick`, `Trick.IsSkaterInAirOrGrinding`; no objectives |
| any other (free-skate activities 42) | everything | `ObjectiveHOM`, `ObjectiveAir`, `ObjectiveContextual`, `ObjectiveAchievements`, `Grind`, `Gesture` conditions |

"Objectives" is the objective system's per-player input: finished grinds, airs (without their
contexts and thresholds), ended tricks, sequences, the sequence score, points, on/off board and
grabbing. The bail's detail (bone damage, bonus scores, durations, entities hit), the air's
contexts (`ObjectiveContextual`, skaters aired over) and the big-trick records
(`ObjectiveAchievements`) are read by free-skate activity objectives alone.

Every console runs the challenge on the *shared* frame (`FactsScope::shared`): out-of-scope
groups at their defaults, the owner's console included. A wrong scope therefore cannot desync
the consoles; it can only make the challenge miss something on all of them alike.

Played live on the S.K.A.T.E. scope (2026-10-16, two consoles, carroll-koston gap): five tricks
set over the gap (five distinct flips from five flick paths), a follower's matching copy,
missed copies spelling S.K.A.T.E., repeated tricks refused (`AddTrickToAlreadyBeenDoneList`), and
the win, every state at the same tick on both consoles.

## Coding

`FactsCoder` codes each frame against the player's previous one, losslessly:

- the smooth values the scope reads as the Elias gamma code of their distance from where the two
  values before extrapolate, in the order of their float bits (negative values below positive
  ones). A steady value costs one bit. The tester's points always travel; the deck speed only for
  S.K.A.T.E., the surface speed for Best Trick and Jam, the wipeout teleport timer for Spot Battle
  and Hall of Meat, and the bail's eight floats for no game type (only a challenge of a type
  outside the table, which reads everything). What a scope does not read stays at its default on
  every console and costs nothing;
- everything else as the XOR of a fixed layout (flags packed into one word) with the previous
  tick's, its zero runs skipped, and the layout length as a change;
- an absent event (grind, air, manual, powerslide, gesture, sequence) keeps its last value in the
  layout, so an event changes the layout once instead of twice.

An unchanged tick is about 4 bytes. Measured in a two-player Spot Battle (2026-10-16, the
turn diagnostics): each player sent facts for about half the ticks (its own turns), 8.5 bytes a
tick on average, about 0.27 KB/s over the challenge; with the replay frames a player took in
7.5 KB/s in 56 datagrams/s, against 10.0 KB/s in 114 datagrams/s with protocol 4. Protocol 7's
sealing adds 20 bytes a datagram (a Death Race: 8.1 KB/s in 53 datagrams/s).

## Pacing: who sends facts

A challenge tick runs once it has the facts of the players *pacing* it (`Round::contributors`).
In a turn-based challenge (S.K.A.T.E., Spot Battle, Hall of Meat, 1-Up), once the first turn has
started, those are the turn player (the last player `AI.RepositionAI` moved with the camera; every
turn-based script passes its shared turn player, `vars.ActivePlayerID`, `vars.LeaderID` or the
follower) and any player `AI.RepositionAI` teleported whose output has not yet finished the
teleport. Everywhere else every player still in the round paces it. Every other player stands
still at the frame of the last tick it sent facts for, its events cleared (`facts::standing`), and
sends no facts at all, only replay frames. Every console computes the set from the same challenge
state after each tick, so they agree. Pins cannot decide it: the scripts pin and unpin
`vars.LocalPlayerID`, a different player on each console.

So in a turn-based challenge the player whose turn it is paces the round, its countdown included:
the turn timer, turn changes and scores run at that player's simulation speed on every console,
and that player's own console runs each tick as soon as it produces it, so its HUD shows exactly
its own time. A slow or paused spectator delays nothing. In Best Trick, Jam and Death Race every
competitor paces the round, and each one's run is still measured in its own ticks; where a
console runs ahead of slower players, its HUD timers show its own position
(`TimerManager::display_lead`).

Measured 2026-10-16 in a Spot Battle with A's game slowed to 54 updates a second
(`SKATE2_DIAGNOSTIC_TIMER_HZ=54`) and B's at 60: the round ran at 54.0 ticks a second on both
consoles through the intro and A's turns and at 60.1 through B's, and both consoles entered every
state at the same tick and reached the same results.

A player who starts pacing again begins a new *stint* (counted alike on every console, sent with
each facts message); facts of an old stint are ignored, so a console that had run ahead before it
was pinned cannot mix old ticks into new ones.

All of this holds only while every console's challenge takes the same inputs at the same tick.
Two consoles a tick apart on a change of pacing wait on each other for good (a Hall of Meat turn
change one tick apart hung a 2026-10-16 run). So nothing a console does on its own clock may
reach the challenge: the NIS manager, whose scenes end in `cMsgNISComplete`,
`cMsgHighlightReelComplete` and `cMsgNISChallengeNotify`, updates once per round tick in an
online round (`update_online_round_nis`; `SimController::Update1` ran it every simulation
update, and the original's simulation updates were the lockstep's ticks) instead of once per
frame. A scene's or speech line's end still comes on each console's own time (loading the
scene, streaming the line), so in an online round the challenge takes those events only from the
pacing leader, the first player pacing the round (the turn player in a turn-based challenge):
every console logs its own, numbered in their kind's order (the round's NIS events in one run,
each speech line's ends in their own); the leader's facts announce the ones no tick has
delivered (`FactsNotes::presented`), and every console delivers each once, at the tick of the
facts announcing it. `SKATE2_TRACE_ONLINE_INPUTS=1` prints a running hash of every tick's frames every 60
ticks (`=<from>-<to>` also prints those ticks' frames), and `SKATE2_TRACE_CAREER` stamps each
state change with its round tick (`career graph <name> @<tick>`), so two consoles' traces can be
compared line by line (`tools/online_diverge.py A.log B.log`). Should two consoles still part,
one wait cannot become a hang: a player's facts arrive in order, so a later tick of its run
without the one a console waits for means the player began the run after it, and that tick runs
with the player standing (traced).

## Sync points

Best Trick and Jam stop at a sync point between rounds (`Online.OnlineChallengeRoundSyncPoint`),
and each console gets there at a tick of its own: `end_round` plays the local player's
highlight (`vars.Competitor[vars.LocalPlayerIndex].roundHighlight`), so the consoles leave it
apart. A console at its sync point is paused, so it sends no facts; the others, still on their
way, would wait for them, and the server let the sync point go only at its 45-second timeout,
after which the console that had kept skating stayed 45 seconds ahead of its round (2026-10-16).
So a console reaching its sync point first sends one more facts frame, its skater standing still,
with the `syncing` note: from that tick every other console runs it at that frame, without
waiting, until they are all there. When the server lets the sync point go, every console goes on
from the same tick, the start of the round's next run of ticks (`Round::base`, the sync points
so far times 2^20), its frames from before dropped.

## Notes: votes, leaves and sync points

The players watching send no facts, so what they do reaches the challenge through the notes of
the pacing players' facts (`FactsNotes`; `syncing` is the sync point's and `presented` the
presentation events', above): every console relays a vote press (`Vote`) to the
others, and each pacing player's facts carry every vote and every player who left that it knows
of and that no tick has counted yet. Every console applies them at the tick of those facts:
a vote sets its voter's `MissionFrame::vote` for `ChallengeVotingManager::Update`, a leave
disconnects a player who is not pacing (a pacing player who leaves is disconnected at the first
tick its facts are missing). A console holds a player who left by user id and numbers them by
the table the notes go out under: the challenge's table (`GameReset(Challenge)`) is packed again
without the players who left during the load, so a load-table index would name whoever moved
into it, and those players are not in the challenge at all. Applying a vote again is harmless,
so a pending vote travels until a tick counts it; pending votes are dropped once voting stops.
Measured 2026-10-16 with three players in Spot Battle: the two watching players' presses reached
the turn player, rode its next
facts, and all three consoles applied them at the same ticks (820 and 821); over a whole
three-player game every vote (18) was applied at the same tick on every console and each turn's
vote ended it alike.

## Inactivity

A game that sends neither a replay frame nor `Busy` for two seconds is kicked
(`KR_KickedForInactivity`, `leave::KICKED_FOR_INACTIVITY`). A game with no frame to send
(travelling behind the fade, loading, paused at a sync point) says `Busy` every quarter second; a
frozen or paused game (a hang, a debugger, Cheat Engine) says nothing and is kicked, and its
challenge goes on without it. Measured 2026-10-16: a game killed in the middle of S.K.A.T.E. was
kicked about 3 seconds after its last datagram (two seconds, then the server's next check); the
other player won by disconnect (`finish_reason` 1) and went back to lobby skate.

## Pinned gestures

Online, a pinned skater (a countdown before its run, waiting for another player's turn) can
still gesture with the D-pad, as in the retail game: the input fill runs and only its four gesture
intents stay (`GenerateActionGraphIntents` 0x825fda28 in the debug build skips the whole fill
while the Actor's `0x4000000` pin is set). The gestures reach the other players through the replay
frames. Offline pins behave as in the debug build.

## Datagrams

A tick's facts go to the network thread with its replay frame (`Client::send_tick`), so they
share one datagram where they fit; any other message sent first flushes the waiting facts, so
the reliable stream keeps its order.
