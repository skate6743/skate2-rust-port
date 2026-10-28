//! Selected terrain admission and moving simulation focus over one retained world.
//!
//! The caller selects a collection after this source's world stream completes
//! and supplies the Environment unlock seed. This owner composes existing
//! resident, normal-arena and world-query implementations. The offline spatial/XML
//! focus and explicit deregistration are connected; unlock-context boot and
//! broader DMO/LivingWorld scheduling remain external.

use crate::{
    collection_resident_streamer::CollectionId,
    game::{WorldStreamPoll, WorldStreamSpec},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_mesh_store::{MeshStore, MeshStoreError},
    physical_player_assembly::PhysicalAssemblyWorld,
    selected_resident_startup::{SelectedResidencyError, SelectedResidentStartup},
    selected_terrain_startup::{SelectedTerrainError, SelectedTerrainStartup},
    selected_world_stream::{Error as SelectedWorldStreamError, SelectedWorldStream},
    simulation_assembly_backend::constraint_session::{
        physical_world_line::PhysicalWorldLineInitError,
        world_line_query::SimulationWorldLineQueryConfig,
    },
    simulation_environment_unlock_router::{
        EnvironmentUnlockRouterCreateError, EnvironmentUnlockSeed,
        SimulationEnvironmentUnlockRouter,
    },
    static_ground_pool::{STATIC_GROUND_NATIVE_CAPACITY, StaticGroundPool},
    world_stream_catalog::WorldStreamCatalog,
    world_stream_runtime::WorldStreamEffects,
};
use skate2_formats::mount::MountSet;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum SelectedSessionTerrainError {
    ForeignWorld,
    WorldLock,
    LivingWorldLock,
    Mesh(MeshStoreError),
    Stream(SelectedWorldStreamError),
    Router(EnvironmentUnlockRouterCreateError),
    Bounds(crate::world_stream_runtime::WorldBoundsError),
    Cvar(InputCvarError),
    Query(PhysicalWorldLineInitError),
    PedestrianQueries(String),
    Residency(SelectedResidencyError),
    Terrain(SelectedTerrainError),
    Failed,
    Pending,
    Focus(&'static str),
    Catalog(crate::world_stream_catalog::CatalogError),
    Hull(String),
}
impl std::fmt::Display for SelectedSessionTerrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignWorld => f.write_str("selected terrain belongs to another world"),
            Self::WorldLock => f.write_str("selected terrain world lock poisoned"),
            Self::LivingWorldLock => f.write_str("selected terrain LivingWorld lock poisoned"),
            Self::Mesh(e) => write!(f, "selected terrain mesh owner: {e}"),
            Self::Stream(e) => write!(f, "selected terrain stream: {e}"),
            Self::Router(e) => write!(f, "selected terrain router: {e}"),
            Self::Bounds(e) => write!(f, "selected terrain world bounds: {e}"),
            Self::Cvar(e) => write!(f, "selected terrain query cvar: {e}"),
            Self::Query(e) => write!(f, "selected terrain query owner: {e}"),
            Self::PedestrianQueries(e) => write!(f, "pedestrian execution query initialization: {e}"),
            Self::Residency(e) => write!(f, "selected terrain residency: {e}"),
            Self::Terrain(e) => write!(f, "selected terrain registration: {e}"),
            Self::Failed => f.write_str("selected terrain has a retained failure"),
            Self::Pending => f.write_str("selected terrain is not yet registered"),
            Self::Focus(message) => write!(f, "selected terrain focus: {message}"),
            Self::Catalog(error) => write!(f, "selected terrain focus catalog: {error}"),
            Self::Hull(error) => write!(f, "selected challenge hull: {error}"),
        }
    }
}

