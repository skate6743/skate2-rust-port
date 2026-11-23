# Polling & Delay: the pad polling rate and Input Delay — November 21, 2026

## How often the game reads a pad

- The original polls its pads on its timer, not on its frames. `timerThread` (0x8298a2c4) fires
  every timer period (`SetTimerHz` 0x82985ec0: 10,000,000 / Hz in 100 ns units, 60 Hz in play),
  and each fire's `Sk8AppMan::TimerCallback` reads every pad once and puts the sample in each
  input group's cache (`CacheInput` 0x8298cc40). The simulation's next tick takes the newest
  cached sample (`LastInput` 0x826dfbd8) and does not run without one.
- Skate 3's title update 3 executable does the same. Measured on it running natively with its
  frame rate uncapped: `XamInputGetState` returned for pad 0 120 times in 2.01 s, 60 a second.
- The port polls as often at every frame rate. The game's clock adds one simulation tick per
  timer period (1/60 s in play, `live_skating_client::FIXED_STEP`, the slow-motion rates through
  `timer_period`), and each tick polls every pad once through that same cache
  (`SelectedProductionInputOwner::poll_and_cache`, then `try_last_input`). A frame rate above 60
  draws more images between ticks and polls no more often; the frame rate cap, V-Sync and the
  unlocked rate leave the count at 60 a second.
- What differed is when each of those polls reads the pad. The original's read is the timer's
  own, so its samples are one timer period apart whenever the simulation takes them. The port's
  poll ran with its tick and read the pad at that moment, and a tick starts when the frames let
  it: the one before it has to be handed to the presentation first, and without the simulation
  thread a tick runs inside a redraw. A tick that starts late read a later pad state than the
  original's timer would have given it. Measured (below): with the frame rate capped, or far
  above 60, the reads stayed a sixtieth of a second apart and only their moment moved (2 to 10 ms
  after the timer at a cap of 60); with the frame rate uncapped and held near 70 by the graphics
  load, a third of the reads came more than 2 ms early or late, 11 to 25 ms apart, while the
  simulation steps each tick as a sixtieth of a second.
- What also differs between machines is the time from a poll to the image that shows it, which
  the frame rate and the display set, and an emulator adds its own. Input Delay is for that.

## Polling Rate

- Control Settings' last row opens Polling & Delay, whose first row is the Polling Rate, a
  selector of two values (`stick_settings::Polling`):
  - `60hz (Vanilla)`, the default and what a `controller_config.json` without the key takes:
    each tick reads every pad as it was at the tick's own timer instant
    (`live_skating_client::since_timer`: the start of the newest timer period, as a tick takes
    the newest cached sample), so the reads are a sixtieth of a second apart as the original's
    are, however the frames pace the ticks.
  - `Frame Timed`: each tick reads every pad at the moment it runs, as the port did before the
    setting.
- The past state comes from the same histories the Input Delay reads (`pad_delay::ReadTime`,
  `History::as_of`): XInput's thread runs while either setting needs the past, which the
  default does. A pad the history has not read yet answers with its present state.
- `controller_config.json` holds it as `polling_60hz`, 1 or 0; a file without the key reads 1.
- `SKATE2_TRACE_PAD_POLL=1` logs, every 600 ticks, how long after their timer instants the ticks
  started and how far apart their starts were.

## Input Delay

- Polling & Delay's second row is the Input Delay, Edit Skater's morph knob with its milliseconds
  after its title: 0 to 30, 0 without a saved value. Since 2026-11-23 the row's title is
  `Added Input Delay`, and 30 reads `Added Input Delay 30 ms` (it read
  `Input Delay 30 ms (RPCS3 Like)` before, as in the run under Verified, which has not been
  repeated with the new title). The stick or the D-pad held moves it a millisecond at a time
  (at once, then from 12 frames every 3), and X puts it back to 0.
- With a delay set, each poll reads every pad as it was that many milliseconds before the moment
  the Polling Rate reads it at: its buttons, sticks and triggers alike, before the stick settings
  and the trigger threshold. The polls themselves stay where they are, one a tick.
