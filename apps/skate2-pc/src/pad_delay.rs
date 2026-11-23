//! PC port addition: Control Settings' Polling & Delay (`skate2_core::stick_settings`). The game
//! polls its pads once per 60 Hz timer period at every frame rate. The original reads them on the
//! timer itself (`timerThread` 0x8298a2c4, `Sk8AppMan::TimerCallback`), so its samples are a
//! sixtieth of a second apart whenever the simulation takes them; the port's poll runs with its
//! tick, whose start follows the frames. Under the vanilla Polling Rate each poll reads a pad as it
//! was at the tick's own timer instant, and with an Input Delay as it was that many milliseconds
//! before ([`ReadTime`]). A pad's states are kept as they change, each with the time it was first
//! read: SDL's pads on their own thread, which reads them every millisecond (`crate::sdl_pads`),
//! XInput's on [`XinputHistory`]'s, which runs while either setting needs the past.
//! See `docs/evidence/input-delay-2026-11-21.md`.
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

/// How long a pad's past states are kept: well past the longest delay.
const KEPT: Duration = Duration::from_millis(250);

/// The delay of `milliseconds` from Control Settings; none at zero.
pub(crate) fn delay(milliseconds: u8) -> Option<Duration> {
    (milliseconds > 0).then(|| Duration::from_millis(u64::from(milliseconds)))
}

/// When a poll reads its pads from: Control Settings' Polling Rate and Input Delay.
#[derive(Clone, Copy, Default)]
pub(crate) struct ReadTime {
    /// The input delay, none at zero.
    pub(crate) delay: Option<Duration>,
    /// The vanilla polling rate: a poll reads the pads at its tick's timer instant.
    pub(crate) timed: bool,
    /// The timer instant of the tick that polls next.
    pub(crate) slot: Option<Instant>,
}

impl ReadTime {
    /// The settings' read time; the next tick's timer instant is kept.
    pub(crate) fn of(settings: skate2_core::stick_settings::ControllerSettings, slot: Option<Instant>) -> Self {
        Self { delay: delay(settings.input_delay), timed: settings.polling == skate2_core::stick_settings::Polling::Vanilla, slot }
    }

    /// Whether a poll reads the past, which the pads' histories then keep.
    pub(crate) const fn past(&self) -> bool {
        self.timed || self.delay.is_some()
    }

    /// The moment a poll at `now` reads the pads as of; none reads them as they are.
    pub(crate) fn instant(&self, now: Instant) -> Option<Instant> {
        if !self.past() {
            return None;
        }
        let read = if self.timed { self.slot.map_or(now, |slot| slot.min(now)) } else { now };
        Some(self.delay.and_then(|delay| read.checked_sub(delay)).unwrap_or(read))
    }
}

/// A pad's states over the last [`KEPT`], each with the time it was first read.
pub(crate) struct History<T> {
    states: VecDeque<(Instant, T)>,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self { states: VecDeque::new() }
    }
}

impl<T: Copy + PartialEq> History<T> {
    /// The pad read as `state` at `now`: kept when it differs from the last one kept.
    pub(crate) fn record(&mut self, now: Instant, state: T) {
        if self.states.back().is_none_or(|(_, last)| *last != state) {
            self.states.push_back((now, state));
        }
        // The state in force when the kept span began stays, to answer for its start.
        while self.states.len() > 1 && now.saturating_duration_since(self.states[1].0) > KEPT {
            self.states.pop_front();
        }
    }

    /// The pad's state `delay` before `now`: the last one read by then, else the oldest kept (a
    /// pad connected more recently than that). None before the pad was read at all.
    #[cfg(test)]
    pub(crate) fn at(&self, now: Instant, delay: Duration) -> Option<T> {
        match now.checked_sub(delay) {
            Some(then) => self.as_of(then),
            None => self.states.front().map(|(_, state)| *state),
        }
    }

    /// The pad's state at `then`: the last one read by then, else the oldest kept. None before
    /// the pad was read at all.
    pub(crate) fn as_of(&self, then: Instant) -> Option<T> {
        self.states.iter().rev().find(|(at, _)| *at <= then).or_else(|| self.states.front()).map(|(_, state)| *state)
    }

    /// The pad went away: its states with it.
    #[cfg(any(windows, test))]
    pub(crate) fn clear(&mut self) {
        self.states.clear();
    }
}

#[cfg(windows)]
pub(crate) use xinput_history::XinputHistory;

#[cfg(windows)]
mod xinput_history {
    use super::History;
    use skate2_core::XboxPadSample;
    use std::{
        sync::{
            Arc, Mutex, PoisonError,
            atomic::{AtomicBool, Ordering},
        },
        thread::JoinHandle,
        time::{Duration, Instant},
    };

    /// How often the thread reads the pads: a millisecond, the delay's own unit.
    const POLL: Duration = Duration::from_millis(1);
    /// How long a slot XInput found empty goes without another call, which looks through the
    /// system's devices (`WindowsPadPlatform::poll_xinput`).
    const EMPTY_SLOT_RECHECK: Duration = Duration::from_secs(1);

    type Slots = [History<(u32, XboxPadSample)>; 4];

