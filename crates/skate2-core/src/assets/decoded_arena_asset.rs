//! Generic streamed-arena construction before subsystem registration.
//!
//! World Presentation, Simulation, and Texture registrations all stream the
//! same XST asset type through compressed-arena format 2 in the validated BAM
//! corpus. This module owns that common decode/RW4-parse boundary without
//! inventing the later per-subsystem registration side effects.

use crate::{
    collection_load_pump::CollectionStreamFormat,
    resident_collection_load_driver::{
        CollectionAssetBuildInput, CollectionAssetBuildMode, CollectionAssetBuilder,
    },
};
use skate2_formats::{
    rw4::{Arena, ArenaError},
    stream::{ArenaDecodeError, ArenaResources, BaseResourceDescriptor, decode_arena},
};
use std::{error::Error, fmt};

/// XST asset type observed for every arena in the validated BAM Presentation,
/// Simulation, and Texture tables.
pub const WORLD_STREAM_ARENA_ASSET_TYPE: u32 = 0xab32_9a6a;

/// Metadata needed to validate one decoded streamed arena.
pub trait StreamArenaAssetMetadata {
    fn asset_id(&self) -> u64;
    fn asset_type(&self) -> u32;
    fn resource_descriptors(&self) -> [BaseResourceDescriptor; 5];
}

impl StreamArenaAssetMetadata for crate::world_stream_catalog::AssetRegistrationSnapshot {
    fn asset_id(&self) -> u64 {
        self.entry.id
    }

    fn asset_type(&self) -> u32 {
        self.entry.asset_type
    }

    fn resource_descriptors(&self) -> [BaseResourceDescriptor; 5] {
        self.entry.resources
    }
}

/// One generic materialized arena, still awaiting stream-kind-specific
/// registration and runtime ownership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedArenaAsset {
    pub asset_id: u64,
    pub asset_type: u32,
    /// Exact five size/alignment pairs retained from the owning XST entry.
    ///
    /// These describe the asset manager's resource-allocation requests. They are
    /// intentionally distinct from the descriptors embedded in [`Arena`].
    pub asset_resource_descriptors: [BaseResourceDescriptor; 5],
    pub resources: ArenaResources,
    pub arena: Arena,
}

#[cfg(test)]
impl DecodedArenaAsset {
    /// Constructs a synthetic decoded owner for downstream unit fixtures.
    ///
    /// Tests which have no XST entry derive descriptor sizes from their actual
    /// retained buffers and copy embedded alignments as an explicit fixture
    /// policy. Production decoding must instead retain the independently
    /// authored XST descriptors through [`build_arena`].
    pub(crate) fn from_parts_for_test(
        asset_id: u64,
        asset_type: u32,
        resources: ArenaResources,
        arena: Arena,
    ) -> Self {
        let asset_resource_descriptors = std::array::from_fn(|index| BaseResourceDescriptor {
            size: u32::try_from(resources.resources[index].len())
                .expect("synthetic arena resource length must fit the XST size domain"),
            alignment: arena.resource_descriptor[index].alignment,
        });
        Self {
            asset_id,
            asset_type,
            asset_resource_descriptors,
            resources,
            arena,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedArenaAssetError {
    UnsupportedBuildMode {
        raw_format: i32,
        mode: CollectionAssetBuildMode,
    },
    AssetIdentityMismatch {
        requested: u64,
        metadata: u64,
    },
    UnsupportedAssetType(u32),
    StoredSizeMismatch {
        declared: u32,
        actual: usize,
    },
    Decode(ArenaDecodeError),
    ResourceSizeMismatch {
        index: usize,
        expected: u32,
        actual: usize,
    },
    Parse(ArenaError),
}

impl fmt::Display for DecodedArenaAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedBuildMode { raw_format, mode } => write!(
                formatter,
                "streamed arena requires raw format 2/CompressedArena, observed {raw_format}/{mode:?}"
            ),
            Self::AssetIdentityMismatch {
                requested,
                metadata,
            } => write!(
                formatter,
                "streamed arena request {requested:#018x} used metadata for {metadata:#018x}"
            ),
            Self::UnsupportedAssetType(asset_type) => write!(
                formatter,
                "streamed arena has unsupported XST asset type {asset_type:#010x}"
            ),
            Self::StoredSizeMismatch { declared, actual } => write!(
                formatter,
                "streamed arena exposes {actual:#x} payload bytes but its request declares {declared:#x}"
            ),
            Self::Decode(source) => write!(formatter, "arena envelope decode failed: {source}"),
            Self::ResourceSizeMismatch {
                index,
                expected,
                actual,
            } => write!(
                formatter,
                "arena resource {index} decoded to {actual:#x} bytes; XST declares {expected:#x}"
            ),
            Self::Parse(source) => write!(formatter, "RW4 arena parse failed: {source}"),
        }
    }
}

