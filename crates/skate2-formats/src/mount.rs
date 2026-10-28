//! Seek-based mounting of Skate 2's EA BIG archives.

use crate::{
    big::{BigArchive, BigEntry, BigEntryKey, BigError},
    refpack::{self, RefPackError},
    stfs::{StfsFile, StfsReader},
};
use std::{
    fmt,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    ops::Range,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, PoisonError, RwLock},
    time::{Duration, Instant},
};

const FIXED_HEADER_SIZE: usize = 16;

/// One payload read from a mounted archive, as the [`observe_reads`] observer sees it.
pub struct ArchiveRead<'a> {
    pub archive: &'a Path,
    pub entry: &'a BigEntry,
    pub bytes: usize,
    /// Opening, reading and decompressing.
    pub elapsed: Duration,
}

static READ_OBSERVER: OnceLock<fn(&ArchiveRead<'_>)> = OnceLock::new();

/// Host diagnostics: once set, `observer` runs on the reading thread after each payload read.
pub fn observe_reads(observer: fn(&ArchiveRead<'_>)) {
    let _ = READ_OBSERVER.set(observer);
}

/// Storage path used to satisfy a logical archive-entry range read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicalEntryStorage {
    /// The requested bytes were read directly from the raw BIG entry.
    Raw,
    /// The complete outer BIG entry was RefPack-expanded before slicing.
    RefPack,
}

/// Bytes from one logical BIG entry together with range diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalEntryRange {
    pub bytes: Vec<u8>,
    pub logical_len: u64,
    pub storage: LogicalEntryStorage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MountGroup {
    Boot,
    Load,
    Game,
    Characters,
    CreateACharacter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchiveSpec {
    pub index: u8,
    pub group: MountGroup,
    pub use_external_header: bool,
    pub relative_path: &'static str,
}

/// The archive table used by `MountBigFileGroup` at `0x8272a2d0`.
pub const SKATE2_ARCHIVES: [ArchiveSpec; 20] = [
    ArchiveSpec::new(0, MountGroup::Boot, false, "data\\big\\render.big"),
    ArchiveSpec::new(1, MountGroup::Boot, false, "data\\big\\miscboot.big"),
    ArchiveSpec::new(2, MountGroup::Boot, false, "data\\big\\colourcubes.big"),
    ArchiveSpec::new(3, MountGroup::Boot, false, "data\\db\\db.big"),
    ArchiveSpec::new(4, MountGroup::Load, true, "data\\big\\scene.big"),
    ArchiveSpec::new(5, MountGroup::Load, false, "data\\content\\worldproxy.big"),
    ArchiveSpec::new(6, MountGroup::Load, false, "data\\content\\worldbam.big"),
    ArchiveSpec::new(7, MountGroup::Load, false, "data\\content\\worldmisc.big"),
    ArchiveSpec::new(
        8,
        MountGroup::Load,
        false,
        "data\\content\\assettoc_cac.big",
    ),
    ArchiveSpec::new(9, MountGroup::Load, false, "data\\content\\assettoc_lw.big"),
    ArchiveSpec::new(
        10,
        MountGroup::Load,
        false,
        "data\\content\\assettoc_mqe.big",
    ),
    ArchiveSpec::new(11, MountGroup::Load, false, "data\\big\\miscload.big"),
    ArchiveSpec::new(12, MountGroup::Load, false, "data\\big\\globallocators.big"),
    ArchiveSpec::new(13, MountGroup::Load, true, "data\\big\\anim.big"),
    ArchiveSpec::new(14, MountGroup::Game, false, "data\\content\\missions.big"),
    ArchiveSpec::new(15, MountGroup::Game, true, "data\\big\\aipaths.big"),
    ArchiveSpec::new(
        16,
        MountGroup::Characters,
        true,
        "data\\content\\livingworld.big",
    ),
    ArchiveSpec::new(
        17,
        MountGroup::Characters,
        false,
        "data\\content\\marquee.big",
    ),
    ArchiveSpec::new(18, MountGroup::Characters, true, "data\\big\\ropa2.big"),
    ArchiveSpec::new(
        19,
        MountGroup::CreateACharacter,
        true,
        "data\\content\\createacharacter.big",
    ),
];

impl ArchiveSpec {
    const fn new(
        index: u8,
        group: MountGroup,
        use_external_header: bool,
        relative_path: &'static str,
    ) -> Self {
        Self {
            index,
            group,
            use_external_header,
            relative_path,
        }
    }

    /// The archive's file under the game's folder, as the file system spells it
    /// ([`crate::host_path::on_disk`]).
    #[must_use]
    pub fn path_under(&self, root: &Path) -> PathBuf {
        crate::host_path::on_disk(&root.join(
            self.relative_path
                .replace('\\', std::path::MAIN_SEPARATOR_STR),
        ))
    }
}

/// Where an archive's bytes are: a file, or a file inside an Xbox 360 content package
/// (downloadable content, `XenonDLCDriver`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArchiveSource {
    File(PathBuf),
    Package(StfsFile),
}

impl ArchiveSource {
    /// A reader over the archive's bytes from its first.
    ///
    /// # Errors
    ///
    /// Returns [`MountError::Io`] when the file or package cannot be opened.
    pub fn open(&self) -> Result<ArchiveReader, MountError> {
        match self {
            Self::File(path) => open_file(path).map(ArchiveReader::File),
            Self::Package(file) => file
                .open()
                .map(ArchiveReader::Package)
                .map_err(|source| MountError::Io { path: file.package().to_owned(), source }),
        }
    }

    fn path(&self) -> &Path {
        match self {
            Self::File(path) => path,
            Self::Package(file) => file.package(),
        }
    }
}

/// `Read` and `Seek` over an [`ArchiveSource`].
#[derive(Debug)]
pub enum ArchiveReader {
    File(File),
    Package(StfsReader),
}

impl Read for ArchiveReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::File(file) => file.read(buf),
            Self::Package(file) => file.read(buf),
        }
    }
}

