//! Retained first-test terrain admission over the actual resident collection.
//! Host-selected initial collection, then native non-XML spatial focus membership.
//! Registration and deregistration use the same adapters and resident occurrences.
use crate::{
    grind_data_runtime::GrindDataRuntime,
    living_world_mesh_store::MeshStore,
    selected_arena_managers::SelectedArenaManagers,
    selected_resident_startup::SelectedResidentStartup,
    selected_simulation_asset_services::SelectedSimulationAssetServices,
    simulation_dmo_arena_adapter::{PreparedSimulationDmoArena, SimulationDmoArenaPrepareError},
    simulation_dmo_arena_plan::{SimulationDmoArenaPlan, SimulationDmoArenaPlanError},
    simulation_environment_unlock_router::SimulationEnvironmentUnlockRouter,
    simulation_normal_arena_adapter::grind_runtime::{
        NormalGrindPrepareError, NormalGrindRegistration, NormalGrindRegistrationError,
        PreparedNormalGrindArena, SimulationNormalGrindArenaAdapter,
    },
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    world_painter_layer::WorldPainterLayerMan,
};
use dmo::{SelectedDmoPass, SelectedDmoPassError, SelectedDmoRoute};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum SelectedTerrainError {
    AlreadyAttempted,
    ResidencyPending,
    Allocation,
    Prepare(NormalGrindPrepareError),
    Trigger(crate::trigger_camera_group::CameraTriggerError),
    Mesh(crate::living_world_mesh_store::MeshStoreError),
    Rain(crate::living_world_rain::network::RainNetworkError),
    Register(NormalGrindRegistrationError),
    Deregister(
        crate::simulation_normal_arena_adapter::grind_runtime::NormalGrindDeregistrationError,
    ),
    DmoPlan(SimulationDmoArenaPlanError),
    DmoPrepare(SimulationDmoArenaPrepareError),
    Dmo(SelectedDmoPassError),
}
impl std::fmt::Display for SelectedTerrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => f.write_str("selected terrain admission already attempted"),
            Self::ResidencyPending => f.write_str("selected terrain residency is incomplete"),
            Self::Allocation => {
                f.write_str("selected terrain registration storage allocation failed")
            }
            Self::Trigger(e) => write!(f, "selected trigger admission: {e:?}"),
            Self::Mesh(e) => write!(f, "selected navigation admission: {e}"),
            Self::Rain(e) => write!(f, "selected RAIN admission: {e}"),
            Self::Prepare(e) => write!(f, "selected terrain preparation: {e}"),
            Self::Register(e) => write!(f, "selected terrain registration: {e}"),
            Self::Deregister(e) => write!(f, "selected terrain deregistration: {e}"),
            Self::DmoPlan(e) => write!(f, "selected DMO arena plan: {e}"),
            Self::DmoPrepare(e) => write!(f, "selected DMO arena preparation: {e}"),
            Self::Dmo(e) => write!(f, "selected {e}"),
        }
    }
}

