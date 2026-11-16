# Skate 3 physics: the 1.0 "3 Shuv Bounce" (PC port option)

Physics Settings' **3 Shuv Bounce** row (Skate 3 physics only, On by default) runs
the Skate 3 player's KnownAir state selection as the base 1.0 `default.xex` did, instead of TU3's.

The evidence comes from a function-matched diff of the base 1.0 `default.xex` and the TU3 image
(functions paired by their `.pdata` bounds and normalised instructions). No 1.0 runtime trace
exists.

## What TU3 added

All four items are **exact** (read in both images' listings).

| # | TU3 | 1.0 | Change |
| --- | --- | --- | --- |
| A1 | `CalcSuggestedState` 0x82D8ADE8, KnownAir 82D8B5B0..B5DC | 0x82D5FF10 | `trajectory collision time (2772) < 0.05 s` (0x3D4CCCCD) and (OB_Air 2476 0x80 or OB_Traj 2476 0x8000) -> WipeoutGround 300 |
| A2 | same function, 82D8B69C..B6BC (landed, after the 2472 0x4 PhysicsAir test) | absent | OB_Unmounting 2480 0x40000, OB_Dismount 0x20000 or the preceding tick's OB_Dismount 0x10000 -> WipeoutGround 300 |
| B | `PhysicalPlayerHiLOD::Input` 0x82DB4048, 82DB4128..4138 | 0x82D89368 has no such store | Processed2480 0x10000 <- Player1296 0x20000 (`publish_player_flags`) |
| C | PostInput leaf 82DB5DDC..5DE8 | absent | Player1296 0x20000 <- Processed2480 0x20000, every tick (`run_post_input`) |

The jump target of A1 and A2 is `li r3,300` (1.0's KnownAir has no path to it from these flags).
A scan of every `lwz 2480` in TU3 finds A2 as the one direct reader of the 0x10000 bit
(**high**: a copy of the whole word into another struct would not show in that scan).

## What the option changes

`StateSelector::v10_known_air` (vendor `skate-core` `player/selector`) skips A1 and A2.
`player_state/selection.rs` sets it every tick from `gameplay_modes::shuv_bounce()`.

B and C still run. Their only reader is A2, so they change nothing while A2 is skipped.

## Why it bounces (hypothesis)

This was first observed in a separate port of the same TU3 code.

1. In the air, Y is taken only while TimeToLand > 0.3 s.
2. Off a 360 pop shove-it, the grab's dismount clip starts late. So Y pressed about 0.4 s
   before the floor puts the touchdown inside the dismount clip: OB_Dismount is set, OB_Air
   is not yet.
3. TU3 bails there (A2). 1.0 lands into PhysicsGround with the board still kicked out (about 42
   degrees nose-up, no wheel in contact).
4. On a downhill, the next tick goes PhysicsGround -> PhysicsAir. The contact solver throws the
   rider up about 10 m, and they land riding.
5. On flat ground the same input just lands riding.

The size of the launch comes from the port's solver. It is not verified against a 1.0 console.

## Status

- Option, profile field (`SK82_PC::GameplayModes` tuple 7, stored as its Off so earlier saves
  read On) and the selector unit test `v10_known_air_skips_both_tu3_dismount_wipeout_gates`.
- The atomic is Off until Options publishes the profile's rows at boot, so tests and corpora
  keep TU3's selection.
- Played by hand in this engine (2026-11-15): with the option On the trick bounces, with it Off
  it bails.
- A scripted run did not reach the window:
  - About 41 ticks of air off `urban_downhill_1` (`SKATE2_DIAGNOSTIC_TELEPORT_TO=120:on:0.243:1308.9,139.6,-877.1`).
  - A grab started while the shove-it plays bails in the air in both modes; that is the air pose check, not A1/A2.
  - So there is no per-tick trace of the bounce yet. `SKATE2_TRACE_SKATE3=1` logs each state change
    (`skate3_state`) with the option's value, for when one is made.
- **Not in the option:** PhysicsGround Enter's entry speed (TU3 82D37538 vs 1.0 82D0C6D0)
  also differs between the builds; this engine keeps TU3's.
