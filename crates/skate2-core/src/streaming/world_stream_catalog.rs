//! Shared ownership catalog for parsed world-stream registrations.
//!
//! `cGameStream::Update` (`0x82d5db60`) registers XSM collections before
//! beginning XST and later registers every XST asset occurrence. The native
//! collection map has a 2,048-node cap; duplicate collection IDs are rejected,
//! while duplicate asset IDs add references and preserve their first metadata.
//! The relevant callees are `cAssetStreamer::AddCollectionEntries`
//! (`0x82d5aca0`), `cAssetManager::AddAssetInfo` (`0x82d643d8`), and
//! `cAssetManager::RemoveAssetInfoRef` (`0x82d644d8`). XSF path construction is
//! at `0x82d5ad14`; `cCollectionMapInfo` validates its 0x60-byte C buffer at
//! `0x82d54048`. `AddAssetInfo` increments signed `mRefCount` at `0x82d64444`;
//! `RemoveAssetInfoRef` checks signed `<= 0` at `0x82d64550` before its
//! decrement at `0x82d64570..0x82d64574`.
//! For failures returned as [`CatalogError`], each callback validates its full
//! logical change before mutation. This does not claim atomic recovery from a
//! process panic or allocator abort.
//! The single-lock snapshots and poison reporting are host policy, not claims
//! about the reference's separate synchronization primitives. No external
//! collaborator is invoked while that lock is held.
//! `is_collection_valid` intentionally remains outside this catalog: the
//! adapter's processed-XSS publication store owns that distinct query.

use crate::{
    game_stream::{RegistrationFileId, StreamKind, StreamPaths, StreamRegistrationContext},
    mounted_stream_backend::StreamRegistrationSink,
};
use skate2_formats::stream::{
    AssetMapEntry, AssetToc, CollectionMap, CollectionMapEntry, CollectionSpace,
};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::{Arc, Mutex, MutexGuard},
};

/// Fixed node capacity of the reference collection-registration hash map.
pub const COLLECTION_CAPACITY: usize = 2_048;
/// Largest completed XSF path before its terminating NUL.
pub const MAX_COLLECTION_PATH_BYTES: usize = 95;

/// A cloneable reader and [`StreamRegistrationSink`] over one shared catalog.
#[derive(Clone, Debug, Default)]
pub struct WorldStreamCatalog {
    state: Arc<Mutex<CatalogState>>,
}

#[derive(Debug, Default)]
struct CatalogState {
    files: HashMap<RegistrationFileId, FileRegistration>,
    collections: HashMap<u64, CollectionRegistration>,
    assets: HashMap<u64, AssetRegistration>,
}

#[derive(Debug)]
struct FileRegistration {
    kind: StreamKind,
    paths: Arc<StreamPaths>,
    collection_map: Option<Arc<CollectionMap>>,
    collection_ids: Arc<[u64]>,
    collections_active: bool,
    asset_toc: Option<Arc<AssetToc>>,
    asset_ids: Arc<[u64]>,
    assets_active: bool,
    collection_space: Option<Arc<CollectionSpace>>,
    stream_xml: Option<Arc<[u8]>>,
    phase: FileRegistrationPhase,
}

impl FileRegistration {
    fn new(context: StreamRegistrationContext<'_>) -> Self {
        Self {
            kind: context.kind(),
            paths: Arc::new(context.paths().clone()),
            collection_map: None,
            collection_ids: Arc::from([]),
            collections_active: false,
            asset_toc: None,
            asset_ids: Arc::from([]),
            assets_active: false,
            collection_space: None,
            stream_xml: None,
            phase: FileRegistrationPhase::Registered,
        }
    }

    fn context_matches(&self, context: StreamRegistrationContext<'_>) -> bool {
        self.kind == context.kind() && self.paths.as_ref() == context.paths()
    }
}

#[derive(Clone, Debug)]
struct CollectionRegistration {
    owner: RegistrationFileId,
    source_index: usize,
    path: Arc<str>,
    entry: CollectionMapEntry,
    stream_format: u32,
}

#[derive(Clone, Debug)]
struct AssetRegistration {
    first_owner: RegistrationFileId,
    entry: AssetMapEntry,
    reference_count: i32,
}

/// File-scoped registration/unload phase. Native `cGameStream::Update`
/// unregisters XSM collections before XST assets (`0x82d5e050..0x82d5e0d0`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileRegistrationPhase {
    Registered,
    CollectionsUnloaded,
    AssetsUnloaded,
}

/// Registration stage named in a typed catalog invariant error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogStage {
    CollectionMap,
    AssetToc,
    CollectionSpace,
    StreamXml,
}

/// Safe-boundary catalog error. Native assertion/partial-prefix behavior is
/// deliberately not reproduced for invalid inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogError {
    Poisoned,
    UnknownFile,
    ContextMismatch,
    StageAlreadyRegistered(CatalogStage),
    StageNotRegistered(CatalogStage),
    DuplicateCollectionId(u64),
    CollectionCapacityExceeded {
        attempted: usize,
        capacity: usize,
    },
    CollectionCountOverflow,
    CollectionPathContainsNul {
        id: u64,
    },
    CollectionPathTooLong {
        id: u64,
        byte_len: usize,
    },
    AssetReferenceOverflow(u64),
    InvalidAssetReferenceCount {
        id: u64,
        count: i32,
    },
    AssetReferenceTotalOverflow,
    CollectionOwnershipMismatch(u64),
    AssetReferenceMismatch(u64),
    CollectionMapMismatch,
    AssetTocMismatch,
    CollectionSpaceMismatch,
    StreamXmlOwnershipMismatch,
    ActiveRegistrationsRemain,
    LoadAfterUnloadStarted {
        stage: CatalogStage,
        phase: FileRegistrationPhase,
    },
    InvalidUnloadOrder {
        stage: CatalogStage,
        phase: FileRegistrationPhase,
    },
}

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Poisoned => formatter.write_str("world-stream catalog lock was poisoned"),
            Self::UnknownFile => formatter.write_str("unknown stream-file registration identity"),
            Self::ContextMismatch => {
                formatter.write_str("stream-file registration context changed")
            }
            Self::StageAlreadyRegistered(stage) => {
                write!(formatter, "{stage:?} already registered")
            }
            Self::StageNotRegistered(stage) => write!(formatter, "{stage:?} is not registered"),
            Self::DuplicateCollectionId(id) => {
                write!(formatter, "duplicate collection ID {id:#018x}")
            }
            Self::CollectionCapacityExceeded {
                attempted,
                capacity,
            } => write!(
                formatter,
                "collection count {attempted} exceeds fixed capacity {capacity}"
            ),
            Self::CollectionCountOverflow => {
                formatter.write_str("collection count overflow while validating fixed capacity")
            }
            Self::CollectionPathContainsNul { id } => {
                write!(formatter, "collection {id:#018x} filename contains NUL")
            }
            Self::CollectionPathTooLong { id, byte_len } => write!(
                formatter,
                "collection {id:#018x} XSF path has {byte_len} bytes; maximum is {MAX_COLLECTION_PATH_BYTES}"
            ),
            Self::AssetReferenceOverflow(id) => {
                write!(
                    formatter,
                    "asset {id:#018x} signed reference count overflow"
                )
            }
            Self::InvalidAssetReferenceCount { id, count } => {
                write!(
                    formatter,
                    "asset {id:#018x} has invalid signed reference count {count}"
                )
            }
            Self::AssetReferenceTotalOverflow => {
                formatter.write_str("asset reference snapshot total overflow")
            }
            Self::CollectionOwnershipMismatch(id) => {
                write!(formatter, "collection {id:#018x} ownership mismatch")
            }
            Self::AssetReferenceMismatch(id) => {
                write!(formatter, "asset {id:#018x} reference ownership mismatch")
            }
            Self::CollectionMapMismatch => formatter.write_str("collection-map unload mismatch"),
            Self::AssetTocMismatch => formatter.write_str("asset-TOC unload mismatch"),
            Self::CollectionSpaceMismatch => {
                formatter.write_str("collection-space unload mismatch")
            }
            Self::StreamXmlOwnershipMismatch => {
                formatter.write_str("stream-XML ownership mismatch")
            }
            Self::ActiveRegistrationsRemain => formatter
                .write_str("stream file still owns active collection or asset registrations"),
            Self::LoadAfterUnloadStarted { stage, phase } => {
                write!(
                    formatter,
                    "cannot register {stage:?} after unload entered {phase:?}"
                )
            }
            Self::InvalidUnloadOrder { stage, phase } => {
                write!(
                    formatter,
                    "cannot unregister {stage:?} while file is in {phase:?}"
                )
            }
        }
    }
}

