//! Retained selected collection load through the existing catalog and streamer.
//! The caller supplies an accepted catalog and chooses the collection. This
//! does not implement spatial focus scheduling or normal-arena registration.
use crate::{
    catalog_resident_load_factory::{
        CatalogResidentCollectionLoadFactory, CatalogResidentLoadError,
        CollectionRequestIdAllocator, CollectionRequestIdLease,
    },
    collection_resident_streamer::{
        CollectionId, CollectionResidentStreamer, ResidentAssetGarbageCollector,
        ResidentCallbackContext, ResidentCallbackObservation, ResidentCollectionCallback,
        ResidentCollectionHold, ResidentCollectionMetadata,
    },
    owned_resident_collection::{OwnedResidentCollectionError, OwnedResidentCollectionStorage},
    resident_collection_load_driver::CollectionDecompressionWake,
    selected_simulation_asset_services::SelectedSimulationAssetServices,
    simulation_collision_asset::SimulationCollisionAssetSessionError,
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_resident_collection::{
        SimulationResidentArenaAsset, SimulationResidentArenaAssetBuilderSessions,
        SimulationResidentArenaBuildError,
    },
    simulation_resident_registry::{
        SimulationResidentLookup, SimulationResidentRegistry, SimulationResidentRegistryError,
    },
    world_stream_catalog::{CollectionRegistrationSnapshot, WorldStreamCatalog},
};
use skate2_formats::mount::{MountError, MountSet};
use std::{
    error::Error,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

/// Host serialization of one live request identity, not native allocation parity.
struct RequestIds(Arc<AtomicBool>);
struct RequestLease(Arc<AtomicBool>);
#[derive(Debug)]
struct RequestBusy;
impl fmt::Display for RequestBusy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("selected collection request identity is still leased")
    }
}
impl Error for RequestBusy {}
impl CollectionRequestIdLease for RequestLease {
    fn raw(&self) -> i32 {
        0
    }
}
impl Drop for RequestLease {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl CollectionRequestIdAllocator for RequestIds {
    type Lease = RequestLease;
    type Error = RequestBusy;
    fn acquire(&mut self) -> Result<RequestLease, RequestBusy> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| RequestBusy)?;
        Ok(RequestLease(Arc::clone(&self.0)))
    }
}

type Cause = CatalogResidentLoadError<
    MountError,
    RequestBusy,
    SimulationCollisionAssetSessionError,
    SimulationResidentArenaBuildError,
>;
type LoadError = Arc<Cause>;
type Storage = OwnedResidentCollectionStorage<
    SimulationResidentArenaAsset,
    fn(OwnedResidentCollectionError) -> LoadError,
    LoadError,
>;
type Registry = SimulationResidentRegistry<
    SimulationResidentArenaAsset,
    fn(SimulationResidentRegistryError) -> LoadError,
    LoadError,
>;
type Factory = CatalogResidentCollectionLoadFactory<
    MountSet,
    RequestIds,
    SimulationResidentArenaAssetBuilderSessions,
>;
type Streamer = CollectionResidentStreamer<
    Storage,
    Metadata,
    Factory,
    Registry,
    SelectedGarbage,
    CollectionDecompressionWake,
    KeepLoaded,
    LoadError,
>;

#[derive(Debug)]
pub(crate) enum SelectedResidencyError {
    Host(&'static str),
    Source(Arc<dyn Error + Send + Sync>),
}
impl fmt::Display for SelectedResidencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(message) => f.write_str(message),
            Self::Source(error) => error.fmt(f),
        }
    }
}
impl Error for SelectedResidencyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Host(_) => None,
            Self::Source(error) => Some(error.as_ref()),
        }
    }
}
fn host(message: &'static str) -> SelectedResidencyError {
    SelectedResidencyError::Host(message)
}
fn report(error: impl Error + Send + Sync + 'static) -> SelectedResidencyError {
    SelectedResidencyError::Source(Arc::new(error))
}
fn external(error: impl Error + Send + Sync + 'static) -> LoadError {
    Arc::new(Cause::external(Arc::new(error)))
}
fn storage_error(error: OwnedResidentCollectionError) -> LoadError {
    external(error)
}
fn registry_error(error: SimulationResidentRegistryError) -> LoadError {
    external(error)
}

