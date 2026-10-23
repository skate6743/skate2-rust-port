//! PC port addition: the folder of the original game a launch plays from. `SKATE2_GAME_ROOT`
//! names it when set; otherwise `config.json` does (`skate2_core::ui::game_config`), and when
//! that names no folder holding the game's data the player picks the game's `.xex`, whose folder
//! (holding the `data` folder) is checked and kept there.

use rfd::{FileDialog, MessageButtons, MessageDialog, MessageLevel};
use skate2_core::ui::game_config::GameConfig;
use skate2_formats::mount::{MountedBig, SKATE2_ARCHIVES};
use std::io::Read as _;
use std::path::{Path, PathBuf};

/// The game folder to play from; `None` when the player closes the picker.
pub(crate) fn resolve() -> Result<Option<PathBuf>, String> {
    if let Some(root) = std::env::var_os("SKATE2_GAME_ROOT") {
        return Ok(Some(PathBuf::from(root)));
    }
    let mut config = GameConfig::load();
    let first = config.game_root.is_empty();
    let mut notice = if config.game_root.is_empty() {
        (MessageLevel::Info, "Welcome! Locate your copy of Skate 2: select its .xex file (for example default.xex). The folder it is in has to hold the game's data folder.".to_owned())
    } else {
        let root = PathBuf::from(&config.game_root);
        match check_root(&root) {
            Ok(()) => return Ok(Some(root)),
            Err(problem) => (MessageLevel::Warning, format!("Skate 2 was not found at {}: {problem}.\n\nLocate your copy of Skate 2 again: select its .xex file.", root.display())),
        }
    };
    loop {
        tell(notice.0, &notice.1);
        let Some(xex) = FileDialog::new().set_title("Locate Skate 2's .xex file").add_filter("Xbox 360 executable", &["xex"]).pick_file() else {
            // Where no dialog can open (a Linux session without a desktop portal), the folder
            // can be written into config.json by hand.
            let config = GameConfig::path().map_or_else(|_| "config.json".into(), |path| path.display().to_string());
            eprintln!("no game folder was chosen: {config} names it as \"game_root\"");
            return Ok(None);
        };
        let root = xex.parent().map(Path::to_path_buf).unwrap_or_default();
        match check_xex(&xex).and_then(|()| check_root(&root)) {
            Ok(()) => {
                config.game_root = root.to_str().ok_or("the game folder's path is not valid Unicode")?.to_owned();
                // A first launch has no update to tell of: this release's changes count as seen.
                if first && config.changelog_seen.is_empty() {
                    config.changelog_seen = env!("CARGO_PKG_VERSION").to_owned();
                }
                config.save().map_err(|error| format!("config.json could not be saved: {error}"))?;
                return Ok(Some(root));
            }
            Err(problem) => {
                notice = (MessageLevel::Warning, format!("That is not a Skate 2 game folder: {problem}.\n\nSelect the .xex file in the folder that holds the game's data folder."));
            }
        }
    }
}

/// A player launch's fatal error, which the console closing with the game would hide.
pub(crate) fn show_error(text: &str) {
    // macOS shows no message box from a running application, so the error is kept where the crash
    // reports go as well as told.
    #[cfg(target_os = "macos")]
    crate::panic_report::save_error_report(text);
    tell(MessageLevel::Error, text);
}

/// The folder holds the original game's data: every archive the game mounts, with a table that
/// opens.
fn check_root(root: &Path) -> Result<(), String> {
    for spec in &SKATE2_ARCHIVES {
        let path = spec.path_under(root);
        if !path.is_file() {
            return Err(format!("{} is missing", spec.relative_path));
        }
        MountedBig::open(&path, spec.use_external_header).map_err(|error| format!("{} cannot be read ({error})", spec.relative_path))?;
    }
    Ok(())
}

/// The file is an Xbox 360 executable.
fn check_xex(path: &Path) -> Result<(), String> {
    let mut magic = [0_u8; 4];
    std::fs::File::open(path).and_then(|mut file| file.read_exact(&mut magic)).map_err(|error| format!("the file cannot be read ({error})"))?;
    if magic == *b"XEX2" { Ok(()) } else { Err("the file is not an Xbox 360 executable".into()) }
}

fn tell(level: MessageLevel, text: &str) {
    let _ = MessageDialog::new().set_level(level).set_title("Skate 2").set_description(text).set_buttons(MessageButtons::Ok).show();
}
