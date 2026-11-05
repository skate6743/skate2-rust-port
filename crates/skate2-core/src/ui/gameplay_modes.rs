//! PC port addition: Physics Settings and Weather Settings as the game follows them at once.
//! Options publishes the rows here; the simulation, renderer and mixer read one atomic word, so
//! a mode that is off costs nothing beyond that read.
pub use skate2_formats::skater_profile::{Difficulty, PcGameplayModes, PhysicsMode, RAIN_VOLUME_STEPS, Weather};
use std::sync::atomic::{AtomicU32, Ordering};

/// The modes packed a byte each: physics, difficulty, weather, then rain volume with Puddle
/// Physics in its top bit.
static MODES: AtomicU32 = AtomicU32::new(pack(PcGameplayModes {
    physics: PhysicsMode::Skate2,
    difficulty: Difficulty::Normal,
    weather: Weather::Vanilla,
    rain_volume: RAIN_VOLUME_STEPS,
    puddle_physics: false,
}));

const fn pack(modes: PcGameplayModes) -> u32 {
    modes.physics as u32 | (modes.difficulty as u32) << 8 | (modes.weather as u32) << 16 | (modes.rain_volume as u32) << 24
        | (modes.puddle_physics as u32) << 31
}

/// The rows' values, which the game follows from the next frame.
pub fn publish(mut modes: PcGameplayModes) {
    if let Some((weather, 0)) = diagnostic_weather() {
        modes.weather = weather;
    }
    if let Some((physics, difficulty)) = diagnostic_physics() {
        modes.physics = physics;
        modes.difficulty = difficulty.unwrap_or(modes.difficulty);
    }
    if cfg!(not(test)) {
        MODES.store(pack(modes), Ordering::Relaxed);
    }
}

/// `SKATE2_DIAGNOSTIC_WEATHER=<weather>[@<tick>]` (a Weather Settings label, spaces optional):
/// that weather from boot, or picked live once that many simulation ticks have completed.
fn diagnostic_weather() -> Option<(Weather, u32)> {
    static PICK: std::sync::OnceLock<Option<(Weather, u32)>> = std::sync::OnceLock::new();
    *PICK.get_or_init(|| {
        use crate::ui::options::Labelled;
        let text = std::env::var("SKATE2_DIAGNOSTIC_WEATHER").ok()?;
        let (name, tick) = text.split_once('@').map_or((text.as_str(), Some(0)), |(name, tick)| (name, tick.parse().ok()));
        let compact = |label: &str| label.replace(' ', "").to_ascii_lowercase();
        let weather = Weather::ALL.into_iter().find(|weather| compact(weather.label()) == compact(name))?;
        Some((weather, tick?))
    })
}

/// `SKATE2_DIAGNOSTIC_PHYSICS=<physics>[:<difficulty>]` (Physics Settings labels, spaces
/// optional): those picks from boot.
fn diagnostic_physics() -> Option<(PhysicsMode, Option<Difficulty>)> {
    static PICK: std::sync::OnceLock<Option<(PhysicsMode, Option<Difficulty>)>> = std::sync::OnceLock::new();
    *PICK.get_or_init(|| {
        use crate::ui::options::Labelled;
        let text = std::env::var("SKATE2_DIAGNOSTIC_PHYSICS").ok()?;
        let compact = |label: &str| label.replace(' ', "").to_ascii_lowercase();
        let (physics, difficulty) = text.split_once(':').map_or((text.as_str(), None), |(p, d)| (p, Some(d)));
        let physics = [PhysicsMode::Skate2, PhysicsMode::Skate3].into_iter().find(|mode| compact(mode.label()) == compact(physics))?;
        let difficulty = difficulty.and_then(|name| {
            [Difficulty::Easy, Difficulty::Normal, Difficulty::Hardcore, Difficulty::Motorized]
                .into_iter()
                .find(|difficulty| compact(difficulty.label()) == compact(name))
        });
        Some((physics, difficulty))
    })
}

