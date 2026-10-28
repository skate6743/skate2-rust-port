//! Putting a downloaded release in place. Every file of the zip is written beside its target as
//! `<name>.new`; then each target in turn becomes `<name>.old` (Windows lets a running executable
//! be renamed, not deleted) and its `.new` takes its name. A step that fails puts every file back.
//! The `.old` files are listed in `skate2-update.cleanup` and deleted at the next start, once the
//! process that ran them has exited. The player's settings and saves are never written.

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};

/// The zip as it downloads, and the list of files a finished update left to delete.
pub(crate) const DOWNLOAD: &str = "skate2-update.zip.part";
const CLEANUP: &str = "skate2-update.cleanup";
/// The most a release's files may come to once unpacked.
const MAX_UNPACKED: u64 = 4 << 30;
/// Top-level names in the game's folder an update never replaces: the player's settings and
/// saves, the dedicated server's private word list, settings, moderation key and ban lists, and
/// the update's own files.
const PROTECTED: [&str; 11] = [
    "config.json",
    "online-config.json",
    "controller_config.json",
    "saves",
    "badwords.txt",
    "server-config.json",
    "moderation-key.txt",
    "chat-shadowbans.json",
    "ip-bans.json",
    DOWNLOAD,
    CLEANUP,
];

/// One file of the zip: where it goes, relative to the game's folder, and its unix mode.
struct Entry {
    index: usize,
    path: PathBuf,
    #[cfg(unix)]
    mode: Option<u32>,
}

/// The files of the zip at `archive` that go in `directory`, without the folder they all sit in
/// when they share one; `executable` must be among them.
fn entries(archive: &mut zip::ZipArchive<File>, executable: &str) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    let mut unpacked = 0_u64;
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(|error| format!("the update's zip: {error}"))?;
        if file.is_dir() {
            continue;
        }
        if file.is_symlink() {
            return Err(format!("the update's zip holds a link: {}", file.name()));
        }
        let path = file.enclosed_name().ok_or_else(|| format!("the update's zip holds an unsafe name: {}", file.name()))?;
        unpacked = unpacked.saturating_add(file.size());
        entries.push(Entry {
            index,
            path,
            #[cfg(unix)]
            mode: file.unix_mode(),
        });
    }
    if unpacked > MAX_UNPACKED {
        return Err("the update's zip unpacks too large".into());
    }
    let top = entries.first().and_then(|entry| entry.path.components().next()).map(|component| component.as_os_str().to_owned());
    if let Some(top) = top
        && entries.iter().all(|entry| entry.path.components().count() > 1 && entry.path.components().next().is_some_and(|component| component.as_os_str() == top))
    {
        for entry in &mut entries {
            entry.path = entry.path.components().skip(1).collect();
        }
    }
    entries.retain(|entry| {
        let first = entry.path.components().next().map(|component| component.as_os_str().to_string_lossy().to_ascii_lowercase());
        first.as_deref().is_some_and(|first| !PROTECTED.contains(&first)) && entry.path.components().all(|component| matches!(component, Component::Normal(_)))
    });
    if !entries.iter().any(|entry| entry.path == Path::new(executable)) {
        return Err(format!("the update's zip has no {executable}"));
    }
    Ok(entries)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// One target replaced: its `.old` exists when the target did.
struct Swapped {
    target: PathBuf,
    kept_old: bool,
}

