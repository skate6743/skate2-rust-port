# Spot Battle: joining a battle in play — November 22, 2026

A PC port addition, by request. It is no part of the original game, which leaves a player who
joins a challenge in play in its pre-game lobby until the challenge ends.

## What it is

A player who joins a Spot Battle in play plays it from its next turn. The battle keeps its round,
its scores and its turn order; the joiner goes last in the round it joins, with no score. A
battle on round 2 has the joiner on round 2. The same holds for a member whose skater was not
yet in the game's world when the battle started. A member who left the battle (Leave Challenge,
or taken out of it) waits for its end as before. Other challenge types are as they were.

## Why the battle starts again

Every console runs the battle's script itself, and the consoles agree only while each takes the
same inputs at the same tick from the same start (`docs/online/facts.md`). A console that was
not running the battle has none of its state: the script's variables, its timers, its HUD, the
players' runs of facts and the coders they are read with. So the battle is not patched on a
console in the middle of a turn. Between two turns its state is small, and from there it starts
again for everyone, the joiner included, through the same load every challenge starts with.

## How

1. The server tells the battle's participants that a member waits to join
   (`ServerMessage::ChallengeJoining`), once a battle, as soon as that member's skater is in the
   game's world (`ServerCore::announce_joiners`, also run as a battle starts).
2. On a console that has heard it, the `countdown` `stateenter` that would start the next turn
   (`51700D2879657162`) starts none (`runtime::spot_battle_join`). The battle's state is read
   from the script's variables (`capture_spot_battle`): `CurrentRound`; `PlayerOrderMap` with
   how many of it have had this round's turn (`CurrentPlayerIndex`); `PlayerRankMap` with each
   player's `Score` and `IsBonusObjectiveComplete`; `HighestVote`. Players who left are left
   out. The battle then holds in `countdown`.
3. That console sends the state to the server with its players named by user id
   (`ClientMessage::Carry`, at most 512 bytes; `encode_carry`).
4. The server takes the first state a participant sends for the battle and loads the battle
   again, a new challenge of the game: its participants and every member waiting, the state
   ahead of the load for all of them (`ChallengeCarry`, then `GameReset(LoadChallenge)`;
   `ServerCore::restart_with`). The consoles that had not reached the turn's end leave the old
   battle there. No console goes on from its own copy of the state: all start from the one the
   server sent.
5. Every console reads the state for the new table (`decode_carry`): the carried players in
   their turn order, then the players who joined; the same for the ranks; a carried player no
   longer in the table left out.
6. The restarted battle runs its script from the start with two changes
   (`script_fixes::JOIN_SOURCE`): `setup` goes to `reset` without the flythrough, and `reset`
   ends with the state in place of the first round it set up: the round, the turn order and
   turns had, each player's score, rank and bonus objective, the leaderboard's rows as
   `sequencecomplete` sets them, the highest vote. `wait_for_players` and the first `countdown`
   then start the next player's turn. With every old player's turn had, the next turn is the
   joiner's, in the same round.

## Checked on the script

`initial` gives each player `Score` 0, `Rank` and turn its own index, and `PlayerOrderMap` and
`PlayerRankMap` the identity; online, `reset` leaves those and sets `CurrentRound` 1 and
`CurrentPlayerIndex` 0. `countdown`'s `stateenter` adds one to `CurrentPlayerIndex`, past the
last player adds one to `CurrentRound`, and skips players marked `IsDisconnected`. A finished
line (`sequencecomplete`, `01A10F679F02A963`) raises the player's `Score`, moves it up
`PlayerRankMap` past lower scores, and sets the board with `LeaderboardHUDSetInt` and
`LeaderboardHUDSetRank`. `PlayerData[].PlayerOrder` is written and never read.

## With Infinite Rounds and End Challenge

A restarted battle loads under the game's Infinite Rounds as any battle does. A battle its host
ended that is restarted before it reached its end is ended again as it starts
(`Game::end_again`); a console whose battle has taken the host's end holds no turn for a joiner.

## Limits

- What a restart does not carry: the highlight clips saved so far (the reel at the battle's end
  shows the clips since the last restart), and the bonus objective, which is drawn again (a
  player who had completed it keeps that).
- The state is one participant's. A modified game can send another, as it can report any result
  of its own.
- The restart shows as a short load between two turns.

## Verified

On a local server, each game from its own save root (2026-11-22):

- One player started a battle alone and a second joined during its second round. The first
  console held at the next `countdown` (round 2, its turn had), the server loaded the battle
  again for both, and it was in play 1.6 seconds after the hold, without the flythrough
  (`initial`, `setup`, `reset`, `wait_for_players`, `countdown` at one tick). The joiner had the
  next turn, in round 2; round 3 followed for both and the battle ended. After the restart the
  two consoles entered every state at the same ticks (271, 1172, 1293 ... 4087).
- With three players, the second joiner came in the same way while two played: the turn
  player's console sent the state (round 2, two of three turns had; in another run round 3, one
  of three), and the joiner went last in that round. The restarts took 1.1 to 1.2 seconds.
- A battle that had taken joiners in ended by its host's End Challenge on all three consoles at
  one tick, and a later battle of the same game started as any does, flythrough included.
- No script error and no leave on any console in these runs.
- Not seen live: a score other than 0 carried over (the scripted skater of these runs lands no
  line in the spot). The carried scores are covered by the tests alone
  (`a_joiner_goes_last_in_the_round_it_joins`,
  `a_restarted_battle_goes_on_from_its_state_with_the_joiner_last`).
- The tests: the server's (`a_spot_battle_in_play_takes_a_joiner_in_from_a_participant_s_state`,
  `a_restart_leaves_out_the_players_who_left_the_battle`,
  `another_challenge_in_play_takes_no_joiner_in`,
  `a_restart_of_a_battle_its_host_ended_ends_again`), the state's coding
  (`a_battle_s_state_reads_back_for_the_same_players`, `bytes_that_are_no_state_read_as_none`)
  and the script's (`a_state_that_does_not_fit_the_players_leaves_the_first_round`).