pub(crate) struct SelectedTerrainStartup {
    // Actual loader/storage/registry lifetime extends through every consumer.
    pub(crate) resident: SelectedResidentStartup,
    pub(crate) adapter: SimulationNormalGrindArenaAdapter<SelectedArenaManagers>,
    pub(crate) registrations: Vec<NormalGrindRegistration>,
    pub(crate) plans: Vec<Arc<SimulationNormalArenaPlan>>,
    pub(crate) mesh: Arc<MeshStore>,
    pub(crate) assets: Arc<SelectedSimulationAssetServices>,
    pub(crate) layers: Arc<Mutex<WorldPainterLayerMan>>,
    pub(crate) triggers: Arc<Mutex<crate::trigger_camera_group::CameraTriggerGroup>>,
    /// `AIPath::PathManager`, fed by the arena's AI path assets.
    pub(crate) ai_paths: Arc<Mutex<crate::ai::path_manager::PathManager>>,
    /// Absent only for corpora that mount terrain without LivingWorld Init.
    dmo: Option<SelectedDmoPass>,
    physical_world: Option<Arc<Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>>,
    attempted: bool,
    complete: bool,
    additional_pending: bool,
    /// Consecutive ranges in plans/registrations, in admission order.
    collections: Vec<(crate::collection_resident_streamer::CollectionId, usize)>,
    /// Challenge-hull collections (`cHullLoader`): their arena userdata and
    /// whether the hull still holds them. Held ones never leave with the focus.
    hulls: std::collections::BTreeMap<u64, (u32, bool)>,
    /// Hull collections not yet requested from the resident loader.
    hull_queue: std::collections::VecDeque<u64>,
}
impl SelectedTerrainStartup {
    /// Supplied router and world-owned mesh/grind data retain their actual identity.
    /// No empty world, guessed capacity, or alternate asset namespace is created.
    pub(crate) fn new(
        resident: SelectedResidentStartup,
        environment: SimulationEnvironmentUnlockRouter,
        mesh: Arc<MeshStore>,
        grind: Arc<Mutex<GrindDataRuntime>>,
        triggers: Arc<Mutex<crate::trigger_camera_group::CameraTriggerGroup>>,
        dmo: Option<SelectedDmoRoute>,
    ) -> Self {
        let assets = Arc::clone(resident.assets());
        let mut layer = WorldPainterLayerMan::construct();
        layer.initialize(Arc::clone(&assets));
        let layers = Arc::new(Mutex::new(layer));
        let ai_paths = Arc::new(Mutex::new(crate::ai::path_manager::PathManager::new()));
        let adapter = SimulationNormalGrindArenaAdapter::new_selected_profile(
            SelectedArenaManagers(
                Arc::clone(&mesh),
                Arc::clone(&layers),
                Arc::clone(&triggers),
                dmo.as_ref().map(
                    |route| crate::selected_arena_managers::rain::SelectedRainRoute {
                        living_world: Arc::clone(&route.living_world),
                        cvars: route.cvars.clone(),
                        assets: Arc::clone(&assets),
                    },
                ),
                Arc::clone(&ai_paths),
                Arc::clone(&assets),
            ),
            environment,
            grind,
        );
        Self {
            resident,
            adapter,
            registrations: Vec::new(),
            plans: Vec::new(),
            mesh,
            assets,
            layers,
            triggers,
            ai_paths,
            physical_world: dmo.as_ref().map(|route|Arc::clone(&route.physical_world)),
            dmo: dmo.map(SelectedDmoPass::new),
            attempted: false,
            complete: false,
            additional_pending: false,
            collections: Vec::new(),
            hulls: std::collections::BTreeMap::new(),
            hull_queue: std::collections::VecDeque::new(),
        }
    }

