//! Selected query projection of ONE Simulation-owned `LivingWorld` occurrence.
//! Root retains this owner across queries. A future complete Manager must retain
//! the same occurrence; this does not execute or certify its unrelated Init.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected entity storage is retained for later live Manager scheduling"
    )
)]
#[path = "../assets/living_world_attrib_cache.rs"]
pub(crate) mod attrib_cache;
#[path = "../population/living_world_census.rs"]
pub(crate) mod census;
#[path = "../entities/living_world_character_models.rs"]
pub(crate) mod character_models;
#[path = "../entities/living_world_collideable_objects.rs"]
pub(crate) mod collideable_objects;
#[path = "../entities/living_world_collision_drive.rs"]
pub(crate) mod collision_drive;
#[path = "../dmos/living_world_dmo_locators.rs"]
pub(crate) mod dmo_locators;
#[path = "../dmos/living_world_dmo_models.rs"]
pub(crate) mod dmo_models;
#[path = "../dmos/living_world_dmo_objects.rs"]
pub(crate) mod dmo_objects;
#[path = "living_world_embedded_splines.rs"]
pub(crate) mod embedded_splines;
#[path = "living_world_grind_entities.rs"]
pub(crate) mod entities;
#[path = "../entities/living_world_entity_core.rs"]
pub(crate) mod entity_core;
#[path = "../entities/living_world_execution.rs"]
pub(crate) mod execution;
#[path = "../pedestrians/pedestrian_waypoints.rs"]
pub(crate) mod waypoints;
#[path = "living_world_grab_provider.rs"]
pub(crate) mod grab_provider;
#[path = "living_world_grind_provider.rs"]
pub(crate) mod provider;
#[path = "living_world_spline_registration.rs"]
pub(crate) mod spline_registration;
#[path = "../vehicles/living_world_vehicle_entity.rs"]
pub(crate) mod vehicle_entity;
#[path = "../vehicles/living_world_vehicle_graph.rs"]
pub(crate) mod vehicle_graph;
#[path = "../vehicles/living_world_vehicle_motion.rs"]
pub(crate) mod vehicle_motion;
#[path = "../vehicles/living_world_vehicle_queries.rs"]
pub(crate) mod vehicle_queries;
#[path = "living_world_scoring.rs"]
pub(crate) mod scoring;
use crate::input_cvar_runtime::InputCvarReferences;
use crate::living_world_mesh_store::{MeshMan, MeshStore, MeshStoreError};
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    living_world_rain::{RainMan, RainServiceError},
    skeleton_root_transforms::SkeletonRootVector,
};
use dmo_locators::{DynamicObjectLocatorMan, LocatorError, LocatorServices};
use dmo_models::DmoModelMan;
use dmo_objects::DmoObjectMan;
use entities::{EntityInitError, EntityMan, EntityNearQueryError};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum LivingWorldInitError {
    PedestrianPerceptions(String),
    Mood(String),
    Prop(String),
    PropExecution(execution::ExecutionError),
    DynamicObjectExecution(execution::ExecutionError),
    PedestrianExecution(execution::ExecutionError),
    CensusCvar(crate::input_cvar_runtime::InputCvarError),
    VehicleExecution(vehicle_entity::execution::VehicleExecutionError),
    Entity(EntityInitError),
    Traffic(crate::living_world_rain::traffic::TrafficError),
    Rain(crate::living_world_rain::network::RainNetworkError),
    Random(crate::native_skater_random::rw_math::RwRandomLockError),
}
impl From<EntityInitError> for LivingWorldInitError {
    fn from(value: EntityInitError) -> Self {
        Self::Entity(value)
    }
}
impl std::fmt::Display for LivingWorldInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LivingWorld initialization: {self:?}")
    }
}

