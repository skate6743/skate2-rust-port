//! Concrete ownership for globally published world-stream files.
//!
//! The valid-domain ordering comes from
//! `cAssetStreamSystem::UpdateStreamFileLoaded` (`0x824e1ad0`) and
//! `cAssetStreamSystem::RemoveStreamFile` (`0x824dc098`). The console uses a
//! separate map Futex and file-vector RW lock. This store deliberately uses
//! one lock so publication, unpublication, and owning reader snapshots are
//! atomic on the PC. That is a safe, high-confidence deviation from the
//! console's transient cross-container visibility.

use crate::{
    game_stream::StreamKind,
    world_stream_adapter::{NativeCPath, compare_native_stream_file_paths},
};
use std::{
    collections::HashMap,
    convert::Infallible,
    error::Error,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex, MutexGuard, Weak},
};

const STREAM_KIND_COUNT: usize = 3;

const fn stream_kind_index(kind: StreamKind) -> usize {
    match kind {
        StreamKind::Presentation => 0,
        StreamKind::Simulation => 1,
        StreamKind::Texture => 2,
    }
}

/// Stable identity assigned to one successful publication.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PublicationId(u64);

impl PublicationId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug)]
struct PublishedEntry<FileIdentity, RecordStorage, LocalSpatialIdentity> {
    id: PublicationId,
    kind: StreamKind,
    path: NativeCPath,
    file: Arc<FileIdentity>,
    records: Arc<RecordStorage>,
    local_spatial: Mutex<LocalSpatialReleaseState<LocalSpatialIdentity>>,
}

#[derive(Debug)]
enum LocalSpatialReleaseState<LocalSpatialIdentity> {
    Retained(LocalSpatialIdentity),
    Releasing,
    Released,
}

/// Owning view of one published file. All identities remain valid after the
/// store lock is released.
#[derive(Debug)]
pub struct PublishedFileSnapshot<FileIdentity, RecordStorage> {
    pub publication_id: PublicationId,
    pub kind: StreamKind,
    pub path: NativeCPath,
    pub file: Arc<FileIdentity>,
    pub records: Arc<RecordStorage>,
}

impl<FileIdentity, RecordStorage> Clone for PublishedFileSnapshot<FileIdentity, RecordStorage> {
    fn clone(&self) -> Self {
        Self {
            publication_id: self.publication_id,
            kind: self.kind,
            path: self.path.clone(),
            file: Arc::clone(&self.file),
            records: Arc::clone(&self.records),
        }
    }
}

/// Owning counterpart of one native `cAssetID -> tOctreeInfo*` entry.
#[derive(Debug)]
pub struct PublishedRecordSnapshot<RecordStorage> {
    pub publication_id: PublicationId,
    pub compact_index: u32,
    pub records: Arc<RecordStorage>,
}

impl<RecordStorage> Clone for PublishedRecordSnapshot<RecordStorage> {
    fn clone(&self) -> Self {
        Self {
            publication_id: self.publication_id,
            compact_index: self.compact_index,
            records: Arc::clone(&self.records),
        }
    }
}

/// One-lock owning snapshot of both global containers.
#[derive(Debug)]
pub struct PublishedStreamStoreSnapshot<FileIdentity, RecordStorage> {
    pub records: HashMap<u64, PublishedRecordSnapshot<RecordStorage>>,
    pub files: [Vec<PublishedFileSnapshot<FileIdentity, RecordStorage>>; STREAM_KIND_COUNT],
}

/// All stable identities needed to publish one accepted compact record prefix.
pub struct StreamPublicationRequest<'a, FileIdentity, RecordStorage, LocalSpatialIdentity> {
    pub kind: StreamKind,
    pub path: NativeCPath,
    pub file: Arc<FileIdentity>,
    pub records: Arc<RecordStorage>,
    /// Store-owned local-spatial value. Ownership is transferred, not cloned.
    /// If this value is itself shared (for example, an `Arc`), release covers
    /// only the store's owned share and cannot claim destruction of aliases.
    pub local_spatial: LocalSpatialIdentity,
    pub compact_ids: &'a [u64],
}

/// Immutable plan passed to an optional pre-commit check.
#[derive(Debug)]
pub struct PublicationPlan<'a> {
    pub publication_id: PublicationId,
    pub kind: StreamKind,
    pub path: &'a NativeCPath,
    pub compact_record_count: usize,
    pub won_ids: &'a [u64],
    pub file_insert_index: usize,
    pub mapped_id_count_before: usize,
}

/// Store-owned failure before any logical publication mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublishStoreError {
    StoreLockPoisoned,
    DuplicateFileIdentity,
    CompactIndexOverflow {
        index: usize,
    },
    IdCapacityExceeded {
        mapped: usize,
        attempted_new: usize,
        maximum: usize,
    },
    PublicationIdExhausted,
    AllocationFailed,
}

impl fmt::Display for PublishStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for PublishStoreError {}

/// A publication rejected either by the store or by its caller's final
/// pre-commit check.
#[derive(Debug)]
pub enum PublishAttemptError<PrecommitError> {
    Store(PublishStoreError),
    Precommit(PrecommitError),
}

impl<PrecommitError: fmt::Display> fmt::Display for PublishAttemptError<PrecommitError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(source) => source.fmt(formatter),
            Self::Precommit(source) => {
                write!(formatter, "publication pre-commit rejected: {source}")
            }
        }
    }
}

impl<PrecommitError> Error for PublishAttemptError<PrecommitError> where
    PrecommitError: Error + 'static
{
}

/// Successful publication capability. Only IDs won by unique insertion are
/// retained, in compact-record order.
#[derive(Debug)]
pub struct PublicationToken<FileIdentity, RecordStorage, LocalSpatialIdentity> {
    store: Weak<Mutex<StoreState<FileIdentity, RecordStorage, LocalSpatialIdentity>>>,
    entry: Weak<PublishedEntry<FileIdentity, RecordStorage, LocalSpatialIdentity>>,
    publication_id: PublicationId,
    won_ids: Arc<[u64]>,
}

impl<FileIdentity, RecordStorage, LocalSpatialIdentity> Clone
    for PublicationToken<FileIdentity, RecordStorage, LocalSpatialIdentity>
{
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            entry: self.entry.clone(),
            publication_id: self.publication_id,
            won_ids: Arc::clone(&self.won_ids),
        }
    }
}

impl<FileIdentity, RecordStorage, LocalSpatialIdentity>
    PublicationToken<FileIdentity, RecordStorage, LocalSpatialIdentity>
{
    #[must_use]
    pub fn publication_id(&self) -> PublicationId {
        self.publication_id
    }

    /// Upgrades the token's weak file identity while its backing entry still
    /// exists. This is not an active-publication check; store operations always
    /// validate the token under the registry lock.
    #[must_use]
    pub fn file_identity(&self) -> Option<Arc<FileIdentity>> {
        self.entry.upgrade().map(|entry| Arc::clone(&entry.file))
    }

    #[must_use]
    pub fn won_ids(&self) -> &[u64] {
        &self.won_ids
    }
}

