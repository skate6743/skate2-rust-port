//! PC port addition: the Skate 3 game folder linked on the Downloadable Content screen (Link
//! Skate 3 Data, `config.json`'s `skate3_data`; `skate2_core::skate3`). The player picks Skate
//! 3's `default.xex` or disc image, as the game's own folder is picked by its `.xex` at the first
//! launch; an image is read in place as the folder. A launch opens the folder again, and its
//! districts' streams join the game's mounts after the downloadable content's.
//!
//! Skate 3's downloadable content (`config.json`'s `skate3_dlc`, `skate2_core::skate3::dlc`),
//! linked through Link DLCs (`crate::dlc_link`), plays over the folder: the data is read again with
//! the packs, and each pack's maps, online challenges and clothing join with it. Unlinking a pack
//! reads the data again without it.
use crate::dlc_link::{LinkProgress, file_name};
use skate2_core::{
    skate3::{
        self, PackContents, Skate3Data, Skate3Error,
        dlc::Pack as Skate3Pack,
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

/// A linked Skate 3 pack: the file `config.json` names for it, and the pack.
type LinkedPack = (String, Arc<Skate3Pack>);

/// What the picker's thread sends: the file picked (none when it was closed), then the folder
/// opened as Skate 3's with the packs `config.json` links over it. An unlink's thread sends the
/// data read again without the pack at its index.
enum Picked {
    File(Option<PathBuf>),
    Opened(PathBuf, Result<(Skate3Data, Vec<LinkedPack>), Skate3Error>),
    Unlinked(usize, Result<Skate3Data, Skate3Error>),
}

/// What linking Skate 3 packs did: the packs linked and already linked (sk8popup's report), the
/// files that could not be linked and why (the log), whether any pack is new, and why
/// `config.json` could not be saved.
#[derive(Default)]
pub(crate) struct LinkReport {
    pub(crate) linked: Vec<String>,
    pub(crate) already: Vec<String>,
    pub(crate) failed: Vec<(String, String)>,
    pub(crate) changed: bool,
    pub(crate) save_error: Option<String>,
}

pub(crate) struct Skate3Link {
    mounts: Arc<MountSet>,
    /// The linked folder's district streams, and its packs' worlds, in the game's mounts.
    archives: Vec<Arc<MountedBig>>,
    picker: Option<mpsc::Receiver<Picked>>,
    /// The linked folder.
    root: Option<PathBuf>,
    /// The linked Skate 3 packs, in the order linked.
    packs: Vec<LinkedPack>,
}

/// The packs of `paths` that open, each once, in order; the others are reported and left out.
fn open_packs(paths: &[String]) -> Vec<LinkedPack> {
    let mut packs: Vec<LinkedPack> = Vec::new();
    for path in paths {
        match Skate3Pack::open(Path::new(path)) {
            Ok(pack) if packs.iter().all(|(_, held)| held.identity != pack.identity) => {
                eprintln!("skate 3 dlc: {} ({path})", pack.title);
                packs.push((path.clone(), Arc::new(pack)));
            }
            Ok(pack) => eprintln!("skate 3 dlc {path}: {} is linked by another file, unlinked", pack.title),
            Err(error) => eprintln!("skate 3 dlc {path} {error}: unlinked"),
        }
    }
    packs
}

/// A pack's row title.
fn title(pack: &Skate3Pack) -> String {
    format!("Skate 3: {}", pack.title)
}

/// What the screen and the link report say of a pack: what it added to the game.
fn contents_text(pack: &Skate3Pack, contents: Option<&PackContents>) -> String {
    let mut parts = Vec::new();
    if let Some(contents) = contents {
        if !contents.maps.is_empty() {
            parts.push(format!("Maps: {}", contents.maps.join(", ")));
        }
        if contents.online > 0 {
            parts.push(format!("{} online challenges in the Map Pool's Skate 3 DLC", contents.online));
        }
        if contents.clothing > 0 {
            parts.push(format!("{} pieces of clothing in Edit Skater's Skate 3 Merchandise", contents.clothing));
        }
    }
    if parts.is_empty() { pack.description.clone() } else { format!("{}.", parts.join(". ")) }
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

    fn pack_handles(&self) -> Vec<Arc<Skate3Pack>> {
        self.packs.iter().map(|(_, pack)| Arc::clone(pack)).collect()
    }

    /// Launch: the folder `config.json` links, when it still opens as Skate 3's, with the packs it
    /// links over it (a file that no longer opens as one leaves the list); a folder that does not
    /// open leaves the file, its packs kept for a later link.
    pub(crate) fn mount_linked(mounts: &Arc<MountSet>) -> Self {
        let mut link = Self { mounts: Arc::clone(mounts), archives: Vec::new(), picker: None, root: None, packs: Vec::new() };
        let mut config = GameConfig::load();
        if config.skate3_data.is_empty() {
            return link;
        }
        let root = PathBuf::from(&config.skate3_data);
        link.packs = open_packs(&config.skate3_dlc);
        let opened = Skate3Data::open_with(&root, &link.pack_handles()).and_then(|data| link.install(data));
        if let Err(error) = opened {
            eprintln!("skate 3 data {} {error}: unlinked", root.display());
            link.packs.clear();
            config.skate3_data.clear();
            if let Err(error) = config.save() {
                eprintln!("config.json could not be saved: {error}");
            }
            return link;
        }
        eprintln!("skate 3 data: {}", root.display());
        link.root = Some(root);
        let kept: Vec<String> = link.packs.iter().map(|(path, _)| path.clone()).collect();
        if kept != config.skate3_dlc {
            config.skate3_dlc = kept;
            if let Err(error) = config.save() {
                eprintln!("config.json could not be saved: {error}");
            }
        }
        link
    }

    /// The pack row the screen lists after the downloadable content's, while linked.
    pub(crate) fn pack(&self) -> Option<Pack> {
        skate3::linked().map(|_| Pack { title: skate3::PACK_TITLE.to_owned(), description: skate3::PACK_DESCRIPTION.to_owned() })
    }

    /// The rows the screen lists after the Skate 3 row: each linked Skate 3 pack, with what it
    /// added.
    pub(crate) fn dlc_packs(&self) -> Vec<Pack> {
        let Some(data) = skate3::linked() else { return Vec::new() };
        self.packs
            .iter()
            .map(|(_, pack)| Pack { title: title(pack), description: contents_text(pack, data.packs.iter().find(|contents| contents.identity == pack.identity)) })
            .collect()
    }

    /// What Link DLCs reads Skate 3 packs over: the linked folder and the packs it holds; none
    /// while no Skate 3 data is linked.
    pub(crate) fn context(&self) -> Option<(PathBuf, Vec<Arc<Skate3Pack>>)> {
        skate3::linked()?;
        Some((self.root.clone()?, self.pack_handles()))
    }

    /// Link DLCs picked Skate 3 packs: those new to the link join it with `rebuilt`, the data the
    /// picker's thread read again with them, and `config.json`.
    pub(crate) fn link_dlc(&mut self, picked: Vec<(PathBuf, Arc<Skate3Pack>)>, rebuilt: Option<Result<Skate3Data, Skate3Error>>) -> LinkReport {
        let mut report = LinkReport::default();
        let mut new: Vec<LinkedPack> = Vec::new();
        for (path, pack) in picked {
            if self.packs.iter().chain(&new).any(|(_, held)| held.identity == pack.identity) {
                let title = title(&pack);
                if !report.already.contains(&title) {
                    report.already.push(title);
                }
            } else {
                new.push((path.to_string_lossy().into_owned(), pack));
            }
        }
        if new.is_empty() {
            return report;
        }
        let refused = |report: &mut LinkReport, new: &[LinkedPack], why: &str| {
            report.failed.extend(new.iter().map(|(path, _)| (file_name(Path::new(path)), why.to_owned())));
        };
        let data = match rebuilt {
            Some(Ok(data)) if skate3::linked().is_some() => data,
            Some(Err(error)) => {
                refused(&mut report, &new, &format!("could not be linked ({error})"));
                return report;
            }
            _ => {
                refused(&mut report, &new, "needs Skate 3 data: link it with Link Skate 3 Data first");
                return report;
            }
        };
        let contents = data.packs.clone();
        if let Err(error) = self.install(data) {
            refused(&mut report, &new, &format!("could not be linked ({error})"));
            return report;
        }
        let mut config = GameConfig::load();
        for (path, pack) in new {
            let added = contents_text(&pack, contents.iter().find(|contents| contents.identity == pack.identity));
            report.linked.push(if added.is_empty() { title(&pack) } else { format!("{}: {added}", title(&pack)) });
            config.skate3_dlc.push(path.clone());
            self.packs.push((path, pack));
        }
        report.changed = true;
        report.save_error = config.save().err().map(|error| error.to_string());
        report
    }

    /// Link Skate 3 Data: the picker opens on a thread of its own, which then reads the folder with
    /// the packs `config.json` links over it.
    pub(crate) fn open_picker(&mut self) {
        if self.picker.is_some() {
            return;
        }
        let paths = GameConfig::load().skate3_dlc;
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
                let packs = open_packs(&paths);
                let handles: Vec<Arc<Skate3Pack>> = packs.iter().map(|(_, pack)| Arc::clone(pack)).collect();
                let opened = opened.and_then(|()| Skate3Data::open_with(&root, &handles)).map(|data| (data, packs));
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

    /// X on a Skate 3 pack's row: the data is read again without it on a thread of its own, and
    /// [`Self::poll`] reports the unlink. False when `index` names no pack or a read is under way.
    pub(crate) fn unlink_dlc(&mut self, index: usize) -> bool {
        if index >= self.packs.len() || self.picker.is_some() {
            return false;
        }
        let Some(root) = self.root.clone() else { return false };
        let rest: Vec<Arc<Skate3Pack>> =
            self.packs.iter().enumerate().filter(|&(at, _)| at != index).map(|(_, (_, pack))| Arc::clone(pack)).collect();
        let (sender, receiver) = mpsc::channel();
        let work = move || {
            let _ = sender.send(Picked::Unlinked(index, Skate3Data::open_with(&root, &rest)));
        };
        if let Err(error) = std::thread::Builder::new().name("skate 3 unlink".into()).spawn(work) {
            eprintln!("skate 3 dlc unlink could not start: {error}");
            return false;
        }
        self.picker = Some(receiver);
        true
    }

    /// The picker's progress since the last call: the file picked, then the folder linked; or an
    /// unlink's end.
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
            Picked::Unlinked(index, rebuilt) => {
                self.picker = None;
                Some(self.finish_unlink(index, rebuilt))
            }
        }
    }

    fn link(&mut self, root: PathBuf, opened: Result<(Skate3Data, Vec<LinkedPack>), Skate3Error>) -> LinkProgress {
        let folder = root.display().to_string();
        let installed = opened.and_then(|(data, packs)| {
            let names: Vec<String> = data
                .districts
                .iter()
                .filter(|district| skate3::is_base_district(district))
                .map(|district| format!("{} ({} teleports)", district.spec.title, district.spots.len()))
                .collect();
            let contents = data.packs.clone();
            self.install(data).map(|()| (names, contents, packs))
        });
        match installed {
            Ok((names, contents, packs)) => {
                let mut config = GameConfig::load();
                config.skate3_data.clone_from(&folder);
                config.skate3_dlc = packs.iter().map(|(path, _)| path.clone()).collect();
                let mut text = String::from("#Skate 3 data was linked:^");
                for name in &names {
                    text.push_str("^- ");
                    text.push_str(name);
                }
                text.push_str("^- Skate 3 clothing in Edit Skater's Merchandise");
                text.push_str("^- Skate 3 Physics available in Physics Settings");
                text.push_str("^- Skate 3 Online Challenges now available");
                for (_, pack) in &packs {
                    let added = contents_text(pack, contents.iter().find(|contents| contents.identity == pack.identity));
                    text.push_str("^- ");
                    text.push_str(&if added.is_empty() { title(pack) } else { format!("{}: {added}", title(pack)) });
                }
                self.root = Some(root);
                self.packs = packs;
                if let Err(error) = config.save() {
                    text.push_str(&format!("^^config.json could not be saved ({error}): the data stays linked until the game closes."));
                }
                LinkProgress::Linked(text, true)
            }
            Err(error) => LinkProgress::Linked(format!("#Skate 3 data was not linked:^^- {folder} {error}"), false),
        }
    }

    fn finish_unlink(&mut self, index: usize, rebuilt: Result<Skate3Data, Skate3Error>) -> LinkProgress {
        let Some((path, pack)) = self.packs.get(index).cloned() else {
            return LinkProgress::Linked("#No DLC was unlinked.".to_owned(), false);
        };
        match rebuilt.and_then(|data| self.install(data)) {
            Ok(()) => {
                self.packs.remove(index);
                let mut config = GameConfig::load();
                config.skate3_dlc.retain(|held| *held != path);
                let saved = config.save().err().map(|error| error.to_string());
                LinkProgress::Linked(dlc_menu::unlink_report(&title(&pack), saved.as_deref()), true)
            }
            Err(error) => LinkProgress::Linked(format!("#{} was not unlinked:^^- {error}", title(&pack)), false),
        }
    }

    /// X on the Skate 3 row: its streams leave the mounts and `config.json` forgets the folder; its
    /// packs stay listed there for a later link.
    pub(crate) fn unlink(&mut self) -> String {
        self.remove();
        self.root = None;
        self.packs.clear();
        let mut config = GameConfig::load();
        config.skate3_data.clear();
        let mut text = String::from("#Skate 3 data was unlinked.");
        if let Err(error) = config.save() {
            text.push_str(&format!("^^config.json could not be saved ({error}): it links again at the next launch."));
        }
        text
    }
}
