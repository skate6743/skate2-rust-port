# Performance: AI skater cost — November 12, 2026

Players reported that AI skaters lower the frame rate the most, more so while
they skate into things and with LivingWorld loaded. This measures what an AI
skater costs and applies three changes that leave physics untouched.

## Measurement

- Scene: Slappy's (`F_PJ_Slappys`) on the test machine (12-core desktop CPU,
  RTX 2070), vsync off, frame cap 120, 960x540 window. The player stands; three
  ambient AI skate the park with LivingWorld loaded.
- The session AI is made by the main menu's front-end world, so a run goes
  through the career demo (`--live-controls-demo career`) and teleports to
  Slappy's (`SKATE2_DIAGNOSTIC_TELEPORT_TO=120:on:0:1682.5,103.2,-535.2`).
  `SKATE2_TRACE_AI=1` logs each spawn and unspawn; `SKATE2_FRAME_PROFILE`
  writes the timings. A direct `--live-skating` run has no AI manager.
- Comparisons average the profile windows in which all three AI were alive.

## What an AI skater costs

| At Slappy's | Tick (SIM thread) | Frame CPU (window thread) |
|---|---|---|
| No AI (a direct `--live-skating` run at the same spot) | 1.96 ms | 1.39 ms |
| 3 AI | 4.67 ms | 1.96 ms |

About 0.9 ms of tick per AI. The worst tick of a one-second window reached
5–8 ms, at times 9–11 ms: on a CPU about half as fast those would pass the
16.7 ms step, and the frame would wait for the late tick.

A sampling profile of the SIM thread with three AI (busy time only):

- Per-physical preparation (`try_prepare_ground_pre_solve_pair`, the
  original's per-physical "1-Input", "2-Logic", "3-State" jobs): 23%.
  The collision line queries (trajectory, wheel and skeleton lines): about
  15%, partly inside it.
- Horus island solve: 14%. Post-solve: 8%.
- Heap allocation and release: 16%, spread over many call sites.
- XInput calls on empty controller slots: 5%. `XInputGetState` and
  `XInputGetCapabilities` on an empty slot enumerate the system's devices.
- Attribute lookups by name: about 6%.

The three animation job threads were blocked 97–100% of the time. Everything
above runs on the SIM thread, one physical after another. On the window
thread, wgpu's command encoding took about a quarter of the busy time, and
every character and DMO was drawn whether or not it was in view.

## Changes

1. **Main-pass culling** (`gpu_renderer.rs`, `map_draw.rs`): skaters,
   pedestrians, NIS characters, vehicles and DMOs whose world `m_BB` misses
   the view are not drawn (`SceneRenderView::Cull` 0x828500a8). The box is
   the model's unrotated box moved to the entity (0x828501c0), so it is first
   widened on every side by its own diagonal (`widened_box_in_view`). The
   widened box holds the model turned any way about any point inside it. Their
   shadows keep `ShadowView::Cull` per cascade.
2. **Allocator** (`main.rs`): mimalloc 0.1.52 (its default mimalloc v3) is the
   global allocator.
3. **Empty controller slots** (`windows_pad_platform.rs`): a slot XInput found
   empty answers device-not-connected for one second without another XInput
   call. A pad plugged in meanwhile is found within that second; SDL pads in
   those slots are read every tick as before.

## Results

Three AI at Slappy's, two pairs of runs:

| | Before | After |
|---|---|---|
| Tick, mean | 4.68–4.70 ms | 4.01–4.17 ms (−11% to −14%) |
| Worst tick of a window, mean | 5.59–5.83 ms | 4.93–5.08 ms |
| Worst tick of the run | 6.7–9.2 ms | 6.0–6.6 ms |
| Frame CPU, mean | 1.91–1.94 ms | 1.71–1.78 ms (−8% to −10%) |
| Command recording per frame | 0.87–0.88 ms | 0.72–0.76 ms |

Memory in the same runs:

| Allocator | Private bytes | Working set |
|---|---|---|
| System heap | 1.91 GB | 1.36 GB |
| mimalloc v3 | 2.32 GB | 1.56 GB |
| mimalloc v2 | 2.45 GB | 1.77 GB |

The extra memory is a constant offset. In every build the private bytes rose
about 2.5 MB a second over a run. `MIMALLOC_PURGE_DELAY=10` or `0` and
`MIMALLOC_PAGE_RECLAIM_ON_FREE=1` did not lower it, and an immediate purge
cost tick time.

- Parity: the pool route (`perf-skate` from `F_UR_PermissionPool`, 1500
  completions, `SKATE2_TRACE_SELECTED_STATES=1`) gives 6000
  `selected (board|states|offboard|manual) frame` lines, byte-identical before
  and after.
- Images: with a second skater spawned beside the player
  (`SKATE2_DIAGNOSTIC_SPAWN_SKATER`), both builds draw it, also with it half off
  the left edge of the screen (`SKATE2_DIAGNOSTIC_SPAWN_SKATER_AT=4.4,0.6`). Two
  runs of the same build differ in about 9% of the pixels (sky animation and
  in-between frames), so captures are not compared pixel for pixel.

## Not changed

The per-physical preparation, the line queries and the island solve stay
serial on the SIM thread. The original ran the per-physical preparation as
separate jobs. Running each skater's share on worker threads, with results
kept in skater order, is what would let the AI cost spread over more cores.
It is the largest remaining cost and a larger change.
