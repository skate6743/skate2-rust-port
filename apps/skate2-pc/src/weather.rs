//! PC port addition: the weather Gameplay Settings' Weather Settings picks
//! (`skate2_core::ui::gameplay_modes`), as the renderer and mixer read it. Vanilla is the
//! original's sky; every weather pass, the rain's world model and its sounds stay idle there.
pub(crate) use skate2_core::ui::gameplay_modes::Weather as Mode;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// The weather as the rows have it.
pub(crate) fn current() -> Mode {
    skate2_core::ui::gameplay_modes::weather()
}

/// Rain Volume, as a fraction of the sound effects' volume.
pub(crate) fn rain_volume() -> f32 {
    skate2_core::ui::gameplay_modes::rain_volume()
}

/// How hard it rains.
pub(crate) trait Precipitation {
    /// The rain, `0..=1`.
    fn rain(self) -> f32;
}
impl Precipitation for Mode {
    fn rain(self) -> f32 {
        match self {
            Self::Vanilla | Self::AfterRain => 0.,
            Self::Drizzle => 0.35,
            Self::Rain => 0.65,
            Self::Storm => 1.,
        }
    }
}

/// A map is drawn this frame, so the weather shows and sounds.
static VISIBLE: AtomicBool = AtomicBool::new(false);
pub(crate) fn set_visible(value: bool) {
    VISIBLE.store(value, Ordering::Relaxed);
}
pub(crate) fn visible() -> bool {
    VISIBLE.load(Ordering::Relaxed)
}

/// The cloud cover the surface, character and post-effect shaders take in place of the sun's
/// direct light: none for Vanilla.
pub(crate) fn cloud_cover() -> f32 {
    let mode = current();
    if mode == Mode::AfterRain {
        return 0.32;
    }
    (mode.rain() * 2.).min(1.)
}

/// The last rendered storm clock: the next tree draw sways with the same gust.
static RAIN_CLOCK: AtomicU32 = AtomicU32::new(0);
pub(crate) fn set_rain_clock(value: f32) {
    RAIN_CLOCK.store(value.to_bits(), Ordering::Relaxed);
}
pub(crate) fn rain_clock() -> f32 {
    f32::from_bits(RAIN_CLOCK.load(Ordering::Relaxed))
}
