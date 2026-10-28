//! PC port addition (user request): RPCS3's stick settings for each analog stick, applied to a
//! pad's sample before the game reads it, as RPCS3 applies its own before the emulated pad
//! (`PadHandlerBase::get_mapping`): each axis scaled by the multiplier and kept to its range
//! (`TranslateButtonPress`'s `NormalizeStickInput`, `MultipliedInput`), then a radial deadzone
//! whose inside ramps up to the anti-deadzone and whose outside starts there
//! (`NormalizeStickDeadzone`). They work at the pad's own 16-bit precision, where RPCS3 hands a
//! game a DualShock 3's 8-bit axes. The game's own stick conversion (`try_convert_xbox_pad`, with
//! its deadzone of a quarter of the stick's travel) follows unchanged, and at the vanilla settings
//! a sample passes through as the pad reported it.
use crate::input_pad::XboxPadSample;
use std::ops::RangeInclusive;

/// The multipliers a stick takes, in hundredths.
pub const MULTIPLIERS: RangeInclusive<u8> = 40..=110;
/// The deadzones and anti-deadzones a stick takes, in percent of its travel.
pub const DEADZONES: RangeInclusive<u8> = 0..=50;

/// One stick's settings: the multiplier in hundredths, the deadzone and the anti-deadzone in
/// percent of the stick's travel from its centre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StickSettings {
    pub multiplier: u8,
    pub deadzone: u8,
    pub anti_deadzone: u8,
}

impl Default for StickSettings {
    fn default() -> Self {
        Self::VANILLA
    }
}

impl StickSettings {
    /// The stick as the game reads it without the PC settings.
    pub const VANILLA: Self = Self { multiplier: 100, deadzone: 0, anti_deadzone: 0 };

    /// The settings kept to [`MULTIPLIERS`] and [`DEADZONES`].
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            multiplier: self.multiplier.clamp(*MULTIPLIERS.start(), *MULTIPLIERS.end()),
            deadzone: self.deadzone.clamp(*DEADZONES.start(), *DEADZONES.end()),
            anti_deadzone: self.anti_deadzone.clamp(*DEADZONES.start(), *DEADZONES.end()),
        }
    }

    /// The stick's position, each axis in `-1..=1`, after the multiplier alone: each axis kept
    /// to its range, as `NormalizeStickInput` keeps each direction to the stick's maximum.
    #[must_use]
    pub fn multiplied(self, x: f32, y: f32) -> [f32; 2] {
        let multiplier = f32::from(self.multiplier) / 100.;
        [(x * multiplier).clamp(-1., 1.), (y * multiplier).clamp(-1., 1.)]
    }

    /// The stick's position, each axis in `-1..=1`, as these settings hand it on: the
    /// multiplier, then `NormalizeStickDeadzone`. Inside the deadzone the stick's distance from
    /// its centre maps onto `0..anti-deadzone`, and outside it onto `anti-deadzone..=1`.
    #[must_use]
    pub fn apply_axes(self, x: f32, y: f32) -> [f32; 2] {
        let [x, y] = self.multiplied(x, y);
        let magnitude = x.hypot(y).min(1.);
        if magnitude <= 0. {
            return [x, y];
        }
        let deadzone = f32::from(self.deadzone) / 100.;
        let anti_deadzone = f32::from(self.anti_deadzone) / 100.;
        let position = if deadzone <= 0. || magnitude > deadzone {
            lerp(anti_deadzone, 1., (magnitude - deadzone) / (1. - deadzone))
        } else {
            lerp(0., anti_deadzone, magnitude / deadzone)
        };
        let scale = position / magnitude;
        [x * scale, y * scale]
    }

    /// A raw stick (`XINPUT_GAMEPAD`'s thumb axes) as these settings hand it to the game.
    #[must_use]
    pub fn apply(self, x: i16, y: i16) -> (i16, i16) {
        if self == Self::VANILLA {
            return (x, y);
        }
        let [x, y] = self.apply_axes(axis(x), axis(y));
        (raw(x), raw(y))
    }
}

