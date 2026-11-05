//! Game Settings value rules: how a settings row's value changes on Left / Right.
//!
//! Ported from the retail settings input handler (TU3 `sub_8260C258`, events 14 = Left,
//! 15 = Right) and the value getters (`sub_8260D7E8` GetIntegerValue, `sub_8260D968`
//! GetStringValue). The retail constants are the defaults of [`retail_rule`]; every owner
//! (engine feature or mod) can stack its own rule on a row through the menu registry, so
//! ranges and steps are data, not code. Engine-independent and deterministic: slider
//! arithmetic runs in f32 with a fused multiply-add like the console's `fmadds`.
use crate::menus::SettingValue;

/// A Left / Right press on a settings row (serialisable by name).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    pub fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
    fn sign(self) -> f32 {
        match self {
            Self::Left => -1.0,
            Self::Right => 1.0,
        }
    }
}

/// How a row's value changes.
#[derive(Clone, Debug, PartialEq)]
pub enum ValueRule {
    /// Retail volume slider: `v = clamp(v + dir * step, min, max)`, then `v < snap -> min`.
    /// The row shows `floor(v * bars + 0.5)` of `bars` bars.
    Slider {
        min: f32,
        max: f32,
        step: f32,
        snap: f32,
        bars: f32,
    },
    /// Retail selector: `x = (x == 0)` on either direction (Left and Right only differ in
    /// their UI sound). `labels[0]` names 0 / off, `labels[1]` names 1 / on.
    Toggle { labels: [String; 2] },
    /// Integer `0..count`, -1 on Left / +1 on Right, wrapping at both ends.
    Cycle { count: i64, labels: Vec<String> },
}

impl ValueRule {
    /// Retail's volume slider (rows 7 / 8 / 9).
    pub fn retail_slider() -> Self {
        // [code] sub_8260C258: step 0.1 at 0x820641A8, min 0.0 at 0x82165A10, max 1.0 at
        // 0x8231A844 (also the Right direction), Left -1.0 at 0x8216DEE0, snap 0.05 at 0x82165A00.
        // [code] sub_8260D7E8: bars = v * 10.0 (0x821963E4) + 0.5 (0x8209975C), floored.
        Self::Slider {
            min: 0.0,
            max: 1.0,
            step: 0.1,
            snap: 0.05,
            bars: 10.0,
        }
    }

    fn toggle(off: &str, on: &str) -> Self {
        Self::Toggle {
            labels: [off.into(), on.into()],
        }
    }

    /// The value after one Left / Right press. A value of the wrong type is read as the
    /// rule's zero (retail stores every row as a plain number).
    pub fn step(&self, current: &SettingValue, direction: Direction) -> SettingValue {
        match self {
            Self::Slider {
                min,
                max,
                step,
                snap,
                ..
            } => {
                let v = number(current) as f32;
                // fmadds f0 = dir * step + v; retail compares against min first, then max.
                let mut next = direction.sign().mul_add(*step, v);
                if next < *min {
                    next = *min;
                } else if next > *max {
                    next = *max;
                }
                if next < *snap {
                    next = *min;
                }
                SettingValue::Float(next as f64)
            }
            Self::Toggle { .. } => match current {
                SettingValue::Bool(b) => SettingValue::Bool(!b),
                SettingValue::Int(i) => SettingValue::Int(i64::from(*i == 0)),
                SettingValue::Float(f) => SettingValue::Float(if *f == 0.0 { 1.0 } else { 0.0 }),
            },
            Self::Cycle { count, .. } => {
                let delta = match direction {
                    Direction::Left => -1,
                    Direction::Right => 1,
                };
                let next = number(current) as i64 + delta;
                // sub_8260DE60: > max -> 0, < 0 -> max (row 29 the same with count 2).
                let next = if next >= *count {
                    0
                } else if next < 0 {
                    count - 1
                } else {
                    next
                };
                SettingValue::Int(next)
            }
        }
    }