struct Metadata(WorldStreamCatalog);
impl ResidentCollectionMetadata<LoadError> for Metadata {
    type Descriptor = CollectionRegistrationSnapshot;
    fn find_collection(&mut self, id: CollectionId) -> Result<Option<Self::Descriptor>, LoadError> {
        self.0.collection(id.0).map_err(external)
    }
}

#[derive(Default)]
struct GarbageQueue {
    tick: u32,
    entries: std::collections::VecDeque<(u32, Arc<SimulationNormalArenaPlan>)>,
}
impl GarbageQueue {
    fn advance(
        &mut self,
        assets: &SelectedSimulationAssetServices,
    ) -> Result<(), SelectedResidencyError> {
        self.tick = self.tick.wrapping_add(1);
        for _ in 0..20 {
            if !self
                .entries
                .front()
                .is_some_and(|(deadline, _)| *deadline < self.tick)
            {
                break;
            }
            let (_, plan) = self.entries.front().expect("matured entry");
            assets
                .try_release_plan(plan)
                .map_err(|_| host("processed arena release failed"))?;
            self.entries.pop_front();
        }
        Ok(())
    }
}
struct SelectedGarbage {
    lookup: SimulationResidentLookup<SimulationResidentArenaAsset>,
    queue: Arc<Mutex<GarbageQueue>>,
}
impl ResidentAssetGarbageCollector<LoadError> for SelectedGarbage {
    fn garbage_collect(&mut self, id: CollectionId) -> Result<(), LoadError> {
        let plan = self
            .lookup
            .arena_lookup()
            .try_with_asset(id, |asset| {
                Ok::<_, SelectedResidencyError>(asset.plan_owner())
            })
            .map_err(external)?
            .ok_or_else(|| external(host("garbage arena is not published")))?;
        let mut queue = self
            .queue
            .lock()
            .map_err(|_| external(host("garbage queue lock")))?;
        // GarbageCollectAsset82d557c8: capacity9000, releaseTick +3.
        if queue.entries.len() >= 9000 {
            return Err(external(host("garbage queue capacity exceeded")));
        }
        let deadline = queue.tick.wrapping_add(3);
        queue.entries.push_back((deadline, plan));
        Ok(())
    }
}
struct KeepLoaded {
    expected: CollectionId,
    hold: Arc<Mutex<Option<ResidentCollectionHold>>>,
}
impl ResidentCollectionCallback<LoadError> for KeepLoaded {
    fn on_complete(
        &mut self,
        id: CollectionId,
        _: ResidentCallbackObservation,
        context: &mut ResidentCallbackContext<'_, Self>,
    ) -> Result<(), LoadError> {
        if id != self.expected || context.active_collection_id() != self.expected {
            return Err(external(host(
                "selected load callback belongs to another collection",
            )));
        }
        let mut slot = self
            .hold
            .lock()
            .map_err(|_| external(host("selected resident hold lock poisoned")))?;
        if slot.is_some() {
            return Err(external(host("selected resident hold already acquired")));
        }
        let (_, hold) = context.acquire_active_collection_hold().map_err(external)?;
        *slot = Some(hold);
        Ok(())
    }
}

