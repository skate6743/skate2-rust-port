# High camera on vert airs: audit (2026-10-26)

Report: in the high camera, airs on a vert ramp look off. Scope: every step from the camera graph
to the framing reference that a high-camera vert air goes through, against the debug xex.

## What the high camera does on vert

- `Default_cameragraph.xml` picks `cameragraph_high.xml` with `IsCameraTypeActive type="1"`
  (`tSettings::mCameraAngle`, which `CameraManager::Update` 0x82584448 copies into the global
  condition data each update, as the port does).
- Its `Air` state (`PredictedAirTime > 0.1`; unlike the low graph it does not also require the
  board off the ground) takes a re-entry air (`IsReentry`: launch incline over 70 degrees and
  landing incline over 30, `UpdateTrajectoryData` 0x82e44dd8) as `bl_high_air` when it is short
  (`PredictedAirTime < 1.35`), low (`PredictedMaxHeight < 1`) or a spine transfer
  (`TransferAngle > 120`), else `bl_reentry_fs` with `FrontsideAngle < 0` or `bl_reentry_bs`. Any
  other air is `low_ollie` or `bl_high_air`. The low graph uses the same re-entry shots.
- `bl_high_air` blends `high_air` (3 m, 20 degrees up, framing the centre of mass) into
  `high_bigair` (5 m, framing the board 1 m along the subject's up vector) by the predicted
  air time from 1 to 3.5 seconds, so most of a vert air frames the board. Every low-camera air
  shot frames the centre of mass.
- `bl_reentry_fs`/`bs` blend their close and far versions by the previous shot's distance from
  1.3 to 2 m. Coming from `bl_high_chase`, whose sub-shots are all 3 m away, a high-camera
  re-entry is always the far version (3.73 m, 30 degrees below, pitched 17 degrees down), blended
  into the landing shot by the time the air reaches twice its apex time
  (`GetTrajectoryProgressWarped`).

## Verified against the xex

- Graph conditions: `IsReentry` (CameraMan+0x380 bit 3), `FrontsideAngle` (+0x318, set on the
  ground by `UpdateInclineAngle` 0x82e41420, kept through the air, negated for a mirrored
  stance after the frontside bit is set), `TransferAngle` (`UpdateTransferAngle` 0x82e41040,
  radians times 57.29578), `LaunchInclineAngle` (+0x328 `mInclineAbsAtLaunch`),
  `PredictedAirTime` (`GetTrajectoryCollisionTime` 0x82e3bfb0), `PredictedMaxHeight`,
  `PredictedLandingHeight`, `PredictedHeightDistanceRatio`, `IsPlayerSkateboardOnGround`.
- `UpdateTrajectoryData`: heights, apex time, height/distance ratio, launch and landing
  inclines, the re-entry thresholds, the transfer angle, the landing height blend and its flags.
- `UpdateInclineAngle`: the level-ground gate (0.99999), the incline, downhill heading, frontside
  angle and grind incline.
- `AttribShotDatabase::WriteToShot` 0x82d617a0: every shot field's offset and unit.
- `ShotManager2::GetBlendRawValue` 0x82e44898: blend types 7 (previous shot distance, the
  interpolated shot the selection keeps), 8, 13 (animation input lean, zero at setup), 16.
- `FrameComposer::CalculateReferencePoint` 0x82e3c328: the board offset along
  `Subject::GetUpVector` added to every weighted reference.
- The live path evaluator runs the full `PathEvaluator::Update` with the obstacle server.

No divergence was found in these. What remains is runtime input (the trajectory prediction's
times and normals on the ramp, the subject's up vector in the air), which
`SKATE2_TRACE_AIR_CAMERA` prints each tick.
