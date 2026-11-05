//! Host diagnostic output policy; independent of native gameplay state.
use std::{
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
#[path = "frame_profile.rs"]
pub mod frame_profile;
pub mod lean_trace;

static QUIET: AtomicBool = AtomicBool::new(false);
static START: OnceLock<Instant> = OnceLock::new();
static LOG_ELAPSED: OnceLock<bool> = OnceLock::new();

/// Set before starting the live client. Fatal errors use explicit std output.
pub fn set_quiet(quiet: bool) {
    START.get_or_init(Instant::now);
    QUIET.store(quiet, Ordering::Relaxed);
}

pub fn is_quiet() -> bool {
    QUIET.load(Ordering::Relaxed)
}

/// The host's start: `set_quiet`, or the first diagnostic that asks.
pub fn start() -> Instant {
    *START.get_or_init(Instant::now)
}

/// `SKATE2_LOG_ELAPSED`: each diagnostic line begins with the seconds since [`start`].
pub fn log_elapsed() -> bool {
    *LOG_ELAPSED.get_or_init(|| std::env::var_os("SKATE2_LOG_ELAPSED").is_some())
}

/// A run traced to be compared step by step with others (a lean trace,
/// `SKATE2_TRACE_SELECTED_STATES`): every load is delivered at its due update and every stream
/// reads on the thread that polls it.
pub fn traced_run() -> bool {
    static TRACED: OnceLock<bool> = OnceLock::new();
    *TRACED.get_or_init(|| lean_trace::enabled() || std::env::var_os("SKATE2_TRACE_SELECTED_STATES").is_some())
}

// Lexical macros cover descendant modules, including opt-in trace sites.
// Arguments are not evaluated or formatted when quiet.
macro_rules! eprintln {
    () => {{
        if !$crate::diagnostics::is_quiet() { std::eprintln!(); }
    }};
    ($($arg:tt)*) => {{
        if !$crate::diagnostics::is_quiet() {
            if $crate::diagnostics::log_elapsed() {
                std::eprintln!("[{:10.3}] {}", $crate::diagnostics::start().elapsed().as_secs_f64(), format_args!($($arg)*));
            } else {
                std::eprintln!($($arg)*);
            }
        }
    }};
}
macro_rules! println {
    ($($arg:tt)*) => {{
        if !$crate::diagnostics::is_quiet() { std::println!($($arg)*); }
    }};
}

pub mod read_trace;
pub mod test_log;
