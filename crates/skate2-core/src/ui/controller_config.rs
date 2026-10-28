//! PC port addition: `controller_config.json` beside `config.json` (the working directory the game
//! runs from, or `SKATE2_SAVE_ROOT`), holding each stick's settings from Control Settings' Stick
//! Settings ([`crate::stick_settings`]). It is kept apart from the profile, so a profile the player
//! loads or replaces leaves them as they are. A missing file or value is vanilla, and a value past
//! its range is kept to it.
use super::online_config::number_fields;
use crate::stick_settings::{ControllerSettings, StickSettings};
use std::{fs, io, path::PathBuf};

/// `SKATE2_SAVE_ROOT` relocates it with the saves; otherwise it sits in the working directory the
/// game runs from.
pub fn path() -> io::Result<PathBuf> {
    Ok(match std::env::var_os("SKATE2_SAVE_ROOT") {
        Some(root) => PathBuf::from(root),
        None => std::env::current_dir()?,
    }
    .join("controller_config.json"))
}

/// The file's settings; vanilla without one.
#[must_use]
pub fn load() -> ControllerSettings {
    path().and_then(fs::read_to_string).ok().map_or(ControllerSettings::VANILLA, |text| parse(&text))
}

/// # Errors
/// The file cannot be written.
pub fn save(settings: &ControllerSettings) -> io::Result<()> {
    let path = path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text(settings))
}

fn text(settings: &ControllerSettings) -> String {
    let stick = |name: &str, stick: &StickSettings| {
        format!(
            "  \"{name}_stick_multiplier\": {}.{:02},\n  \"{name}_stick_deadzone\": {},\n  \"{name}_stick_anti_deadzone\": {}",
            stick.multiplier / 100,
            stick.multiplier % 100,
            stick.deadzone,
            stick.anti_deadzone
        )
    };
    format!("{{\n{},\n{}\n}}\n", stick("left", &settings.left), stick("right", &settings.right))
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "kept to the setting's range first")]
fn parse(text: &str) -> ControllerSettings {
    let fields = number_fields(text).unwrap_or_default();
    let field = |name: &str, scale: f64, vanilla: u8| {
        fields
            .iter()
            .find(|(key, _)| key == name)
            .map_or(vanilla, |&(_, value)| (value * scale).round().clamp(0., f64::from(u8::MAX)) as u8)
    };
    let stick = |name: &str| {
        let vanilla = StickSettings::VANILLA;
        StickSettings {
            multiplier: field(&format!("{name}_stick_multiplier"), 100., vanilla.multiplier),
            deadzone: field(&format!("{name}_stick_deadzone"), 1., vanilla.deadzone),
            anti_deadzone: field(&format!("{name}_stick_anti_deadzone"), 1., vanilla.anti_deadzone),
        }
        .clamped()
    };
    ControllerSettings { left: stick("left"), right: stick("right") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_through_their_own_writer() {
        let settings = ControllerSettings {
            left: StickSettings { multiplier: 40, deadzone: 12, anti_deadzone: 25 },
            right: StickSettings { multiplier: 107, deadzone: 0, anti_deadzone: 50 },
        };
        assert_eq!(parse(&text(&settings)), settings);
        assert_eq!(parse(&text(&ControllerSettings::VANILLA)), ControllerSettings::VANILLA);
        assert!(text(&settings).contains("\"left_stick_multiplier\": 0.40,"));
        assert!(text(&settings).contains("\"right_stick_multiplier\": 1.07,"));
    }

    #[test]
    fn a_missing_or_broken_file_is_vanilla_and_values_past_their_range_are_kept_to_it() {
        assert_eq!(parse("not json"), ControllerSettings::VANILLA);
        assert_eq!(parse("{}"), ControllerSettings::VANILLA);
        let settings = parse(r#"{"left_stick_multiplier": 3, "left_stick_deadzone": -4, "right_stick_anti_deadzone": 80, "other": "x"}"#);
        assert_eq!(settings.left, StickSettings { multiplier: 110, deadzone: 0, anti_deadzone: 0 });
        assert_eq!(settings.right, StickSettings { anti_deadzone: 50, ..StickSettings::VANILLA });
        assert_eq!(parse(r#"{"right_stick_deadzone": "12"}"#), ControllerSettings::VANILLA, "a number written as text is no number");
    }
}