impl std::error::Error for CatalogError {}

/// Owning snapshot of one registered collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionRegistrationSnapshot {
    /// Full map key, retained so an operation cannot pair metadata with a
    /// different dispatch identity.
    pub id: u64,
    pub owner: RegistrationFileId,
    pub source_index: usize,
    pub path: Arc<str>,
    pub offset: u64,
    pub size: u32,
    pub asset_count: u32,
    pub stream_format: u32,
    pub parent_id: u64,
}

/// Owning snapshot of one global asset entry and its current references.
#[derive(Clone, Debug)]
pub struct AssetRegistrationSnapshot {
    pub first_owner: RegistrationFileId,
    /// Owning copy of the first parsed source entry. The reference's observed
    /// global registration consumes its type and five resource descriptors;
    /// retaining the remaining parsed fields is host-side diagnostic ownership.
    pub entry: AssetMapEntry,
    /// Exact signed `cLoadedAssetInfo::mRefCount` domain.
    pub reference_count: i32,
}

/// Owning snapshot of all registration state retained for one stream file.
#[derive(Clone, Debug, PartialEq)]
pub struct FileRegistrationSnapshot {
    pub id: RegistrationFileId,
    pub kind: StreamKind,
    pub paths: Arc<StreamPaths>,
    pub collection_map: Option<Arc<CollectionMap>>,
    pub collection_ids: Arc<[u64]>,
    pub collections_active: bool,
    pub asset_toc: Option<Arc<AssetToc>>,
    pub asset_ids: Arc<[u64]>,
    pub assets_active: bool,
    /// Opaque owning clone of the parsed XSS, with all float and LOD bits
    /// unchanged. Filtering belongs to the later publication adapter.
    pub collection_space: Option<Arc<CollectionSpace>>,
    /// `Some` means the XML load succeeded; an empty slice remains `Some`.
    pub stream_xml: Option<Arc<[u8]>>,
    pub phase: FileRegistrationPhase,
}

/// Coherent counts sampled while holding the catalog's one host-policy lock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogCounts {
    pub files: usize,
    pub collections: usize,
    pub asset_entries: usize,
    pub asset_references: usize,
    pub collection_spaces: usize,
    pub stream_xml_files: usize,
}

