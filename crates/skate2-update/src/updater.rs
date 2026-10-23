//! The update check and the update, each on its own thread so that the game never waits: the
//! newest release's manifest and signature, then on request its zip for this platform, checked
//! against the signed size and SHA-256 and put in place ([`crate::install`]).

use crate::fetch::Client;
use crate::install::{DOWNLOAD, install};
use crate::manifest::{self, Asset, Manifest, Platform, Version};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

/// What a release build knows to update itself: where its releases are served (GitHub), its
/// repository there and the public key that signs its releases' manifests
/// (`apps/build_secret.rs`), its version and platform, its folder and the file name its executable
/// has in a release's zip. Only the signature is trusted, never the address.
#[derive(Clone, Debug)]
pub struct Config {
    pub server: String,
    pub repository: String,
    pub public_key: [u8; 32],
    pub version: Version,
    pub platform: Platform,
    pub directory: PathBuf,
    pub executable: String,
}

impl Config {
    /// The update settings of this build: none for a development build, which has no update key,
    /// or for a build that names no repository (`owner/name`).
    #[must_use]
    pub fn for_build(public_key: Option<&str>, repository: Option<&str>, version: &str, executable: &str) -> Option<Self> {
        let public_key = manifest::unhex(public_key?.trim()).and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())?;
        let repository = repository?.trim();
        let mut parts = repository.split('/');
        let valid = |part: Option<&str>| part.is_some_and(manifest::valid_name);
        if !(valid(parts.next()) && valid(parts.next()) && parts.next().is_none()) {
            return None;
        }
        let directory = std::env::current_exe().ok()?.parent()?.to_path_buf();
        Some(Self {
            server: "https://github.com".into(),
            repository: repository.to_owned(),
            public_key,
            version: Version::parse(version)?,
            platform: Platform::current()?,
            directory,
            executable: executable.to_owned(),
        })
    }

    fn latest_manifest_url(&self) -> String {
        format!("{}/{}/releases/latest/download/{}", self.server, self.repository, manifest::MANIFEST)
    }

    fn release_url(&self, version: Version, name: &str) -> String {
        format!("{}/{}/releases/download/v{version}/{name}", self.server, self.repository)
    }

    fn client(&self) -> Client {
        Client::new(&format!("skate2-recomp/{}", self.version))
    }

    /// The newest release's signed manifest.
    pub(crate) fn latest(&self) -> Result<Manifest, String> {
        let client = self.client();
        let bytes = client.small(&self.latest_manifest_url(), 64 * 1024)?;
        let text = String::from_utf8(bytes).map_err(|_| "the release manifest is not text".to_owned())?;
        // The signature of the release the manifest names: the newest release may change between
        // the two downloads, and a manifest only matches its own release's signature.
        let named = Manifest::parse(&text)?;
        let signature = client.small(&self.release_url(named.version, manifest::SIGNATURE), 1024)?;
        let signature = String::from_utf8(signature).map_err(|_| "the release signature is not text".to_owned())?;
        if !manifest::verify(&self.public_key, text.as_bytes(), &signature) {
            return Err("the release manifest's signature does not match this build's update key".into());
        }
        Ok(named)
    }

    /// The release's zip for this platform: downloaded, checked and put in place.
    pub(crate) fn update(&self, version: Version, asset: &Asset, progress: &mut dyn FnMut(u64)) -> Result<(), Failure> {
        let probe = self.directory.join("skate2-update.probe");
        std::fs::write(&probe, b"").map_err(|error| Failure { message: format!("{} cannot be written to: {error}", self.directory.display()), folder: true })?;
        let _ = std::fs::remove_file(&probe);
        let path = self.directory.join(DOWNLOAD);
        let downloaded = self.client().download(&self.release_url(version, &asset.name), &path, asset.size, progress);
        let installed = downloaded.and_then(|sha256| {
            if sha256 == asset.sha256 { install(&self.directory, &path, &self.executable).map(drop) } else { Err(format!("{} does not match its release's SHA-256", asset.name)) }
        });
        let _ = std::fs::remove_file(&path);
        installed.map_err(Failure::from)
    }
}

/// Why the check or the update stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub message: String,
    /// The folder the game is in cannot be written to (as under Program Files): the player can
    /// move the game.
    pub folder: bool,
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self { message, folder: false }
    }
}

/// Where the update stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// No check yet.
    Idle,
    Checking,
    /// This build is the newest release's, or newer.
    UpToDate,
    /// A newer release has a zip for this platform.
    Available { version: Version },
    Downloading { version: Version, done: u64, total: u64 },
    /// The release is in place; it runs once the game starts again.
    Installed { version: Version },
    /// The check or the update failed; the game is as it was.
    Failed(Failure),
}

struct Shared {
    status: Status,
    /// The newer release and its zip for this platform, once a check found one.
    release: Option<(Version, Asset)>,
    busy: bool,
}

