//! Skate 3 assets read from the player's linked Skate 3 data (PC port). The reserved root
//! `ROOT` names the linked data; any other root keeps ordinary file I/O.
//!
//! Under `ROOT`, `private/stock/<path>` is the linked game's `<path>` (its archives and loose
//! files, through the host's provider), `private/game.json` is the stock graphs' manifest, and
//! the converted vault and physics skeletons are made from the linked bytes by their loaders
//! (`collections::Collections::load`, `physics_skeleton::PhysicsSkeleton::load`).
use std::{
    collections::HashMap,
    io,
    path::{Component, Path},
    sync::{Arc, Mutex, PoisonError, RwLock},
};

pub const ROOT: &str = "__skate3_linked__/assets";

/// The stock graphs the physics player runs, as the old asset pipeline's `game.json` named them.
const GAME_JSON: &str = r#"{"version":1,"action_graph":"private/stock/data/state/ActionGraph_OnBoard.stategraph","motion_graph":"private/stock/data/state/MotionGraph_OnBoard.stategraph"}"#;

/// Reads one linked file by its game path (`data/anim/OnBoard.abin`).
pub type Provider = dyn Fn(&str) -> io::Result<Vec<u8>> + Send + Sync;

struct Linked {
    provider: Arc<Provider>,
    /// Files read so far: several loaders read the same banks.
    cache: Mutex<HashMap<String, Arc<[u8]>>>,
}

static LINKED: RwLock<Option<Linked>> = RwLock::new(None);

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && !key.contains(['\\', ':', '\0'])
        && key
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// Reads under `ROOT` through `provider` from now on, with nothing read yet.
pub fn link(provider: Arc<Provider>) -> &'static Path {
    *LINKED.write().unwrap_or_else(PoisonError::into_inner) = Some(Linked {
        provider,
        cache: Mutex::new(HashMap::new()),
    });
    Path::new(ROOT)
}

/// Drops the provider and every file read through it.
pub fn unlink() {
    *LINKED.write().unwrap_or_else(PoisonError::into_inner) = None;
}

/// Drops the files read so far, keeping the provider (a mode that no longer needs them).
pub fn release_cache() {
    if let Some(linked) = LINKED.read().unwrap_or_else(PoisonError::into_inner).as_ref() {
        linked.cache.lock().unwrap_or_else(PoisonError::into_inner).clear();
    }
}

pub fn is_linked_root(root: &Path) -> bool {
    root == Path::new(ROOT)
}

fn linked_key(path: &Path) -> io::Result<Option<String>> {
    let Ok(relative) = path.strip_prefix(ROOT) else {
        return Ok(None);
    };
    let mut parts = Vec::new();
    for part in relative.components() {
        let Component::Normal(part) = part else {
            return Err(invalid("Invalid linked asset path"));
        };
        parts.push(
            part.to_str()
                .ok_or_else(|| invalid("Non-UTF-8 linked asset path"))?,
        );
    }
    let key = parts.join("/");
    if !valid_key(&key) {
        return Err(invalid("Invalid linked asset key"));
    }
    Ok(Some(key))
}

fn not_found(key: &str) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("Linked Skate 3 asset missing: {key}"))
}

/// One file under `ROOT` by its key, shared with every other reader of it.
fn linked(key: &str) -> io::Result<Arc<[u8]>> {
    if key == "private/game.json" {
        return Ok(Arc::from(GAME_JSON.as_bytes()));
    }
    let game = key.strip_prefix("private/stock/").ok_or_else(|| not_found(key))?;
    let guard = LINKED.read().unwrap_or_else(PoisonError::into_inner);
    let linked = guard.as_ref().ok_or_else(|| not_found(key))?;
    if let Some(bytes) = linked.cache.lock().unwrap_or_else(PoisonError::into_inner).get(game) {
        return Ok(Arc::clone(bytes));
    }
    let bytes: Arc<[u8]> = Arc::from((linked.provider)(game)?);
    linked
        .cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(game.to_owned(), Arc::clone(&bytes));
    Ok(bytes)
}

/// A file, shared without a copy when it is a linked one.
pub fn read_shared(path: impl AsRef<Path>) -> io::Result<Arc<[u8]>> {
    let path = path.as_ref();
    match linked_key(path)? {
        Some(key) => linked(&key),
        None => std::fs::read(path).map(Arc::from),
    }
}

pub fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let path = path.as_ref();
    match linked_key(path)? {
        Some(key) => linked(&key).map(|bytes| bytes.to_vec()),
        None => std::fs::read(path),
    }
}

pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    String::from_utf8(read(path)?).map_err(|error| invalid(error.to_string()))
}

pub fn is_file(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    match linked_key(path) {
        Ok(Some(key)) => linked(&key).is_ok(),
        Ok(None) => path.is_file(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_root_reads_stock_files_through_the_provider_once() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let root = link(Arc::new(move |path: &str| {
            counted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            match path {
                "data/config/input.cfg" => Ok(b"input".to_vec()),
                _ => Err(io::Error::new(io::ErrorKind::NotFound, path.to_owned())),
            }
        }));
        let file = root.join("private/stock/data/config/input.cfg");
        assert_eq!(read(&file).unwrap(), b"input");
        assert_eq!(&*read_shared(&file).unwrap(), b"input");
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert!(is_file(root.join("private/game.json")));
        assert!(!is_file(root.join("private/stock/data/absent.bin")));
        assert!(read(root.join("private/other.txt")).is_err());
        assert!(read(root.join("private/../a")).is_err());
        unlink();
        assert!(read(&file).is_err());
    }
}