    /// What the row shows: slider bar count, or the label string id of a selector value.
    pub fn display(&self, value: &SettingValue) -> Display {
        match self {
            Self::Slider { bars, .. } => {
                Display::Bars((number(value) as f32).mul_add(*bars, 0.5).floor() as i64)
            }
            Self::Toggle { labels } => {
                Display::Label(labels[usize::from(number(value) != 0.0)].clone())
            }
            Self::Cycle { labels, .. } => Display::Label(
                labels
                    .get(number(value).max(0.0) as usize)
                    .cloned()
                    .unwrap_or_default(),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Display {
    Bars(i64),
    Label(String),
}

pub fn number(value: &SettingValue) -> f64 {
    match value {
        SettingValue::Bool(b) => f64::from(u8::from(*b)),
        SettingValue::Int(i) => *i as f64,
        SettingValue::Float(f) => *f,
    }
}

/// Retail's rule for a Game Settings row, by string id. `None` = the row is not a value
/// row or its rule is not decoded yet (it stays greyed until something binds a rule).
///
/// Labels and offsets ([code] sub_8260C258 / sub_8260D968, settings object
/// `[[0x83067060] + 28]`): 11 Camera Angle +188 (0 LOW, 1 HIGH); 12 / 13 offboard axis
/// +192 / +193 (NORMAL, INVERTED); 14 Subtitles +157, 15 Minimap +158, 16 HUD +164 (then
/// applied by sub_824AD2F0 / sub_825DA2A0), 17 Camera manual +165, 19 Vibration +163
/// (sub_8260DF40, a test rumble when switched on), 26 Transparency +195, 30..34 Skate Feed
/// +196..+200 (OFF, ON); 24 Units +168 (METRIC, IMPERIAL); 28 Play Mode +180 cycles 3
/// (sub_8260DE60, applied at once by sub_827D8450 outside an online session); 29 HOM mode
/// +184 cycles 2 (OFF, ON).
pub fn retail_rule(id: &str) -> Option<ValueRule> {
    const OFF: &str = "ID_COMMON_OFF";
    const ON: &str = "ID_COMMON_ON";
    Some(match id {
        "ID_GAMESETTINGS_SFXVOLUME"
        | "ID_GAMESETTINGS_DIALOGVOLUME"
        | "ID_GAMESETTINGS_MUSICVOLUME" => ValueRule::retail_slider(),
        "ID_GAMESETTINGS_CAMERA_ANGLE_TOGGLE" => {
            ValueRule::toggle("ID_GAMESETTINGS_CAMERA_LOW", "ID_GAMESETTINGS_CAMERA_HIGH")
        }
        "ID_GAMESETTINGS_OFFBOARD_Y_AXIS_TOGGLE" | "ID_GAMESETTINGS_OFFBOARD_X_AXIS_TOGGLE" => {
            ValueRule::toggle(
                "ID_GAMESETTINGS_AXIS_NORMAL",
                "ID_GAMESETTINGS_AXIS_INVERTED",
            )
        }
        "ID_GAMESETTINGS_SUBTITLE_TOGGLE"
        | "ID_GAMESETTINGS_MINIMAP_TOGGLE"
        | "ID_GAMESETTINGS_HUD_TOGGLE"
        | "ID_GAMESETTINGS_CAM_MAN_TOGGLE"
        | "ID_GAMESETTINGS_VIBRATION_TOGGLE"
        | "ID_GAMESETTINGS_TRANSPARENCY_TOGGLE"
        | "ID_GAMESETTINGS_SKATEFEED_BOOTFLOW"
        | "ID_GAMESETTINGS_SKATEFEED_CHYRON"
        | "ID_GAMESETTINGS_SKATEFEED_INCOMING_MESSAGE"
        | "ID_GAMESETTINGS_SKATEFEED_OUTGOING_MESSAGE"
        | "ID_GAMESETTINGS_SKATEFEED_STATUS_MESSAGE" => ValueRule::toggle(OFF, ON),
        "ID_GAMESETTINGS_UNITS_TOGGLE" => ValueRule::toggle(
            "ID_GAMESETTINGS_UNITS_METRIC",
            "ID_GAMESETTINGS_UNITS_IMPERIAL",
        ),
        // Labels: sub_825E7648 maps 0..4 (values above 4 show nothing); the rule cycles 0..2,
        // Motorized (3) and the test slot (4, our Custom) are only labelled.
        "ID_GAMESETTINGS_DIFFICULTY_OPTIONS" => ValueRule::Cycle {
            count: 3,
            labels: ["EASY", "NORMAL", "HARDCORE", "MOTORIZED", "TEST"]
                .map(|l| format!("ID_GAMESETTINGS_DIFFICULTY_{l}"))
                .to_vec(),
        },
        "ID_GAMESETTINGS_HOM_MODE_OPTIONS" => ValueRule::Cycle {
            count: 2,
            labels: vec![OFF.into(), ON.into()],
        },
        // Open: 10 SFX pack (sub_82486870 picks the next pack), 21 Auto sign-in (sub_8260E068
        // popup). 18 VSync and 20 Sixaxis have no case in the handler (Left / Right do nothing).
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use SettingValue::{Bool, Float, Int};

    fn f(v: SettingValue) -> f32 {
        number(&v) as f32
    }

    #[test]
    fn slider_steps_clamps_and_snaps_like_retail() {
        let s = ValueRule::retail_slider();
        // Ten Right presses from 0 reach exactly the max (clamped), then stay.
        let mut v = Float(0.0);
        for _ in 0..12 {
            v = s.step(&v, Direction::Right);
        }
        assert_eq!(f(v.clone()), 1.0);
        assert_eq!(s.display(&v), Display::Bars(10));
        // Left from 0.1 lands on 0 (no float residue), from 0.04 snaps to 0, from 0.12 to 0
        // (0.02 < 0.05), and 0.17 -> 0.07 stays.
        assert_eq!(f(s.step(&Float(0.1f32 as f64), Direction::Left)), 0.0);
        assert_eq!(f(s.step(&Float(0.04), Direction::Left)), 0.0);
        assert_eq!(f(s.step(&Float(0.12), Direction::Left)), 0.0);
        assert!((f(s.step(&Float(0.17), Direction::Left)) - 0.07).abs() < 1e-6);
        // A value below the snap threshold going Right: 0.0 -> 0.1; 0.75 (our 5 % steps) -> 0.85.
        assert!((f(s.step(&Float(0.75), Direction::Right)) - 0.85).abs() < 1e-6);
        assert_eq!(
            s.display(&Float(0.75)),
            Display::Bars(8),
            "floor(7.5 + 0.5)"
        );
        assert_eq!(s.display(&Float(0.74)), Display::Bars(7));
    }

    #[test]
    fn toggles_flip_on_either_direction() {
        let t = retail_rule("ID_GAMESETTINGS_SUBTITLE_TOGGLE").unwrap();
        assert_eq!(t.step(&Int(0), Direction::Left), Int(1));
        assert_eq!(t.step(&Int(1), Direction::Left), Int(0));
        assert_eq!(t.step(&Int(0), Direction::Right), Int(1));
        assert_eq!(
            t.step(&Int(7), Direction::Right),
            Int(0),
            "any non-zero counts as on"
        );
        assert_eq!(t.step(&Bool(true), Direction::Right), Bool(false));
        let cam = retail_rule("ID_GAMESETTINGS_CAMERA_ANGLE_TOGGLE").unwrap();
        assert_eq!(
            cam.display(&Int(1)),
            Display::Label("ID_GAMESETTINGS_CAMERA_HIGH".into())
        );
        assert_eq!(
            cam.display(&Int(0)),
            Display::Label("ID_GAMESETTINGS_CAMERA_LOW".into())
        );
    }

    #[test]
    fn play_mode_and_hom_wrap() {
        let p = retail_rule("ID_GAMESETTINGS_DIFFICULTY_OPTIONS").unwrap();
        assert_eq!(p.step(&Int(2), Direction::Right), Int(0));
        assert_eq!(p.step(&Int(0), Direction::Left), Int(2));
        assert_eq!(p.step(&Int(1), Direction::Right), Int(2));
        // Out-of-range engine values (Motorized 3, Custom 4) follow the same retail arithmetic.
        assert_eq!(p.step(&Int(3), Direction::Right), Int(0));
        assert_eq!(p.step(&Int(3), Direction::Left), Int(2));
        assert_eq!(
            p.display(&Int(1)),
            Display::Label("ID_GAMESETTINGS_DIFFICULTY_NORMAL".into())
        );
        assert_eq!(
            p.display(&Int(4)),
            Display::Label("ID_GAMESETTINGS_DIFFICULTY_TEST".into())
        );
        assert_eq!(p.display(&Int(5)), Display::Label(String::new()));
        let h = retail_rule("ID_GAMESETTINGS_HOM_MODE_OPTIONS").unwrap();
        assert_eq!(h.step(&Int(1), Direction::Right), Int(0));
        assert_eq!(h.step(&Int(0), Direction::Left), Int(1));
    }

    #[test]
    fn undecoded_rows_have_no_rule() {
        for id in [
            "ID_GAMESETTINGS_SFXPACK",
            "ID_GAMESETTINGS_AUTOSIGNIN_TOGGLE",
            "ID_GAMESETTINGS_VSYNC_TOGGLE",
            "ID_GAMESETTINGS_AUDIO_SETTINGS",
        ] {
            assert!(retail_rule(id).is_none(), "{id}");
        }
    }
}