/// Both sticks' settings, which `controller_config.json` keeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControllerSettings {
    pub left: StickSettings,
    pub right: StickSettings,
}

impl ControllerSettings {
    pub const VANILLA: Self = Self { left: StickSettings::VANILLA, right: StickSettings::VANILLA };

    /// A pad's sample with both sticks as these settings hand them to the game; its buttons and
    /// triggers as they were.
    #[must_use]
    pub fn apply(&self, sample: XboxPadSample) -> XboxPadSample {
        let (left_x, left_y) = self.left.apply(sample.left_x, sample.left_y);
        let (right_x, right_y) = self.right.apply(sample.right_x, sample.right_y);
        XboxPadSample { left_x, left_y, right_x, right_y, ..sample }
    }
}

/// A raw axis in `-1..=1`, as the game's conversion scales it (`try_convert_xbox_pad`'s 2^-15).
#[must_use]
pub fn axis(raw: i16) -> f32 {
    f32::from(raw) / 32768.
}

#[expect(clippy::cast_possible_truncation, reason = "rounded and clamped to the axis range first")]
fn raw(axis: f32) -> i16 {
    (axis * 32768.).round().clamp(-32768., 32767.) as i16
}

/// `std::lerp`.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (b - a).mul_add(t, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AXES: [i16; 13] = [-32768, -30000, -20000, -9000, -2000, -500, 0, 500, 2000, 9000, 20000, 30000, 32767];

    fn settings(multiplier: u8, deadzone: u8, anti_deadzone: u8) -> StickSettings {
        StickSettings { multiplier, deadzone, anti_deadzone }
    }

    /// RPCS3's XInput handler on one stick (`thumb_max` 32767), the deadzones in its units: each
    /// direction's value (`get_button_values`), `TranslateButtonPress`'s `NormalizeStickInput`
    /// (the multiplier, no threshold), the directions' difference, then
    /// `NormalizeStickDeadzone` and `ConvertAxis`, as `get_mapping` runs them.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss, reason = "RPCS3's own casts")]
    fn rpcs3(x: i16, y: i16, stick: StickSettings) -> (u16, u16) {
        const THUMB_MAX: f32 = 32767.;
        let direction = |raw: i16, positive: bool| -> i32 {
            let value = if positive { i32::from(raw).max(0) } else if raw < 0 { i32::from(raw).abs() - 1 } else { 0 };
            let scaled = i32::from(stick.multiplier) * value / 100;
            i32::from((255. * (scaled as f32).clamp(0., THUMB_MAX) / THUMB_MAX) as u16)
        };
        let value = |raw: i16| (direction(raw, true) - direction(raw, false)) as f32 / 255.;
        let (mut x, mut y) = (value(x), value(y));
        let magnitude = x.hypot(y).min(1.);
        if magnitude > 0. {
            let deadzone = (f32::from(stick.deadzone) * THUMB_MAX / 100.).trunc() / THUMB_MAX;
            let anti_deadzone = (f32::from(stick.anti_deadzone) * THUMB_MAX / 100.).trunc() / THUMB_MAX;
            let position = if deadzone <= 0. || magnitude > deadzone {
                anti_deadzone + (1. - anti_deadzone) * (magnitude - deadzone) / (1. - deadzone)
            } else {
                anti_deadzone * magnitude / deadzone
            };
            x *= position / magnitude;
            y *= position / magnitude;
        }
        let convert = |value: f32| ((f64::from(value) + 1.) * (255. / 2.)) as u16;
        (convert(x), convert(y))
    }

    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "the DualShock 3 axis RPCS3 converts to")]
    fn as_rpcs3_axis(raw: i16) -> u16 {
        ((f64::from(axis(raw)) + 1.) * (255. / 2.)) as u16
    }

    #[test]
    fn vanilla_hands_every_sample_on_as_the_pad_reported_it() {
        for &x in &AXES {
            for &y in &AXES {
                assert_eq!(StickSettings::VANILLA.apply(x, y), (x, y));
                // The arithmetic itself changes nothing at the vanilla settings either.
                let [ax, ay] = StickSettings::VANILLA.apply_axes(axis(x), axis(y));
                assert_eq!((raw(ax), raw(ay)), (x, y));
            }
        }
        let sample = XboxPadSample { buttons: 0x1234, left_trigger: 7, right_trigger: 200, left_x: -3, left_y: 32767, right_x: -32768, right_y: 99 };
        assert_eq!(ControllerSettings::VANILLA.apply(sample), sample);
        assert_eq!(ControllerSettings::default(), ControllerSettings::VANILLA);
    }

    #[test]
    fn a_deadzone_holds_the_stick_still_inside_and_scales_outside() {
        let stick = settings(100, 20, 0);
        assert_eq!(stick.apply(3277, 0), (0, 0), "a tenth of the travel");
        assert_eq!(stick.apply(0, -6000), (0, 0));
        // Three fifths out is half way through what lies past the deadzone.
        assert_eq!(stick.apply(19661, 0), (16384, 0));
        assert_eq!(stick.apply(-32768, 0), (-32768, 0));
    }

    #[test]
    fn an_anti_deadzone_starts_the_stick_past_it() {
        // Inside a tenth's deadzone the stick ramps up to a quarter; outside, it starts there.
        let stick = settings(100, 10, 25);
        assert_eq!(stick.apply(1638, 0), (4095, 0));
        assert_eq!(stick.apply(18022, 0), (20480, 0));
        // Without a deadzone the least movement starts at the anti-deadzone.
        let stick = settings(100, 0, 25);
        assert_eq!(stick.apply(0, 1311), (0, 9175));
        assert_eq!(stick.apply(0, 0), (0, 0));
    }

    #[test]
    fn the_multiplier_scales_each_axis_and_keeps_it_to_its_range() {
        assert_eq!(settings(50, 0, 0).apply(26214, -26214), (13107, -13107));
        // Past full travel each axis stops at its end, as RPCS3 keeps each direction to the
        // stick's maximum.
        assert_eq!(settings(110, 0, 0).apply(31130, 31130), (32767, 32767));
        assert_eq!(settings(110, 0, 0).apply(-32768, 0), (-32768, 0));
    }

    #[test]
    fn every_setting_moves_the_stick_as_rpcs3_does() {
        let cases = [
            settings(100, 0, 0),
            settings(40, 0, 0),
            settings(110, 0, 0),
            settings(100, 20, 0),
            settings(100, 0, 25),
            settings(100, 10, 25),
            settings(70, 15, 30),
            settings(110, 50, 50),
            settings(85, 50, 0),
        ];
        for stick in cases {
            for &x in &AXES {
                for &y in &AXES {
                    let (ours_x, ours_y) = stick.apply(x, y);
                    let ours = (as_rpcs3_axis(ours_x), as_rpcs3_axis(ours_y));
                    let theirs = rpcs3(x, y, stick);
                    // RPCS3 rounds each direction to 8 bits before the deadzone; nothing more apart.
                    assert!(
                        ours.0.abs_diff(theirs.0) <= 2 && ours.1.abs_diff(theirs.1) <= 2,
                        "{stick:?} at ({x}, {y}): {ours:?} against RPCS3's {theirs:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn settings_out_of_range_are_kept_to_it() {
        assert_eq!(settings(0, 90, 51).clamped(), settings(40, 50, 50));
        assert_eq!(settings(255, 0, 0).clamped(), settings(110, 0, 0));
        assert_eq!(StickSettings::VANILLA.clamped(), StickSettings::VANILLA);
    }
}
