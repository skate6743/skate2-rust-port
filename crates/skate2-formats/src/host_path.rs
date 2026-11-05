//! The original game's files on the host's file system (PC port addition). The Xbox 360's file
//! system ignores case, so the game's data names a file in any case: a script's movie name, an
//! audio stream's path. Windows ignores case as well; on Linux a name finds its file by ignoring
//! ASCII case. A game folder can also be a disc image (`crate::disc`): a path through one is read
//! from the image in place.

use crate::disc::{self, DiscFile, Found};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

/// `path` as the file system spells it. When `path` is missing on a system whose names keep case
/// apart (Linux), each missing component becomes its directory's entry with the same name
/// ignoring ASCII case. `path` itself when it exists, on Windows, when a component matches
/// nothing, or when it runs through a disc image (whose names ignore case themselves).
#[must_use]
pub fn on_disk(path: &Path) -> PathBuf {
    if cfg!(windows) || path.exists() || disc::find(path).is_some() {
        return path.to_owned();
    }
    let mut found = PathBuf::new();
    for component in path.components() {
        let exact = found.join(component);
        if exact.exists() {
            found = exact;
            continue;
        }
        let Component::Normal(name) = component else { return path.to_owned() };
        let directory = if found.as_os_str().is_empty() { Path::new(".") } else { found.as_path() };
        let Some(entry) = entry_ignoring_case(directory, name) else { return path.to_owned() };
        found.push(entry);
    }
    found
}

/// The bytes of the file at `path` ([`on_disk`]), or none when it is missing, a missing folder on
/// the way included: a debug build's game folder has loose configuration (`data\startup.ini`,
/// `data\juice`) that a retail game's does not.
///
/// # Errors
/// The file is there but could not be read.
pub fn read_if_present(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    match read(&on_disk(path)) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Whether `path` names a file, on the host or in a disc image.
#[must_use]
pub fn is_file(path: &Path) -> bool {
    match disc::find(path) {
        Some(found) => matches!(found, Found::File { .. }),
        None => path.is_file(),
    }
}

/// Whether `path` names a folder, on the host or in a disc image (an image itself is one).
#[must_use]
pub fn is_dir(path: &Path) -> bool {
    match disc::find(path) {
        Some(found) => found == Found::Directory,
        None => path.is_dir(),
    }
}

/// A game file opened for reading: a host file or one in a disc image.
#[derive(Debug)]
pub enum GameFile {
    Host(File),
    Disc(DiscFile),
}

impl Read for GameFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Host(file) => file.read(buf),
            Self::Disc(file) => file.read(buf),
        }
    }
}

impl Seek for GameFile {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        match self {
            Self::Host(file) => file.seek(to),
            Self::Disc(file) => file.seek(to),
        }
    }
}

/// Opens the file at `path` for reading, on the host or in a disc image.
///
/// # Errors
/// The file is missing or cannot be opened.
pub fn open(path: &Path) -> io::Result<GameFile> {
    match disc::open(path) {
        Some(file) => file.map(GameFile::Disc),
        None => File::open(path).map(GameFile::Host),
    }
}

/// The bytes of the file at `path`, on the host or in a disc image.
///
/// # Errors
/// The file is missing or cannot be read.
pub fn read(path: &Path) -> io::Result<Vec<u8>> {
    match disc::open(path) {
        Some(file) => {
            let mut file = file?;
            let mut bytes = Vec::with_capacity(usize::try_from(file.len()).unwrap_or(0));
            file.read_to_end(&mut bytes)?;
            Ok(bytes)
        }
        None => std::fs::read(path),
    }
}

/// The size of the file at `path`, on the host or in a disc image.
///
/// # Errors
/// The file is missing.
pub fn file_len(path: &Path) -> io::Result<u64> {
    match disc::find(path) {
        Some(Found::File { size, .. }) => Ok(size),
        Some(_) => Err(io::Error::new(io::ErrorKind::NotFound, format!("{} is not a file in the disc image", path.display()))),
        None => std::fs::metadata(path).map(|metadata| metadata.len()),
    }
}

/// The entries of the folder `path` names, on the host or in a disc image: each one's path and
/// whether it is a folder.
///
/// # Errors
/// The folder is missing or cannot be read.
pub fn read_dir(path: &Path) -> io::Result<Vec<(PathBuf, bool)>> {
    if let Some(entries) = disc::entries(path) {
        return entries.map(|entries| entries.into_iter().map(|(name, folder)| (path.join(name), folder)).collect());
    }
    std::fs::read_dir(path)?
        .map(|entry| {
            let entry = entry?;
            let folder = entry.file_type()?.is_dir();
            Ok((entry.path(), folder))
        })
        .collect()
}

/// The entry of `directory` named `name` ignoring ASCII case; the first in byte order when
/// several are.
fn entry_ignoring_case(directory: &Path, name: &OsStr) -> Option<OsString> {
    let name = name.to_str()?;
    std::fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .filter(|entry| entry.to_str().is_some_and(|entry| entry.eq_ignore_ascii_case(name)))
        .min()
}

#[cfg(test)]
mod tests {
    use super::{on_disk, read_if_present};
    use std::fs;

    #[test]
    fn a_name_in_another_case_finds_its_file() {
        let root = std::env::temp_dir().join(format!("skate2-host-path-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Data").join("Audio")).expect("folders");
        fs::write(root.join("Data").join("Audio").join("MixMapSK8.mxb"), b"mix").expect("file");
        let named = root.join("data").join("audio").join("mixmapsk8.MXB");
        assert_eq!(fs::read(on_disk(&named)).ok().as_deref(), Some(&b"mix"[..]));
        let exact = root.join("Data").join("Audio").join("MixMapSK8.mxb");
        assert_eq!(on_disk(&exact), exact);
        let missing = root.join("data").join("audio").join("missing.mxb");
        assert_eq!(on_disk(&missing), missing);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_file_or_folder_reads_as_none() {
        let root = std::env::temp_dir().join(format!("skate2-host-path-optional-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("folder");
        fs::write(root.join("data").join("startup.ini"), b"[Config]").expect("file");
        let present = read_if_present(&root.join("data").join("startup.ini")).expect("read");
        assert_eq!(present.as_deref(), Some(&b"[Config]"[..]));
        assert_eq!(read_if_present(&root.join("data").join("debugfe.ini")).expect("missing file"), None);
        let in_missing_folder = root.join("data").join("juice").join("Juice_m.ini");
        assert_eq!(read_if_present(&in_missing_folder).expect("missing folder"), None);
        let _ = fs::remove_dir_all(&root);
    }
}
