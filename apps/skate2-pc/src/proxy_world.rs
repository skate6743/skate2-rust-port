//! The live city's PROXY world: one `proxy_world_manager::ProxyWorld` shared
//! by the world stream (`AddHighCollection`) and the city renderer (high
//! collection registration), with the loaded proxy geometry it registers.
//!
//! Port order differs from `GameModule::OneOffLoad` only in when data
//! arrives: the manager reads the PROXY XSS before the world stream starts,
//! as the original does, but the collections load when the renderer exists
//! (texture uploads need it), after the initial high collections. Activation
//! still precedes every high registration the port applies, and
//! `ProxyWorld` settles a mapping that arrives after its high collection
//! registered into the state the original reaches.
use crate::{
    gpu_renderer::RenderResult,
    proxy_texture_world::{ProxyCollection, ProxyGeometry},
};
use skate2_core::{
    game_stream::{StreamKind, StreamPaths},
    presentation::PresentationCollection,
    proxy_world_manager::{ProxyWorld, ProxyWorldError, ProxyWorldManager},
    world_stream_runtime::WorldStreamEffects,
};
use skate2_formats::{mount::MountSet, stream::CollectionSpace};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

pub(crate) type SharedProxyWorld = Arc<Mutex<ProxyWorldOwner>>;

pub(crate) struct ProxyWorldOwner {
    world: ProxyWorld,
    collections: BTreeMap<u64, ProxyCollection>,
    /// The first registration fault; the original asserts there.
    fault: Option<ProxyWorldError>,
}

impl ProxyWorldOwner {
    /// `SyncLoadProxyWorldStream` 0x824df648's order and proxy map, from the
    /// PROXY Presentation stream's XSS (`AssetPaths::BuildStreamPath` of
    /// `data\content\world\stream` and `PROXY`).
    pub(crate) fn load_shared(mounts: &MountSet) -> RenderResult<SharedProxyWorld> {
        Self::load_shared_stream(mounts, "PROXY")
    }

    /// [`Self::load_shared`] of another proxy stream (PC port: a linked Skate 3 data folder's
    /// district's, `<District>_100_Proxy`, `skate2_core::skate3`).
    pub(crate) fn load_shared_stream(mounts: &MountSet, stream: &str) -> RenderResult<SharedProxyWorld> {
        let paths = StreamPaths::derive(
            "data\\content\\world\\stream",
            stream,
            StreamKind::Presentation,
        )?;
        let space = CollectionSpace::parse(&mounts.read_path(&paths.collection_space)?)?;
        // SKATE2_TRACE_DISTRICT_PROXY: each proxy's hull on the ground, to set against the high
        // tiles that hide it.
        if std::env::var_os("SKATE2_TRACE_DISTRICT_PROXY").is_some() {
            for row in &space.collections {
                eprintln!(
                    "{stream} proxy {:016x}: x {:.0}..{:.0} z {:.0}..{:.0}",
                    row.id, row.hull_min[0], row.hull_max[0], row.hull_min[2], row.hull_max[2]
                );
            }
        }
        let manager = ProxyWorldManager::from_collection_space(&space);
        eprintln!(
            "{stream} world: {} collections in load order",
            manager.collection_ids().len()
        );
        Ok(Arc::new(Mutex::new(Self {
            world: ProxyWorld::new(manager),
            collections: BTreeMap::new(),
            fault: None,
        })))
    }

    /// `mProxyCollectionID`.
    pub(crate) fn collection_ids(&self) -> Vec<u64> {
        self.world.manager().collection_ids().to_vec()
    }

    /// `SyncActivateProxyWorld` 0x824d9ef8 over the loaded collections.
    pub(crate) fn activate(&mut self, geometry: ProxyGeometry) -> RenderResult<()> {
        let result = self.world.activate(geometry.containers);
        self.collections = geometry.collections;
        self.record(result)
    }

    pub(crate) fn high_registered(&mut self, high: u64) -> RenderResult<()> {
        let result = self.world.high_registered(high);
        self.record(result)
    }

    /// `AddHighCollection` for a high collection outside the world stream's own effects (PC port:
    /// a Skate 3 district's, named on its tile `c<kind>_<x>_<z>_high`).
    pub(crate) fn add_high_collection(&mut self, high: u64, x: i32, z: i32) -> RenderResult<()> {
        let result = self.world.add_high_collection(high, u32::from_ne_bytes(x.to_ne_bytes()), u32::from_ne_bytes(z.to_ne_bytes()));
        self.record(result)
    }

    /// The proxy a high collection hides (`GetProxyCollection`), for diagnostics.
    pub(crate) fn proxy_of(&self, high: u64) -> Option<u64> {
        self.world.manager().proxy_collection(high)
    }

    pub(crate) fn high_deregistered(&mut self, high: u64) -> RenderResult<()> {
        let result = self.world.high_deregistered(high);
        self.record(result)
    }

    /// Advances with every proxy registration change.
    pub(crate) fn revision(&self) -> RenderResult<u64> {
        match self.fault {
            Some(fault) => Err(fault.into()),
            None => Ok(self.world.revision()),
        }
    }

    /// The proxies whose arenas are registered (`RegArena` ran), in
    /// `mProxyCollectionID` order. A collection only partly registered keeps
    /// its registered arenas.
    pub(crate) fn registered_collections(&self) -> Vec<Arc<PresentationCollection>> {
        let mut registered = Vec::new();
        for id in self.world.manager().collection_ids() {
            let Some(proxy) = self.collections.get(id) else {
                continue;
            };
            let arenas = proxy
                .drawn_arenas
                .iter()
                .map(|&arena| self.world.is_arena_registered(arena))
                .collect::<Vec<_>>();
            if arenas.iter().all(|&arena| arena) {
                registered.push(Arc::clone(&proxy.presentation));
            } else if arenas.iter().any(|&arena| arena) {
                let mut partial = (*proxy.presentation).clone();
                partial.arenas = partial
                    .arenas
                    .into_iter()
                    .zip(arenas)
                    .filter_map(|(arena, kept)| kept.then_some(arena))
                    .collect();
                registered.push(Arc::new(partial));
            }
        }
        registered
    }

    fn record(&mut self, result: Result<(), ProxyWorldError>) -> RenderResult<()> {
        if let Err(error) = result {
            if self.fault.is_none() {
                self.fault = Some(error);
            }
            return Err(error.into());
        }
        Ok(())
    }
}

/// The world stream's `AddHighCollection` (`cStreamFile::Update` 0x824e0ba8)
/// into the shared proxy manager.
pub(crate) struct ProxyHighCollections(pub(crate) SharedProxyWorld);

impl WorldStreamEffects for ProxyHighCollections {
    fn add_high_collection(&mut self, id: u64, x: u32, z: u32) {
        eprintln!("controlled world stream: high collection {id:016x} at ({x},{z})");
        let Ok(mut owner) = self.0.lock() else {
            eprintln!("PROXY world lock poisoned; AddHighCollection {id:016x} lost");
            return;
        };
        let result = owner.world.add_high_collection(id, x, z);
        // A failure latches in the owner; the renderer's next revision read
        // stops the city with it.
        let _ = owner.record(result);
    }

    fn finalize_world_bounds(&mut self) {
        eprintln!("controlled world stream: world bounds finalized");
    }
}
