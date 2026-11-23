# Skate During Turn Wait (PC port addition)

Status on 2026-11-23: the code type-checks (`cargo check` of the net, audio, core, game and
server crates, no new warnings). **Its tests have not been run, no release build holds it, and
it has not been played.**

Pull request 34 ("Brainrot Mode") is its base: skating through other players' turns of a
turn-based online challenge, with the local skater's own camera in a corner of the screen. That
change is in the tree under a new name, with three additions.

## What the pull request does

- Online Settings' third row, saved in `online-config.json`, off unless the file turns it on.
- While a turn-based round (S.K.A.T.E., Spot Battle, Hall of Meat, 1-Up) shows another
  player's turn from that player's camera, the local skater is unpinned every tick and skates on
  its own pad and game camera. Not while a scene plays, while the console watches through
  Spectate, or while the challenge teleports the local skater. The challenge's own pin
  (`cMsgPinPlayer`) is remembered and comes back when that stops.
- The round is not touched by it: only the turn's player and players being teleported pace a
  round (`online_contributors`, which leaves pins out), and a waiting player is read at its last
  facts without their events (`standing_frame`).
- The local game camera is submitted beside the shown one and drawn as a second eye into targets
  of its own (`gpu_renderer/pip.rs`: 30% of the window's width at the window's aspect, without
  PostFX), then copied into a corner at the left, just above the trick display.

## Changes made on top of it

| Change | How |
| --- | --- |
| Name | "Skate During Turn Wait"; the key is `skate_during_turn_wait`. The keys it had before (`brainrot_mode`, `skate_while_waiting`) are not read. |
| The corner over the HUD | Its copy into the window comes after the HUD's draw list and before the front end's fade, so a gap's or a trick's notice goes under it. No corner while the pause menu, Options, the music player or the online hub is up (`online_turn_wait_preview`): it would cover them. The skater skates on meanwhile. |
| Session markers through the wait | The game's marker is the wait's own while the skater skates through a turn (`Online::swap_turn_wait_marker`), under an online free skate's rules: LB with the D-pad sets it within the game's boundaries and goes to it. It is kept from one wait of the round to the next and goes with the round. |
| Invisible to the others | The replay frames a skater sends through a wait carry bit `0x0800` of the presentation's flags word (`remote_skater::TURN_WAIT_FLAG`); a console leaves a skater whose shown frame carries it undrawn and unheard (`online_remote_hidden`). |

### The marker, in detail

- `CreateSessionMarker` (`0x82729c28`) gives an online Spot Battle, S.K.A.T.E. and Death Race no
  session marker at all; a 1-Up has one, which its script sets and clears
  (`SessionMarker.SetSessionMarker`, `ClearSessionMarker`).
- So the wait's marker is its own object. The challenge's (or none) is held back while the
  wait's stands in, and takes the challenge's set and clear messages meanwhile
  (`send_session_markers`); it stands again, untouched, from the tick the wait ends. A turn
  therefore plays with exactly the marker the challenge gave it.
- A marker's set and use reach no online round's script: the Challenge queue's messages
  (`cMsgSetMarker`, `cMsgUseMarker`) are posted to the challenge system offline only
  (`career_step`). The script's `cMsgTeleportTransitionFinished` comes from the challenge's own
  teleports alone (`challenge/teleport.rs`), not from a marker's.
- A new game's `ResetGame` drops both markers.

### The flag, in detail

- Header word 48 (byte 192) holds the wipeout count in its high half and the flags in its low
  half: `0x8000` local, `0x4000` collidable, `0x2000` main camera only, `0x1000` teleported
  (`SkaterPresentation`'s writer and `UpdatePresEntity`'s readers). The word travels as it is
  in a replay frame. No presentation writes or reads `0x0800`.
- A game without this change ignores the bit and shows the skater as before (hidden only past
  4 m from its waiting spot). The protocol is unchanged.

## Not verified

1. The tests: `ui::options`, `ui::online_config`, `gpu_renderer::pip`, and the new
   `a_turn_wait_has_its_own_session_marker`.
2. Live, with two or three clients on a local server, in S.K.A.T.E., Spot Battle, Hall of Meat
   and a Skate 3 1-Up: skating through a turn, the corner over the HUD, a marker set and used in
   a wait and found again at the next, the challenge's marker in the skater's own turn, and the
   waiting skater unseen and unheard on the other consoles.
3. In a Skate 3 1-Up, the ticks between a player's own run ending and the next countdown: the
   camera is already on the next player while the old one still paces the round, so it is
   unpinned with its facts live. The script's objectives are off there; this is read, not run.
4. Cost: the scene and the shadow maps draw twice while the corner shows.