pub(crate) struct SelectedResidentStartup {
    catalog: WorldStreamCatalog,
    streamer: Streamer,
    lookup: SimulationResidentLookup<SimulationResidentArenaAsset>,
    hold: Arc<Mutex<Option<ResidentCollectionHold>>>,
    assets: Arc<SelectedSimulationAssetServices>,
    garbage: Arc<Mutex<GarbageQueue>>,
    retained_holds: Vec<Arc<Mutex<Option<ResidentCollectionHold>>>>,
    requested: Vec<CollectionId>,
    selected: Option<CollectionId>,
    plans: Option<Vec<Arc<SimulationNormalArenaPlan>>>,
    failed: bool,
}
impl SelectedResidentStartup {
    pub(crate) fn active_loaded_size(&self,id:CollectionId)->u32 {self.streamer.active_loaded_size(id)}
    pub(crate) fn new(mounts: Arc<MountSet>, catalog: WorldStreamCatalog) -> Self {
        let factory = Factory::new(
            mounts,
            catalog.clone(),
            RequestIds(Arc::new(AtomicBool::new(false))),
            SimulationResidentArenaAssetBuilderSessions::new(catalog.clone()),
        );
        let wake = factory.decompression_kick();
        let (mut registry, lookup) = Registry::new(registry_error);
        let assets = SelectedSimulationAssetServices::construct_resident(&mut registry);
        let garbage = Arc::new(Mutex::new(GarbageQueue::default()));
        let streamer = Streamer::new(
            Storage::new(storage_error),
            Metadata(catalog.clone()),
            factory,
            registry,
            SelectedGarbage {
                lookup: lookup.clone(),
                queue: Arc::clone(&garbage),
            },
            wake,
        );
        Self {
            catalog,
            streamer,
            lookup,
            hold: Arc::new(Mutex::new(None)),
            assets,
            garbage,
            retained_holds: Vec::new(),
            requested: Vec::new(),
            selected: None,
            plans: None,
            failed: false,
        }
    }
    /// One explicit host-selected collection; no probe ordinal, deadline, or
    /// deferred-registration policy is substituted for runtime scheduling.
    pub(crate) fn request(&mut self, id: CollectionId) -> Result<(), SelectedResidencyError> {
        if self.selected.is_some() || self.failed {
            return Err(host("selected residency already attempted"));
        }
        self.failed = true;
        self.selected = Some(id);
        self.requested.push(id);
        let collection = self
            .catalog
            .collection(id.0)
            .map_err(report)?
            .ok_or_else(|| host("selected collection is absent from accepted catalog"))?;
        self.streamer.enqueue_load(
            id,
            i32::from_ne_bytes(collection.asset_count.to_ne_bytes()),
            Some(KeepLoaded {
                expected: id,
                hold: Arc::clone(&self.hold),
            }),
        );
        self.failed = false;
        Ok(())
    }
    pub(crate) fn poll(&mut self) -> Result<bool, SelectedResidencyError> {
        if self.failed {
            return Err(host("selected residency has a retained failure"));
        }
        if self.plans.is_some() {
            return Ok(true);
        }
        let id = self
            .selected
            .ok_or_else(|| host("selected residency has not been requested"))?;
        self.failed = true;
        self.streamer.update().map_err(report)?;
        let ready = self
            .hold
            .lock()
            .map_err(|_| host("selected resident hold lock poisoned"))?
            .is_some();
        if ready {
            let occurrence = self
                .lookup
                .lookup(id)
                .map_err(report)?
                .ok_or_else(|| host("loaded selected collection is not published"))?;
            let mut plans = Vec::new();
            occurrence
                .try_for_each(|asset| {
                    plans
                        .try_reserve(1)
                        .map_err(|_| host("selected plan owner allocation failed"))?;
                    plans.push(asset.plan_owner());
                    Ok::<_, SelectedResidencyError>(())
                })
                .map_err(report)?;
            self.plans = Some(plans);
        }
        self.failed = false;
        Ok(ready)
    }
    pub(crate) fn plans(&self) -> Option<&[Arc<SimulationNormalArenaPlan>]> {
        self.plans.as_deref()
    }
    /// Continue the same streamer/registry namespace with one additional load.
    /// Completed collections stay held until their explicit deregistration.
    pub(crate) fn request_additional(
        &mut self,
        id: CollectionId,
    ) -> Result<(), SelectedResidencyError> {
        if self.failed || (self.selected.is_some() && self.plans.is_none()) {
            return Err(host(
                "additional terrain request before previous admission completed",
            ));
        }
        if self.requested.contains(&id) {
            return Err(host("terrain collection already requested"));
        }
        if self
            .hold
            .lock()
            .map_err(|_| host("resident hold lock"))?
            .is_some()
        {
            self.retained_holds.push(Arc::clone(&self.hold));
        }
        self.hold = Arc::new(Mutex::new(None));
        self.plans = None;
        self.selected = None;
        self.request(id)
    }
    pub(crate) fn was_requested(&self, id: CollectionId) -> bool {
        self.requested.contains(&id)
    }
    /// The collection's parent (`mParentCollectionMap`, from its XSM entry) when that is a
    /// collection of its own, as the global collection holding an additional world's arenas is
    /// for its empty cells.
    pub(crate) fn parent(&self, id: CollectionId) -> Result<Option<CollectionId>, SelectedResidencyError> {
        let Some(collection) = self.catalog.collection(id.0).map_err(report)? else { return Ok(None) };
        let parent = CollectionId(collection.parent_id);
        if parent == crate::collection_parent_manager::INVALID_PARENT_COLLECTION_ID {
            return Ok(None);
        }
        Ok(self.catalog.collection(parent.0).map_err(report)?.map(|_| parent))
    }
    pub(crate) fn selected(&self) -> Option<CollectionId> {
        self.selected
    }

