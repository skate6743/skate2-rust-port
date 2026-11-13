//! PC port addition: the downloadable content the player links (`config.json`'s `linked_dlc`;
//! `skate2_core::dlc`). Each linked file is checked and mounted at launch, in place of
//! `StateBootFlow::EnumerateDLC`'s look at the console's storage, and more are linked from the
//! Downloadable Content screen through the system's file picker. A file that is Skate 3's
//! downloadable content (`skate2_core::skate3::dlc`) links over the linked Skate 3 data instead
//! (`crate::skate3_link`).
use crate::skate3_link::Skate3Link;
use skate2_core::{
    dlc::{DlcError, DlcPack},
    skate3::{
        Skate3Data, Skate3Error,
        dlc::{Pack as Skate3Pack, PackError},
    },
    ui::{
        dlc_menu::{self, Pack},
        game_config::GameConfig,
    },
};
use skate2_formats::mount::{MountSet, MountedBig};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
};

/// A linked pack: which pack it is, what the screen shows of it, the file `config.json` names
/// for it and its mounted archives.
#[derive(Clone, Debug)]
struct Linked {
    identity: String,
    pack: Pack,
    path: String,
    archives: Vec<Arc<MountedBig>>,
}

/// A picked file opened as downloadable content: Skate 2's, Skate 3's, or neither's.
enum Opened {
    Skate2(Result<DlcPack, DlcError>),
    Skate3(Result<Arc<Skate3Pack>, PackError>),
    Neither,
}

/// What the picker's thread sends: the files picked (none when it was closed), then each file
/// opened, with the linked Skate 3 data read again with the Skate 3 packs among them that it does
/// not hold yet (none without such a pack or without linked Skate 3 data).
enum Picked {
    Files(Vec<PathBuf>),
    Opened(Vec<(PathBuf, Opened)>, Option<Result<Skate3Data, Skate3Error>>),
}

/// What [`DlcLinks::poll`] reports to the screen.
pub(crate) enum LinkProgress {
    /// The picker closed, with files to read or none.
    Picked(bool),
    /// The picked files were linked: sk8popup's report, and whether any pack is new.
    Linked(String, bool),
}

pub(crate) struct DlcLinks {
    mounts: Arc<MountSet>,
    linked: Vec<Linked>,
    picker: Option<mpsc::Receiver<Picked>>,
}

fn mount(mounts: &MountSet, pack: DlcPack, path: String) -> Linked {
    Linked {
        identity: pack.identity.clone(),
        pack: Pack { title: pack.title.clone(), description: pack.description.clone() },
        path,
        archives: pack.archives.into_iter().map(|archive| mounts.push_dlc(archive)).collect(),
    }
}

pub(crate) fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned())
}

/// A picked file as downloadable content: Skate 2's when it opens as one of its packs, else
/// Skate 3's when it opens as one of that game's.
fn open(path: &Path) -> Opened {
    match DlcPack::open(path) {
        Ok(pack) => Opened::Skate2(Ok(pack)),
        Err(error @ DlcError::Read(_)) => Opened::Skate2(Err(error)),
        Err(_) => match Skate3Pack::open(path) {
            Ok(pack) => Opened::Skate3(Ok(Arc::new(pack))),
            Err(PackError::NotSkate3) => Opened::Neither,
            Err(error) => Opened::Skate3(Err(error)),
        },
    }
}

impl DlcLinks {
    /// Launch: every file `config.json` links that opens as a pack not yet linked mounts after the
    /// game's archives, in the order linked; any other leaves the file.
    pub(crate) fn mount_linked(mounts: &Arc<MountSet>) -> Self {
        let mut links = Self { mounts: Arc::clone(mounts), linked: Vec::new(), picker: None };
        let mut config = GameConfig::load();
        let mut kept = Vec::new();
        for path in &config.linked_dlc {
            match DlcPack::open(Path::new(path)) {
                Ok(pack) if links.linked.iter().all(|linked| linked.identity != pack.identity) => {
                    eprintln!("downloadable content: {} ({path})", pack.title);
                    kept.push(path.clone());
                    links.linked.push(mount(mounts, pack, path.clone()));
                }
                Ok(pack) => eprintln!("downloadable content {path}: {} is linked by another file, unlinked", pack.title),
                Err(error) => eprintln!("downloadable content {path} {error}: unlinked"),
            }
        }
        if kept != config.linked_dlc {
            config.linked_dlc = kept;
            if let Err(error) = config.save() {
                eprintln!("config.json could not be saved: {error}");
            }
        }
        links
    }

    /// The packs as the screen lists them, in the order linked.
    pub(crate) fn packs(&self) -> Vec<Pack> {
        self.linked.iter().map(|linked| linked.pack.clone()).collect()
    }