/// Safe invariant error from local-spatial release or global unpublication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnpublishError {
    ForeignStore,
    StoreLockPoisoned,
    LocalSpatialLockPoisoned,
    UnknownPublication {
        publication_id: PublicationId,
    },
    TokenIdentityMismatch {
        publication_id: PublicationId,
    },
    LocalSpatialAlreadyReleased {
        publication_id: PublicationId,
    },
    LocalSpatialReleaseInProgress {
        publication_id: PublicationId,
    },
    LocalSpatialReleasePanicked {
        publication_id: PublicationId,
    },
    LocalSpatialStillRetained {
        publication_id: PublicationId,
    },
    MissingFileEntry {
        publication_id: PublicationId,
    },
    DuplicateFileEntry {
        publication_id: PublicationId,
    },
    MissingOwnedId {
        publication_id: PublicationId,
        id: u64,
    },
    IdOwnerMismatch {
        publication_id: PublicationId,
        id: u64,
        actual_owner: PublicationId,
    },
    RecordStorageMismatch {
        publication_id: PublicationId,
        id: u64,
    },
}

impl fmt::Display for UnpublishError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for UnpublishError {}

/// Identities returned after their global references have been removed.
#[derive(Debug)]
pub struct UnpublishedStream<FileIdentity, RecordStorage> {
    pub publication_id: PublicationId,
    pub kind: StreamKind,
    pub path: NativeCPath,
    pub file: Arc<FileIdentity>,
    pub records: Arc<RecordStorage>,
    /// The exact compact-order sequence erased by this successful operation.
    pub erased_ids: Arc<[u64]>,
}

#[derive(Debug)]
struct StoreState<FileIdentity, RecordStorage, LocalSpatialIdentity> {
    next_publication_id: u64,
    records: HashMap<u64, PublishedRecordSnapshot<RecordStorage>>,
    files: [Vec<Arc<PublishedEntry<FileIdentity, RecordStorage, LocalSpatialIdentity>>>;
        STREAM_KIND_COUNT],
    active: HashMap<
        PublicationId,
        Arc<PublishedEntry<FileIdentity, RecordStorage, LocalSpatialIdentity>>,
    >,
}

impl<FileIdentity, RecordStorage, LocalSpatialIdentity> Default
    for StoreState<FileIdentity, RecordStorage, LocalSpatialIdentity>
{
    fn default() -> Self {
        Self {
            next_publication_id: 1,
            records: HashMap::new(),
            files: std::array::from_fn(|_| Vec::new()),
            active: HashMap::new(),
        }
    }
}

impl<FileIdentity, RecordStorage, LocalSpatialIdentity> Drop
    for StoreState<FileIdentity, RecordStorage, LocalSpatialIdentity>
{
    fn drop(&mut self) {
        // The last store owner is gone, so token Weak pointers can no longer
        // enter an operation. Take every store-owned local resource first.
        let mut releasing = Vec::new();
        for entry in self.active.values() {
            let mut local = match entry.local_spatial.lock() {
                Ok(local) => local,
                Err(poisoned) => poisoned.into_inner(),
            };
            if matches!(*local, LocalSpatialReleaseState::Retained(_)) {
                let LocalSpatialReleaseState::Retained(value) =
                    std::mem::replace(&mut *local, LocalSpatialReleaseState::Releasing)
                else {
                    unreachable!();
                };
                releasing.push((Arc::clone(entry), value));
            }
        }

        for (entry, local) in releasing {
            let released = catch_unwind(AssertUnwindSafe(|| drop(local))).is_ok();
            if released {
                let mut state = match entry.local_spatial.lock() {
                    Ok(state) => state,
                    Err(poisoned) => poisoned.into_inner(),
                };
                *state = LocalSpatialReleaseState::Released;
            }
        }

        // Global map/vector ownership is released only after all local values
        // have completed (or panicked during) their destructor attempt.
        self.records.clear();
        for files in &mut self.files {
            files.clear();
        }
        self.active.clear();
    }
}

/// Concrete global registry for native-equivalent published stream files.
///
/// `RecordStorage` is intentionally generic. The future adapter should pass
/// its existing `Arc<ProcessedOctreeStorage>` and supply only accepted-prefix
/// IDs as `compact_ids`; the opaque raw-vector tail never enters this API.
#[derive(Debug)]
pub struct PublishedStreamStore<FileIdentity, RecordStorage, LocalSpatialIdentity> {
    state: Arc<Mutex<StoreState<FileIdentity, RecordStorage, LocalSpatialIdentity>>>,
}