    /// Called only after DRegDMOArena and DRegArena finished for this collection.
    /// The queued unload releases the load reference; the occurrence-bound
    /// simulation hold is released last, allowing normal zero-count teardown.
    pub(crate) fn unload(&mut self, id: CollectionId) -> Result<(), SelectedResidencyError> {
        if self.failed {
            return Err(host("resident unload after failure"));
        }
        self.failed = true;
        let mut released = None;
        for slot in std::iter::once(&self.hold).chain(self.retained_holds.iter()) {
            let mut slot = slot.lock().map_err(|_| host("resident hold lock"))?;
            if slot.as_ref().is_some_and(|hold| hold.id() == id) {
                released = slot.take();
                break;
            }
        }
        let hold = released.ok_or_else(|| host("resident unload has no occurrence hold"))?;
        self.streamer.enqueue_unload(id, None);
        self.streamer.update().map_err(report)?;
        self.streamer
            .release_collection_hold(hold)
            .map_err(report)?;
        self.requested.retain(|entry| *entry != id);
        self.retained_holds
            .retain(|slot| slot.lock().map_or(true, |slot| slot.is_some()));
        if self.selected == Some(id) {
            self.plans = None;
            self.selected = None;
        }
        self.failed = false;
        Ok(())
    }

    /// Native UpdateGarbageCollector82d5cc40 drains at most20 matured assets;
    /// equality to the deadline still waits. The selected synchronous host
    /// advances this release clock once per terrain/simulation update.
    pub(crate) fn update_garbage(&mut self) -> Result<(), SelectedResidencyError> {
        let mut queue = self
            .garbage
            .lock()
            .map_err(|_| host("garbage queue lock"))?;
        queue.advance(&self.assets)
    }
    pub(crate) fn assets(&self) -> &Arc<SelectedSimulationAssetServices> {
        &self.assets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn garbage_waits_past_three_ticks_and_releases_twenty_assets_per_update() {
        let assets = SelectedSimulationAssetServices::construct();
        let mut queue = GarbageQueue::default();
        let mut weak = Vec::new();
        for id in 1..=25 {
            let plan = crate::world_painter_layer_asset::tests::fixture(id, 0, 1, Some([64.0; 8]));
            assets.try_process_plan(&plan).unwrap();
            weak.push(Arc::downgrade(&plan));
            queue.entries.push_back((3, plan));
        }
        for _ in 0..3 {
            queue.advance(&assets).unwrap();
            assert_eq!(assets.retained_counts().0, 25);
        }
        queue.advance(&assets).unwrap();
        assert_eq!(assets.retained_counts().0, 5);
        assert!(weak[..20].iter().all(|plan| plan.upgrade().is_none()));
        queue.advance(&assets).unwrap();
        assert_eq!(assets.retained_counts(), (0, 0));
        assert!(weak.iter().all(|plan| plan.upgrade().is_none()));
    }
}
