//! PC port addition: the downloadable content the player links (`config.json`'s `linked_dlc`;
//! `skate2_core::dlc`). Each linked file is checked and mounted at launch, in place of
//! `StateBootFlow::EnumerateDLC`'s look at the console's storage, and more are linked from the
//! Downloadable Content screen through the system's file picker.
use skate2_core::{
    dlc::{DlcError, DlcPack},
    ui::{
        dlc_menu::{self, Pack},
        game_config::GameConfig,
    },
};
use skate2_formats::mount::MountSet;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
};

/// A linked pack: which pack it is, and what the screen shows of it.
#[derive(Clone, Debug)]
struct Linked {
    identity: String,
    pack: Pack,
}

/// What the picker's thread sends: the files picked (none when it was closed), then each file
/// opened as downloadable content.
enum Picked {
    Files(Vec<PathBuf>),
    Opened(Vec<(PathBuf, Result<DlcPack, DlcError>)>),
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

fn mount(mounts: &MountSet, pack: DlcPack) -> Linked {
    let linked = Linked { identity: pack.identity.clone(), pack: Pack { title: pack.title.clone(), description: pack.description.clone() } };
    for archive in pack.archives {
        mounts.push_dlc(archive);
    }
    linked
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned())
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
                    links.linked.push(mount(mounts, pack));
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
    /// it, and that thread then reads the picked files. macOS keeps its dialogs on the main
    /// thread, which waits for the picker there.
    pub(crate) fn open_picker(&mut self) {
        if self.picker.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let work = move || {
            let files = rfd::FileDialog::new().set_title("Link Skate 2 downloadable content").pick_files().unwrap_or_default();
            let opened = files.iter().map(|path| (path.clone(), DlcPack::open(path))).collect();
            let _ = sender.send(Picked::Files(files));
            let _ = sender.send(Picked::Opened(opened));
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
    /// mounted and added to `config.json`.
    pub(crate) fn poll(&mut self) -> Option<LinkProgress> {
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
            Picked::Opened(opened) => {
                self.picker = None;
                Some(self.link(opened))
            }
        }
    }

    fn link(&mut self, opened: Vec<(PathBuf, Result<DlcPack, DlcError>)>) -> LinkProgress {
        let (mut linked, mut already, mut failed) = (Vec::new(), Vec::new(), Vec::new());
        let mut config = GameConfig::load();
        for (path, result) in opened {
            match result {
                Ok(pack) if self.linked.iter().any(|linked| linked.identity == pack.identity) => {
                    if !already.contains(&pack.title) {
                        already.push(pack.title.clone());
                    }
                }
                Ok(pack) => {
                    linked.push(pack.title.clone());
                    config.linked_dlc.push(path.to_string_lossy().into_owned());
                    self.linked.push(mount(&self.mounts, pack));
                }
                Err(error) => failed.push((file_name(&path), error.to_string())),
            }
        }
        if !linked.is_empty()
            && let Err(error) = config.save()
        {
            failed.push(("config.json".into(), format!("could not be saved ({error}): the DLCs stay linked until the game closes")));
        }
        let changed = !linked.is_empty();
        LinkProgress::Linked(dlc_menu::link_report(&linked, &already, &failed), changed)
    }
}