impl Seek for ArchiveReader {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        match self {
            Self::File(file) => file.seek(to),
            Self::Package(file) => file.seek(to),
        }
    }
}

/// One entry's bytes as they are taken (see [`MountSet::open_stream`]): a stored entry read from
/// its archive, or a RefPack one unpacked whole.
#[derive(Debug)]
pub enum EntryStream {
    Stored(io::Take<ArchiveReader>),
    Unpacked(io::Cursor<Vec<u8>>),
}

impl Read for EntryStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Stored(entry) => entry.read(buf),
            Self::Unpacked(entry) => entry.read(buf),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MountedBig {
    data_path: PathBuf,
    header_path: PathBuf,
    source: ArchiveSource,
    archive: BigArchive,
}

impl MountedBig {
    /// Opens an archive table without reading the archive payload into memory.
    ///
    /// When `use_external_header` is true, the reference game first tries the
    /// sibling `.bh` file and falls back to the `.big` header.
    ///
    /// # Errors
    ///
    /// Returns [`MountError`] if neither header can be read or its table is
    /// malformed.
    pub fn open(
        data_path: impl AsRef<Path>,
        use_external_header: bool,
    ) -> Result<Self, MountError> {
        let data_path = crate::host_path::on_disk(data_path.as_ref());
        let mut preferred_header = data_path.clone();
        if use_external_header {
            preferred_header.set_extension("bh");
            preferred_header = crate::host_path::on_disk(&preferred_header);
        }

        let header_path = if use_external_header && !preferred_header.is_file() {
            data_path.clone()
        } else {
            preferred_header
        };
        let archive = read_archive_header(&mut open_file(&header_path)?, &header_path)?;

        Ok(Self {
            source: ArchiveSource::File(data_path.clone()),
            data_path,
            header_path,
            archive,
        })
    }

    /// Opens an archive whose table is at the start of its own bytes, as
    /// `BigHandler::AddArchiveFromFile` (`0x82bb91d8`) reads downloadable content's (no `.bh`).
    ///
    /// # Errors
    ///
    /// Returns [`MountError`] if the header cannot be read or its table is malformed.
    pub fn open_source(source: ArchiveSource) -> Result<Self, MountError> {
        let path = source.path().to_owned();
        let archive = read_archive_header(&mut source.open()?, &path)?;
        Ok(Self { data_path: path.clone(), header_path: path, source, archive })
    }

    #[must_use]
    pub fn archive(&self) -> &BigArchive {
        &self.archive
    }

    /// The file holding the archive: the `.big`, or the content package it is inside (see
    /// [`MountedBig::source`]).
    #[must_use]
    pub fn data_path(&self) -> &Path {
        &self.data_path
    }

    #[must_use]
    pub fn source(&self) -> &ArchiveSource {
        &self.source
    }

    #[must_use]
    pub fn header_path(&self) -> &Path {
        &self.header_path
    }

    /// Reads and, when necessary, RefPack-decompresses one archive entry.
    ///
    /// # Errors
    ///
    /// Returns [`MountError`] if the entry does not exist, its payload cannot
    /// be read, or decompression/size validation fails.
    pub fn read_path(&self, path: &str) -> Result<Vec<u8>, MountError> {
        let normalized = normalize_game_path(path);
        let entry = self
            .archive
            .find_path(&normalized)
            .ok_or_else(|| MountError::EntryNotFound(normalized.clone()))?;
        self.read_entry(entry)
    }