/// Caller-owned provenance for the one world, one mount set and one stream.
/// The same `MountSet` Arc is passed to stream startup and later resident load;
/// the router is constructed here for that world from the supplied raw seed.
pub(crate) struct SelectedTerrainSource<Effects: WorldStreamEffects> {
    world: Arc<Mutex<PhysicalAssemblyWorld>>,
    mounts: Arc<MountSet>,
    cvars: InputCvarReferences,
    stream: SelectedWorldStream<Effects>,
    environment: Option<SimulationEnvironmentUnlockRouter>,
}
impl<Effects: WorldStreamEffects> SelectedTerrainSource<Effects> {
    pub(crate) fn try_new(
        world: Arc<Mutex<PhysicalAssemblyWorld>>,
        mounts: Arc<MountSet>,
        catalog: WorldStreamCatalog,
        cvars: InputCvarReferences,
        spec: WorldStreamSpec,
        effects: Effects,
        unlock_seed: EnvironmentUnlockSeed,
    ) -> Result<Self, SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        let mut environment = SimulationEnvironmentUnlockRouter::try_new_with_external_seed(
            unlock_seed,
            STATIC_GROUND_NATIVE_CAPACITY,
        )
        .map_err(E::Router)?;
        environment.warm_ground_decodes();
        let stream = SelectedWorldStream::try_new(
            Arc::clone(&mounts),
            catalog,
            cvars.clone(),
            spec,
            effects,
        )
        .map_err(E::Stream)?;
        Ok(Self {
            world,
            mounts,
            cvars,
            stream,
            environment: Some(environment),
        })
    }
    pub(crate) fn try_poll_stream(
        &mut self,
    ) -> Result<WorldStreamPoll, SelectedSessionTerrainError> {
        self.stream
            .try_poll()
            .map_err(SelectedSessionTerrainError::Stream)
    }
    pub(crate) fn stream(&self) -> &SelectedWorldStream<Effects> {
        &self.stream
    }
}

/// Owns the resident decode exactly once and retains every registration token.
/// The world handle is the same occurrence used for the local skater and queries.
#[expect(
    clippy::struct_excessive_bools,
    reason = "distinct monotonic request, residency, failure and completion gates retain partial owners"
)]
pub(crate) struct SelectedSessionTerrain<Effects: WorldStreamEffects> {
    source: SelectedTerrainSource<Effects>,
    collection: CollectionId,
    dmo_update_in_parallel: bool,
    resident: Option<SelectedResidentStartup>,
    terrain: Option<SelectedTerrainStartup>,
    mesh: Option<Arc<MeshStore>>,
    grind: Option<Arc<Mutex<crate::grind_data_runtime::GrindDataRuntime>>>,
    userdata: u32,
    request_attempted: bool,
    request_complete: bool,
    failed: bool,
    complete: bool,
    focus_space: Option<Arc<skate2_formats::stream::CollectionSpace>>,
    focus_xml: Option<skate2_formats::stream_xml::StreamXml>,
    focus: Option<crate::streaming::StreamFocusState>,
    /// An online game's simulation area: no streamer controller moves the focus
    /// (`CreateSimStreamerController` 0x8277c080 makes none for a network game).
    area: Option<OnlineSimArea>,
    /// The challenge asset manager's hull loader, made at its first hull.
    hulls: Option<crate::selected_hull_loader::SelectedHullLoader>,
    /// The dynamic hull manager's own hull loader (`cChallengeDynamicHullManager::HullLoader`),
    /// made at its first hull.
    dynamic_hulls: Option<crate::selected_hull_loader::SelectedHullLoader>,
    /// `GameModule::mAdditionalStreamHandle`'s world.
    additional: Option<additional::AdditionalWorld>,
}

/// `GameModule::GameLoad`'s network branch: the simulation focus locked at the game's challenge
/// location (`Net::GetStreamerPosition` 0x82784318 for player -1) with the challenge's own stream
/// XML (`LoadSimStreamXml` 0x824d8730, `data\stream\<challenges.Name>.xml`), and online mode
/// lifting the simulation's collection cap (`cAssetStreamSystem::FindCollections` 0x824dfea8).
struct OnlineSimArea {
    position: [f32; 3],
    /// The XML's first tile; none without a usable XML, leaving the spatial selection there.
    tile: Option<Vec<usize>>,
}

impl<Effects: WorldStreamEffects> SelectedSessionTerrain<Effects> {
    /// Retain the caller's world-stream source before the first fallible
    /// request step. The catalog comes from this same completed stream.
    /// DMO parallel and arena userdata remain supplied until their source
    /// producers are connected; neither gets a fixture default here.
    pub(crate) fn new(
        source: SelectedTerrainSource<Effects>,
        collection: CollectionId,
        dmo_update_in_parallel: bool,
        userdata: u32,
    ) -> Self {
        Self {
            source,
            collection,
            dmo_update_in_parallel,
            resident: None,
            terrain: None,
            mesh: None,
            grind: None,
            userdata,
            request_attempted: false,
            request_complete: false,
            failed: false,
            complete: false,
            focus_space: None,
            focus_xml: None,
            focus: None,
            area: None,
            hulls: None,
            dynamic_hulls: None,
            additional: None,
        }
    }

