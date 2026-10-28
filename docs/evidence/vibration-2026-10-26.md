# Vibration and Control Settings (2026-10-26)

Scope: the Options screen's Control Settings page and the whole rumble system behind its
Vibration row, from the skater's physics to the pad's motors. Addresses are the debug xex's.

## Control Settings

- `eSk8FEGameOptionsScreen_ControlSettings` is `gGameOptionsScreens` (0x82fdfa70) row 8, titled
  `ID_GAMESETTINGS_CONTROL_SETTINGS` ("Control Settings"). Its rows are
  `eSk8FEGameOption_OffboardYAxisToggle` ("Offboard Y Axis") and
  `eSk8FEGameOption_VibrationToggle` ("Vibration"), both `selector`s. The main list (row 0, the
  front end) is Audio, Video, Control, Online Settings and Device Selection; in game (row 1) the
  Music Player, Audio, Video, Control and Online Settings. `OnMenuSelect` (0x826871a0) opens the
  page as it opens Audio Settings (A sound, `SetScreenState(1)`).
- `GetStringValue` (0x82643d48): Offboard Y Axis reads "Normal" (`ID_GAMESETTINGS_AXIS_NORMAL`)
  with `tSettings::mInvertYAxis` set and "Inverted" otherwise; a new profile's is clear
  (`tSettings::Default` 0x825644d8), so it reads "Inverted". Vibration reads "On"/"Off" from
  `mRumble`, on by default. `CameraManager::Update` (0x82584448) hands `mInvertYAxis` to the
  camera every update.
- `ReceiveInput` (0x82686978): left or right on either row sounds `core_nav_left`/`right` and
  toggles it. `ToggleInvertYAxis` (0x8265c948) flips `mInvertYAxis`. `ToggleVibration`
  (0x8265ca68) flips `mRumble`, logs it, sends `cMsgRumbleState` with it and, turning vibration on,
  sets the front end's controller's `Vibration::mEnabled` and adds a test rumble (0.5 on both
  motors, decay 5, event 0). Both mark the profile dirty for `RequestExit`'s save.
- `eSk8FEGameOption_OffboardXAxisToggle` is on no screen; `SixaxisToggle` is the PS3's.

## The rumble system

- `Vibration` (one per pad, made by `InputManager::Init` 0x826e2788 with `mEnabled` set):
  eight channels, each a low frequency, a high frequency and a per-frame decay factor, mapped
  from the event that set them. `AddRumbleEvent` (0x826e2300), only while
  `kInputEnableVibration` and `mEnabled` hold: the event's channel (a free one the first time,
  none while all eight are taken) takes both frequencies clamped to 0..1 and
  `clamp(1 - decay / 60)`. `Update` (0x826e1650): the channels' sums (`kInputEnableAdditiveVibration`),
  each capped at `(1 - max) * kVibrationCapScale + max` of the strongest channel
  (`kInputEnableVibrationCap`, scale 0.2) and clamped to 0..1, go to actuator 0 (low frequency)
  and 1 (high); every channel then multiplies by its factor and is freed once both values are
  below `kVibrationCutOffFreq` (0.001). `Reset` (0x826e1590) frees every channel and resets the
  pad's actuators.
- `InputManager::UpdateVibration` (0x826e22b0) runs every pad's `Update` at the end of
  `Sk8AppMan::PreUpdate` (0x8272c478), each frame.
- `cXenonPadDevice::SetActuator` (0x8298d178): on a wireless pad (`XINPUT_CAPS_WIRELESS`) the
  value is first `scale * value + offset`, clamped: `kLowFreqScale` 1.2 and `kLowFreqOffset`
  -0.02, `kHighFreqScale` 1.05 and `kHighFreqOffset` 0.05. Zero stays zero, anything else is
  `value * 65535`; the pad's next sample sends a changed pair with `XInputSetState`.
- `PlayerUI` (its constructor 0x828ae0f0 registers the four handlers) acts for its controller:
  `cMsgRumble` (event 0) and `cMsgRumbleChannel` (its event) of its player add a rumble,
  `cMsgRumbleState` sets `mEnabled`, `cMsgRumbleReset` resets the `Vibration`.
