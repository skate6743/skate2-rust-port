# 1-Up Round Lock (PC port addition)

Status on 2026-11-23: written and syntax-checked. **Not compiled, no test run, not played.**

A host's option for 1-Up games, beside a Spot Battle's Infinite Rounds in Host Options >
Challenge Options. Nothing here is the original's.

## What the host sees

- Challenge Options lists "Round Lock: Off", or "Round Lock: Round N" with a lock set. The item
  shows for a 1-Up that has round starts (below) and can be selected in lobby skate, with no
  activity proposed and no change of challenge counting down.
- A opens a popup titled "Round Lock": "Off", then "1", "2", "3" and so on, one for each round
  start the 1-Up has, opening on the lock as it stands. A sets it, B leaves it as it was. The
  Challenge Options menu stays up under it and reads the new lock once the server has said it.
- Every member's log says "The host has locked every round to round N." or "The host has turned
  the round lock off."

## What it does

Skate 3's 1-Up graph (`challenge_stategraph/1up`, which a linked Skate 3 data folder's 1-Ups run)
keeps the starts of its rounds in its local data: `StartLocationRotation` and
`WaitLocationRotation`. Its `setup_round` `stateenter` (`3A771A0DE883277C`) adds one to
`CurrentStartIndex` and to `CurrentWaitIndex` each round, going back to the first past the last
(a restarted round keeps its indices). `setup_run` then puts the run's player at
`StartLocationRotation[CurrentStartIndex]` and the others at
`WaitLocationRotation[CurrentWaitIndex]`, and with `UseStaticCameras` takes
`CameraLocations[CurrentStartIndex]`.

With the lock at round N the port's wrapper of `setup_round`
(`script_fixes::SKATE3_ONE_UP_SETUP_ROUND`, key `C00A2C1293F4FB42` =
`skate3/1up/3a771a0de883277c`) runs the handler, then sets `CurrentStartIndex` to N, and
`CurrentWaitIndex` to N where the 1-Up has a wait of that number. So every round's runs start
where round N's do, for every player. The number of rounds, the scores and the letters are the
1-Up's own.

"How many rounds" is the count of `StartLocationRotation` (`Skate3Data::one_up_rounds`). Skate
2's own 1-Ups, which are not in the map pool, pick each round's spot by the leader's line and
have no such starts: they get no Round Lock.

## How it reaches every console

- Host: `GameRequest(SET_CHALLENGE_OPTION)`, value `option::ROUND_LOCK | round << 8`.
- Server: taken in lobby skate, for a 1-Up game, from 0 to `option::MAX_ROUND_LOCK` (32; the
  server does not know a 1-Up's data), and not while the game changes challenge. It tells every
  member `ChallengeOption { ROUND_LOCK, round }`, and a later joiner after its `GameJoined`.
- The lock is one 1-Up's. The server lets it go when the host asks for a change to another
  challenge (`RequestChange`) and when the next challenge's attributes name another one; each
  member lets it go at the `GameChange` and `GameAttributes` that tell it the same, so no message
  is needed.
- At a 1-Up's load each console takes the lock into the challenge
  (`OnlineChallenge::round_lock`, read by the script through `Online.RoundLock()`), or none when
  the 1-Up has no round start of that number. The option only changes in lobby skate, so every
  console loads a 1-Up under the same lock.
- Protocol 17 is unchanged: an older server ignores the option and an older game ignores the
  message. A member on an older build would start rounds where the others do not, so every
  player of a locked 1-Up needs this build.

## Tests written (not yet run)

- `skate2-net`: `the_host_s_round_lock_reaches_every_member_until_the_challenge_changes`,
  `the_round_lock_is_a_one_up_s`.
- `ui::online_hub`: `round_lock_opens_a_popup_of_off_and_the_one_up_s_rounds`,
  `round_lock_is_a_one_up_s_and_waits_for_lobby_skate`.
- `challenge::script_fixes`: `a_round_lock_starts_every_skate3_1up_round_at_its_round`.
- `skate3` (real data): `one_up_rounds` against each online 1-Up's `StartLocationRotation`.

## To verify

1. `cargo check` and the tests above.
2. Two clients on a local server in a Skate 3 1-Up: lock to round 2, play three rounds, and see
   every run start at round 2's start on both; turn it off and see the starts take their turn.
3. The popup on screen with three and with more rounds.
