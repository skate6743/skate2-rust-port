//! EA BIG archive tables used by the reference build.

use std::{fmt, ops::Range};

const FIXED_HEADER_SIZE: usize = 16;
const HASHED_ENTRY_SIZE: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BigFormat {
    Named { integer_width: u8 },
    Hash16,
    /// Skate 3's `EB` archives (PC port: a linked Skate 3 data folder): a fixed 0x30-byte header,
    /// then 16-byte (or 20-byte) entry records, then fixed-width name records (a folder index and
    /// a file name) and the folder names. Entry offsets count blocks of 16 bytes (64 with flag
    /// `0x200`).
    Eb { version: u16, flags: u32 },
}

/// The fixed part of an `EB` archive header, which declares where the name table ends.
pub const EB_FIXED_HEADER_SIZE: usize = 0x30;

/// The size of an `EB` archive's whole header (fixed part, entries, names and folders) from its
/// fixed part, or `None` when `fixed` is not an `EB` header.
#[must_use]
pub fn eb_header_size(fixed: &[u8]) -> Option<usize> {
    if fixed.get(..2)? != b"EB" {
        return None;
    }
    let name_offset = usize::try_from(read_be_u32(fixed, 12).ok()?).ok()?;
    let name_size = usize::try_from(read_be_u32(fixed, 16).ok()?).ok()?;
    name_offset.checked_add(name_size)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BigEntryKey {
    Name(String),
    Hash(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BigEntry {
    pub key: BigEntryKey,
    pub offset: u64,
    pub stored_size: u64,
    pub unpacked_size: u64,
    pub header_index: u32,
}

impl BigEntry {
    /// Returns the byte range occupied in the `.big` data file.
    ///
    /// # Errors
    ///
    /// Returns [`BigError::IntegerOverflow`] if the end offset is not
    /// representable as `usize`.
    pub fn data_range(&self) -> Result<Range<usize>, BigError> {
        let start = usize::try_from(self.offset).map_err(|_| BigError::IntegerOverflow)?;
        let length = usize::try_from(self.stored_size).map_err(|_| BigError::IntegerOverflow)?;
        let end = start.checked_add(length).ok_or(BigError::IntegerOverflow)?;
        Ok(start..end)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BigArchive {
    pub format: BigFormat,
    pub declared_archive_size: u32,
    pub header_size: u32,
    pub entries: Vec<BigEntry>,
}

impl BigArchive {
    /// Parses either a named `BIG*` table or a `Viv4` hashed table.
    ///
    /// A `Viv4` table is commonly stored in a separate `.bh` file while entry
    /// offsets refer to its corresponding `.big` data file.
    ///
    /// # Errors
    ///
    /// Returns [`BigError`] when the magic, bounds, widths, or entry records are
    /// invalid.
    pub fn parse(bytes: &[u8]) -> Result<Self, BigError> {
        let header = bytes
            .get(..FIXED_HEADER_SIZE)
            .ok_or(BigError::TruncatedHeader)?;
        match &header[0..4] {
            magic if &magic[0..2] == b"EB" => parse_eb(bytes),
            b"Viv4" => parse_hash16(bytes),
            magic if &magic[0..3] == b"BIG" => parse_named(bytes),
            magic => Err(BigError::UnknownMagic([
                magic[0], magic[1], magic[2], magic[3],
            ])),
        }
    }

    #[must_use]
    pub fn find_name(&self, name: &str) -> Option<&BigEntry> {
        self.entries.iter().find(|entry| {
            matches!(&entry.key, BigEntryKey::Name(candidate) if candidate.eq_ignore_ascii_case(name))
        })
    }

    #[must_use]
    pub fn find_hash(&self, hash: u32) -> Option<&BigEntry> {
        self.entries
            .iter()
            .find(|entry| matches!(entry.key, BigEntryKey::Hash(candidate) if candidate == hash))
    }

    #[must_use]
    pub fn find_hashed_name(&self, name: &str) -> Option<&BigEntry> {
        self.find_hash(djb2(name.as_bytes()))
    }

    /// Finds an entry using the lookup scheme encoded by this archive.
    #[must_use]
    pub fn find_path(&self, path: &str) -> Option<&BigEntry> {
        match self.format {
            BigFormat::Named { .. } | BigFormat::Eb { .. } => self.find_name(path),
            BigFormat::Hash16 => self.find_hashed_name(path),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BigError {
    TruncatedHeader,
    UnknownMagic([u8; 4]),
    UnsupportedNamedWidth(u8),
    HeaderOutsideFile { declared: usize, actual: usize },
    EntryTableOverflow,
    UnterminatedName { entry: u32 },
    IntegerOverflow,
}

impl fmt::Display for BigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid EA BIG archive: {self:?}")
    }
}

impl std::error::Error for BigError {}

/// Original `rw::core::Big::Locate` DJB2 name hash at `0x82bb8680`.
#[must_use]
pub fn djb2(name: &[u8]) -> u32 {
    name.iter().fold(5381_u32, |hash, byte| {
        hash.wrapping_mul(33).wrapping_add(u32::from(*byte))
    })
}

fn parse_named(bytes: &[u8]) -> Result<BigArchive, BigError> {
    let variant = bytes[3];
    let integer_width = match variant {
        b'F' | b'4' => 4,
        b'1'..=b'9' => variant - b'0',
        _ => return Err(BigError::UnsupportedNamedWidth(variant)),
    };
    let count = read_be_u32(bytes, 8)?;
    let header_size = read_be_u32(bytes, 12)?;
    let header_end = checked_header_end(bytes, header_size)?;
    let declared_archive_size = read_le_u32(bytes, 4)?;
    let mut cursor = FIXED_HEADER_SIZE;
    let mut entries = Vec::with_capacity(count as usize);

    for header_index in 0..count {
        let offset = read_be_width(bytes, cursor, integer_width)?;
        cursor += usize::from(integer_width);
        let stored_size = read_be_width(bytes, cursor, integer_width.min(4))?;
        cursor += usize::from(integer_width.min(4));
        if cursor >= header_end {
            return Err(BigError::EntryTableOverflow);
        }
        let relative_end = bytes[cursor..header_end]
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(BigError::UnterminatedName {
                entry: header_index,
            })?;
        let name_end = cursor + relative_end;
        let name = String::from_utf8_lossy(&bytes[cursor..name_end]).into_owned();
        cursor = name_end + 1;
        entries.push(BigEntry {
            key: BigEntryKey::Name(name),
            offset,
            stored_size,
            unpacked_size: stored_size,
            header_index,
        });
    }

    Ok(BigArchive {
        format: BigFormat::Named { integer_width },
        declared_archive_size,
        header_size,
        entries,
    })
}

fn parse_hash16(bytes: &[u8]) -> Result<BigArchive, BigError> {
    let count = read_be_u32(bytes, 8)?;
    let header_size = read_be_u32(bytes, 12)?;
    let header_end = checked_header_end(bytes, header_size)?;
    let table_size = usize::try_from(count)
        .ok()
        .and_then(|value| value.checked_mul(HASHED_ENTRY_SIZE))
        .and_then(|value| value.checked_add(FIXED_HEADER_SIZE))
        .ok_or(BigError::IntegerOverflow)?;
    if table_size > header_end {
        return Err(BigError::EntryTableOverflow);
    }

    let mut entries = Vec::with_capacity(count as usize);
    for header_index in 0..count {
        let cursor = FIXED_HEADER_SIZE + header_index as usize * HASHED_ENTRY_SIZE;
        let offset = u64::from(read_be_u32(bytes, cursor)?);
        let stored_size = u64::from(read_be_u32(bytes, cursor + 4)?);
        let unpacked_field = u64::from(read_be_u32(bytes, cursor + 8)?);
        let hash = read_be_u32(bytes, cursor + 12)?;
        entries.push(BigEntry {
            key: BigEntryKey::Hash(hash),
            offset,
            stored_size,
            unpacked_size: if unpacked_field == 0 {
                stored_size
            } else {
                unpacked_field
            },
            header_index,
        });
    }
    entries.sort_by_key(|entry| match entry.key {
        BigEntryKey::Hash(hash) => hash,
        BigEntryKey::Name(_) => unreachable!(),
    });

    Ok(BigArchive {
        format: BigFormat::Hash16,
        // This field is not a conventional archive size in Viv4. Preserve its
        // raw little-endian value for evidence/debugging.
        declared_archive_size: read_le_u32(bytes, 4)?,
        header_size,
        entries,
    })
}

/// Skate 3's `EB` table. Names are kept as `folder\file` with backslashes, the spelling the
/// game's virtual paths use elsewhere in this crate.
fn parse_eb(bytes: &[u8]) -> Result<BigArchive, BigError> {
    let fixed = bytes.get(..EB_FIXED_HEADER_SIZE).ok_or(BigError::TruncatedHeader)?;
    let version = u16::from_be_bytes([fixed[2], fixed[3]]);
    let count = read_be_u32(fixed, 4)?;
    let flags = read_be_u32(fixed, 8)?;
    let name_offset = usize::try_from(read_be_u32(fixed, 12)?).map_err(|_| BigError::IntegerOverflow)?;
    let name_size = usize::try_from(read_be_u32(fixed, 16)?).map_err(|_| BigError::IntegerOverflow)?;
    let name_width = usize::from(fixed[20]);
    let folder_width = usize::from(fixed[21]);
    let folder_count = usize::from(u16::from_be_bytes([fixed[22], fixed[23]]));
    let header_end = name_offset.checked_add(name_size).ok_or(BigError::IntegerOverflow)?;
    if header_end > bytes.len() {
        return Err(BigError::HeaderOutsideFile { declared: header_end, actual: bytes.len() });
    }
    if name_width < 2 {
        return Err(BigError::EntryTableOverflow);
    }
    let entry_size = if flags & 0x1_0000 != 0 { 0x14 } else { 0x10 };
    let block = if flags & 0x200 != 0 { 0x40 } else { 0x10 };
    let count_usize = usize::try_from(count).map_err(|_| BigError::IntegerOverflow)?;
    let names_end = count_usize
        .checked_mul(name_width)
        .and_then(|size| size.checked_add(name_offset))
        .ok_or(BigError::IntegerOverflow)?;
    let folder_offset = names_end.checked_add(0xF).ok_or(BigError::IntegerOverflow)? & !0xF;
    let mut folders = Vec::with_capacity(folder_count);
    for index in 0..folder_count {
        let start = folder_offset + index * folder_width;
        let field = bytes.get(start..start + folder_width).ok_or(BigError::EntryTableOverflow)?;
        let end = field.iter().position(|byte| *byte == 0).unwrap_or(field.len());
        folders.push(String::from_utf8_lossy(&field[..end]).replace('/', "\\"));
    }
    let mut entries = Vec::with_capacity(count_usize);
    for header_index in 0..count {
        let index = header_index as usize;
        let cursor = EB_FIXED_HEADER_SIZE + index * entry_size;
        let offset = u64::from(read_be_u32(bytes, cursor)?) * block;
        let stored = u64::from(read_be_u32(bytes, cursor + 4)?);
        let unpacked = u64::from(read_be_u32(bytes, cursor + 8)?);
        let record_start = name_offset + index * name_width;
        let record = bytes.get(record_start..record_start + name_width).ok_or(BigError::EntryTableOverflow)?;
        let folder = usize::from(u16::from_be_bytes([record[0], record[1]]));
        let name_field = &record[2..];
        let end = name_field.iter().position(|byte| *byte == 0).unwrap_or(name_field.len());
        let name = String::from_utf8_lossy(&name_field[..end]).replace('/', "\\");
        let path = match folders.get(folder) {
            Some(folder) if !folder.is_empty() => format!("{folder}\\{name}"),
            _ => name,
        };
        entries.push(BigEntry {
            key: BigEntryKey::Name(path),
            offset,
            stored_size: if stored == 0 { unpacked } else { stored },
            unpacked_size: unpacked,
            header_index,
        });
    }
    Ok(BigArchive {
        format: BigFormat::Eb { version, flags },
        declared_archive_size: u32::try_from(u64::from_be_bytes(fixed[24..32].try_into().expect("8 bytes"))).unwrap_or(u32::MAX),
        header_size: u32::try_from(header_end).map_err(|_| BigError::IntegerOverflow)?,
        entries,
    })
}

fn checked_header_end(bytes: &[u8], header_size: u32) -> Result<usize, BigError> {
    let declared = usize::try_from(header_size).map_err(|_| BigError::IntegerOverflow)?;
    if declared < FIXED_HEADER_SIZE || declared > bytes.len() {
        return Err(BigError::HeaderOutsideFile {
            declared,
            actual: bytes.len(),
        });
    }
    Ok(declared)
}

fn read_be_width(bytes: &[u8], offset: usize, width: u8) -> Result<u64, BigError> {
    if width == 0 || width > 8 {
        return Err(BigError::UnsupportedNamedWidth(width));
    }
    let end = offset
        .checked_add(usize::from(width))
        .ok_or(BigError::IntegerOverflow)?;
    let field = bytes.get(offset..end).ok_or(BigError::EntryTableOverflow)?;
    Ok(field
        .iter()
        .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte)))
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32, BigError> {
    let field = bytes
        .get(offset..offset + 4)
        .ok_or(BigError::TruncatedHeader)?;
    Ok(u32::from_be_bytes(field.try_into().unwrap()))
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Result<u32, BigError> {
    let field = bytes
        .get(offset..offset + 4)
        .ok_or(BigError::TruncatedHeader)?;
    Ok(u32::from_le_bytes(field.try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_big4_entries() {
        let mut bytes = Vec::from(*b"BIG4");
        bytes.extend_from_slice(&44_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u32.to_be_bytes());
        bytes.extend_from_slice(&44_u32.to_be_bytes());
        bytes.extend_from_slice(&32_u32.to_be_bytes());
        bytes.extend_from_slice(&3_u32.to_be_bytes());
        bytes.extend_from_slice(b"a.bin\0");
        bytes.extend_from_slice(&35_u32.to_be_bytes());
        bytes.extend_from_slice(&7_u32.to_be_bytes());
        bytes.extend_from_slice(b"B.BIN\0");

        let archive = BigArchive::parse(&bytes).unwrap();

        assert_eq!(archive.format, BigFormat::Named { integer_width: 4 });
        assert_eq!(archive.entries.len(), 2);
        assert_eq!(archive.find_name("b.bin").unwrap().offset, 35);
        assert_eq!(archive.find_name("A.BIN").unwrap().stored_size, 3);
    }

    #[test]
    fn parses_and_finds_viv4_hash_entries() {
        let name = "data\\config\\input.cfg";
        let mut bytes = Vec::from(*b"Viv4");
        bytes.extend_from_slice(&32_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.extend_from_slice(&32_u32.to_be_bytes());
        bytes.extend_from_slice(&0x1540_u32.to_be_bytes());
        bytes.extend_from_slice(&0x0bc9_u32.to_be_bytes());
        bytes.extend_from_slice(&0x2789_u32.to_be_bytes());
        bytes.extend_from_slice(&djb2(name.as_bytes()).to_be_bytes());

        let archive = BigArchive::parse(&bytes).unwrap();
        let entry = archive.find_hashed_name(name).unwrap();

        assert_eq!(archive.format, BigFormat::Hash16);
        assert_eq!(entry.offset, 0x1540);
        assert_eq!(entry.stored_size, 0x0bc9);
        assert_eq!(entry.unpacked_size, 0x2789);
    }

    #[test]
    fn djb2_matches_reference_miscboot_entry() {
        assert_eq!(djb2(b"data\\config\\input.cfg"), 0x5efc_bd5b);
    }
}
