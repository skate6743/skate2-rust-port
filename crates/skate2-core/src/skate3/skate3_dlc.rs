//! PC port addition: Skate 3's downloadable content, linked on the Downloadable Content screen as
//! Skate 2's is (Link DLCs, `config.json`'s `skate3_dlc`), over linked Skate 3 data. A pack is one
//! of Skate 3's Xbox 360 content packages (title 454108E6, marketplace content) or the `EB` archive
//! extracted from one.
//!
//! Skate 3 reads a pack's archive through its `dlcbig:` device (TU3 `0x826dab98`): each
//! `data\db\*.vaultlist.xml` names Vaults to load in turn (`cVaultManager::DLCDirectorySearch`,
//! `cVaultListLoadCallback`), then the language databases of `data\fe\languages\<language>` load,
//! the subtitles and the assets. The archive joins Skate 3's own mounts whole
//! ([`super::Skate3Data::open_with`]) and the game's through [`game_view`].
//!
//! What a pack adds:
//! - each of its locations whose world it streams and that holds challenges of its own (Maloof
//!   Money Cup NYC, the Art Gallery, the Sanatorium, Danny Way's Mega Compound): a map beside the
//!   districts, with the online challenges there;
//! - each location its unlock lists name (`LOCATIONS,dist_blackboxpark`) whose world is on Skate
//!   3's disc (Black Box Distribution's park): a map too;
//! - its clothing (`data\content\recipe\createacharacter\cas_db_*.recipe`), joined to Skate 3's.
//!
//! Left out: Skate.Park lots (`WORLDS,<lot>`), which stand empty without the park editor's
//! objects, and the rest of what packs unlock.

use skate2_formats::{
    big::{BigEntryKey, BigFormat},
    mount::{ArchiveSource, MountedBig},
    stfs::StfsPackage,
};
use std::{
    fmt,
    io::Read as _,
    path::{Path, PathBuf},
};

/// Skate 3's title id, which a pack's package has, as marketplace content (content type 2).
pub const TITLE_ID: u32 = 0x4541_08E6;
const MARKETPLACE_CONTENT: u32 = 2;

/// The folders of a pack that join the game's mounts ([`game_view`]): its worlds' streams and
/// models, its challenge hulls, and the models and textures its worlds and clothing load.
const GAME_FOLDERS: [&str; 6] = [
    "data\\content\\world\\",
    "data\\content\\missions\\",
    "data\\content\\createacharacter\\",
    "data\\content\\static\\",
    "data\\content\\dynamic\\",
    "data\\content\\textures\\",
];

/// A linked pack.
#[derive(Debug)]
pub struct Pack {
    /// The file the player picked: a content package or an extracted archive.
    pub path: PathBuf,
    /// Which pack it is, the same for its package and its extracted archive: its archive's name
    /// (`play_00000000.big`).
    pub identity: String,
    /// The package's display name (the Xbox LIVE Marketplace's), else the archive's name.
    pub title: String,
    /// The package's description; empty for an extracted archive.
    pub description: String,
    /// Its archives, in Skate 3's `EB` format.
    pub archives: Vec<MountedBig>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackError {
    /// The file could not be read.
    Read(String),
    /// Not Skate 3's downloadable content.
    NotSkate3,
    /// Skate 3's downloadable content with no map or clothing: unlocks alone (the Filmer Pack, the
    /// Skate Share Pack, Time is Money).
    NothingToAdd,
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(f, "cannot be read ({error})"),
            Self::NotSkate3 => f.write_str("is not Skate 3 downloadable content"),
            Self::NothingToAdd => f.write_str("adds no map or clothing the game uses"),
        }
    }
}

impl std::error::Error for PackError {}

fn names(archive: &MountedBig) -> impl Iterator<Item = &str> {
    archive.archive().entries.iter().filter_map(|entry| match &entry.key {
        BigEntryKey::Name(name) => Some(name.as_str()),
        BigEntryKey::Hash(_) => None,
    })
}

/// An entry's name as compared here: lowercase, with backslashes.
fn normal(name: &str) -> String {
    name.to_ascii_lowercase().replace('/', "\\")
}

/// The archive's entries directly in `folder` (lowercase, backslashed, ending in `\`) whose name
/// ends with `suffix` (lowercase), as the archive names them, in its table's order.
fn files_in<'a>(archive: &'a MountedBig, folder: &'a str, suffix: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    names(archive).filter(move |name| {
        let name = normal(name);
        name.strip_prefix(folder).is_some_and(|file| !file.contains('\\') && file.ends_with(suffix))
    })
}

/// Whether an archive is a pack's: Skate 3's `EB` format, with the version file every pack keeps
/// at its root (`.\maloofmc.ver`).
fn is_pack_archive(archive: &MountedBig) -> bool {
    matches!(archive.archive().format, BigFormat::Eb { .. })
        && names(archive).any(|name| {
            let name = normal(name);
            let file = name.strip_prefix(".\\").unwrap_or(&name);
            !file.contains('\\') && file.ends_with(".ver")
        })
}