    /// Reads a dictionary entry already obtained from this mounted archive.
    /// This is required for hashed BIG tables, whose original virtual names
    /// are intentionally absent from the shipping header.
    ///
    /// # Errors
    ///
    /// Returns [`MountError`] when the archive payload cannot be read or the
    /// entry cannot be decompressed and size-validated.
    pub fn read_entry(&self, entry: &BigEntry) -> Result<Vec<u8>, MountError> {
        let started = Instant::now();
        let mut file = self.source.open()?;
        let bytes = read_entry(&mut file, entry)?;
        self.observe(entry, bytes.len(), started);
        Ok(bytes)
    }

    /// Reads a logical byte range from one named or hashed archive entry.
    ///
    /// Raw entries use a direct seek after the outer entry's first two bytes
    /// have been probed. `RefPack` entries must be expanded in full because a
    /// decoded offset cannot be sought independently through back-references.
    ///
    /// # Errors
    ///
    /// Returns [`MountError`] if the entry is absent, the range is reversed or
    /// outside the logical entry, offset arithmetic overflows, or I/O,
    /// decompression, or hashed-entry size validation fails.
    pub fn read_path_range(
        &self,
        path: &str,
        range: Range<u64>,
    ) -> Result<LogicalEntryRange, MountError> {
        let normalized = normalize_game_path(path);
        let entry = self
            .archive
            .find_path(&normalized)
            .ok_or_else(|| MountError::EntryNotFound(normalized.clone()))?;
        self.read_entry_range(entry, range)
    }

    /// Reads a logical byte range from an entry already resolved in this
    /// mounted archive.
    ///
    /// # Errors
    ///
    /// Returns [`MountError`] for invalid ranges, offset overflow, I/O,
    /// decompression, or hashed-entry size mismatch.
    pub fn read_entry_range(
        &self,
        entry: &BigEntry,
        range: Range<u64>,
    ) -> Result<LogicalEntryRange, MountError> {
        let started = Instant::now();
        let mut file = self.source.open()?;
        let read = read_entry_range(&mut file, entry, range, &self.data_path)?;
        self.observe(entry, read.bytes.len(), started);
        Ok(read)
    }

    fn observe(&self, entry: &BigEntry, bytes: usize, started: Instant) {
        if let Some(observer) = READ_OBSERVER.get() {
            observer(&ArchiveRead { archive: &self.data_path, entry, bytes, elapsed: started.elapsed() });
        }
    }
}

#[derive(Debug, Default)]
pub struct MountSet {
    archives: Vec<MountedBig>,
    /// Downloadable content's archives, searched after the game's own (the shipped packs hold
    /// no path the game's archives do): native `DLC::Manager` adds every `*.big` of its content
    /// mounts to the `dlcbig:` device (`EnumerateContent` `0x825e86c8`). Linking more while the
    /// game runs adds to them.
    dlc: RwLock<Vec<Arc<MountedBig>>>,
}

