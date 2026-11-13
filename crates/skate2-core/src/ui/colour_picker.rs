//! PC port addition: the colour picker Edit Skater opens on a colour row whose colours are tints of
//! the parts' shaders (clothing, hair and skin; an eye or deck colour is a part of its own). See
//! `docs/evidence/edit-skater-colour-picker-2026-11-13.md`.
//!
//! A colour is HSV. The wheel holds hue (red at the top, turning clockwise through yellow, green,
//! cyan, blue and magenta) and saturation (white at the centre, the full colour at the rim);
//! intensity is the value, the tint's largest channel. The game's own colours take a tint past 1
//! (Slappy's white top is 1.8, 1.8, 2.2), so intensity runs to [`MAX_INTENSITY`], and its slider
//! gives the first [`FULL_AT`] of its length to 0..=1 and the rest to the brighter tints.

use std::f32::consts::TAU;

/// The largest intensity, past every colour the game's own rows hold.
pub const MAX_INTENSITY: f32 = 2.5;
/// Where intensity 1 sits along the slider (0 to 1).
pub const FULL_AT: f32 = 0.6;
/// The largest tint channel a picked colour writes, the bound the replay archive and the online
/// character keep a skater's colours within.
pub const MAX_TINT: f32 = 8.0;

/// Full deflection moves the cursor this many wheel radii a second.
const CURSOR_SPEED: f32 = 1.2;
/// Full deflection, or a trigger pulled all the way, moves the intensity slider this much of its
/// length a second.
const SLIDER_SPEED: f32 = 0.55;
/// A D-pad step, in wheel radii.
const NUDGE: f32 = 0.025;
const STICK_DEADZONE: f32 = 0.2;
const TRIGGER_DEADZONE: f32 = 0.1;
/// The longest update a step integrates: a hitch does not throw the cursor across the wheel.
const MAX_STEP_SECONDS: f32 = 0.1;

/// A picked colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    /// 0 (red) to 1, around the wheel.
    pub hue: f32,
    /// 0 (white) to 1 (the full colour), out from the wheel's centre.
    pub saturation: f32,
    /// 0 (black) to [`MAX_INTENSITY`].
    pub intensity: f32,
}

impl Hsv {
    /// The colour of a tint. A channel below 0 or not a number reads as 0.
    #[must_use]
    pub fn from_rgb(rgb: [f32; 3]) -> Self {
        let [r, g, b] = rgb.map(|c| if c.is_finite() { c.max(0.0) } else { 0.0 });
        let max = r.max(g).max(b);
        let delta = max - r.min(g).min(b);
        let hue = if delta <= 0.0 {
            0.0
        } else if max == r {
            ((g - b) / delta).rem_euclid(6.0) / 6.0
        } else if max == g {
            ((b - r) / delta + 2.0) / 6.0
        } else {
            ((r - g) / delta + 4.0) / 6.0
        };
        Self {
            hue: wrap(hue),
            saturation: if max > 0.0 { delta / max } else { 0.0 },
            intensity: max.min(MAX_INTENSITY),
        }
    }

    /// The tint: the colour at its intensity.
    #[must_use]
    pub fn rgb(self) -> [f32; 3] {
        full(self.hue, self.saturation).map(|c| c * self.intensity)
    }

    /// The colour a swatch shows, each channel 0 to 1: the tint itself up to intensity 1, then the
    /// full colour lightened toward white, half way there at [`MAX_INTENSITY`].
    #[must_use]
    pub fn display(self) -> [f32; 3] {
        let colour = full(self.hue, self.saturation);
        if self.intensity <= 1.0 {
            colour.map(|c| c * self.intensity.max(0.0))
        } else {
            let toward = ((self.intensity - 1.0) / (MAX_INTENSITY - 1.0)).min(1.0) * 0.5;
            colour.map(|c| (1.0 - c).mul_add(toward, c))
        }
    }

