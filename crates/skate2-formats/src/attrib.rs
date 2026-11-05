//! `AttribSys` VLT chunk envelopes.

use std::{collections::BTreeMap, fmt, ops::Range};

pub const VERSION_CHUNK: [u8; 4] = *b"Vers";
pub const STRING_CHUNK: [u8; 4] = *b"StrN";
pub const POINTER_CHUNK: [u8; 4] = *b"PtrN";
pub const DEPENDENCY_CHUNK: [u8; 4] = *b"DepN";
pub const EXPORT_CHUNK: [u8; 4] = *b"ExpN";
pub const DATA_CHUNK: [u8; 4] = *b"DatN";
pub const EXPORTED_STRING_HEADER: [u8; 4] = *b"StrE";
pub const ATTRIBUTE_STRING_HASH_SEED: u64 = 0xABCD_EF00_1122_3344;
pub const DATABASE_LOAD_TYPE_ID: u64 = 0x0B38_8468_45E9_C175;
pub const CLASS_LOAD_TYPE_ID: u64 = 0x2A78_95AC_4A87_6152;
pub const COLLECTION_LOAD_TYPE_ID: u64 = 0xAD30_3B8F_42B3_307E;
pub const INLINE_COLLECTION_VALUE_FLAG: u8 = 0x40;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeChunkFile {
    pub chunks: Vec<AttributeChunk>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeChunk {
    pub tag: [u8; 4],
    pub offset: usize,
    pub size: usize,
    pub payload: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeVaultMetadata {
    pub version: u64,
    pub dependencies: Vec<AttributeDependency>,
    pub exports: Vec<AttributeExport>,
    pub pointer_fixups: Vec<AttributePointerFixup>,
    pub pointer_terminator: AttributePointerTerminator,
    pub data: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeDependency {
    pub id: u64,
    pub name: String,
    pub is_asset: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeExport {
    pub object_id: u64,
    pub type_id: u64,
    pub data_size: u32,
    pub data_offset: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttributeExportKind {
    Database,
    Class,
    Collection,
    Unknown(u64),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum AttributePointerKind {
    Null = 1,
    SelectTargetBase = 2,
    Direct = 3,
    Export = 4,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributePointerFixup {
    pub target_delta: u32,
    pub kind: AttributePointerKind,
    pub dependency_index: u16,
    pub argument: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributePointerTerminator {
    pub target_delta: u32,
    pub encoded: u32,
    pub padding: [u8; 16],
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AttributePointerLocation {
    pub dependency_index: u16,
    pub offset: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttributePointerValue {
    Null,
    Direct {
        dependency_index: u16,
        offset: u64,
    },
    Export {
        dependency_index: u16,
        object_id: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeDatabaseLoadData {
    pub class_count: u32,
    pub default_data_size: u32,
    pub type_count: u32,
    pub type_names_pointer: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeClassLoadData {
    pub class_id: u64,
    pub collection_reserve: u32,
    pub definition_count: u32,
    pub definitions_pointer: u32,
    pub static_size: u32,
    pub static_data_pointer: u32,
    pub layout_size: u32,
    pub layout_key_shift: u16,
    pub layout_count: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeCollectionLoadData {
    pub collection_id: u64,
    pub class_id: u64,
    pub parent_id: u64,
    pub table_reserve: u32,
    pub table_key_shift: u32,
    pub entry_count: u32,
    pub type_count: u16,
    pub types_length: u16,
    pub layout_pointer: u32,
    pub types: Vec<u64>,
    pub entries: Vec<AttributeCollectionEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeCollectionEntry {
    pub key: u64,
    pub data_pointer: u32,
    pub type_index: u16,
    pub node_flags: u8,
    pub entry_flags: u8,
}

impl AttributeCollectionEntry {
    /// Returns the serialized four-byte payload of an inline collection node.
    ///
    /// Flag `0x40` repurposes the node's pointer word as value storage. Since
    /// VLT data is big-endian, callers take the leading definition-sized
    /// portion of this word.
    #[must_use]
    pub fn inline_value_word(&self) -> Option<[u8; 4]> {
        (self.node_flags & INLINE_COLLECTION_VALUE_FLAG != 0)
            .then(|| self.data_pointer.to_be_bytes())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeDefinition {
    pub key: u64,
    pub type_id: u64,
    pub offset: u16,
    pub size: u16,
    pub maximum_count: u16,
    pub flags: u8,
    pub alignment: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeDataFile {
    pub string_table: Range<usize>,
    pub strings: Vec<AttributeString>,
    pub object_data: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeString {
    pub offset: usize,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttributeChunkError {
    TruncatedHeader {
        offset: usize,
    },
    InvalidSize {
        offset: usize,
        size: usize,
    },
    ChunkOutsideFile {
        offset: usize,
        size: usize,
        file_size: usize,
    },
    IntegerOverflow,
    MissingChunk {
        tag: [u8; 4],
    },
    DuplicateChunk {
        tag: [u8; 4],
    },
    InvalidChunkPayload {
        tag: [u8; 4],
    },
    InvalidUtf8DependencyName,
}

impl fmt::Display for AttributeChunkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid AttribSys chunk file: {self:?}")
    }
}

impl std::error::Error for AttributeChunkError {}

impl AttributeChunkFile {
    /// Parses the big-endian `(fourcc, size, payload)` envelope used by VLT
    /// metadata.
    ///
    /// # Errors
    ///
    /// Returns [`AttributeChunkError`] for truncated, undersized, overflowing,
    /// or out-of-range chunks.
    pub fn parse(bytes: &[u8]) -> Result<Self, AttributeChunkError> {
        let mut chunks = Vec::new();
        let mut offset = 0_usize;
        while offset < bytes.len() {
            let header = bytes
                .get(offset..offset + 8)
                .ok_or(AttributeChunkError::TruncatedHeader { offset })?;
            let tag = [header[0], header[1], header[2], header[3]];
            let size = usize::try_from(u32::from_be_bytes([
                header[4], header[5], header[6], header[7],
            ]))
            .map_err(|_| AttributeChunkError::IntegerOverflow)?;
            if size < 8 {
                return Err(AttributeChunkError::InvalidSize { offset, size });
            }
            let end = offset
                .checked_add(size)
                .ok_or(AttributeChunkError::IntegerOverflow)?;
            if end > bytes.len() {
                return Err(AttributeChunkError::ChunkOutsideFile {
                    offset,
                    size,
                    file_size: bytes.len(),
                });
            }
            chunks.push(AttributeChunk {
                tag,
                offset,
                size,
                payload: offset + 8..end,
            });
            offset = end;
        }
        Ok(Self { chunks })
    }

    fn required_chunk(&self, tag: [u8; 4]) -> Result<&AttributeChunk, AttributeChunkError> {
        let mut matches = self.chunks.iter().filter(|chunk| chunk.tag == tag);
        let chunk = matches
            .next()
            .ok_or(AttributeChunkError::MissingChunk { tag })?;
        if matches.next().is_some() {
            return Err(AttributeChunkError::DuplicateChunk { tag });
        }
        Ok(chunk)
    }
}

impl AttributeVaultMetadata {
    /// Parses the symbol-proven VLT node tables used to resolve sibling assets,
    /// initialize exported objects, and apply pointer fixups.
    ///
    /// # Errors
    ///
    /// Returns [`AttributeChunkError`] when required tables are absent or when
    /// any record, string, or object range falls outside its owning file.
    pub fn parse(bytes: &[u8]) -> Result<Self, AttributeChunkError> {
        let file = AttributeChunkFile::parse(bytes)?;
        let version_chunk = file.required_chunk(VERSION_CHUNK)?;
        let dependency_chunk = file.required_chunk(DEPENDENCY_CHUNK)?;
        let export_chunk = file.required_chunk(EXPORT_CHUNK)?;
        let pointer_chunk = file.required_chunk(POINTER_CHUNK)?;
        let data_chunk = file.required_chunk(DATA_CHUNK)?;

        let version = read_u64(bytes, version_chunk.payload.start, VERSION_CHUNK)?;
        let dependencies = parse_dependencies(bytes, dependency_chunk)?;
        let exports = parse_exports(bytes, export_chunk)?;

        let (pointer_fixups, pointer_terminator) = parse_pointer_fixups(bytes, pointer_chunk)?;

        for export in &exports {
            let start = usize::try_from(export.data_offset)
                .map_err(|_| AttributeChunkError::IntegerOverflow)?;
            let end = start
                .checked_add(
                    usize::try_from(export.data_size)
                        .map_err(|_| AttributeChunkError::IntegerOverflow)?,
                )
                .ok_or(AttributeChunkError::IntegerOverflow)?;
            if start < data_chunk.payload.start || end > data_chunk.payload.end {
                return Err(AttributeChunkError::InvalidChunkPayload { tag: EXPORT_CHUNK });
            }
        }

        Ok(Self {
            version,
            dependencies,
            exports,
            pointer_fixups,
            pointer_terminator,
            data: data_chunk.payload.clone(),
        })
    }

    /// Resolves the compact fixup stream into logical dependency-and-offset
    /// pointers without creating process addresses or mutating source buffers.
    /// Dependency zero is the VLT itself; external dependencies follow in the
    /// order returned by [`Self::dependencies`].
    ///
    /// # Errors
    ///
    /// Returns [`AttributeChunkError`] when a dependency, pointer slot, direct
    /// target, or duplicate fixup lies outside the supplied buffers.
    pub fn resolve_pointers(
        &self,
        dependency_sizes: &[usize],
    ) -> Result<BTreeMap<AttributePointerLocation, AttributePointerValue>, AttributeChunkError>
    {
        if dependency_sizes.len() != self.dependencies.len() + 1 {
            return Err(AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK });
        }
        let mut target_dependency = None;
        let mut pointers = BTreeMap::new();
        for fixup in &self.pointer_fixups {
            if fixup.kind == AttributePointerKind::SelectTargetBase {
                validate_dependency_index(fixup.dependency_index, dependency_sizes)?;
                target_dependency = Some(fixup.dependency_index);
                continue;
            }
            let dependency_index = target_dependency
                .ok_or(AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK })?;
            let target_size = dependency_sizes[usize::from(dependency_index)];
            let target_offset = usize::try_from(fixup.target_delta)
                .map_err(|_| AttributeChunkError::IntegerOverflow)?;
            if target_offset
                .checked_add(4)
                .is_none_or(|end| end > target_size)
            {
                return Err(AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK });
            }
            let value = match fixup.kind {
                AttributePointerKind::Null => AttributePointerValue::Null,
                AttributePointerKind::Direct => {
                    validate_dependency_index(fixup.dependency_index, dependency_sizes)?;
                    let pointee_size = dependency_sizes[usize::from(fixup.dependency_index)];
                    let pointee_offset = usize::try_from(fixup.argument)
                        .map_err(|_| AttributeChunkError::IntegerOverflow)?;
                    if pointee_offset >= pointee_size {
                        return Err(AttributeChunkError::InvalidChunkPayload {
                            tag: POINTER_CHUNK,
                        });
                    }
                    AttributePointerValue::Direct {
                        dependency_index: fixup.dependency_index,
                        offset: fixup.argument,
                    }
                }
                AttributePointerKind::Export => {
                    validate_dependency_index(fixup.dependency_index, dependency_sizes)?;
                    AttributePointerValue::Export {
                        dependency_index: fixup.dependency_index,
                        object_id: fixup.argument,
                    }
                }
                AttributePointerKind::SelectTargetBase => unreachable!(),
            };
            let location = AttributePointerLocation {
                dependency_index,
                offset: fixup.target_delta,
            };
            if pointers.insert(location, value).is_some() {
                return Err(AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK });
            }
        }
        Ok(pointers)
    }
}

fn validate_dependency_index(
    index: u16,
    dependency_sizes: &[usize],
) -> Result<(), AttributeChunkError> {
    if usize::from(index) >= dependency_sizes.len() {
        return Err(AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK });
    }
    Ok(())
}

impl AttributeExport {
    #[must_use]
    pub const fn kind(&self) -> AttributeExportKind {
        match self.type_id {
            DATABASE_LOAD_TYPE_ID => AttributeExportKind::Database,
            CLASS_LOAD_TYPE_ID => AttributeExportKind::Class,
            COLLECTION_LOAD_TYPE_ID => AttributeExportKind::Collection,
            unknown => AttributeExportKind::Unknown(unknown),
        }
    }

    /// Decodes the fixed database header at the start of a database export.
    ///
    /// # Errors
    ///
    /// Returns [`AttributeChunkError`] if this is not a database export or its
    /// serialized range is too short.
    pub fn database_load_data(
        &self,
        bytes: &[u8],
    ) -> Result<AttributeDatabaseLoadData, AttributeChunkError> {
        let offset = self.typed_data_offset(bytes, AttributeExportKind::Database, 16)?;
        Ok(AttributeDatabaseLoadData {
            class_count: read_u32(bytes, offset, EXPORT_CHUNK)?,
            default_data_size: read_u32(bytes, offset + 4, EXPORT_CHUNK)?,
            type_count: read_u32(bytes, offset + 8, EXPORT_CHUNK)?,
            type_names_pointer: read_u32(bytes, offset + 12, EXPORT_CHUNK)?,
        })
    }

    /// Decodes the exact 40-byte `Attrib::ClassLoadData` layout from symbols.
    ///
    /// # Errors
    ///
    /// Returns [`AttributeChunkError`] if this is not a class export or its
    /// serialized range is not exactly 40 bytes.
    pub fn class_load_data(
        &self,
        bytes: &[u8],
    ) -> Result<AttributeClassLoadData, AttributeChunkError> {
        if self.data_size != 40 {
            return Err(AttributeChunkError::InvalidChunkPayload { tag: EXPORT_CHUNK });
        }
        let offset = self.typed_data_offset(bytes, AttributeExportKind::Class, 40)?;
        Ok(AttributeClassLoadData {
            class_id: read_u64(bytes, offset, EXPORT_CHUNK)?,
            collection_reserve: read_u32(bytes, offset + 8, EXPORT_CHUNK)?,
            definition_count: read_u32(bytes, offset + 12, EXPORT_CHUNK)?,
            definitions_pointer: read_u32(bytes, offset + 16, EXPORT_CHUNK)?,
            static_size: read_u32(bytes, offset + 20, EXPORT_CHUNK)?,
            static_data_pointer: read_u32(bytes, offset + 24, EXPORT_CHUNK)?,
            layout_size: read_u32(bytes, offset + 28, EXPORT_CHUNK)?,
            layout_key_shift: read_u16(bytes, offset + 32, EXPORT_CHUNK)?,
            layout_count: read_u16(bytes, offset + 34, EXPORT_CHUNK)?,
        })
    }

    /// Follows the class header's relocated definitions pointer and decodes the
    /// exact 24-byte `Attrib::Definition` array.
    ///
    /// # Errors
    ///
    /// Returns [`AttributeChunkError`] if the fixup is absent/non-direct or if
    /// the selected dependency cannot contain all declared definitions.
    pub fn class_definitions(
        &self,
        buffers: &[&[u8]],
        pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
    ) -> Result<Vec<AttributeDefinition>, AttributeChunkError> {
        let vlt = buffers
            .first()
            .ok_or(AttributeChunkError::InvalidChunkPayload { tag: EXPORT_CHUNK })?;
        let class = self.class_load_data(vlt)?;
        let pointer_slot = self
            .data_offset
            .checked_add(16)
            .ok_or(AttributeChunkError::IntegerOverflow)?;
        let (dependency_index, definitions_offset) = direct_pointer_target(
            pointers,
            AttributePointerLocation {
                dependency_index: 0,
                offset: pointer_slot,
            },
        )?;
        let definition_bytes = dependency_bytes(buffers, dependency_index)?;
        let start = usize::try_from(definitions_offset)
            .map_err(|_| AttributeChunkError::IntegerOverflow)?;
        let count = usize::try_from(class.definition_count)
            .map_err(|_| AttributeChunkError::IntegerOverflow)?;
        let end = start
            .checked_add(
                count
                    .checked_mul(24)
                    .ok_or(AttributeChunkError::IntegerOverflow)?,
            )
            .ok_or(AttributeChunkError::IntegerOverflow)?;
        if end > definition_bytes.len() {
            return Err(AttributeChunkError::InvalidChunkPayload { tag: EXPORT_CHUNK });
        }
        let mut definitions = Vec::with_capacity(count);
        for index in 0..count {
            let offset = start + index * 24;
            definitions.push(AttributeDefinition {
                key: read_u64(definition_bytes, offset, EXPORT_CHUNK)?,
                type_id: read_u64(definition_bytes, offset + 8, EXPORT_CHUNK)?,
                offset: read_u16(definition_bytes, offset + 16, EXPORT_CHUNK)?,
                size: read_u16(definition_bytes, offset + 18, EXPORT_CHUNK)?,
                maximum_count: read_u16(definition_bytes, offset + 20, EXPORT_CHUNK)?,
                flags: read_u8(definition_bytes, offset + 22, EXPORT_CHUNK)?,
                alignment: read_u8(definition_bytes, offset + 23, EXPORT_CHUNK)?,
            });
        }
        Ok(definitions)
    }

    /// Decodes the 48-byte `Attrib::CollectionLoadData` header. Variable entry
    /// storage, when present, follows that header in the same export.
    ///
    /// # Errors
    ///
    /// Returns [`AttributeChunkError`] if this is not a collection export or
    /// its serialized range is shorter than the symbol-proven header.
    pub fn collection_load_data(
        &self,
        bytes: &[u8],
    ) -> Result<AttributeCollectionLoadData, AttributeChunkError> {
        let offset = self.typed_data_offset(bytes, AttributeExportKind::Collection, 48)?;
        let entry_count = read_u32(bytes, offset + 32, EXPORT_CHUNK)?;
        let types_length = read_u16(bytes, offset + 38, EXPORT_CHUNK)?;
        let types_start = offset + 48;
        let entries_start = types_start
            .checked_add(usize::from(types_length) * 8)
            .ok_or(AttributeChunkError::IntegerOverflow)?;
        let expected_end = entries_start
            .checked_add(
                usize::try_from(entry_count)
                    .map_err(|_| AttributeChunkError::IntegerOverflow)?
                    .checked_mul(16)
                    .ok_or(AttributeChunkError::IntegerOverflow)?,
            )
            .ok_or(AttributeChunkError::IntegerOverflow)?;
        let export_end = offset
            .checked_add(
                usize::try_from(self.data_size)
                    .map_err(|_| AttributeChunkError::IntegerOverflow)?,
            )
            .ok_or(AttributeChunkError::IntegerOverflow)?;
        if expected_end != export_end {
            return Err(AttributeChunkError::InvalidChunkPayload { tag: EXPORT_CHUNK });
        }
        let mut types = Vec::with_capacity(usize::from(types_length));
        for index in 0..usize::from(types_length) {
            types.push(read_u64(bytes, types_start + index * 8, EXPORT_CHUNK)?);
        }
        let mut entries = Vec::with_capacity(
            usize::try_from(entry_count).map_err(|_| AttributeChunkError::IntegerOverflow)?,
        );
        for index in 0..entries.capacity() {
            let entry_offset = entries_start + index * 16;
            entries.push(AttributeCollectionEntry {
                key: read_u64(bytes, entry_offset, EXPORT_CHUNK)?,
                data_pointer: read_u32(bytes, entry_offset + 8, EXPORT_CHUNK)?,
                type_index: read_u16(bytes, entry_offset + 12, EXPORT_CHUNK)?,
                node_flags: read_u8(bytes, entry_offset + 14, EXPORT_CHUNK)?,
                entry_flags: read_u8(bytes, entry_offset + 15, EXPORT_CHUNK)?,
            });
        }
        Ok(AttributeCollectionLoadData {
            collection_id: read_u64(bytes, offset, EXPORT_CHUNK)?,
            class_id: read_u64(bytes, offset + 8, EXPORT_CHUNK)?,
            parent_id: read_u64(bytes, offset + 16, EXPORT_CHUNK)?,
            table_reserve: read_u32(bytes, offset + 24, EXPORT_CHUNK)?,
            table_key_shift: read_u32(bytes, offset + 28, EXPORT_CHUNK)?,
            entry_count,
            type_count: read_u16(bytes, offset + 36, EXPORT_CHUNK)?,
            types_length,
            layout_pointer: read_u32(bytes, offset + 40, EXPORT_CHUNK)?,
            types,
            entries,
        })
    }

    fn typed_data_offset(
        &self,
        bytes: &[u8],
        expected: AttributeExportKind,
        minimum_size: usize,
    ) -> Result<usize, AttributeChunkError> {
        if self.kind() != expected {
            return Err(AttributeChunkError::InvalidChunkPayload { tag: EXPORT_CHUNK });
        }
        let offset =
            usize::try_from(self.data_offset).map_err(|_| AttributeChunkError::IntegerOverflow)?;
        let size =
            usize::try_from(self.data_size).map_err(|_| AttributeChunkError::IntegerOverflow)?;
        let end = offset
            .checked_add(size)
            .ok_or(AttributeChunkError::IntegerOverflow)?;
        if size < minimum_size || end > bytes.len() {
            return Err(AttributeChunkError::InvalidChunkPayload { tag: EXPORT_CHUNK });
        }
        Ok(offset)
    }
}

fn direct_pointer_target(
    pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
    location: AttributePointerLocation,
) -> Result<(u16, u64), AttributeChunkError> {
    match pointers.get(&location) {
        Some(AttributePointerValue::Direct {
            dependency_index,
            offset,
        }) => Ok((*dependency_index, *offset)),
        _ => Err(AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK }),
    }
}

fn dependency_bytes<'a>(buffers: &[&'a [u8]], index: u16) -> Result<&'a [u8], AttributeChunkError> {
    buffers
        .get(usize::from(index))
        .copied()
        .ok_or(AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK })
}

impl AttributeDataFile {
    /// Parses the sibling BIN payload. Its `StrE` header gives the absolute
    /// end of a NUL-separated string table; serialized reflected objects begin
    /// immediately at that offset and are not another chunk stream.
    ///
    /// # Errors
    ///
    /// Returns [`AttributeChunkError`] when the header or string-table boundary
    /// is invalid, or when a non-empty table entry is not UTF-8.
    pub fn parse(bytes: &[u8]) -> Result<Self, AttributeChunkError> {
        if bytes.get(..4) != Some(EXPORTED_STRING_HEADER.as_slice()) {
            return Err(AttributeChunkError::MissingChunk {
                tag: EXPORTED_STRING_HEADER,
            });
        }
        let table_end = usize::try_from(read_u32(bytes, 4, EXPORTED_STRING_HEADER)?)
            .map_err(|_| AttributeChunkError::IntegerOverflow)?;
        if !(8..=bytes.len()).contains(&table_end) {
            return Err(AttributeChunkError::InvalidChunkPayload {
                tag: EXPORTED_STRING_HEADER,
            });
        }

        let mut strings = Vec::new();
        let mut start = 8;
        while start < table_end {
            let tail = &bytes[start..table_end];
            let length = tail.iter().position(|byte| *byte == 0).ok_or(
                AttributeChunkError::InvalidChunkPayload {
                    tag: EXPORTED_STRING_HEADER,
                },
            )?;
            if length != 0 {
                let value = std::str::from_utf8(&tail[..length])
                    .map_err(|_| AttributeChunkError::InvalidUtf8DependencyName)?
                    .to_owned();
                strings.push(AttributeString {
                    offset: start,
                    value,
                });
            }
            start += length + 1;
        }

        Ok(Self {
            string_table: 8..table_end,
            strings,
            object_data: table_end..bytes.len(),
        })
    }
}

fn parse_dependencies(
    bytes: &[u8],
    chunk: &AttributeChunk,
) -> Result<Vec<AttributeDependency>, AttributeChunkError> {
    let count = usize::try_from(read_u64(bytes, chunk.offset + 8, DEPENDENCY_CHUNK)?)
        .map_err(|_| AttributeChunkError::IntegerOverflow)?;
    if count == 0 {
        return Err(AttributeChunkError::InvalidChunkPayload {
            tag: DEPENDENCY_CHUNK,
        });
    }
    let ids_start = chunk.offset + 16;
    let offsets_start = ids_start
        .checked_add(
            count
                .checked_mul(8)
                .ok_or(AttributeChunkError::IntegerOverflow)?,
        )
        .ok_or(AttributeChunkError::IntegerOverflow)?;
    let names_start = offsets_start
        .checked_add(
            count
                .checked_mul(4)
                .ok_or(AttributeChunkError::IntegerOverflow)?,
        )
        .ok_or(AttributeChunkError::IntegerOverflow)?;
    if names_start > chunk.payload.end {
        return Err(AttributeChunkError::InvalidChunkPayload {
            tag: DEPENDENCY_CHUNK,
        });
    }

    let mut dependencies = Vec::with_capacity(count - 1);
    // The reference getters deliberately omit entry zero: it represents the
    // VLT buffer itself. Entries one onward are external sibling dependencies.
    for index in 1..count {
        let id = read_u64(bytes, ids_start + index * 8, DEPENDENCY_CHUNK)?;
        let name_offset = usize::try_from(read_u32(
            bytes,
            offsets_start + index * 4,
            DEPENDENCY_CHUNK,
        )?)
        .map_err(|_| AttributeChunkError::IntegerOverflow)?;
        let string_start = names_start
            .checked_add(name_offset)
            .ok_or(AttributeChunkError::IntegerOverflow)?;
        let tail = bytes.get(string_start..chunk.payload.end).ok_or(
            AttributeChunkError::InvalidChunkPayload {
                tag: DEPENDENCY_CHUNK,
            },
        )?;
        let name_end = tail.iter().position(|byte| *byte == 0).ok_or(
            AttributeChunkError::InvalidChunkPayload {
                tag: DEPENDENCY_CHUNK,
            },
        )?;
        let name = std::str::from_utf8(&tail[..name_end])
            .map_err(|_| AttributeChunkError::InvalidUtf8DependencyName)?
            .to_owned();
        dependencies.push(AttributeDependency {
            id,
            name,
            // IsAssetDependency8297ca50 LDX / ca54 CMPLDI tests all 64 bits.
            is_asset: id != 0,
        });
    }
    Ok(dependencies)
}

fn parse_exports(
    bytes: &[u8],
    chunk: &AttributeChunk,
) -> Result<Vec<AttributeExport>, AttributeChunkError> {
    let count = usize::try_from(read_u64(bytes, chunk.offset + 8, EXPORT_CHUNK)?)
        .map_err(|_| AttributeChunkError::IntegerOverflow)?;
    let records_start = chunk.offset + 16;
    let records_end = records_start
        .checked_add(
            count
                .checked_mul(24)
                .ok_or(AttributeChunkError::IntegerOverflow)?,
        )
        .ok_or(AttributeChunkError::IntegerOverflow)?;
    // Vault::Vault8297d368 reads exactly mCount records and ignores the chunk
    // tail; January 2009 vaults pad an odd-count ExpN to 16 bytes.
    if records_end > chunk.payload.end {
        return Err(AttributeChunkError::InvalidChunkPayload { tag: EXPORT_CHUNK });
    }
    let mut exports = Vec::with_capacity(count);
    for index in 0..count {
        let offset = records_start + index * 24;
        exports.push(AttributeExport {
            object_id: read_u64(bytes, offset, EXPORT_CHUNK)?,
            type_id: read_u64(bytes, offset + 8, EXPORT_CHUNK)?,
            data_size: read_u32(bytes, offset + 16, EXPORT_CHUNK)?,
            data_offset: read_u32(bytes, offset + 20, EXPORT_CHUNK)?,
        });
    }
    Ok(exports)
}

fn parse_pointer_fixups(
    bytes: &[u8],
    chunk: &AttributeChunk,
) -> Result<(Vec<AttributePointerFixup>, AttributePointerTerminator), AttributeChunkError> {
    // Initialize() consumes pairs of eight-byte PointerNode records and stops
    // on one final eight-byte record whose kind is not 1..=4. The producer
    // pads that terminator with sixteen zero bytes.
    let pointer_bytes = chunk.payload.len();
    if pointer_bytes < 24 || (pointer_bytes - 24) % 16 != 0 {
        return Err(AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK });
    }
    let count = (pointer_bytes - 24) / 16;
    let mut fixups = Vec::with_capacity(count);
    for index in 0..count {
        let offset = chunk.payload.start + index * 16;
        let encoded = read_u32(bytes, offset + 4, POINTER_CHUNK)?;
        let kind = match encoded >> 16 {
            1 => AttributePointerKind::Null,
            2 => AttributePointerKind::SelectTargetBase,
            3 => AttributePointerKind::Direct,
            4 => AttributePointerKind::Export,
            _ => return Err(AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK }),
        };
        fixups.push(AttributePointerFixup {
            target_delta: read_u32(bytes, offset, POINTER_CHUNK)?,
            kind,
            dependency_index: u16::try_from(encoded & u32::from(u16::MAX))
                .map_err(|_| AttributeChunkError::IntegerOverflow)?,
            argument: read_u64(bytes, offset + 8, POINTER_CHUNK)?,
        });
    }

    let terminator_offset = chunk.payload.start + count * 16;
    let terminator = AttributePointerTerminator {
        target_delta: read_u32(bytes, terminator_offset, POINTER_CHUNK)?,
        encoded: read_u32(bytes, terminator_offset + 4, POINTER_CHUNK)?,
        padding: bytes[terminator_offset + 8..terminator_offset + 24]
            .try_into()
            .map_err(|_| AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK })?,
    };
    let terminator_kind = u16::try_from(terminator.encoded >> 16)
        .map_err(|_| AttributeChunkError::IntegerOverflow)?;
    if (1..=4).contains(&terminator_kind) {
        return Err(AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK });
    }
    if terminator.padding != [0; 16] {
        return Err(AttributeChunkError::InvalidChunkPayload { tag: POINTER_CHUNK });
    }
    Ok((fixups, terminator))
}

fn read_u8(bytes: &[u8], offset: usize, tag: [u8; 4]) -> Result<u8, AttributeChunkError> {
    bytes
        .get(offset)
        .copied()
        .ok_or(AttributeChunkError::InvalidChunkPayload { tag })
}

fn read_u16(bytes: &[u8], offset: usize, tag: [u8; 4]) -> Result<u16, AttributeChunkError> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or(AttributeChunkError::InvalidChunkPayload { tag })?;
    Ok(u16::from_be_bytes([value[0], value[1]]))
}

/// Alignment kept for every buffer offset the merge moves: offsets shift by multiples of it.
const MERGE_ALIGNMENT: usize = 0x80;

/// PC port addition: Skate 3 splits Skate 2's single skater database (`skater.vlt` + `.bin`)
/// into a class Vault (`skaterschema`) and a collection Vault (`skatercollections`) that the
/// game loads into one `Attrib::Database`. This joins two Vaults into one equivalent `.vlt` and
/// `.bin`, so readers of the single database read both: `first`'s version, string and end
/// chunks; a dependency table naming `names` (the joined `.vlt` and `.bin`); both Vaults' data in
/// one `DatN`, their exports in one `ExpN` and their fixups in one `PtrN`, every offset moved by
/// its Vault's shift (a multiple of [`MERGE_ALIGNMENT`]). The `.bin` is `first`'s, then
/// `second`'s at the next aligned offset; its string table is `first`'s.
///
/// # Errors
///
/// [`AttributeChunkError`] when either Vault does not parse, has other than one external
/// dependency, or holds a fixup outside its data.
pub fn merge_vaults(
    first: (&[u8], &[u8]),
    second: (&[u8], &[u8]),
    names: [&str; 2],
) -> Result<(Vec<u8>, Vec<u8>), AttributeChunkError> {
    let parts = [first, second];
    let metas = [AttributeVaultMetadata::parse(first.0)?, AttributeVaultMetadata::parse(second.0)?];
    let files = [AttributeChunkFile::parse(first.0)?, AttributeChunkFile::parse(second.0)?];
    for meta in &metas {
        if meta.dependencies.len() != 1 {
            return Err(AttributeChunkError::InvalidChunkPayload { tag: DEPENDENCY_CHUNK });
        }
    }
    let align = |value: usize| value.div_ceil(MERGE_ALIGNMENT) * MERGE_ALIGNMENT;
    let pad16 = |buffer: &mut Vec<u8>| buffer.resize(buffer.len().div_ceil(16) * 16, 0);

    // The joined `.bin`.
    let mut bin = first.1.to_vec();
    let second_bin = align(bin.len());
    bin.resize(second_bin, 0);
    bin.extend_from_slice(second.1);
    let bin_shift = [0_i64, i64::try_from(second_bin).map_err(|_| AttributeChunkError::IntegerOverflow)?];

    let mut vlt = Vec::new();
    for tag in [VERSION_CHUNK, STRING_CHUNK] {
        let chunk = files[0].required_chunk(tag)?;
        vlt.extend_from_slice(&first.0[chunk.offset..chunk.offset + chunk.size]);
    }

    // Dependency table: entry zero (the `.vlt` itself) and the `.bin`, under the joined names.
    let dependency = files[0].required_chunk(DEPENDENCY_CHUNK)?;
    let ids = [read_u64(first.0, dependency.offset + 16, DEPENDENCY_CHUNK)?, read_u64(first.0, dependency.offset + 24, DEPENDENCY_CHUNK)?];
    let start = vlt.len();
    vlt.extend_from_slice(&DEPENDENCY_CHUNK);
    vlt.extend_from_slice(&[0; 4]);
    vlt.extend_from_slice(&2_u64.to_be_bytes());
    for id in ids {
        vlt.extend_from_slice(&id.to_be_bytes());
    }
    let second_name = u32::try_from(names[0].len() + 1).map_err(|_| AttributeChunkError::IntegerOverflow)?;
    vlt.extend_from_slice(&0_u32.to_be_bytes());
    vlt.extend_from_slice(&second_name.to_be_bytes());
    for name in names {
        vlt.extend_from_slice(name.as_bytes());
        vlt.push(0);
    }
    pad16(&mut vlt);
    let size = u32::try_from(vlt.len() - start).map_err(|_| AttributeChunkError::IntegerOverflow)?;
    vlt[start + 4..start + 8].copy_from_slice(&size.to_be_bytes());

    // Both Vaults' data, each at an offset congruent to its original one.
    let data_start = vlt.len();
    vlt.extend_from_slice(&DATA_CHUNK);
    vlt.extend_from_slice(&[0; 4]);
    let mut vlt_shift = [0_i64; 2];
    for (index, meta) in metas.iter().enumerate() {
        let original = meta.data.start;
        let mut position = vlt.len();
        while position % MERGE_ALIGNMENT != original % MERGE_ALIGNMENT {
            position += 1;
        }
        vlt.resize(position, 0);
        vlt.extend_from_slice(&parts[index].0[meta.data.clone()]);
        vlt_shift[index] = i64::try_from(position).map_err(|_| AttributeChunkError::IntegerOverflow)?
            - i64::try_from(original).map_err(|_| AttributeChunkError::IntegerOverflow)?;
    }
    pad16(&mut vlt);
    let size = u32::try_from(vlt.len() - data_start).map_err(|_| AttributeChunkError::IntegerOverflow)?;
    vlt[data_start + 4..data_start + 8].copy_from_slice(&size.to_be_bytes());

    let moved = |value: u64, shift: i64| -> Result<u64, AttributeChunkError> {
        u64::try_from(i64::try_from(value).map_err(|_| AttributeChunkError::IntegerOverflow)? + shift)
            .map_err(|_| AttributeChunkError::IntegerOverflow)
    };

    // Exports.
    let export_start = vlt.len();
    vlt.extend_from_slice(&EXPORT_CHUNK);
    vlt.extend_from_slice(&[0; 4]);
    let count = metas.iter().map(|meta| meta.exports.len()).sum::<usize>();
    vlt.extend_from_slice(&(count as u64).to_be_bytes());
    for (index, meta) in metas.iter().enumerate() {
        for export in &meta.exports {
            vlt.extend_from_slice(&export.object_id.to_be_bytes());
            vlt.extend_from_slice(&export.type_id.to_be_bytes());
            vlt.extend_from_slice(&export.data_size.to_be_bytes());
            let offset = u32::try_from(moved(u64::from(export.data_offset), vlt_shift[index])?).map_err(|_| AttributeChunkError::IntegerOverflow)?;
            vlt.extend_from_slice(&offset.to_be_bytes());
        }
    }
    pad16(&mut vlt);
    let size = u32::try_from(vlt.len() - export_start).map_err(|_| AttributeChunkError::IntegerOverflow)?;
    vlt[export_start + 4..export_start + 8].copy_from_slice(&size.to_be_bytes());

    // Fixups: slots and pointees in the `.vlt` (dependency 0) move with their Vault's data,
    // those in the `.bin` (dependency 1) with its `.bin`.
    let pointer_start = vlt.len();
    vlt.extend_from_slice(&POINTER_CHUNK);
    vlt.extend_from_slice(&[0; 4]);
    for (index, meta) in metas.iter().enumerate() {
        let shift_of = |dependency: u16| if dependency == 0 { vlt_shift[index] } else { bin_shift[index] };
        let mut target = 0_u16;
        for fixup in &meta.pointer_fixups {
            let (delta, argument) = match fixup.kind {
                AttributePointerKind::SelectTargetBase => {
                    target = fixup.dependency_index;
                    (fixup.target_delta, fixup.argument)
                }
                AttributePointerKind::Direct => (
                    u32::try_from(moved(u64::from(fixup.target_delta), shift_of(target))?).map_err(|_| AttributeChunkError::IntegerOverflow)?,
                    moved(fixup.argument, shift_of(fixup.dependency_index))?,
                ),
                AttributePointerKind::Null | AttributePointerKind::Export => (
                    u32::try_from(moved(u64::from(fixup.target_delta), shift_of(target))?).map_err(|_| AttributeChunkError::IntegerOverflow)?,
                    fixup.argument,
                ),
            };
            vlt.extend_from_slice(&delta.to_be_bytes());
            vlt.extend_from_slice(&((u32::from(fixup.kind as u16) << 16) | u32::from(fixup.dependency_index)).to_be_bytes());
            vlt.extend_from_slice(&argument.to_be_bytes());
        }
    }
    vlt.extend_from_slice(&metas[0].pointer_terminator.target_delta.to_be_bytes());
    vlt.extend_from_slice(&metas[0].pointer_terminator.encoded.to_be_bytes());
    vlt.extend_from_slice(&[0; 16]);
    let size = u32::try_from(vlt.len() - pointer_start).map_err(|_| AttributeChunkError::IntegerOverflow)?;
    vlt[pointer_start + 4..pointer_start + 8].copy_from_slice(&size.to_be_bytes());

    for chunk in files[0].chunks.iter().filter(|chunk| chunk.tag == *b"EndC") {
        vlt.extend_from_slice(&first.0[chunk.offset..chunk.offset + chunk.size]);
    }
    // The joined file must parse and resolve as one Vault.
    let merged = AttributeVaultMetadata::parse(&vlt)?;
    merged.resolve_pointers(&[vlt.len(), bin.len()])?;
    Ok((vlt, bin))
}

fn read_u32(bytes: &[u8], offset: usize, tag: [u8; 4]) -> Result<u32, AttributeChunkError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(AttributeChunkError::InvalidChunkPayload { tag })?;
    Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_u64(bytes: &[u8], offset: usize, tag: [u8; 4]) -> Result<u64, AttributeChunkError> {
    let value = bytes
        .get(offset..offset + 8)
        .ok_or(AttributeChunkError::InvalidChunkPayload { tag })?;
    Ok(u64::from_be_bytes([
        value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
    ]))
}

/// Reconstructs the reference `Attrib::StringHash64` operation. The executable
/// supplies a fixed seed to Bob Jenkins' public-domain lookup8 mixing routine.
#[must_use]
pub fn string_hash64(value: &str) -> u64 {
    if value.is_empty() {
        return 0;
    }
    hash64(value.as_bytes(), ATTRIBUTE_STRING_HASH_SEED)
}

fn hash64(mut key: &[u8], seed: u64) -> u64 {
    const GOLDEN_RATIO: u64 = 0x9E37_79B9_7F4A_7C13;

    let original_length = key.len();
    let mut a = seed;
    let mut b = seed;
    let mut c = GOLDEN_RATIO;
    while key.len() >= 24 {
        a = a.wrapping_add(little_endian_u64(&key[..8]));
        b = b.wrapping_add(little_endian_u64(&key[8..16]));
        c = c.wrapping_add(little_endian_u64(&key[16..24]));
        (a, b, c) = mix64(a, b, c);
        key = &key[24..];
    }

    c = c.wrapping_add(original_length as u64);
    for (index, byte) in key.iter().copied().enumerate() {
        let value = u64::from(byte);
        match index {
            0..=7 => a = a.wrapping_add(value << (index * 8)),
            8..=15 => b = b.wrapping_add(value << ((index - 8) * 8)),
            // lookup8 reserves c's low byte for the original length.
            16..=22 => c = c.wrapping_add(value << ((index - 15) * 8)),
            _ => unreachable!("the full-block loop leaves at most 23 bytes"),
        }
    }
    mix64(a, b, c).2
}

fn little_endian_u64(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .copied()
        .enumerate()
        .fold(0_u64, |value, (shift, byte)| {
            value | (u64::from(byte) << (shift * 8))
        })
}

#[allow(clippy::many_single_char_names)]
fn mix64(mut a: u64, mut b: u64, mut c: u64) -> (u64, u64, u64) {
    a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 43);
    b = b.wrapping_sub(c).wrapping_sub(a) ^ (a << 9);
    c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 8);
    a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 38);
    b = b.wrapping_sub(c).wrapping_sub(a) ^ (a << 23);
    c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 5);
    a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 35);
    b = b.wrapping_sub(c).wrapping_sub(a) ^ (a << 49);
    c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 11);
    a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 12);
    b = b.wrapping_sub(c).wrapping_sub(a) ^ (a << 18);
    c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 22);
    (a, b, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_big_endian_attribute_chunks() {
        let bytes = [
            b'V', b'e', b'r', b's', 0, 0, 0, 12, 1, 2, 3, 4, b'P', b't', b'r', b'N', 0, 0, 0, 8,
        ];
        let file = AttributeChunkFile::parse(&bytes).unwrap();
        assert_eq!(file.chunks.len(), 2);
        assert_eq!(file.chunks[0].tag, VERSION_CHUNK);
        assert_eq!(file.chunks[0].payload, 8..12);
        assert_eq!(file.chunks[1].tag, POINTER_CHUNK);
    }

    #[test]
    fn rejects_chunks_beyond_the_file() {
        let bytes = [b'D', b'e', b'p', b'N', 0, 0, 0, 16];
        assert!(matches!(
            AttributeChunkFile::parse(&bytes),
            Err(AttributeChunkError::ChunkOutsideFile { .. })
        ));
    }

    #[test]
    fn parses_symbol_proven_vault_tables() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"Vers\0\0\0\x10\0\0\0\0\0\0\0\x2a");
        bytes.extend_from_slice(b"StrN\0\0\0\x10\0\0\0\0\0\0\0\0");
        bytes.extend_from_slice(b"DepN\0\0\0\x34\0\0\0\0\0\0\0\x02");
        bytes.extend_from_slice(&0x1122_3344_0000_0000_u64.to_be_bytes());
        bytes.extend_from_slice(&0x5566_7788_0000_0001_u64.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(b"skater.bin\0\0");
        bytes.extend_from_slice(b"ExpN\0\0\0\x28\0\0\0\0\0\0\0\x01");
        bytes.extend_from_slice(&1_u64.to_be_bytes());
        bytes.extend_from_slice(&2_u64.to_be_bytes());
        bytes.extend_from_slice(&4_u32.to_be_bytes());
        bytes.extend_from_slice(&164_u32.to_be_bytes());
        bytes.extend_from_slice(b"PtrN\0\0\0\x20");
        bytes.extend_from_slice(&[0; 24]);
        bytes.extend_from_slice(b"DatN\0\0\0\x0cDATA");

        let vault = AttributeVaultMetadata::parse(&bytes).unwrap();
        assert_eq!(vault.version, 42);
        assert_eq!(vault.dependencies.len(), 1);
        assert_eq!(vault.dependencies[0].name, "skater.bin");
        assert!(vault.dependencies[0].is_asset);
        assert_eq!(vault.exports.len(), 1);
        assert_eq!(vault.exports[0].data_offset, 164);
        assert!(vault.pointer_fixups.is_empty());
        assert_eq!(&bytes[vault.data], b"DATA");
        // High-only IDs are assets too; a low-word-only test loses them.
        for id in [0x0000_0001_0000_0000_u64, 0] {
            bytes[56..64].copy_from_slice(&id.to_be_bytes());
            let parsed = AttributeVaultMetadata::parse(&bytes).unwrap();
            assert_eq!(parsed.dependencies[0].id, id);
            assert_eq!(parsed.dependencies[0].is_asset, id != 0);
        }
    }

    #[test]
    fn exposes_inline_collection_node_bytes_in_vault_order() {
        let entry = AttributeCollectionEntry {
            key: 1,
            data_pointer: 0x3F73_3333,
            type_index: 3,
            node_flags: INLINE_COLLECTION_VALUE_FLAG,
            entry_flags: 0,
        };
        assert_eq!(entry.inline_value_word(), Some([0x3F, 0x73, 0x33, 0x33]));

        let pointer_entry = AttributeCollectionEntry {
            node_flags: 0,
            ..entry
        };
        assert_eq!(pointer_entry.inline_value_word(), None);
    }

    #[test]
    fn parses_bin_string_boundary_without_treating_objects_as_chunks() {
        let bytes = b"StrE\0\0\0\x14\0default\0ip\0OBJECT";
        let data = AttributeDataFile::parse(bytes).unwrap();
        assert_eq!(data.string_table, 8..20);
        assert_eq!(data.strings[0].offset, 9);
        assert_eq!(data.strings[0].value, "default");
        assert_eq!(data.strings[1].value, "ip");
        assert_eq!(&bytes[data.object_data], b"OBJECT");
    }

    #[test]
    fn hashes_reference_export_type_names() {
        assert_eq!(
            [
                string_hash64("Attrib::DatabaseLoadData"),
                string_hash64("Attrib::ClassLoadData"),
                string_hash64("Attrib::CollectionLoadData"),
            ],
            [
                0x0B38_8468_45E9_C175,
                0x2A78_95AC_4A87_6152,
                0xAD30_3B8F_42B3_307E,
            ]
        );
    }

    #[test]
    fn classifies_reference_export_types() {
        let mut export = AttributeExport {
            object_id: 0,
            type_id: DATABASE_LOAD_TYPE_ID,
            data_size: 0,
            data_offset: 0,
        };
        assert_eq!(export.kind(), AttributeExportKind::Database);
        export.type_id = CLASS_LOAD_TYPE_ID;
        assert_eq!(export.kind(), AttributeExportKind::Class);
        export.type_id = COLLECTION_LOAD_TYPE_ID;
        assert_eq!(export.kind(), AttributeExportKind::Collection);
    }

    #[test]
    fn resolves_fixups_to_logical_dependency_offsets() {
        let vault = AttributeVaultMetadata {
            version: 0,
            dependencies: vec![AttributeDependency {
                id: 1,
                name: "data.bin".to_owned(),
                is_asset: true,
            }],
            exports: Vec::new(),
            pointer_fixups: vec![
                AttributePointerFixup {
                    target_delta: 0,
                    kind: AttributePointerKind::SelectTargetBase,
                    dependency_index: 1,
                    argument: 0,
                },
                AttributePointerFixup {
                    target_delta: 4,
                    kind: AttributePointerKind::Direct,
                    dependency_index: 0,
                    argument: 12,
                },
                AttributePointerFixup {
                    target_delta: 0,
                    kind: AttributePointerKind::SelectTargetBase,
                    dependency_index: 0,
                    argument: 0,
                },
                AttributePointerFixup {
                    target_delta: 8,
                    kind: AttributePointerKind::Null,
                    dependency_index: 0,
                    argument: 0,
                },
            ],
            pointer_terminator: AttributePointerTerminator {
                target_delta: 0,
                encoded: 0,
                padding: [0; 16],
            },
            data: 0..0,
        };
        let pointers = vault.resolve_pointers(&[32, 24]).unwrap();
        assert_eq!(
            pointers[&AttributePointerLocation {
                dependency_index: 1,
                offset: 4,
            }],
            AttributePointerValue::Direct {
                dependency_index: 0,
                offset: 12,
            }
        );
        assert_eq!(
            pointers[&AttributePointerLocation {
                dependency_index: 0,
                offset: 8,
            }],
            AttributePointerValue::Null
        );
    }
}