impl<FileIdentity, RecordStorage, LocalSpatialIdentity> Clone
    for PublishedStreamStore<FileIdentity, RecordStorage, LocalSpatialIdentity>
{
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

impl<FileIdentity, RecordStorage, LocalSpatialIdentity> Default
    for PublishedStreamStore<FileIdentity, RecordStorage, LocalSpatialIdentity>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<FileIdentity, RecordStorage, LocalSpatialIdentity>
    PublishedStreamStore<FileIdentity, RecordStorage, LocalSpatialIdentity>
{
    /// Exact PDB `mCollectionOctreeMap` fixed-node capacity. The native
    /// allocator's terminal exhaustion behavior remains unrecovered.
    pub const MAX_MAPPED_IDS: usize = 2048;

    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(StoreState::default())),
        }
    }

    fn lock_state(
        &self,
    ) -> Result<
        MutexGuard<'_, StoreState<FileIdentity, RecordStorage, LocalSpatialIdentity>>,
        PublishStoreError,
    > {
        self.state
            .lock()
            .map_err(|_| PublishStoreError::StoreLockPoisoned)
    }

    /// Atomically performs first-wins ID publication followed by stable file
    /// insertion. The caller must pass compact accepted-prefix IDs in source
    /// order.
    ///
    /// # Errors
    ///
    /// Returns a typed invariant, capacity, allocation, or poisoned-lock error
    /// before any logical global publication is changed.
    pub fn publish(
        &self,
        request: StreamPublicationRequest<'_, FileIdentity, RecordStorage, LocalSpatialIdentity>,
    ) -> Result<
        PublicationToken<FileIdentity, RecordStorage, LocalSpatialIdentity>,
        PublishStoreError,
    > {
        match self.publish_with_precommit(request, |_| Ok::<(), Infallible>(())) {
            Ok(token) => Ok(token),
            Err(PublishAttemptError::Store(source)) => Err(source),
            Err(PublishAttemptError::Precommit(source)) => match source {},
        }
    }

    /// Variant with a final failure-injection/coordination boundary after the
    /// complete plan is validated but before logical state is changed.
    ///
    /// The callback runs while the store lock is held and must not re-enter
    /// this store. Returning `Err` leaves both logical containers unchanged.
    ///
    /// # Errors
    ///
    /// Returns `Store` for a typed store validation failure or `Precommit`
    /// when the supplied final check rejects the complete plan.
    pub fn publish_with_precommit<PrecommitError>(
        &self,
        request: StreamPublicationRequest<'_, FileIdentity, RecordStorage, LocalSpatialIdentity>,
        precommit: impl FnOnce(&PublicationPlan<'_>) -> Result<(), PrecommitError>,
    ) -> Result<
        PublicationToken<FileIdentity, RecordStorage, LocalSpatialIdentity>,
        PublishAttemptError<PrecommitError>,
    > {
        let StreamPublicationRequest {
            kind,
            path,
            file,
            records,
            local_spatial,
            compact_ids,
        } = request;
        let mut state = self.lock_state().map_err(PublishAttemptError::Store)?;

        if state
            .active
            .values()
            .any(|entry| Arc::ptr_eq(&entry.file, &file))
        {
            return Err(PublishAttemptError::Store(
                PublishStoreError::DuplicateFileIdentity,
            ));
        }

        let publication_id = PublicationId(state.next_publication_id);
        let next_publication_id =
            state
                .next_publication_id
                .checked_add(1)
                .ok_or(PublishAttemptError::Store(
                    PublishStoreError::PublicationIdExhausted,
                ))?;

        let won =
            plan_winning_ids(&state.records, compact_ids).map_err(PublishAttemptError::Store)?;

        let maximum = crate::skate3::stream_capacity(Self::MAX_MAPPED_IDS);
        if won.len() > maximum.saturating_sub(state.records.len()) {
            return Err(PublishAttemptError::Store(
                PublishStoreError::IdCapacityExceeded {
                    mapped: state.records.len(),
                    attempted_new: won.len(),
                    maximum,
                },
            ));
        }

        let slot = stream_kind_index(kind);
        let insert_index = state.files[slot]
            .iter()
            .position(|entry| compare_native_stream_file_paths(&path, &entry.path).is_lt())
            .unwrap_or(state.files[slot].len());

        state
            .records
            .try_reserve(won.len())
            .map_err(|_| PublishAttemptError::Store(PublishStoreError::AllocationFailed))?;
        state.files[slot]
            .try_reserve(1)
            .map_err(|_| PublishAttemptError::Store(PublishStoreError::AllocationFailed))?;
        state
            .active
            .try_reserve(1)
            .map_err(|_| PublishAttemptError::Store(PublishStoreError::AllocationFailed))?;

        let mut won_ids = Vec::new();
        won_ids
            .try_reserve(won.len())
            .map_err(|_| PublishAttemptError::Store(PublishStoreError::AllocationFailed))?;
        won_ids.extend(won.iter().map(|(id, _)| *id));
        let won_ids: Arc<[u64]> = won_ids.into();
        precommit(&PublicationPlan {
            publication_id,
            kind,
            path: &path,
            compact_record_count: compact_ids.len(),
            won_ids: &won_ids,
            file_insert_index: insert_index,
            mapped_id_count_before: state.records.len(),
        })
        .map_err(PublishAttemptError::Precommit)?;

        let entry = Arc::new(PublishedEntry {
            id: publication_id,
            kind,
            path,
            file,
            records,
            local_spatial: Mutex::new(LocalSpatialReleaseState::Retained(local_spatial)),
        });
        for (id, compact_index) in won {
            state.records.insert(
                id,
                PublishedRecordSnapshot {
                    publication_id,
                    compact_index,
                    records: Arc::clone(&entry.records),
                },
            );
        }
        state.files[slot].insert(insert_index, Arc::clone(&entry));
        state.active.insert(publication_id, Arc::clone(&entry));
        state.next_publication_id = next_publication_id;

        Ok(PublicationToken {
            store: Arc::downgrade(&self.state),
            entry: Arc::downgrade(&entry),
            publication_id,
            won_ids,
        })
    }

    fn validate_token<'a>(
        &'a self,
        state: &'a StoreState<FileIdentity, RecordStorage, LocalSpatialIdentity>,
        token: &PublicationToken<FileIdentity, RecordStorage, LocalSpatialIdentity>,
    ) -> Result<
        &'a Arc<PublishedEntry<FileIdentity, RecordStorage, LocalSpatialIdentity>>,
        UnpublishError,
    > {
        if !Weak::ptr_eq(&token.store, &Arc::downgrade(&self.state)) {
            return Err(UnpublishError::ForeignStore);
        }
        let id = token.publication_id;
        let active = state
            .active
            .get(&id)
            .ok_or(UnpublishError::UnknownPublication { publication_id: id })?;
        if !Weak::ptr_eq(&token.entry, &Arc::downgrade(active)) {
            return Err(UnpublishError::TokenIdentityMismatch { publication_id: id });
        }
        Ok(active)
    }

    /// Releases the stream-file aggregate's local-spatial ownership before
    /// global unpublication. Native performs the equivalent local Octree free
    /// at `0x824e1420..0x824e1468`, before state 6 and `RemoveStreamFile`.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a foreign, inactive, mismatched, repeated, or
    /// poisoned publication capability.
    pub fn release_local_spatial(
        &self,
        token: &PublicationToken<FileIdentity, RecordStorage, LocalSpatialIdentity>,
    ) -> Result<(), UnpublishError> {
        let (entry, local_spatial) = {
            let state = self
                .state
                .lock()
                .map_err(|_| UnpublishError::StoreLockPoisoned)?;
            let entry = Arc::clone(self.validate_token(&state, token)?);
            let mut local = entry
                .local_spatial
                .lock()
                .map_err(|_| UnpublishError::LocalSpatialLockPoisoned)?;
            let prior = std::mem::replace(&mut *local, LocalSpatialReleaseState::Releasing);
            let value = match prior {
                LocalSpatialReleaseState::Retained(value) => value,
                LocalSpatialReleaseState::Releasing => {
                    return Err(UnpublishError::LocalSpatialReleaseInProgress {
                        publication_id: entry.id,
                    });
                }
                LocalSpatialReleaseState::Released => {
                    *local = LocalSpatialReleaseState::Released;
                    return Err(UnpublishError::LocalSpatialAlreadyReleased {
                        publication_id: entry.id,
                    });
                }
            };
            drop(local);
            drop(state);
            (entry, value)
        };

        // Destruction runs without either store lock. A reentrant attempt sees
        // `Releasing` and cannot globally unpublish or deadlock this call.
        if catch_unwind(AssertUnwindSafe(|| drop(local_spatial))).is_err() {
            return Err(UnpublishError::LocalSpatialReleasePanicked {
                publication_id: entry.id,
            });
        }

        let mut local = entry
            .local_spatial
            .lock()
            .map_err(|_| UnpublishError::LocalSpatialLockPoisoned)?;
        debug_assert!(matches!(*local, LocalSpatialReleaseState::Releasing));
        *local = LocalSpatialReleaseState::Released;
        Ok(())
    }

    /// Atomically owner-checks and removes the token's won IDs in compact
    /// order, then removes its exact file identity while preserving all other
    /// file ordering.
    ///
    /// # Errors
    ///
    /// Returns a typed error without mutation if the token is inactive or any
    /// local-spatial, file-vector, ID-owner, or record-storage invariant fails.
    pub fn unpublish(
        &self,
        token: &PublicationToken<FileIdentity, RecordStorage, LocalSpatialIdentity>,
    ) -> Result<UnpublishedStream<FileIdentity, RecordStorage>, UnpublishError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| UnpublishError::StoreLockPoisoned)?;
        let entry = Arc::clone(self.validate_token(&state, token)?);
        let id = entry.id;

        match &*entry
            .local_spatial
            .lock()
            .map_err(|_| UnpublishError::LocalSpatialLockPoisoned)?
        {
            LocalSpatialReleaseState::Retained(_) => {
                return Err(UnpublishError::LocalSpatialStillRetained { publication_id: id });
            }
            LocalSpatialReleaseState::Releasing => {
                return Err(UnpublishError::LocalSpatialReleaseInProgress { publication_id: id });
            }
            LocalSpatialReleaseState::Released => {}
        }

        let slot = stream_kind_index(entry.kind);
        let positions: Vec<_> = state.files[slot]
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| Arc::ptr_eq(candidate, &entry).then_some(index))
            .collect();
        let file_index = match positions.as_slice() {
            [] => {
                return Err(UnpublishError::MissingFileEntry { publication_id: id });
            }
            [index] => *index,
            _ => {
                return Err(UnpublishError::DuplicateFileEntry { publication_id: id });
            }
        };

        for mapped_id in token.won_ids.iter().copied() {
            let record = state
                .records
                .get(&mapped_id)
                .ok_or(UnpublishError::MissingOwnedId {
                    publication_id: id,
                    id: mapped_id,
                })?;
            if record.publication_id != id {
                return Err(UnpublishError::IdOwnerMismatch {
                    publication_id: id,
                    id: mapped_id,
                    actual_owner: record.publication_id,
                });
            }
            if !Arc::ptr_eq(&record.records, &entry.records) {
                return Err(UnpublishError::RecordStorageMismatch {
                    publication_id: id,
                    id: mapped_id,
                });
            }
        }

        for mapped_id in token.won_ids.iter() {
            state.records.remove(mapped_id);
        }
        state.files[slot].remove(file_index);
        state.active.remove(&id);

        Ok(UnpublishedStream {
            publication_id: id,
            kind: entry.kind,
            path: entry.path.clone(),
            file: Arc::clone(&entry.file),
            records: Arc::clone(&entry.records),
            erased_ids: Arc::clone(&token.won_ids),
        })
    }

    /// Returns an owning record snapshot after releasing the store lock.
    ///
    /// # Errors
    ///
    /// Returns `StoreLockPoisoned` if a previous panic poisoned the registry.
    pub fn record_snapshot(
        &self,
        id: u64,
    ) -> Result<Option<PublishedRecordSnapshot<RecordStorage>>, UnpublishError> {
        let state = self
            .state
            .lock()
            .map_err(|_| UnpublishError::StoreLockPoisoned)?;
        Ok(state.records.get(&id).cloned())
    }

    /// Returns owning snapshots in exact per-kind stable vector order.
    ///
    /// # Errors
    ///
    /// Returns `StoreLockPoisoned` if a previous panic poisoned the registry.
    pub fn file_snapshots(
        &self,
        kind: StreamKind,
    ) -> Result<Vec<PublishedFileSnapshot<FileIdentity, RecordStorage>>, UnpublishError> {
        let state = self
            .state
            .lock()
            .map_err(|_| UnpublishError::StoreLockPoisoned)?;
        Ok(state.files[stream_kind_index(kind)]
            .iter()
            .map(file_snapshot)
            .collect())
    }

    /// Takes an owning, coherent snapshot of both containers under one lock.
    ///
    /// # Errors
    ///
    /// Returns `StoreLockPoisoned` if a previous panic poisoned the registry.
    pub fn snapshot(
        &self,
    ) -> Result<PublishedStreamStoreSnapshot<FileIdentity, RecordStorage>, UnpublishError> {
        let state = self
            .state
            .lock()
            .map_err(|_| UnpublishError::StoreLockPoisoned)?;
        Ok(PublishedStreamStoreSnapshot {
            records: state.records.clone(),
            files: std::array::from_fn(|slot| {
                state.files[slot].iter().map(file_snapshot).collect()
            }),
        })
    }
}