impl MountSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, archive: MountedBig) {
        self.archives.push(archive);
    }

    /// The game's own archives in mount order (not downloadable content's).
    #[must_use]
    pub fn archives(&self) -> &[MountedBig] {
        &self.archives
    }

    /// Adds a downloadable content archive, which later reads find after the game's archives
    /// and every earlier one's.
    pub fn push_dlc(&self, archive: MountedBig) {
        self.dlc.write().unwrap_or_else(PoisonError::into_inner).push(Arc::new(archive));
    }

    /// The downloadable content archives in mount order.
    #[must_use]
    pub fn dlc_archives(&self) -> Vec<Arc<MountedBig>> {
        self.dlc.read().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// The archive that owns a path: the game's first, then downloadable content's.
    fn owner<T>(&self, normalized: &str, mut found: impl FnMut(&MountedBig, &BigEntry) -> T) -> Option<T> {
        for mounted in &self.archives {
            if let Some(entry) = mounted.archive.find_path(normalized) {
                return Some(found(mounted, entry));
            }
        }
        let dlc = self.dlc.read().unwrap_or_else(PoisonError::into_inner);
        dlc.iter().find_map(|mounted| mounted.archive.find_path(normalized).map(|entry| found(mounted.as_ref(), entry)))
    }

    /// Whether a mounted archive holds the virtual game path
    /// (`xcore::FileLoader::FileExists` for archived files).
    #[must_use]
    pub fn contains_path(&self, path: &str) -> bool {
        self.owner(&normalize_game_path(path), |_, _| ()).is_some()
    }

    /// Resolves a virtual game path in mount order.
    ///
    /// # Errors
    ///
    /// Returns [`MountError::EntryNotFound`] when no mounted archive contains
    /// the path, or propagates a read/decompression failure from the owner.
    pub fn read_path(&self, path: &str) -> Result<Vec<u8>, MountError> {
        let normalized = normalize_game_path(path);
        self.owner(&normalized, |mounted, entry| mounted.read_entry(entry))
            .unwrap_or_else(|| Err(MountError::EntryNotFound(normalized.clone())))
    }

    /// Resolves a virtual path in mount order and reads a logical entry range.
    ///
    /// Once a matching archive is found, any read or decoding error from that
    /// owner is returned; later mounts are not considered.
    ///
    /// # Errors
    ///
    /// Returns [`MountError::EntryNotFound`] when no mounted archive contains
    /// the path, or propagates the selected archive's range/read failure.
    pub fn read_path_range(
        &self,
        path: &str,
        range: Range<u64>,
    ) -> Result<LogicalEntryRange, MountError> {
        let normalized = normalize_game_path(path);
        let mut range = Some(range);
        self.owner(&normalized, |mounted, entry| mounted.read_entry_range(entry, range.take().unwrap_or(0..0)))
            .unwrap_or_else(|| Err(MountError::EntryNotFound(normalized.clone())))
    }

    /// Resolves a virtual path in mount order and streams its entry, for an entry too large to
    /// read whole (a downloadable content movie).
    ///
    /// # Errors
    ///
    /// Returns [`MountError::EntryNotFound`] when no mounted archive contains the path, or the
    /// owner's read failure.
    pub fn open_stream(&self, path: &str) -> Result<EntryStream, MountError> {
        let normalized = normalize_game_path(path);
        self.owner(&normalized, |mounted, entry| {
            let failed = |source| MountError::Io { path: mounted.data_path.clone(), source };
            let mut reader = mounted.source.open()?;
            reader.seek(SeekFrom::Start(entry.offset)).map_err(failed)?;
            let probe = usize::try_from(entry.stored_size.min(2)).unwrap_or(0);
            let mut prefix = [0_u8; 2];
            reader.read_exact(&mut prefix[..probe]).map_err(failed)?;
            if is_refpack(&prefix[..probe]) {
                return mounted.read_entry(entry).map(|bytes| EntryStream::Unpacked(io::Cursor::new(bytes)));
            }
            reader.seek(SeekFrom::Start(entry.offset)).map_err(failed)?;
            Ok(EntryStream::Stored(reader.take(entry.stored_size)))
        })
        .unwrap_or_else(|| Err(MountError::EntryNotFound(normalized.clone())))
    }
}

#[derive(Debug)]
pub enum MountError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Big(BigError),
    RefPack(RefPackError),
    EntryNotFound(String),
    StoredSizeTooLarge(u64),
    UnpackedSizeMismatch {
        expected: u64,
        actual: usize,
    },
    ReversedLogicalRange {
        start: u64,
        end: u64,
    },
    LogicalRangeOutsideEntry {
        start: u64,
        end: u64,
        logical_len: u64,
    },
    LogicalRangeTooLarge(u64),
    ArchiveOffsetOverflow {
        entry_offset: u64,
        logical_offset: u64,
    },
}

impl fmt::Display for MountError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Big(source) => source.fmt(formatter),
            Self::RefPack(source) => source.fmt(formatter),
            Self::EntryNotFound(path) => write!(formatter, "BIG entry not found: {path}"),
            Self::StoredSizeTooLarge(size) => write!(formatter, "BIG entry is too large: {size}"),
            Self::UnpackedSizeMismatch { expected, actual } => {
                write!(
                    formatter,
                    "BIG entry expected {expected} unpacked bytes, got {actual}"
                )
            }
            Self::ReversedLogicalRange { start, end } => {
                write!(formatter, "BIG logical range is reversed: {start}..{end}")
            }
            Self::LogicalRangeOutsideEntry {
                start,
                end,
                logical_len,
            } => write!(
                formatter,
                "BIG logical range {start}..{end} is outside {logical_len} bytes"
            ),
            Self::LogicalRangeTooLarge(size) => {
                write!(formatter, "BIG logical range is too large: {size}")
            }
            Self::ArchiveOffsetOverflow {
                entry_offset,
                logical_offset,
            } => write!(
                formatter,
                "BIG archive offset overflow: {entry_offset} + {logical_offset}"
            ),
        }
    }
}

impl std::error::Error for MountError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Big(source) => Some(source),
            Self::RefPack(source) => Some(source),
            _ => None,
        }
    }
}

