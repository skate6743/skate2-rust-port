# Skate 2 reconstruction: contributor guide

These rules apply to every contributor and every file in this repository.

## Objective

Reconstruct the offline Xbox 360 Skate 2 reference build as maintainable,
human-readable native Rust with observable behavior as close to 1:1 as the
available evidence permits. Preserve uncertainty instead of hiding it. PC-only
behavior is an explicit, documented exception, never a silent substitute.

## Layout

- `apps/skate2-pc`: the PC game (window, input, renderer, first-launch game-folder prompt).
- `apps/skate2-server`: the online server, for Windows, Linux and macOS.
- `apps/build_secret.rs`: the build secret both apps' build scripts share.
- `crates/skate2-core`: the reconstructed game (simulation, physics, animation, challenges,
  career, UI, sessions). Folder guide: `crates/skate2-core/src/README.md` and
  `docs/CORE_LAYOUT.md`.
- `crates/skate2-formats`: the original file formats (BIG archives, Vault, parts databases).
- `crates/skate2-audio`, `crates/skate2-xma`: audio and the XMA decoder.
- `crates/skate2-net`: the online protocol, transport and server.
- `crates/skate2-symbols`: linker-map tooling.
- `docs/`: the reconstruction rules (`ARCHITECTURE.md`), subsystem references (`ai/`, `audio/`,
  `career/`, `online/`, `partyplay/`, `release/`) and per-topic evidence (`evidence/`).
- `tools/`: research scripts (IDA, Vault, APT, shader disassembly).

## Non-negotiable boundaries

- Treat the executable, PDB, linker map, decompiler output, disassembly,
  runtime traces, and original game data as behavioral evidence.
- Do not ship a PPC interpreter, guest CPU context, instruction-by-instruction
  translation, assembly-shaped generated source, or emulator subsystem.
- Keep original executable, symbols, databases, and game assets external to
  the repository.
- Express recovered behavior with meaningful Rust types, ownership, names, and
  subsystem boundaries.
- Do not replace unknown game behavior with plausible generic skating,
  physics, animation, rendering, UI, or gameplay behavior and call it parity.
- Do not silently sanitize unusual source values or change reference ordering,
  tie behavior, floating-point precision, update timing, caps, or failure paths.
- Before hands-on skating tests, connect the original input history,
  grab/button combinations, held/released flick and multi-flip behavior
  through the actual animation/physics state path. Automated component and
  corpus tests should continue throughout implementation.
- Preserve runtime Vault fetching, inheritance/default behavior and source
  sampling/update timing. Verified payload values must not silently become
  hardcoded runtime tuning; a narrow corpus reader is not the full service.

## Evidence required for reconstructed behavior

Before calling behavior recovered, record the applicable:

1. decorated/demangled symbols and reference virtual addresses;
2. callers, callees, object/library ownership, and relevant control flow;
3. type layouts, field offsets, enums, flags, vtables, and data ownership;
4. constants and their exact origin in code, Vault/VLT, or serialized data;
5. branch, iteration, insertion, tie, update, and floating-point ordering;
6. validation procedure and result against real corpus data or a reference
   runtime trace.

Use the confidence vocabulary from `docs/ARCHITECTURE.md`: `exact`, `high`,
`medium`, or `hypothesis`. A hypothesis is not parity and must not be presented
as proven. Important reconstructed Rust items should carry a reference-address
doc comment when an address is known.

## Conventions

- `unsafe` is forbidden. Clippy `all` and `pedantic` are on; the gate denies warnings.
- Source files mix CRLF and LF line endings, sometimes within one file. An edit keeps
  each line's existing ending; never normalize a file.
- Native addresses belong in doc comments. Explanations that outgrow a comment belong in
  the relevant `docs/` file.
- A test or diagnostic run that can write a save sets `SKATE2_SAVE_ROOT` to a scratch
  folder holding a copy of a save.
- The original game stays outside the repository: `SKATE2_GAME_ROOT`, or the folder the
  game asks for at first launch (by its `.xex`), kept in `config.json`.
- Machine-specific files are git-ignored: `config.json`, `online-config.json`,
  `local-paths.bat` (game folders for the `play-*.bat` launchers), `release-secret.txt`,
  `saves/`, traces and logs. Local working notes, when present, are in `private-notes/`
  (git-ignored; `private-notes/AGENTS.md` is the running log).

## Building

- `build-release.bat` (Windows), `build-release.sh` (Linux) and `build-release-macos.sh` (macOS)
  build the release binaries. With a `release-secret.txt` key (`tools/new_release_secret.py`)
  both platforms' builds of one version share their secret; without it every build has its own.
- `skate2-pc.exe` with no arguments runs the quiet game; `play-*.bat` run the diagnostic
  variants (traces, performance, career tests).

## Parallel work

- One integrator owns shared interfaces, merge order, status, and final parity claims.
- Start delegated work as read-only evidence collection unless exclusive file/module
  ownership was assigned.
- One contributor owns a behavior at a time. Independent review is welcome, but two
  contributors must not implement competing versions in shared files.
- Return a proof packet before integration: evidence, typed semantics, current
  mismatch, confidence, unknowns, and focused tests.
- Never broaden an assigned subsystem or edit shared architecture without
  coordinating with the integrator.
- Preserve unrelated changes in the working tree. Do not use destructive Git or file
  operations.

## Validation gate

Run validation proportional to the change. The strict repository checkpoint is:

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Parser, world, renderer, and physics changes also require the relevant real-data
corpus/probe. Physics, animation, tricks, and state-machine parity ultimately
require deterministic per-tick reference traces; visual similarity alone is
not sufficient.

Record material progress and remaining deviations in the relevant
`docs/evidence/*.md` file or subsystem reference.