/// The update's thread and what it found.
pub struct Updater {
    config: Arc<Config>,
    shared: Arc<Mutex<Shared>>,
}

impl Updater {
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self { config: Arc::new(config), shared: Arc::new(Mutex::new(Shared { status: Status::Idle, release: None, busy: false })) }
    }

    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    #[must_use]
    pub fn status(&self) -> Status {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner).status.clone()
    }

    /// Asks GitHub for the newest release, unless the updater is busy.
    pub fn check(&self) {
        let Some(shared) = self.start(Status::Checking) else { return };
        let config = Arc::clone(&self.config);
        std::thread::Builder::new()
            .name("skate2 update check".into())
            .spawn(move || {
                let found = config.latest().map(|manifest| {
                    manifest.asset(config.platform).filter(|_| manifest.version > config.version).map(|asset| (manifest.version, asset.clone()))
                });
                let mut shared = shared.lock().unwrap_or_else(PoisonError::into_inner);
                shared.busy = false;
                shared.status = match found {
                    Ok(Some((version, asset))) => {
                        shared.release = Some((version, asset));
                        Status::Available { version }
                    }
                    Ok(None) => Status::UpToDate,
                    Err(message) => Status::Failed(Failure::from(message)),
                };
            })
            .map_or_else(|error| self.fail(&format!("update check thread: {error}")), drop);
    }

    /// Downloads and installs the release the last check found, unless the updater is busy.
    pub fn update(&self) {
        let release = self.shared.lock().unwrap_or_else(PoisonError::into_inner).release.clone();
        let Some((version, asset)) = release else { return };
        let Some(shared) = self.start(Status::Downloading { version, done: 0, total: asset.size }) else { return };
        let config = Arc::clone(&self.config);
        std::thread::Builder::new()
            .name("skate2 update".into())
            .spawn(move || {
                let mut progress = |done| {
                    shared.lock().unwrap_or_else(PoisonError::into_inner).status = Status::Downloading { version, done, total: asset.size };
                };
                let result = config.update(version, &asset, &mut progress);
                let mut shared = shared.lock().unwrap_or_else(PoisonError::into_inner);
                shared.busy = false;
                shared.status = match result {
                    Ok(()) => Status::Installed { version },
                    Err(failure) => Status::Failed(failure),
                };
            })
            .map_or_else(|error| self.fail(&format!("update thread: {error}")), drop);
    }

    /// The shared state, marked busy with `status`, when nothing else runs.
    fn start(&self, status: Status) -> Option<Arc<Mutex<Shared>>> {
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        if shared.busy || matches!(shared.status, Status::Installed { .. }) {
            return None;
        }
        shared.busy = true;
        shared.status = status;
        Some(Arc::clone(&self.shared))
    }

    fn fail(&self, message: &str) {
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        shared.busy = false;
        shared.status = Status::Failed(Failure::from(message.to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead as _, BufReader, Write as _};
    use std::net::TcpListener;

    /// The platform these tests update from: a release's own, whichever it is. The host's
    /// platform is `None` where releases have no zip, so the tests name one instead.
    const PLATFORM: Platform = Platform::Windows;

    /// An HTTP server on this machine serving `files` by path, for `requests` requests.
    fn serve(files: Vec<(String, Vec<u8>)>, requests: usize) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = format!("http://{}", listener.local_addr().expect("address"));
        std::thread::spawn(move || {
            for stream in listener.incoming().take(requests) {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().expect("stream"));
                let mut line = String::new();
                let _ = reader.read_line(&mut line);
                let path = line.split(' ').nth(1).unwrap_or_default().to_owned();
                loop {
                    let mut header = String::new();
                    if reader.read_line(&mut header).unwrap_or(0) <= 2 {
                        break;
                    }
                }
                let response = match files.iter().find(|(file, _)| *file == path) {
                    Some((_, body)) => [format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes(), body.clone()].concat(),
                    None => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
                };
                let _ = stream.write_all(&response);
            }
        });
        address
    }

    struct Release {
        folder: PathBuf,
        config: Config,
    }

    impl Drop for Release {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.folder);
        }
    }

    /// Version 0.2.0 on a local server, its zip holding `game` as `game.exe`, signed by
    /// `signer`'s key, for a 0.1.0 game in its own folder.
    fn release(name: &str, game: &[u8], signer: &str, sha_of: Option<&[u8]>) -> Release {
        use sha2::{Digest as _, Sha256};
        let folder = std::env::temp_dir().join(format!("skate2-update-release-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("folder");
        std::fs::write(folder.join("game.exe"), b"version 0.1.0").expect("old game");
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("game.exe", zip::write::SimpleFileOptions::default()).expect("entry");
        zip.write_all(game).expect("game");
        let zip = zip.finish().expect("zip").into_inner();
        let mut sha256 = [0_u8; 32];
        sha256.copy_from_slice(&Sha256::digest(sha_of.unwrap_or(&zip)));
        let manifest = Manifest {
            version: Version::parse("0.2.0").expect("version"),
            assets: vec![Asset {
                platform: PLATFORM,
                name: "game.zip".into(),
                size: zip.len() as u64,
                sha256,
            }],
        }
        .text();
        let seed = crate::release_key::update_signing_seed(&signer.repeat(64));
        let signature = manifest::sign(&seed, manifest.as_bytes()).into_bytes();
        let files = vec![
            ("/owner/game/releases/latest/download/latest-release.txt".to_owned(), manifest.into_bytes()),
            ("/owner/game/releases/download/v0.2.0/latest-release.txt.sig".to_owned(), signature),
            ("/owner/game/releases/download/v0.2.0/game.zip".to_owned(), zip),
        ];
        let config = Config {
            server: serve(files, 3),
            repository: "owner/game".into(),
            public_key: manifest::public_key(&crate::release_key::update_signing_seed(&"7".repeat(64))),
            version: Version::parse("0.1.0").expect("version"),
            platform: PLATFORM,
            directory: folder.clone(),
            executable: "game.exe".into(),
        };
        Release { folder, config }
    }

    #[test]
    fn a_signed_newer_release_replaces_the_game() {
        let release = release("signed", b"version 0.2.0", "7", None);
        let mut seen = 0;
        let installed = crate::update_now(&release.config, &mut |done, _| seen = done).expect("updated");
        assert_eq!(installed, Version::parse("0.2.0"));
        assert!(seen > 0);
        assert_eq!(std::fs::read(release.folder.join("game.exe")).expect("game"), b"version 0.2.0");
        assert!(!release.folder.join(DOWNLOAD).exists());
    }

    #[test]
    fn a_release_signed_with_another_key_is_refused() {
        let release = release("unsigned", b"version 0.2.0", "8", None);
        let error = crate::update_now(&release.config, &mut |_, _| {}).expect_err("refused");
        assert!(error.contains("signature"), "{error}");
        assert_eq!(std::fs::read(release.folder.join("game.exe")).expect("game"), b"version 0.1.0");
    }

    #[test]
    fn a_zip_that_does_not_match_its_signed_hash_is_refused() {
        let release = release("tampered", b"version 0.2.0", "7", Some(b"another zip"));
        let error = crate::update_now(&release.config, &mut |_, _| {}).expect_err("refused");
        assert!(error.contains("SHA-256"), "{error}");
        assert_eq!(std::fs::read(release.folder.join("game.exe")).expect("game"), b"version 0.1.0");
        assert!(!release.folder.join(DOWNLOAD).exists());
    }

    /// Where releases hold a zip for this platform, a build with the key and the repository
    /// updates, and nothing else does.
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn only_a_build_with_a_key_and_a_repository_updates() {
        let key = manifest::hex(&[3; 32]);
        assert!(Config::for_build(Some(&key), Some("owner/skate2-recomp"), "0.1.0", "game.exe").is_some());
        assert!(Config::for_build(None, Some("owner/skate2-recomp"), "0.1.0", "game.exe").is_none(), "a development build");
        assert!(Config::for_build(Some(&key), None, "0.1.0", "game.exe").is_none());
        for repository in ["", "owner", "owner/", "/name", "owner/name/extra", "owner/..", "own er/name", "owner/na?me"] {
            assert!(Config::for_build(Some(&key), Some(repository), "0.1.0", "game.exe").is_none(), "{repository}");
        }
        assert!(Config::for_build(Some("12"), Some("owner/name"), "0.1.0", "game.exe").is_none(), "a short key");
        assert!(Config::for_build(Some(&key), Some("owner/name"), "0.1", "game.exe").is_none(), "a bad version");
    }

    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn a_release_s_files_come_from_its_tag() {
        let key = manifest::hex(&[3; 32]);
        let config = Config::for_build(Some(&key), Some("owner/name"), "0.1.0", "game.exe").expect("config");
        assert_eq!(config.latest_manifest_url(), "https://github.com/owner/name/releases/latest/download/latest-release.txt");
        let version = Version::parse("0.2.0").expect("version");
        assert_eq!(config.release_url(version, "game.zip"), "https://github.com/owner/name/releases/download/v0.2.0/game.zip");
    }

    /// Where releases hold no zip for this platform, even a build with the key and the repository
    /// has nothing to update from: the same files are a build that never looks.
    #[cfg(not(any(windows, target_os = "linux")))]
    #[test]
    fn a_platform_without_a_release_zip_does_not_update() {
        let key = manifest::hex(&[3; 32]);
        assert!(Config::for_build(Some(&key), Some("owner/name"), "0.1.0", "game.exe").is_none());
    }
}
