# Stick Settings (2026-10-27)

Scope: the PC port's Stick Settings under Options' Control Settings: what each setting does to a
stick, where it applies, the pages and their input, and the file that keeps them.

## The original's sticks

- `try_convert_xbox_pad` (`input/devices/input_pad.rs`, native 0x82989448) scales each thumb axis
  by 2^-15, then treats each stick as a point: under a thousandth of the travel it reads as
  centred; otherwise its distance from the centre becomes `(length - 0.25) / 0.7`, kept to
  `0..=1`. The original's deadzone is therefore round and a quarter of the stick's travel, and the
  stick reads full tilt from 95% of it. This conversion is unchanged.

## What the settings do

- The settings apply to every pad's sample before the game reads it (`pad_sticks.rs`, from both
  pad platforms), as RPCS3 applies its own before the emulated pad. At the vanilla settings
  (multiplier 1.00, deadzone and anti-deadzone 0%) a sample passes through as the pad reported it,
  so the game reads exactly what it read before.
- `stick_settings.rs` follows RPCS3's `PadHandlerBase::get_mapping`: `TranslateButtonPress`'s
  `NormalizeStickInput` multiplies each axis and keeps it to its range (so past full travel each
  axis stops at its end on its own), then `NormalizeStickDeadzone` treats the stick as a point at
  distance `m = min(length, 1)` from its centre: with a deadzone `d` above zero and `m <= d`, the
  stick moves to `a * m / d` of its travel along the same direction (`a` the anti-deadzone);
  otherwise to `a + (1 - a) * (m - d) / (1 - d)`. RPCS3 hands a game a DualShock 3's 8-bit axes;
  here the pad's 16 bits are kept. A test transcribes RPCS3's XInput path (`get_button_values`,
  `thumb_max` 32767) and checks every setting against it across the stick's range.
- An anti-deadzone as large as a game's own deadzone skips it. The original's own deadzone is a
  quarter of the stick's travel (above), so an anti-deadzone of 25% skips it, and a deadzone above
  zero alongside it keeps a resting stick still.
- Ranges: the multiplier 0.40 to 1.10 in hundredths, the deadzone and the anti-deadzone 0% to 50%.

## The pages

- Control Settings ends with Stick Settings, which holds Left Stick, Right Stick and Reset To
  Default. Each stick's page holds Stick Multiplier (a selector), Stick Deadzone and Stick
  Anti-Deadzone (Edit Skater's morph knob, `highlight_morph`, the percent after the title). B goes
  back one page, to the row that opened it.
- Beside the rows, two boxes, L above R, draw both sticks live as RPCS3's pad settings draw their
  previews (`pad_settings_dialog::RepaintPreviewLabel`): the stick's full travel, the deadzone as a
  red ring, the anti-deadzone as a green one, the stick under the multiplier alone as a blue dot and
  the stick as the game reads it as a red dot.
- The pages take the D-pad alone (`AptDUp`, `AptDDown` and `DPadLR`), as the front end's generic
  directions hold the left stick, which the boxes show. Held left or right, the multiplier steps a
  hundredth at once, then from 24 frames every 4, from a second every 2 and from two seconds every
  frame; a deadzone's knob steps at once, then from 12 frames every 3 (20% a second), looping
  `FE_SOUND_CAS_SLIDER` while it moves, as Edit Skater's morph knobs do.
- Reset To Default puts both sticks back to vanilla.

## The file

- `controller_config.json` beside `config.json` (the folder the game runs from, or
  `SKATE2_SAVE_ROOT`), apart from the profile, so loading or replacing a profile leaves the sticks
  as they are. Exiting Options writes it when a stick setting changed. Keys:
  `left_stick_multiplier` (a decimal), `left_stick_deadzone` and `left_stick_anti_deadzone`
  (percent), and the same for `right_stick`. A missing file or key is vanilla; a value past its
  range is kept to it. Updates never replace the file.