    /// A hull's `cAssetStreamManager::LoadCollection`, registered on load with
    /// the hull's asset flags (`InitiateCollectionRegistration` `0x82eecd50`).
    /// Hull loads go before the next focus load.
    pub(crate) fn request_hull_collection(&mut self, id: u64, userdata: u32) -> Result<(), &'static str> {
        if self.hulls.contains_key(&id) || self.collections.iter().any(|(held, _)| held.0 == id) {
            return Err("hull collection is already resident");
        }
        self.hulls.insert(id, (userdata, true));
        self.hull_queue.push_back(id);
        Ok(())
    }

    /// The hull lets the collection go: the next stream pass deregisters and
    /// unloads it; one not yet requested is dropped.
    pub(crate) fn release_hull_collection(&mut self, id: u64) {
        if let Some(position) = self.hull_queue.iter().position(|queued| *queued == id) {
            self.hull_queue.remove(position);
            self.hulls.remove(&id);
        } else if let Some(hull) = self.hulls.get_mut(&id) {
            hull.1 = false;
        }
    }

    pub(crate) fn hull_collection_registered(&self, id: u64) -> bool {
        self.collections.iter().any(|(held, _)| held.0 == id)
    }

    pub(crate) fn hull_collection_released(&self, id: u64) -> bool {
        !self.hulls.contains_key(&id)
    }

    /// Retain successful tokens and any failed adapter prefix. No replay or rollback.
    pub(crate) fn try_register(&mut self,userdata:u32)->Result<(),SelectedTerrainError>{self.try_register_with_props(userdata,None)}
    pub(crate) fn try_register_with_props(&mut self, userdata: u32,
        mut props:Option<&mut crate::selected_living_world_recipes::props::PropRecipeFrame<'_>>,
    ) -> Result<(), SelectedTerrainError> {
        use SelectedTerrainError as E;
        if self.attempted {
            return Err(E::AlreadyAttempted);
        }
        let plans = self.resident.plans().ok_or(E::ResidencyPending)?;
        self.attempted = true;
        if let Some(route) = &self.adapter.dispatch().3 {
            route.bind_assets().map_err(E::Rain)?;
        }
        self.mesh
            .bind_assets(Arc::clone(&self.assets))
            .map_err(E::Mesh)?;
        self.plans
            .try_reserve_exact(plans.len())
            .map_err(|_| E::Allocation)?;
        self.registrations
            .try_reserve_exact(plans.len())
            .map_err(|_| E::Allocation)?;
        self.plans.extend(plans.iter().cloned());
        for plan in &self.plans {
            let prepared =
                PreparedNormalGrindArena::try_new(Arc::clone(plan)).map_err(E::Prepare)?;
            let assets=Arc::clone(&self.assets);
            let route=self.adapter.dispatch().3.as_ref().map(|route|(Arc::clone(&route.living_world),route.cvars.clone()));
            let enabled=props.is_some();
            let mut callback=|id,send| waypoint_operation(props.as_deref_mut().ok_or("waypoint recipe scope absent")?,
                route.as_ref().ok_or("waypoint LivingWorld route absent")?,&assets,self.physical_world.as_ref(),&self.layers,&self.triggers,id,send);
            self.registrations.push(self.adapter.register_with_waypoints(&prepared,userdata,
                if enabled{Some(&mut callback)}else{None}).map_err(E::Register)?);
        }
        self.triggers
            .lock()
            .expect("trigger manager lock")
            .try_finish_admission(&self.plans)
            .map_err(E::Trigger)?;
        let plans = self.plans.clone();
        self.try_register_dmo(&plans, userdata)?;
        self.collections.push((
            self.resident.selected().ok_or(E::ResidencyPending)?,
            plans.len(),
        ));
        self.complete = true;
        Ok(())
    }

    /// `RegDMOArena` over the arenas `RegArena` just admitted, in their order.
    fn try_register_dmo(
        &mut self,
        plans: &[Arc<SimulationNormalArenaPlan>],
        userdata: u32,
    ) -> Result<(), SelectedTerrainError> {
        use SelectedTerrainError as E;
        let Some(dmo) = self.dmo.as_mut() else {
            return Ok(());
        };
        let mut arenas = Vec::new();
        arenas
            .try_reserve_exact(plans.len())
            .map_err(|_| E::Allocation)?;
        for plan in plans {
            let dmo_plan =
                SimulationDmoArenaPlan::from_normal_plan(Arc::clone(plan)).map_err(E::DmoPlan)?;
            arenas.push(
                PreparedSimulationDmoArena::try_new(Arc::new(dmo_plan)).map_err(E::DmoPrepare)?,
            );
        }
        dmo.try_register(&arenas, userdata).map_err(E::Dmo)
    }

    /// Load new focus members into the SAME resident registry and ground adapter.
    /// Departed focus members execute the original DMO-first deregistration
    /// order, then release their resident hold. Returning members can load again.
    pub(crate) fn try_stream_collections(
        &mut self,
        selected: &[skate2_formats::stream::FocusCollection],
        userdata: u32,
    ) -> Result<(), crate::selected_session_terrain::SelectedSessionTerrainError>  {
        self.try_stream_collections_with_props(selected,userdata,None)
    }

    pub(crate) fn try_stream_collections_with_props(
        &mut self,
        selected: &[skate2_formats::stream::FocusCollection],
        userdata: u32,
        mut props:Option<&mut crate::selected_living_world_recipes::props::PropRecipeFrame<'_>>,
    ) -> Result<(), crate::selected_session_terrain::SelectedSessionTerrainError> {
        use crate::selected_session_terrain::SelectedSessionTerrainError as E;
        if !self.complete {
            return Err(E::Pending);
        }
        use crate::diagnostics::frame_profile as profile;
        if self.additional_pending {
            let span = profile::span(profile::TERRAIN_POLL);
            if !self.resident.poll().map_err(E::Residency)? {
                return Ok(());
            }
            drop(span);
            let span = profile::span(profile::TERRAIN_ADMIT);
            let userdata = self.resident.selected().and_then(|id| self.hulls.get(&id.0)).map_or(userdata, |hull| hull.0);
            let plans = self.resident.plans().ok_or(E::Pending)?;
            let mut admitted = Vec::new();
            admitted
                .try_reserve_exact(plans.len())
                .map_err(|_| E::Terrain(SelectedTerrainError::Allocation))?;
            for plan in plans {
                // Different collection decodes can own different Arcs with the
                // same arena ID. Do not silently duplicate or skip Simulation
                // registration until that overlapping ownership path is admitted.
                if self.plans.iter().any(|existing| {
                    existing.static_plan().asset().decoded.asset_id
                        == plan.static_plan().asset().decoded.asset_id
                }) {
                    return Err(E::Focus(
                        "overlapping arena ID requires shared registration ownership",
                    ));
                }
                let prepared = PreparedNormalGrindArena::try_new(Arc::clone(plan))
                    .map_err(|e| E::Terrain(SelectedTerrainError::Prepare(e)))?;
                self.plans
                    .try_reserve(1)
                    .map_err(|_| E::Terrain(SelectedTerrainError::Allocation))?;
                self.registrations
                    .try_reserve(1)
                    .map_err(|_| E::Terrain(SelectedTerrainError::Allocation))?;
                // Retain the plan even if registration fails after partial writes.
                self.plans.push(Arc::clone(plan));
                let assets=Arc::clone(&self.assets);
                let route=self.adapter.dispatch().3.as_ref().map(|route|(Arc::clone(&route.living_world),route.cvars.clone()));
                let enabled=props.is_some();
                let mut callback=|id,send| waypoint_operation(props.as_deref_mut().ok_or("waypoint recipe scope absent")?,
                    route.as_ref().ok_or("waypoint LivingWorld route absent")?,&assets,self.physical_world.as_ref(),&self.layers,&self.triggers,id,send);
                let token=self.adapter.register_with_waypoints(&prepared,userdata,
                    if enabled{Some(&mut callback)}else{None}).map_err(|e| E::Terrain(SelectedTerrainError::Register(e)))?;
                self.registrations.push(token);
                admitted.push(Arc::clone(plan));
            }
            drop(span);
            let _span = profile::span(profile::TERRAIN_FINISH);
            self.triggers
                .lock()
                .expect("trigger manager lock")
                .try_finish_admission(&self.plans)
                .map_err(|e| E::Terrain(SelectedTerrainError::Trigger(e)))?;
            self.try_register_dmo(&admitted, userdata)
                .map_err(E::Terrain)?;
            self.collections
                .push((self.resident.selected().ok_or(E::Pending)?, admitted.len()));
            self.additional_pending = false;
        }
        // cAssetStreamManager::PlanLoad (0x82d57f58): a selected collection's parent loads before
        // it and stays while a selected collection has it.
        let mut parents = Vec::with_capacity(selected.len());
        for entry in selected {
            let id = crate::collection_resident_streamer::CollectionId(entry.id);
            parents.push(self.resident.parent(id).map_err(E::Residency)?.map(|parent| parent.0));
        }
        let span = profile::span(profile::TERRAIN_DEPART);
        let mut index = 0;
        let mut start = 0;
        while index < self.collections.len() {
            let (id, count) = self.collections[index];
            if selected.iter().any(|entry| entry.id == id.0)
                || parents.contains(&Some(id.0))
                || self.hulls.get(&id.0).is_some_and(|hull| hull.1)
            {
                start += count;
                index += 1;
                continue;
            }
            let userdata = self.hulls.get(&id.0).map_or(userdata, |hull| hull.0);
            let range = start..start + count;
            if let Some(dmo) = self.dmo.as_mut() {
                dmo.try_deregister(range.clone(), &self.plans[range.clone()], userdata,
                    props.as_deref(), &self.layers, &self.triggers)
                    .map_err(|error| E::Terrain(SelectedTerrainError::Dmo(error)))?;
            }
            for registration in self.registrations.drain(range.clone()) {
                let assets=Arc::clone(&self.assets);
                let route=self.adapter.dispatch().3.as_ref().map(|route|(Arc::clone(&route.living_world),route.cvars.clone()));
                let enabled=props.is_some();
                let mut callback=|id,send| waypoint_operation(props.as_deref_mut().ok_or("waypoint recipe scope absent")?,
                    route.as_ref().ok_or("waypoint LivingWorld route absent")?,&assets,self.physical_world.as_ref(),&self.layers,&self.triggers,id,send);
                self.adapter.deregister_with_waypoints(registration,userdata,
                    if enabled{Some(&mut callback)}else{None}).map_err(|error| E::Terrain(SelectedTerrainError::Deregister(error)))?;
            }
            self.plans.drain(range);
            self.triggers
                .lock()
                .expect("trigger manager lock")
                .try_finish_admission(&self.plans)
                .map_err(|error| E::Terrain(SelectedTerrainError::Trigger(error)))?;
            self.resident.unload(id).map_err(E::Residency)?;
            self.collections.remove(index);
            self.hulls.remove(&id.0);
        }
        drop(span);
        let _span = profile::span(profile::TERRAIN_REQUEST);
        let next = self.hull_queue.pop_front().or_else(|| {
            let requested = |id: u64| self.resident.was_requested(crate::collection_resident_streamer::CollectionId(id));
            selected.iter().zip(&parents).find_map(|(entry, parent)| {
                parent.filter(|&parent| !requested(parent)).or_else(|| (!requested(entry.id)).then_some(entry.id))
            })
        });
        if let Some(next) = next {
            self.resident
                .request_additional(crate::collection_resident_streamer::CollectionId(next))
                .map_err(E::Residency)?;
            self.additional_pending = true;
        }
        Ok(())
    }
    /// The challenge DMO manager's groups and the kind-31 records their locators come from;
    /// none for a terrain without LivingWorld.
    pub(crate) fn challenge_dmo_chunks(&self) -> Result<Option<dmo::ChallengeDmoChunks>, dmo::DmoPassError> {
        self.dmo.as_ref().map(dmo::SelectedDmoPass::challenge_dmo_chunks).transpose()
    }
    /// The online game's static DMO groups (`Dmo::Registration`'s locators); none for a terrain
    /// without LivingWorld.
    pub(crate) fn static_dmo_groups(&self) -> Result<Vec<u64>, dmo::DmoPassError> {
        self.dmo.as_ref().map_or_else(|| Ok(Vec::new()), dmo::SelectedDmoPass::static_dmo_groups)
    }
    /// Those groups with the kind-31 records their locators come from.
    pub(crate) fn static_dmo_chunks(&self) -> Result<Option<dmo::ChallengeDmoChunks>, dmo::DmoPassError> {
        self.dmo.as_ref().map(dmo::SelectedDmoPass::static_dmo_chunks).transpose()
    }
    /// Actual completed registrations, not requested/in-flight resident loads.
    pub(crate) fn focus_resident(&self, ids: &[u64]) -> bool {
        !ids.is_empty() && !self.additional_pending
            && ids.iter().all(|id| self.collections.iter().any(|(resident, _)| resident.0 == *id))
    }
    /// The collection's registration has completed: the activated state
    /// (`tOctreeInfo::mActivation` 2) `cAssetStreamSystem::IsCollectionLoaded` (`0x824daa58`) reads.
    pub(crate) fn collection_registered(&self, id: u64) -> bool {
        self.collections.iter().any(|(resident, _)| resident.0 == id)
    }
    /// `cAssetStreamSystem::IsStreamFocusLoaded` (`0x824d7a00`) with
    /// `kFocusLoadType_GameLoad`: every collection the focus counts is
    /// registered and none is in flight, so a focus of none is loaded.
    pub(crate) fn focus_loaded(&self, ids: &[u64]) -> bool {
        !self.additional_pending && ids.iter().all(|id| self.collections.iter().any(|(resident, _)| resident.0 == *id))
    }
    #[cfg(test)]
    pub(crate) fn focus_resident_for_test(
        &self,
        selected: &[skate2_formats::stream::FocusCollection],
    ) -> bool {
        !self.additional_pending
            && self.collections.len() == selected.len()
            && selected.iter().all(|entry| {
                self.resident
                    .was_requested(crate::collection_resident_streamer::CollectionId(entry.id))
            })
    }
}

