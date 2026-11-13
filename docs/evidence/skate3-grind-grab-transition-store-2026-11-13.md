# Skate 3 physics: fast grind-grab taps and the animation transition store — November 13, 2026

With Skate 3 physics, tapping the triggers quickly during a darkslide (or any board slide) moved
the skater only a little way down. In Skate 3 the same taps make him reach down fast and deep on
every press. This compares the two with the same scripted input and fixes the cause.

## Measurement

- TU3 ran as a native recompilation; its state was read from guest memory each tick and the pad
  input was set each tick through a breakpoint after `XamInputGetState`
  (`skate-3-physics/07-verifying-against-tu3.md`, "Grind grabs").
- The port ran the same input with `SKATE2_DIAGNOSTIC_TELEPORT_TO` and the career input
  diagnostics, and printed the same fields with `SKATE2_TRACE_SKATE3_GRIND_GRAB`.
- Scene: University, the 0.63 m box east of the stadium. On board at (332.88, 3.0, 354.25), yaw
  −1.5558; two pushes; a kickflip darkcatch; a darkslide on the box's south edge for 40–45 ticks.
- Measured value: the crouched-enough height (PhysOut animation +0x48), the skater's centre of
  mass above the deck along the reckoning up. IsCrouchedEnoughForBlendToGrabCycle (0x82BBDE40)
  tests it against 0.6; it also shows how far down the animated skater is.

## What was ruled out

- The grab inputs: TU3 maps the trigger byte to raw / 255 (`cXenonPadDevice::MapStateToButtons`
  0x8296D5F8) and emits `Left/RightGroundGrab` only at exactly 1.0 (Fill 0x825999F0), as the port.
- The graphs: the skater's Action and Motion graphs are compiled into the TU3 executable
  (StateGraph::Gen). Their grind-grab states, transitions and expressions match the shipped
  `.stategraph` the port runs (Into 0x1CD: HasIntent and not crouched enough; GrabCycleDirect
  0x269: HasIntent and (grab type, or crouched enough and neither FS nor BS)).
- The first tap of a darkslide: TU3 and the port agree within 0.02 m tick for tick.

## Cause

Andale::TransitionStore (constructor 0x82D17370) holds at most three transition blends
(`mMaxNumTransitionBlends`) and `mMaxNumTrees` − 1 = three transitions. Before adding one,
TransitionAnim (0x82D186F8, 0x82D18780..7CC) force-completes the oldest blend when three blends
are live, then the oldest transition when three transitions are live. ForceTransitionRelease
(0x82D18658) gives the transition's parent (or the root) its destination and releases the source
subtree.

Each tap creates three blends: Into (0.3 s), Out (0.1 s) and Cycle (0.3 s, from the current
frame). In TU3 the skater's store reaches three transitions on the second tap and stays there for
the rest of the spam; every new tap completes the oldest transition, which exposes the grab pose,
and the height drops 0.1–0.2 m within one or two ticks. The port's Skate 3 playback had no limit:
every unfinished blend stayed in the tree and kept the upright pose weighted.

## Results

Spam phase (from the second tap to the end of the darkslide), crouched-enough height in metres:

| Run | Lowest | Mean | Ticks below 0.65 |
|---|---|---|---|
| TU3, RT 3 ticks on / 3 off | 0.538 | 0.644 | 71% |
| TU3, RT and LT alternating, 3 / 3 | 0.557 | 0.646 | 60% |
| Port before, RT 3 / 3 | 0.692 | 0.713 | 0% |
| Port after, RT 3 / 3 | 0.552 | 0.662 | 65% |
| TU3, RT 6 / 6 | 0.593 | 0.687 | 24% |
| Port after, RT 6 / 6 | 0.638 | 0.686 | 11% |

The rows of TU3 and the port after the change line up tick for tick: over the first twelve
ticks of the RT 3 / 3 spam both read 0.730, 0.610, 0.625, 0.638, 0.552, 0.744 (TU3 0.624, 0.637,
0.551, 0.743), and the later taps dip to the same values a tick apart (the TU3 input loop drifts
by a tick there). A held RT grab reaches GrabCycle on the same tick in both, both hold at
0.47–0.48, and the port lags TU3 by up to 0.04 on the way down.

The change covers every grind grab, not only the darkslide's. On the tipslide that a drop-in
starts at the University stadium deck edge (BS_TAILSLIDE in the port, FS_BLUNT in TU3; see
`skate-3-physics/08-open-items.md`), the second 4-tick tap now dips to 0.59 / 0.58 / 0.49
(TU3 0.59 / 0.61 / 0.62); before the change it stayed at 0.77.

## Change

`crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs`: every transition
`play` makes carries a creation serial (`PlaybackTransition::serial`); before a new one,
`release_for_capacity` applies TU3's two limits and `force_release` completes the oldest
transition. A transition placed under goes below the youngest transition, as in 0x82D18950
(it went below the root before, which differs once two are nested).