    /// [`Self::display`] as a swatch's bytes, truncated as the row's own swatches are.
    #[must_use]
    pub fn display_bytes(self) -> [u8; 3] {
        self.display().map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8)
    }

    /// Where the colour sits on the wheel: x to the right and y down, within the unit circle.
    #[must_use]
    pub fn cursor(self) -> [f32; 2] {
        let angle = self.hue * TAU;
        [self.saturation * angle.sin(), -self.saturation * angle.cos()]
    }

    /// The colour at wheel point `at`, kept within the wheel; the centre keeps this colour's hue.
    #[must_use]
    pub fn with_cursor(self, [x, y]: [f32; 2]) -> Self {
        let distance = x.hypot(y);
        Self {
            hue: if distance > 1e-6 { wrap(x.atan2(-y) / TAU) } else { self.hue },
            saturation: distance.min(1.0),
            ..self
        }
    }

    /// Where the intensity sits along its slider (0 to 1).
    #[must_use]
    pub fn slider(self) -> f32 {
        slider_at(self.intensity)
    }

    /// The colour with the intensity at slider position `at`.
    #[must_use]
    pub fn with_slider(self, at: f32) -> Self {
        Self { intensity: intensity_at(at), ..self }
    }
}

/// `hue` within 0..1, where 1 (the top again) reads as 0.
fn wrap(hue: f32) -> f32 {
    let hue = hue.rem_euclid(1.0);
    if hue >= 1.0 { 0.0 } else { hue }
}

/// The colour of `hue` at `saturation` and value 1.
fn full(hue: f32, saturation: f32) -> [f32; 3] {
    let h = wrap(hue) * 6.0;
    let sector = h.floor();
    let f = h - sector;
    let s = saturation.clamp(0.0, 1.0);
    let (p, q, t) = (1.0 - s, s.mul_add(-f, 1.0), s.mul_add(f - 1.0, 1.0));
    match sector as u32 {
        0 => [1.0, t, p],
        1 => [q, 1.0, p],
        2 => [p, 1.0, t],
        3 => [p, q, 1.0],
        4 => [t, p, 1.0],
        _ => [1.0, p, q],
    }
}

/// The slider position (0 to 1) of `intensity`.
#[must_use]
pub fn slider_at(intensity: f32) -> f32 {
    let intensity = if intensity.is_finite() { intensity.clamp(0.0, MAX_INTENSITY) } else { 0.0 };
    if intensity <= 1.0 {
        intensity * FULL_AT
    } else {
        ((intensity - 1.0) / (MAX_INTENSITY - 1.0)).mul_add(1.0 - FULL_AT, FULL_AT)
    }
}

/// The intensity at slider position `at` (0 to 1).
#[must_use]
pub fn intensity_at(at: f32) -> f32 {
    let at = if at.is_finite() { at.clamp(0.0, 1.0) } else { 0.0 };
    if at <= FULL_AT {
        at / FULL_AT
    } else {
        ((at - FULL_AT) / (1.0 - FULL_AT)).mul_add(MAX_INTENSITY - 1.0, 1.0)
    }
}

/// What moves the picker during one update.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PickerInput {
    /// The left stick, x right and y up (-1 to 1): moves the cursor over the wheel.
    pub stick: [f32; 2],
    /// The right stick, x right and y up: right or up brightens, left or down darkens.
    pub right_stick: [f32; 2],
    /// The left and right triggers (0 to 1): the left darkens, the right brightens.
    pub triggers: [f32; 2],
    /// D-pad steps of the cursor: x right and y down.
    pub nudge: [i8; 2],
}

impl PickerInput {
    /// A stick or a trigger moves the colour, as a morph slider's stick does.
    #[must_use]
    pub fn analog(&self) -> bool {
        cursor_rate(self.stick) > 0.0 || self.intensity_rate() != 0.0
    }

    /// The intensity slider's rate, -1 to 1 of [`SLIDER_SPEED`]: the right stick's longer axis and
    /// the triggers.
    fn intensity_rate(&self) -> f32 {
        let [x, y] = self.right_stick;
        let axis = if x.abs() >= y.abs() { x } else { y };
        let stick = response(axis.abs(), STICK_DEADZONE).copysign(axis);
        let [left, right] = self.triggers;
        (stick + response(right, TRIGGER_DEADZONE) - response(left, TRIGGER_DEADZONE)).clamp(-1.0, 1.0)
    }
}