/// The Vault files a `*.vaultlist.xml` names (`<VaultFile>challengebanks\dlc_mmcn.vlt</VaultFile>`),
/// each as `data\db\<file>`, in its order.
fn parse_vault_list(text: &str) -> Vec<String> {
    let mut vaults = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<VaultFile>") {
        rest = &rest[start + "<VaultFile>".len()..];
        let Some(end) = rest.find("</VaultFile>") else { break };
        let file = rest[..end].trim();
        if !file.is_empty() {
            vaults.push(format!("data\\db\\{}", file.replace('/', "\\")));
        }
        rest = &rest[end..];
    }
    vaults
}

/// The Vaults a pack loads: each `data\db\*.vaultlist.xml`'s in turn, in the archive's order.
pub(super) fn vaults(archive: &MountedBig) -> Vec<String> {
    let mut vaults = Vec::new();
    for list in files_in(archive, "data\\db\\", ".vaultlist.xml") {
        match archive.read_path(list) {
            Ok(bytes) => vaults.extend(parse_vault_list(&String::from_utf8_lossy(&bytes))),
            Err(error) => eprintln!("skate 3 dlc: {list}: {error}"),
        }
    }
    vaults
}

/// The clothing databases a pack adds (`data\content\recipe\createacharacter\cas_db_*.recipe`).
pub(super) fn clothing(archive: &MountedBig) -> Vec<String> {
    files_in(archive, "data\\content\\recipe\\createacharacter\\", ".recipe")
        .filter(|name| normal(name).rsplit('\\').next().is_some_and(|file| file.starts_with("cas_db_")))
        .map(str::to_owned)
        .collect()
}

/// A pack's English language databases (`dlcbig:data\fe\languages\english\*.BIN`): its histogram
/// with each other file of the folder, a category.
pub(super) fn english(archive: &MountedBig) -> Vec<(Vec<u8>, Vec<u8>)> {
    let files: Vec<&str> = files_in(archive, "data\\fe\\languages\\english\\", ".bin").collect();
    let Some(histogram) = files.iter().find(|name| normal(name).contains("histogram")).and_then(|name| archive.read_path(name).ok()) else {
        return Vec::new();
    };
    files
        .iter()
        .filter(|name| !normal(name).contains("histogram"))
        .filter_map(|name| archive.read_path(name).ok())
        .map(|category| (histogram.clone(), category))
        .collect()
}

/// A pack's global locator files (`data\content\global_locators\<group>\<world>\*.rx2`).
pub(super) fn locators(archive: &MountedBig) -> Vec<String> {
    names(archive)
        .filter(|name| {
            let name = normal(name);
            name.starts_with("data\\content\\global_locators\\") && name.ends_with(".rx2")
        })
        .map(str::to_owned)
        .collect()
}

/// Whether a pack streams the world `stream`: its `<stream>_Sim.xsm` is the pack's.
pub(super) fn streams(archive: &MountedBig, stream: &str) -> bool {
    archive.archive().find_path(&format!("data\\content\\world\\stream\\{stream}\\{stream}_Sim.xsm")).is_some()
}

/// The locations one unlock list unlocks (`LOCATIONS,<fe_locations name>`), as
/// `UnlockablesBackEnd::LoadUnlocksFromFiles` (TU3 `0x82505220`) reads a line: up to `\r` or `\n`,
/// the category compared whole, the item trimmed of spaces and tabs.
fn parse_unlocked_locations(text: &[u8]) -> Vec<String> {
    let mut locations = Vec::new();
    for line in text.split(|&byte| byte == b'\n' || byte == b'\r') {
        let Some(comma) = line.iter().position(|&byte| byte == b',') else { continue };
        if &line[..comma] != b"LOCATIONS" {
            continue;
        }
        let item = String::from_utf8_lossy(&line[comma + 1..]).trim_matches([' ', '\t']).to_owned();
        if !item.is_empty() {
            locations.push(item);
        }
    }
    locations
}

/// The locations a pack's unlock lists (`data\unlocks\*.unlock`) unlock.
pub(super) fn unlocked_locations(archive: &MountedBig) -> Vec<String> {
    files_in(archive, "data\\unlocks\\", ".unlock")
        .filter_map(|list| archive.read_path(list).ok())
        .flat_map(|text| parse_unlocked_locations(&text))
        .collect()
}

/// Whether an archive adds what the game takes: clothing, Vaults (its maps' locations and worlds)
/// or an unlock list naming a location.
fn adds(archive: &MountedBig) -> bool {
    !clothing(archive).is_empty()
        || files_in(archive, "data\\db\\", ".vaultlist.xml").next().is_some()
        || !unlocked_locations(archive).is_empty()
}