impl From<BigError> for MountError {
    fn from(value: BigError) -> Self {
        Self::Big(value)
    }
}

impl From<RefPackError> for MountError {
    fn from(value: RefPackError) -> Self {
        Self::RefPack(value)
    }
}

fn normalize_game_path(path: &str) -> String {
    path.replace('/', "\\")
}

fn open_file(path: &Path) -> Result<File, MountError> {
    File::open(path).map_err(|source| MountError::Io {
        path: path.to_owned(),
        source,
    })
}

fn read_archive_header(file: &mut impl Read, path: &Path) -> Result<BigArchive, MountError> {
    let mut fixed = [0_u8; FIXED_HEADER_SIZE];
    file.read_exact(&mut fixed)
        .map_err(|source| MountError::Io {
            path: path.to_owned(),
            source,
        })?;
    let header_size = usize::try_from(u32::from_be_bytes(fixed[12..16].try_into().unwrap()))
        .map_err(|_| BigError::IntegerOverflow)?;
    if header_size < FIXED_HEADER_SIZE {
        return Err(BigError::HeaderOutsideFile {
            declared: header_size,
            actual: FIXED_HEADER_SIZE,
        }
        .into());
    }
    let mut header = vec![0_u8; header_size];
    header[..FIXED_HEADER_SIZE].copy_from_slice(&fixed);
    file.read_exact(&mut header[FIXED_HEADER_SIZE..])
        .map_err(|source| MountError::Io {
            path: path.to_owned(),
            source,
        })?;
    BigArchive::parse(&header).map_err(Into::into)
}

fn read_entry(reader: &mut (impl Read + Seek), entry: &BigEntry) -> Result<Vec<u8>, MountError> {
    let stored_size = usize::try_from(entry.stored_size)
        .map_err(|_| MountError::StoredSizeTooLarge(entry.stored_size))?;
    reader
        .seek(SeekFrom::Start(entry.offset))
        .map_err(|source| MountError::Io {
            path: PathBuf::from("<archive payload>"),
            source,
        })?;
    let mut stored = vec![0_u8; stored_size];
    reader
        .read_exact(&mut stored)
        .map_err(|source| MountError::Io {
            path: PathBuf::from("<archive payload>"),
            source,
        })?;

    let unpacked = if is_refpack(&stored) {
        refpack::decompress(&stored)?
    } else {
        stored
    };
    if matches!(entry.key, BigEntryKey::Hash(_))
        && u64::try_from(unpacked.len()).ok() != Some(entry.unpacked_size)
    {
        return Err(MountError::UnpackedSizeMismatch {
            expected: entry.unpacked_size,
            actual: unpacked.len(),
        });
    }
    Ok(unpacked)
}

fn read_entry_range(
    reader: &mut (impl Read + Seek),
    entry: &BigEntry,
    range: Range<u64>,
    data_path: &Path,
) -> Result<LogicalEntryRange, MountError> {
    if range.start > range.end {
        return Err(MountError::ReversedLogicalRange {
            start: range.start,
            end: range.end,
        });
    }

    reader
        .seek(SeekFrom::Start(entry.offset))
        .map_err(|source| MountError::Io {
            path: data_path.to_owned(),
            source,
        })?;
    let prefix_len = usize::try_from(entry.stored_size.min(2))
        .map_err(|_| MountError::StoredSizeTooLarge(entry.stored_size))?;
    let mut prefix = [0_u8; 2];
    reader
        .read_exact(&mut prefix[..prefix_len])
        .map_err(|source| MountError::Io {
            path: data_path.to_owned(),
            source,
        })?;

    if is_refpack(&prefix[..prefix_len]) {
        let stored_size = usize::try_from(entry.stored_size)
            .map_err(|_| MountError::StoredSizeTooLarge(entry.stored_size))?;
        let mut stored = vec![0_u8; stored_size];
        stored[..prefix_len].copy_from_slice(&prefix[..prefix_len]);
        reader
            .read_exact(&mut stored[prefix_len..])
            .map_err(|source| MountError::Io {
                path: data_path.to_owned(),
                source,
            })?;
        let unpacked = refpack::decompress(&stored)?;
        validate_hashed_unpacked_size(entry, unpacked.len())?;
        let logical_len = u64::try_from(unpacked.len())
            .map_err(|_| MountError::LogicalRangeTooLarge(u64::MAX))?;
        let slice = checked_logical_slice(&unpacked, range, logical_len)?;
        return Ok(LogicalEntryRange {
            bytes: slice.to_vec(),
            logical_len,
            storage: LogicalEntryStorage::RefPack,
        });
    }

    if matches!(entry.key, BigEntryKey::Hash(_)) {
        let actual = usize::try_from(entry.stored_size)
            .map_err(|_| MountError::StoredSizeTooLarge(entry.stored_size))?;
        validate_hashed_unpacked_size(entry, actual)?;
    }
    validate_logical_range(&range, entry.stored_size)?;
    let physical_offset =
        entry
            .offset
            .checked_add(range.start)
            .ok_or(MountError::ArchiveOffsetOverflow {
                entry_offset: entry.offset,
                logical_offset: range.start,
            })?;
    entry
        .offset
        .checked_add(range.end)
        .ok_or(MountError::ArchiveOffsetOverflow {
            entry_offset: entry.offset,
            logical_offset: range.end,
        })?;
    reader
        .seek(SeekFrom::Start(physical_offset))
        .map_err(|source| MountError::Io {
            path: data_path.to_owned(),
            source,
        })?;
    let byte_len = range.end - range.start;
    let byte_len =
        usize::try_from(byte_len).map_err(|_| MountError::LogicalRangeTooLarge(byte_len))?;
    let mut bytes = vec![0_u8; byte_len];
    reader
        .read_exact(&mut bytes)
        .map_err(|source| MountError::Io {
            path: data_path.to_owned(),
            source,
        })?;
    Ok(LogicalEntryRange {
        bytes,
        logical_len: entry.stored_size,
        storage: LogicalEntryStorage::Raw,
    })
}