/// A control's travel past its deadzone, squared for fine control near rest (0 to 1).
fn response(travel: f32, deadzone: f32) -> f32 {
    if !travel.is_finite() {
        return 0.0;
    }
    ((travel - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0).powi(2)
}

/// The cursor's speed under the left stick, 0 to 1 of [`CURSOR_SPEED`].
fn cursor_rate([x, y]: [f32; 2]) -> f32 {
    response(x.hypot(y), STICK_DEADZONE)
}

/// The colour `input` moves `hsv` to over `seconds`.
#[must_use]
pub fn step(hsv: Hsv, input: &PickerInput, seconds: f32) -> Hsv {
    let seconds = if seconds.is_finite() { seconds.clamp(0.0, MAX_STEP_SECONDS) } else { 0.0 };
    let mut out = hsv;
    let [mut x, mut y] = hsv.cursor();
    let mut moved = false;
    let rate = cursor_rate(input.stick);
    if rate > 0.0 {
        let [sx, sy] = input.stick;
        let length = sx.hypot(sy);
        let distance = rate * CURSOR_SPEED * seconds;
        x += sx / length * distance;
        y -= sy / length * distance;
        moved = true;
    }
    if input.nudge != [0, 0] {
        x += f32::from(input.nudge[0]) * NUDGE;
        y += f32::from(input.nudge[1]) * NUDGE;
        moved = true;
    }
    if moved {
        out = out.with_cursor([x, y]);
    }
    let rate = input.intensity_rate();
    if rate != 0.0 {
        out = out.with_slider(rate.mul_add(SLIDER_SPEED * seconds, hsv.slider()));
    }
    out
}

/// A colour as the vault and a skater's customizations hold it: the CAS shaders' base (cloth, skin
/// and face multiply their diffuse by it; hair takes it for its diffuse) and highlight (hair's
/// specular colour).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tint {
    pub base: [f32; 3],
    pub highlight: [f32; 3],
}

/// The tint of a row's colours that its swatches show, which the picker sets; the other follows
/// it as the row's nearest colour has it ([`ratio`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Primary {
    Base,
    Highlight,
}

impl Primary {
    /// The tint a row's swatch shows: the base for clothing (whose white has no highlight), the
    /// highlight for hair and skin (whose swatches are their highlights). `presets` holds each of
    /// the row's colours with its swatch.
    #[must_use]
    pub fn of_row(presets: &[(Tint, [f32; 3])]) -> Self {
        let lit = |c: [f32; 3]| c.iter().any(|&v| v > 0.01);
        if presets.iter().any(|(tint, swatch)| lit(*swatch) && !lit(tint.highlight)) {
            return Self::Base;
        }
        if presets.iter().any(|(tint, swatch)| lit(*swatch) && !lit(tint.base)) {
            return Self::Highlight;
        }
        let distance = |a: [f32; 3], b: [f32; 3]| (0..3).map(|i| (a[i] - b[i]).abs()).sum::<f32>();
        let base: f32 = presets.iter().map(|(tint, swatch)| distance(tint.base, *swatch)).sum();
        let highlight: f32 = presets.iter().map(|(tint, swatch)| distance(tint.highlight, *swatch)).sum();
        if highlight < base { Self::Highlight } else { Self::Base }
    }

    /// This tint of `tint`.
    #[must_use]
    pub fn of(self, tint: Tint) -> [f32; 3] {
        match self {
            Self::Base => tint.base,
            Self::Highlight => tint.highlight,
        }
    }

    /// The other tint of `tint`.
    #[must_use]
    pub fn other(self, tint: Tint) -> [f32; 3] {
        match self {
            Self::Base => tint.highlight,
            Self::Highlight => tint.base,
        }
    }

    /// The tint whose primary is `primary` and whose other tint is `primary` times `ratio`, each
    /// channel within 0..=[`MAX_TINT`].
    #[must_use]
    pub fn tint(self, primary: [f32; 3], ratio: [f32; 3]) -> Tint {
        let primary = primary.map(|c| if c.is_finite() { c.clamp(0.0, MAX_TINT) } else { 0.0 });
        let secondary: [f32; 3] = std::array::from_fn(|i| {
            let c = primary[i] * ratio[i];
            if c.is_finite() { c.clamp(0.0, MAX_TINT) } else { 0.0 }
        });
        match self {
            Self::Base => Tint { base: primary, highlight: secondary },
            Self::Highlight => Tint { base: secondary, highlight: primary },
        }
    }
}