- A pad's states are kept as they change, each with the time it was first read
  (`pad_delay::History`, a quarter of a second of them). SDL's pads are read every millisecond
  on their own thread, which keeps their history; XInput's are read every millisecond on a
  thread that runs while a poll reads the past (`pad_delay::XinputHistory`), a slot found empty
  being asked again after a second, as the game's own poll asks. A pad read for less long than
  the delay answers with its first state.
- At 0 under the Frame Timed rate nothing is kept for XInput and a poll reads the pad itself.
- `controller_config.json` holds it as `input_delay_ms`; a missing key is 0 and a value past 30
  is 30.
- A diagnostic sample (`demo_sample`) bypasses it, as it bypasses the stick settings.

## Verified

- A scripted run of the front end's Options (`SKATE2_DIAGNOSTIC_OPTIONS_PAGE=control_delay`) at
  1280x720: Control Settings' last row opens the page titled Polling & Delay, which lists
  Polling Rate with `60hz (Vanilla)` under it and Input Delay; right presses step the knob a
  millisecond each (`Input Delay 27 ms` on the way) to `Input Delay 30 ms (RPCS3 Like)` with the
  knob at the line's end and no further, the button help reading X Reset to Default and B Back on
  the row; one right on Polling Rate then reads `Frame Timed` between the selector's arrows, and
  the run's settings end at `input_delay: 30, polling: FrameTimed`.
- How late the ticks start (`SKATE2_TRACE_PAD_POLL=1`, scripted runs of the career with the
  simulation on its own thread, V-Sync off, each line the 600 ticks of ten seconds of play;
  `SKATE2_DIAGNOSTIC_FRAME_RATE_CAP`, `_VSYNC` and `_RESOLUTION_SCALE` set the pacing):

  | Frame pacing | A tick's start after its timer instant | Starts off a sixtieth of a second |
  | --- | --- | --- |
  | No cap | 0.3 to 0.4 ms on average | 0.16 to 0.24 ms on average |
  | Cap 120 | 0.3 ms on average, under 1 ms at most | 0.16 to 0.18 ms on average |
  | Cap 60 | 2 to 10 ms on average, 16 ms at most | 0.33 to 0.38 ms on average |
  | No cap, 1904x984 at 200% scale and 4x MSAA | 0.3 to 0.5 ms on average | 0.2 to 0.55 ms on average |
  | Cap at the monitor's refresh rate (120 Hz) | 0.3 ms on average, under 1 ms at most | 0.18 ms on average |
  | V-Sync on, no cap, the window focused (120 Hz display) | 0.3 ms on average, 1.5 ms at most | 0.18 to 0.21 ms on average |
  | No cap, the same with two more games drawing beside it: about 70 frames a second | 1.3 to 1.6 ms on average, 8.5 ms at most | 1.5 to 1.8 ms on average |
  | No cap, three more games beside it: slower still (its counter was not captured) | 8.4 ms on average, 16.7 ms at most | 2.5 to 3.2 ms on average |

  In the first six cases at most 13 of the 600 starts were more than 2 ms off a sixtieth of a
  second, single late ticks (up to 16 ms) at a travel, a load or a screen capture. With two more
  games beside it, 200 to 237 of the 600 were, the starts 11 to 25 ms apart throughout; with
  three, 189 to 322, the starts 5 to 30 ms apart.
- What the numbers say:
  - The port never read the pad more often than the original: 60 reads a second in every case.
  - With the frame rate capped at 120 or 60, or uncapped on a machine that draws far faster than
    60, its reads were a sixtieth of a second apart to a fraction of a millisecond. At a cap of
    60 each was taken 2 to 10 ms after its timer instant, where the original's sample is the
    timer's own.
  - With the frame rate uncapped and the graphics load holding it near 70, the reads were
    uneven: a tick waits for a frame to take the one before it, and frames 14 ms apart do not
    line up with ticks 16.67 ms apart. A stick moving steadily then reads as moving up to half
    again as fast or two thirds as fast from one tick to the next. This is the case the report
    of the game feeling off at an unlocked frame rate describes, and the one the vanilla rate
    corrects: it takes the timer's moment whenever the tick runs.
  - V-Sync on a 120 Hz display reads as evenly as the caps do. V-Sync on a 60 Hz display was
    not measured.
