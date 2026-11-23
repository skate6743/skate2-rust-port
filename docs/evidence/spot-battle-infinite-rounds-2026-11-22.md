# Spot Battle: Infinite Rounds and End Challenge — November 22, 2026

A PC port addition, by request. It is no part of the original game.

## What it is

- The host's **Host Options** hold **Challenge Options** in every challenge game (not in an
  online free skate). It opens where the game's challenge type has settings, and is greyed where
  it has none. A Spot Battle has one: **Infinite Rounds**, off unless the host turns it on. The
  item reads `Infinite Rounds: Off` or `Infinite Rounds: On`; A turns it the other way and the
  menu stays up. It changes in lobby skate with no activity proposed, as Match Setup does.
- Every member's log says `The host has enabled infinite rounds.` or
  `The host has disabled infinite rounds.` as it changes.
- A Spot Battle started with it on has no last round: after every player's third run comes a
  fourth round, and so on.
- While such a battle plays, the host's menu opens on **End Challenge**, its first item. A asks
  `Are you sure you want to end the challenge?`, Yes or No, opening on No. Yes ends the battle
  as its last round's end does: "challenge complete", the highlight reel, the leaderboard and
  the results. Every member's log says `The host has ended the challenge.`
- Since 2026-11-23 the host's Start menu lists **End Challenge** too, as its first row and the
  row it opens on, above Game Settings (and above Save Replay for a host waiting in the pre-game
  lobby), under the same condition as the online hub's item. Its confirmation is the Start
  menu's own (No above Yes, opening on No) with the same question; Yes closes the menu and sends
  the same request. The row and an open confirmation go the moment the battle is no longer in
  play or the player no longer hosts (`ui/pause_menu.rs`, `online_end_challenge_offered`).
  Unit-tested (`end_challenge_tops_the_online_hubs_while_the_game_offers_it`); not yet seen in
  a game.
- Such a battle is found and joined as any challenge in play is. A player who joins it does
  not wait for its end: a Spot Battle takes joiners in at its next turn
  (`docs/evidence/spot-battle-joining-2026-11-22.md`).

## How the rounds are lifted

The battle's script (`skate2_ots_online`, every Spot Battle's, a linked Skate 3 folder's too)
counts its rounds in one place. `countdown`'s `stateenter` (`51700D2879657162`) moves on to the
next player, and past the last player to the next round (`CurrentRound + 1`); then, where
`CurrentRound` is past `self.NumRounds` (3 in the data), it sends every player's overall result,
moves the last turn's player to the attempt spawn and changes to `challenge_finished` once that
teleport is done (`teleporttransitionfinished`, `A25636CBBBD33D97`, which makes the same
comparison). `challenge_finished` shows `CentralMessageHALIDChallengeComplete` for two seconds,
then `highlight_reel` and `leaderboard`. Nothing else reads `NumRounds` but a debug text and the
port's Skate 3 rule for a last run (`script_fixes::SKATE3_SOURCE`'s `IsLastRun`).

`self` is the script's data object, whose fields come from the challenge's record
(`MultiDataProviderMetaTable__index`). Under Infinite Rounds that lookup answers `NumRounds` with
infinity (`AttribHost::field_override`, `World`'s from `OnlineChallenge::infinite_rounds`), so no
round is past it and no run is the last. Once the host has ended the battle it answers 0, so the
next `countdown` finds every round played and ends the battle through the script's own end.

## How the end reaches every console at one tick

A challenge runs alike on every console only while each takes the same inputs at the same tick
(`docs/online/facts.md`). The host's end therefore travels as a vote or a leave does:

1. The host's game asks the server (`GameRequest(END_CHALLENGE)`); the server tells every member
   once (`ChallengeEnded`).
2. A console playing the battle holds the end until a tick counts it, and its facts say so
   meanwhile (`FactsNotes::ended`). In a Spot Battle the turn player's facts pace the round, so
   theirs carry it.
3. Every console takes it at the tick of the first facts that name it (`Round::next_tick`), and
   asks its challenge to end from that tick (`ChallengeSystem::end_infinite_rounds`).

The turn in play ends first, as the script ends it when its player leaves, with no one marked
gone (`script_fixes::END_SOURCE`): `challenge_running` loses its run timer and timer HUD and
changes to `end_turn` (its `playerdisconnected`, `C6964B10D9CE89DC`), `countdown` its countdown
and countdown HUD (`2526D9824B97BDD8`). `end_turn` stops the vote after its two seconds and goes
on to `countdown`, which ends the battle. A Skate 3 game's two seconds before a run ends go as
well, since `end_turn` takes any timer's end for its own. Before the battle's first turn its
script has no turn player to move (`reset` leaves `ActivePlayerID` nil), and its own end would
read one: the end waits, tick by tick and alike on every console, for the first `countdown`.

## How the option reaches every console before a battle

The host's game asks the server (`GameRequest(SET_CHALLENGE_OPTION)`), which takes it in lobby
skate only, in a Spot Battle game only, and tells every member (`ChallengeOption`); a member who
joins a game with it on hears it after `GameJoined`. A member's messages come in order, so each
has heard the option as it stands before the load of any battle it plays
(`GameReset(LoadChallenge)`), and every console loads the battle with the same rounds
(`Round::infinite_rounds`). Both requests are the host's alone: from another member they drop
the connection, as the other host requests do.

## Protocol

Protocol 17: `ServerMessage::ChallengeOption` and `ChallengeEnded`, `FactsNotes::ended`,
`request::SET_CHALLENGE_OPTION` and `END_CHALLENGE`, `option::INFINITE_ROUNDS`. A game and a
server of different protocols refuse each other, and the game then asks for its update.

## Limits

- A participant who used Leave Challenge waits in the pre-game lobby until the host ends the
  battle.
- The option stays with the game when its challenge type changes, and applies again when the
  game is a Spot Battle again.

## Verified

On a local server, each game from its own save root (2026-11-22):

- The host's menu: Host Actions lists Challenge Options under Session Access; it opens on
  `Infinite Rounds: Off`, A turns it on, and the log's first line reads `The host has enabled
  infinite rounds.` A player who joined later heard the option with the game.
- A battle of three started with it on went past its third round (`ID_CONTEST_CENTRALMESSAGE_
  ROUNDNUMBER` 5 after 4 minutes 52 seconds), every console in the same state at the same
  tick; a battle with it off ended after its third round.
- End Challenge: the item tops the host's menu while the battle plays; A opens `Are you sure you
  want to end the challenge?` on No. After Yes every console took the end at one tick (two
  players: 7293; three: 5278), the turn in play ended there, and `countdown`,
  `challenge_finished` ("Challenge Complete!"), the leaderboard and the results followed, with
  lobby skate ten seconds after the press. It did so in another player's turn and in the host's
  own. The log read `The host has ended the challenge.`
- The tests: the server's (`the_host_s_infinite_rounds_reaches_every_member_and_later_joiners`,
  `infinite_rounds_is_the_host_s_and_a_spot_battle_s`,
  `the_host_ends_an_infinite_spot_battle_once`), the hub's (`ui::online_hub::tests`), the
  round's (`the_host_s_end_rides_the_facts_to_one_tick`) and the script's
  (`the_host_s_end_ends_the_turn_in_play`, `the_host_s_end_waits_for_the_first_turn`).