    fn hull_loader(&mut self) -> Result<&mut crate::selected_hull_loader::SelectedHullLoader, SelectedSessionTerrainError> {
        if self.hulls.is_none() {
            let loader = crate::selected_hull_loader::SelectedHullLoader::try_new(
                Arc::clone(&self.source.mounts),
                self.source.stream.catalog.clone(),
            )
            .map_err(SelectedSessionTerrainError::Hull)?;
            self.hulls = Some(loader);
        }
        Ok(self.hulls.as_mut().expect("hull loader made above"))
    }

    /// `cPathedHullLoader::LoadHull` of the challenge asset manager, whose
    /// callback answers every hull.
    pub(crate) fn load_hull(&mut self, file_name: &str, high_priority: bool, asset_flags: u32) -> Result<u32, SelectedSessionTerrainError> {
        Ok(self.hull_loader()?.load_hull(file_name, high_priority, asset_flags, true))
    }

    fn dynamic_hull_loader(&mut self) -> Result<&mut crate::selected_hull_loader::SelectedHullLoader, SelectedSessionTerrainError> {
        if self.dynamic_hulls.is_none() {
            let loader = crate::selected_hull_loader::SelectedHullLoader::try_new(
                Arc::clone(&self.source.mounts),
                self.source.stream.catalog.clone(),
            )
            .map_err(SelectedSessionTerrainError::Hull)?;
            self.dynamic_hulls = Some(loader);
        }
        Ok(self.dynamic_hulls.as_mut().expect("dynamic hull loader made above"))
    }

    /// `cPathedHullLoader::LoadHull` of the dynamic hull manager (registered on
    /// load, normal priority).
    pub(crate) fn load_dynamic_hull(&mut self, file_name: &str, asset_flags: u32, callback: bool) -> Result<u32, SelectedSessionTerrainError> {
        Ok(self.dynamic_hull_loader()?.load_hull(file_name, false, asset_flags, callback))
    }

    /// `cPathedHullLoader::UnloadHull` of the dynamic hull manager.
    pub(crate) fn unload_dynamic_hull(&mut self, hull: u32) -> Result<(), SelectedSessionTerrainError> {
        self.dynamic_hull_loader()?.unload_hull(hull, false);
        Ok(())
    }

    /// `cPathedHullLoader::Update` of the dynamic hull manager's loader: the
    /// hulls whose load callback fires, in order.
    pub(crate) fn try_update_dynamic_hulls(&mut self) -> Result<Vec<u32>, SelectedSessionTerrainError> {
        let Some(hulls) = self.dynamic_hulls.as_mut() else { return Ok(Vec::new()) };
        let terrain = self.terrain.as_mut().ok_or(SelectedSessionTerrainError::Pending)?;
        hulls.try_update(terrain).map_err(SelectedSessionTerrainError::Hull)
    }

    /// `cChallengeDynamicHullManager::IsLoading` (`0x82da16b0`).
    pub(crate) fn dynamic_hulls_loading(&self) -> bool {
        self.dynamic_hulls.as_ref().is_some_and(crate::selected_hull_loader::SelectedHullLoader::is_loading)
    }

    /// The challenge asset manager's loader holds no hull.
    pub(crate) fn hulls_unloaded(&self) -> bool {
        self.hulls.as_ref().is_none_or(crate::selected_hull_loader::SelectedHullLoader::is_all_unloaded)
    }

    /// Whether the challenge asset manager's loader holds the hull.
    pub(crate) fn holds_hull(&self, hull: u32) -> bool {
        self.hulls.as_ref().is_some_and(|hulls| hulls.holds_hull(hull))
    }

    /// `cChallengeDynamicHullManager::IsAllUnloaded` (`0x82da1680`).
    pub(crate) fn dynamic_hulls_unloaded(&self) -> bool {
        self.dynamic_hulls.as_ref().is_none_or(crate::selected_hull_loader::SelectedHullLoader::is_all_unloaded)
    }