/// With each completed simulation tick: a live diagnostic weather pick reaching its tick.
pub fn diagnostic_tick(tick: u32) {
    if let Some((weather, at)) = diagnostic_weather()
        && at > 0
        && tick == at
    {
        publish(PcGameplayModes { weather, ..current() });
    }
}

/// The rows as last published.
#[must_use]
pub fn current() -> PcGameplayModes {
    let word = MODES.load(Ordering::Relaxed);
    let byte = |shift: u32| (word >> shift) as u8;
    PcGameplayModes {
        physics: if byte(0) == PhysicsMode::Skate3 as u8 { PhysicsMode::Skate3 } else { PhysicsMode::Skate2 },
        difficulty: match byte(8) {
            0 => Difficulty::Easy,
            2 => Difficulty::Hardcore,
            3 => Difficulty::Motorized,
            _ => Difficulty::Normal,
        },
        weather: Weather::ALL.get(usize::from(byte(16))).copied().unwrap_or_default(),
        rain_volume: (byte(24) & 0x7F).min(RAIN_VOLUME_STEPS),
        puddle_physics: word >> 31 != 0,
    }
}

/// The physics the local skater plays with: Skate 3's only while Skate 3 data is linked.
#[must_use]
pub fn physics() -> PhysicsMode {
    match current().physics {
        PhysicsMode::Skate3 if crate::skate3::is_linked() => PhysicsMode::Skate3,
        _ => PhysicsMode::Skate2,
    }
}

/// Skate 3's difficulty, which applies with Skate 3's physics.
#[must_use]
pub fn difficulty() -> Difficulty {
    current().difficulty
}

/// The sky over the city.
#[must_use]
pub fn weather() -> Weather {
    current().weather
}

/// The Skate 3 physics player's settings: all of Skate 3 at the picked difficulty, or none of it
/// (Skate 2) when Skate 2 physics is picked or no Skate 3 data is linked.
#[must_use]
pub fn skate3_settings() -> crate::ui::gameplay::GameplaySettings {
    use crate::ui::gameplay::{Difficulty as Tuning, GameplaySettings};
    if physics() != PhysicsMode::Skate3 {
        return GameplaySettings::default();
    }
    let difficulty = match difficulty() {
        Difficulty::Easy => Tuning::Easy,
        Difficulty::Normal => Tuning::Normal,
        Difficulty::Hardcore => Tuning::Hardcore,
        Difficulty::Motorized => Tuning::Motorized,
    };
    GameplaySettings { skate3: true, difficulty, airtime_slow_motion: false }
}

/// Puddle Physics is on.
#[must_use]
pub fn puddle_physics() -> bool {
    current().puddle_physics
}

/// The rain's loudness as a fraction of the sound effects' volume.
#[must_use]
pub fn rain_volume() -> f32 {
    f32::from(current().rain_volume) / f32::from(RAIN_VOLUME_STEPS)
}

impl crate::ui::options::Labelled for PhysicsMode {
    fn label(self) -> &'static str {
        match self {
            Self::Skate2 => "Skate 2",
            Self::Skate3 => "Skate 3",
        }
    }
}
impl crate::ui::options::Labelled for Difficulty {
    fn label(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hardcore => "Hardcore",
            Self::Motorized => "Motorized",
        }
    }
}
impl crate::ui::options::Labelled for Weather {
    fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Drizzle => "Drizzle",
            Self::Rain => "Rain",
            Self::Storm => "Storm",
            Self::AfterRain => "After Rain",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_pack_and_unpack() {
        let modes = PcGameplayModes { physics: PhysicsMode::Skate3, difficulty: Difficulty::Motorized, weather: Weather::Storm, rain_volume: 4, puddle_physics: true };
        let word = pack(modes);
        assert_eq!(word & 0xff, 1);
        assert_eq!(word >> 8 & 0xff, 3);
        assert_eq!(word >> 16 & 0xff, 3);
        assert_eq!(word >> 24 & 0x7f, 4);
        assert_eq!(word >> 31, 1);
    }
}
