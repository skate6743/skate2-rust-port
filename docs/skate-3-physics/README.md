# Skate 3 physics in the Skate 2 Rust port

This folder documents the whole Skate 3 (title update 3, "TU3") physics as it runs inside the
Skate 2 Rust port: where every piece of code is, what it ports (with TU3 addresses), how it plugs
into Skate 2, which parts Skate 2 and Skate 3 share, and what was fixed against the real game.
It is written so the physics can be lifted into another host in one pass.

All paths are relative to the repository root. "TU3" means Skate 3's title update 3 executable
(`default.xex`), the version every address and measurement here refers to. "Skate 2" addresses
are Skate 2's executable. Both load at 0x82000000, so always note which game an address is for.

## Where it came from

1. **Skate 3 Rust Engine** (dumbad and contributors, GPL-3.0-only,
   https://github.com/SK8-ENGINE/skate-3-rust-engine): a standalone engine whose `skate-core`
   crate holds recovered Skate 3 physics/animation calculations and whose `skate-game` crate held
   the player (input phase, graph host, states, camera). Captured at revision `60efdef`, with the
   later upstream `2e16697` three-way merged in (`SOURCE_PORT.json`).
2. **Smokey's mod**: a fork of this repository at 0.2.9 that moved the engine's player into Skate
   2 (`crates/skate2-core/src/physics/modes/skate3/`) and vendored the engine crates. Its
   integration embedded a Skate 3 asset pack in the executable, read the pad beside the game's
   input settings, ran a trace journal on every launch, and linked rapier, mlua and Bevy into the
   core crate.
3. **This port**: took Smokey's code as the starting point and rebuilt it into the game natively,
   then checked it against TU3 in IDA and against the running game, function by function. It
   reads every asset from the player's linked Skate 3 data, uses the game's own input pipeline,
   costs nothing in Skate 2 mode, and adds the systems the engine never had (prop pushing,
   shoves, TU3 collision groups, Skate 3 scoring branches, breadcrumbs, darkcatch audio, ...).

How much was built on top since Smokey's mod (from `92-changes-since-smokeys-mod.md`):

| Area | Files | Lines |
|---|---|---|
| Physics module (`crates/skate2-core/src/physics/modes/skate3/`) | 82 changed, 3 new, 5 removed of 428 | +2161 / -1286 |
| Vendored `skate-core` | 17 changed, 1 new | +367 / -48 |
| Vendored `skate-data` | 6 changed, 1 new, 4 removed | +561 / -268 |
| Integration layer (the 30 driver and seam files compared; 42 in all) | | +2176 / -395 |
| Shared Skate 2 systems carrying Skate 3 behaviour (84 files; scoring, audio, collision, breadcrumbs) | 74 changed since 0.2.9 | +2672 / -234 |

## Size of the whole thing

| Part | Where | Size |
|---|---|---|
| Engine core (math, board, skeleton, riding, air, grind, camera kernels, graphs) | `vendor/skate3-engine/crates/skate-core` | 739 files, ~101k lines |
| Engine data readers (abin banks, state graphs, vault collections, physics skeletons) | `vendor/skate3-engine/crates/skate-data` | 55 files, ~10k lines |
| Engine dynamics (deformable/solid helpers) | `vendor/skate3-engine/crates/skate-dynamics` | 6 files, ~5k lines |
| The player (states, input phase, graph host, camera, animation) | `crates/skate2-core/src/physics/modes/skate3` | 428 files, ~60k lines |
| Integration with Skate 2 | `crates/skate2-core/src/physics/skate3_*.rs` and ~35 more | ~9k lines |

## Read in this order

| File | What it holds |
|---|---|
| [01-architecture.md](01-architecture.md) | The layers, who owns what, the host boundary types, data and settings |
| [02-tick-pipeline.md](02-tick-pipeline.md) | One tick end to end: Skate 2's host tick, the Skate 3 tick in TU3 order, publication back |
| [03-states-and-subsystems.md](03-states-and-subsystems.md) | Every physical state (IDs, TU3 vtables/functions, files) and every subsystem |
| [04-shared-skate2-skate3.md](04-shared-skate2-skate3.md) | What Skate 2 and Skate 3 share, what differs, ID/group/state translations |
| [05-what-we-fixed.md](05-what-we-fixed.md) | Every parity fix and addition against the engine/mod, with the TU3 evidence |
| [06-porting-guide.md](06-porting-guide.md) | How to move all of it to another host in one pass: copy list, services, order, checks |
| [07-verifying-against-tu3.md](07-verifying-against-tu3.md) | IDA pitfalls, address conventions, measuring the running game |
| [08-open-items.md](08-open-items.md) | Known gaps and unported pieces |
| [20-scoring-audit.md](20-scoring-audit.md) | TU3 ScoreModule vs Skate 2, per function |
| [21-hall-of-meat.md](21-hall-of-meat.md) | TU3 WipeoutScorer (Hall of Meat) notes |
| [90-file-index.md](90-file-index.md) | Generated: every file with its own module doc |
| [91-address-index.md](91-address-index.md) | Generated: every executable address the code names, by address, with file:line |
| [92-changes-since-smokeys-mod.md](92-changes-since-smokeys-mod.md) | Generated: per-file changes since Smokey's mod |

The generated files come from the source itself; regenerate them after code changes rather than
editing them:

```
python docs/skate-3-physics/tools/extract_indexes.py
python docs/skate-3-physics/tools/changes_since_smokeys_mod.py <Smokey's mod checkout>
```
