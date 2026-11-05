# Skate 3 Hall of Meat (TU3 WipeoutScorer) port notes

TU3's WipeoutScorer (Hall of Meat) is a different design from Skate 2's: it tracks the run-up
before a bail (air, grind and drop) and scores it with curves Skate 2 does not have. It is ported in
`scoring/wipeout_skate3.rs`, over the Skate 3 `scoring_wipeouts` data (`scoring/skate3_hom.rs`
remaps six shared curves). This file maps TU3's members and functions.

## Functions (TU3)

| Address | Role |
|---|---|
| 82DAE4F0 | Update: wipeout gate, reset, start/end, dispatch, UpdateCharacterDamage |
| 82DAF1D8 | UpdateInWipeout |
| 82DAEDC8 | Update outside a wipeout (heavy landing timer, pre-wipeout window) |
| 82DADB48 | StartWipeout: heavy landing foot damage (curve +2420 * +3068, toes * +3064) |
| 82DADC48 | OutputBailData |
| 82DADA30 | bonus score |
| 82DAD528 | run-up score: curves +2340, +2100, +2740, +2660 + member +2056 |
| 82DAD640 | rotation score: combined angle -> curve +2260 (pre-wipeout window) or +2900 |
| 82DAD2C0 | bail speed score: curve +2820 of length(+384) |
| 82DAECB8 | air drop update (curves +2516/+2596 = layout 629/649) + UpdateBodyTweaks |
| 82DAEBC0 | EndAir |
| 82DAD0A8 | Reset |
| 82DAD368 / 82DAD448 | GetBoneScoreForDamageLevel / GetBoneScore |
| 82DAD728 | UpdateBodyTweaks |
| 82DAE230 | UpdateCharacterDamage |
| 82DAE6F0 | DamageBone: running total per bone, damage = max(old, total); fixed 0.1 propagation threshold |
| 82DAE7A8 | PreUpdateBones (same as Skate 2) |
| 82DAE9A8 | UpdateBones (CheckForDamage inlined) |

Skate 2 equivalents for comparison: Update 82DF6CB8, UpdateInWipeout 82DF5BC0, OutputBailData
82DF12C0, GetBonusScore 82DEC6C8, DamageBone 82DECA00 (cvar propagation).

## Wipeout gate (82DAE4F0)

- in wipeout = filtered state 4 (Wipeout) and PhysOut State byte +0x44 set and +0x45 clear.
- pre-wipeout window (+2229) = filtered state 2 or 7, or (window was set and timer +2236 < 0.1 s);
  the timer counts up outside states 2/7 and is zeroed in them.
- Reset when the window opens without a wipeout, or when a wipeout starts without the window;
  the latter also stores the bail start velocity (+384) and scores bail speed (+2044).
- Wipeout start: StartWipeout. Wipeout end: Scoring2 +14646 = 1, OutputBailData.

## Members (byte offsets)

- +4 layout; +16/+20/+24 entity scorers (score +124, new collisions +128); +400 PhysOut.
- +112/+192/+272 pitch/yaw/roll RotationTrackers (angle at +64 of each).
- +352/+368 positions (drop start / highest), +384 bail start velocity.
- +416 bones: 25 x 64 bytes (damage level at +32, previous at +36? damage +40, decay +44).
- +2016 frames in wipeout (+ outside it while the window is open), +2020 moving frames,
  +2024 frames since +2230 latched, +2028 frames since +2231 (bone score > layout +3060) latched.
- +2036 broken-bone hold (30 frames), published with the new-break count to Scoring2 +200/+208.
- +2040 heavy landing countdown; +2084/+2088 heavy landing speeds (PhysOut State +44 kind).
- +2044 bail speed score; +2048 rotation score; +2052 max rotation score; +2056 run-up term.
- +2068 speed samples; +2072 banked rotation; +2076 combined rotation; +2080 max combined.
- +2092 accumulated skeleton output +580 (scored by curve +2180 in the bonus); +2096 maximum.
- +2100 banked rotation at drop start; +2104/+2108 air time and its maximum; +2112 time.
- +2168 entity score sum while the body tweak is state 2; +2172/+2176 body tweak states.
- +2180 air frames; +2184/+2188 drop time / drop height scores; +2192 drop height.
- +2228 in wipeout; +2229 window; +2230/+2231 latches; +2232 airborne; +2235 bail speed scored;
  +2236 window timer.
- +2180..+2224: current (545..548) and accumulated (553..556) and maximum (549..552) air-drop
  records, folded by EndAir.

## Skate 3 layout (`scoring_wipeouts`, class 9E4C471BC6EAB615)

Bones 0 (2100 bytes). PointNegGraphData8 curves (x at +16, y at +48): 2100, 2180, 2260, 2340,
2420 (heavy landing), 2500 (drop time), 2580 (drop height), 2660, 2740, 2820 (bail speed), 2900
(rotation), 2980 (moving duration). +3060 u32 severe bone-score threshold, +3064/+3068/+3072
floats (toe damage scale, heavy landing scale, ...).

## Status

Ported in `scoring/wipeout_skate3.rs` (dispatched from WipeoutScorer::update_wipeout when the
scoring attributes are in Skate 3 mode). First test showed all-zero bails: the gate had State+68
inverted. TU3: in wipeout = filtered Wipeout && !State+69 && !State+68 (scoring stops when the
respawn countdown starts; our countdown starts 2 frames before the teleport request).

Heavy landing input: TU3 Update5 82DB71C8..7214 writes State+48 = Wipeout+0x50 and State+44 =
1 (Air+449 left footplant) / 2 (Air+450 right) / 0 when Wipeout+0x1A is set: the port's
`Requests` reason 6 and its value (air.rs `request(6, into)`, the heavy landing). It is published
from that request with the footplant side, read before SystemLogic's clear_after_selection
(82DB6000) as TU3's Update2/Update5 order does.

Verified (SKATE2_TRACE_SCORING=1): the window/wipeout gate counts the whole bail
(bail-speed: 3.55 s, 8.82 m/s, 516 deg, 0.54 m drop, peak bone damage 3.7), and a held body tweak
scores its authored 500. The other terms are zero by Skate 3's own thresholds, confirmed by the
linked_hall_of_meat_data test: bail speed scores from 10 m/s (+2820), wipeout rotation from 720
deg (+2900; the window's +2260 from 180), moving duration from 5 s (+2980), drop height from 7 m
(+2580), and a bone's first damage level needs 9.0. Skate 3's Hall of Meat rewards big crashes
only.
