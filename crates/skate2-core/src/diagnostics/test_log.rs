//! PC Skate 3 physics trace lines (`SKATE2_TRACE_SKATE3=1`): the Skate 3 player's stages and
//! events on stderr. Nothing is recorded otherwise; every call returns after one cached flag.
use std::{fmt, sync::OnceLock, time::Instant};

#[must_use]
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("SKATE2_TRACE_SKATE3").is_ok_and(|value| value == "1"))
}

pub fn event(kind: &str, detail: fmt::Arguments<'_>) {
    if enabled() {
        eprintln!("skate3 {kind}: {detail}");
    }
}

pub fn anomaly(kind: &'static str, detail: &str) {
    if enabled() {
        eprintln!("skate3 anomaly {kind}: {detail}");
    }
}

/// A stage; nothing is measured.
pub struct StageGuard;

#[must_use]
pub fn span(_stage: &'static str) -> StageGuard {
    StageGuard
}

/// A stage whose duration is traced when it ends.
pub struct TimedStageGuard(Option<(&'static str, Instant)>);

#[must_use]
pub fn timed_span(stage: &'static str) -> TimedStageGuard {
    TimedStageGuard(enabled().then(|| (stage, Instant::now())))
}

impl Drop for TimedStageGuard {
    fn drop(&mut self) {
        if let Some((stage, start)) = self.0 {
            eprintln!("skate3 stage {stage}: {:.3} ms", start.elapsed().as_secs_f64() * 1000.);
        }
    }
}