#[path = "selected_terrain_camera.rs"]
pub(crate) mod camera;
#[path = "selected_terrain_dmo.rs"]
pub(crate) mod dmo;

fn waypoint_operation(frame:&mut crate::selected_living_world_recipes::props::PropRecipeFrame<'_>,
    route:&(Arc<Mutex<crate::living_world_grind_services::LivingWorldGrindServices>>,crate::input_cvar_runtime::InputCvarReferences),
    assets:&crate::selected_simulation_asset_services::SelectedSimulationAssetServices,
    physical_world:Option<&Arc<Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>>,
    layers:&Arc<Mutex<WorldPainterLayerMan>>,
    triggers:&Arc<Mutex<crate::trigger_camera_group::CameraTriggerGroup>>,
    id:u64,register:bool)->Result<(),String>{
    let Some(asset)=assets.try_inner(id).map_err(|e|format!("waypoint registry: {e:?}"))? else{return Ok(());};
    let asset=crate::living_world_grind_services::waypoints::WaypointAsset::new(asset)?;
    if register{
        let mut living=route.0.lock().map_err(|_|"waypoint LivingWorld owner lock")?;
        return frame.register_waypoint_asset(&mut living,id,asset,true,&route.1);
    }
    // Same world/living lock order as the live simulation and DMO teardown.
    let mut world=physical_world.ok_or("waypoint cull physics owner absent")?
        .lock().map_err(|_|"waypoint cull physics owner lock")?;
    let mut living=route.0.lock().map_err(|_|"waypoint LivingWorld owner lock")?;
    let avoid={let triggers=triggers.lock().map_err(|_|"waypoint cull trigger lock")?;
        frame.avoid_volumes.resolve(&triggers)?};
    let painter=layers.lock().map_err(|_|"waypoint cull painter lock")?;
    frame.deregister_waypoint_asset(&mut living,id,asset,false,&mut world,&route.1,&painter,&avoid)
}