    /// Both hull loaders' presentation collections, the asset manager's first.
    pub(crate) fn hull_presentations(&self) -> Vec<crate::presentation::HullPresentation> {
        self.hulls.iter().chain(&self.dynamic_hulls).flat_map(crate::selected_hull_loader::SelectedHullLoader::presentations).collect()
    }

    /// `cPathedHullLoader::UnloadHull` of the challenge asset manager.
    pub(crate) fn unload_hull(&mut self, hull: u32) -> Result<(), SelectedSessionTerrainError> {
        self.hull_loader()?.unload_hull(hull, true);
        Ok(())
    }

    /// `cPathedHullLoader::Update` (`0x82eee718`): the hulls whose
    /// `cHullLoadRequestCallback::OnComplete` fires, in order.
    pub(crate) fn try_update_hulls(&mut self) -> Result<Vec<u32>, SelectedSessionTerrainError> {
        let Some(hulls) = self.hulls.as_mut() else { return Ok(Vec::new()) };
        let terrain = self.terrain.as_mut().ok_or(SelectedSessionTerrainError::Pending)?;
        hulls.try_update(terrain).map_err(SelectedSessionTerrainError::Hull)
    }

    /// Continue the retained source when this owner was constructed before
    /// world bounds became available. No stream effects may run after a
    /// resident request has begun.
    pub(crate) fn try_poll_stream(
        &mut self,
    ) -> Result<WorldStreamPoll, SelectedSessionTerrainError> {
        if self.failed || self.request_attempted {
            return Err(SelectedSessionTerrainError::Failed);
        }
        let result = self.source.try_poll_stream();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    /// Start exactly one resident request. Pending world bounds have made no
    /// writes and may be checked again; every subsequent failure ends this
    /// attempt while this owner retains its source and any completed prefix.
    pub(crate) fn try_request(&mut self) -> Result<(), SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        if self.failed || self.request_attempted {
            return Err(E::Failed);
        }
        match self.source.stream.bounds.read() {
            Ok(_) => {}
            Err(crate::world_stream_runtime::WorldBoundsError::Pending) => {
                return Err(E::Bounds(
                    crate::world_stream_runtime::WorldBoundsError::Pending,
                ));
            }
            Err(error) => {
                self.failed = true;
                return Err(E::Bounds(error));
            }
        }
        self.request_attempted = true;
        let result = self.try_request_after_bounds();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn try_request_after_bounds(&mut self) -> Result<(), SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        let collection = self
            .source
            .stream
            .catalog
            .collection(self.collection.0)
            .map_err(E::Catalog)?
            .ok_or(E::Focus("initial collection missing"))?;
        let file = self
            .source
            .stream
            .catalog
            .file(&collection.owner)
            .map_err(E::Catalog)?
            .ok_or(E::Focus("simulation stream missing"))?;
        if file.kind != crate::game_stream::StreamKind::Simulation {
            return Err(E::Focus("terrain collection is not from simulation stream"));
        }
        self.focus_space = Some(
            file.collection_space
                .ok_or(E::Focus("simulation XSS missing"))?,
        );
        self.focus_xml = file
            .stream_xml
            .as_ref()
            .filter(|bytes| !bytes.is_empty())
            .map(|bytes| {
                skate2_formats::stream_xml::StreamXml::parse(
                    bytes,
                    file.kind.suffix(),
                    self.focus_space.as_ref().expect("space retained above"),
                )
            })
            .transpose()
            .map_err(E::Focus)?;
        let (mesh, grind) = {
            let mut selected = self.source.world.lock().map_err(|_| E::WorldLock)?;
            // World::World constructs ground/query owners before streamed
            // normal arenas call Simulation::RegArena.
            let config = SimulationWorldLineQueryConfig::try_sample_selected(
                &self.source.cvars,
                selected.island_flags(),
            )
            .map_err(E::Cvar)?;
            selected
                .try_initialize_world_line_queries(config, self.dmo_update_in_parallel)
                .map_err(E::Query)?;
            let living_owner = Arc::clone(selected.grind_services.living_world());
            let mut living_world = living_owner
                .lock()
                .map_err(|_| E::LivingWorldLock)?;
            // Native execution Init82e9dc50 leases its persistent batches from
            // this same PhysicsWorld. Query construction must precede leases.
            let (queries, dmos) = selected.world_line_owners();
            living_world.pedestrian_queries.as_mut()
                .ok_or_else(|| E::PedestrianQueries("execution owner absent".into()))?
                .initialize(
                    queries.ok_or_else(|| E::PedestrianQueries("world queries absent".into()))?,
                    dmos.ok_or_else(|| E::PedestrianQueries("dynamic pool absent".into()))?,
                ).map_err(E::PedestrianQueries)?;
            let mesh = living_world.try_mesh_store().map_err(E::Mesh)?;
            (mesh, Arc::clone(selected.grind_services.static_data()))
        };
        self.mesh = Some(mesh);
        self.grind = Some(grind);
        self.resident = Some(SelectedResidentStartup::new(
            Arc::clone(&self.source.mounts),
            self.source.stream.catalog.clone(),
        ));
        self.resident
            .as_mut()
            .expect("resident inserted before request")
            .request(self.collection)
            .map_err(E::Residency)?;
        self.request_complete = true;
        Ok(())
    }