fn validate_hashed_unpacked_size(entry: &BigEntry, actual: usize) -> Result<(), MountError> {
    if matches!(entry.key, BigEntryKey::Hash(_))
        && u64::try_from(actual).ok() != Some(entry.unpacked_size)
    {
        return Err(MountError::UnpackedSizeMismatch {
            expected: entry.unpacked_size,
            actual,
        });
    }
    Ok(())
}

fn validate_logical_range(range: &Range<u64>, logical_len: u64) -> Result<(), MountError> {
    if range.end > logical_len {
        return Err(MountError::LogicalRangeOutsideEntry {
            start: range.start,
            end: range.end,
            logical_len,
        });
    }
    Ok(())
}

fn checked_logical_slice(
    bytes: &[u8],
    range: Range<u64>,
    logical_len: u64,
) -> Result<&[u8], MountError> {
    validate_logical_range(&range, logical_len)?;
    let start =
        usize::try_from(range.start).map_err(|_| MountError::LogicalRangeTooLarge(range.start))?;
    let end =
        usize::try_from(range.end).map_err(|_| MountError::LogicalRangeTooLarge(range.end))?;
    Ok(&bytes[start..end])
}

fn is_refpack(bytes: &[u8]) -> bool {
    bytes
        .get(..2)
        .is_some_and(|prefix| u16::from_be_bytes([prefix[0], prefix[1]]) & 0x3eff == 0x10fb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::big::BigFormat;
    use std::{
        io::Cursor,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEST_FILE: AtomicU64 = AtomicU64::new(0);

    struct TestFile {
        path: PathBuf,
    }

    impl TestFile {
        fn new(bytes: &[u8]) -> Self {
            let serial = NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "skate2-formats-mount-{}-{serial}.big",
                std::process::id()
            ));
            std::fs::write(&path, bytes).unwrap();
            Self { path }
        }
    }

    impl Drop for TestFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn entry(key: BigEntryKey, offset: u64, stored_size: u64, unpacked_size: u64) -> BigEntry {
        BigEntry {
            key,
            offset,
            stored_size,
            unpacked_size,
            header_index: 0,
        }
    }

    fn mounted_named(file: &TestFile, path: &str, stored_size: u64) -> MountedBig {
        MountedBig {
            data_path: file.path.clone(),
            header_path: file.path.clone(),
            source: ArchiveSource::File(file.path.clone()),
            archive: BigArchive {
                format: BigFormat::Named { integer_width: 4 },
                declared_archive_size: 0,
                header_size: u32::try_from(FIXED_HEADER_SIZE).unwrap(),
                entries: vec![entry(
                    BigEntryKey::Name(path.to_owned()),
                    0,
                    stored_size,
                    stored_size,
                )],
            },
        }
    }

    #[derive(Default)]
    struct RecordingCursor {
        inner: Cursor<Vec<u8>>,
        reads: Vec<(u64, usize)>,
        seeks: Vec<u64>,
    }

    impl RecordingCursor {
        fn new(bytes: Vec<u8>) -> Self {
            Self {
                inner: Cursor::new(bytes),
                ..Self::default()
            }
        }
    }

    impl Read for RecordingCursor {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.reads.push((self.inner.position(), buffer.len()));
            self.inner.read(buffer)
        }
    }

    impl Seek for RecordingCursor {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            let offset = self.inner.seek(position)?;
            self.seeks.push(offset);
            Ok(offset)
        }
    }

    #[derive(Default)]
    struct VirtualZeroReader {
        position: u64,
    }

    impl Read for VirtualZeroReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.position = self
                .position
                .checked_add(u64::try_from(buffer.len()).unwrap())
                .ok_or_else(|| io::Error::other("virtual position overflow"))?;
            buffer.fill(0);
            Ok(buffer.len())
        }
    }

    impl Seek for VirtualZeroReader {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            let SeekFrom::Start(offset) = position else {
                return Err(io::Error::other("unsupported virtual seek"));
            };
            self.position = offset;
            Ok(offset)
        }
    }

    #[test]
    fn reads_seeked_and_compressed_payload() {
        let stream = b"\x10\xfb\x00\x00\x06\xe0abcd\xfeef";
        let mut bytes = vec![0xaa; 23];
        bytes.extend_from_slice(stream);
        let entry = BigEntry {
            key: BigEntryKey::Hash(1),
            offset: 23,
            stored_size: stream.len() as u64,
            unpacked_size: 6,
            header_index: 0,
        };

        assert_eq!(
            read_entry(&mut Cursor::new(bytes), &entry).unwrap(),
            b"abcdef"
        );
    }

    #[test]
    fn raw_range_probes_entry_start_then_reads_only_requested_bytes() {
        let mut bytes = vec![0xaa; 11];
        bytes.extend_from_slice(b"raw-payload");
        let entry = entry(BigEntryKey::Name("raw".into()), 11, 11, 11);
        let mut reader = RecordingCursor::new(bytes);

        let result =
            read_entry_range(&mut reader, &entry, 4..8, Path::new("recording.big")).unwrap();

        assert_eq!(result.bytes, b"payl");
        assert_eq!(result.logical_len, 11);
        assert_eq!(result.storage, LogicalEntryStorage::Raw);
        assert_eq!(reader.reads, vec![(11, 2), (15, 4)]);
        assert_eq!(reader.seeks, vec![11, 15]);
    }

    #[test]
    fn empty_raw_range_is_valid_and_still_probes_true_entry_start() {
        let entry = entry(BigEntryKey::Name("raw".into()), 0, 3, 3);
        let mut reader = RecordingCursor::new(b"abc".to_vec());

        let result = read_entry_range(&mut reader, &entry, 3..3, Path::new("raw.big")).unwrap();

        assert!(result.bytes.is_empty());
        assert_eq!(result.logical_len, 3);
        assert_eq!(reader.reads, vec![(0, 2)]);
        assert_eq!(reader.seeks, vec![0, 3]);
    }

    #[test]
    fn refpack_range_expands_whole_entry_once_then_slices() {
        let stream = b"\x10\xfb\x00\x00\x06\xe0abcd\xfeef";
        let entry = entry(BigEntryKey::Hash(1), 0, stream.len() as u64, 6);
        let mut reader = RecordingCursor::new(stream.to_vec());

        let result =
            read_entry_range(&mut reader, &entry, 1..5, Path::new("compressed.big")).unwrap();

        assert_eq!(result.bytes, b"bcde");
        assert_eq!(result.logical_len, 6);
        assert_eq!(result.storage, LogicalEntryStorage::RefPack);
        assert_eq!(reader.reads, vec![(0, 2), (2, stream.len() - 2)]);
        assert_eq!(reader.seeks, vec![0]);
    }

    #[test]
    fn named_refpack_does_not_treat_stored_size_as_unpacked_size() {
        let stream = b"\x10\xfb\x00\x00\x06\xe0abcd\xfeef";
        let entry = entry(
            BigEntryKey::Name("named".into()),
            0,
            stream.len() as u64,
            stream.len() as u64,
        );

        let result = read_entry_range(
            &mut Cursor::new(stream),
            &entry,
            0..6,
            Path::new("named.big"),
        )
        .unwrap();

        assert_eq!(result.bytes, b"abcdef");
        assert_eq!(result.logical_len, 6);
        assert_eq!(result.storage, LogicalEntryStorage::RefPack);
    }

    #[test]
    fn refpack_range_is_checked_against_decoded_logical_length() {
        let stream = b"\x10\xfb\x00\x00\x06\xe0abcd\xfeef";
        let entry = entry(
            BigEntryKey::Name("named".into()),
            0,
            stream.len() as u64,
            stream.len() as u64,
        );

        assert!(matches!(
            read_entry_range(
                &mut Cursor::new(stream),
                &entry,
                5..7,
                Path::new("named.big")
            ),
            Err(MountError::LogicalRangeOutsideEntry {
                start: 5,
                end: 7,
                logical_len: 6
            })
        ));
    }

    #[test]
    fn hashed_entries_validate_raw_and_refpack_logical_sizes() {
        let raw = entry(BigEntryKey::Hash(1), 0, 3, 4);
        assert!(matches!(
            read_entry_range(&mut Cursor::new(b"abc"), &raw, 0..1, Path::new("raw.big")),
            Err(MountError::UnpackedSizeMismatch {
                expected: 4,
                actual: 3
            })
        ));

        let stream = b"\x10\xfb\x00\x00\x06\xe0abcd\xfeef";
        let compressed = entry(BigEntryKey::Hash(2), 0, stream.len() as u64, 7);
        assert!(matches!(
            read_entry_range(
                &mut Cursor::new(stream),
                &compressed,
                0..1,
                Path::new("compressed.big")
            ),
            Err(MountError::UnpackedSizeMismatch {
                expected: 7,
                actual: 6
            })
        ));
    }

    #[test]
    fn rejects_reversed_and_out_of_bounds_logical_ranges() {
        let entry = entry(BigEntryKey::Name("raw".into()), 0, 3, 3);
        assert!(matches!(
            read_entry_range(
                &mut Cursor::new(b"abc"),
                &entry,
                Range { start: 2, end: 1 },
                Path::new("raw.big")
            ),
            Err(MountError::ReversedLogicalRange { start: 2, end: 1 })
        ));
        assert!(matches!(
            read_entry_range(&mut Cursor::new(b"abc"), &entry, 2..4, Path::new("raw.big")),
            Err(MountError::LogicalRangeOutsideEntry {
                start: 2,
                end: 4,
                logical_len: 3
            })
        ));
    }

    #[test]
    fn rejects_archive_offset_overflow_after_raw_probe() {
        let start_overflow = entry(BigEntryKey::Name("raw".into()), u64::MAX - 2, 4, 4);
        assert!(matches!(
            read_entry_range(
                &mut VirtualZeroReader::default(),
                &start_overflow,
                3..4,
                Path::new("virtual.big")
            ),
            Err(MountError::ArchiveOffsetOverflow {
                entry_offset,
                logical_offset: 3
            }) if entry_offset == u64::MAX - 2
        ));

        let end_overflow = entry(BigEntryKey::Name("raw".into()), u64::MAX - 5, 8, 8);
        assert!(matches!(
            read_entry_range(
                &mut VirtualZeroReader::default(),
                &end_overflow,
                4..6,
                Path::new("virtual.big")
            ),
            Err(MountError::ArchiveOffsetOverflow {
                entry_offset,
                logical_offset: 6
            }) if entry_offset == u64::MAX - 5
        ));
    }

    #[test]
    fn mount_set_uses_normalized_first_matching_archive() {
        let first = TestFile::new(b"first");
        let second = TestFile::new(b"later");
        let mut mounts = MountSet::new();
        mounts.push(mounted_named(&first, "world\\item.xsf", 5));
        mounts.push(mounted_named(&second, "world\\item.xsf", 5));

        let result = mounts.read_path_range("WORLD/item.xsf", 1..4).unwrap();

        assert_eq!(result.bytes, b"irs");
        assert_eq!(result.storage, LogicalEntryStorage::Raw);
    }

    #[test]
    fn first_matching_archive_failure_does_not_fall_through() {
        let short = TestFile::new(b"ab");
        let valid = TestFile::new(b"valid");
        let mut mounts = MountSet::new();
        mounts.push(mounted_named(&short, "same.xsf", 5));
        mounts.push(mounted_named(&valid, "same.xsf", 5));

        assert!(matches!(
            mounts.read_path_range("same.xsf", 0..5),
            Err(MountError::Io { .. })
        ));
    }

    #[test]
    fn range_reports_missing_virtual_path() {
        let mounts = MountSet::new();
        assert!(matches!(
            mounts.read_path_range("missing.xsf", 0..0),
            Err(MountError::EntryNotFound(path)) if path == "missing.xsf"
        ));
    }

    #[test]
    fn normalizes_pc_separators_for_virtual_paths() {
        assert_eq!(
            normalize_game_path("data/config/input.cfg"),
            "data\\config\\input.cfg"
        );
    }

    #[test]
    fn mount_table_indices_are_stable() {
        for (expected, spec) in SKATE2_ARCHIVES.iter().enumerate() {
            assert_eq!(usize::from(spec.index), expected);
        }
        assert_eq!(SKATE2_ARCHIVES[4].group, MountGroup::Load);
        assert!(SKATE2_ARCHIVES[4].use_external_header);
    }
}