    /// Link DLCs: the picker opens, on a thread of its own so that the game keeps drawing under
    /// it, and that thread then reads the picked files and, when Skate 3 packs new to the link are
    /// among them, the linked Skate 3 data again with them (`skate3`: the linked folder and the
    /// packs it holds). macOS keeps its dialogs on the main thread, which waits for the picker
    /// there.
    pub(crate) fn open_picker(&mut self, skate3: Option<(PathBuf, Vec<Arc<Skate3Pack>>)>) {
        if self.picker.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let work = move || {
            let files = rfd::FileDialog::new().set_title("Link Skate 2 or Skate 3 downloadable content").pick_files().unwrap_or_default();
            let _ = sender.send(Picked::Files(files.clone()));
            let opened: Vec<(PathBuf, Opened)> = files.iter().map(|path| (path.clone(), open(path))).collect();
            let rebuilt = skate3.and_then(|(root, mut packs)| {
                let before = packs.len();
                for (_, opened) in &opened {
                    if let Opened::Skate3(Ok(pack)) = opened
                        && packs.iter().all(|held| held.identity != pack.identity)
                    {
                        packs.push(Arc::clone(pack));
                    }
                }
                (packs.len() > before).then(|| Skate3Data::open_with(&root, &packs))
            });
            let _ = sender.send(Picked::Opened(opened, rebuilt));
        };
        if cfg!(target_os = "macos") {
            work();
        } else if let Err(error) = std::thread::Builder::new().name("dlc picker".into()).spawn(work) {
            eprintln!("downloadable content picker could not start: {error}");
            let (sender, failed) = mpsc::channel();
            let _ = sender.send(Picked::Files(Vec::new()));
            self.picker = Some(failed);
            return;
        }
        self.picker = Some(receiver);
    }

    /// The picker's progress since the last call: the files picked, then the packs linked, each
    /// mounted and added to `config.json` (Skate 3's through `skate3`). sk8popup names the packs
    /// linked and already linked; the log, the files that could not be linked and why.
    pub(crate) fn poll(&mut self, skate3: &mut Skate3Link) -> Option<LinkProgress> {
        let message = match self.picker.as_ref()?.try_recv() {
            Ok(message) => message,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.picker = None;
                return Some(LinkProgress::Picked(false));
            }
        };
        match message {
            Picked::Files(files) => {
                if files.is_empty() {
                    self.picker = None;
                }
                Some(LinkProgress::Picked(!files.is_empty()))
            }
            Picked::Opened(opened, rebuilt) => {
                self.picker = None;
                Some(self.link(opened, rebuilt, skate3))
            }
        }
    }

    fn link(&mut self, opened: Vec<(PathBuf, Opened)>, rebuilt: Option<Result<Skate3Data, Skate3Error>>, skate3: &mut Skate3Link) -> LinkProgress {
        let (mut linked, mut already, mut failed) = (Vec::new(), Vec::new(), Vec::new());
        let mut config = GameConfig::load();
        let mut skate3_packs = Vec::new();
        for (path, opened) in opened {
            match opened {
                Opened::Skate2(Ok(pack)) if self.linked.iter().any(|linked| linked.identity == pack.identity) => {
                    if !already.contains(&pack.title) {
                        already.push(pack.title.clone());
                    }
                }
                Opened::Skate2(Ok(pack)) => {
                    linked.push(pack.title.clone());
                    let path = path.to_string_lossy().into_owned();
                    config.linked_dlc.push(path.clone());
                    self.linked.push(mount(&self.mounts, pack, path));
                }
                Opened::Skate2(Err(error)) => failed.push((file_name(&path), error.to_string())),
                Opened::Skate3(Ok(pack)) => skate3_packs.push((path, pack)),
                Opened::Skate3(Err(error)) => failed.push((file_name(&path), error.to_string())),
                Opened::Neither => failed.push((file_name(&path), "is not Skate 2 or Skate 3 downloadable content".to_owned())),
            }
        }
        let mut save_error = if linked.is_empty() { None } else { config.save().err().map(|error| error.to_string()) };
        let mut changed = !linked.is_empty();
        if !skate3_packs.is_empty() {
            let report = skate3.link_dlc(skate3_packs, rebuilt);
            linked.extend(report.linked);
            already.extend(report.already);
            failed.extend(report.failed);
            save_error = save_error.or(report.save_error);
            changed |= report.changed;
        }
        for (file, reason) in &failed {
            eprintln!("downloadable content {file} {reason}: not linked");
        }
        LinkProgress::Linked(dlc_menu::link_report(&linked, &already, save_error.as_deref()), changed)
    }

    /// X on the Downloadable Content screen: the pack at `row` leaves the mounts and
    /// `config.json`. Returns sk8popup's report, none for a row that names no pack.
    pub(crate) fn unlink(&mut self, row: usize) -> Option<String> {
        if row >= self.linked.len() {
            return None;
        }
        let linked = self.linked.remove(row);
        for archive in &linked.archives {
            self.mounts.remove_dlc(archive);
        }
        let mut config = GameConfig::load();
        config.linked_dlc.retain(|path| *path != linked.path);
        let saved = config.save().err().map(|error| error.to_string());
        Some(dlc_menu::unlink_report(&linked.pack.title, saved.as_deref()))
    }
}
