//! Downloadable content. PC port: the player links packs from the Downloadable Content screen,
//! in place of the Xbox LIVE Marketplace and `DLC::Manager`'s content enumeration. A pack is
//! one of Skate 2's Xbox 360 content packages (title 4541087F, marketplace content) or one of
//! the six packs' `.big` archives extracted from its package. Its archives join the game's
//! after them, as `DLC::Manager::EnumerateContent` (`0x825e86c8`) adds every `*.big` of its
//! content mounts to the `dlcbig:` device, and the game reads from them what the native does:
//! the unlock lists (`LoadUnlocksFromFiles` `0x82573100`), the attribute vaults
//! (`cVaultManager::StartDLCLoad` `0x82ab8b50`) and the language databases
//! (`LanguageManager::StartDLCLanguageDBLoad` `0x82d67d48`).

use crate::challenge::unlockables::{category, string_hash};
use skate2_formats::{
    big::{BigArchive, BigEntryKey},
    localization::{Histogram, LanguageCategory},
    mount::{ArchiveSource, MountSet, MountedBig},
    stfs::StfsPackage,
};
use std::{
    fmt,
    io::Read as _,
    path::{Path, PathBuf},
};

/// Skate 2's title id and the content type of marketplace content, which a package must have.
pub const TITLE_ID: u32 = 0x4541_087F;
pub const MARKETPLACE_CONTENT: u32 = 2;

/// One of the packs Skate 2 was sold with: the entry its archive alone holds, the name its
/// package shows (the Xbox LIVE Marketplace's, which the original's Downloadable Content
/// screen listed) and what it adds.
struct KnownPack {
    marker: &'static str,
    title: &'static str,
    description: &'static str,
}

const KNOWN: [KnownPack; 6] = [
    KnownPack {
        marker: "data\\db\\dlc_skate1_classic.vlt",
        title: "San Van Classic Pack",
        description: "The Community Center, Oliver School and the Parkade from skate., with their challenges.",
    },
    KnownPack {
        marker: "data\\db\\dlc_rob_fantasy_plaza.vlt",
        title: "Dyrdek's Fantasy Park Pack",
        description: "Rob Dyrdek's Fantasy Plaza and its challenges.",
    },
    KnownPack {
        marker: "data\\db\\dlc_maloof_money_cup.vlt",
        title: "Maloof Money Cup Pack",
        description: "The Maloof Money Cup course, its contests and its challenges.",
    },
    KnownPack {
        marker: "data\\unlocks\\timeismoney.unlock",
        title: "Time Is Money",
        description: "Unlocks every location, skater and piece of gear the career can unlock, and three gestures.",
    },
    KnownPack {
        marker: "data\\unlocks\\throwback.unlock",
        title: "Throwback Pack",
        description: "Retro gear and five gestures.",
    },
    KnownPack {
        marker: "data\\unlocks\\filmpack.unlock",
        title: "Skate 2 Filmer Pack",
        description: "Advanced skate.Reel camera tools and their tutorial, extra gear and four gestures.",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DlcError {
    /// The file could not be read.
    Read(String),
    /// A content package of another game's, or no downloadable content of Skate 2's.
    OtherContent,
    /// No archive of Skate 2's downloadable content.
    NotDlc,
}

impl fmt::Display for DlcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(f, "cannot be read ({error})"),
            Self::OtherContent => f.write_str("is not Skate 2 downloadable content"),
            Self::NotDlc => f.write_str("is not a Skate 2 downloadable content file"),
        }
    }
}

impl std::error::Error for DlcError {}

/// A linked pack.
#[derive(Debug, Clone)]
pub struct DlcPack {
    /// The file the player picked: a package or an extracted archive.
    pub path: PathBuf,
    /// Which pack it is, the same for its package and its extracted archive: a known pack's
    /// identifying entry, else the package's first archive's name.
    pub identity: String,
    pub title: String,
    pub description: String,
    pub archives: Vec<MountedBig>,
}

fn names(archive: &BigArchive) -> impl Iterator<Item = &str> {
    archive.entries.iter().filter_map(|entry| match &entry.key {
        BigEntryKey::Name(name) => Some(name.as_str()),
        BigEntryKey::Hash(_) => None,
    })
}

/// An archive of downloadable content: one with an unlock list, a vault or a world stream.
fn holds_content(archive: &BigArchive) -> bool {
    names(archive).any(|name| {
        let name = name.to_ascii_lowercase().replace('/', "\\");
        (name.starts_with("data\\unlocks\\") && name.ends_with(".unlock"))
            || (name.starts_with("data\\db\\") && name.ends_with(".vlt"))
            || name.starts_with("data\\content\\world\\stream\\")
    })
}

