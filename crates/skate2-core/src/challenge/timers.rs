//! `Sk8::Challenge::cChallengeTimerManager`: sim-tick timers owned by
//! activities. A countdown that reaches zero posts `cMsgTimerExpired` on the
//! next update and is removed on the one after.

use super::system::{Event, World};
use std::collections::BTreeMap;

/// `eChallengeTimerType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerKind {
    CountUp = 0,
    CountDown = 1,
}

/// `ChallengeTimer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timer {
    pub ticks: i32,
    pub kind: TimerKind,
    pub paused: bool,
    expired: bool,
    owner: Option<usize>,
}

#[derive(Debug, Default)]
pub struct TimerManager {
    next_id: i32,
    timers: BTreeMap<i32, Timer>,
    /// Online, how many ticks the local skater's own play runs ahead of the challenge's ticks
    /// (a round paced by slower players); the HUD shows a run's timers that much further on,
    /// and a countdown at the challenge's own time.
    pub display_lead: u32,
}

impl TimerManager {
    /// `AddTimer` (`0x82db3a50`): ids count up from 1; time is in seconds,
    /// stored as 60 Hz ticks (`(int)(t * 60)`).
    #[expect(clippy::cast_possible_truncation, reason = "(int)(float)")]
    pub fn add(&mut self, kind: TimerKind, seconds: f32, owner: Option<usize>) -> i32 {
        self.next_id += 1;
        let ticks = (seconds * 60.0) as i32;
        self.timers.insert(self.next_id, Timer { ticks, kind, paused: false, expired: false, owner });
        self.next_id
    }

    /// `GetTime` (`0x82da79f0`): seconds, or -1 for an unknown timer.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "tick count")]
    pub fn time(&self, id: i32) -> f32 {
        self.timers.get(&id).map_or(-1.0, |t| t.ticks as f32 * 0.016_666_668)
    }

    /// [`Self::time`] where the local skater's own play stands ([`Self::display_lead`] ticks
    /// on), for the HUD only.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "tick count")]
    pub fn display_time(&self, id: i32) -> f32 {
        let lead = if self.display_lead == 0 { 0 } else { i32::try_from(self.display_lead).unwrap_or(i32::MAX) };
        self.timers.get(&id).map_or(-1.0, |t| {
            let ticks = match (t.paused, t.kind) {
                (true, _) => t.ticks,
                (false, TimerKind::CountDown) => t.ticks.saturating_sub(lead).max(0),
                (false, TimerKind::CountUp) => t.ticks.saturating_add(lead),
            };
            ticks as f32 * 0.016_666_668
        })
    }

    /// `ResetTimer`.
    #[expect(clippy::cast_possible_truncation, reason = "(int)(float)")]
    pub fn reset(&mut self, id: i32, seconds: f32) {
        if let Some(t) = self.timers.get_mut(&id) {
            t.ticks = (seconds * 60.0) as i32;
            t.expired = false;
        }
    }

    pub fn set_paused(&mut self, id: i32, paused: bool) {
        if let Some(t) = self.timers.get_mut(&id) {
            t.paused = paused;
        }
    }

    /// `ReleaseAsset`/`RemoveTimer`.
    pub fn remove(&mut self, id: i32) {
        self.timers.remove(&id);
    }

    /// `ShutdownActivity` (`0x82dac4c8`): release every timer the activity owns.
    pub fn shutdown_activity(&mut self, activity: usize) {
        self.timers.retain(|_, t| t.owner != Some(activity));
    }
}

/// `cChallengeTimerManager::Update` (`0x82dac370`).
pub fn update(world: &mut World) {
    let mut expired = Vec::new();
    world.timers.timers.retain(|&id, timer| {
        if timer.expired {
            return false;
        }
        match timer.kind {
            TimerKind::CountDown if timer.ticks <= 0 => {
                expired.push(id);
                timer.expired = true;
            }
            TimerKind::CountUp if timer.ticks == i32::MAX => timer.expired = true,
            _ => {
                // ChallengeTimer::UpdateTicks.
                if !timer.paused {
                    match timer.kind {
                        TimerKind::CountDown => timer.ticks -= 1,
                        TimerKind::CountUp => timer.ticks += 1,
                    }
                }
            }
        }
        true
    });
    for id in expired {
        world.send(Event::TimerExpired { timer: id });
    }
}