impl WorldStreamCatalog {
    /// Constructs an empty shared catalog.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> Result<MutexGuard<'_, CatalogState>, CatalogError> {
        self.state.lock().map_err(|_| CatalogError::Poisoned)
    }

    /// Returns the registered collection payload offset widened from its raw
    /// XSM `u32`, matching `cAssetStreamManager::GetStreamOffset`
    /// (`0x82d599f0`).
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError::Poisoned`] if a prior panic poisoned the shared
    /// host-policy lock.
    pub fn stream_offset(&self, id: u64) -> Result<Option<u64>, CatalogError> {
        Ok(self
            .lock()?
            .collections
            .get(&id)
            .map(|item| u64::from(item.entry.payload_offset)))
    }

    /// Returns the raw XSM collection payload size, matching
    /// `cAssetStreamManager::GetStreamSize` (`0x82d599f8`).
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError::Poisoned`] if a prior panic poisoned the shared
    /// host-policy lock.
    pub fn stream_size(&self, id: u64) -> Result<Option<u32>, CatalogError> {
        Ok(self
            .lock()?
            .collections
            .get(&id)
            .map(|item| item.entry.payload_size))
    }

    /// Returns an owning snapshot of a collection registration.
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError::Poisoned`] if a prior panic poisoned the shared
    /// host-policy lock.
    pub fn collection(
        &self,
        id: u64,
    ) -> Result<Option<CollectionRegistrationSnapshot>, CatalogError> {
        Ok(self
            .lock()?
            .collections
            .get(&id)
            .map(|item| CollectionRegistrationSnapshot {
                id: item.entry.id,
                owner: item.owner.clone(),
                source_index: item.source_index,
                path: Arc::clone(&item.path),
                offset: u64::from(item.entry.payload_offset),
                size: item.entry.payload_size,
                asset_count: item.entry.asset_count,
                stream_format: item.stream_format,
                parent_id: item.entry.parent_id,
            }))
    }

    /// Returns an owning snapshot of global first-wins asset metadata.
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError::Poisoned`] if a prior panic poisoned the shared
    /// host-policy lock.
    pub fn asset(&self, id: u64) -> Result<Option<AssetRegistrationSnapshot>, CatalogError> {
        Ok(self
            .lock()?
            .assets
            .get(&id)
            .map(|item| AssetRegistrationSnapshot {
                first_owner: item.first_owner.clone(),
                entry: item.entry.clone(),
                reference_count: item.reference_count,
            }))
    }

    /// Returns an owning snapshot of a live file registration.
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError::Poisoned`] if a prior panic poisoned the shared
    /// host-policy lock.
    pub fn file(
        &self,
        id: &RegistrationFileId,
    ) -> Result<Option<FileRegistrationSnapshot>, CatalogError> {
        Ok(self
            .lock()?
            .files
            .get(id)
            .map(|file| FileRegistrationSnapshot {
                id: id.clone(),
                kind: file.kind,
                paths: Arc::clone(&file.paths),
                collection_map: file.collection_map.as_ref().map(Arc::clone),
                collection_ids: Arc::clone(&file.collection_ids),
                collections_active: file.collections_active,
                asset_toc: file.asset_toc.as_ref().map(Arc::clone),
                asset_ids: Arc::clone(&file.asset_ids),
                assets_active: file.assets_active,
                collection_space: file.collection_space.as_ref().map(Arc::clone),
                stream_xml: file.stream_xml.as_ref().map(Arc::clone),
                phase: file.phase,
            }))
    }

    /// The registered file of `kind` with these paths: a stream's own file of that kind, found
    /// from the paths its basename derives (`StreamPaths::derive`), as the native finds a game
    /// stream's `cStreamFile` through its handle.
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError::Poisoned`] if a prior panic poisoned the shared
    /// host-policy lock.
    pub fn file_with_paths(
        &self,
        kind: StreamKind,
        paths: &StreamPaths,
    ) -> Result<Option<FileRegistrationSnapshot>, CatalogError> {
        let id = self
            .lock()?
            .files
            .iter()
            .find(|(_, file)| file.kind == kind && file.paths.as_ref() == paths)
            .map(|(id, _)| id.clone());
        match id {
            Some(id) => self.file(&id),
            None => Ok(None),
        }
    }

    /// Returns coherent catalog counts from one lock acquisition.
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError::Poisoned`] if a prior panic poisoned the shared
    /// host-policy lock.
    pub fn counts(&self) -> Result<CatalogCounts, CatalogError> {
        let state = self.lock()?;
        let asset_references = state
            .assets
            .iter()
            .try_fold(0usize, |total, (&id, asset)| {
                let count = usize::try_from(asset.reference_count).map_err(|_| {
                    CatalogError::InvalidAssetReferenceCount {
                        id,
                        count: asset.reference_count,
                    }
                })?;
                if count == 0 {
                    return Err(CatalogError::InvalidAssetReferenceCount {
                        id,
                        count: asset.reference_count,
                    });
                }
                total
                    .checked_add(count)
                    .ok_or(CatalogError::AssetReferenceTotalOverflow)
            })?;
        Ok(CatalogCounts {
            files: state.files.len(),
            collections: state.collections.len(),
            asset_entries: state.assets.len(),
            asset_references,
            collection_spaces: state
                .files
                .values()
                .filter(|file| file.collection_space.is_some())
                .count(),
            stream_xml_files: state
                .files
                .values()
                .filter(|file| file.stream_xml.is_some())
                .count(),
        })
    }
}

impl StreamRegistrationSink for WorldStreamCatalog {
    type Error = CatalogError;

    fn collection_map_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        map: &CollectionMap,
    ) -> Result<(), Self::Error> {
        let mut state = self.lock()?;
        if let Some(file) = state.files.get(context.id()) {
            ensure_load_open(file, context, CatalogStage::CollectionMap)?;
            if file.collection_map.is_some() {
                return Err(CatalogError::StageAlreadyRegistered(
                    CatalogStage::CollectionMap,
                ));
            }
        }

        let mut ids = HashSet::with_capacity(map.collections.len());
        let mut prepared = Vec::with_capacity(map.collections.len());
        for (source_index, entry) in map.collections.iter().enumerate() {
            if !ids.insert(entry.id) || state.collections.contains_key(&entry.id) {
                return Err(CatalogError::DuplicateCollectionId(entry.id));
            }
            if entry.filename.as_bytes().contains(&0) {
                return Err(CatalogError::CollectionPathContainsNul { id: entry.id });
            }
            let path = format!(
                "{}{}.xsf",
                context.paths().collection_directory,
                entry.filename
            );
            if path.len() > MAX_COLLECTION_PATH_BYTES {
                return Err(CatalogError::CollectionPathTooLong {
                    id: entry.id,
                    byte_len: path.len(),
                });
            }
            prepared.push(CollectionRegistration {
                owner: context.id().clone(),
                source_index,
                path: Arc::from(path),
                entry: entry.clone(),
                stream_format: map.stream_format,
            });
        }
        let attempted = state
            .collections
            .len()
            .checked_add(prepared.len())
            .ok_or(CatalogError::CollectionCountOverflow)?;
        if attempted > COLLECTION_CAPACITY {
            return Err(CatalogError::CollectionCapacityExceeded {
                attempted,
                capacity: COLLECTION_CAPACITY,
            });
        }

        let source_ids: Arc<[u64]> = map.collections.iter().map(|entry| entry.id).collect();
        for registration in prepared {
            state
                .collections
                .insert(registration.entry.id, registration);
        }
        let file = state
            .files
            .entry(context.id().clone())
            .or_insert_with(|| FileRegistration::new(context));
        file.collection_map = Some(Arc::new(map.clone()));
        file.collection_ids = source_ids;
        file.collections_active = true;
        Ok(())
    }

    fn asset_toc_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        toc: &AssetToc,
    ) -> Result<(), Self::Error> {
        let mut state = self.lock()?;
        let file = state
            .files
            .get(context.id())
            .ok_or(CatalogError::UnknownFile)?;
        ensure_load_open(file, context, CatalogStage::AssetToc)?;
        if file.collection_map.is_none() {
            return Err(CatalogError::StageNotRegistered(
                CatalogStage::CollectionMap,
            ));
        }
        if file.asset_toc.is_some() {
            return Err(CatalogError::StageAlreadyRegistered(CatalogStage::AssetToc));
        }

        let mut increments = HashMap::<u64, i32>::new();
        for entry in &toc.assets {
            let count = increments.entry(entry.id).or_default();
            *count = count
                .checked_add(1)
                .ok_or(CatalogError::AssetReferenceOverflow(entry.id))?;
        }
        for (&id, &increment) in &increments {
            let Some(asset) = state.assets.get(&id) else {
                continue;
            };
            if asset.reference_count <= 0 {
                return Err(CatalogError::InvalidAssetReferenceCount {
                    id,
                    count: asset.reference_count,
                });
            }
            asset
                .reference_count
                .checked_add(increment)
                .ok_or(CatalogError::AssetReferenceOverflow(id))?;
        }

        for (&id, &increment) in &increments {
            if let Some(asset) = state.assets.get_mut(&id) {
                asset.reference_count = asset
                    .reference_count
                    .checked_add(increment)
                    .expect("signed asset-reference increment was preflighted");
            } else {
                let first_entry = toc
                    .assets
                    .iter()
                    .find(|entry| entry.id == id)
                    .expect("increment ID came from this TOC")
                    .clone();
                state.assets.insert(
                    id,
                    AssetRegistration {
                        first_owner: context.id().clone(),
                        entry: first_entry,
                        reference_count: increment,
                    },
                );
            }
        }
        let source_ids: Arc<[u64]> = toc.assets.iter().map(|entry| entry.id).collect();
        let file = state
            .files
            .get_mut(context.id())
            .expect("file validated above");
        file.asset_toc = Some(Arc::new(toc.clone()));
        file.asset_ids = source_ids;
        file.assets_active = true;
        Ok(())
    }

    fn collection_space_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        space: &CollectionSpace,
    ) -> Result<(), Self::Error> {
        let mut state = self.lock()?;
        let file = checked_file_mut(&mut state, context)?;
        ensure_phase_open(file, CatalogStage::CollectionSpace)?;
        if file.asset_toc.is_none() {
            return Err(CatalogError::StageNotRegistered(CatalogStage::AssetToc));
        }
        if file.collection_space.is_some() {
            return Err(CatalogError::StageAlreadyRegistered(
                CatalogStage::CollectionSpace,
            ));
        }
        file.collection_space = Some(Arc::new(space.clone()));
        Ok(())
    }

    fn stream_xml_loaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        bytes: &[u8],
    ) -> Result<(), Self::Error> {
        let mut state = self.lock()?;
        let file = checked_file_mut(&mut state, context)?;
        ensure_phase_open(file, CatalogStage::StreamXml)?;
        if file.collection_space.is_none() {
            return Err(CatalogError::StageNotRegistered(
                CatalogStage::CollectionSpace,
            ));
        }
        if file.stream_xml.is_some() {
            return Err(CatalogError::StageAlreadyRegistered(
                CatalogStage::StreamXml,
            ));
        }
        file.stream_xml = Some(Arc::from(bytes));
        Ok(())
    }

    fn collection_map_unloaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        map: &CollectionMap,
    ) -> Result<(), Self::Error> {
        let mut state = self.lock()?;
        let file = state
            .files
            .get(context.id())
            .ok_or(CatalogError::UnknownFile)?;
        ensure_context(file, context)?;
        if file.phase != FileRegistrationPhase::Registered {
            return Err(CatalogError::InvalidUnloadOrder {
                stage: CatalogStage::CollectionMap,
                phase: file.phase,
            });
        }
        let retained = file
            .collection_map
            .as_deref()
            .ok_or(CatalogError::StageNotRegistered(
                CatalogStage::CollectionMap,
            ))?;
        if retained != map {
            return Err(CatalogError::CollectionMapMismatch);
        }
        if !file.collections_active {
            return Err(CatalogError::StageNotRegistered(
                CatalogStage::CollectionMap,
            ));
        }
        let ids = Arc::clone(&file.collection_ids);
        for &id in ids.iter() {
            let registration = state
                .collections
                .get(&id)
                .ok_or(CatalogError::CollectionOwnershipMismatch(id))?;
            if registration.owner != *context.id() {
                return Err(CatalogError::CollectionOwnershipMismatch(id));
            }
        }
        for &id in ids.iter() {
            state.collections.remove(&id);
        }
        let file = state
            .files
            .get_mut(context.id())
            .expect("file validated above");
        file.collections_active = false;
        file.phase = FileRegistrationPhase::CollectionsUnloaded;
        Ok(())
    }

    fn asset_toc_unloaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        toc: &AssetToc,
    ) -> Result<(), Self::Error> {
        let mut state = self.lock()?;
        let file = state
            .files
            .get(context.id())
            .ok_or(CatalogError::UnknownFile)?;
        ensure_context(file, context)?;
        if file.phase != FileRegistrationPhase::CollectionsUnloaded {
            return Err(CatalogError::InvalidUnloadOrder {
                stage: CatalogStage::AssetToc,
                phase: file.phase,
            });
        }
        let retained = file
            .asset_toc
            .as_deref()
            .ok_or(CatalogError::StageNotRegistered(CatalogStage::AssetToc))?;
        if retained != toc {
            return Err(CatalogError::AssetTocMismatch);
        }
        if !file.assets_active {
            return Err(CatalogError::StageNotRegistered(CatalogStage::AssetToc));
        }
        let ids = Arc::clone(&file.asset_ids);
        let mut decrements = HashMap::<u64, i32>::new();
        for &id in ids.iter() {
            let decrement = decrements.entry(id).or_default();
            *decrement = decrement
                .checked_add(1)
                .ok_or(CatalogError::AssetReferenceMismatch(id))?;
        }
        for (&id, &decrement) in &decrements {
            let Some(asset) = state.assets.get(&id) else {
                return Err(CatalogError::AssetReferenceMismatch(id));
            };
            if asset.reference_count <= 0 {
                return Err(CatalogError::InvalidAssetReferenceCount {
                    id,
                    count: asset.reference_count,
                });
            }
            if asset.reference_count < decrement {
                return Err(CatalogError::AssetReferenceMismatch(id));
            }
        }
        for (&id, &decrement) in &decrements {
            let remove = {
                let asset = state.assets.get_mut(&id).expect("asset validated above");
                asset.reference_count = asset
                    .reference_count
                    .checked_sub(decrement)
                    .expect("signed asset-reference decrement was preflighted");
                asset.reference_count == 0
            };
            if remove {
                state.assets.remove(&id);
            }
        }
        let file = state
            .files
            .get_mut(context.id())
            .expect("file validated above");
        file.assets_active = false;
        file.phase = FileRegistrationPhase::AssetsUnloaded;
        Ok(())
    }

    fn stream_file_unloaded(
        &mut self,
        context: StreamRegistrationContext<'_>,
        space: Option<&CollectionSpace>,
        had_stream_xml: bool,
    ) -> Result<(), Self::Error> {
        let mut state = self.lock()?;
        let Some(file) = state.files.get(context.id()) else {
            // This is the valid early-failure unload path. A repeated invalid
            // call has the same observable arguments, so no stronger parity
            // claim is made for that case.
            return if space.is_none() && !had_stream_xml {
                Ok(())
            } else {
                Err(CatalogError::UnknownFile)
            };
        };
        ensure_context(file, context)?;
        if file.collections_active || file.assets_active {
            return Err(CatalogError::ActiveRegistrationsRemain);
        }
        if !optional_space_bits_equal(file.collection_space.as_deref(), space) {
            return Err(CatalogError::CollectionSpaceMismatch);
        }
        if file.stream_xml.is_some() != had_stream_xml {
            return Err(CatalogError::StreamXmlOwnershipMismatch);
        }
        state.files.remove(context.id());
        Ok(())
    }
}

fn ensure_context(
    file: &FileRegistration,
    context: StreamRegistrationContext<'_>,
) -> Result<(), CatalogError> {
    if file.context_matches(context) {
        Ok(())
    } else {
        Err(CatalogError::ContextMismatch)
    }
}

fn ensure_load_open(
    file: &FileRegistration,
    context: StreamRegistrationContext<'_>,
    stage: CatalogStage,
) -> Result<(), CatalogError> {
    ensure_context(file, context)?;
    ensure_phase_open(file, stage)
}

fn ensure_phase_open(file: &FileRegistration, stage: CatalogStage) -> Result<(), CatalogError> {
    if file.phase == FileRegistrationPhase::Registered {
        Ok(())
    } else {
        Err(CatalogError::LoadAfterUnloadStarted {
            stage,
            phase: file.phase,
        })
    }
}

fn checked_file_mut<'a>(
    state: &'a mut CatalogState,
    context: StreamRegistrationContext<'_>,
) -> Result<&'a mut FileRegistration, CatalogError> {
    let file = state
        .files
        .get_mut(context.id())
        .ok_or(CatalogError::UnknownFile)?;
    ensure_context(file, context)?;
    Ok(file)
}

fn optional_space_bits_equal(
    retained: Option<&CollectionSpace>,
    supplied: Option<&CollectionSpace>,
) -> bool {
    match (retained, supplied) {
        (None, None) => true,
        (Some(left), Some(right)) => space_bits_equal(left, right),
        (None, Some(_)) | (Some(_), None) => false,
    }
}

fn space_bits_equal(left: &CollectionSpace, right: &CollectionSpace) -> bool {
    left.version == right.version
        && left.stamp == right.stamp
        && left.collections.len() == right.collections.len()
        && left
            .collections
            .iter()
            .zip(&right.collections)
            .all(|(left, right)| {
                left.id == right.id
                    && left.lod == right.lod
                    && float_lanes_equal(left.hull_min, right.hull_min)
                    && float_lanes_equal(left.hull_max, right.hull_max)
                    && float_lanes_equal(left.geometry_min, right.geometry_min)
                    && float_lanes_equal(left.geometry_max, right.geometry_max)
            })
}

fn float_lanes_equal(left: [f32; 4], right: [f32; 4]) -> bool {
    left.into_iter()
        .zip(right)
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_stream::StreamFile;
    use skate2_formats::stream::{BaseResourceDescriptor, CollectionSpaceEntry};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    type TestFile = StreamFile<(), CatalogError>;

    fn file(kind: StreamKind) -> TestFile {
        StreamFile::load("world", "boot", kind).expect("test paths fit")
    }

    fn collection(id: u64, filename: &str) -> CollectionMapEntry {
        let raw_id = u32::try_from(id).expect("test collection IDs fit u32");
        CollectionMapEntry {
            id,
            parent_id: id.wrapping_add(0x100),
            payload_offset: 0x8000_0000 | raw_id,
            payload_size: 0xf000_0000 | raw_id,
            asset_count: raw_id,
            filename: filename.to_owned(),
            name_offset: 0,
            alignment_pad: -1,
        }
    }

    fn map(entries: Vec<CollectionMapEntry>) -> CollectionMap {
        CollectionMap {
            version: 3,
            stamp: 0x1020_3040_5060_7080,
            stream_format: 9,
            name_table_offset: 0,
            collections: entries,
        }
    }

    fn asset(id: u64, asset_type: u32) -> AssetMapEntry {
        AssetMapEntry {
            id,
            asset_type,
            serialized_size: 0x1111_0000 | asset_type,
            resources: std::array::from_fn(|index| {
                let index = u32::try_from(index).expect("five resource indices fit u32");
                BaseResourceDescriptor {
                    size: asset_type.wrapping_add(index),
                    alignment: 0x10_u32 << index,
                }
            }),
            name_offset: asset_type,
            unused: !asset_type,
        }
    }

    fn toc(entries: Vec<AssetMapEntry>) -> AssetToc {
        AssetToc {
            version: 3,
            stamp: 0x8877_6655_4433_2211,
            name_table_offset: 0,
            assets: entries,
        }
    }

    fn space(stamp: u64) -> CollectionSpace {
        CollectionSpace {
            version: 3,
            stamp,
            collections: vec![CollectionSpaceEntry {
                id: 7,
                lod: 0xfe,
                hull_min: [
                    f32::from_bits(0x7fc0_1234),
                    f32::from_bits(0x8000_0000),
                    1.0,
                    f32::from_bits(1),
                ],
                hull_max: [2.0, 3.0, 4.0, 5.0],
                geometry_min: [6.0, 7.0, 8.0, 9.0],
                geometry_max: [10.0, 11.0, 12.0, f32::from_bits(0xffc0_5678)],
            }],
        }
    }

    fn register_map(catalog: &mut WorldStreamCatalog, file: &TestFile, map: &CollectionMap) {
        catalog
            .collection_map_loaded(file.registration_context(), map)
            .unwrap();
    }

    fn register_toc(catalog: &mut WorldStreamCatalog, file: &TestFile, toc: &AssetToc) {
        catalog
            .asset_toc_loaded(file.registration_context(), toc)
            .unwrap();
    }

    #[test]
    fn xsm_path_offset_size_and_equal_filenames_are_exact() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Presentation);
        let map = map(vec![collection(1, "same"), collection(2, "same")]);
        register_map(&mut catalog, &file, &map);

        let one = catalog.collection(1).unwrap().unwrap();
        let two = catalog.collection(2).unwrap().unwrap();
        assert_eq!(one.id, 1);
        assert_eq!(two.id, 2);
        assert_eq!(&*one.path, "world\\boot\\same.xsf");
        assert_eq!(one.path, two.path);
        assert_eq!(one.source_index, 0);
        assert_eq!(two.source_index, 1);
        assert_eq!(catalog.stream_offset(1).unwrap(), Some(0x8000_0001));
        assert_eq!(catalog.stream_size(1).unwrap(), Some(0xf000_0001));
    }

    #[test]
    fn xsm_path_accepts_95_bytes_and_rejects_96_transactionally() {
        let mut catalog = WorldStreamCatalog::new();
        let accepted_base = "w".repeat(21);
        let accepted = StreamFile::load(&accepted_base, "boot", StreamKind::Presentation)
            .expect("test paths fit");
        let filename = "a".repeat(64);
        register_map(
            &mut catalog,
            &accepted,
            &map(vec![collection(1, &filename)]),
        );
        assert_eq!(catalog.collection(1).unwrap().unwrap().path.len(), 95);

        let rejected_base = "w".repeat(22);
        let rejected: TestFile = StreamFile::load(&rejected_base, "boot", StreamKind::Simulation)
            .expect("test paths fit");
        let error = catalog
            .collection_map_loaded(
                rejected.registration_context(),
                &map(vec![collection(2, &filename)]),
            )
            .unwrap_err();
        assert_eq!(
            error,
            CatalogError::CollectionPathTooLong {
                id: 2,
                byte_len: MAX_COLLECTION_PATH_BYTES + 1,
            }
        );
        assert!(catalog.file(rejected.registration_id()).unwrap().is_none());
        assert_eq!(catalog.counts().unwrap().collections, 1);
    }

    #[test]
    fn duplicate_collection_batch_is_atomic() {
        let mut catalog = WorldStreamCatalog::new();
        let owner = file(StreamKind::Presentation);
        let duplicate = map(vec![collection(1, "a"), collection(1, "b")]);
        assert_eq!(
            catalog.collection_map_loaded(owner.registration_context(), &duplicate),
            Err(CatalogError::DuplicateCollectionId(1))
        );
        assert_eq!(
            catalog.counts().unwrap(),
            CatalogCounts {
                files: 0,
                collections: 0,
                asset_entries: 0,
                asset_references: 0,
                collection_spaces: 0,
                stream_xml_files: 0,
            }
        );
    }

    #[test]
    fn collection_conflict_with_another_owner_rejects_whole_file() {
        let mut catalog = WorldStreamCatalog::new();
        let first = file(StreamKind::Presentation);
        let second = file(StreamKind::Simulation);
        register_map(&mut catalog, &first, &map(vec![collection(7, "first")]));

        let error = catalog
            .collection_map_loaded(
                second.registration_context(),
                &map(vec![
                    collection(8, "would_insert"),
                    collection(7, "conflict"),
                ]),
            )
            .unwrap_err();
        assert_eq!(error, CatalogError::DuplicateCollectionId(7));
        assert!(catalog.collection(8).unwrap().is_none());
        assert!(catalog.file(second.registration_id()).unwrap().is_none());
        assert_eq!(catalog.counts().unwrap().collections, 1);
    }

    #[test]
    fn collection_capacity_is_fixed_and_overflow_is_atomic() {
        let mut catalog = WorldStreamCatalog::new();
        let first = file(StreamKind::Presentation);
        let entries = (0..COLLECTION_CAPACITY as u64)
            .map(|id| collection(id, "c"))
            .collect();
        register_map(&mut catalog, &first, &map(entries));
        let extra = file(StreamKind::Simulation);
        assert_eq!(
            catalog.collection_map_loaded(
                extra.registration_context(),
                &map(vec![collection(9_000, "x")])
            ),
            Err(CatalogError::CollectionCapacityExceeded {
                attempted: COLLECTION_CAPACITY + 1,
                capacity: COLLECTION_CAPACITY,
            })
        );
        assert_eq!(catalog.counts().unwrap().collections, COLLECTION_CAPACITY);
        assert!(catalog.file(extra.registration_id()).unwrap().is_none());
    }

    #[test]
    fn asset_occurrences_refcount_and_preserve_first_metadata() {
        let mut catalog = WorldStreamCatalog::new();
        let first = file(StreamKind::Presentation);
        let second = file(StreamKind::Presentation);
        let empty_map = map(vec![]);
        register_map(&mut catalog, &first, &empty_map);
        register_map(&mut catalog, &second, &empty_map);
        let first_toc = toc(vec![asset(7, 1), asset(7, 2)]);
        let second_toc = toc(vec![asset(7, 99)]);
        register_toc(&mut catalog, &first, &first_toc);
        register_toc(&mut catalog, &second, &second_toc);

        let snapshot = catalog.asset(7).unwrap().unwrap();
        assert_eq!(snapshot.entry.asset_type, 1);
        assert_eq!(snapshot.entry.resources, first_toc.assets[0].resources);
        assert_eq!(snapshot.reference_count, 3);
        assert_eq!(snapshot.first_owner, *first.registration_id());
        assert_eq!(
            &*catalog
                .file(first.registration_id())
                .unwrap()
                .unwrap()
                .asset_ids,
            &[7, 7]
        );

        catalog
            .collection_map_unloaded(first.registration_context(), &empty_map)
            .unwrap();
        catalog
            .asset_toc_unloaded(first.registration_context(), &first_toc)
            .unwrap();
        catalog
            .stream_file_unloaded(first.registration_context(), None, false)
            .unwrap();
        assert!(catalog.file(first.registration_id()).unwrap().is_none());
        let after_first = catalog.asset(7).unwrap().unwrap();
        assert_eq!(after_first.entry, first_toc.assets[0]);
        assert_eq!(after_first.first_owner, *first.registration_id());
        assert_eq!(after_first.reference_count, 1);
        catalog
            .collection_map_unloaded(second.registration_context(), &empty_map)
            .unwrap();
        catalog
            .asset_toc_unloaded(second.registration_context(), &second_toc)
            .unwrap();
        assert!(catalog.asset(7).unwrap().is_none());
        assert_eq!(snapshot.entry.asset_type, 1);
    }

    #[test]
    fn asset_overflow_preflight_is_transactional() {
        let mut catalog = WorldStreamCatalog::new();
        let first = file(StreamKind::Presentation);
        let second = file(StreamKind::Simulation);
        register_map(&mut catalog, &first, &map(vec![]));
        register_map(&mut catalog, &second, &map(vec![]));
        register_toc(&mut catalog, &first, &toc(vec![asset(7, 1)]));
        catalog
            .state
            .lock()
            .unwrap()
            .assets
            .get_mut(&7)
            .unwrap()
            .reference_count = i32::MAX;

        let incoming = toc(vec![asset(8, 8), asset(7, 99)]);
        assert_eq!(
            catalog.asset_toc_loaded(second.registration_context(), &incoming),
            Err(CatalogError::AssetReferenceOverflow(7))
        );
        assert!(catalog.asset(8).unwrap().is_none());
        assert!(
            catalog
                .file(second.registration_id())
                .unwrap()
                .unwrap()
                .asset_toc
                .is_none()
        );
        assert_eq!(catalog.asset(7).unwrap().unwrap().reference_count, i32::MAX);
    }

    #[test]
    fn signed_asset_unload_boundary_rejects_nonpositive_and_decrements_positive_max() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Presentation);
        let map = map(vec![]);
        let registered_toc = toc(vec![asset(7, 1)]);
        register_map(&mut catalog, &file, &map);
        register_toc(&mut catalog, &file, &registered_toc);
        catalog
            .collection_map_unloaded(file.registration_context(), &map)
            .unwrap();
        let before = catalog.file(file.registration_id()).unwrap().unwrap();
        for invalid_count in [0, -1, i32::MIN] {
            catalog
                .state
                .lock()
                .unwrap()
                .assets
                .get_mut(&7)
                .unwrap()
                .reference_count = invalid_count;
            assert_eq!(
                catalog.asset_toc_unloaded(file.registration_context(), &registered_toc),
                Err(CatalogError::InvalidAssetReferenceCount {
                    id: 7,
                    count: invalid_count,
                })
            );
            assert_eq!(
                catalog.file(file.registration_id()).unwrap().unwrap(),
                before
            );
            let state = catalog.state.lock().unwrap();
            assert_eq!(state.files.len(), 1);
            assert_eq!(state.collections.len(), 0);
            assert_eq!(state.assets.len(), 1);
            assert_eq!(state.assets.get(&7).unwrap().reference_count, invalid_count);
        }

        catalog
            .state
            .lock()
            .unwrap()
            .assets
            .get_mut(&7)
            .unwrap()
            .reference_count = i32::MAX;
        catalog
            .asset_toc_unloaded(file.registration_context(), &registered_toc)
            .unwrap();
        assert_eq!(
            catalog.asset(7).unwrap().unwrap().reference_count,
            i32::MAX - 1
        );
    }

    #[test]
    fn duplicate_asset_occurrences_unload_correctly_in_both_owner_orders() {
        for unload_second_first in [false, true] {
            let mut catalog = WorldStreamCatalog::new();
            let first = file(StreamKind::Presentation);
            let second = file(StreamKind::Simulation);
            let empty_map = map(vec![]);
            let first_toc = toc(vec![asset(9, 1), asset(9, 2)]);
            let second_toc = toc(vec![asset(9, 99), asset(9, 100), asset(9, 101)]);
            for owner in [&first, &second] {
                register_map(&mut catalog, owner, &empty_map);
            }
            register_toc(&mut catalog, &first, &first_toc);
            register_toc(&mut catalog, &second, &second_toc);
            assert_eq!(catalog.asset(9).unwrap().unwrap().reference_count, 5);

            let order = if unload_second_first {
                [(&second, &second_toc, 2), (&first, &first_toc, 0)]
            } else {
                [(&first, &first_toc, 3), (&second, &second_toc, 0)]
            };
            for (owner, owner_toc, remaining) in order {
                catalog
                    .collection_map_unloaded(owner.registration_context(), &empty_map)
                    .unwrap();
                catalog
                    .asset_toc_unloaded(owner.registration_context(), owner_toc)
                    .unwrap();
                assert_eq!(
                    catalog
                        .asset(9)
                        .unwrap()
                        .map_or(0, |snapshot| snapshot.reference_count),
                    remaining
                );
            }
        }
    }

    #[test]
    fn unload_phase_rejects_late_loads_and_asset_first_order_without_mutation() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Presentation);
        let registered_map = map(vec![collection(1, "a")]);
        let registered_toc = toc(vec![asset(7, 1)]);
        register_map(&mut catalog, &file, &registered_map);
        register_toc(&mut catalog, &file, &registered_toc);

        let before_asset_first = catalog.file(file.registration_id()).unwrap().unwrap();
        assert_eq!(
            catalog.asset_toc_unloaded(file.registration_context(), &registered_toc),
            Err(CatalogError::InvalidUnloadOrder {
                stage: CatalogStage::AssetToc,
                phase: FileRegistrationPhase::Registered,
            })
        );
        assert_eq!(
            catalog.file(file.registration_id()).unwrap().unwrap(),
            before_asset_first
        );

        catalog
            .collection_map_unloaded(file.registration_context(), &registered_map)
            .unwrap();
        let after_map_unload = catalog.file(file.registration_id()).unwrap().unwrap();
        let after_map_counts = catalog.counts().unwrap();
        assert_eq!(
            after_map_unload.phase,
            FileRegistrationPhase::CollectionsUnloaded
        );
        assert_eq!(
            catalog.collection_map_loaded(
                file.registration_context(),
                &map(vec![collection(2, "late-nonempty")]),
            ),
            Err(CatalogError::LoadAfterUnloadStarted {
                stage: CatalogStage::CollectionMap,
                phase: FileRegistrationPhase::CollectionsUnloaded,
            })
        );
        assert_eq!(
            catalog.asset_toc_loaded(file.registration_context(), &toc(vec![asset(8, 8)])),
            Err(CatalogError::LoadAfterUnloadStarted {
                stage: CatalogStage::AssetToc,
                phase: FileRegistrationPhase::CollectionsUnloaded,
            })
        );
        assert_eq!(
            catalog.collection_space_loaded(file.registration_context(), &space(5)),
            Err(CatalogError::LoadAfterUnloadStarted {
                stage: CatalogStage::CollectionSpace,
                phase: FileRegistrationPhase::CollectionsUnloaded,
            })
        );
        assert_eq!(
            catalog.stream_xml_loaded(file.registration_context(), b"late-after-xsm"),
            Err(CatalogError::LoadAfterUnloadStarted {
                stage: CatalogStage::StreamXml,
                phase: FileRegistrationPhase::CollectionsUnloaded,
            })
        );
        assert_eq!(
            catalog.file(file.registration_id()).unwrap().unwrap(),
            after_map_unload
        );
        assert_eq!(catalog.counts().unwrap(), after_map_counts);
        assert!(catalog.collection(2).unwrap().is_none());
        assert_eq!(catalog.asset(7).unwrap().unwrap().reference_count, 1);
        assert!(catalog.asset(8).unwrap().is_none());
    }

    #[test]
    fn assets_unloaded_phase_rejects_every_late_load_without_mutation() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Presentation);
        let registered_map = map(vec![collection(1, "a")]);
        let registered_toc = toc(vec![asset(7, 1)]);
        register_map(&mut catalog, &file, &registered_map);
        register_toc(&mut catalog, &file, &registered_toc);
        catalog
            .collection_map_unloaded(file.registration_context(), &registered_map)
            .unwrap();
        catalog
            .asset_toc_unloaded(file.registration_context(), &registered_toc)
            .unwrap();
        let after_toc_unload = catalog.file(file.registration_id()).unwrap().unwrap();
        let after_toc_counts = catalog.counts().unwrap();
        assert_eq!(
            after_toc_unload.phase,
            FileRegistrationPhase::AssetsUnloaded
        );
        assert_eq!(
            catalog.collection_map_loaded(
                file.registration_context(),
                &map(vec![collection(3, "late-after-xst")]),
            ),
            Err(CatalogError::LoadAfterUnloadStarted {
                stage: CatalogStage::CollectionMap,
                phase: FileRegistrationPhase::AssetsUnloaded,
            })
        );
        assert_eq!(
            catalog.asset_toc_loaded(file.registration_context(), &toc(vec![asset(8, 8)])),
            Err(CatalogError::LoadAfterUnloadStarted {
                stage: CatalogStage::AssetToc,
                phase: FileRegistrationPhase::AssetsUnloaded,
            })
        );
        assert_eq!(
            catalog.collection_space_loaded(file.registration_context(), &space(5)),
            Err(CatalogError::LoadAfterUnloadStarted {
                stage: CatalogStage::CollectionSpace,
                phase: FileRegistrationPhase::AssetsUnloaded,
            })
        );
        assert_eq!(
            catalog.stream_xml_loaded(file.registration_context(), b"late"),
            Err(CatalogError::LoadAfterUnloadStarted {
                stage: CatalogStage::StreamXml,
                phase: FileRegistrationPhase::AssetsUnloaded,
            })
        );
        assert_eq!(
            catalog.file(file.registration_id()).unwrap().unwrap(),
            after_toc_unload
        );
        assert_eq!(catalog.counts().unwrap(), after_toc_counts);
        assert!(catalog.collection(3).unwrap().is_none());
        assert!(catalog.asset(8).unwrap().is_none());
    }

    #[test]
    fn asset_toc_before_collection_map_is_unknown_and_creates_nothing() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Simulation);
        assert_eq!(
            catalog.asset_toc_loaded(file.registration_context(), &toc(vec![asset(7, 1)])),
            Err(CatalogError::UnknownFile)
        );
        assert!(catalog.file(file.registration_id()).unwrap().is_none());
        assert!(catalog.asset(7).unwrap().is_none());
        assert_eq!(
            catalog.counts().unwrap(),
            CatalogCounts {
                files: 0,
                collections: 0,
                asset_entries: 0,
                asset_references: 0,
                collection_spaces: 0,
                stream_xml_files: 0,
            }
        );
    }

    #[test]
    fn wrong_asset_toc_unload_is_atomic_after_collection_unload() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Simulation);
        let map = map(vec![]);
        let registered = toc(vec![asset(7, 1)]);
        let wrong = toc(vec![asset(8, 2)]);
        register_map(&mut catalog, &file, &map);
        register_toc(&mut catalog, &file, &registered);
        catalog
            .collection_map_unloaded(file.registration_context(), &map)
            .unwrap();
        let before = catalog.file(file.registration_id()).unwrap().unwrap();
        assert_eq!(
            catalog.asset_toc_unloaded(file.registration_context(), &wrong),
            Err(CatalogError::AssetTocMismatch)
        );
        assert_eq!(
            catalog.file(file.registration_id()).unwrap().unwrap(),
            before
        );
        assert_eq!(catalog.asset(7).unwrap().unwrap().reference_count, 1);
    }

    #[test]
    fn final_unload_rejects_active_registrations_without_mutation() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Presentation);
        register_map(&mut catalog, &file, &map(vec![collection(1, "a")]));
        let before = catalog.file(file.registration_id()).unwrap().unwrap();
        assert_eq!(
            catalog.stream_file_unloaded(file.registration_context(), None, false),
            Err(CatalogError::ActiveRegistrationsRemain)
        );
        assert_eq!(
            catalog.file(file.registration_id()).unwrap().unwrap(),
            before
        );
        assert_eq!(catalog.counts().unwrap().collections, 1);
    }

    #[test]
    fn cloned_sink_and_reader_share_one_catalog() {
        let mut sink = WorldStreamCatalog::new();
        let reader = sink.clone();
        let file =
            StreamFile::load("world\\", "boot", StreamKind::Presentation).expect("test paths fit");
        register_map(&mut sink, &file, &map(vec![collection(1, "mixed/path")]));
        assert_eq!(reader.counts().unwrap().collections, 1);
        assert_eq!(
            &*reader.collection(1).unwrap().unwrap().path,
            "world\\\\boot\\mixed/path.xsf"
        );
    }

    #[test]
    fn identical_paths_still_have_distinct_file_owners() {
        let mut catalog = WorldStreamCatalog::new();
        let first = file(StreamKind::Presentation);
        let second = file(StreamKind::Presentation);
        assert_ne!(first.registration_id(), second.registration_id());
        register_map(&mut catalog, &first, &map(vec![collection(1, "a")]));
        assert_eq!(
            catalog.collection_map_loaded(
                second.registration_context(),
                &map(vec![collection(1, "a")])
            ),
            Err(CatalogError::DuplicateCollectionId(1))
        );
        assert!(catalog.file(first.registration_id()).unwrap().is_some());
        assert!(catalog.file(second.registration_id()).unwrap().is_none());
    }

    #[test]
    fn xss_clone_preserves_all_bits_and_does_not_match_stamps() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Simulation);
        let map = map(vec![]);
        let toc = toc(vec![]);
        let space = space(0xdead_beef);
        register_map(&mut catalog, &file, &map);
        register_toc(&mut catalog, &file, &toc);
        catalog
            .collection_space_loaded(file.registration_context(), &space)
            .unwrap();

        let retained = catalog
            .file(file.registration_id())
            .unwrap()
            .unwrap()
            .collection_space
            .unwrap();
        let entry = &retained.collections[0];
        assert_eq!(retained.stamp, 0xdead_beef);
        assert_eq!(entry.lod, 0xfe);
        assert_eq!(entry.hull_min[0].to_bits(), 0x7fc0_1234);
        assert_eq!(entry.hull_min[1].to_bits(), 0x8000_0000);
        assert_eq!(entry.geometry_max[3].to_bits(), 0xffc0_5678);
    }

    #[test]
    fn final_unload_distinguishes_absent_xss_from_present_empty_xss() {
        let empty_space = CollectionSpace {
            version: 3,
            stamp: 77,
            collections: vec![],
        };

        let mut absent_catalog = WorldStreamCatalog::new();
        let absent_file = file(StreamKind::Texture);
        let absent_map = map(vec![]);
        register_map(&mut absent_catalog, &absent_file, &absent_map);
        absent_catalog
            .collection_map_unloaded(absent_file.registration_context(), &absent_map)
            .unwrap();
        assert_eq!(
            absent_catalog.stream_file_unloaded(
                absent_file.registration_context(),
                Some(&empty_space),
                false,
            ),
            Err(CatalogError::CollectionSpaceMismatch)
        );
        assert!(
            absent_catalog
                .file(absent_file.registration_id())
                .unwrap()
                .is_some()
        );
        absent_catalog
            .stream_file_unloaded(absent_file.registration_context(), None, false)
            .unwrap();

        let mut present_catalog = WorldStreamCatalog::new();
        let present_file = file(StreamKind::Texture);
        let present_map = map(vec![]);
        let present_toc = toc(vec![]);
        register_map(&mut present_catalog, &present_file, &present_map);
        register_toc(&mut present_catalog, &present_file, &present_toc);
        present_catalog
            .collection_space_loaded(present_file.registration_context(), &empty_space)
            .unwrap();
        present_catalog
            .collection_map_unloaded(present_file.registration_context(), &present_map)
            .unwrap();
        present_catalog
            .asset_toc_unloaded(present_file.registration_context(), &present_toc)
            .unwrap();
        assert_eq!(
            present_catalog.stream_file_unloaded(present_file.registration_context(), None, false,),
            Err(CatalogError::CollectionSpaceMismatch)
        );
        assert!(
            present_catalog
                .file(present_file.registration_id())
                .unwrap()
                .is_some()
        );
        present_catalog
            .stream_file_unloaded(
                present_file.registration_context(),
                Some(&empty_space),
                false,
            )
            .unwrap();
    }

    #[test]
    fn final_unload_compares_nan_payloads_and_signed_zero_by_bits() {
        let mutations: [fn(&mut CollectionSpace); 2] = [
            |space: &mut CollectionSpace| {
                space.collections[0].hull_min[0] = f32::from_bits(0x7fc0_1235);
            },
            |space: &mut CollectionSpace| {
                space.collections[0].hull_min[1] = 0.0;
            },
        ];
        for mutate in mutations {
            let mut catalog = WorldStreamCatalog::new();
            let file = file(StreamKind::Simulation);
            let map = map(vec![]);
            let toc = toc(vec![]);
            let retained = space(1);
            register_map(&mut catalog, &file, &map);
            register_toc(&mut catalog, &file, &toc);
            catalog
                .collection_space_loaded(file.registration_context(), &retained)
                .unwrap();
            catalog
                .collection_map_unloaded(file.registration_context(), &map)
                .unwrap();
            catalog
                .asset_toc_unloaded(file.registration_context(), &toc)
                .unwrap();
            let mut bit_distinct = retained.clone();
            mutate(&mut bit_distinct);
            assert_eq!(
                catalog.stream_file_unloaded(
                    file.registration_context(),
                    Some(&bit_distinct),
                    false,
                ),
                Err(CatalogError::CollectionSpaceMismatch)
            );
            assert!(catalog.file(file.registration_id()).unwrap().is_some());
        }
    }

    #[test]
    fn final_unload_rejects_xml_ownership_mismatch_without_removing_file() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Presentation);
        let map = map(vec![]);
        let toc = toc(vec![]);
        let space = space(1);
        register_map(&mut catalog, &file, &map);
        register_toc(&mut catalog, &file, &toc);
        catalog
            .collection_space_loaded(file.registration_context(), &space)
            .unwrap();
        catalog
            .stream_xml_loaded(file.registration_context(), b"")
            .unwrap();
        catalog
            .collection_map_unloaded(file.registration_context(), &map)
            .unwrap();
        catalog
            .asset_toc_unloaded(file.registration_context(), &toc)
            .unwrap();
        assert_eq!(
            catalog.stream_file_unloaded(file.registration_context(), Some(&space), false),
            Err(CatalogError::StreamXmlOwnershipMismatch)
        );
        assert!(catalog.file(file.registration_id()).unwrap().is_some());
        catalog
            .stream_file_unloaded(file.registration_context(), Some(&space), true)
            .unwrap();
    }

    #[test]
    fn high_word_ids_are_preserved_and_missing_metadata_returns_none() {
        let collection_id = 0xfedc_ba98_7654_3210;
        let asset_id = 0x8123_4567_89ab_cdef;
        let mut high_collection = collection(1, "high");
        high_collection.id = collection_id;
        high_collection.parent_id = 0xf111_2222_3333_4444;
        let map = map(vec![high_collection]);
        let toc = toc(vec![asset(asset_id, 0x55)]);
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Presentation);
        register_map(&mut catalog, &file, &map);
        register_toc(&mut catalog, &file, &toc);

        assert_eq!(
            catalog
                .collection(collection_id)
                .unwrap()
                .unwrap()
                .parent_id,
            0xf111_2222_3333_4444
        );
        assert_eq!(catalog.asset(asset_id).unwrap().unwrap().entry.id, asset_id);
        let file_snapshot = catalog.file(file.registration_id()).unwrap().unwrap();
        assert_eq!(&*file_snapshot.collection_ids, &[collection_id]);
        assert_eq!(&*file_snapshot.asset_ids, &[asset_id]);
        assert_eq!(catalog.stream_offset(0xdead_beef_dead_beef).unwrap(), None);
        assert_eq!(catalog.stream_size(0xdead_beef_dead_beef).unwrap(), None);
    }

    #[test]
    fn successful_empty_xml_is_owned_as_some_empty() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Presentation);
        register_map(&mut catalog, &file, &map(vec![]));
        register_toc(&mut catalog, &file, &toc(vec![]));
        catalog
            .collection_space_loaded(file.registration_context(), &space(1))
            .unwrap();
        catalog
            .stream_xml_loaded(file.registration_context(), &[])
            .unwrap();
        let snapshot = catalog.file(file.registration_id()).unwrap().unwrap();
        assert_eq!(snapshot.stream_xml.as_deref(), Some([].as_slice()));
        assert_eq!(catalog.counts().unwrap().stream_xml_files, 1);
    }

    #[test]
    fn wrong_unload_table_leaves_state_unchanged() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Presentation);
        let registered = map(vec![collection(1, "a")]);
        let wrong = map(vec![collection(2, "b")]);
        register_map(&mut catalog, &file, &registered);
        assert_eq!(
            catalog.collection_map_unloaded(file.registration_context(), &wrong),
            Err(CatalogError::CollectionMapMismatch)
        );
        assert_eq!(catalog.counts().unwrap().collections, 1);
        assert!(
            catalog
                .file(file.registration_id())
                .unwrap()
                .unwrap()
                .collections_active
        );
    }

    #[test]
    fn full_reverse_unload_removes_every_owner_and_reference() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Presentation);
        let map = map(vec![collection(1, "a"), collection(2, "b")]);
        let toc = toc(vec![asset(5, 1), asset(5, 2), asset(6, 3)]);
        let space = space(0x9999);
        register_map(&mut catalog, &file, &map);
        register_toc(&mut catalog, &file, &toc);
        catalog
            .collection_space_loaded(file.registration_context(), &space)
            .unwrap();
        catalog
            .stream_xml_loaded(file.registration_context(), b"<stream/>")
            .unwrap();

        catalog
            .collection_map_unloaded(file.registration_context(), &map)
            .unwrap();
        catalog
            .asset_toc_unloaded(file.registration_context(), &toc)
            .unwrap();
        catalog
            .stream_file_unloaded(file.registration_context(), Some(&space), true)
            .unwrap();
        assert_eq!(
            catalog.counts().unwrap(),
            CatalogCounts {
                files: 0,
                collections: 0,
                asset_entries: 0,
                asset_references: 0,
                collection_spaces: 0,
                stream_xml_files: 0,
            }
        );
    }

    #[test]
    fn early_unload_without_any_registration_is_a_noop() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Texture);
        catalog
            .stream_file_unloaded(file.registration_context(), None, false)
            .unwrap();
        assert_eq!(catalog.counts().unwrap().files, 0);
    }

    #[test]
    fn stage_errors_do_not_partially_change_file() {
        let mut catalog = WorldStreamCatalog::new();
        let file = file(StreamKind::Presentation);
        register_map(&mut catalog, &file, &map(vec![]));
        assert_eq!(
            catalog.stream_xml_loaded(file.registration_context(), b"early"),
            Err(CatalogError::StageNotRegistered(
                CatalogStage::CollectionSpace
            ))
        );
        assert!(
            catalog
                .file(file.registration_id())
                .unwrap()
                .unwrap()
                .stream_xml
                .is_none()
        );
    }

    #[test]
    fn reader_reports_poison_instead_of_panicking_or_hanging() {
        let catalog = WorldStreamCatalog::new();
        let shared = Arc::clone(&catalog.state);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = shared.lock().unwrap();
            panic!("poison test lock");
        }));
        assert_eq!(catalog.counts(), Err(CatalogError::Poisoned));
    }
}