    /// One asynchronous resident update. At ready, move the same loaded plan
    /// owners into the normal adapter and register exactly once. A failed
    /// prefix stays held for diagnosis; no implicit retry or unload occurs.
    pub(crate) fn try_poll(&mut self)->Result<bool,SelectedSessionTerrainError>{self.try_poll_with_props(None)}
    pub(crate) fn try_poll_with_props(&mut self,props:Option<&mut crate::selected_living_world_recipes::props::PropRecipeFrame<'_>>)->Result<bool,SelectedSessionTerrainError>{
        use SelectedSessionTerrainError as E;
        if self.failed {
            return Err(E::Failed);
        }
        if !self.request_complete {
            return Err(E::Pending);
        }
        if self.complete {
            return Ok(true);
        }
        let resident = self.resident.as_mut().ok_or(E::Failed)?;
        let ready = match resident.poll() {
            Ok(ready) => ready,
            Err(error) => {
                self.failed = true;
                return Err(E::Residency(error));
            }
        };
        if !ready {
            return Ok(false);
        }
        let bounds = self.source.stream.bounds.read().map_err(E::Bounds)?;
        let mut triggers = crate::trigger_camera_group::CameraTriggerGroup::construct();
        triggers
            .try_initialize(
                Arc::clone(self.resident.as_ref().ok_or(E::Failed)?.assets()),
                crate::grind_data_octree::OctreeBounds {
                    minimum: bounds.min.map(f32::from_bits),
                    maximum: bounds.max.map(f32::from_bits),
                },
            )
            .map_err(|e| E::Terrain(SelectedTerrainError::Trigger(e)))?;
        let living_world = Arc::clone(
            self.source
                .world
                .lock()
                .map_err(|_| E::WorldLock)?
                .grind_services
                .living_world(),
        );
        let mut terrain = SelectedTerrainStartup::new(
            self.resident.take().ok_or(E::Failed)?,
            self.source.environment.take().ok_or(E::Failed)?,
            Arc::clone(self.mesh.as_ref().ok_or(E::Failed)?),
            Arc::clone(self.grind.as_ref().ok_or(E::Failed)?),
            Arc::new(Mutex::new(triggers)),
            Some(crate::selected_terrain_startup::dmo::SelectedDmoRoute {
                physical_world:Arc::clone(&self.source.world),
                living_world,
                cvars: self.source.cvars.clone(),
            }),
        );
        let result = terrain.try_register_with_props(self.userdata,props);
        self.terrain = Some(terrain);
        if let Err(error) = result {
            self.failed = true;
            return Err(E::Terrain(error));
        }
        self.complete = true;
        Ok(true)
    }