fn file_snapshot<FileIdentity, RecordStorage, LocalSpatialIdentity>(
    entry: &Arc<PublishedEntry<FileIdentity, RecordStorage, LocalSpatialIdentity>>,
) -> PublishedFileSnapshot<FileIdentity, RecordStorage> {
    PublishedFileSnapshot {
        publication_id: entry.id,
        kind: entry.kind,
        path: entry.path.clone(),
        file: Arc::clone(&entry.file),
        records: Arc::clone(&entry.records),
    }
}

fn plan_winning_ids<RecordStorage>(
    mapped: &HashMap<u64, PublishedRecordSnapshot<RecordStorage>>,
    compact_ids: &[u64],
) -> Result<Vec<(u64, u32)>, PublishStoreError> {
    let mut won = Vec::new();
    won.try_reserve(compact_ids.len())
        .map_err(|_| PublishStoreError::AllocationFailed)?;
    for (index, id) in compact_ids.iter().copied().enumerate() {
        let compact_index =
            u32::try_from(index).map_err(|_| PublishStoreError::CompactIndexOverflow { index })?;
        if !mapped.contains_key(&id) && !won.iter().any(|(won_id, _)| *won_id == id) {
            won.push((id, compact_index));
        }
    }
    Ok(won)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashSet,
        sync::{
            Barrier,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        thread,
    };

    fn path(bytes: &'static [u8]) -> NativeCPath {
        NativeCPath::from_bytes(bytes)
    }

    fn publish(
        store: &PublishedStreamStore<usize, usize, usize>,
        kind: StreamKind,
        bytes: &'static [u8],
        file: Arc<usize>,
        records: Arc<usize>,
        ids: &[u64],
    ) -> PublicationToken<usize, usize, usize> {
        store
            .publish(StreamPublicationRequest {
                kind,
                path: path(bytes),
                file,
                records,
                local_spatial: 0,
                compact_ids: ids,
            })
            .unwrap()
    }

    #[test]
    fn canonical_stream_kind_maps_to_exact_vector_slots() {
        assert_eq!(stream_kind_index(StreamKind::Presentation), 0);
        assert_eq!(stream_kind_index(StreamKind::Simulation), 1);
        assert_eq!(stream_kind_index(StreamKind::Texture), 2);
    }

    #[test]
    fn unsigned_first_nul_order_and_stable_equal_insertion_are_exact() {
        let store = PublishedStreamStore::new();
        let inputs: &[&'static [u8]] = &[b"B", b"A", b"B\0tail", b"B\x80", b"B\xff", b"B\0later"];
        for (serial, bytes) in inputs.iter().enumerate() {
            publish(
                &store,
                StreamKind::Presentation,
                bytes,
                Arc::new(serial),
                Arc::new(serial),
                &[],
            );
        }
        let files = store.file_snapshots(StreamKind::Presentation).unwrap();
        let stored: Vec<_> = files.iter().map(|file| file.path.stored_bytes()).collect();
        assert_eq!(
            stored,
            [
                b"A".as_slice(),
                b"B",
                b"B\0tail",
                b"B\0later",
                b"B\x80",
                b"B\xff"
            ]
        );
        assert_eq!(
            compare_native_stream_file_paths(&path(b"B\0x"), &path(b"B\0y")),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_native_stream_file_paths(&path(b"B\x80"), &path(b"B\x7f")),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn zero_record_files_and_more_than_sixteen_files_are_published_per_kind() {
        let store = PublishedStreamStore::new();
        for serial in 0..33 {
            publish(
                &store,
                StreamKind::Simulation,
                b"same",
                Arc::new(serial),
                Arc::new(serial),
                &[],
            );
        }
        assert!(store.record_snapshot(1).unwrap().is_none());
        let files = store.file_snapshots(StreamKind::Simulation).unwrap();
        assert_eq!(files.len(), 33);
        assert_eq!(
            files.iter().map(|file| *file.file).collect::<Vec<_>>(),
            (0..33).collect::<Vec<_>>()
        );
        assert!(
            store
                .file_snapshots(StreamKind::Presentation)
                .unwrap()
                .is_empty()
        );
        assert!(
            store
                .file_snapshots(StreamKind::Texture)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn identical_paths_remain_separated_by_exact_file_kind() {
        let store = PublishedStreamStore::new();
        for (serial, kind) in [
            StreamKind::Presentation,
            StreamKind::Simulation,
            StreamKind::Texture,
        ]
        .into_iter()
        .enumerate()
        {
            publish(
                &store,
                kind,
                b"shared",
                Arc::new(serial),
                Arc::new(serial),
                &[],
            );
        }
        for kind in [
            StreamKind::Presentation,
            StreamKind::Simulation,
            StreamKind::Texture,
        ] {
            let files = store.file_snapshots(kind).unwrap();
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].kind, kind);
        }
    }

    #[test]
    fn only_compact_prefix_ids_enter_the_global_map() {
        #[derive(Debug)]
        struct RawAndCompact {
            raw_logical_len: usize,
            accepted: Vec<u64>,
        }
        let store = PublishedStreamStore::new();
        let storage = Arc::new(RawAndCompact {
            raw_logical_len: 5,
            accepted: vec![10, 11],
        });
        let token = store
            .publish(StreamPublicationRequest {
                kind: StreamKind::Presentation,
                path: path(b"prefix"),
                file: Arc::new(1),
                records: Arc::clone(&storage),
                local_spatial: 0,
                compact_ids: &storage.accepted,
            })
            .unwrap();
        assert_eq!(storage.raw_logical_len, 5);
        assert_eq!(token.won_ids(), [10, 11]);
        assert_eq!(store.snapshot().unwrap().records.len(), 2);
        assert_eq!(store.record_snapshot(10).unwrap().unwrap().compact_index, 0);
        assert_eq!(store.record_snapshot(11).unwrap().unwrap().compact_index, 1);
    }

    #[test]
    fn combined_snapshot_preserves_shared_file_and_record_storage_identity() {
        let store = PublishedStreamStore::new();
        let file = Arc::new(41);
        let records = Arc::new(73);
        let token = store
            .publish(StreamPublicationRequest {
                kind: StreamKind::Simulation,
                path: path(b"snapshot"),
                file: Arc::clone(&file),
                records: Arc::clone(&records),
                local_spatial: 0,
                compact_ids: &[9],
            })
            .unwrap();
        let snapshot = store.snapshot().unwrap();
        let published_file = &snapshot.files[stream_kind_index(StreamKind::Simulation)][0];
        let published_record = snapshot.records.get(&9).unwrap();
        assert!(Arc::ptr_eq(&published_file.file, &file));
        assert!(Arc::ptr_eq(&published_file.records, &records));
        assert!(Arc::ptr_eq(&published_record.records, &records));
        assert!(Arc::ptr_eq(
            &published_file.records,
            &published_record.records
        ));
        assert!(Arc::ptr_eq(&token.file_identity().unwrap(), &file));
    }

    #[test]
    fn within_file_duplicates_are_first_wins_and_erase_once_in_compact_order() {
        let store = PublishedStreamStore::new();
        let token = publish(
            &store,
            StreamKind::Presentation,
            b"dup",
            Arc::new(1),
            Arc::new(100),
            &[7, 7, 8, 7, 9],
        );
        assert_eq!(token.won_ids(), [7, 8, 9]);
        assert_eq!(store.record_snapshot(7).unwrap().unwrap().compact_index, 0);
        assert_eq!(store.record_snapshot(8).unwrap().unwrap().compact_index, 2);
        assert_eq!(store.record_snapshot(9).unwrap().unwrap().compact_index, 4);
        store.release_local_spatial(&token).unwrap();
        let unpublished = store.unpublish(&token).unwrap();
        assert_eq!(&*unpublished.erased_ids, [7, 8, 9]);
        assert!(store.snapshot().unwrap().records.is_empty());
    }

    #[test]
    fn cross_file_collision_publishes_both_files_and_loser_cannot_erase_winner() {
        let store = PublishedStreamStore::new();
        let first = publish(
            &store,
            StreamKind::Presentation,
            b"same",
            Arc::new(1),
            Arc::new(101),
            &[77],
        );
        let second = publish(
            &store,
            StreamKind::Presentation,
            b"same",
            Arc::new(2),
            Arc::new(202),
            &[77],
        );
        assert_eq!(first.won_ids(), [77]);
        assert!(second.won_ids().is_empty());
        assert_eq!(
            store
                .file_snapshots(StreamKind::Presentation)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            store.record_snapshot(77).unwrap().unwrap().publication_id,
            first.publication_id()
        );

        store.release_local_spatial(&second).unwrap();
        store.unpublish(&second).unwrap();
        assert_eq!(
            store.record_snapshot(77).unwrap().unwrap().publication_id,
            first.publication_id()
        );

        store.release_local_spatial(&first).unwrap();
        store.unpublish(&first).unwrap();
        assert!(store.record_snapshot(77).unwrap().is_none());

        let no_promotion_store = PublishedStreamStore::new();
        let winner = publish(
            &no_promotion_store,
            StreamKind::Simulation,
            b"winner",
            Arc::new(3),
            Arc::new(303),
            &[88],
        );
        let loser = publish(
            &no_promotion_store,
            StreamKind::Simulation,
            b"loser",
            Arc::new(4),
            Arc::new(404),
            &[88],
        );
        no_promotion_store.release_local_spatial(&winner).unwrap();
        no_promotion_store.unpublish(&winner).unwrap();
        assert!(no_promotion_store.record_snapshot(88).unwrap().is_none());
        assert_eq!(
            no_promotion_store
                .file_snapshots(StreamKind::Simulation)
                .unwrap()
                .len(),
            1
        );
        no_promotion_store.release_local_spatial(&loser).unwrap();
        no_promotion_store.unpublish(&loser).unwrap();
    }

    #[test]
    fn id_first_wins_is_global_across_file_kinds() {
        let store = PublishedStreamStore::new();
        let presentation = publish(
            &store,
            StreamKind::Presentation,
            b"pres",
            Arc::new(1),
            Arc::new(101),
            &[55],
        );
        let texture = publish(
            &store,
            StreamKind::Texture,
            b"tex",
            Arc::new(2),
            Arc::new(202),
            &[55],
        );
        assert_eq!(presentation.won_ids(), [55]);
        assert!(texture.won_ids().is_empty());
        assert_eq!(
            store.record_snapshot(55).unwrap().unwrap().publication_id,
            presentation.publication_id()
        );
        assert_eq!(
            store
                .file_snapshots(StreamKind::Presentation)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(store.file_snapshots(StreamKind::Texture).unwrap().len(), 1);

        store.release_local_spatial(&texture).unwrap();
        store.unpublish(&texture).unwrap();
        assert_eq!(
            store.record_snapshot(55).unwrap().unwrap().publication_id,
            presentation.publication_id()
        );
    }

    #[test]
    fn fixed_id_capacity_counts_only_new_distinct_winners() {
        let store = PublishedStreamStore::new();
        let first_ids: Vec<_> = (0..2047_u64).collect();
        let first = publish(
            &store,
            StreamKind::Presentation,
            b"first",
            Arc::new(1),
            Arc::new(1),
            &first_ids,
        );
        assert_eq!(first.won_ids().len(), 2047);

        let second = publish(
            &store,
            StreamKind::Simulation,
            b"second",
            Arc::new(2),
            Arc::new(2),
            &[0, 2047, 2047],
        );
        assert_eq!(second.won_ids(), [2047]);
        assert_eq!(store.snapshot().unwrap().records.len(), 2048);

        let duplicate_only = publish(
            &store,
            StreamKind::Texture,
            b"duplicate-only-at-capacity",
            Arc::new(20),
            Arc::new(20),
            &[0, 2047, 0],
        );
        assert!(duplicate_only.won_ids().is_empty());
        assert_eq!(store.snapshot().unwrap().records.len(), 2048);

        let before = store.snapshot().unwrap();
        let error = store
            .publish(StreamPublicationRequest {
                kind: StreamKind::Texture,
                path: path(b"third"),
                file: Arc::new(3),
                records: Arc::new(3),
                local_spatial: 3,
                compact_ids: &[2048],
            })
            .unwrap_err();
        assert_eq!(
            error,
            PublishStoreError::IdCapacityExceeded {
                mapped: 2048,
                attempted_new: 1,
                maximum: 2048,
            }
        );
        let after = store.snapshot().unwrap();
        assert_eq!(after.records.len(), before.records.len());
        assert_eq!(
            after.files.iter().map(Vec::len).collect::<Vec<_>>(),
            before.files.iter().map(Vec::len).collect::<Vec<_>>()
        );
    }

    #[test]
    fn repeated_publication_and_repeated_token_use_are_rejected() {
        let store = PublishedStreamStore::new();
        let file = Arc::new(1);
        let token = publish(
            &store,
            StreamKind::Presentation,
            b"repeat",
            Arc::clone(&file),
            Arc::new(1),
            &[1],
        );
        assert_eq!(
            store
                .publish(StreamPublicationRequest {
                    kind: StreamKind::Presentation,
                    path: path(b"repeat"),
                    file,
                    records: Arc::new(2),
                    local_spatial: 2,
                    compact_ids: &[2],
                })
                .unwrap_err(),
            PublishStoreError::DuplicateFileIdentity
        );
        store.release_local_spatial(&token).unwrap();
        assert_eq!(
            store.release_local_spatial(&token),
            Err(UnpublishError::LocalSpatialAlreadyReleased {
                publication_id: token.publication_id()
            })
        );
        store.unpublish(&token).unwrap();
        assert_eq!(
            store.unpublish(&token).unwrap_err(),
            UnpublishError::UnknownPublication {
                publication_id: token.publication_id()
            }
        );
    }

    #[test]
    fn local_spatial_release_precedes_global_unpublish_and_arcs_survive_it() {
        #[derive(Debug)]
        struct Probe(Arc<AtomicUsize>);
        impl Drop for Probe {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let store = PublishedStreamStore::new();
        let file_drops = Arc::new(AtomicUsize::new(0));
        let record_drops = Arc::new(AtomicUsize::new(0));
        let local_drops = Arc::new(AtomicUsize::new(0));
        let file = Arc::new(Probe(Arc::clone(&file_drops)));
        let records = Arc::new(Probe(Arc::clone(&record_drops)));
        let token = store
            .publish(StreamPublicationRequest {
                kind: StreamKind::Presentation,
                path: path(b"drops"),
                file: Arc::clone(&file),
                records: Arc::clone(&records),
                local_spatial: Probe(Arc::clone(&local_drops)),
                compact_ids: &[1],
            })
            .unwrap();
        drop(file);
        drop(records);

        assert_eq!(
            store.unpublish(&token).unwrap_err(),
            UnpublishError::LocalSpatialStillRetained {
                publication_id: token.publication_id()
            }
        );
        store.release_local_spatial(&token).unwrap();
        assert_eq!(local_drops.load(Ordering::SeqCst), 1);
        assert_eq!(file_drops.load(Ordering::SeqCst), 0);
        assert_eq!(record_drops.load(Ordering::SeqCst), 0);

        let unpublished = store.unpublish(&token).unwrap();
        assert_eq!(file_drops.load(Ordering::SeqCst), 0);
        assert_eq!(record_drops.load(Ordering::SeqCst), 0);
        drop(unpublished);
        assert_eq!(file_drops.load(Ordering::SeqCst), 1);
        assert_eq!(record_drops.load(Ordering::SeqCst), 1);
        assert!(token.file_identity().is_none());
        drop(token);
        assert_eq!(file_drops.load(Ordering::SeqCst), 1);
        assert_eq!(record_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn transferred_shared_local_ownership_does_not_overclaim_external_alias_destruction() {
        #[derive(Debug)]
        struct Probe(Arc<AtomicUsize>);
        impl Drop for Probe {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let drops = Arc::new(AtomicUsize::new(0));
        let external = Arc::new(Probe(Arc::clone(&drops)));
        let store = PublishedStreamStore::new();
        let token = store
            .publish(StreamPublicationRequest {
                kind: StreamKind::Presentation,
                path: path(b"shared-local"),
                file: Arc::new(1),
                records: Arc::new(1),
                local_spatial: Arc::clone(&external),
                compact_ids: &[],
            })
            .unwrap();

        store.release_local_spatial(&token).unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(external);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        store.unpublish(&token).unwrap();
    }

    #[test]
    fn releasing_blocks_concurrent_release_and_unpublish_until_drop_finishes() {
        use std::sync::mpsc;

        struct BlockingLocal {
            entered: mpsc::Sender<()>,
            proceed: mpsc::Receiver<()>,
        }
        impl Drop for BlockingLocal {
            fn drop(&mut self) {
                self.entered.send(()).unwrap();
                self.proceed.recv().unwrap();
            }
        }

        let (entered_tx, entered_rx) = mpsc::channel();
        let (proceed_tx, proceed_rx) = mpsc::channel();
        let store = PublishedStreamStore::new();
        let token = store
            .publish(StreamPublicationRequest {
                kind: StreamKind::Presentation,
                path: path(b"blocking-local"),
                file: Arc::new(1),
                records: Arc::new(1),
                local_spatial: BlockingLocal {
                    entered: entered_tx,
                    proceed: proceed_rx,
                },
                compact_ids: &[1],
            })
            .unwrap();

        let releasing_store = store.clone();
        let releasing_token = token.clone();
        let releaser =
            thread::spawn(move || releasing_store.release_local_spatial(&releasing_token));
        entered_rx.recv().unwrap();
        assert_eq!(
            store.unpublish(&token).unwrap_err(),
            UnpublishError::LocalSpatialReleaseInProgress {
                publication_id: token.publication_id()
            }
        );
        assert_eq!(
            store.release_local_spatial(&token),
            Err(UnpublishError::LocalSpatialReleaseInProgress {
                publication_id: token.publication_id()
            })
        );
        proceed_tx.send(()).unwrap();
        assert_eq!(releaser.join().unwrap(), Ok(()));
        store.unpublish(&token).unwrap();
    }

    #[test]
    fn local_destructor_can_reenter_without_deadlock_but_cannot_unpublish_early() {
        use std::sync::mpsc;

        type Action = Box<dyn FnOnce() + Send>;
        struct ReentrantLocal(Arc<Mutex<Option<Action>>>);
        impl Drop for ReentrantLocal {
            fn drop(&mut self) {
                let action = self.0.lock().unwrap().take();
                if let Some(action) = action {
                    action();
                }
            }
        }

        let action = Arc::new(Mutex::new(None::<Action>));
        let store = PublishedStreamStore::new();
        let token = store
            .publish(StreamPublicationRequest {
                kind: StreamKind::Presentation,
                path: path(b"reentrant"),
                file: Arc::new(1),
                records: Arc::new(1),
                local_spatial: ReentrantLocal(Arc::clone(&action)),
                compact_ids: &[1],
            })
            .unwrap();
        let callback_store = store.clone();
        let callback_token = token.clone();
        let (result_tx, result_rx) = mpsc::channel();
        *action.lock().unwrap() = Some(Box::new(move || {
            result_tx
                .send(callback_store.unpublish(&callback_token))
                .unwrap();
        }));

        store.release_local_spatial(&token).unwrap();
        assert_eq!(
            result_rx.recv().unwrap().unwrap_err(),
            UnpublishError::LocalSpatialReleaseInProgress {
                publication_id: token.publication_id()
            }
        );
        store.unpublish(&token).unwrap();
    }

    #[test]
    fn destructor_panic_leaves_release_non_publishable() {
        struct PanicLocal;
        impl Drop for PanicLocal {
            fn drop(&mut self) {
                panic!("injected local destructor panic");
            }
        }

        let store = PublishedStreamStore::new();
        let token = store
            .publish(StreamPublicationRequest {
                kind: StreamKind::Presentation,
                path: path(b"panic-local"),
                file: Arc::new(1),
                records: Arc::new(1),
                local_spatial: PanicLocal,
                compact_ids: &[1],
            })
            .unwrap();
        assert_eq!(
            store.release_local_spatial(&token),
            Err(UnpublishError::LocalSpatialReleasePanicked {
                publication_id: token.publication_id()
            })
        );
        assert_eq!(
            store.unpublish(&token).unwrap_err(),
            UnpublishError::LocalSpatialReleaseInProgress {
                publication_id: token.publication_id()
            }
        );
    }

    #[test]
    fn dropping_last_store_releases_local_before_global_identities_despite_live_token() {
        #[derive(Debug)]
        struct OrderedDrop(&'static str, Arc<Mutex<Vec<&'static str>>>);
        impl Drop for OrderedDrop {
            fn drop(&mut self) {
                self.1.lock().unwrap().push(self.0);
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let store = PublishedStreamStore::new();
        let token = store
            .publish(StreamPublicationRequest {
                kind: StreamKind::Presentation,
                path: path(b"store-drop"),
                file: Arc::new(OrderedDrop("file", Arc::clone(&events))),
                records: Arc::new(OrderedDrop("records", Arc::clone(&events))),
                local_spatial: OrderedDrop("local", Arc::clone(&events)),
                compact_ids: &[1],
            })
            .unwrap();
        assert!(token.file_identity().is_some());

        drop(store);
        assert_eq!(&*events.lock().unwrap(), &["local", "file", "records"]);
        assert!(token.file_identity().is_none());
    }

    #[test]
    fn exact_identity_removal_preserves_remaining_equal_path_order() {
        let store = PublishedStreamStore::new();
        let mut tokens = Vec::new();
        for serial in 1..=3 {
            tokens.push(publish(
                &store,
                StreamKind::Presentation,
                b"equal",
                Arc::new(serial),
                Arc::new(serial),
                &[],
            ));
        }
        store.release_local_spatial(&tokens[1]).unwrap();
        store.unpublish(&tokens[1]).unwrap();
        let files = store.file_snapshots(StreamKind::Presentation).unwrap();
        assert_eq!(
            files.iter().map(|file| *file.file).collect::<Vec<_>>(),
            [1, 3]
        );
    }

    #[test]
    fn every_unpublish_invariant_is_validated_before_any_removal() {
        let missing_map_store = PublishedStreamStore::new();
        let missing_map = publish(
            &missing_map_store,
            StreamKind::Presentation,
            b"map",
            Arc::new(1),
            Arc::new(1),
            &[1, 2],
        );
        missing_map_store
            .release_local_spatial(&missing_map)
            .unwrap();
        missing_map_store.state.lock().unwrap().records.remove(&2);
        assert!(matches!(
            missing_map_store.unpublish(&missing_map),
            Err(UnpublishError::MissingOwnedId { id: 2, .. })
        ));
        assert!(missing_map_store.record_snapshot(1).unwrap().is_some());
        assert_eq!(
            missing_map_store
                .file_snapshots(StreamKind::Presentation)
                .unwrap()
                .len(),
            1
        );

        let wrong_owner_store = PublishedStreamStore::new();
        let wrong_owner = publish(
            &wrong_owner_store,
            StreamKind::Presentation,
            b"owner",
            Arc::new(1),
            Arc::new(1),
            &[1, 2],
        );
        wrong_owner_store
            .release_local_spatial(&wrong_owner)
            .unwrap();
        wrong_owner_store
            .state
            .lock()
            .unwrap()
            .records
            .get_mut(&2)
            .unwrap()
            .publication_id = PublicationId(999);
        assert!(matches!(
            wrong_owner_store.unpublish(&wrong_owner),
            Err(UnpublishError::IdOwnerMismatch { id: 2, .. })
        ));
        assert!(wrong_owner_store.record_snapshot(1).unwrap().is_some());

        let missing_file_store = PublishedStreamStore::new();
        let missing_file = publish(
            &missing_file_store,
            StreamKind::Presentation,
            b"file",
            Arc::new(1),
            Arc::new(1),
            &[1],
        );
        missing_file_store
            .release_local_spatial(&missing_file)
            .unwrap();
        missing_file_store.state.lock().unwrap().files[0].clear();
        assert!(matches!(
            missing_file_store.unpublish(&missing_file),
            Err(UnpublishError::MissingFileEntry { .. })
        ));
        assert!(missing_file_store.record_snapshot(1).unwrap().is_some());
    }

    #[test]
    fn duplicate_file_entry_is_rejected_before_any_removal() {
        let duplicate_file_store = PublishedStreamStore::new();
        let duplicate_file = publish(
            &duplicate_file_store,
            StreamKind::Presentation,
            b"duplicate-file",
            Arc::new(1),
            Arc::new(1),
            &[1],
        );
        duplicate_file_store
            .release_local_spatial(&duplicate_file)
            .unwrap();
        {
            let mut state = duplicate_file_store.state.lock().unwrap();
            let entry = Arc::clone(state.active.get(&duplicate_file.publication_id()).unwrap());
            state.files[stream_kind_index(StreamKind::Presentation)].push(entry);
        }
        assert!(matches!(
            duplicate_file_store.unpublish(&duplicate_file),
            Err(UnpublishError::DuplicateFileEntry { .. })
        ));
        assert!(duplicate_file_store.record_snapshot(1).unwrap().is_some());
        assert_eq!(
            duplicate_file_store
                .file_snapshots(StreamKind::Presentation)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn record_storage_mismatch_is_rejected_before_any_removal() {
        let storage_mismatch_store = PublishedStreamStore::new();
        let storage_mismatch = publish(
            &storage_mismatch_store,
            StreamKind::Simulation,
            b"storage",
            Arc::new(1),
            Arc::new(1),
            &[1, 2],
        );
        storage_mismatch_store
            .release_local_spatial(&storage_mismatch)
            .unwrap();
        storage_mismatch_store
            .state
            .lock()
            .unwrap()
            .records
            .get_mut(&2)
            .unwrap()
            .records = Arc::new(999);
        assert!(matches!(
            storage_mismatch_store.unpublish(&storage_mismatch),
            Err(UnpublishError::RecordStorageMismatch { id: 2, .. })
        ));
        assert!(storage_mismatch_store.record_snapshot(1).unwrap().is_some());
        assert!(storage_mismatch_store.record_snapshot(2).unwrap().is_some());
        assert_eq!(
            storage_mismatch_store
                .file_snapshots(StreamKind::Simulation)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn precommit_failure_leaves_no_logical_publication() {
        let store = PublishedStreamStore::<usize, usize, usize>::new();
        let error = store
            .publish_with_precommit(
                StreamPublicationRequest {
                    kind: StreamKind::Presentation,
                    path: path(b"fail"),
                    file: Arc::new(1),
                    records: Arc::new(1),
                    local_spatial: 1,
                    compact_ids: &[1, 2],
                },
                |plan| {
                    assert_eq!(plan.won_ids, [1, 2]);
                    assert_eq!(plan.file_insert_index, 0);
                    Err("injected")
                },
            )
            .unwrap_err();
        assert!(matches!(error, PublishAttemptError::Precommit("injected")));
        let snapshot = store.snapshot().unwrap();
        assert!(snapshot.records.is_empty());
        assert!(snapshot.files.iter().all(Vec::is_empty));
    }

    #[test]
    fn foreign_tokens_are_rejected_without_touching_either_store() {
        let first = PublishedStreamStore::new();
        let second = PublishedStreamStore::new();
        let token = publish(
            &first,
            StreamKind::Presentation,
            b"foreign",
            Arc::new(1),
            Arc::new(1),
            &[1],
        );
        assert_eq!(
            second.release_local_spatial(&token),
            Err(UnpublishError::ForeignStore)
        );
        assert_eq!(
            second.unpublish(&token).unwrap_err(),
            UnpublishError::ForeignStore
        );
        assert!(first.record_snapshot(1).unwrap().is_some());
        assert!(second.snapshot().unwrap().records.is_empty());
    }

    #[test]
    fn concurrent_snapshots_never_observe_cross_container_partial_state() {
        let store = Arc::new(PublishedStreamStore::<usize, usize, usize>::new());
        let start = Arc::new(Barrier::new(2));
        let done = Arc::new(AtomicBool::new(false));

        let writer_store = Arc::clone(&store);
        let writer_start = Arc::clone(&start);
        let writer_done = Arc::clone(&done);
        let writer = thread::spawn(move || {
            writer_start.wait();
            for serial in 0..200_usize {
                let id = u64::try_from(serial).unwrap();
                let token = writer_store
                    .publish(StreamPublicationRequest {
                        kind: StreamKind::Presentation,
                        path: NativeCPath::from_bytes(format!("{serial:03}").into_bytes()),
                        file: Arc::new(serial),
                        records: Arc::new(serial),
                        local_spatial: serial,
                        compact_ids: &[id],
                    })
                    .unwrap();
                writer_store.release_local_spatial(&token).unwrap();
                writer_store.unpublish(&token).unwrap();
            }
            writer_done.store(true, Ordering::Release);
        });

        start.wait();
        while !done.load(Ordering::Acquire) {
            let snapshot = store.snapshot().unwrap();
            let file_ids: HashSet<_> = snapshot
                .files
                .iter()
                .flatten()
                .map(|file| file.publication_id)
                .collect();
            assert!(
                snapshot
                    .records
                    .values()
                    .all(|record| file_ids.contains(&record.publication_id))
            );
        }
        writer.join().unwrap();
    }
}