fn known(archives: &[MountedBig]) -> Option<&'static KnownPack> {
    KNOWN.iter().find(|pack| archives.iter().any(|archive| names(archive.archive()).any(|name| name.replace('/', "\\").eq_ignore_ascii_case(pack.marker))))
}

impl DlcPack {
    /// Opens a package or an extracted archive and names the pack.
    ///
    /// # Errors
    ///
    /// [`DlcError::Read`] when the file cannot be read, [`DlcError::OtherContent`] for a package
    /// of another game's or no downloadable content, [`DlcError::NotDlc`] for any other file.
    pub fn open(path: &Path) -> Result<Self, DlcError> {
        let read = |error: &dyn fmt::Display| DlcError::Read(error.to_string());
        let mut magic = [0_u8; 4];
        let mut file = std::fs::File::open(path).map_err(|e| read(&e))?;
        if file.read_exact(&mut magic).is_err() {
            return Err(DlcError::NotDlc);
        }
        drop(file);
        if StfsPackage::is_package(&magic) {
            let package = StfsPackage::open(path).map_err(|e| read(&e))?;
            if package.title_id != TITLE_ID || package.content_type != MARKETPLACE_CONTENT {
                return Err(DlcError::OtherContent);
            }
            let mut archives = Vec::new();
            for entry in package.root_files().filter(|entry| entry.name.to_ascii_lowercase().ends_with(".big")) {
                let file = package.file(entry).map_err(|e| read(&e))?;
                let archive = MountedBig::open_source(ArchiveSource::Package(file)).map_err(|e| read(&e))?;
                if holds_content(archive.archive()) {
                    archives.push(archive);
                }
            }
            if archives.is_empty() {
                return Err(DlcError::OtherContent);
            }
            return Ok(match known(&archives) {
                Some(pack) => Self::known(path, pack, archives),
                None => {
                    let identity = archive_name(&archives[0]);
                    let title = match package.display_name() {
                        "" => identity.clone(),
                        name => name.to_owned(),
                    };
                    Self { path: path.to_owned(), identity, title, description: package.description().to_owned(), archives }
                }
            });
        }
        let archive = MountedBig::open_source(ArchiveSource::File(path.to_owned())).map_err(|_| DlcError::NotDlc)?;
        let archives = vec![archive];
        // An extracted archive could be any game's: only the packs Skate 2 was sold with link.
        known(&archives).map(|pack| Self::known(path, pack, archives)).ok_or(DlcError::NotDlc)
    }

    fn known(path: &Path, pack: &KnownPack, archives: Vec<MountedBig>) -> Self {
        Self {
            path: path.to_owned(),
            identity: pack.marker.to_owned(),
            title: pack.title.to_owned(),
            description: pack.description.to_owned(),
            archives,
        }
    }
}

fn archive_name(archive: &MountedBig) -> String {
    match archive.source() {
        ArchiveSource::Package(file) => file.name().to_ascii_lowercase(),
        ArchiveSource::File(path) => path.file_name().map(|name| name.to_string_lossy().to_ascii_lowercase()).unwrap_or_default(),
    }
}

/// The mounted downloadable content's entries under `folder` (lowercase, backslashed) whose
/// name passes `keep`, archive by archive in mount order and each archive's in table order (the
/// order `AllDeviceDirectoryEntry` lists a device's files). The game's own archives hold none
/// of the files the DLC flow looks for.
fn dlc_files(mounts: &MountSet, folder: &str, keep: impl Fn(&str) -> bool) -> Vec<(String, std::sync::Arc<MountedBig>)> {
    let mut files = Vec::new();
    for archive in mounts.dlc_archives() {
        for name in names(archive.archive()) {
            let normal = name.replace('/', "\\");
            let lower = normal.to_ascii_lowercase();
            if let Some(file) = lower.strip_prefix(folder)
                && !file.contains('\\')
                && keep(file)
            {
                files.push((normal.clone(), std::sync::Arc::clone(&archive)));
            }
        }
    }
    files
}

/// `LoadUnlocksFromFiles` (`0x82573100`): each line of every `data\unlocks\*.unlock`, `NAME,
/// item`, unlocks the item's `GetStringHash` in the category of that `kUnlockCategories` name
/// (compared whole), the item trimmed of spaces and tabs. A line without a comma or naming no
/// category does nothing.
#[must_use]
pub fn unlocks(mounts: &MountSet) -> Vec<(u32, u32)> {
    let mut unlocks = Vec::new();
    for (path, archive) in dlc_files(mounts, "data\\unlocks\\", |file| file.ends_with(".unlock")) {
        let Ok(text) = archive.read_path(&path) else { continue };
        unlocks.extend(parse_unlocks(&text));
    }
    unlocks
}