/// The largest per channel ratio of a colour's other tint to its primary.
const MAX_RATIO: f32 = 4.0;
/// A primary channel this dark takes the colour's overall ratio, not its own.
const DARK_CHANNEL: f32 = 0.02;

/// How a colour's other tint follows its primary, each channel's ratio of the two: the overall ratio
/// for a channel too dark to tell, each within 0..=4. A picked colour takes the ratio of the row's
/// colour nearest it, so a row's own colour picked is that colour again, and a skin colour's base
/// keeps the calibration of the skin parts its nearest colour wears.
#[must_use]
pub fn ratio(primary: [f32; 3], secondary: [f32; 3]) -> [f32; 3] {
    let clean = |c: [f32; 3]| c.map(|v| if v.is_finite() { v.max(0.0) } else { 0.0 });
    let (primary, secondary) = (clean(primary), clean(secondary));
    let total: f32 = primary.iter().sum();
    let overall = if total > DARK_CHANNEL {
        secondary.iter().sum::<f32>() / total
    } else if secondary.iter().any(|&c| c > 0.0) {
        1.0
    } else {
        0.0
    };
    std::array::from_fn(|i| {
        let r = if primary[i] >= DARK_CHANNEL { secondary[i] / primary[i] } else { overall };
        r.clamp(0.0, MAX_RATIO)
    })
}

