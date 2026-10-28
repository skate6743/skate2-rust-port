//! Read-only Xbox 360 STFS content packages (`CON `, `LIVE`, `PIRS`): the header's title,
//! content type and localized names, the file table, and each file's bytes, which sit in 0x1000
//! byte data blocks with the package's hash tables between them.
//!
//! Header: the size at 0x340, the content type at 0x344, the title id at 0x360, the volume
//! descriptor at 0x379 (block separation +2, file table block count +3 and first block +5, both
//! little endian, allocated block count +0x1C), the display names at 0x411 and descriptions at
//! 0xD11 (18 locales of 0x80 bytes, UTF-16 big endian). Data blocks start at the header size
//! rounded up to 0x1000. Each run of 0xAA data blocks follows its level 0 hash table, each run of
//! 0x70E4 its level 1 table, and past the first 0x70E4 blocks a level 2 table; a read-only
//! package keeps one copy of each table, any other two.

use std::{
    fmt,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Arc,
};

/// Bytes in a data block and in a hash table.
pub const BLOCK: u64 = 0x1000;
/// Data blocks a level 0 and a level 1 hash table cover.
const LEVEL0_BLOCKS: u32 = 0xAA;
const LEVEL1_BLOCKS: u32 = 0x70E4;
/// A hash table entry: the block's SHA-1, its status byte and the next block of its file.
const HASH_ENTRY: usize = 0x18;
/// A file table entry.
const FILE_ENTRY: usize = 0x40;
/// Locales the header names the package in.
pub const LOCALES: usize = 18;
const NAME_BYTES: usize = 0x80;
const DISPLAY_NAMES: usize = 0x411;
const DESCRIPTIONS: usize = 0xD11;
const HEADER_FIELDS: usize = DESCRIPTIONS + LOCALES * NAME_BYTES;

#[derive(Debug)]
pub enum StfsError {
    Io { path: PathBuf, source: io::Error },
    /// The file is no `CON `, `LIVE` or `PIRS` package.
    NotPackage,
    /// The header or volume descriptor holds values no package has.
    BadHeader(&'static str),
    /// A file's blocks run outside the package or out of its chain.
    BadFile(String),
    /// A fragmented file in a package with two copies of each hash table, which this reader does
    /// not follow.
    Fragmented(String),
}

impl fmt::Display for StfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::NotPackage => f.write_str("not an Xbox 360 content package"),
            Self::BadHeader(what) => write!(f, "invalid content package header: {what}"),
            Self::BadFile(name) => write!(f, "content package file {name} lies outside the package"),
            Self::Fragmented(name) => write!(f, "content package file {name} is fragmented"),
        }
    }
}

impl std::error::Error for StfsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Where data block `n` sits: the first table's offset and how many copies each table has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    base: u64,
    /// 0 for one copy of each hash table (read-only), 1 for two.
    shift: u32,
}

impl Layout {
    /// The block in the package that holds data block `n`: the data blocks and every hash table
    /// before it.
    fn backing(self, n: u32) -> u64 {
        let n = u64::from(n);
        let (level0, level1) = (u64::from(LEVEL0_BLOCKS), u64::from(LEVEL1_BLOCKS));
        let block = (((n + level0) / level0) << self.shift) + n;
        if n < level0 {
            return block;
        }
        let block = block + (((n + level1) / level1) << self.shift);
        if n < level1 { block } else { block + (1 << self.shift) }
    }

    fn offset(self, n: u32) -> u64 {
        self.base + self.backing(n) * BLOCK
    }

    /// The level 0 hash table covering data block `n`: the block before its run's first.
    fn level0_table(self, n: u32) -> u64 {
        self.base + (self.backing(n - n % LEVEL0_BLOCKS) - (1 << self.shift)) * BLOCK
    }
}

/// One file table entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StfsEntry {
    pub name: String,
    pub directory: bool,
    /// The folder's file table index, -1 for the root.
    pub parent: i16,
    pub size: u64,
    first_block: u32,
    block_count: u32,
    contiguous: bool,
}

