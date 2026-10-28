//! PC fatal-panic diagnostics. Capture before unwinding poisons the SIM mutex;
//! never clear that poison or attempt to continue the interrupted game state.

use std::{
    backtrace::Backtrace,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

static SIM_PANIC: OnceLock<String> = OnceLock::new();
static REPORT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static GPU: OnceLock<String> = OnceLock::new();

/// The adapter the renderer chose, named in every later report.
pub(crate) fn set_gpu(adapter: String) {
    let _ = GPU.set(adapter);
}

pub(crate) fn gpu() -> Option<&'static str> {
    GPU.get().map(String::as_str)
}

/// Install once at process startup, before any worker can panic. Resolving the
/// save directory here avoids consulting profile/save locks inside the hook.
pub(crate) fn install() {
    let directory = report_directory();
    let executable = std::env::current_exe().ok();
    // A player's launch (main.rs), who sees no console.
    let player = crate::game_update::player_launch();
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("<unnamed>");
        // The game stops with the SIM or window thread: so do the pads' motors, which keep
        // their last command after the process.
        if matches!(thread_name, "sk8 SIM" | "main") {
            crate::sdl_pads::silence();
            #[cfg(windows)]
            crate::windows_pad_platform::silence_xinput();
        }
        let reason = info.to_string();
        let report = format!(
            "Skate 2 PC panic\nVersion: {}\nExecutable: {}\nGPU: {}\nThread: {thread_name} ({:?})\n\n{reason}\n\nBacktrace (captured automatically):\n{}\n",
            env!("CARGO_PKG_VERSION"),
            executable
                .as_deref()
                .map_or_else(|| "<unknown>".into(), |path| path.display().to_string()),
            gpu().unwrap_or("<not chosen yet>"),
            thread.id(),
            Backtrace::force_capture(),
        );
        let saved = write_report(directory.as_deref(), &report);
        let destination = match &saved {
            Ok(path) => format!("crash report: {}", path.display()),
            Err(error) => format!("could not save crash report: {error}; see console backtrace"),
        };
        if thread_name == "sk8 SIM" {
            // This hook runs before the slot guard is unwound, so a window
            // thread that observes poison also sees the original failure.
            let _ = SIM_PANIC.set(format!("SIM thread panicked: {reason}\n{destination}"));
        }
        {
            // Best effort and independent of --quiet. Ignore a closed stderr
            // rather than causing another panic from the reporting code.
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "{destination}");
            if saved.is_err() {
                let _ = writeln!(stderr, "{report}");
            }
        }
        previous_hook(info);
        // The window thread's panic ends the game; without this the window just closes. The
        // SIM thread's reaches the player through the window thread's error instead.
        if player && thread_name == "main" {
            crate::game_root::show_error(&format!(
                "Skate 2 stopped because of an error:\n\n{reason}\n\nGPU: {}\n{destination}",
                gpu().unwrap_or("<not chosen yet>")
            ));
        }
    }));
}

/// The folder the reports go to: `crash_reports` beside the game's saves.
fn report_directory() -> Option<PathBuf> {
    skate2_core::edit_skater_profile::EditSkaterProfileStore::default_path()
        .ok()
        .and_then(|profile| profile.parent().map(|parent| parent.join("crash_reports")))
        .and_then(|path| {
            if path.is_absolute() {
                Some(path)
            } else {
                std::env::current_dir().ok().map(|root| root.join(path))
            }
        })
}

/// A fatal error that is not a panic, written where the crash reports go. macOS has no message box
/// to show it in from a running application, so the file is all a player's launch can leave.
#[cfg(target_os = "macos")]
pub(crate) fn save_error_report(text: &str) {
    let report = format!(
        "Skate 2 PC error\nVersion: {}\nExecutable: {}\nGPU: {}\n\n{text}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::current_exe()
            .as_deref()
            .map_or_else(|_| "<unknown>".into(), |path| path.display().to_string()),
        gpu().unwrap_or("<not chosen yet>"),
    );
    if let Err(error) = write_report(report_directory().as_deref(), &report) {
        eprintln!("could not save the error report: {error}");
    }
}

fn write_report(directory: Option<&Path>, report: &str) -> io::Result<PathBuf> {
    let directory = directory.ok_or_else(|| io::Error::other("save directory unavailable"))?;
    fs::create_dir_all(directory)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = REPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = directory.join(format!(
        "panic-{timestamp}-{}-{sequence}.txt",
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(report.as_bytes())?;
    file.sync_all()?;
    Ok(path)
}

/// Used at every SIM slot lock/wait failure, retaining the panic reason instead
/// of replacing it with the generic poisoned-lock message.
pub(crate) fn sim_error() -> String {
    SIM_PANIC
        .get()
        .cloned()
        .unwrap_or_else(|| "SIM state lock poisoned; panic details unavailable".into())
}