    /// Give the caller only the pool admitted for this exact world handle.
    pub(crate) fn ground_pool_for(
        &self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
    ) -> Result<&StaticGroundPool, SelectedSessionTerrainError> {
        if !Arc::ptr_eq(&self.source.world, world) {
            return Err(SelectedSessionTerrainError::ForeignWorld);
        }
        if !self.complete {
            return Err(SelectedSessionTerrainError::Pending);
        }
        Ok(self
            .terrain
            .as_ref()
            .expect("completed terrain retains normal adapter")
            .adapter
            .environment()
            .ground_pool())
    }
    pub(crate) fn terrain_for(
        &self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
    ) -> Result<&SelectedTerrainStartup, SelectedSessionTerrainError> {
        self.ground_pool_for(world)?;
        Ok(self.terrain.as_ref().expect("completed terrain retained"))
    }
    /// Borrow the same MeshStore captured from the constructed LivingWorld
    /// manager before this world's selected terrain was registered.
    pub(crate) fn mesh_store_for(
        &self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
    ) -> Result<&Arc<MeshStore>, SelectedSessionTerrainError> {
        self.ground_pool_for(world)?;
        Ok(self
            .mesh
            .as_ref()
            .expect("completed terrain retained mesh store"))
    }
    pub(crate) fn stream(&self) -> &SelectedWorldStream<Effects> {
        &self.source.stream
    }

    /// Once at the host's simulation frame boundary, independent of focus
    /// movement (teleports may move the focus a second time in that frame).
    pub(crate) fn try_advance_asset_release(&mut self) -> Result<(), SelectedSessionTerrainError> {
        if self.failed {
            return Err(SelectedSessionTerrainError::Failed);
        }
        let result = self
            .terrain
            .as_mut()
            .ok_or(SelectedSessionTerrainError::Pending)?
            .resident
            .update_garbage()
            .map_err(SelectedSessionTerrainError::Residency);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    /// Offline Actor-position focus (Sim_LocalPlayerStreamerController8277d4d0).
    /// Selection reuses the original non-XML XSS path; resident admission is
    /// serialized by the existing loader. Departed collections are deregistered.
    pub(crate) fn try_update_focus(
        &mut self,
        position: [f32; 3],
    ) -> Result<(), SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        if self.failed {
            return Err(E::Failed);
        }
        let result = self.try_update_focus_inner(position);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    /// Selected teleport loading gate: keep the original resident loader and
    /// collision registrations alive while the simulation remains paused.
    pub(crate) fn try_prepare_teleport(&mut self, position: [f32; 3]) -> Result<bool, SelectedSessionTerrainError> {
        self.try_update_focus(position)?;
        let focus = self.focus.as_ref().ok_or(SelectedSessionTerrainError::Focus("teleport focus missing"))?;
        // An online area holds the focus wherever the skater goes.
        if (self.area.is_none() && focus.position() != position) || focus.entries().is_empty() {
            return Err(SelectedSessionTerrainError::Focus("teleport destination has no admitted focus"));
        }
        let ids: Vec<_> = focus.entries().iter().map(|entry| entry.id).collect();
        Ok(self.terrain.as_ref().ok_or(SelectedSessionTerrainError::Pending)?.focus_resident(&ids))
    }

    /// Whether a travel can land at `position`: a number at which the simulation focus selects a
    /// collection (an online area holds the focus wherever the skater goes). Elsewhere the stream
    /// holds nothing, as at another world's location (the training facility's), which the port
    /// does not load.
    pub(crate) fn landable(&self, position: [f32; 3]) -> Result<bool, SelectedSessionTerrainError> {
        if self.area.is_some() {
            return Ok(true);
        }
        if position.iter().any(|value| !value.is_finite()) {
            return Ok(false);
        }
        Ok(!self.focus_selection(position)?.is_empty())
    }

    /// Where the simulation focus is.
    pub(crate) fn focus_position(&self) -> Option<[f32; 3]> {
        self.focus.as_ref().map(crate::streaming::StreamFocusState::position)
    }

    /// Holds the simulation focus on an online game's area, `name` being its challenge's name and
    /// `position` the challenge's location; `None` lets the skater move it again (`BackToSP`
    /// 0x8273da50: `UnloadSimStreamXml`, online mode off).
    pub(crate) fn try_set_online_area(&mut self, area: Option<(&str, [f32; 3])>) -> Result<(), SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        let Some((name, position)) = area else {
            self.area = None;
            return Ok(());
        };
        let tile = match self.source.mounts.read_path(&format!("data\\stream\\{name}.xml")) {
            Ok(bytes) => {
                let space = self.focus_space.as_ref().ok_or(E::Focus("simulation XSS missing"))?;
                skate2_formats::stream_xml::StreamXml::parse(&bytes, "Sim", space)
                    .map_err(E::Focus)?
                    .first_tile()
                    .map(<[usize]>::to_vec)
            }
            Err(_) => None,
        };
        self.area = Some(OnlineSimArea { position, tile });
        Ok(())
    }