/// The index of the colour in `primaries` nearest `rgb`: the row's own colour a picked colour
/// stands for where the game reads a colour by name (the hair colour's hairline shadow, the skin
/// colour's skin parts).
#[must_use]
pub fn nearest(primaries: &[[f32; 3]], rgb: [f32; 3]) -> Option<usize> {
    let distance = |c: &[f32; 3]| (0..3).map(|i| (c[i] - rgb[i]).powi(2)).sum::<f32>();
    primaries
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| distance(a).total_cmp(&distance(b)))
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4)
    }

    /// The vault's colour rows (`skater.vlt`, `cac_colour`): each tint and swatch of Slappy's
    /// tops (zone 9), of Back Alley Pete's hair (zone 1) and skin (zone 2).
    fn tops() -> Vec<(Tint, [f32; 3])> {
        let same = |c: [f32; 3]| (Tint { base: c, highlight: c }, c);
        vec![
            same([0.1, 0.1, 0.7]),
            same([0.1, 0.7, 0.1]),
            same([0.1, 0.1, 0.1]),
            (Tint { base: [1.8, 1.8, 2.2], highlight: [0.0; 3] }, [0.9, 0.9, 0.9]),
            same([0.7, 0.1, 0.1]),
            same([0.7, 0.7, 0.1]),
            same([0.4, 0.1, 0.4]),
        ]
    }
    fn hair() -> Vec<(Tint, [f32; 3])> {
        vec![
            (Tint { base: [0.09, 0.06, 0.06], highlight: [0.09, 0.06, 0.03] }, [0.17, 0.12, 0.1]),
            (Tint { base: [0.28, 0.15, 0.11], highlight: [0.22, 0.17, 0.04] }, [0.4, 0.3, 0.2]),
            (Tint { base: [0.05, 0.05, 0.05], highlight: [0.07, 0.06, 0.04] }, [0.0, 0.0, 0.0]),
            (Tint { base: [0.2, 0.19, 0.18], highlight: [0.44, 0.43, 0.38] }, [0.62, 0.62, 0.62]),
            (Tint { base: [0.63, 0.43, 0.25], highlight: [0.8, 0.73, 0.6] }, [1.0, 0.9, 0.65]),
            (Tint { base: [0.3, 0.1, 0.08], highlight: [0.21, 0.1, 0.02] }, [0.45, 0.15, 0.117]),
        ]
    }
    fn skin() -> Vec<(Tint, [f32; 3])> {
        vec![
            (Tint { base: [1.5, 1.31, 1.31], highlight: [0.58, 0.41, 0.34] }, [0.58, 0.41, 0.34]),
            (Tint { base: [0.71, 0.6, 0.52], highlight: [0.71, 0.54, 0.46] }, [0.71, 0.54, 0.46]),
            (Tint { base: [0.65, 0.71, 0.85], highlight: [0.74, 0.52, 0.44] }, [0.38, 0.3, 0.27]),
        ]
    }

    #[test]
    fn hsv_round_trips_the_game_colours_and_reads_their_intensity() {
        for rgb in [[0.7, 0.1, 0.1], [0.1, 0.7, 0.1], [0.1, 0.1, 0.7], [0.4, 0.1, 0.4], [0.1, 0.1, 0.1], [1.8, 1.8, 2.2], [0.0; 3]] {
            assert!(close(Hsv::from_rgb(rgb).rgb(), rgb), "{rgb:?}");
        }
        let red = Hsv::from_rgb([0.7, 0.1, 0.1]);
        assert_eq!(red.hue, 0.0);
        assert!((red.saturation - 6.0 / 7.0).abs() < 1e-6);
        assert!((red.intensity - 0.7).abs() < 1e-6);
        // A swatch shows a tint up to 1 as it is, so the picker opens on the colour the row showed.
        let swatch = red.display_bytes();
        assert!(swatch[0].abs_diff(178) <= 1 && swatch[1].abs_diff(25) <= 1 && swatch[2].abs_diff(25) <= 1, "{swatch:?}");
        // Slappy's white is past 1: it shows lightened, close to its swatch (0.9).
        let white = Hsv::from_rgb([1.8, 1.8, 2.2]).display();
        assert!(white.iter().all(|&c| c > 0.85 && c <= 1.0), "{white:?}");
        assert!(close(Hsv::from_rgb([f32::NAN, -1.0, 0.5]).rgb(), [0.0, 0.0, 0.5]));
        assert_eq!(Hsv::from_rgb([9.0, 0.0, 0.0]).intensity, MAX_INTENSITY);
    }

    #[test]
    fn the_wheel_puts_red_at_the_top_and_turns_clockwise() {
        let at = |hue: f32| Hsv { hue, saturation: 1.0, intensity: 1.0 }.cursor();
        let near = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5;
        assert!(near(at(0.0), [0.0, -1.0]), "red at the top");
        assert!(near(at(0.25), [1.0, 0.0]), "a quarter turn right");
        assert!(near(at(0.5), [0.0, 1.0]), "cyan at the bottom");
        assert!(near(at(0.75), [-1.0, 0.0]));
        for hue in [0.0, 0.1, 1.0 / 3.0, 0.5, 0.9] {
            for saturation in [0.25, 1.0] {
                let hsv = Hsv { hue, saturation, intensity: 0.7 };
                let back = hsv.with_cursor(hsv.cursor());
                assert!((back.hue - hue).abs() < 1e-5 && (back.saturation - saturation).abs() < 1e-5, "{hsv:?} {back:?}");
            }
        }
        // Past the rim stays on it; the centre keeps the hue, so moving back out keeps the colour.
        let hsv = Hsv { hue: 0.3, saturation: 0.5, intensity: 1.0 };
        assert_eq!(hsv.with_cursor([3.0, 0.0]).saturation, 1.0);
        assert_eq!(hsv.with_cursor([0.0, 0.0]), Hsv { saturation: 0.0, ..hsv });
    }

    #[test]
    fn the_slider_gives_most_of_its_length_to_the_colour_and_the_rest_to_brighter_tints() {
        assert_eq!(slider_at(0.0), 0.0);
        assert!((slider_at(1.0) - FULL_AT).abs() < 1e-6);
        assert!((slider_at(MAX_INTENSITY) - 1.0).abs() < 1e-6);
        for at in [0.0, 0.2, FULL_AT, 0.75, 1.0] {
            assert!((slider_at(intensity_at(at)) - at).abs() < 1e-6);
        }
        assert_eq!(intensity_at(2.0), MAX_INTENSITY);
        assert_eq!(slider_at(f32::NAN), 0.0);
    }

    #[test]
    fn sticks_move_the_cursor_and_the_intensity_and_rest_leaves_the_colour() {
        let red = Hsv::from_rgb([0.7, 0.1, 0.1]);
        let rest = PickerInput { stick: [0.1, -0.15], right_stick: [0.15, 0.0], triggers: [0.05, 0.08], nudge: [0, 0] };
        assert!(!rest.analog());
        assert_eq!(step(red, &rest, 1.0 / 60.0), red);
        // Full right for a tenth of a second: the cursor moves 0.12 radii right of the top.
        let right = step(red, &PickerInput { stick: [1.0, 0.0], ..Default::default() }, 0.1);
        let [x, y] = right.cursor();
        assert!((x - 0.12).abs() < 1e-4 && (y + red.saturation).abs() < 1e-4, "{x} {y}");
        assert_eq!(right.intensity, red.intensity);
        // A hitch integrates no more than a tenth of a second.
        assert_eq!(step(red, &PickerInput { stick: [1.0, 0.0], ..Default::default() }, 2.0), right);
        // The right trigger brightens, the left darkens, the right stick's longer axis as well.
        let brighter = step(red, &PickerInput { triggers: [0.0, 1.0], ..Default::default() }, 0.1);
        assert!((brighter.slider() - red.slider() - 0.055).abs() < 1e-5);
        assert!(step(red, &PickerInput { triggers: [1.0, 0.0], ..Default::default() }, 0.1).intensity < red.intensity);
        assert!(step(red, &PickerInput { right_stick: [0.3, -1.0], ..Default::default() }, 0.1).intensity < red.intensity);
        assert!(step(red, &PickerInput { right_stick: [1.0, 0.3], ..Default::default() }, 0.1).intensity > red.intensity);
        // A D-pad step moves the cursor by a fixed amount, and the slider stops at its ends.
        let nudged = step(red, &PickerInput { nudge: [0, 1], ..Default::default() }, 0.0);
        assert!((nudged.saturation - (red.saturation - NUDGE)).abs() < 1e-5);
        let black = Hsv { intensity: 0.0, ..red };
        assert_eq!(step(black, &PickerInput { triggers: [1.0, 0.0], ..Default::default() }, 0.1), black);
    }

    #[test]
    fn a_row_picks_the_tint_its_swatches_show() {
        assert_eq!(Primary::of_row(&tops()), Primary::Base, "white tops have no highlight");
        assert_eq!(Primary::of_row(&hair()), Primary::Highlight);
        assert_eq!(Primary::of_row(&skin()), Primary::Highlight);
        let tint = skin()[0].0;
        assert_eq!((Primary::Highlight.of(tint), Primary::Highlight.other(tint)), (tint.highlight, tint.base));
        assert_eq!((Primary::Base.of(tint), Primary::Base.other(tint)), (tint.base, tint.highlight));
    }

    #[test]
    fn the_other_tint_keeps_the_colour_s_own_relation_to_the_primary() {
        // Slappy's colours: the highlight is the base; white's highlight stays black.
        let red = tops()[4].0;
        assert_eq!(ratio(red.base, red.highlight), [1.0; 3]);
        let white = tops()[3].0;
        assert_eq!(ratio(white.base, white.highlight), [0.0; 3]);
        assert_eq!(Primary::Base.tint([0.2, 0.9, 0.3], [0.0; 3]), Tint { base: [0.2, 0.9, 0.3], highlight: [0.0; 3] });
        // The same primary gives the colour back.
        for (tint, _) in hair().into_iter().chain(skin()) {
            let r = ratio(tint.highlight, tint.base);
            let back = Primary::Highlight.tint(tint.highlight, r);
            assert!(close(back.base, tint.base) && close(back.highlight, tint.highlight), "{tint:?} {back:?}");
        }
        // A dark channel takes the overall ratio, and the ratio and the tint stay bounded.
        let r = ratio([0.5, 0.0, 0.5], [1.0, 0.7, 1.0]);
        assert!((r[1] - 2.7).abs() < 1e-5, "{r:?}");
        assert_eq!(ratio([0.02, 0.5, 0.5], [1.0, 0.5, 0.5])[0], MAX_RATIO);
        assert_eq!(Primary::Base.tint([2.5, 0.0, 0.0], [MAX_RATIO, 1.0, 1.0]).highlight[0], MAX_TINT);
        assert_eq!(ratio([0.0; 3], [0.0; 3]), [0.0; 3]);
    }

    #[test]
    fn a_picked_colour_stands_for_the_nearest_of_the_row_s_own() {
        let primaries: Vec<_> = tops().iter().map(|(tint, _)| tint.base).collect();
        assert_eq!(nearest(&primaries, [0.8, 0.05, 0.1]), Some(4), "red");
        assert_eq!(nearest(&primaries, [2.0, 2.0, 2.0]), Some(3), "white");
        assert_eq!(nearest(&[], [0.0; 3]), None);
    }
}