    /// The XInput slots' states over the last moments, read on a thread of its own while a poll
    /// reads the past; the thread ends with its owner.
    pub(crate) struct XinputHistory {
        slots: Arc<Mutex<Slots>>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl XinputHistory {
        pub(crate) fn start() -> Self {
            let slots = Arc::<Mutex<Slots>>::default();
            let stop = Arc::new(AtomicBool::new(false));
            let thread = {
                let (slots, stop) = (Arc::clone(&slots), Arc::clone(&stop));
                std::thread::Builder::new().name("sk8 pad delay".to_owned()).spawn(move || run(&slots, &stop))
            };
            if let Err(error) = &thread {
                eprintln!("input delay thread did not start: {error}");
            }
            Self { slots, stop, thread: thread.ok() }
        }

        /// A slot's packet number and sample at `then`; none before the thread read the slot's
        /// pad.
        pub(crate) fn as_of(&self, slot: usize, then: Instant) -> Option<(u32, XboxPadSample)> {
            self.slots.lock().unwrap_or_else(PoisonError::into_inner).get(slot)?.as_of(then)
        }
    }

    impl Drop for XinputHistory {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn run(slots: &Mutex<Slots>, stop: &AtomicBool) {
        let mut empty_since: [Option<Instant>; 4] = [None; 4];
        while !stop.load(Ordering::Relaxed) {
            for (index, empty) in (0u32..).zip(&mut empty_since) {
                let now = Instant::now();
                if empty.is_some_and(|since| now.saturating_duration_since(since) < EMPTY_SLOT_RECHECK) {
                    continue;
                }
                let state = xinput::get_state(index);
                let mut slots = slots.lock().unwrap_or_else(PoisonError::into_inner);
                let Some(history) = usize::try_from(index).ok().and_then(|slot| slots.get_mut(slot)) else { continue };
                match state {
                    Ok(state) => {
                        *empty = None;
                        let pad = state.gamepad;
                        let sample = XboxPadSample {
                            buttons: pad.buttons.into(),
                            left_trigger: pad.left_trigger,
                            right_trigger: pad.right_trigger,
                            left_x: pad.left_thumb_x,
                            left_y: pad.left_thumb_y,
                            right_x: pad.right_thumb_x,
                            right_y: pad.right_thumb_y,
                        };
                        history.record(now, (state.packet_number, sample));
                    }
                    Err(_) => {
                        *empty = Some(now);
                        history.clear();
                    }
                }
            }
            std::thread::sleep(POLL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_delay_at_zero_and_its_milliseconds_past_it() {
        assert_eq!(delay(0), None);
        assert_eq!(delay(30), Some(Duration::from_millis(30)));
    }

    #[test]
    fn a_poll_reads_at_its_timer_instant_less_the_delay() {
        let start = Instant::now();
        let at = |milliseconds: u64| start + Duration::from_millis(milliseconds);
        // Frame timed without a delay: the pads as they are.
        assert!(ReadTime::default().instant(at(50)).is_none());
        // Frame timed with a delay: the delay before the poll.
        let delayed = ReadTime { delay: Some(Duration::from_millis(10)), ..ReadTime::default() };
        assert_eq!(delayed.instant(at(50)), Some(at(40)));
        // The vanilla rate: the tick's timer instant, never the future, and now before any tick.
        let timed = ReadTime { timed: true, slot: Some(at(44)), ..ReadTime::default() };
        assert_eq!(timed.instant(at(50)), Some(at(44)));
        assert_eq!(ReadTime { slot: Some(at(60)), ..timed }.instant(at(50)), Some(at(50)));
        assert_eq!(ReadTime { slot: None, ..timed }.instant(at(50)), Some(at(50)));
        // Both: the delay before the timer instant.
        assert_eq!(ReadTime { delay: Some(Duration::from_millis(30)), ..timed }.instant(at(50)), Some(at(14)));
    }

    #[test]
    fn a_pad_reads_as_it_was_the_delay_ago() {
        let start = Instant::now();
        let at = |milliseconds: u64| start + Duration::from_millis(milliseconds);
        let mut history = History::default();
        assert_eq!(history.at(at(0), Duration::from_millis(10)), None, "never read");
        // Read every millisecond; it changes at 5 and at 20.
        for millisecond in 0..40 {
            history.record(at(millisecond), if millisecond < 5 { 'a' } else if millisecond < 20 { 'b' } else { 'c' });
        }
        let delay = Duration::from_millis(10);
        assert_eq!(history.at(at(14), delay), Some('a'), "at 4 it still read a");
        assert_eq!(history.at(at(15), delay), Some('b'), "the change at 5 arrives 10 later");
        assert_eq!(history.at(at(29), delay), Some('b'));
        assert_eq!(history.at(at(30), delay), Some('c'));
        assert_eq!(history.at(at(39), Duration::ZERO), Some('c'));
        // A pad read for less long than the delay answers with its first state.
        let mut young = History::default();
        young.record(at(100), 'x');
        assert_eq!(young.at(at(103), delay), Some('x'));
        young.clear();
        assert_eq!(young.at(at(103), delay), None);
    }

    #[test]
    fn states_past_the_kept_span_go_but_the_one_in_force_at_its_start() {
        let start = Instant::now();
        let at = |milliseconds: u64| start + Duration::from_millis(milliseconds);
        let mut history = History::default();
        for (millisecond, state) in [(0, 1), (10, 2), (20, 3)] {
            history.record(at(millisecond), state);
        }
        // Unchanged for two seconds: the last state is all that is kept, and it answers.
        for millisecond in (30..2000).step_by(7) {
            history.record(at(millisecond), 3);
        }
        assert_eq!(history.states.len(), 1);
        assert_eq!(history.at(at(2000), Duration::from_millis(30)), Some(3));
        history.record(at(2000), 4);
        assert_eq!(history.at(at(2010), Duration::from_millis(30)), Some(3));
        assert_eq!(history.at(at(2030), Duration::from_millis(30)), Some(4));
    }
}