/// A file inside a package, as [`StfsPackage::file`] resolves it: the package, the file's size
/// and the data block holding each 0x1000 bytes of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StfsFile {
    package: PathBuf,
    name: String,
    size: u64,
    layout: Layout,
    blocks: Blocks,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Blocks {
    /// Consecutive data blocks from the first.
    Run(u32),
    Chain(Arc<[u32]>),
}

impl Blocks {
    fn get(&self, index: u32) -> u32 {
        match self {
            Self::Run(first) => first + index,
            Self::Chain(chain) => chain[index as usize],
        }
    }
}

impl StfsFile {
    #[must_use]
    pub fn package(&self) -> &Path {
        &self.package
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// A reader over the file's bytes.
    ///
    /// # Errors
    ///
    /// Returns the error opening the package.
    pub fn open(&self) -> io::Result<StfsReader> {
        Ok(StfsReader { file: File::open(&self.package)?, source: self.clone(), position: 0 })
    }
}

/// `Read` and `Seek` over one file of a package.
#[derive(Debug)]
pub struct StfsReader {
    file: File,
    source: StfsFile,
    position: u64,
}

impl Read for StfsReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self.source.size.saturating_sub(self.position);
        if left == 0 || buf.is_empty() {
            return Ok(0);
        }
        let index = u32::try_from(self.position / BLOCK).map_err(|_| io::ErrorKind::InvalidInput)?;
        let within = self.position % BLOCK;
        let block = self.source.blocks.get(index);
        // Consecutive data blocks stay adjacent in the package up to the end of their level 0
        // run, where the next run's hash table sits.
        let mut run = 1;
        if let Blocks::Run(_) = self.source.blocks {
            run = LEVEL0_BLOCKS - block % LEVEL0_BLOCKS;
        }
        let span = u64::from(run) * BLOCK - within;
        let take = usize::try_from(span.min(left).min(buf.len() as u64)).unwrap_or(buf.len());
        self.file.seek(SeekFrom::Start(self.source.layout.offset(block) + within))?;
        self.file.read_exact(&mut buf[..take])?;
        self.position += take as u64;
        Ok(take)
    }
}

impl Seek for StfsReader {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let position = match to {
            SeekFrom::Start(at) => Some(at),
            SeekFrom::End(by) => self.source.size.checked_add_signed(by),
            SeekFrom::Current(by) => self.position.checked_add_signed(by),
        }
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        self.position = position;
        Ok(position)
    }
}

/// A package's header and file table.
#[derive(Clone, Debug)]
pub struct StfsPackage {
    path: PathBuf,
    pub magic: [u8; 4],
    pub content_type: u32,
    pub title_id: u32,
    /// The display name in each locale (English first); empty where the header has none.
    pub display_names: Vec<String>,
    pub descriptions: Vec<String>,
    pub entries: Vec<StfsEntry>,
    layout: Layout,
    allocated: u32,
}

fn be32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn le24(bytes: &[u8], at: usize) -> u32 {
    u32::from(bytes[at]) | u32::from(bytes[at + 1]) << 8 | u32::from(bytes[at + 2]) << 16
}

fn utf16_name(bytes: &[u8]) -> String {
    let units: Vec<u16> =
        bytes.chunks_exact(2).map(|pair| u16::from_be_bytes([pair[0], pair[1]])).take_while(|&unit| unit != 0).collect();
    String::from_utf16_lossy(&units).trim().to_owned()
}

impl StfsPackage {
    /// Whether the bytes start a content package.
    #[must_use]
    pub fn is_package(magic: &[u8]) -> bool {
        matches!(magic.get(..4), Some(b"CON " | b"LIVE" | b"PIRS"))
    }