/// One unlock list's unlocks: `EA::StdC::GetTextLine` ends a line at `\r` or `\n`.
#[must_use]
pub fn parse_unlocks(text: &[u8]) -> Vec<(u32, u32)> {
    let mut unlocks = Vec::new();
    for line in text.split(|&byte| byte == b'\n' || byte == b'\r') {
        let Some(comma) = line.iter().position(|&byte| byte == b',') else { continue };
        let name = &line[..comma];
        let Some(index) = category::NAMES.iter().position(|category| category.as_bytes() == name) else { continue };
        let item: String = trim_tabs(&line[comma + 1..]).iter().map(|&byte| char::from(byte)).collect();
        unlocks.push((u32::try_from(index).unwrap_or(0), string_hash(&item)));
    }
    unlocks
}

/// `FindFirstNotOf(" \t")` and `RFindFirstNotOf(" \t")`: spaces and tabs off both ends.
fn trim_tabs(bytes: &[u8]) -> &[u8] {
    let tab = |byte: &u8| *byte == b' ' || *byte == b'\t';
    let start = bytes.iter().position(|byte| !tab(byte)).unwrap_or(bytes.len());
    let end = bytes.iter().rposition(|byte| !tab(byte)).map_or(start, |at| at + 1);
    &bytes[start..end]
}

/// `StartDLCLoad("dlcbig:data\\db\\")` and `DLCLoadUpdate` (`0x82ab9c98`): every `*.vlt` of the
/// downloadable content's `data\db`, each loaded as `data\db\<name>`.
#[must_use]
pub fn vaults(mounts: &MountSet) -> Vec<String> {
    dlc_files(mounts, "data\\db\\", |file| file.ends_with(".vlt")).into_iter().map(|(path, _)| path).collect()
}

/// `StartDLCLanguageDBLoad` and `DLCLanguageDBLoadUpdate` (`0x82d6a768`): every
/// `data\fe\languages\english\*.BIN` whose name holds `dlc`, read in turn by
/// `LoadDLCLanguagePacks` (`0x82d69140`): a `HISTOGRAM` file is the database's histogram, any
/// other its category, and a database is done once it has both.
#[must_use]
pub fn language_databases(mounts: &MountSet) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut databases = Vec::new();
    let (mut histogram, mut category) = (None, None);
    for (path, archive) in dlc_files(mounts, "data\\fe\\languages\\english\\", |file| file.ends_with(".bin") && file.contains("dlc")) {
        let Ok(bytes) = archive.read_path(&path) else { continue };
        if path.contains("HISTOGRAM") {
            histogram = Some(bytes);
        } else {
            category = Some(bytes);
        }
        if histogram.is_some() && category.is_some() {
            databases.push((histogram.take().unwrap_or_default(), category.take().unwrap_or_default()));
        }
    }
    databases
}

/// Every downloadable content database's strings, each unpacked with its own histogram, in
/// database order; a malformed database adds none.
#[must_use]
pub fn strings(mounts: &MountSet) -> Vec<(u32, Vec<u16>)> {
    let mut strings = Vec::new();
    for (histogram, category) in language_databases(mounts) {
        let (Ok(histogram), Ok(category)) = (Histogram::parse(&histogram), LanguageCategory::parse(&category)) else { continue };
        for (hash, packed) in category.packed_strings() {
            if let Ok(text) = histogram.unpack(packed, 16384, false)
                && text.valid
                && text.terminated
            {
                strings.push((hash, text.units));
            }
        }
    }
    strings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unlock_list_unlocks_its_categories_items() {
        let text = b"CLOTHING_SPONSOR, cac_throwback_pack_inventory \r\nGESTURES, gesture_spock\r\nREPLAY,filmpack\n\
                     NOT_A_CATEGORY, x\r\nno comma here\r\nGESTURES , gesture_peace\r\n";
        assert_eq!(
            parse_unlocks(text),
            [
                (category::CLOTHING_SPONSOR, string_hash("cac_throwback_pack_inventory")),
                (category::GESTURES, string_hash("gesture_spock")),
                (category::REPLAY, string_hash("filmpack")),
            ],
            "the category is compared whole, untrimmed"
        );
        assert_eq!(parse_unlocks(b"GESTURES,\tgesture_flex\t"), [(category::GESTURES, string_hash("gesture_flex"))]);
    }

    #[test]
    fn the_known_packs_markers_name_distinct_files() {
        for (i, pack) in KNOWN.iter().enumerate() {
            assert!(KNOWN.iter().skip(i + 1).all(|other| other.marker != pack.marker));
            assert!(pack.marker.starts_with("data\\"));
        }
    }
}