- `cMsgRumbleState` carries `mRumble`: `ProfileBackEnd::LoadData` (0x8256d820) after the
  settings load, `ResetProfile` (from the dashboard's vibration setting), `FrontEndManager::StartInGame`
  and `ToggleVibration` send it.
- `StateManagerFrontEnd::ResetRumble` (0x82656998), which `UpdateOnEnter` (0x82d663f0) calls for
  every front-end state entered and `ExecutePopState` for every pop, sends `cMsgRumbleReset` in a
  game (`mFEState != eFE_STATE_IN_FE`).
- `RumbleManager` (`SkaterManager`'s, constructor 0x825fa948): `SkaterManager::PostUpdate`
  (0x825f8b88) runs `UpdateRumbleEvents` (0x825f64e0) over the local skater's `PhysOut`, with a
  1/60 step, in a game while FE state 7 (the StartHub) is not on the stack. Each of its fifteen
  events (`eRumbleEvent` order) is a `rumble` collection; `ProcessRumbleEvent` (0x825f2b10), for an
  enabled event whose condition holds, maps the event's input over the layout's range (through
  its curves when they are on), scales it by the surface's rumble and the wheels' pattern when the
  event takes them, then by the layout's scale and offset, then post-processes it. A result not
  within 2^-16 of zero on both frequencies is a `cMsgRumbleChannel` on the event's channel.
- The surface rumbles (`RumbleDataMapper`) come from three `rumble_surface` collections: the
  physics surface type for the power slide and the two brakes, the first wheel's audio surface's
  rolling material for the wheels, the contacting truck's or the deck's grind material for the
  trucks and the deck (an untagged grind for an undefined surface). The wheels' pattern
  (`rumble_pattern`, by the first wheel's surface pattern) advances by the distance rolled and
  wraps; it restarts when the pattern changes.

| event | enabled | input | condition | decay |
|---|---|---|---|---|
| SkeletonImpact | yes | sum of the skeleton's contact forces (scaled 0.75 unless wiping out) | always | 20 |
| DeckImpact | yes | averaged deck contact force | a foot on the deck | 20 |
| Landing | yes | impact of the wheels that just landed | a foot on the deck, a wheel just landed | 15 |
| PowerSlide | yes | wheel slide, physics surface | a foot on the deck, wheels down | 10 |
| SpeedWobble | no | none | always | 3 |
| Push | no | deck acceleration | physically pushing | 10 |
| Wheels | yes | surface speed, rolling surface and pattern | a foot on the deck, wheels down | 10 |
| Trucks | yes | surface speed, grind surface | a foot on the deck, a truck in contact | 20 |
| Deck | yes | surface speed, grind surface | a foot on the deck, the deck in contact | 20 |
| FootBrake | yes | surface speed, physics surface | foot braking | 30 |
| DeckBrake | yes | surface speed, physics surface | manual braking | 20 |
| FootSteps | yes | the centre of mass's fall speed | a foot just landed, no object held | 5 |
| Pumping | yes | pumping acceleration (zero unless meant) | always | 5 |
| MoveObject | yes | the held object's movement over its mass curve | holding an object | 40 |
| Carving | no | carving slide | a foot on the deck, wheels down, carving | 15 |

Every enabled event decays, so a rumble whose event stops dies out within about 1.3 seconds.
The Lua bindings of the four messages exist, but no shipped script or challenge posts them.

## The port

- `input/input_vibration.rs`: `Vibration`; `input/input_rumble_manager.rs`: the manager, its
  events, surfaces and pattern, read once from the skater vault (the three rumble classes are in
  the selected classes); `physics/player/output/physical_player_phys_out_rumble.rs`: the `PhysOut`
  fields the events read. The device runtime holds a `Vibration` per pad
  (`InputManager::mVibration`) and its `UpdateVibration`.
- `session/selected_session_vibration.rs`: after each completed simulation tick of a game (no
  paused simulation, no replay, no StartHub or screen it pushed) the manager's rumbles go to the
  player's controller; at the end of every frame a front-end state a game entered or left resets
  the controller's rumble, its `mEnabled` follows the profile's Vibration setting, Options' test
  rumble plays, and every pad's rumble goes to its motors.
- `ui/options.rs`: the Control Settings page with both rows; the PC Display Settings page follows
  it. The Offboard Y Axis row drives the existing camera setting.
- PC port changes:
  - A request of zero stays zero on a wireless pad. The 360 tuning's high-frequency offset would
    otherwise hold that motor at 5% whenever nothing rumbles.
  - While the window has no focus the motors are still (the game takes no pad input then); on
    its return they take what the game last set.
  - The pads' owner leaves every motor still when it is dropped, the pad thread stops every
    SDL pad's rumble when it ends, and a panic of the game's SIM or window thread stops every
    motor, since a pad keeps its last command after the process.
- The Lua rumble bindings are not ported.