/// Puts the release in the zip at `archive` into `directory`; the files it replaced, relative to
/// `directory`.
///
/// # Errors
/// What stopped it; every file is then as it was.
pub(crate) fn install(directory: &Path, archive: &Path, executable: &str) -> Result<Vec<PathBuf>, String> {
    let file = File::open(archive).map_err(|error| format!("{}: {error}", archive.display()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| format!("the update's zip: {error}"))?;
    let entries = entries(&mut archive, executable)?;
    let mut written: Vec<PathBuf> = Vec::new();
    let unpacked = (|| -> Result<(), String> {
        for entry in &entries {
            let target = directory.join(&entry.path);
            let new = with_suffix(&target, ".new");
            if let Some(parent) = new.parent() {
                fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
            }
            let mut source = archive.by_index(entry.index).map_err(|error| format!("the update's zip: {error}"))?;
            let mut out = File::create(&new).map_err(|error| format!("{}: {error}", new.display()))?;
            written.push(new.clone());
            io::copy(&mut source, &mut out).map_err(|error| format!("{}: {error}", new.display()))?;
            out.sync_all().map_err(|error| format!("{}: {error}", new.display()))?;
            #[cfg(unix)]
            set_mode(&new, entry.mode, entry.path == Path::new(executable)).map_err(|error| format!("{}: {error}", new.display()))?;
        }
        Ok(())
    })();
    if let Err(error) = unpacked {
        for new in &written {
            let _ = fs::remove_file(new);
        }
        return Err(error);
    }
    let mut swapped: Vec<Swapped> = Vec::new();
    for entry in &entries {
        let target = directory.join(&entry.path);
        let new = with_suffix(&target, ".new");
        let old = with_suffix(&target, ".old");
        let step = (|| -> io::Result<bool> {
            let kept_old = target.exists();
            if kept_old {
                match fs::remove_file(&old) {
                    Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
                    _ => {}
                }
                fs::rename(&target, &old)?;
            }
            if let Err(error) = fs::rename(&new, &target) {
                if kept_old {
                    let _ = fs::rename(&old, &target);
                }
                return Err(error);
            }
            Ok(kept_old)
        })();
        match step {
            Ok(kept_old) => swapped.push(Swapped { target, kept_old }),
            Err(error) => {
                for done in swapped.iter().rev() {
                    let _ = fs::remove_file(&done.target);
                    if done.kept_old {
                        let _ = fs::rename(with_suffix(&done.target, ".old"), &done.target);
                    }
                }
                for new in &written {
                    let _ = fs::remove_file(new);
                }
                return Err(format!("{}: {error}", target.display()));
            }
        }
    }
    let replaced: Vec<PathBuf> = entries.iter().zip(&swapped).filter(|(_, done)| done.kept_old).map(|(entry, _)| entry.path.clone()).collect();
    let list = replaced.iter().fold(String::new(), |mut list, path| {
        let _ = writeln!(list, "{}.old", path.to_string_lossy());
        list
    });
    let _ = fs::write(directory.join(CLEANUP), list);
    let _ = cleanup(directory);
    Ok(replaced)
}

/// The zip's unix mode for a file, the executable always executable.
#[cfg(unix)]
fn set_mode(path: &Path, mode: Option<u32>, executable: bool) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = mode.map_or(if executable { 0o755 } else { 0o644 }, |mode| mode & 0o777) | if executable { 0o755 } else { 0 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

/// Deletes what an earlier update left in `directory`: the files it replaced, once nothing runs
/// them, and a download that never finished. Only the files it listed are touched. Whether
/// nothing is left to delete.
#[must_use]
pub fn cleanup(directory: &Path) -> bool {
    let _ = fs::remove_file(directory.join(DOWNLOAD));
    let list = directory.join(CLEANUP);
    let Ok(text) = fs::read_to_string(&list) else { return true };
    let left: Vec<&str> = text
        .lines()
        .filter(|line| {
            let path = Path::new(line);
            let ours = path.extension().is_some_and(|extension| extension == "old") && path.components().all(|component| matches!(component, Component::Normal(_)));
            ours && match fs::remove_file(directory.join(path)) {
                Ok(()) => false,
                Err(error) => error.kind() != io::ErrorKind::NotFound,
            }
        })
        .collect();
    if left.is_empty() {
        let _ = fs::remove_file(&list);
        return true;
    }
    let _ = fs::write(&list, left.iter().fold(String::new(), |mut text, line| {
        let _ = writeln!(text, "{line}");
        text
    }));
    false
}

/// Starts the executable an update put in place, with the arguments this process was started
/// with, in its working folder. The path is the installed one: the running process's own file is
/// the `.old` the update renamed it to. On Unix it replaces this process (`exec`), so whatever
/// started the game (a shell, Steam) still has it; elsewhere it starts beside this one, which
/// then ends.
///
/// # Errors
/// The executable could not be started.
pub fn restart(config: &crate::Config) -> io::Result<()> {
    let mut command = std::process::Command::new(config.directory.join(&config.executable));
    command.args(std::env::args_os().skip(1));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        Err(command.exec())
    }
    #[cfg(not(unix))]
    {
        command.spawn().map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    struct Folder(PathBuf);

    impl Folder {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("skate2-update-test-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("test folder");
            Self(path)
        }

        fn write(&self, name: &str, text: &str) {
            let path = self.0.join(name);
            fs::create_dir_all(path.parent().expect("parent")).expect("folders");
            fs::write(path, text).expect("file");
        }

        fn read(&self, name: &str) -> Option<String> {
            fs::read_to_string(self.0.join(name)).ok()
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn zip(path: &Path, files: &[(&str, &str)]) {
        let mut writer = zip::ZipWriter::new(File::create(path).expect("zip"));
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, text) in files {
            writer.start_file(*name, options).expect("entry");
            writer.write_all(text.as_bytes()).expect("entry text");
        }
        writer.finish().expect("zip written");
    }

    #[test]
    fn an_update_replaces_the_game_and_keeps_the_player_s_files() {
        let folder = Folder::new("replace");
        folder.write("game.exe", "old game");
        folder.write("config.json", "player config");
        folder.write("saves/SKATER.P", "player save");
        folder.write("other.txt", "untouched");
        let archive = folder.0.join(DOWNLOAD);
        zip(&archive, &[
            ("release/game.exe", "new game"),
            ("release/docs/readme.txt", "readme"),
            ("release/config.json", "release config"),
            ("release/saves/SKATER.P", "release save"),
        ]);
        let replaced = install(&folder.0, &archive, "game.exe").expect("installed");
        assert_eq!(replaced, vec![PathBuf::from("game.exe")]);
        assert_eq!(folder.read("game.exe").as_deref(), Some("new game"));
        assert_eq!(folder.read("docs/readme.txt").as_deref(), Some("readme"));
        assert_eq!(folder.read("config.json").as_deref(), Some("player config"));
        assert_eq!(folder.read("saves/SKATER.P").as_deref(), Some("player save"));
        assert_eq!(folder.read("other.txt").as_deref(), Some("untouched"));
        assert_eq!(folder.read("game.exe.old"), None, "nothing runs the old file here, so it went");
        assert_eq!(folder.read(CLEANUP), None);
        assert_eq!(folder.read(DOWNLOAD), None, "the download is cleaned up with the rest");
    }

    #[test]
    fn a_zip_without_the_game_changes_nothing() {
        let folder = Folder::new("missing");
        folder.write("game.exe", "old game");
        let archive = folder.0.join("update.zip");
        zip(&archive, &[("server", "a server"), ("game.exe.txt", "not it")]);
        assert!(install(&folder.0, &archive, "game.exe").is_err());
        assert_eq!(folder.read("game.exe").as_deref(), Some("old game"));
        assert_eq!(folder.read("server"), None);
        assert_eq!(folder.read("server.new"), None);
    }

    #[test]
    fn a_file_that_cannot_be_replaced_puts_every_file_back() {
        let folder = Folder::new("rollback");
        folder.write("game.exe", "old game");
        folder.write("data.bin", "old data");
        // A folder where data.bin's .old must go: the second swap fails after the first is done.
        folder.write("data.bin.old/inside.txt", "blocks the swap");
        let archive = folder.0.join("update.zip");
        zip(&archive, &[("game.exe", "new game"), ("data.bin", "new data")]);
        assert!(install(&folder.0, &archive, "game.exe").is_err());
        assert_eq!(folder.read("game.exe").as_deref(), Some("old game"));
        assert_eq!(folder.read("data.bin").as_deref(), Some("old data"));
        assert_eq!(folder.read("data.bin.old/inside.txt").as_deref(), Some("blocks the swap"));
        for leftover in ["game.exe.new", "data.bin.new", "game.exe.old"] {
            assert_eq!(folder.read(leftover), None, "{leftover}");
        }
    }

    #[test]
    fn cleanup_deletes_only_the_files_it_listed() {
        let folder = Folder::new("cleanup");
        folder.write("game.exe.old", "replaced");
        folder.write("mine.old", "the player's");
        folder.write(CLEANUP, "game.exe.old\n../outside.old\nnot-old.txt\n");
        folder.write("not-old.txt", "kept");
        assert!(cleanup(&folder.0));
        assert_eq!(folder.read("game.exe.old"), None);
        assert_eq!(folder.read("mine.old").as_deref(), Some("the player's"));
        assert_eq!(folder.read("not-old.txt").as_deref(), Some("kept"));
        assert_eq!(folder.read(CLEANUP), None);
    }
}
