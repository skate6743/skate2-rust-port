//! PC port addition: the Skate 3 game folder linked on the Downloadable Content screen (Link
//! Skate 3 Data, `config.json`'s `skate3_data`; `skate2_core::skate3`). The player picks Skate
//! 3's `default.xex` or disc image, as the game's own folder is picked by its `.xex` at the first
//! launch; an image is read in place as the folder. A launch opens the folder again, and its
//! districts' streams join the game's mounts after the downloadable content's.
use crate::dlc_link::LinkProgress;
use skate2_core::{
    skate3::{self, Skate3Data, Skate3Error},
    ui::{dlc_menu::Pack, game_config::GameConfig},
};
use skate2_formats::mount::{MountSet, MountedBig};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
};

/// What the picker's thread sends: the file picked (none when it was closed), then the folder
/// opened as Skate 3's.
enum Picked {
    File(Option<PathBuf>),
    Opened(PathBuf, Result<Skate3Data, Skate3Error>),
}

pub(crate) struct Skate3Link {
    mounts: Arc<MountSet>,
    /// The linked folder's district streams in the game's mounts.
    archives: Vec<Arc<MountedBig>>,
    picker: Option<mpsc::Receiver<Picked>>,
}

impl Skate3Link {
    fn install(&mut self, data: Skate3Data) -> Result<(), Skate3Error> {
        let archives = data.world_archives()?;
        self.remove();
        self.archives = archives.into_iter().map(|archive| self.mounts.push_dlc(archive)).collect();
        for district in &data.districts {
            eprintln!("skate 3 data: {} with {} teleports", district.spec.title, district.spots.len());
        }
        skate3::set_linked(Some(Arc::new(data)));
        Ok(())
    }

    fn remove(&mut self) {
        for archive in self.archives.drain(..) {
            self.mounts.remove_dlc(&archive);
        }
        skate3::set_linked(None);
    }

    /// Launch: the folder `config.json` links, when it still opens as Skate 3's; otherwise the
    /// link leaves the file.
    pub(crate) fn mount_linked(mounts: &Arc<MountSet>) -> Self {
        let mut link = Self { mounts: Arc::clone(mounts), archives: Vec::new(), picker: None };
        let mut config = GameConfig::load();
        if config.skate3_data.is_empty() {
            return link;
        }
        let root = PathBuf::from(&config.skate3_data);
        let opened = Skate3Data::open(&root).and_then(|data| link.install(data));
        if let Err(error) = opened {
            eprintln!("skate 3 data {} {error}: unlinked", root.display());
            config.skate3_data.clear();
            if let Err(error) = config.save() {
                eprintln!("config.json could not be saved: {error}");
            }
        } else {
            eprintln!("skate 3 data: {}", root.display());
        }
        link
    }

    /// The pack row the screen lists after the downloadable content's, while linked.
    pub(crate) fn pack(&self) -> Option<Pack> {
        skate3::linked().map(|_| Pack { title: skate3::PACK_TITLE.to_owned(), description: skate3::PACK_DESCRIPTION.to_owned() })
    }

    /// Link Skate 3 Data: the picker opens on a thread of its own, which then reads the folder.
    pub(crate) fn open_picker(&mut self) {
        if self.picker.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let work = move || {
            let file = rfd::FileDialog::new()
                .set_title("Link Skate 3 Data: pick Skate 3's default.xex or disc image")
                .add_filter("Skate 3 default.xex or disc image", &["xex", "iso"])
                .pick_file();
            let _ = sender.send(Picked::File(file.clone()));
            if let Some(file) = file {
                // A disc image is read in place as Skate 3's folder (`skate2_formats::disc`).
                let opened = if skate2_formats::disc::is_image(&file) {
                    skate2_formats::disc::check_image(&file).map_err(Skate3Error::Read)
                } else {
                    Ok(())
                };
                let root = skate3::folder_of_pick(&file);
                let opened = opened.and_then(|()| Skate3Data::open(&root));
                let _ = sender.send(Picked::Opened(root, opened));
            }
        };
        if cfg!(target_os = "macos") {
            work();
        } else if let Err(error) = std::thread::Builder::new().name("skate 3 picker".into()).spawn(work) {
            eprintln!("skate 3 data picker could not start: {error}");
            let (sender, failed) = mpsc::channel();
            let _ = sender.send(Picked::File(None));
            self.picker = Some(failed);
            return;
        }
        self.picker = Some(receiver);
    }

    /// The picker's progress since the last call: the file picked, then the folder linked.
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
            Picked::File(file) => {
                if file.is_none() {
                    self.picker = None;
                }
                Some(LinkProgress::Picked(file.is_some()))
            }
            Picked::Opened(root, opened) => {
                self.picker = None;
                Some(self.link(root, opened))
            }
        }
    }

    fn link(&mut self, root: PathBuf, opened: Result<Skate3Data, Skate3Error>) -> LinkProgress {
        let folder = root.display().to_string();
        let installed = opened.and_then(|data| {
            let names: Vec<String> = data.districts.iter().map(|district| format!("{} ({} teleports)", district.spec.title, district.spots.len())).collect();
            self.install(data).map(|()| names)
        });
        match installed {
            Ok(names) => {
                let mut config = GameConfig::load();
                config.skate3_data.clone_from(&folder);
                let mut text = String::from("#Skate 3 data was linked:^");
                for name in &names {
                    text.push_str("^- ");
                    text.push_str(name);
                }
                text.push_str("^- Skate 3 clothing in Edit Skater's Merchandise");
                if let Err(error) = config.save() {
                    text.push_str(&format!("^^config.json could not be saved ({error}): the data stays linked until the game closes."));
                }
                LinkProgress::Linked(text, true)
            }
            Err(error) => LinkProgress::Linked(format!("#Skate 3 data was not linked:^^- {folder} {error}"), false),
        }
    }

    /// X on the Skate 3 row: its streams leave the mounts and `config.json` forgets the folder.
    pub(crate) fn unlink(&mut self) -> String {
        self.remove();
        let mut config = GameConfig::load();
        config.skate3_data.clear();
        let mut text = String::from("#Skate 3 data was unlinked.");
        if let Err(error) = config.save() {
            text.push_str(&format!("^^config.json could not be saved ({error}): it links again at the next launch."));
        }
        text
    }
}
