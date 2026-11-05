//! PC port addition: Smokey's puddle physics, off unless Gameplay Settings' Puddle Physics is
//! picked. The renderer's splashes publish the local board's wet wheels; Skate 3 physics reads a
//! fresh sample while riding: a wet wheel keeps 15% of its grip, deeper water drags, and riding
//! fast through deep water bails. The thresholds are Smokey's, not the original game's.
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Contact {
    position: [f32; 3],
    /// Back left, back right, front left, front right.
    pub(crate) depths: [f32; 4],
    at: Instant,
}

static LATEST: Mutex<Option<Contact>> = Mutex::new(None);
static HELD: AtomicBool = AtomicBool::new(false);

/// Puddle Physics applies: picked, with rain on the ground.
#[must_use]
pub fn active() -> bool {
    use super::gameplay_modes::Weather;
    let modes = super::gameplay_modes::current();
    modes.puddle_physics && matches!(modes.weather, Weather::Drizzle | Weather::Rain | Weather::Storm | Weather::AfterRain)
}

/// The local board's place and its wheels' water depths, from the frame the renderer drew.
pub fn publish(position: [f32; 3], depths: [f32; 4]) {
    if !active() || !position.iter().chain(&depths).all(|value| value.is_finite()) {
        clear();
        return;
    }
    if let Ok(mut latest) = LATEST.lock() {
        *latest = Some(Contact { position, depths: depths.map(|depth| depth.clamp(0.0, 0.65)), at: Instant::now() });
        HELD.store(true, Ordering::Relaxed);
    }
}

/// No wet wheel under the board.
pub fn clear() {
    if HELD.swap(false, Ordering::Relaxed)
        && let Ok(mut latest) = LATEST.lock()
    {
        *latest = None;
    }
}

/// A sample under the deck at `position` from the last 0.2 s.
pub(crate) fn contact_at(position: [f32; 3]) -> Option<Contact> {
    if !HELD.load(Ordering::Relaxed) || !active() || !position.iter().all(|value| value.is_finite()) {
        return None;
    }
    let contact = LATEST.lock().ok().and_then(|latest| *latest)?;
    let apart = (position[0] - contact.position[0]).hypot(position[2] - contact.position[2]);
    if contact.at.elapsed() > Duration::from_millis(200)
        || apart > 0.8
        || position[1] > contact.position[1] + 0.12
        || position[1] < contact.position[1] - 0.8
    {
        return None;
    }
    Some(contact)
}

/// A visibly wet wheel keeps 15% of its contact friction.
pub(crate) fn grip_scale(depth: f32) -> f32 {
    if depth.is_finite() && depth > 0.003 { 0.15 } else { 1.0 }
}

/// The wet riding that leads to a bail.
#[derive(Default, Debug)]
pub(crate) struct Response {
    wet_seconds: f32,
    cooldown: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Decision {
    pub(crate) depths: [f32; 4],
    pub(crate) drag: f32,
    pub(crate) wipeout: bool,
}

impl Response {
    /// Over 0.6 m/s through water deeper than 0.14 for 0.35 s bails, then 2 s pass before
    /// another; water deeper than 0.06 drags, up to 0.8.
    pub(crate) fn step(&mut self, depths: Option<[f32; 4]>, speed: f32, dt: f32) -> Decision {
        if !dt.is_finite() || !(0.0..=0.1).contains(&dt) || !speed.is_finite() {
            *self = Self::default();
            return Decision::default();
        }
        self.cooldown = (self.cooldown - dt).max(0.0);
        let Some(depths) = depths.filter(|depths| depths.iter().all(|depth| depth.is_finite())) else {
            self.wet_seconds = 0.0;
            return Decision::default();
        };
        let depth = depths.iter().copied().fold(0.0, f32::max);
        if depth <= 0.003 {
            self.wet_seconds = 0.0;
            return Decision::default();
        }
        if speed > 0.6 && depth > 0.14 {
            self.wet_seconds += dt;
        } else {
            self.wet_seconds = 0.0;
        }
        let wipeout = self.wet_seconds >= 0.35 && self.cooldown <= 0.0;
        if wipeout {
            self.cooldown = 2.0;
            self.wet_seconds = 0.0;
        }
        Decision { depths, drag: ((depth - 0.06).max(0.0) * 2.5).min(0.8), wipeout }
    }
}