/// A pack's archive as it joins the game's mounts: the entries of [`GAME_FOLDERS`] alone. Its
/// databases, unlocks, languages, scenes, stream lists and boundaries stay Skate 3's: the game's
/// downloadable content flow reads its mounts' `data\db\*.vlt`, unlock lists, language databases
/// and scenes as Skate 2's own (`crate::dlc`), and a stream list it finds there before Skate 3's.
#[must_use]
pub fn game_view(archive: &MountedBig) -> MountedBig {
    archive.retaining(|name| {
        let name = normal(name);
        GAME_FOLDERS.iter().any(|folder| name.starts_with(folder))
    })
}

fn archive_name(archive: &MountedBig) -> String {
    match archive.source() {
        ArchiveSource::Package(file) => file.name().to_ascii_lowercase(),
        ArchiveSource::File(path) => path.file_name().map(|name| name.to_string_lossy().to_ascii_lowercase()).unwrap_or_default(),
    }
}

impl Pack {
    /// Opens a content package or an extracted archive as a pack.
    ///
    /// # Errors
    ///
    /// [`PackError::Read`] when the file cannot be read, [`PackError::NotSkate3`] for anything but
    /// Skate 3's downloadable content, [`PackError::NothingToAdd`] for a pack of unlocks alone.
    pub fn open(path: &Path) -> Result<Self, PackError> {
        let read = |error: &dyn fmt::Display| PackError::Read(error.to_string());
        let mut magic = [0_u8; 4];
        let mut file = std::fs::File::open(path).map_err(|e| read(&e))?;
        if file.read_exact(&mut magic).is_err() {
            return Err(PackError::NotSkate3);
        }
        drop(file);
        let (archives, title, description) = if StfsPackage::is_package(&magic) {
            let package = StfsPackage::open(path).map_err(|e| read(&e))?;
            if package.title_id != TITLE_ID || package.content_type != MARKETPLACE_CONTENT {
                return Err(PackError::NotSkate3);
            }
            let mut archives = Vec::new();
            for entry in package.root_files().filter(|entry| entry.name.to_ascii_lowercase().ends_with(".big")) {
                let file = package.file(entry).map_err(|e| read(&e))?;
                let archive = MountedBig::open_source(ArchiveSource::Package(file)).map_err(|e| read(&e))?;
                if is_pack_archive(&archive) {
                    archives.push(archive);
                }
            }
            (archives, package.display_name().to_owned(), package.description().to_owned())
        } else {
            let archive = MountedBig::open_source(ArchiveSource::File(path.to_owned())).map_err(|_| PackError::NotSkate3)?;
            let archives = if is_pack_archive(&archive) { vec![archive] } else { Vec::new() };
            (archives, String::new(), String::new())
        };
        let Some(first) = archives.first() else { return Err(PackError::NotSkate3) };
        if !archives.iter().any(adds) {
            return Err(PackError::NothingToAdd);
        }
        let identity = archive_name(first);
        let title = if title.is_empty() { identity.clone() } else { title };
        Ok(Self { path: path.to_owned(), identity, title, description, archives })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vault_list_names_its_vaults_in_order() {
        let text = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<VaultList>\r\n  <VaultFile>dlc_maloof_money_cup.vlt</VaultFile>\r\n  \
                    <VaultFile> challengebanks\\dlc_mmcn.vlt </VaultFile>\r\n  <VaultFile></VaultFile>\r\n</VaultList>\r\n";
        assert_eq!(parse_vault_list(text), ["data\\db\\dlc_maloof_money_cup.vlt", "data\\db\\challengebanks\\dlc_mmcn.vlt"]);
    }

    #[test]
    fn an_unlock_list_unlocks_its_locations() {
        let text = b"ENTITLEMENTS,BLACKBOX_SKATE_PARK\r\nLOCATIONS,dist_blackboxpark\r\nCHALLENGES,otl_bbx_01\r\nLOCATIONS ,x\nLOCATIONS,\t dist_other \n";
        assert_eq!(parse_unlocked_locations(text), ["dist_blackboxpark", "dist_other"]);
    }

    /// `SKATE3_DLC_DIR` names a folder of Skate 3's content packages (`454108E6\00000002`): the
    /// packs that add maps or clothing open, a pack of unlocks alone does not.
    #[test]
    fn skate3_packages_open_as_packs() {
        let Some(dir) = std::env::var_os("SKATE3_DLC_DIR") else { return };
        let mut opened = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            match Pack::open(&path) {
                Ok(pack) => opened.push(pack.title),
                Err(error) => eprintln!("{}: {error}", path.display()),
            }
        }
        for title in ["Maloof Money Cup 2010 NYC", "San Van Party Pack", "After Dark", "Danny Way's Hawaiian Dream", "Skate.Create Upgrade Pack", "Black Box Distribution Skate Park"] {
            assert!(opened.iter().any(|opened| opened == title), "{title} opens: {opened:?}");
        }
        assert!(!opened.iter().any(|title| title == "Filmer Pack"), "unlocks alone add nothing");
    }
}