- What the uneven reads do to a flick (`tools/flick_sampling.py`: one right-stick
  flick, pulled down, then up to the top in a set time and held, read either every sixtieth of
  a second or at uneven moments like those measured, 11.5 and 21.8 ms apart in turn or 25, 11
  and 14; each reading replayed tick by tick with `SKATE2_DIAGNOSTIC_RIGHT_STICK` from a
  standstill, the pop read from `SKATE2_TRACE_KNOWN_AIR` and `SKATE2_TRACE_SKATE3_LAUNCH`):

  | Flick | Physics | Read every sixtieth, three phases | Read unevenly, five patterns |
  | --- | --- | --- | --- |
  | 70 ms | Skate 2 and Skate 3 | 3.38 m/s up, every time | 3.38 m/s, every time |
  | 110 ms | Skate 2 | 3.05 to 3.06 m/s (jump strength 0.72) | 2.82, 3.05, 3.05, 3.05 and 3.38 m/s (strength 0.53 to 1.0) |
  | 110 ms | Skate 3 | 3.05 to 3.06 m/s | 2.82, 3.05, 3.06, 3.06 and 3.38 m/s |
  | 150 ms | Skate 3 | 2.82 to 2.83 m/s | 2.82 to 2.83 m/s |
  | 220 ms | Skate 3 | 2.09 m/s | 2.09, 2.09, 2.09, 2.14 and 2.15 m/s |

  The game grades a flick by how fast the stick travels between ticks, so the same flick read
  at uneven moments can be graded as a faster or a slower one: a 110 ms flick, which an even
  reading always pops at 3.05 m/s, popped at 2.82 or at 3.38 m/s under two of the five uneven
  readings, about a fifth less or more height. A flick fast or slow enough to sit inside one
  grade comes out the same either way. So at an uncapped frame rate the graphics hold near 60
  to 100, the Frame Timed reads could make the same flick pop differently from one try to the
  next, which the vanilla rate does not. Whether a player notices was not tested with a pad.
- Unit tests, run from the crates' own test builds:
  - `skate2-core`: `stick_settings`, `ui::controller_config` and `ui::options` (24 pass): the
    file keys round-trip, a file without them reads no delay and the vanilla rate, and
    `the_input_delay_slides_a_millisecond_at_a_time_and_x_puts_it_back_to_none` walks Control
    Settings' last row into the page, both Polling Rate values, the knob to 30 and back.
  - `skate2-pc`: `pad_delay` (4 pass: a change read at one moment arrives the delay later, a
    young pad answers with its first state, old states go, a poll reads at its timer instant
    less the delay) and `pad_sticks` (1).
  - `skate2-pc` over the original game data:
    `control_settings_and_polling_and_delay_lay_out_their_rows` (Control Settings' and the
    page's titles in glyphs their font has and within their fields, the Trigger Threshold's and
    the Input Delay's knobs with their titles).
  - `display_preferences_original_options_layout_and_lifecycle`, an older test of the same
    movie, expected Display Settings to start where seven selector rows put it; the list has had
    eight since Anti-Aliasing joined it, and with that row counted the test passes.
  - `stick_settings_pages_lay_out_their_rows_and_boxes`, another older one, held every row 16
    clear of the boxes at x 800 by its quads. A highlighted knob shows its arrows, 64-wide
    sprites with clear margins, and the right one's quad reaches x 793.85 while what it draws
    ends short of the gap; the test now holds that row to the boxes' edge and every other row to
    the gap, and passes. The layout is unchanged.

## Not verified

- The delay and the polling rate with a pad in hand: the scripted runs' pad is the diagnostic
  sample, which bypasses both.
