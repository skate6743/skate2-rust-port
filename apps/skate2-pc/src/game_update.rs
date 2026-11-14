//! The game's updates (PC port addition, `skate2-update`, `docs/release/auto-update.md`): a
//! release build looks for a newer release at startup, the main menu offers it
//! (`skate2_core::ui::update_prompt`), and once it is in place the window closes and the new
//! version starts. A player's first launch of a release lists its changes once (`changelog.txt`
//! at the repository's root, built in), as `config.json` names another release as seen.
//! `skate2-pc --update` updates from a command line, and `--server` says when a newer release
//! is out, as the dedicated server does.

use skate2_core::ui::game_config::GameConfig;
use skate2_core::ui::update_prompt::{UpdateRequest, UpdateStatus};
use skate2_update::{Status, Updater};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// The executable's name in this platform's release zip.
const EXECUTABLE: &str = if cfg!(windows) { "skate2-pc.exe" } else { "skate2-pc" };

/// The new version is in place: `play` starts it once the window has closed.
static RESTART: AtomicBool = AtomicBool::new(false);

/// This release's changes, a change a line (`skate2_core::ui::update_prompt::Changelog::parse`).
const CHANGELOG: &str = include_str!("../../../changelog.txt");

/// Lists this release's changes at any launch, to see them again.
const CHANGELOG_FLAG: &str = "--changelog";

/// A player's launch: no arguments, or only `--changelog`.
pub(crate) fn player_launch() -> bool {
    std::env::args_os().skip(1).all(|argument| argument == CHANGELOG_FLAG)
}

/// This release's changes when this launch shows them: with `--changelog`, or at a player's
/// first launch of the release, which `config.json` then records as seen (`changelog_seen`).
/// A changelog without entries shows nothing.
pub(crate) fn changelog() -> Option<skate2_core::ui::update_prompt::Changelog> {
    let version = env!("CARGO_PKG_VERSION");
    let asked = std::env::args_os().skip(1).any(|argument| argument == CHANGELOG_FLAG);
    let first = player_launch() && changelog_unseen(version);
    (asked || first)
        .then(|| {
            // SKATE2_DIAGNOSTIC_CHANGELOG=<file>: that file's changes instead of this release's.
            let text = std::env::var_os("SKATE2_DIAGNOSTIC_CHANGELOG").and_then(|path| std::fs::read_to_string(path).ok());
            skate2_core::ui::update_prompt::Changelog::parse(version, text.as_deref().unwrap_or(CHANGELOG))
        })
        .filter(|changelog| !changelog.entries.is_empty())
}

/// Records `version`'s changes as seen in `config.json`: whether they were not yet. Where the
/// file cannot take the record, none show, as they would at every launch.
fn changelog_unseen(version: &str) -> bool {
    let mut config = GameConfig::load();
    if config.changelog_seen == version {
        return false;
    }
    config.changelog_seen = version.to_owned();
    config.save().is_ok()
}

/// This build's update settings: none for a development build, or for one built without
/// `SKATE2_UPDATE_REPOSITORY`.
fn config() -> Option<skate2_update::Config> {
    skate2_update::Config::for_build(option_env!("SKATE2_UPDATE_PUBLIC_KEY"), option_env!("SKATE2_UPDATE_REPOSITORY"), env!("CARGO_PKG_VERSION"), EXECUTABLE)
}

/// The game, after deleting what an earlier update left; then the new version, when the window
/// closed for one.
///
/// # Errors
/// The game's error, or the new version not starting.
pub(crate) fn play(root: &std::path::Path) -> Result<(), String> {
    clean_up();
    crate::live_skating_client::run(root).map_err(|error| error.to_string())?;
    if RESTART.load(Ordering::Relaxed)
        && let Some(config) = config()
    {
        skate2_update::restart(&config).map_err(|error| format!("the updated game could not be started: {error}"))?;
    }
    Ok(())
}

/// `--update`: the newest release in place of this one, from a command line (a server's host);
/// false when it failed or this build does not update.
pub(crate) fn update_from_command_line() -> bool {
    skate2_update::update_from_command_line(config().as_ref(), "skate2-pc")
}

/// `--server`'s start: what an earlier update left is deleted, and the log says when a newer
/// release is out.
pub(crate) fn announce_newer_release() {
    skate2_update::announce_newer_release(config(), "skate2-pc");
}

/// Deletes what an earlier update left beside the executable, retrying for a while: the version
/// it replaced may still be closing.
fn clean_up() {
    let Some(config) = config() else { return };
    let _ = std::thread::Builder::new().name("skate2 update cleanup".into()).spawn(move || {
        for _ in 0..40 {
            if skate2_update::cleanup(&config.directory) {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    });
}

/// The updater and the version it updates.
pub(crate) struct GameUpdate {
    updater: Updater,
    current: String,
    /// The failure last logged.
    logged: Option<String>,
}

impl GameUpdate {
    /// The release build's updater, looking for a newer release.
    pub(crate) fn start() -> Option<Self> {
        let config = config()?;
        let current = config.version.to_string();
        let updater = Updater::new(config);
        updater.check();
        Some(Self { updater, current, logged: None })
    }

    pub(crate) fn current(&self) -> &str {
        &self.current
    }

    /// The updater's status for the main menu.
    pub(crate) fn status(&mut self) -> UpdateStatus {
        match self.updater.status() {
            Status::Idle | Status::Checking | Status::UpToDate => UpdateStatus::None,
            Status::Available { version } => UpdateStatus::Available { version: version.to_string() },
            Status::Downloading { version, done, total } => {
                let percent = u8::try_from(done.saturating_mul(100) / total.max(1)).unwrap_or(100).min(100);
                UpdateStatus::Downloading { version: version.to_string(), percent }
            }
            Status::Installed { version } => {
                RESTART.store(true, Ordering::Relaxed);
                UpdateStatus::Installed { version: version.to_string() }
            }
            Status::Failed(failure) => {
                if self.logged.as_ref() != Some(&failure.message) {
                    eprintln!("update: {}", failure.message);
                    self.logged = Some(failure.message.clone());
                }
                let message = if failure.folder && cfg!(windows) {
                    "The game's folder cannot be written to: move the game to a folder of your own (not Program Files) and try again."
                } else if failure.folder {
                    "The game's folder cannot be written to: move the game to a folder of your own and try again."
                } else {
                    "Check the internet connection and try again, or download the newest release from the project's GitHub page."
                };
                UpdateStatus::Failed { message: message.into() }
            }
        }
    }

    pub(crate) fn request(&self, request: UpdateRequest) {
        match request {
            UpdateRequest::Check => self.updater.check(),
            UpdateRequest::Install => self.updater.update(),
        }
    }
}
