//! Asset and collection tables used by streamed worlds.

use crate::refpack::{self, RefPackError};
use std::{array, fmt, ops::Range};

const ASSET_HEADER_SIZE: usize = 24;
const ASSET_ENTRY_SIZE: usize = 64;
const COLLECTION_HEADER_SIZE: usize = 32;
const COLLECTION_ENTRY_SIZE: usize = 104;
const SPACE_HEADER_SIZE: usize = 24;
const SPACE_ENTRY_SIZE: usize = 80;
const STREAM_FILE_MIN_HEADER_SIZE: usize = 32;
const COLLECTION_ASSET_SIZE: usize = 24;
const COMPRESSED_ARENA_HEADER_SIZE: usize = 80;

/// Largest block stride accepted by
/// `tLoadCollectionRequest::ParseRawFunction` at `0x82d54700`.
pub const MAX_COLLECTION_ASSET_STRIDE: u32 = 0x0028_0000;

/// Default horizontal radius assigned to presentation and simulation focuses by
/// `cAssetStreamSystem::AddStreamFocus` at `0x824dacb0`.
pub const WORLD_FOCUS_RADIUS: f32 = 195.0;
/// Default horizontal radius assigned to texture focuses.
pub const TEXTURE_FOCUS_RADIUS: f32 = 70.0;
/// Vertical half-extent of the AABB queried by `cStreamFile::FindCollections`.
pub const FOCUS_HALF_HEIGHT: f32 = 8_000.0;
/// Default simulation collection cap (`kSimMaxFocusCollections`).
pub const DEFAULT_SIMULATION_FOCUS_COLLECTIONS: usize = 4;
/// Default presentation collection cap (`kPresMaxFocusCollections`).
pub const DEFAULT_PRESENTATION_FOCUS_COLLECTIONS: usize = 16;
/// Texture focuses share the fixed 16-entry `tFocusInfo` capacity.
pub const DEFAULT_TEXTURE_FOCUS_COLLECTIONS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BaseResourceDescriptor {
    pub size: u32,
    pub alignment: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetMapEntry {
    pub id: u64,
    pub asset_type: u32,
    pub serialized_size: u32,
    pub resources: [BaseResourceDescriptor; 5],
    pub name_offset: u32,
    pub unused: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetToc {
    pub version: u32,
    pub stamp: u64,
    pub name_table_offset: u32,
    pub assets: Vec<AssetMapEntry>,
}

impl AssetToc {
    /// Parses an `ATOC` (`*.xst`) table.
    ///
    /// Layout evidence: `assman::tAssetMapHeader` and
    /// `assman::tAssetMapEntry` in the reference debug types.
    ///
    /// # Errors
    ///
    /// Returns [`StreamTableError`] if the header, magic, record count, or
    /// fixed-size records are malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, StreamTableError> {
        require_magic(bytes, *b"ATOC")?;
        let version = be_u32(bytes, 4)?;
        let stamp = be_u64(bytes, 8)?;
        let count = be_u32(bytes, 16)?;
        let name_table_offset = be_u32(bytes, 20)?;
        let table_end = checked_table_end(count, ASSET_ENTRY_SIZE, ASSET_HEADER_SIZE)?;
        if table_end > bytes.len() {
            return Err(StreamTableError::TableOutsideFile {
                required: table_end,
                actual: bytes.len(),
            });
        }

        let mut assets = Vec::with_capacity(count as usize);
        for index in 0..count as usize {
            let start = ASSET_HEADER_SIZE + index * ASSET_ENTRY_SIZE;
            let mut resources = [BaseResourceDescriptor {
                size: 0,
                alignment: 0,
            }; 5];
            for (resource_index, resource) in resources.iter_mut().enumerate() {
                let offset = start + 16 + resource_index * 8;
                *resource = BaseResourceDescriptor {
                    size: be_u32(bytes, offset)?,
                    alignment: be_u32(bytes, offset + 4)?,
                };
            }
            assets.push(AssetMapEntry {
                id: be_u64(bytes, start)?,
                asset_type: be_u32(bytes, start + 8)?,
                serialized_size: be_u32(bytes, start + 12)?,
                resources,
                name_offset: be_u32(bytes, start + 56)?,
                unused: be_u32(bytes, start + 60)?,
            });
        }

        Ok(Self {
            version,
            stamp,
            name_table_offset,
            assets,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionMapEntry {
    pub id: u64,
    pub parent_id: u64,
    pub payload_offset: u32,
    pub payload_size: u32,
    pub asset_count: u32,
    pub filename: String,
    pub name_offset: u32,
    pub alignment_pad: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionMap {
    pub version: u32,
    pub stamp: u64,
    pub stream_format: u32,
    pub name_table_offset: u32,
    pub collections: Vec<CollectionMapEntry>,
}

impl CollectionMap {
    /// Parses a `CMAP` (`*.xsm`) table.
    ///
    /// Layout evidence: `assman::tCollectionMapHeader` and
    /// `assman::tCollectionMapEntry` in the reference debug types.
    ///
    /// # Errors
    ///
    /// Returns [`StreamTableError`] if the header, magic, record count,
    /// filename, or fixed-size records are malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, StreamTableError> {
        require_magic(bytes, *b"CMAP")?;
        let version = be_u32(bytes, 4)?;
        let stamp = be_u64(bytes, 8)?;
        let count = be_u32(bytes, 16)?;
        let stream_format = be_u32(bytes, 20)?;
        let name_table_offset = be_u32(bytes, 24)?;
        let table_end = checked_table_end(count, COLLECTION_ENTRY_SIZE, COLLECTION_HEADER_SIZE)?;
        if table_end > bytes.len() {
            return Err(StreamTableError::TableOutsideFile {
                required: table_end,
                actual: bytes.len(),
            });
        }

        let mut collections = Vec::with_capacity(count as usize);
        for index in 0..count as usize {
            let start = COLLECTION_HEADER_SIZE + index * COLLECTION_ENTRY_SIZE;
            let filename_bytes = &bytes[start + 28..start + 92];
            let filename_end = filename_bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(filename_bytes.len());
            let filename = std::str::from_utf8(&filename_bytes[..filename_end])
                .map_err(|_| StreamTableError::InvalidFilename { index })?
                .to_owned();
            collections.push(CollectionMapEntry {
                id: be_u64(bytes, start)?,
                parent_id: be_u64(bytes, start + 8)?,
                payload_offset: be_u32(bytes, start + 16)?,
                payload_size: be_u32(bytes, start + 20)?,
                asset_count: be_u32(bytes, start + 24)?,
                filename,
                name_offset: be_u32(bytes, start + 92)?,
                alignment_pad: be_i32(bytes, start + 96)?,
            });
        }

        Ok(Self {
            version,
            stamp,
            stream_format,
            name_table_offset,
            collections,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollectionSpaceEntry {
    pub id: u64,
    pub lod: u8,
    pub hull_min: [f32; 4],
    pub hull_max: [f32; 4],
    pub geometry_min: [f32; 4],
    pub geometry_max: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollectionSpace {
    pub version: u32,
    pub stamp: u64,
    pub collections: Vec<CollectionSpaceEntry>,
}

/// One spatial collection selected for a stream focus, in reference distance
/// order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusCollection {
    pub index: usize,
    pub id: u64,
    pub position: [f32; 3],
    pub distance_xz: f32,
    pub stream_order: i32,
}

/// How resident collections compared with their focus selection ([`order_like_selection`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionOrder {
    /// The residents are exactly the selection's collections.
    pub matches: bool,
    /// Pairs of collections at equal distances the residents held in the other order, which a
    /// sort by distance alone would have kept.
    pub ties_reordered: usize,
}

/// Puts resident collections in the order of the focus selection they were reconciled with
/// ([`CollectionSpace::focus_collections`]: by distance, collections at equal distances in its own
/// order) and says whether they are exactly its collections. Sorted by distance alone, two
/// collections at the same distance keep the order an earlier selection made them resident in.
pub fn order_like_selection<T>(resident: &mut [T], id: impl Fn(&T) -> u64, selected: &[FocusCollection]) -> SelectionOrder {
    let before: Vec<u64> = resident.iter().map(&id).collect();
    resident.sort_by_key(|collection| {
        let id = id(collection);
        selected.iter().position(|entry| entry.id == id).unwrap_or(usize::MAX)
    });
    let matches = resident.len() == selected.len() && resident.iter().zip(selected).all(|(collection, entry)| id(collection) == entry.id);
    let at = |id: u64| before.iter().position(|&resident| resident == id);
    let ties_reordered = selected
        .windows(2)
        .filter(|pair| pair[0].distance_xz.total_cmp(&pair[1].distance_xz).is_eq())
        .filter(|pair| matches!((at(pair[0].id), at(pair[1].id)), (Some(first), Some(second)) if first > second))
        .count();
    SelectionOrder { matches, ties_reordered }
}

/// Reconstructs the high-detail collection name selected by
/// `cAssetStreamSystem::GetCollectionId` at `0x824d73b0`.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // The reference explicitly truncates float positions.
pub fn high_detail_collection_filename(stream_suffix: &str, position: [f32; 3]) -> String {
    let x = position[0] as i32;
    let z = position[2] as i32;
    let center_x = 200 * (x / 200) + if x < 0 { -100 } else { 100 };
    let center_z = 200 * (z / 200) + if z < 0 { -100 } else { 100 };
    format!("c{stream_suffix}_{center_x}_{center_z}_high")
}

impl CollectionSpace {
    /// Parses a `CSPA` (`*.xss`) spatial table.
    ///
    /// Layout evidence: `assman::tCollectionSpaceHeader` and
    /// `assman::tCollectionSpaceEntry` in the reference debug types.
    ///
    /// # Errors
    ///
    /// Returns [`StreamTableError`] if the header, count, or fixed-size
    /// spatial records are malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, StreamTableError> {
        require_magic(bytes, *b"CSPA")?;
        let version = be_u32(bytes, 4)?;
        let stamp = be_u64(bytes, 8)?;
        let count = be_u32(bytes, 16)?;
        let table_end = checked_table_end(count, SPACE_ENTRY_SIZE, SPACE_HEADER_SIZE)?;
        if table_end > bytes.len() {
            return Err(StreamTableError::TableOutsideFile {
                required: table_end,
                actual: bytes.len(),
            });
        }

        let mut collections = Vec::with_capacity(count as usize);
        for index in 0..count as usize {
            let start = SPACE_HEADER_SIZE + index * SPACE_ENTRY_SIZE;
            collections.push(CollectionSpaceEntry {
                id: be_u64(bytes, start)?,
                lod: bytes[start + 8],
                hull_min: be_vec4(bytes, start + 16)?,
                hull_max: be_vec4(bytes, start + 32)?,
                geometry_min: be_vec4(bytes, start + 48)?,
                geometry_max: be_vec4(bytes, start + 64)?,
            });
        }
        Ok(Self {
            version,
            stamp,
            collections,
        })
    }

    /// Reconstructs the non-XML spatial path in
    /// `cStreamFile::FindCollections` at `0x824df068`.
    ///
    /// The reference octree is an acceleration structure over each entry's
    /// hull AABB. Testing every serialized hull for the same AABB overlap gives
    /// the same candidate set without carrying the console's octree memory
    /// representation into the native runtime. Entries whose serialized LOD
    /// byte is nonzero are omitted when the reference builds that octree.
    #[must_use]
    pub fn focus_collections(
        &self,
        focus: [f32; 3],
        radius: f32,
        max_collections: usize,
    ) -> Vec<FocusCollection> {
        let query_min = [
            focus[0] - radius,
            focus[1] - FOCUS_HALF_HEIGHT,
            focus[2] - radius,
        ];
        let query_max = [
            focus[0] + radius,
            focus[1] + FOCUS_HALF_HEIGHT,
            focus[2] + radius,
        ];
        let mut matches = self
            .collections
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.lod == 0)
            .filter(|(_, entry)| {
                aabb_overlaps(entry.hull_min, entry.hull_max, query_min, query_max)
            })
            .map(|(index, _)| self.focus_collection(index, focus))
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| left.distance_xz.total_cmp(&right.distance_xz));
        matches.truncate(max_collections);
        matches
    }

    #[must_use]
    pub fn focus_collection(&self, index: usize, focus: [f32; 3]) -> FocusCollection {
        let entry = &self.collections[index];
        let position = [
            entry.hull_min[0] + (entry.hull_max[0] - entry.hull_min[0]) * 0.5,
            entry.hull_min[1] + (entry.hull_max[1] - entry.hull_min[1]) * 0.5,
            entry.hull_min[2] + (entry.hull_max[2] - entry.hull_min[2]) * 0.5,
        ];
        let delta_x = focus[0] - position[0];
        let delta_z = focus[2] - position[2];
        FocusCollection {
            index,
            id: entry.id,
            position,
            distance_xz: (delta_x * delta_x + delta_z * delta_z).sqrt(),
            stream_order: collection_stream_order(position),
        }
    }
}

/// `tStreamFocusInfo::tFocusInfo::mEntries`: the collections a focus selection holds.
pub const MAX_FOCUS_ENTRIES: usize = 16;

/// `cAssetStreamSystem::FindCollections` (`0x824dfea8`) over every stream file of one type,
/// without stream XML: the first file's selection, then each later file's entries, one the
/// selection already holds taking its place and a new one joining while fewer than
/// [`MAX_FOCUS_ENTRIES`] are held, then the whole sorted by distance (`FocusLODDistanceCompare`
/// over the octrees' level-0 collections). The caller applies the type's cap.
#[must_use]
pub fn merge_focus_collections(files: impl IntoIterator<Item = Vec<FocusCollection>>) -> Vec<FocusCollection> {
    let mut files = files.into_iter();
    let mut selected = files.next().unwrap_or_default();
    for file in files {
        for entry in file {
            if let Some(held) = selected.iter_mut().find(|held| held.id == entry.id) {
                *held = entry;
            } else if selected.len() < MAX_FOCUS_ENTRIES {
                selected.push(entry);
            }
        }
    }
    selected.sort_by(|left, right| left.distance_xz.total_cmp(&right.distance_xz));
    selected
}

/// Reconstructs `tOctreeInfo::mStreamOrder` as initialized by
/// `cStreamFile::Update` at `0x824e0ba8`.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // The reference converts the rounded floats to i32.
pub fn collection_stream_order(position: [f32; 3]) -> i32 {
    let x = (position[0] + 0.5).floor() as i32;
    let z = (position[2] + 0.5).floor() as i32;
    10_000 * x + z / 10
}

fn aabb_overlaps(
    left_min: [f32; 4],
    left_max: [f32; 4],
    right_min: [f32; 3],
    right_max: [f32; 3],
) -> bool {
    (0..3).all(|axis| left_min[axis] <= right_max[axis] && left_max[axis] >= right_min[axis])
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamAssetBlock {
    pub id: u64,
    pub stored_size: u32,
    pub data_offset: u32,
    pub stride: u32,
    pub header_offset: usize,
    pub data_range: Range<usize>,
}

/// Asset blocks parsed from the payload-relative portion of an XSF entry.
///
/// `tLoadCollectionRequest::Load` at `0x82d54bd0` starts its stream at the
/// collection map's 64-bit payload offset. Its `ParseRawFunction` at
/// `0x82d546b0` obtains every block size, including the final one, from the
/// current 24-byte `tCollectionAsset` stride. This type models that payload
/// boundary without requiring or interpreting the preceding `SFIL` header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionPayload {
    pub assets: Vec<StreamAssetBlock>,
    /// Number of payload bytes covered by the declared asset strides.
    /// Trailing bytes are preserved outside this span rather than rejected.
    pub consumed_size: usize,
}

impl CollectionPayload {
    /// Parses `asset_count` strided asset blocks from an XSF payload slice.
    ///
    /// Every stride is required to be in the exact native domain
    /// `1..=0x280000`, including the final block. Rust additionally validates
    /// that each complete header and data range stays inside both its stride
    /// and the supplied payload window; malformed native input was an
    /// assert/unsafe domain rather than a recoverable branch.
    ///
    /// # Errors
    ///
    /// Returns [`StreamTableError`] for a truncated record, zero or oversized
    /// stride, impossible count, allocation failure, overflowing arithmetic,
    /// or a data range outside its block or the supplied payload.
    pub fn parse(bytes: &[u8], asset_count: u32) -> Result<Self, StreamTableError> {
        let count = usize::try_from(asset_count).map_err(|_| StreamTableError::IntegerOverflow)?;
        if count != 0 && bytes.len() <= COLLECTION_ASSET_SIZE {
            return Err(StreamTableError::AssetParseWindowTooSmall {
                index: 0,
                available: bytes.len(),
            });
        }
        if count != 0 && count > (bytes.len() - 1) / COLLECTION_ASSET_SIZE {
            return Err(StreamTableError::AssetCountOutsidePayload {
                count: asset_count,
                payload_len: bytes.len(),
            });
        }
        let mut assets = Vec::new();
        assets
            .try_reserve_exact(count)
            .map_err(|_| StreamTableError::AssetAllocationFailed { count: asset_count })?;
        let mut cursor = 0_usize;
        for index in 0..count {
            let available = bytes
                .len()
                .checked_sub(cursor)
                .ok_or(StreamTableError::IntegerOverflow)?;
            if available <= COLLECTION_ASSET_SIZE {
                return Err(StreamTableError::AssetParseWindowTooSmall { index, available });
            }
            let header_end = cursor
                .checked_add(COLLECTION_ASSET_SIZE)
                .ok_or(StreamTableError::IntegerOverflow)?;
            if header_end > bytes.len() {
                return Err(StreamTableError::StreamAssetOutsideFile { index });
            }

            let stored_size = be_u32(bytes, cursor + 8)?;
            let data_offset = be_u32(bytes, cursor + 12)?;
            let stride = be_u32(bytes, cursor + 16)?;
            if stride == 0 {
                return Err(StreamTableError::ZeroAssetStride { index });
            }
            if stride > MAX_COLLECTION_ASSET_STRIDE {
                return Err(StreamTableError::AssetStrideTooLarge {
                    index,
                    stride,
                    maximum: MAX_COLLECTION_ASSET_STRIDE,
                });
            }

            let block_end = cursor
                .checked_add(
                    usize::try_from(stride).map_err(|_| StreamTableError::IntegerOverflow)?,
                )
                .ok_or(StreamTableError::IntegerOverflow)?;
            if block_end > bytes.len() {
                return Err(StreamTableError::AssetStrideOutsidePayload { index });
            }
            let data_start = cursor
                .checked_add(
                    usize::try_from(data_offset).map_err(|_| StreamTableError::IntegerOverflow)?,
                )
                .ok_or(StreamTableError::IntegerOverflow)?;
            let data_end = data_start
                .checked_add(
                    usize::try_from(stored_size).map_err(|_| StreamTableError::IntegerOverflow)?,
                )
                .ok_or(StreamTableError::IntegerOverflow)?;
            if data_start < header_end || data_end > block_end {
                return Err(StreamTableError::AssetDataOutsideStride { index });
            }

            assets.push(StreamAssetBlock {
                id: be_u64(bytes, cursor)?,
                stored_size,
                data_offset,
                stride,
                header_offset: cursor,
                data_range: data_start..data_end,
            });
            cursor = block_end;
        }
        Ok(Self {
            assets,
            consumed_size: cursor,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamFile {
    pub version: u32,
    pub stamp: u64,
    pub header_size: u32,
    /// Three currently-unresolved header words preserved for evidence.
    pub header_words: [u32; 3],
    pub assets: Vec<StreamAssetBlock>,
}

impl StreamFile {
    /// Parses an `SFIL` (`*.xsf`) collection using the asset count supplied by
    /// its matching `CMAP` entry.
    ///
    /// Asset blocks use the exact `assman::tCollectionAsset` layout. Each
    /// block's stride points to the next block header, while its data offset is
    /// relative to its own header.
    ///
    /// # Errors
    ///
    /// Returns [`StreamTableError`] if headers, strides, or payload ranges are
    /// invalid.
    pub fn parse(bytes: &[u8], asset_count: u32) -> Result<Self, StreamTableError> {
        require_magic(bytes, *b"SFIL")?;
        let version = be_u32(bytes, 4)?;
        let stamp = be_u64(bytes, 8)?;
        let header_size = be_u32(bytes, 16)?;
        let header_words = [be_u32(bytes, 20)?, be_u32(bytes, 24)?, be_u32(bytes, 28)?];
        let mut cursor =
            usize::try_from(header_size).map_err(|_| StreamTableError::IntegerOverflow)?;
        if cursor < STREAM_FILE_MIN_HEADER_SIZE || cursor > bytes.len() {
            return Err(StreamTableError::TableOutsideFile {
                required: cursor,
                actual: bytes.len(),
            });
        }

        let mut assets = Vec::with_capacity(asset_count as usize);
        for index in 0..asset_count as usize {
            let block_header_end = cursor
                .checked_add(COLLECTION_ASSET_SIZE)
                .ok_or(StreamTableError::IntegerOverflow)?;
            if block_header_end > bytes.len() {
                return Err(StreamTableError::StreamAssetOutsideFile { index });
            }
            let stored_size = be_u32(bytes, cursor + 8)?;
            let data_offset = be_u32(bytes, cursor + 12)?;
            let stride = be_u32(bytes, cursor + 16)?;
            let data_start = cursor
                .checked_add(
                    usize::try_from(data_offset).map_err(|_| StreamTableError::IntegerOverflow)?,
                )
                .ok_or(StreamTableError::IntegerOverflow)?;
            let data_end = data_start
                .checked_add(
                    usize::try_from(stored_size).map_err(|_| StreamTableError::IntegerOverflow)?,
                )
                .ok_or(StreamTableError::IntegerOverflow)?;
            if data_start < block_header_end || data_end > bytes.len() {
                return Err(StreamTableError::StreamAssetOutsideFile { index });
            }
            assets.push(StreamAssetBlock {
                id: be_u64(bytes, cursor)?,
                stored_size,
                data_offset,
                stride,
                header_offset: cursor,
                data_range: data_start..data_end,
            });
            if index + 1 < asset_count as usize {
                if stride == 0 {
                    return Err(StreamTableError::ZeroAssetStride { index });
                }
                cursor = cursor
                    .checked_add(
                        usize::try_from(stride).map_err(|_| StreamTableError::IntegerOverflow)?,
                    )
                    .ok_or(StreamTableError::IntegerOverflow)?;
            }
        }
        Ok(Self {
            version,
            stamp,
            header_size,
            header_words,
            assets,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArenaSegment {
    pub destination_size: u32,
    pub source_size: u32,
    pub compressed: bool,
    pub unused: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompressedArenaHeader {
    pub segments: [ArenaSegment; 5],
}

impl CompressedArenaHeader {
    /// Parses the 80-byte resource header at the start of a streamed asset.
    ///
    /// Layout evidence: `assman::tCompressedArenaHeader` in the reference
    /// debug types.
    ///
    /// # Errors
    ///
    /// Returns [`StreamTableError::Truncated`] when the full header is absent.
    pub fn parse(bytes: &[u8]) -> Result<Self, StreamTableError> {
        if bytes.len() < COMPRESSED_ARENA_HEADER_SIZE {
            return Err(StreamTableError::Truncated);
        }
        let mut segments = [ArenaSegment {
            destination_size: 0,
            source_size: 0,
            compressed: false,
            unused: 0,
        }; 5];
        for (index, segment) in segments.iter_mut().enumerate() {
            let offset = index * 16;
            *segment = ArenaSegment {
                destination_size: be_u32(bytes, offset)?,
                source_size: be_u32(bytes, offset + 4)?,
                compressed: be_u32(bytes, offset + 8)? != 0,
                unused: be_u32(bytes, offset + 12)?,
            };
        }
        Ok(Self { segments })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArenaResources {
    pub resources: [Vec<u8>; 5],
}

/// Expands all five resource classes in a streamed arena payload.
///
/// Source segments immediately follow the 80-byte header. A nonzero
/// `compressed` field selects the same `RefPack` decoder used by BIG entries.
///
/// # Errors
///
/// Returns [`ArenaDecodeError`] when a segment extends beyond the payload,
/// `RefPack` decoding fails, or its decoded size disagrees with the descriptor.
pub fn decode_arena(bytes: &[u8]) -> Result<ArenaResources, ArenaDecodeError> {
    let header = CompressedArenaHeader::parse(bytes)?;
    let mut cursor = COMPRESSED_ARENA_HEADER_SIZE;
    let mut resources: [Vec<u8>; 5] = array::from_fn(|_| Vec::new());
    for (index, segment) in header.segments.iter().enumerate() {
        let source_size =
            usize::try_from(segment.source_size).map_err(|_| StreamTableError::IntegerOverflow)?;
        let end = cursor
            .checked_add(source_size)
            .ok_or(StreamTableError::IntegerOverflow)?;
        let source = bytes
            .get(cursor..end)
            .ok_or(ArenaDecodeError::SegmentOutsidePayload { index })?;
        let decoded = if segment.compressed {
            refpack::decompress(source)?
        } else {
            source.to_vec()
        };
        if u32::try_from(decoded.len()).ok() != Some(segment.destination_size) {
            return Err(ArenaDecodeError::SegmentSizeMismatch {
                index,
                expected: segment.destination_size,
                actual: decoded.len(),
            });
        }
        resources[index] = decoded;
        cursor = end;
    }
    if cursor != bytes.len() {
        return Err(ArenaDecodeError::TrailingData {
            decoded_end: cursor,
            actual: bytes.len(),
        });
    }
    Ok(ArenaResources { resources })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArenaDecodeError {
    Table(StreamTableError),
    RefPack(RefPackError),
    SegmentOutsidePayload {
        index: usize,
    },
    SegmentSizeMismatch {
        index: usize,
        expected: u32,
        actual: usize,
    },
    TrailingData {
        decoded_end: usize,
        actual: usize,
    },
}

impl fmt::Display for ArenaDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Skate 2 compressed arena: {self:?}")
    }
}

impl std::error::Error for ArenaDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Table(source) => Some(source),
            Self::RefPack(source) => Some(source),
            _ => None,
        }
    }
}

impl From<StreamTableError> for ArenaDecodeError {
    fn from(value: StreamTableError) -> Self {
        Self::Table(value)
    }
}

impl From<RefPackError> for ArenaDecodeError {
    fn from(value: RefPackError) -> Self {
        Self::RefPack(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamTableError {
    Truncated,
    WrongMagic {
        expected: [u8; 4],
        actual: [u8; 4],
    },
    IntegerOverflow,
    TableOutsideFile {
        required: usize,
        actual: usize,
    },
    InvalidFilename {
        index: usize,
    },
    StreamAssetOutsideFile {
        index: usize,
    },
    ZeroAssetStride {
        index: usize,
    },
    AssetStrideTooLarge {
        index: usize,
        stride: u32,
        maximum: u32,
    },
    AssetStrideOutsidePayload {
        index: usize,
    },
    AssetDataOutsideStride {
        index: usize,
    },
    AssetParseWindowTooSmall {
        index: usize,
        available: usize,
    },
    AssetCountOutsidePayload {
        count: u32,
        payload_len: usize,
    },
    AssetAllocationFailed {
        count: u32,
    },
}

impl fmt::Display for StreamTableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Skate 2 stream table: {self:?}")
    }
}

impl std::error::Error for StreamTableError {}

fn require_magic(bytes: &[u8], expected: [u8; 4]) -> Result<(), StreamTableError> {
    let field = bytes.get(..4).ok_or(StreamTableError::Truncated)?;
    if field == expected {
        Ok(())
    } else {
        Err(StreamTableError::WrongMagic {
            expected,
            actual: field.try_into().unwrap(),
        })
    }
}

fn checked_table_end(
    count: u32,
    record_size: usize,
    header_size: usize,
) -> Result<usize, StreamTableError> {
    usize::try_from(count)
        .ok()
        .and_then(|value| value.checked_mul(record_size))
        .and_then(|value| value.checked_add(header_size))
        .ok_or(StreamTableError::IntegerOverflow)
}

fn be_u32(bytes: &[u8], offset: usize) -> Result<u32, StreamTableError> {
    bytes
        .get(offset..offset + 4)
        .ok_or(StreamTableError::Truncated)
        .map(|field| u32::from_be_bytes(field.try_into().unwrap()))
}

fn be_u64(bytes: &[u8], offset: usize) -> Result<u64, StreamTableError> {
    bytes
        .get(offset..offset + 8)
        .ok_or(StreamTableError::Truncated)
        .map(|field| u64::from_be_bytes(field.try_into().unwrap()))
}

fn be_i32(bytes: &[u8], offset: usize) -> Result<i32, StreamTableError> {
    bytes
        .get(offset..offset + 4)
        .ok_or(StreamTableError::Truncated)
        .map(|field| i32::from_be_bytes(field.try_into().unwrap()))
}

fn be_vec4(bytes: &[u8], offset: usize) -> Result<[f32; 4], StreamTableError> {
    Ok([
        f32::from_bits(be_u32(bytes, offset)?),
        f32::from_bits(be_u32(bytes, offset + 4)?),
        f32::from_bits(be_u32(bytes, offset + 8)?),
        f32::from_bits(be_u32(bytes, offset + 12)?),
    ])
}

#[cfg(test)]
mod tests {
    fn focus(id: u64, distance_xz: f32) -> super::FocusCollection {
        super::FocusCollection { index: 0, id, position: [0.0; 3], distance_xz, stream_order: 0 }
    }

    #[test]
    fn resident_collections_at_equal_distances_follow_the_selection() {
        // A three-player run's crash (2026-10-16): two collections 133.29745 from the focus,
        // resident in the other order.
        let selected = [focus(0x7b60_9446_246d_b82a, 133.297_45), focus(0xe319_9128_c8b2_4204, 133.297_45), focus(3, 140.0)];
        let ids = |list: &[(u64, f32)]| list.iter().map(|&(id, _)| id).collect::<Vec<_>>();
        let expected: Vec<u64> = selected.iter().map(|entry| entry.id).collect();
        let mut resident: Vec<(u64, f32)> = vec![(0xe319_9128_c8b2_4204, 133.297_45), (3, 140.0), (0x7b60_9446_246d_b82a, 133.297_45)];
        let mut by_distance = resident.clone();
        by_distance.sort_by(|left, right| left.1.total_cmp(&right.1));
        assert_ne!(ids(&by_distance), expected);
        let order = super::order_like_selection(&mut resident, |&(id, _)| id, &selected);
        assert_eq!(order, super::SelectionOrder { matches: true, ties_reordered: 1 });
        assert_eq!(ids(&resident), expected);
    }

    #[test]
    fn resident_collections_other_than_the_selection_disagree() {
        let selected = [focus(1, 10.0), focus(2, 20.0)];
        assert!(!super::order_like_selection(&mut [1_u64, 9], |&id| id, &selected).matches);
        assert!(!super::order_like_selection(&mut [2_u64], |&id| id, &selected).matches);
        assert_eq!(super::order_like_selection(&mut [2_u64, 1], |&id| id, &selected), super::SelectionOrder { matches: true, ties_reordered: 0 });
    }

    use super::*;

    fn payload_block(id: u64, stride: u32, data_offset: u32, data: &[u8]) -> Vec<u8> {
        let len = usize::try_from(stride).unwrap().max(COLLECTION_ASSET_SIZE);
        let mut block = vec![0_u8; len];
        block[..8].copy_from_slice(&id.to_be_bytes());
        block[8..12].copy_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        block[12..16].copy_from_slice(&data_offset.to_be_bytes());
        block[16..20].copy_from_slice(&stride.to_be_bytes());
        let start = usize::try_from(data_offset).unwrap();
        if let Some(destination) = block.get_mut(start..start + data.len()) {
            destination.copy_from_slice(data);
        }
        block
    }

    #[test]
    fn maps_focus_position_to_reference_high_detail_grid() {
        assert_eq!(
            high_detail_collection_filename("Sim", [588.861_7, -54.231_647, 996.504_8]),
            "cSim_500_900_high"
        );
        assert_eq!(
            high_detail_collection_filename("Pres", [-3500.0, 103.026, -900.0]),
            "cPres_-3500_-900_high"
        );
    }

    #[test]
    fn calculates_reference_spatial_stream_order() {
        assert_eq!(collection_stream_order([500.0, 0.0, 900.0]), 5_000_090);
        assert_eq!(
            collection_stream_order([-3_500.0, 0.0, -900.0]),
            -35_000_090
        );
    }

    #[test]
    fn selects_spatial_focus_collections_by_xz_distance() {
        fn entry(id: u64, lod: u8, x: f32, z: f32) -> CollectionSpaceEntry {
            CollectionSpaceEntry {
                id,
                lod,
                hull_min: [x - 100.0, -100.0, z - 100.0, 1.0],
                hull_max: [x + 100.0, 100.0, z + 100.0, 1.0],
                geometry_min: [x - 90.0, -90.0, z - 90.0, 1.0],
                geometry_max: [x + 90.0, 90.0, z + 90.0, 1.0],
            }
        }

        let space = CollectionSpace {
            version: 3,
            stamp: 1,
            collections: vec![
                entry(1, 0, 500.0, 900.0),
                entry(2, 0, 700.0, 900.0),
                entry(3, 0, 500.0, 1_100.0),
                entry(4, 0, 300.0, 900.0),
                entry(5, 1, 600.0, 1_000.0),
                entry(6, 0, 1_100.0, 1_500.0),
            ],
        };
        let selected =
            space.focus_collections([588.861_7, -54.231_647, 996.504_8], WORLD_FOCUS_RADIUS, 3);
        assert_eq!(
            selected.iter().map(|entry| entry.id).collect::<Vec<_>>(),
            vec![1, 3, 2]
        );
        assert!(selected.iter().all(|entry| entry.id != 5));
    }

    #[test]
    fn parses_reference_shaped_asset_entry() {
        let mut bytes = Vec::from(*b"ATOC");
        bytes.extend_from_slice(&3_u32.to_be_bytes());
        bytes.extend_from_slice(&0x9546_2c2e_u64.to_be_bytes());
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&0xeddd_81d8_287b_14cf_u64.to_be_bytes());
        bytes.extend_from_slice(&0xab32_9a6a_u32.to_be_bytes());
        bytes.extend_from_slice(&0x4238_u32.to_be_bytes());
        for (size, alignment) in [
            (0x238_u32, 0x10_u32),
            (0, 1),
            (0x4000, 0x1000),
            (0, 1),
            (0, 1),
        ] {
            bytes.extend_from_slice(&size.to_be_bytes());
            bytes.extend_from_slice(&alignment.to_be_bytes());
        }
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&0x3200_8529_u32.to_be_bytes());

        let table = AssetToc::parse(&bytes).unwrap();
        assert_eq!(table.assets.len(), 1);
        assert_eq!(table.assets[0].serialized_size, 0x4238);
        assert_eq!(table.assets[0].resources[2].alignment, 0x1000);
    }

    #[test]
    fn parses_reference_shaped_collection_entry() {
        let mut bytes = Vec::from(*b"CMAP");
        bytes.extend_from_slice(&3_u32.to_be_bytes());
        bytes.extend_from_slice(&0x9546_2c2e_u64.to_be_bytes());
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.extend_from_slice(&2_u32.to_be_bytes());
        bytes.extend_from_slice(&0xdead_c0de_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&0xe872_38ef_d7f0_2b45_u64.to_be_bytes());
        bytes.extend_from_slice(&0xe171_e1ab_fedb_69ed_u64.to_be_bytes());
        bytes.extend_from_slice(&0x80_u32.to_be_bytes());
        bytes.extend_from_slice(&0x13_cf80_u32.to_be_bytes());
        bytes.extend_from_slice(&35_u32.to_be_bytes());
        let mut filename = [0_u8; 64];
        filename[..12].copy_from_slice(b"cPres_Global");
        bytes.extend_from_slice(&filename);
        bytes.extend_from_slice(&u32::MAX.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());

        let table = CollectionMap::parse(&bytes).unwrap();
        assert_eq!(table.collections.len(), 1);
        assert_eq!(table.collections[0].filename, "cPres_Global");
        assert_eq!(table.collections[0].asset_count, 35);
    }

    #[test]
    fn parses_reference_shaped_collection_space_entry() {
        let mut bytes = Vec::from(*b"CSPA");
        bytes.extend_from_slice(&3_u32.to_be_bytes());
        bytes.extend_from_slice(&0x9546_2c2e_u64.to_be_bytes());
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&0x2ff8_ed6c_cb99_5206_u64.to_be_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&[0x73, 0x5c, 0x65, 0x61, 0x78, 0x2d, 0x62]);
        for vector in [
            [-3600.0_f32, 103.026, -1000.0, 1.0],
            [-3400.0, 103.026, -800.0, 1.0],
            [-3446.0, 103.026, -995.0, 1.0],
            [-3400.0, 103.026, -800.0, 1.0],
        ] {
            for component in vector {
                bytes.extend_from_slice(&component.to_bits().to_be_bytes());
            }
        }

        let table = CollectionSpace::parse(&bytes).unwrap();
        assert_eq!(table.collections.len(), 1);
        assert_eq!(table.collections[0].id, 0x2ff8_ed6c_cb99_5206);
        assert!((table.collections[0].hull_min[0] + 3600.0).abs() < f32::EPSILON);
    }

    #[test]
    fn parses_strided_stream_asset_blocks() {
        let mut bytes = vec![0_u8; 0x500];
        bytes[..4].copy_from_slice(b"SFIL");
        bytes[4..8].copy_from_slice(&3_u32.to_be_bytes());
        bytes[8..16].copy_from_slice(&0x9532_ee28_u64.to_be_bytes());
        bytes[16..20].copy_from_slice(&0x80_u32.to_be_bytes());
        for (cursor, id, stride) in [(0x80, 10_u64, 0x200_u32), (0x280, 20, 0x180)] {
            bytes[cursor..cursor + 8].copy_from_slice(&id.to_be_bytes());
            bytes[cursor + 8..cursor + 12].copy_from_slice(&0x40_u32.to_be_bytes());
            bytes[cursor + 12..cursor + 16].copy_from_slice(&0x80_u32.to_be_bytes());
            bytes[cursor + 16..cursor + 20].copy_from_slice(&stride.to_be_bytes());
        }

        let stream = StreamFile::parse(&bytes, 2).unwrap();
        assert_eq!(stream.assets[0].data_range, 0x100..0x140);
        assert_eq!(stream.assets[1].header_offset, 0x280);
        assert_eq!(stream.assets[1].id, 20);
    }

    #[test]
    fn parses_payload_relative_blocks_and_counts_final_stride() {
        let first = payload_block(10, 0x30, 0x20, b"first");
        let second = payload_block(20, 0x28, 0x18, b"second");
        let mut bytes = first;
        bytes.extend_from_slice(&second);
        bytes.extend_from_slice(b"trailing");

        let payload = CollectionPayload::parse(&bytes, 2).unwrap();

        assert_eq!(payload.consumed_size, 0x58);
        assert_eq!(payload.assets.len(), 2);
        assert_eq!(payload.assets[0].header_offset, 0);
        assert_eq!(payload.assets[0].data_range, 0x20..0x25);
        assert_eq!(payload.assets[1].header_offset, 0x30);
        assert_eq!(payload.assets[1].data_range, 0x48..0x4e);
        assert_eq!(&bytes[payload.assets[1].data_range.clone()], b"second");
    }

    #[test]
    fn payload_parser_accepts_exact_stride_boundaries() {
        let mut minimum = payload_block(1, u32::try_from(COLLECTION_ASSET_SIZE).unwrap(), 24, b"");
        minimum.push(0);
        let parsed = CollectionPayload::parse(&minimum, 1).unwrap();
        assert_eq!(parsed.consumed_size, COLLECTION_ASSET_SIZE);

        let mut maximum = payload_block(2, MAX_COLLECTION_ASSET_STRIDE, 24, b"");
        maximum.push(0);
        let parsed = CollectionPayload::parse(&maximum, 1).unwrap();
        assert_eq!(parsed.consumed_size, MAX_COLLECTION_ASSET_STRIDE as usize);
    }

    #[test]
    fn payload_parser_rejects_zero_stride_including_final_block() {
        let mut first_zero = payload_block(1, 0, 24, b"");
        first_zero.push(0);
        assert_eq!(
            CollectionPayload::parse(&first_zero, 1),
            Err(StreamTableError::ZeroAssetStride { index: 0 })
        );

        let mut final_zero = payload_block(1, 0x20, 24, b"");
        final_zero.extend_from_slice(&payload_block(2, 0, 24, b""));
        final_zero.push(0);
        assert_eq!(
            CollectionPayload::parse(&final_zero, 2),
            Err(StreamTableError::ZeroAssetStride { index: 1 })
        );
    }

    #[test]
    fn payload_parser_rejects_oversized_stride_including_final_block() {
        let too_large = MAX_COLLECTION_ASSET_STRIDE + 1;
        let mut first = payload_block(1, too_large, 24, b"");
        first.push(0);
        assert_eq!(
            CollectionPayload::parse(&first, 1),
            Err(StreamTableError::AssetStrideTooLarge {
                index: 0,
                stride: too_large,
                maximum: MAX_COLLECTION_ASSET_STRIDE,
            })
        );

        let mut final_block = payload_block(1, 0x20, 24, b"");
        final_block.extend_from_slice(&payload_block(2, too_large, 24, b""));
        final_block.push(0);
        assert_eq!(
            CollectionPayload::parse(&final_block, 2),
            Err(StreamTableError::AssetStrideTooLarge {
                index: 1,
                stride: too_large,
                maximum: MAX_COLLECTION_ASSET_STRIDE,
            })
        );
    }

    #[test]
    fn payload_parser_rejects_native_parse_window_of_24_or_less() {
        let exact_header = payload_block(1, 24, 24, b"");
        assert_eq!(
            CollectionPayload::parse(&exact_header, 1),
            Err(StreamTableError::AssetParseWindowTooSmall {
                index: 0,
                available: 24,
            })
        );
        assert_eq!(
            CollectionPayload::parse(&exact_header[..23], 1),
            Err(StreamTableError::AssetParseWindowTooSmall {
                index: 0,
                available: 23,
            })
        );
    }

    #[test]
    fn payload_parser_rejects_stride_outside_supplied_window() {
        let mut bytes = payload_block(1, 0x30, 24, b"");
        bytes.truncate(0x2f);
        assert_eq!(
            CollectionPayload::parse(&bytes, 1),
            Err(StreamTableError::AssetStrideOutsidePayload { index: 0 })
        );
    }

    #[test]
    fn payload_parser_rejects_data_before_header_or_beyond_stride() {
        let mut before_header = payload_block(1, 0x20, 20, b"x");
        before_header.push(0);
        assert_eq!(
            CollectionPayload::parse(&before_header, 1),
            Err(StreamTableError::AssetDataOutsideStride { index: 0 })
        );

        let mut beyond_stride = payload_block(1, 0x20, 0x1f, b"ab");
        beyond_stride.push(0);
        assert_eq!(
            CollectionPayload::parse(&beyond_stride, 1),
            Err(StreamTableError::AssetDataOutsideStride { index: 0 })
        );
    }

    #[test]
    fn payload_parser_reports_missing_declared_block_without_consuming_trailing_data() {
        let mut bytes = payload_block(1, 0x20, 24, b"");
        bytes.push(0);
        assert_eq!(
            CollectionPayload::parse(&bytes, 2),
            Err(StreamTableError::AssetCountOutsidePayload {
                count: 2,
                payload_len: 33,
            })
        );

        let empty = CollectionPayload::parse(b"diagnostic trailing bytes", 0).unwrap();
        assert!(empty.assets.is_empty());
        assert_eq!(empty.consumed_size, 0);
    }

    #[test]
    fn payload_parser_rejects_impossible_count_before_reserving() {
        assert_eq!(
            CollectionPayload::parse(&[0_u8; COLLECTION_ASSET_SIZE * 2], 2),
            Err(StreamTableError::AssetCountOutsidePayload {
                count: 2,
                payload_len: COLLECTION_ASSET_SIZE * 2,
            })
        );
        assert_eq!(
            CollectionPayload::parse(&[0_u8; COLLECTION_ASSET_SIZE + 1], u32::MAX),
            Err(StreamTableError::AssetCountOutsidePayload {
                count: u32::MAX,
                payload_len: COLLECTION_ASSET_SIZE + 1,
            })
        );

        let empty = CollectionPayload::parse(&[], 0).unwrap();
        assert!(empty.assets.is_empty());
        assert_eq!(empty.consumed_size, 0);
    }

    #[test]
    fn existing_stream_file_parser_still_accepts_zero_final_stride() {
        let mut bytes = vec![0_u8; 0xa0];
        bytes[..4].copy_from_slice(b"SFIL");
        bytes[16..20].copy_from_slice(&0x80_u32.to_be_bytes());
        bytes[0x80 + 12..0x80 + 16].copy_from_slice(&24_u32.to_be_bytes());

        let parsed = StreamFile::parse(&bytes, 1).unwrap();

        assert_eq!(parsed.assets[0].stride, 0);
        assert_eq!(parsed.assets[0].data_range, 0x98..0x98);
    }

    #[test]
    fn parses_compressed_arena_segments() {
        let mut bytes = [0_u8; 80];
        bytes[..4].copy_from_slice(&0x6dc_u32.to_be_bytes());
        bytes[4..8].copy_from_slice(&0x6dc_u32.to_be_bytes());

        let header = CompressedArenaHeader::parse(&bytes).unwrap();
        assert_eq!(header.segments[0].destination_size, 0x6dc);
        assert!(!header.segments[0].compressed);
        assert_eq!(header.segments[1].source_size, 0);
    }

    #[test]
    fn expands_raw_and_refpack_arena_segments() {
        let compressed = b"\x10\xfb\x00\x00\x06\xe0abcd\xfeef";
        let mut bytes = vec![0_u8; 80];
        bytes[..4].copy_from_slice(&6_u32.to_be_bytes());
        bytes[4..8].copy_from_slice(&u32::try_from(compressed.len()).unwrap().to_be_bytes());
        bytes[8..12].copy_from_slice(&1_u32.to_be_bytes());
        bytes[16..20].copy_from_slice(&3_u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&3_u32.to_be_bytes());
        bytes.extend_from_slice(compressed);
        bytes.extend_from_slice(b"raw");

        let arena = decode_arena(&bytes).unwrap();
        assert_eq!(arena.resources[0], b"abcdef");
        assert_eq!(arena.resources[1], b"raw");
        assert!(arena.resources[2].is_empty());
    }
}