impl Error for DecodedArenaAssetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(source) => Some(source),
            Self::Parse(source) => Some(source),
            _ => None,
        }
    }
}

/// Stateless constructor for the common native compressed-arena path.
#[derive(Clone, Copy, Debug, Default)]
pub struct DecodedArenaAssetBuilder;

impl<I: StreamArenaAssetMetadata> CollectionAssetBuilder<I> for DecodedArenaAssetBuilder {
    type Asset = DecodedArenaAsset;
    type Error = DecodedArenaAssetError;

    fn build(
        &mut self,
        input: CollectionAssetBuildInput<'_, I>,
    ) -> Result<Option<Self::Asset>, Self::Error> {
        build_arena(
            input.asset_id,
            input.asset_info,
            input.payload,
            input.stored_size,
            input.raw_format,
            input.mode,
        )
        .map(Some)
    }
}

fn build_arena<I: StreamArenaAssetMetadata>(
    asset_id: u64,
    asset_info: &I,
    payload: &[u8],
    stored_size: u32,
    raw_format: CollectionStreamFormat,
    mode: CollectionAssetBuildMode,
) -> Result<DecodedArenaAsset, DecodedArenaAssetError> {
    if raw_format != CollectionStreamFormat(2) || mode != CollectionAssetBuildMode::CompressedArena
    {
        return Err(DecodedArenaAssetError::UnsupportedBuildMode {
            raw_format: raw_format.0,
            mode,
        });
    }
    let metadata_id = asset_info.asset_id();
    if metadata_id != asset_id {
        return Err(DecodedArenaAssetError::AssetIdentityMismatch {
            requested: asset_id,
            metadata: metadata_id,
        });
    }
    let asset_type = asset_info.asset_type();
    if asset_type != WORLD_STREAM_ARENA_ASSET_TYPE {
        return Err(DecodedArenaAssetError::UnsupportedAssetType(asset_type));
    }
    if usize::try_from(stored_size).ok() != Some(payload.len()) {
        return Err(DecodedArenaAssetError::StoredSizeMismatch {
            declared: stored_size,
            actual: payload.len(),
        });
    }

    let mut resources = decode_arena(payload).map_err(DecodedArenaAssetError::Decode)?;
    let asset_resource_descriptors = asset_info.resource_descriptors();
    for (index, (descriptor, resource)) in asset_resource_descriptors
        .iter()
        .zip(&mut resources.resources)
        .enumerate()
    {
        // The stream allocates each resource at the XST's size and decompresses the arena's
        // segment into it: a segment shorter than that (some downloadable content's PROXY
        // collections are 0x10 short) leaves the rest of the allocation zero. A longer one
        // would overrun it.
        let expected = descriptor.size;
        match usize::try_from(expected) {
            Ok(size) if size >= resource.len() => resource.resize(size, 0),
            _ => {
                return Err(DecodedArenaAssetError::ResourceSizeMismatch {
                    index,
                    expected,
                    actual: resource.len(),
                });
            }
        }
    }
    let arena = Arena::parse(&resources.resources).map_err(DecodedArenaAssetError::Parse)?;
    Ok(DecodedArenaAsset {
        asset_id,
        asset_type,
        asset_resource_descriptors,
        resources,
        arena,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Metadata {
        id: u64,
        asset_type: u32,
        resource_descriptors: [BaseResourceDescriptor; 5],
    }

    impl StreamArenaAssetMetadata for Metadata {
        fn asset_id(&self) -> u64 {
            self.id
        }

        fn asset_type(&self) -> u32 {
            self.asset_type
        }

        fn resource_descriptors(&self) -> [BaseResourceDescriptor; 5] {
            self.resource_descriptors
        }
    }

    fn descriptors(sizes: [u32; 5], alignments: [u32; 5]) -> [BaseResourceDescriptor; 5] {
        std::array::from_fn(|index| BaseResourceDescriptor {
            size: sizes[index],
            alignment: alignments[index],
        })
    }

    fn arena_resource() -> Vec<u8> {
        let mut bytes = vec![0_u8; 0x200];
        bytes[..12].copy_from_slice(b"\x89RW4xb2\0\r\n\x1a\n");
        bytes[12] = 1;
        bytes[13] = 32;
        bytes[14] = 4;
        bytes[16..20].copy_from_slice(b"454\0");
        bytes[20..24].copy_from_slice(b"000\0");
        bytes[28..32].copy_from_slice(&0x8646_a1f7_u32.to_be_bytes());
        bytes[32..36].copy_from_slice(&1_u32.to_be_bytes());
        bytes[36..40].copy_from_slice(&1_u32.to_be_bytes());
        bytes[40..44].copy_from_slice(&16_u32.to_be_bytes());
        bytes[48..52].copy_from_slice(&0x1e8_u32.to_be_bytes());
        bytes[68..72].copy_from_slice(&0x200_u32.to_be_bytes());
        bytes[72..76].copy_from_slice(&16_u32.to_be_bytes());
        bytes[0x1e8..0x1ec].copy_from_slice(&0x100_u32.to_be_bytes());
        bytes[0x1f0..0x1f4].copy_from_slice(&0x10_u32.to_be_bytes());
        bytes[0x1f4..0x1f8].copy_from_slice(&16_u32.to_be_bytes());
        bytes[0x1f8..0x1fc].copy_from_slice(&0x10_u32.to_be_bytes());
        bytes[0x1fc..0x200].copy_from_slice(&0x00eb_0008_u32.to_be_bytes());
        bytes
    }

    fn envelope(resource: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0_u8; 80];
        let size = u32::try_from(resource.len()).unwrap();
        bytes[..4].copy_from_slice(&size.to_be_bytes());
        bytes[4..8].copy_from_slice(&size.to_be_bytes());
        bytes.extend_from_slice(resource);
        bytes
    }

    fn build(
        metadata: Metadata,
        payload: &[u8],
        format: i32,
    ) -> Result<Option<DecodedArenaAsset>, DecodedArenaAssetError> {
        build_arena(
            7,
            &metadata,
            payload,
            u32::try_from(payload.len()).unwrap(),
            CollectionStreamFormat(format),
            CollectionAssetBuildMode::from_raw(CollectionStreamFormat(format)),
        )
        .map(Some)
    }

    #[test]
    fn builds_exact_format_two_arena_and_preserves_resources() {
        let resource = arena_resource();
        let payload = envelope(&resource);
        let asset = build(
            Metadata {
                id: 7,
                asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
                resource_descriptors: descriptors(
                    [0x200, 0, 0, 0, 0],
                    [0x80, 0x20, 0x40, 0x100, 0x200],
                ),
            },
            &payload,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(asset.asset_id, 7);
        assert_eq!(asset.asset_type, WORLD_STREAM_ARENA_ASSET_TYPE);
        assert_eq!(asset.arena.id, 0x8646_a1f7);
        assert_eq!(asset.arena.dictionary.len(), 1);
        assert_eq!(asset.resources.resources[0], resource);
        assert_eq!(
            asset.asset_resource_descriptors,
            descriptors([0x200, 0, 0, 0, 0], [0x80, 0x20, 0x40, 0x100, 0x200])
        );
        assert_eq!(asset.asset_resource_descriptors[0].alignment, 0x80);
        assert_eq!(asset.arena.resource_descriptor[0].alignment, 0x10);
        assert_ne!(
            asset.asset_resource_descriptors[0].alignment,
            asset.arena.resource_descriptor[0].alignment
        );
    }

    #[test]
    fn synthetic_constructor_derives_sizes_and_labels_embedded_alignment_policy() {
        let resource = arena_resource();
        let resources = ArenaResources {
            resources: [resource.clone(), vec![1, 2, 3], vec![], vec![4], vec![]],
        };
        let mut arena =
            Arena::parse(&[resource, Vec::new(), Vec::new(), Vec::new(), Vec::new()]).unwrap();
        arena.resource_descriptor[0].alignment = 0x100;
        arena.resource_descriptor[1].alignment = 0x20;
        arena.resource_descriptor[2].alignment = 0x40;
        arena.resource_descriptor[3].alignment = 0x80;
        arena.resource_descriptor[4].alignment = 0x200;

        let asset = DecodedArenaAsset::from_parts_for_test(
            7,
            WORLD_STREAM_ARENA_ASSET_TYPE,
            resources,
            arena,
        );

        assert_eq!(
            asset.asset_resource_descriptors,
            descriptors([0x200, 3, 0, 1, 0], [0x100, 0x20, 0x40, 0x80, 0x200])
        );
    }

    #[test]
    fn rejects_non_arena_mode_identity_type_and_xst_size_mismatch() {
        let resource = arena_resource();
        let payload = envelope(&resource);
        let metadata = Metadata {
            id: 7,
            asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
            resource_descriptors: descriptors([0x200, 0, 0, 0, 0], [0x80, 1, 1, 1, 1]),
        };
        assert!(matches!(
            build(metadata, &payload, 1),
            Err(DecodedArenaAssetError::UnsupportedBuildMode { .. })
        ));
        assert!(matches!(
            build(Metadata { id: 8, ..metadata }, &payload, 2),
            Err(DecodedArenaAssetError::AssetIdentityMismatch { .. })
        ));
        assert_eq!(
            build(
                Metadata {
                    asset_type: 0x1234,
                    ..metadata
                },
                &payload,
                2,
            ),
            Err(DecodedArenaAssetError::UnsupportedAssetType(0x1234))
        );
        assert!(matches!(
            build(
                Metadata {
                    resource_descriptors: descriptors([0x1ff, 0, 0, 0, 0], [0x80, 1, 1, 1, 1],),
                    ..metadata
                },
                &payload,
                2,
            ),
            Err(DecodedArenaAssetError::ResourceSizeMismatch {
                index: 0,
                expected: 0x1ff,
                actual: 0x200,
            })
        ));
        assert_eq!(
            build_arena(
                7,
                &metadata,
                &payload,
                u32::try_from(payload.len() - 1).unwrap(),
                CollectionStreamFormat(2),
                CollectionAssetBuildMode::CompressedArena,
            ),
            Err(DecodedArenaAssetError::StoredSizeMismatch {
                declared: u32::try_from(payload.len() - 1).unwrap(),
                actual: payload.len(),
            })
        );
    }

    #[test]
    fn surfaces_envelope_and_rw4_parse_errors_without_null_substitution() {
        let metadata = Metadata {
            id: 7,
            asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
            resource_descriptors: descriptors([0, 0, 0, 0, 0], [1, 1, 1, 1, 1]),
        };
        assert!(matches!(
            build(metadata, &[0; 79], 2),
            Err(DecodedArenaAssetError::Decode(_))
        ));

        let payload = envelope(&[0; 0x200]);
        assert!(matches!(
            build(
                Metadata {
                    resource_descriptors: descriptors([0x1ff, 0, 0, 0, 0], [0x80, 1, 1, 1, 1],),
                    ..metadata
                },
                &payload,
                2,
            ),
            Err(DecodedArenaAssetError::ResourceSizeMismatch {
                index: 0,
                expected: 0x1ff,
                actual: 0x200,
            })
        ));
        assert!(matches!(
            build(
                Metadata {
                    resource_descriptors: descriptors([0x200, 0, 0, 0, 0], [0x80, 1, 1, 1, 1],),
                    ..metadata
                },
                &payload,
                2,
            ),
            Err(DecodedArenaAssetError::Parse(_))
        ));
    }
}