    /// Reads the header and file table.
    ///
    /// # Errors
    ///
    /// Returns [`StfsError`] when the file cannot be read, is no package, or its header or file
    /// table does not describe one.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StfsError> {
        let path = path.as_ref().to_owned();
        let io = |source| StfsError::Io { path: path.clone(), source };
        let mut file = File::open(&path).map_err(io)?;
        let length = file.metadata().map_err(io)?.len();
        let mut header = vec![0_u8; HEADER_FIELDS];
        file.read_exact(&mut header).map_err(|source| match source.kind() {
            io::ErrorKind::UnexpectedEof => StfsError::NotPackage,
            _ => io(source),
        })?;
        if !Self::is_package(&header) {
            return Err(StfsError::NotPackage);
        }
        let header_size = u64::from(be32(&header, 0x340));
        let descriptor = 0x379;
        if header[descriptor] != 0x24 || header_size < HEADER_FIELDS as u64 {
            return Err(StfsError::BadHeader("volume descriptor"));
        }
        let separation = header[descriptor + 2];
        let layout = Layout { base: (header_size + BLOCK - 1) & !(BLOCK - 1), shift: u32::from(separation & 1 == 0) };
        let table_blocks = u32::from(u16::from_le_bytes([header[descriptor + 3], header[descriptor + 4]]));
        let table_first = le24(&header, descriptor + 5);
        let allocated = be32(&header, descriptor + 0x1C);
        if table_blocks == 0 || table_first >= allocated.max(1) {
            return Err(StfsError::BadHeader("file table"));
        }
        let locale = |start: usize| -> Vec<String> {
            (0..LOCALES).map(|i| utf16_name(&header[start + i * NAME_BYTES..start + (i + 1) * NAME_BYTES])).collect()
        };
        let mut package = Self {
            path: path.clone(),
            magic: [header[0], header[1], header[2], header[3]],
            content_type: be32(&header, 0x344),
            title_id: be32(&header, 0x360),
            display_names: locale(DISPLAY_NAMES),
            descriptions: locale(DESCRIPTIONS),
            entries: Vec::new(),
            layout,
            allocated,
        };
        let table = package.chain(&mut file, length, "file table", table_first, table_blocks, layout.shift != 0)?;
        let mut block = vec![0_u8; BLOCK as usize];
        for number in table.iter() {
            file.seek(SeekFrom::Start(layout.offset(*number))).map_err(io)?;
            file.read_exact(&mut block).map_err(io)?;
            for entry in block.chunks_exact(FILE_ENTRY) {
                let flags = entry[0x28];
                let length = usize::from(flags & 0x3F);
                if length == 0 {
                    continue;
                }
                package.entries.push(StfsEntry {
                    name: entry[..length.min(0x28)].iter().map(|&byte| char::from(byte)).collect(),
                    directory: flags & 0x80 != 0,
                    parent: i16::from_be_bytes([entry[0x32], entry[0x33]]),
                    size: u64::from(be32(entry, 0x34)),
                    first_block: le24(entry, 0x2F),
                    block_count: le24(entry, 0x29),
                    contiguous: flags & 0x40 != 0,
                });
            }
        }
        Ok(package)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The display name in the first locale that has one.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.display_names.iter().find(|name| !name.is_empty()).map_or("", String::as_str)
    }

    #[must_use]
    pub fn description(&self) -> &str {
        self.descriptions.iter().find(|text| !text.is_empty()).map_or("", String::as_str)
    }

    /// The files in the package's root folder.
    pub fn root_files(&self) -> impl Iterator<Item = &StfsEntry> {
        self.entries.iter().filter(|entry| !entry.directory && entry.parent == -1)
    }

    /// Resolves a file's blocks.
    ///
    /// # Errors
    ///
    /// Returns [`StfsError`] when the file's blocks run outside the package, out of its chain,
    /// or are fragmented in a package keeping two copies of each hash table.
    pub fn file(&self, entry: &StfsEntry) -> Result<StfsFile, StfsError> {
        let blocks_needed = u32::try_from(entry.size.div_ceil(BLOCK)).map_err(|_| StfsError::BadFile(entry.name.clone()))?;
        if blocks_needed > entry.block_count {
            return Err(StfsError::BadFile(entry.name.clone()));
        }
        let blocks = if entry.contiguous || blocks_needed <= 1 {
            if blocks_needed != 0 && entry.first_block.checked_add(blocks_needed - 1).is_none_or(|last| last >= self.allocated) {
                return Err(StfsError::BadFile(entry.name.clone()));
            }
            Blocks::Run(entry.first_block)
        } else {
            let io = |source| StfsError::Io { path: self.path.clone(), source };
            let mut file = File::open(&self.path).map_err(io)?;
            let length = file.metadata().map_err(io)?.len();
            Blocks::Chain(self.chain(&mut file, length, &entry.name, entry.first_block, blocks_needed, false)?)
        };
        Ok(StfsFile { package: self.path.clone(), name: entry.name.clone(), size: entry.size, layout: self.layout, blocks })
    }

    /// The `count` data blocks of a chain from `first`, each the next block its level 0 hash
    /// entry names; `assume_run` takes them as consecutive where the tables come in pairs.
    fn chain(&self, file: &mut File, length: u64, name: &str, first: u32, count: u32, assume_run: bool) -> Result<Arc<[u32]>, StfsError> {
        let io = |source| StfsError::Io { path: self.path.clone(), source };
        let mut blocks = Vec::with_capacity(count as usize);
        let mut block = first;
        for i in 0..count {
            if block >= self.allocated || self.layout.offset(block) + BLOCK > length {
                return Err(StfsError::BadFile(name.to_owned()));
            }
            blocks.push(block);
            if i + 1 == count {
                break;
            }
            if assume_run {
                block += 1;
                continue;
            }
            if self.layout.shift != 0 {
                return Err(StfsError::Fragmented(name.to_owned()));
            }
            let at = self.layout.level0_table(block) + ((block % LEVEL0_BLOCKS) as usize * HASH_ENTRY) as u64;
            let mut entry = [0_u8; HASH_ENTRY];
            file.seek(SeekFrom::Start(at)).map_err(io)?;
            file.read_exact(&mut entry).map_err(io)?;
            block = u32::from(entry[0x15]) << 16 | u32::from(entry[0x16]) << 8 | u32::from(entry[0x17]);
        }
        Ok(blocks.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A read-only package: one hash table per level, the header 0xAD0E bytes as the shipped
    /// marketplace packages have.
    const READ_ONLY: Layout = Layout { base: 0xB000, shift: 0 };

    #[test]
    fn data_blocks_skip_every_hash_table_before_them() {
        // Run 0's level 0 table, then its 0xAA blocks.
        assert_eq!(READ_ONLY.backing(0), 1);
        assert_eq!(READ_ONLY.backing(0xA9), 0xAA);
        // The level 1 table and run 1's level 0 table.
        assert_eq!(READ_ONLY.backing(0xAA), 0xAD);
        assert_eq!(READ_ONLY.backing(0x153), 0x156);
        // Past the first level 1 run: the level 2 table and the next level 1 table.
        assert_eq!(READ_ONLY.backing(0x70E4), 0x70E4 + 0xAB + 3);
        let pairs = Layout { base: 0xB000, shift: 1 };
        assert_eq!(pairs.backing(0), 2);
        assert_eq!(pairs.backing(0xAA), 0xAA + 6);
    }

    #[test]
    fn a_runs_hash_table_is_the_block_before_its_first_data_block() {
        assert_eq!(READ_ONLY.level0_table(5), READ_ONLY.base);
        assert_eq!(READ_ONLY.level0_table(0xAA), READ_ONLY.base + (0xAD - 1) * BLOCK);
        assert_eq!(READ_ONLY.level0_table(0x153), READ_ONLY.level0_table(0xAA));
    }

    /// Builds a read-only package holding `files` (name, bytes) as contiguous files, each block
    /// also chained to the next through its hash entry.
    fn package(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let header_size = 0xAD0E_u32;
        let mut header = vec![0_u8; READ_ONLY.base as usize];
        header[..4].copy_from_slice(b"LIVE");
        header[0x340..0x344].copy_from_slice(&header_size.to_be_bytes());
        header[0x344..0x348].copy_from_slice(&2_u32.to_be_bytes());
        header[0x360..0x364].copy_from_slice(&0x4541_087F_u32.to_be_bytes());
        for (i, unit) in "Test Pack".encode_utf16().enumerate() {
            header[DISPLAY_NAMES + 2 * i..DISPLAY_NAMES + 2 * i + 2].copy_from_slice(&unit.to_be_bytes());
        }
        let descriptor = 0x379;
        header[descriptor] = 0x24;
        header[descriptor + 2] = 1;
        header[descriptor + 3] = 1;
        // Block 0 is the file table; the files follow.
        let mut blocks: Vec<Vec<u8>> = vec![vec![0; BLOCK as usize]];
        let mut table = vec![0_u8; BLOCK as usize];
        for (index, (name, bytes)) in files.iter().enumerate() {
            let first = u32::try_from(blocks.len()).unwrap();
            let count = u32::try_from(bytes.len().div_ceil(BLOCK as usize)).unwrap();
            for chunk in bytes.chunks(BLOCK as usize) {
                let mut block = chunk.to_vec();
                block.resize(BLOCK as usize, 0);
                blocks.push(block);
            }
            let entry = &mut table[index * FILE_ENTRY..(index + 1) * FILE_ENTRY];
            entry[..name.len()].copy_from_slice(name.as_bytes());
            entry[0x28] = u8::try_from(name.len()).unwrap() | 0x40;
            entry[0x29..0x2C].copy_from_slice(&count.to_le_bytes()[..3]);
            entry[0x2F..0x32].copy_from_slice(&first.to_le_bytes()[..3]);
            entry[0x32..0x34].copy_from_slice(&(-1_i16).to_be_bytes());
            entry[0x34..0x38].copy_from_slice(&u32::try_from(bytes.len()).unwrap().to_be_bytes());
        }
        blocks[0] = table;
        let allocated = u32::try_from(blocks.len()).unwrap();
        header[descriptor + 0x1C..descriptor + 0x20].copy_from_slice(&allocated.to_be_bytes());
        let mut out = header;
        let last = READ_ONLY.backing(allocated - 1);
        out.resize((READ_ONLY.base + (last + 1) * BLOCK) as usize, 0);
        for (n, block) in blocks.iter().enumerate() {
            let n = u32::try_from(n).unwrap();
            let at = READ_ONLY.offset(n) as usize;
            out[at..at + BLOCK as usize].copy_from_slice(block);
            let next = n + 1;
            let entry = READ_ONLY.level0_table(n) as usize + (n % LEVEL0_BLOCKS) as usize * HASH_ENTRY;
            out[entry + 0x15..entry + 0x18].copy_from_slice(&next.to_be_bytes()[1..]);
        }
        out
    }

    fn temp(bytes: &[u8], tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("skate2-formats-stfs-{}-{tag}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn a_files_bytes_read_back_across_hash_tables() {
        let big: Vec<u8> = (0..(0xAA + 3) * BLOCK as usize + 17).map(|i| (i * 7 % 251) as u8).collect();
        let path = temp(&package(&[("small.bh", b"hello".to_vec()), ("data.big", big.clone())]), "read");
        let package = StfsPackage::open(&path).unwrap();
        assert_eq!(package.title_id, 0x4541_087F);
        assert_eq!(package.content_type, 2);
        assert_eq!(package.display_name(), "Test Pack");
        let names: Vec<_> = package.root_files().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, ["small.bh", "data.big"]);
        let entry = package.root_files().find(|entry| entry.name == "data.big").unwrap();
        let mut read = Vec::new();
        package.file(entry).unwrap().open().unwrap().read_to_end(&mut read).unwrap();
        assert!(read == big);
        let mut reader = package.file(entry).unwrap().open().unwrap();
        let at = 0xAA * BLOCK - 3;
        reader.seek(SeekFrom::Start(at)).unwrap();
        let mut window = [0_u8; 6];
        reader.read_exact(&mut window).unwrap();
        assert_eq!(window, big[at as usize..at as usize + 6]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_fragmented_file_follows_its_hash_chain() {
        let bytes: Vec<u8> = (0..3 * BLOCK as usize).map(|i| (i % 13) as u8).collect();
        let mut raw = package(&[("frag.big", bytes.clone())]);
        // Unmark contiguous, then chain block 1 -> 3 -> 2 with the data moved to match.
        let table = READ_ONLY.offset(0) as usize;
        raw[table + 0x28] &= !0x40;
        let (b2, b3) = (READ_ONLY.offset(2) as usize, READ_ONLY.offset(3) as usize);
        let second: Vec<u8> = raw[b2..b2 + BLOCK as usize].to_vec();
        let third: Vec<u8> = raw[b3..b3 + BLOCK as usize].to_vec();
        raw[b3..b3 + BLOCK as usize].copy_from_slice(&second);
        raw[b2..b2 + BLOCK as usize].copy_from_slice(&third);
        let entry = |n: u32| READ_ONLY.level0_table(n) as usize + (n % LEVEL0_BLOCKS) as usize * HASH_ENTRY;
        raw[entry(1) + 0x15..entry(1) + 0x18].copy_from_slice(&3_u32.to_be_bytes()[1..]);
        raw[entry(3) + 0x15..entry(3) + 0x18].copy_from_slice(&2_u32.to_be_bytes()[1..]);
        let path = temp(&raw, "fragmented");
        let package = StfsPackage::open(&path).unwrap();
        let entry = package.root_files().next().unwrap();
        let mut read = Vec::new();
        package.file(entry).unwrap().open().unwrap().read_to_end(&mut read).unwrap();
        assert!(read == bytes);
        let _ = std::fs::remove_file(path);
    }

    /// A `BIGF` archive of `files`, their bytes after the table.
    fn bigf(files: &[(&str, &[u8])]) -> Vec<u8> {
        let table = 16 + files.iter().map(|(name, _)| 8 + name.len() + 1).sum::<usize>();
        let total = table + files.iter().map(|(_, bytes)| bytes.len()).sum::<usize>();
        let mut out = Vec::from(*b"BIGF");
        out.extend_from_slice(&u32::try_from(total).unwrap().to_le_bytes());
        out.extend_from_slice(&u32::try_from(files.len()).unwrap().to_be_bytes());
        out.extend_from_slice(&u32::try_from(table).unwrap().to_be_bytes());
        let mut data = Vec::new();
        for (name, bytes) in files {
            out.extend_from_slice(&u32::try_from(table + data.len()).unwrap().to_be_bytes());
            out.extend_from_slice(&u32::try_from(bytes.len()).unwrap().to_be_bytes());
            out.extend_from_slice(name.as_bytes());
            out.push(0);
            data.extend_from_slice(bytes);
        }
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn an_archive_inside_a_package_mounts_after_the_games_own() {
        use crate::mount::{ArchiveSource, MountSet, MountedBig};
        let game = temp(&bigf(&[("data\\a.txt", b"game")]), "game.big");
        let dlc = bigf(&[("data\\a.txt", b"dlc"), ("data\\unlocks\\x.unlock", b"GESTURES, gesture_spock")]);
        let path = temp(&package(&[("x_xbox2.big", dlc)]), "dlc");
        let package = StfsPackage::open(&path).unwrap();
        let entry = package.root_files().next().unwrap();
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open(&game, false).unwrap());
        mounts.push_dlc(MountedBig::open_source(ArchiveSource::Package(package.file(entry).unwrap())).unwrap());
        assert_eq!(mounts.read_path("data/a.txt").unwrap(), b"game");
        assert_eq!(mounts.read_path("DATA\\UNLOCKS\\X.UNLOCK").unwrap(), b"GESTURES, gesture_spock");
        assert!(mounts.contains_path("data\\unlocks\\x.unlock"));
        assert_eq!(mounts.archives().len(), 1);
        assert_eq!(mounts.dlc_archives().len(), 1);
        let _ = std::fs::remove_file(game);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn other_files_are_no_package() {
        let path = temp(b"BIGF\0\0\0\0", "big");
        assert!(matches!(StfsPackage::open(&path), Err(StfsError::NotPackage)));
        let _ = std::fs::remove_file(path);
    }

    /// The real packages, when `SKATE2_DLC_DIR` names a folder of them: every root file reads.
    #[test]
    #[ignore = "needs SKATE2_DLC_DIR"]
    fn shipped_packages_open() {
        let Some(dir) = std::env::var_os("SKATE2_DLC_DIR") else { return };
        for path in std::fs::read_dir(dir).unwrap().flatten().map(|entry| entry.path()) {
            let Ok(package) = StfsPackage::open(&path) else { continue };
            for entry in package.root_files() {
                let file = package.file(entry).unwrap();
                let mut reader = file.open().unwrap();
                let mut head = [0_u8; 4];
                reader.read_exact(&mut head).unwrap();
                reader.seek(SeekFrom::End(-1)).unwrap();
                let mut last = [0_u8; 1];
                reader.read_exact(&mut last).unwrap();
            }
        }
    }
}