pub(crate) struct LivingWorldGrindServices {
    /// InitLWMan82702eb8 borrows Simulation's broadcaster; no private LW queue.
    pub(crate) game_sim_events: Option<Arc<crate::hermes_game_sim_queue::GameSimEventRoute>>,
    pub(crate) graph_runtime: Option<Arc<std::sync::Mutex<crate::actor_construction::graph_startup::ActorGraphStartupOwners>>>,
    pub(crate) recipe_runtime: Option<Arc<std::sync::Mutex<crate::selected_living_world_recipes::SelectedLivingWorldRecipes>>>,
    /// One native global sequence for pedestrians, vehicles, props and DMOs.
    pub(crate) entity_ids: crate::world::LivingWorldEntityIdAllocator,
    pub(crate) spline_ids: spline_registration::LivingWorldSplineIds,
    pub(crate) census: census::CensusState,
    pub(crate) census_targets: attrib_cache::CensusTargetCache,
    /// Manager.mRandom; distinct from the source-global rw::math generator.
    pub(crate) random: Option<crate::native_skater_random::NativeSkaterRandom>,
    /// Live Manager counter82710cc0; dormant physical constructor snapshot
    /// does not drive frames. Initialized by Manager ctor82721f00.
    pub(crate) behaviour_evaluate_frame_timer: u32,
    /// Same embedded `mPedestrianMan` in the retained Manager occurrence.
    pub(crate) pedestrian_entities: Option<Box<EntityMan<40, crate::living_world_pedestrian_entity::PedestrianConstruction>>>,
    pub(crate) pedestrian_memory: Option<crate::living_world_pedestrian_memory::PedestrianMemory>,
    pub(crate) pedestrian_execution: execution::pedestrian::PedestrianExecutionMan,
    /// LWPedestrianMan+0xa64, distinct from each entity's perception RefSpec.
    pub(crate) pedestrian_group_perceptions: Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    /// Persistent double buffers belonging to the same pedestrian execution man.
    /// Constructed with the roster; physics leases are acquired during execution Init.
    pub(crate) pedestrian_queries: Option<crate::pedestrian_execution_queries::PedestrianExecutionQueries>,
    pub(crate) pedestrian_animation_construction: crate::living_world_pedestrian_entity::factory::PedestrianAnimConstructInfo,
    /// A failed native constructor/spawn prefix, never an active-roster member.
    pub(crate) pedestrian_spawn_unfinished: Option<crate::living_world_pedestrian_entity::PedestrianConstruction>,
    /// Census RequestCull detaches immediately; Census.Update destroys later.
    pub(crate) pedestrian_culls: Vec<crate::living_world_pedestrian_entity::PedestrianConstruction>,
    /// Sole PropMan owner, including waypoint-group payloads and reservations.
    pub(crate) props: Option<Box<waypoints::PropMan>>,
    /// `Manager::mWaypointMan`: the spectator waypoint groups waiting for forced spectators.
    pub(crate) waypoint_man: waypoints::spectators::WaypointMan,
    /// Same embedded ChaseMan constructed at Manager82721f00. Its message
    /// subscriptions and per-frame consumers are separate initialization work.
    pub(crate) chase: Option<crate::living_world_chase::ChaseMan>,
    pub(crate) mood: Option<crate::pedestrian_mood::MoodMan>,
    /// Manager.mpBigBlack explicitly null at82721fec. The actual spawn handler
    /// must publish its retained entity ID here when that owner is constructed.
    pub(crate) big_black_entity: Option<u32>,
    /// `LWDynamicObjectMan`'s entity manager, priorities and the DMO census.
    pub(crate) dmo_objects: Option<Box<DmoObjectMan>>,
    pub(crate) grind_vehicle_entities: Option<Box<EntityMan<24, vehicle_entity::Vehicle>>>,
    pub(crate) vehicle_factory: Option<vehicle_entity::factory::VehicleFactory>,
    /// A checked spawn failure retains its prefix; not a second active roster.
    pub(crate) vehicle_spawn_unfinished: Option<vehicle_entity::Vehicle>,
    /// Detached entities awaiting the census destructor, still owning bodies.
    pub(crate) vehicle_culls: Vec<vehicle_entity::Vehicle>,
    pub(crate) vehicle_ground_results: Vec<(u32, [Option<[f32; 4]>; 3])>,
    pub(crate) vehicle_execution: vehicle_entity::execution::VehicleExecutionMan,
    pub(crate) vehicle_collideables: collideable_objects::CollideableObjectMan,
    pub(crate) vehicle_grabbables: spline_registration::GrabbableObjectMan,
    /// `LWDynamicObjectMan::mpLocatorMan` (+0x3aac).
    pub(crate) dmo_locators: Option<Box<DynamicObjectLocatorMan>>,
    /// `Sk8::LivingWorld::ModelMan` DMO models (`AddDMOModel`).
    pub(crate) dmo_models: DmoModelMan,
    pedestrian_man_published: bool,
    pub(crate) mesh: Option<MeshMan>,
    pub(crate) rain: Option<RainMan>,
    /// PC port (Skate 3 physics): this tick's Skate 3 solver contacts with pedestrian Parts, read
    /// by the next `pedestrian_resolve_contacts`.
    pub(crate) skate3_contacts: Vec<crate::pedestrian_collision::contacts::Skate3PedestrianContact>,
    attempted: bool,
}
impl LivingWorldGrindServices {
    /// `PedestrianMan::ReleaseNavMeshTile` (`0x82e6e0f8`) for each tile the MeshStore removed
    /// since the last frame: no pedestrian's rig or path keeps a cell or edge of it.
    pub(crate) fn release_removed_nav_tiles(&mut self) -> Result<(), String> {
        let Some(mesh) = self.mesh.as_ref() else { return Ok(()) };
        let released = mesh.store.take_released().map_err(|e| format!("MeshStore released tiles: {e:?}"))?;
        let Some(roster) = self.pedestrian_entities.as_mut() else { return Ok(()) };
        for tile in &released {
            for id in roster.ids() {
                let Some(entity) = roster.get_mut(id) else { continue };
                if let Some(rig) = entity.nav_rig.as_mut() {
                    rig.mesh_info.release_tile(tile);
                }
                if let Some(navigation) = entity.mesh_navigation.as_mut() {
                    navigation.release_tile(tile);
                }
            }
        }
        Ok(())
    }
    pub(crate) const fn new() -> Self {
        Self {
            game_sim_events: None,
            graph_runtime: None,
            recipe_runtime: None,
            entity_ids: crate::world::LivingWorldEntityIdAllocator::from_next_counter(1),
            spline_ids: spline_registration::LivingWorldSplineIds::new(),
            census: census::CensusState::new(true),
            census_targets: attrib_cache::CensusTargetCache::new(),
            random: None,
            behaviour_evaluate_frame_timer: 0,
            pedestrian_entities: None,
            pedestrian_memory: None,
            pedestrian_execution: execution::pedestrian::PedestrianExecutionMan::new(),
            pedestrian_group_perceptions: None,
            pedestrian_queries: None,
            pedestrian_animation_construction: crate::living_world_pedestrian_entity::factory::PedestrianAnimConstructInfo::new(),
            pedestrian_spawn_unfinished: None,
            pedestrian_culls: Vec::new(),
            props: None,
            waypoint_man: waypoints::spectators::WaypointMan::new(),
            chase: None,
            mood: None,
            big_black_entity: None,
            dmo_objects: None,
            grind_vehicle_entities: None,
            vehicle_factory: None,
            vehicle_spawn_unfinished: None,
            vehicle_culls: Vec::new(),
            vehicle_ground_results: Vec::new(),
            vehicle_execution: vehicle_entity::execution::VehicleExecutionMan::new(),
            vehicle_collideables: collideable_objects::CollideableObjectMan::new(),
            vehicle_grabbables: spline_registration::GrabbableObjectMan::new(),
            dmo_locators: None,
            dmo_models: DmoModelMan::new(),
            pedestrian_man_published: false,
            mesh: None,
            rain: None,
            skate3_contacts: Vec::new(),
            attempted: false,
        }
    }
    /// Selected source subsequence of `Manager.Init`8271f6f0: pedestrian Init
    /// f8a8 precedes vehicle f8bc and DMO f8e4, with Prop Init between them.
    /// Preserve each published constructor before its own `MemStore` Init.
    pub(crate) fn try_initialize_selected_entities(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<(), LivingWorldInitError> {
        self.initialize_entity_owners(vault, None)
    }
    pub(crate) fn try_initialize_selected_entities_with_execution(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
    ) -> Result<(), LivingWorldInitError> {
        self.initialize_entity_owners(vault, Some(cvars))
    }
    fn initialize_entity_owners(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: Option<&crate::input_cvar_runtime::InputCvarReferences>,
    ) -> Result<(), LivingWorldInitError> {
        if self.attempted {
            return Err(EntityInitError::AlreadyConstructed.into());
        }
        self.attempted = true;
        // Manager constructor82721e40 Reset precedes embedded MeshMan/RAIN.
        self.random = Some(crate::native_skater_random::NativeSkaterRandom::reset());
        self.census.pedestrian_range_key = Some(skate2_formats::attrib::string_hash64("pedestrians"));
        // Manager ctor82721e4c constructs CensusMan before MeshMan/RAIN;
        // CensusMan82720a0c captures the linked initial vehicle allowance.
        if let Some(cvars) = cvars {
            self.census.max_pedestrians = Some(
                cvars.try_initial_spawn_pedestrian_count()
                    .map_err(LivingWorldInitError::CensusCvar)?,
            );
            self.census.max_vehicles = Some(
                cvars
                    .try_initial_spawn_vehicle_count()
                    .map_err(LivingWorldInitError::CensusCvar)?,
            );
        }
        // Manager constructor82721e58 precedes the selected EntityMan Init calls.
        // This retains its actual MeshMan child, not full Manager initialization.
        self.mesh = Some(MeshMan::construct_fields());
        // Native embedded RAIN constructor82721e64 follows MeshMan82721e58.
        // Both precede the selected later entity Init subsequence.
        self.rain = Some(RainMan::construct(vault));
        self.chase = Some(crate::living_world_chase::ChaseMan::default());
        // Manager constructor82721f18 follows the embedded ChaseMan.
        self.mood = Some(crate::pedestrian_mood::MoodMan::new());
        // Original82722018 seeds this field from rw::math::Random. This is
        // the selected constructor subsequence, not full Manager construction.
        let seed = crate::native_skater_random::rw_math::global_next()
            .map_err(LivingWorldInitError::Random)?;
        self.random
            .as_mut()
            .expect("Manager RNG reset")
            .set_seed(seed);
        // Manager.Init8271f86c initializes the traffic clock before entity Init.
        let rain = self.rain.as_mut().expect("constructed RAIN owner");
        rain.initialize_storage()
            .map_err(LivingWorldInitError::Rain)?;
        rain.initialize_traffic()
            .map_err(LivingWorldInitError::Traffic)?;
        if cvars.is_some() {
            // Manager.Init8271f890 before pedestrian Init8271f8a8. Preserve
            // category binding, subscription, event-map construction order.
            let mood = self.mood.as_mut().expect("constructed MoodMan");
            mood.initialize_categories(vault).map_err(LivingWorldInitError::Mood)?;
            self.game_sim_events.as_ref().ok_or_else(|| LivingWorldInitError::Mood(
                "MoodMan.Init requires GameSimulationBroadcaster".into()))?.register_mood_man();
            mood.initialize_event_map(vault).map_err(LivingWorldInitError::Mood)?;
        }
        // InitLWMan827030c4 publishes the embedded pedestrian manager before
        // its virtual Init at8271f8a8. This is only that selected publication,
        // not the whole native InitLWMan or LWPedestrianMan.Init side effects.
        self.pedestrian_man_published = true;
        // LWPedestrianMemoryMan.Init82e6b070 precedes the manager's entity
        // initialization. Same allocation owner will be used by spawn/cull.
        self.pedestrian_memory = Some(crate::living_world_pedestrian_memory::PedestrianMemory::initialized());
        self.pedestrian_entities = Some(EntityMan::try_construct()?);
        self.pedestrian_queries = Some(crate::pedestrian_execution_queries::PedestrianExecutionQueries::new());
        self.pedestrian_entities
            .as_mut()
            .expect("published pedestrian EntityMan")
            .try_initialize()?;
        if let Some(cvars) = cvars {
            // LWPedestrianMan.Init82e9ef00 captures its three execution rates
            // before Manager proceeds to vehicle initialization.
            self.pedestrian_execution.initialize(cvars)
                .map_err(LivingWorldInitError::PedestrianExecution)?;
            // Init82e9ef90..eff4 binds the global group after execution rates.
            let perceptions=vault.instance(0x86eb_6b56_4cf1_2561,
                skate2_formats::attrib::string_hash64("global"));
            let missing=perceptions.is_null();
            self.pedestrian_group_perceptions=Some(perceptions);
            if missing {return Err(LivingWorldInitError::PedestrianPerceptions(
                "LWPedestrianMan global perceptions assertion82e9eff0".into()));}
        }
        // Vehicle MemoryMan.Init precedes EntityMan.Init and factory Init.
        // Factory default is Regular with an empty callback override map.
        self.vehicle_factory = Some(vehicle_entity::factory::VehicleFactory::initialized());
        self.grind_vehicle_entities = Some(EntityMan::try_construct()?);
        self.grind_vehicle_entities
            .as_mut()
            .expect("published vehicle EntityMan")
            .try_initialize()?;
        // LWVehicleMan.Init creates its execution groups after base Init and
        // before returning to Manager's later DMO Init. Old query-only probes
        // select the constructor/storage prefix without claiming scheduling.
        if let Some(cvars) = cvars {
            self.vehicle_execution
                .initialize(cvars)
                .map_err(LivingWorldInitError::VehicleExecution)?;
        }
        // LWPropMan.Init82e69ac0: component memory/entities/factory precede
        // the three execution groups, before Manager proceeds to DMO Init.
        self.props = Some(Box::new(waypoints::PropMan::initialize().map_err(LivingWorldInitError::Prop)?));
        // Manager.Init's WaypointMan::Init (0x82ce7f50) empties the spectator queue.
        self.waypoint_man.init();
        if let Some(cvars) = cvars {
            let props = self.props.as_mut().expect("PropMan retained above");
            props.factory_callbacks = Some(cvars.try_prop_factory_callbacks().map_err(LivingWorldInitError::CensusCvar)?);
            props.execution.initialize_with(|| cvars.try_prop_execution_rates()).map_err(LivingWorldInitError::PropExecution)?;
        }
        // DMO EntityMan ctor 0x82d366d0/Init 0x82d36748 and the CensusMan
        // ctor/Init: `Simulation::IsLWSpawnEnabled` is true offline (neither a
        // network game nor the ghost system).
        self.dmo_objects = Some(DmoObjectMan::try_construct().ok_or(EntityInitError::Allocation)?);
        self.dmo_objects.as_deref_mut().expect("constructed DMOMan").game_sim_events=self.game_sim_events.clone();
        if let Some(cvars)=cvars {
            self.dmo_objects.as_deref_mut().expect("constructed DMOMan").execution
                .initialize_with(||cvars.try_dynamic_object_execution_rates())
                .map_err(LivingWorldInitError::DynamicObjectExecution)?;
        }
        // LWDynamicObjectMan::Init 0x82d3e0cc..e124, after its LWComponentMan Init.
        self.dmo_locators =
            Some(DynamicObjectLocatorMan::try_construct().ok_or(EntityInitError::Allocation)?);
        Ok(())
    }

    /// `LWDynamicObjectMan::AddStreamedLocatorData` 0x82d3d6d0: persistence and
    /// priority tracking allowed, not treated as static.
    pub(crate) fn try_add_streamed_locator_data<S: LocatorServices>(
        &mut self,
        asset_id: u64,
        asset_flags: u32,
        services: &mut S,
    ) -> Result<(), DmoLocatorRouteError<S::Error>> {
        self.dmo_locators
            .as_mut()
            .ok_or(DmoLocatorRouteError::MissingLocatorMan)?
            .add_streamed_locator_data(asset_id, asset_flags, true, true, false, services)
            .map_err(DmoLocatorRouteError::Locator)
    }
    pub(crate) fn try_remove_streamed_locator_data<S: LocatorServices>(
        &mut self,
        asset_id: u64,
        asset_flags: u32,
        services: &mut S,
    ) -> Result<(), DmoLocatorRouteError<S::Error>> {
        let objects = self
            .dmo_objects
            .as_mut()
            .ok_or(DmoLocatorRouteError::MissingLocatorMan)?;
        self.dmo_locators
            .as_mut()
            .ok_or(DmoLocatorRouteError::MissingLocatorMan)?
            .remove_streamed_locator_data(asset_id, asset_flags, objects, services)
            .map_err(DmoLocatorRouteError::Locator)
    }
    /// Same native child used by normal kind-23 admission and camera road queries.
    pub(crate) fn try_remove_streamed_locator_data_with_world<S:LocatorServices>(
        &mut self,asset_id:u64,asset_flags:u32,services:&mut S,
        world:&mut crate::physical_player_assembly::PhysicalAssemblyWorld,cvars:&InputCvarReferences,
    )->Result<(),DmoLocatorRouteError<S::Error>>{
        let pedestrians=self.pedestrian_entities.as_deref_mut().ok_or(DmoLocatorRouteError::MissingLocatorMan)?;
        let objects=self.dmo_objects.as_deref_mut().ok_or(DmoLocatorRouteError::MissingLocatorMan)?;
        self.dmo_locators.as_deref_mut().ok_or(DmoLocatorRouteError::MissingLocatorMan)?
            .remove_streamed_locator_data_with_monitors(asset_id,asset_flags,objects,services,|objects,monitor,id|{
                if let Some(entity)=pedestrians.get_mut(monitor){entity.monitored_object_being_culled(id,world,objects,cvars)?;}
                Ok(())
            }).map_err(DmoLocatorRouteError::Locator)
    }
    /// Same native child used by normal kind-23 admission and camera road queries.
    pub(crate) fn try_mesh_store(&self) -> Result<Arc<MeshStore>, MeshStoreError> {
        self.mesh
            .as_ref()
            .map(|mesh| Arc::clone(&mesh.store))
            .ok_or(MeshStoreError::MissingConstruction)
    }
    /// Retained RAIN receiver and SAME native `MeshMan` child for camera road reads.
    /// This does not supply the still-missing `InitLWMan` global publication token.
    pub(crate) fn try_point_on_road_network(
        &self,
        position: SkeletonRootVector,
    ) -> Result<bool, RainServiceError> {
        let rain = self
            .rain
            .as_ref()
            .ok_or(RainServiceError::MissingConstruction)?;
        let mesh = self.try_mesh_store().map_err(RainServiceError::Mesh)?;
        rain.try_point_on_road_network(&mesh, position)
            .map_err(RainServiceError::Mesh)
    }
    /// Host admission proof for the two selected entity stores, not full LW Init.
    pub(crate) fn selected_entities_initialized(&self) -> bool {
        self.pedestrian_man_published
            && self
                .pedestrian_entities
                .as_ref()
                .is_some_and(|owner| owner.initialized())
            && self
                .grind_vehicle_entities
                .as_ref()
                .is_some_and(|owner| owner.initialized())
            && self.dmo_objects.is_some()
    }

    /// `GetPedestrianMan`82e4f078 -> `GetPedestriansWithinRange`82e9ab88 ->
    /// `KDTree::FindNear`82d28c00 through the same typed EntityMan40 tree.
    /// The manager's separate spatial rebuild phase controls visibility.
    pub(crate) fn try_get_pedestrians_within_range(
        &self,
        point: [f32; 2],
        radius: f32,
        max_entities: u32,
        result_count: &mut u32,
        entity_ids: &mut [u32],
        distances: &mut [f32],
    ) -> Result<(), PedestrianNearQueryError> {
        if !self.pedestrian_man_published {
            return Err(PedestrianNearQueryError::UnpublishedManager);
        }
        let owner = self
            .pedestrian_entities
            .as_ref()
            .ok_or(PedestrianNearQueryError::MissingEntityMan)?;
        *result_count = 0;
        let found = owner.find_near(point, radius, max_entities.min(40),
            &mut crate::reckoning_numeric::HostReckoningNumeric)
            .map_err(|error| PedestrianNearQueryError::Spatial(format!("{error:?}")))?;
        if found.len() > entity_ids.len() || found.len() > distances.len() {
            return Err(PedestrianNearQueryError::OutputCapacity);
        }
        for (index, (id, distance)) in found.iter().copied().enumerate() {
            entity_ids[index] = id;
            distances[index] = distance;
        }
        *result_count = found.len() as u32;
        Ok(())
    }

    /// `GetVehicleMan` -> `LWVehicleMan::GetVehiclesWithinRange` 0x82d29030
    /// -> `KDTree::FindNear` 0x82d28c00. This uses the SAME selected vehicle
    /// EntityMan and populated KD traversal as grind queries.
    pub(crate) fn try_get_vehicles_within_range(
        &self,
        point: [f32; 2],
        radius: f32,
        max_entities: u32,
        result_count: &mut u32,
        entity_ids: &mut [u32],
        distances: &mut [f32],
    ) -> Result<(), VehicleNearQueryError> {
        let owner = self
            .grind_vehicle_entities
            .as_ref()
            .ok_or(VehicleNearQueryError::MissingEntityMan)?;
        *result_count = 0;
        let found = owner
            .find_near(
                point,
                radius,
                max_entities.min(24), // 0x82d29034..40, before FindNear.
                &mut crate::reckoning_numeric::HostReckoningNumeric,
            )
            .map_err(VehicleNearQueryError::Spatial)?;
        if found.len() > entity_ids.len() || found.len() > distances.len() {
            return Err(VehicleNearQueryError::OutputCapacity);
        }
        // Native writes only returned entries, in the KD query's sorted order.
        for (index, (id, distance)) in found.iter().copied().enumerate() {
            entity_ids[index] = id;
            distances[index] = distance;
        }
        *result_count = found.len() as u32;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum VehicleNearQueryError {
    MissingEntityMan,
    Spatial(entities::EntitySpatialError<crate::reckoning_numeric::ReckoningHostNumericError>),
    OutputCapacity,
}

impl std::fmt::Display for VehicleNearQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LivingWorld vehicle range query: {self:?}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PedestrianNearQueryError {
    UnpublishedManager,
    MissingEntityMan,
    Entity(EntityNearQueryError),
    Spatial(String),
    OutputCapacity,
}

impl std::fmt::Display for PedestrianNearQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LivingWorld pedestrian range query: {self:?}")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DmoLocatorRouteError<E> {
    /// The dynamic-object manager Init has not run.
    MissingLocatorMan,
    Locator(LocatorError<E>),
}

impl<E: std::fmt::Display> std::fmt::Display for DmoLocatorRouteError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingLocatorMan => {
                f.write_str("LivingWorld DMO locator manager is not constructed")
            }
            Self::Locator(error) => error.fmt(f),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::{EntityInitError, LivingWorldGrindServices, PedestrianNearQueryError};
    #[test]
    fn selected_bootstrap_retains_actual_entity_stores_and_refuses_replay() {
        let mut service = LivingWorldGrindServices::new();
        let vault = std::sync::Arc::new(crate::attribute_runtime::tests::graph(0, vec![]));
        assert!(!service.selected_entities_initialized());
        let mut count = 7;
        let mut ids = [0xdead_beef; 20];
        let mut distances = [42.0; 20];
        assert_eq!(
            service.try_get_pedestrians_within_range(
                [1.0, 2.0],
                10.0,
                20,
                &mut count,
                &mut ids,
                &mut distances,
            ),
            Err(PedestrianNearQueryError::UnpublishedManager)
        );
        assert_eq!(count, 7);
        let position = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
        assert_eq!(
            service.try_point_on_road_network(position),
            Err(crate::living_world_rain::RainServiceError::MissingConstruction)
        );
        service.try_initialize_selected_entities(&vault).unwrap();
        assert!(service.selected_entities_initialized());
        service
            .try_get_pedestrians_within_range(
                [1.0, 2.0],
                10.0,
                20,
                &mut count,
                &mut ids,
                &mut distances,
            )
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(ids, [0xdead_beef; 20]);
        assert_eq!(distances, [42.0; 20]);
        count = 5;
        service
            .try_get_pedestrians_within_range(
                [1.0, 2.0],
                10.0,
                257,
                &mut count,
                &mut ids,
                &mut distances,
            )
            .unwrap();
        assert_eq!(count, 0); // Pedestrian wrapper clamps max entities to 40.
        assert!(!service.try_point_on_road_network(position).unwrap());
        let mesh = service.try_mesh_store().unwrap();
        let expected = crate::living_world_mesh_store::MeshStoreError::PendingTileAdmission {
            asset_id: 0x1234,
        };
        assert_eq!(mesh.try_add_tile(0x1234), Err(expected));
        assert_eq!(
            service.try_point_on_road_network(position),
            Err(crate::living_world_rain::RainServiceError::Mesh(expected))
        );
        let pedestrian = std::ptr::from_ref(service.pedestrian_entities.as_deref().unwrap());
        let vehicle = std::ptr::from_ref(service.grind_vehicle_entities.as_deref().unwrap());
        let dmo = std::ptr::from_ref(service.dmo_objects.as_deref().unwrap());
        assert!(matches!(
            service.try_initialize_selected_entities(&vault),
            Err(super::LivingWorldInitError::Entity(
                EntityInitError::AlreadyConstructed
            ))
        ));
        assert_eq!(
            pedestrian,
            std::ptr::from_ref(service.pedestrian_entities.as_deref().unwrap())
        );
        assert_eq!(
            vehicle,
            std::ptr::from_ref(service.grind_vehicle_entities.as_deref().unwrap())
        );
        assert_eq!(
            dmo,
            std::ptr::from_ref(service.dmo_objects.as_deref().unwrap())
        );
    }
}