    /// `IsStreamFocusLoaded` for the simulation focus.
    pub(crate) fn try_focus_loaded(&self) -> Result<bool, SelectedSessionTerrainError> {
        let ids: Vec<_> = self.focus.as_ref().map(|focus| focus.entries().iter().map(|entry| entry.id).collect()).unwrap_or_default();
        Ok(self.terrain.as_ref().ok_or(SelectedSessionTerrainError::Pending)?.focus_loaded(&ids))
    }

    /// CalcStreamLoadStats824d6798 and IsStreamFocusLoaded824d7a00, sampled
    /// after the simulation focus has reconciled its current selection.
    pub(crate) fn replay_focus_status(&self)->Result<crate::streaming::replay_focus::Status,SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        let terrain=self.terrain.as_ref().ok_or(E::Pending)?;
        let mut result=crate::streaming::replay_focus::Status::default();
        if let Some(focus)=&self.focus {
            for (i,entry) in focus.entries().iter().enumerate() {
                let loaded=terrain.collection_registered(entry.id);
                result.loaded&=loaded;
                let size=self.source.stream.catalog.stream_size(entry.id).map_err(E::Catalog)?
                    .ok_or(E::Focus("replay focus XSM collection absent"))?;
                result.ticks=result.ticks.wrapping_add(crate::streaming::replay_focus::collection_ticks(
                    i,size,loaded,terrain.resident.active_loaded_size(CollectionId(entry.id))));
            }
        }
        Ok(result)
    }

    /// `cAssetStreamSystem::IsAreaLoaded` (`0x824e0340`) for the simulation focus. The stream
    /// system knows a cell collection only when the simulation XSS lists it at level 0
    /// (`cStreamFile::Update` `0x824e0ba8` makes no octree entry for the others), and
    /// `IsCollectionLoaded` takes it as loaded once its registration has completed;
    /// `FindCollections` is this focus's own selection at `position`.
    pub(crate) fn try_area_loaded(&self, position: [f32; 3], radius: f32) -> Result<bool, SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        let space = self.focus_space.as_ref().ok_or(E::Focus("simulation XSS missing"))?;
        let additional = self.additional.as_ref().and_then(additional::AdditionalWorld::selectable);
        let terrain = self.terrain.as_ref().ok_or(E::Pending)?;
        let known = |id| {
            std::iter::once(&**space).chain(additional).any(|space| space.collections.iter().any(|entry| entry.lod == 0 && entry.id == id))
        };
        crate::streaming::area_loaded::is_area_loaded(
            position,
            radius,
            crate::game_stream::StreamKind::Simulation,
            |id| Ok(!known(id) || terrain.collection_registered(id)),
            |at| Ok(self.focus_selection(at)?.iter().map(|collection| collection.id).collect()),
        )
    }

    /// `FindCollections` without a focus stream XML: each Sim file's `cStreamFile::FindCollections`
    /// (`0x824df068`), BAM's first, then a loaded additional world's merged in, nearest first, at
    /// most `cap`. `xml` is BAM's stream XML when `kEnableStreamXml` has its file select through
    /// it; the additional worlds ship none (no `data\stream\<world>_Sim.xml`), so theirs is the
    /// radius query.
    fn spatial_selection(
        &self,
        space: &skate2_formats::stream::CollectionSpace,
        xml: Option<&skate2_formats::stream_xml::StreamXml>,
        position: [f32; 3],
        radius: f32,
        cap: usize,
    ) -> Vec<skate2_formats::stream::FocusCollection> {
        use skate2_formats::stream::{MAX_FOCUS_ENTRIES, merge_focus_collections};
        let first = match xml {
            Some(xml) => xml.focus_collections(space, position, radius, MAX_FOCUS_ENTRIES),
            None => space.focus_collections(position, radius, MAX_FOCUS_ENTRIES),
        };
        let mut selected = match self.additional.as_ref().and_then(additional::AdditionalWorld::selectable) {
            Some(more) => merge_focus_collections([first, more.focus_collections(position, radius, MAX_FOCUS_ENTRIES)]),
            None => first,
        };
        selected.truncate(cap);
        selected
    }

    /// `cAssetStreamSystem::FindCollections` (`0x824dfea8`) for the simulation focus at
    /// `position`: every Sim file's selection, BAM's through its stream XML with
    /// `kEnableStreamXml`, within the focus radius, at most `kSimMaxFocusCollections`.
    fn focus_selection(&self, position: [f32; 3]) -> Result<Vec<skate2_formats::stream::FocusCollection>, SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        let cap = self
            .source
            .cvars
            .try_sim_max_focus_collections()
            .map_err(E::Cvar)?;
        let space = self
            .focus_space
            .as_ref()
            .ok_or(E::Focus("simulation XSS missing"))?;
        let radius = skate2_formats::stream::WORLD_FOCUS_RADIUS;
        if let Some(area) = &self.area {
            // Online mode: the area's tile whole, else the spatial selection without the cap.
            return Ok(match &area.tile {
                Some(tile) => {
                    let mut selected = tile.iter().map(|&index| space.focus_collection(index, position)).collect::<Vec<_>>();
                    selected.sort_by(|a, b| a.distance_xz.total_cmp(&b.distance_xz));
                    selected
                }
                None => self.spatial_selection(space, None, position, radius, 16),
            });
        }
        let xml = match self.focus_xml.as_ref() {
            Some(xml) if self.source.cvars.try_enable_stream_xml().map_err(E::Cvar)? => Some(xml),
            _ => None,
        };
        Ok(self.spatial_selection(space, xml, position, radius, cap.min(16)))
    }

    fn try_update_focus_inner(
        &mut self,
        position: [f32; 3],
    ) -> Result<(), SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        if !self.complete {
            return Err(E::Pending);
        }
        let disabled = self
            .source
            .cvars
            .try_disable_stream_focus_movement()
            .map_err(E::Cvar)?;
        // MoveStreamFocus824d8250 rejects NaNs instead of replacing coordinates.
        let position = if let Some(area) = &self.area {
            area.position
        } else if disabled || position.iter().any(|v| v.is_nan()) {
            // Movement rejection does not stop an already accepted load.
            let Some(focus) = self.focus.as_ref() else {
                return Ok(());
            };
            focus.position()
        } else {
            position
        };
        let selected = self.focus_selection(position)?;
        // Retain native focus identity and membership; the terrain owner
        // reconciles actual completed registrations with this desired set.
        let focus = self
            .focus
            .get_or_insert_with(|| crate::streaming::StreamFocusState::new(position, false));
        focus.move_to(position, false);
        focus.reconcile(&selected);
        let (recipe_owner,graph_owner) = {
            let world = self.source.world.lock().map_err(|_| E::WorldLock)?;
            let living = world.grind_services.living_world().lock().map_err(|_| E::LivingWorldLock)?;
            (living.recipe_runtime.clone(),living.graph_runtime.clone())
        };
        let graph_guard=graph_owner.as_ref().map(|owner|owner.lock()
            .map_err(|_|E::Focus("graph owner lock poisoned"))).transpose()?;
        let recipe_guard = recipe_owner.as_ref().map(|owner| owner.lock()
            .map_err(|_| E::Focus("RecipeMan owner lock poisoned"))).transpose()?;
        let mut props = recipe_guard.as_ref().map(|runtime| {
            Ok::<_, E>(crate::selected_living_world_recipes::props::PropRecipeFrame {
                graphs: graph_guard.as_deref().ok_or(E::Focus("Prop graph owner absent"))?,
                recipes: &runtime.recipes,
                avoid_volumes: &runtime.challenge_avoid_volumes,
                vault: runtime.vault.as_ref().ok_or(E::Focus("RecipeMan attribute owner missing"))?,
            })
        }).transpose()?;
        self.terrain
            .as_mut()
            .ok_or(E::Pending)?
            .try_stream_collections_with_props(&selected, self.userdata, props.as_mut())
    }
}

#[path = "selected_session_terrain_additional.rs"]
mod additional;

#[cfg(test)]
#[path = "selected_session_terrain_tests.rs"]
mod tests;
