//! Selected-session LivingWorld DMO schedule over the retained World and the
//! World's single LivingWorld Manager occurrence.
//!
//! Boot: `GameModule::SecondaryLoad01` 0x8272d550 loads the main DMO bank
//! (`cDMOBankManager::LoadMainDMOBank` 0x82e0e628, gated by
//! `kEnableMainDMOBank`); its `Simulation::RegArena` 0x828a01e0 pass sends
//! every kind-11 instance to `ModelMan::AddDMOModel` 0x82ce9c60. The exit of
//! `StateLoading::CharacterPrepareState` 0x82760058 runs the initial census:
//! `ClearSpatialLookups`, `EvaluateCensus(streamFocus, 0, true)` and
//! `UpdateSpatialLookups`.
//!
//! Tick: `SimUpdate1` 0x8289f410 runs `Manager::Update1` 0x82727ee0 and
//! `World::BeginDMOUpdate` 0x827adec0 after `RecipeMan::Update` and before
//! `SkaterManager::UpdateAnimation1`; `SimUpdate2` 0x8289f980 runs
//! `Manager::UpdatePostTransforms` 0x827031e8 before `CameraManager::Update`.
//! Both LivingWorld calls pass `kUpdateTime`.
use crate::{
    actor_construction::ActorConstruction,
    attribute_runtime::AttributeRuntimeVault,
    dmo_bank_processed_asset::DmoBankProcessedArenaAsset,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_grind_services::{
        LivingWorldGrindServices,
        dmo_models::{DmoModelCvars, DmoModelError, DmoModelSource},
        dmo_objects::{DmoCensusCvars, DmoCensusError, DmoCensusInputs, DmoFrame},
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::HostReckoningNumeric,
    selected_dmo_bank_startup::{SelectedDmoBankError, SelectedDmoBankStartup},
    simulation_assembly_backend::constraint_session::{
        dmo_living_world::{
            DmoFrameSettings, DmoWorldError, LivingWorldDmoWorld, local_skater_census_centre,
            local_skater_census_gate,
        },
        dmo_pool::SimulationDmoPoolInitError,
    },
    simulation_normal_arena_plan::{SimulationNormalArenaPlan, SimulationNormalOperation},
    simulation_xenon_address_image::{
        SimulationArenaXenonAddressBinding, SimulationArenaXenonAddressBindingError,
        SimulationXenonAddressImage, SimulationXenonAddressImageError,
    },
    skater_manager_local_identity::{
        LocalIdentityError, SkaterManagerLocalIdentity, observers::SkaterObserverError,
    },
};
use skate2_formats::mount::MountSet;
use std::{
    fmt,
    sync::{Arc, Mutex},
};

/// `SpawnStatic` 0x825eb1c8..b204: a model whose Assembly has no Part or more than 26 is
/// skipped.
const SPAWN_STATIC_MAX_PARTS: usize = 26;

#[path = "selected_living_world_hand_prop_spawn.rs"]
mod hand_prop_spawn;
#[path = "selected_living_world_game_reset.rs"]
mod game_reset;

/// `kUpdateTime` (`__real_3c888889`), the dT both LivingWorld calls receive.
const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889);
/// Host layout of the `Simulation` allocation the source routes publish into:
/// the World's `SimulationManager` capacities (`Island::InitWorld`: 860
/// bodies, 360 joints, 540 drives) at an 0x80-aligned base.
const SIMULATION_IMAGE_BASE: u32 = 0x1000_0080;
const SIMULATION_IMAGE_BODIES: u32 = 860;
const SIMULATION_IMAGE_JOINTS: u32 = 360;
const SIMULATION_IMAGE_DRIVES: u32 = 540;
/// Host layout of the bank arenas: arena `n`'s five resources packed at their
/// descriptor alignments from `ARENA_REGION_BASE + n * ARENA_REGION_SIZE`.
const ARENA_REGION_BASE: u32 = 0x2000_0000;
const ARENA_REGION_SIZE: u32 = 0x0100_0000;

#[derive(Debug)]
pub(crate) enum SelectedLivingWorldDmoError {
    Cvar(InputCvarError),
    PresentationAttribute(crate::attribute_runtime::AttributeRuntimeError),
    Bank(SelectedDmoBankError),
    /// Registration requested before the bank finished loading.
    BankPending,
    AlreadyRegistered,
    LivingWorldLock,
    /// `Manager::Init` has not published the DMO entity or locator manager.
    MissingManagers,
    Image(SimulationXenonAddressImageError),
    Binding {
        asset: usize,
        source: SimulationArenaXenonAddressBindingError,
    },
    /// One bank arena's resources exceed its host address region.
    ArenaRegion {
        asset: usize,
    },
    Source(SimulationDmoPoolInitError),
    Model(DmoModelError),
    World(DmoWorldError),
    Census(DmoCensusError<DmoWorldError>),
    Camera(SkaterObserverError),
    Identity(LocalIdentityError),
    Traffic(crate::living_world_rain::traffic::TrafficError),
    Rain(crate::living_world_rain::network::RainNetworkError),
    Vehicles(crate::living_world_grind_services::vehicle_entity::frame::VehicleFrameError),
    Pedestrians(String),
    PedestrianContacts(String),
    ScoringContacts(crate::living_world_grind_services::scoring::ScoringEntitiesError),
}

impl fmt::Display for SelectedLivingWorldDmoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bank(error) => error.fmt(f),
            Self::Model(error) => error.fmt(f),
            Self::Census(error) => error.fmt(f),
            other => write!(f, "LivingWorld DMOs: {other:?}"),
        }
    }
}

impl std::error::Error for SelectedLivingWorldDmoError {}

enum Phase {
    /// `kEnableMainDMOBank` was false: no bank, no models, no spawns.
    Disabled,
    Loading(Box<SelectedDmoBankStartup>),
    Loaded(Box<SelectedDmoBankStartup>),
    Registered {
        /// The held bank collection, retained for the session.
        _bank: Box<SelectedDmoBankStartup>,
        vault: Arc<AttributeRuntimeVault>,
    },
}

pub(crate) struct SelectedLivingWorldDmo {
    phase: Phase,
}

/// One spawned DMO as its `cPresDynamicObject` draws it. `CreatePresDynamicObject`
/// 0x82811100 initializes the entity from the `tInstance` of its model
/// (`AddLODModelInstances` 0x827fa998 takes `tInstance::m_pRModel`), and
/// `cLivingWorldSimplePresEntity::Update` 0x82812168 hands every mesh's bone
/// slots the Part transform (`UpdatePartTransforms` 0x827efa10), which the
/// `dynamicobject` programs read as `i_partArray[0..3]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectedDynamicObjectVisual {
    /// `LivingWorldEntity` ID.
    pub entity_id: u32,
    /// The bank asset whose arena holds the model's `tInstance` and RModel.
    pub asset_id: u64,
    /// The RModel's dictionary entry in that arena (`tInstance +0x80`).
    pub render_model_entry: u32,
    /// Part 0's world transform after `UpdatePostTransforms` (the core
    /// `mTransform`): x, y and z axis rows, then the translation row.
    pub part_transform: [f32; 16],
    pub opacity_parameters: crate::LivingWorldOpacityParameters,
    pub opacity_settings: crate::LivingWorldOpacitySettings,
    /// DoSimSubmit virtual+78: Manager.GetSimulationCenterVelocity826f4e48.
    pub simulation_center_velocity: [f32; 4],
}

/// Every spawned DMO in `LWDynamicObjectMan` entity-map order. Empty until
/// the LivingWorld Manager publishes its entity manager.
pub(crate) fn dynamic_object_visuals(
    world: &PhysicalAssemblyWorld,
    actor: &ActorConstruction,
    vault: &Arc<AttributeRuntimeVault>,
    cvars: &InputCvarReferences,
) -> Result<Vec<SelectedDynamicObjectVisual>, SelectedLivingWorldDmoError> {
    // Reuse the actual Manager getter port; never substitute an object's speed.
    let (_, simulation_center_velocity) =
        local_skater_census_centre(actor).map_err(SelectedLivingWorldDmoError::World)?;
    let owner = Arc::clone(world.grind_services.living_world());
    let living_world = owner
        .lock()
        .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
    let Some(objects) = living_world.dmo_objects.as_deref() else {
        return Ok(Vec::new());
    };
    objects
        .ids()
        .into_iter()
        .filter_map(|id| objects.object(id))
        .map(|object| {
            Ok(SelectedDynamicObjectVisual {
                entity_id: object.entity_id,
                asset_id: object
                    .model
                    .asset
                    .plan()
                    .static_plan()
                    .asset()
                    .decoded
                    .asset_id,
                render_model_entry: object.model.render_model.entry_index(),
                part_transform: object.transform,
                opacity_parameters: crate::LivingWorldOpacityParameters::try_read(&vault.instance(
                    crate::living_world_presentation_opacity::MODELS_CLASS,
                    object.presentation_model_key,
                ))
                .map_err(SelectedLivingWorldDmoError::PresentationAttribute)?,
                opacity_settings: {
                    let settings = cvars
                        .try_living_world_opacity_settings()
                        .map_err(SelectedLivingWorldDmoError::Cvar)?;
                    // DmoPresentationEntity 0x825e90d0 draws a static online body unfaded.
                    crate::LivingWorldOpacitySettings { enabled: settings.enabled && !object.online_static, ..settings }
                },
                simulation_center_velocity,
            })
        })
        .collect()
}

/// Census cvars in `EvaluateCensus`/`EvaluativeSpawnDynamicObjects` order.
fn census_cvars(cvars: &InputCvarReferences) -> Result<DmoCensusCvars, InputCvarError> {
    Ok(DmoCensusCvars {
        enable_cull: cvars.try_enable_cull()?,
        enable_spawn: cvars.try_enable_spawn()?,
        enable_spawn_dynamic_objects: cvars.try_enable_spawn_dynamic_objects()?,
        one_component_per_frame: cvars.try_evaluate_census_for_one_component_per_frame()?,
        cull_all_components_per_frame: cvars.try_evaluate_cull_for_all_components_per_frame()?,
        max_dynamic_objects: cvars.try_max_dynamic_objects()?,
        close_by_locator_radius: cvars.try_close_by_locator_search_radius()?,
    })
}

/// `Manager::GetCameraPosition` 0x82703340 / `GetCameraHeading` 0x82703410:
/// the first camera's `BasicCamera` matrix translation and Z axis.
fn camera_lanes(
    local: &SkaterManagerLocalIdentity,
) -> Result<([f32; 4], [f32; 4]), SelectedLivingWorldDmoError> {
    let camera = local
        .try_simulation_camera_matrix()
        .map_err(SelectedLivingWorldDmoError::Camera)?;
    let columns = camera.columns();
    let lanes = |column: usize| {
        let vector = columns[column];
        [
            vector.xyz[0],
            vector.xyz[1],
            vector.xyz[2],
            vector.w.unwrap_or(0.0),
        ]
    };
    Ok((lanes(3), lanes(2)))
}

/// `SKATE2_TRACE_LIVING_WORLD_COUNTS=<updates>`: every that many `Update1`s, the living world's
/// population, the dynamic objects within 30 m of the census centre and the census switches.
fn trace_population(
    world: &PhysicalAssemblyWorld,
    inputs: Option<&DmoCensusInputs>,
) -> Result<(), SelectedLivingWorldDmoError> {
    static EVERY: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    static UPDATES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let Some(every) = *EVERY.get_or_init(|| {
        std::env::var("SKATE2_TRACE_LIVING_WORLD_COUNTS").ok().and_then(|n| n.parse().ok()).filter(|&n| n > 0)
    }) else {
        return Ok(());
    };
    let update = UPDATES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if update % every != 0 {
        return Ok(());
    }
    let living = world
        .grind_services
        .living_world()
        .lock()
        .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
    let objects = living.dmo_objects.as_deref();
    let near = match (objects, inputs) {
        (Some(objects), Some(inputs)) => objects
            .ids()
            .into_iter()
            .filter_map(|id| objects.object(id))
            .filter(|object| {
                let dx = object.transform[12] - inputs.center[0];
                let dz = object.transform[14] - inputs.center[2];
                dx * dx + dz * dz < 900.0
            })
            .count(),
        _ => 0,
    };
    eprintln!(
        "living world {update}: dmos={} near={near} locators={} vehicles={} pedestrians={} enable={:#04x} banned={}/{} dmo range={:016x}",
        objects.map_or(0, |objects| objects.count()),
        living.dmo_locators.as_deref().map_or(0, |locators| locators.locator_count()),
        living.grind_vehicle_entities.as_deref().map_or(0, |vehicles| vehicles.ids().len()),
        living.pedestrian_entities.as_deref().map_or(0, |pedestrians| pedestrians.ids().len()),
        living.census.enable,
        living.census.pedestrians_banned,
        living.census.vehicles_banned,
        living.census.dynamic_objects_range_key,
    );
    if std::env::var_os("SKATE2_DIAGNOSTIC_PROP_LOCATIONS").is_some()
        && let (Some(objects),Some(inputs))=(objects,inputs)
    {
        let mut nearby:Vec<_>=objects.ids().into_iter().filter_map(|id| {
            let object=objects.object(id)?;
            if object.grab_splines.is_empty() {return None;}
            let distance=(0..3).map(|i|(object.transform[12+i]-inputs.center[i]).powi(2)).sum::<f32>();
            Some((distance,id,object))
        }).collect();
        nearby.sort_by(|a,b|a.0.total_cmp(&b.0));
        for (distance,id,object) in nearby.into_iter().take(8) {
            eprintln!("prop location: object={id} locator={:016x} position={:?} distance={} flags={:x} grab_splines={}",
                object.locator_id,&object.transform[12..15],distance.sqrt(),object.flags,object.grab_splines.len());
        }
    }
    Ok(())
}

/// `CommonBackEnd::sGhostSystemEnabled` and `Simulation::IsNetworkGame`, read
/// through the same backend the selected Actor samples. The selected session
/// has no FrontEnd: gameplay runs with `FEBackEnd::mFEState` IN_GAME.
fn census_inputs(
    local: &SkaterManagerLocalIdentity,
    center: [f32; 4],
    velocity: [f32; 4],
    initial: bool,
) -> Result<DmoCensusInputs, SelectedLivingWorldDmoError> {
    let (camera_position, camera_heading) = camera_lanes(local)?;
    let backend = local.backend();
    let ghost = backend
        .try_ghost_enabled()
        .map_err(SelectedLivingWorldDmoError::Identity)?;
    let network_game = backend
        .try_is_network_game()
        .map_err(SelectedLivingWorldDmoError::Identity)?;
    Ok(DmoCensusInputs {
        center,
        velocity,
        camera_position,
        camera_heading,
        initial,
        in_game: true,
        ghost,
        network_game,
    })
}

/// One host address binding per bank arena, in collection order.
fn bank_source_bindings(
    assets: &[Arc<DmoBankProcessedArenaAsset>],
) -> Result<
    Vec<(
        Arc<SimulationNormalArenaPlan>,
        Arc<SimulationArenaXenonAddressBinding>,
    )>,
    SelectedLivingWorldDmoError,
> {
    let mut bindings = Vec::with_capacity(assets.len());
    for (ordinal, asset) in assets.iter().enumerate() {
        let region = u32::try_from(ordinal)
            .ok()
            .and_then(|ordinal| ordinal.checked_mul(ARENA_REGION_SIZE))
            .and_then(|offset| offset.checked_add(ARENA_REGION_BASE))
            .ok_or(SelectedLivingWorldDmoError::ArenaRegion { asset: ordinal })?;
        let region_end = region
            .checked_add(ARENA_REGION_SIZE)
            .ok_or(SelectedLivingWorldDmoError::ArenaRegion { asset: ordinal })?;
        let plan = asset.plan_owner();
        let descriptors = plan
            .static_plan()
            .asset()
            .decoded
            .asset_resource_descriptors;
        let mut cursor = region;
        let mut bases = [0_u32; 5];
        for (base, descriptor) in bases.iter_mut().zip(descriptors) {
            if descriptor.size == 0 {
                continue;
            }
            let alignment = descriptor.alignment.max(1);
            let aligned = cursor
                .checked_add(alignment - 1)
                .map(|value| value & !(alignment - 1))
                .ok_or(SelectedLivingWorldDmoError::ArenaRegion { asset: ordinal })?;
            let end = aligned
                .checked_add(descriptor.size)
                .filter(|end| *end <= region_end)
                .ok_or(SelectedLivingWorldDmoError::ArenaRegion { asset: ordinal })?;
            *base = aligned;
            cursor = end;
        }
        let binding = SimulationArenaXenonAddressBinding::try_new(Arc::clone(&plan), bases)
            .map_err(|source| SelectedLivingWorldDmoError::Binding {
                asset: ordinal,
                source,
            })?;
        bindings.push((plan, Arc::new(binding)));
    }
    Ok(bindings)
}

impl SelectedLivingWorldDmo {
    /// `LoadMainDMOBank` 0x82e0e628: nothing loads when `kEnableMainDMOBank`
    /// is false.
    pub(crate) fn try_new(
        mounts: Arc<MountSet>,
        cvars: &InputCvarReferences,
    ) -> Result<Self, SelectedLivingWorldDmoError> {
        let phase = if cvars
            .try_enable_main_dmo_bank()
            .map_err(SelectedLivingWorldDmoError::Cvar)?
        {
            Phase::Loading(Box::new(
                SelectedDmoBankStartup::try_new(mounts)
                    .map_err(SelectedLivingWorldDmoError::Bank)?,
            ))
        } else {
            Phase::Disabled
        };
        Ok(Self { phase })
    }

    /// One bank update; `Ok(true)` once the bank is held (or disabled).
    pub(crate) fn try_poll(&mut self) -> Result<bool, SelectedLivingWorldDmoError> {
        let bank = match &mut self.phase {
            Phase::Disabled | Phase::Loaded(_) | Phase::Registered { .. } => return Ok(true),
            Phase::Loading(bank) => bank,
        };
        if !bank.try_poll().map_err(SelectedLivingWorldDmoError::Bank)? {
            return Ok(false);
        }
        let Phase::Loading(bank) = std::mem::replace(&mut self.phase, Phase::Disabled) else {
            unreachable!("phase matched above");
        };
        self.phase = Phase::Loaded(bank);
        Ok(true)
    }

    /// The bank's `RegArena`: source routes for every bank arena on the
    /// World's DMO pool, then `ModelMan::AddDMOModel` for each kind-11
    /// instance in arena and operation order. Models whose Create routes are
    /// not ported (the `LWModel::Create` fallback, procedural parts, the
    /// sphere Definition) are left unregistered and reported; their locators
    /// then find no model, as `GetModel` misses do natively.
    pub(crate) fn try_register(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
    ) -> Result<(), SelectedLivingWorldDmoError> {
        let bank = match std::mem::replace(&mut self.phase, Phase::Disabled) {
            Phase::Disabled => return Ok(()),
            Phase::Loading(bank) => {
                self.phase = Phase::Loading(bank);
                return Err(SelectedLivingWorldDmoError::BankPending);
            }
            Phase::Loaded(bank) => bank,
            registered @ Phase::Registered { .. } => {
                self.phase = registered;
                return Err(SelectedLivingWorldDmoError::AlreadyRegistered);
            }
        };
        let assets = bank
            .assets()
            .ok_or(SelectedLivingWorldDmoError::BankPending)?;
        let bindings = bank_source_bindings(assets)?;
        let image = SimulationXenonAddressImage::try_new(
            SIMULATION_IMAGE_BASE,
            SIMULATION_IMAGE_BODIES,
            SIMULATION_IMAGE_JOINTS,
            SIMULATION_IMAGE_DRIVES,
        )
        .map_err(SelectedLivingWorldDmoError::Image)?;
        world
            .try_bind_living_world_sources(image, bindings)
            .map_err(SelectedLivingWorldDmoError::Source)?;
        let model_cvars = DmoModelCvars {
            enable_model_loading: cvars
                .try_enable_dmo_model_loading()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            enable_bank_id_lookup: cvars
                .try_enable_dmo_bank_id_lookup()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
        };
        let living_world = Arc::clone(world.grind_services.living_world());
        let mut living_world = living_world
            .lock()
            .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
        let mut unported = 0_usize;
        for asset in assets {
            for (operation_index, operation) in asset.plan().operations().iter().enumerate() {
                if !matches!(operation, SimulationNormalOperation::DmoInstance { .. }) {
                    continue;
                }
                let source = DmoModelSource {
                    asset,
                    operation_index,
                };
                match living_world.dmo_models.try_add_dmo_model(
                    vault,
                    &source,
                    model_cvars,
                    &mut HostReckoningNumeric,
                ) {
                    Ok(()) => {}
                    Err(
                        DmoModelError::UnportedFallback { .. }
                        | DmoModelError::UnportedProceduralParts { .. }
                        | DmoModelError::UnportedSphereDefinition { .. },
                    ) => unported += 1,
                    Err(error) => return Err(SelectedLivingWorldDmoError::Model(error)),
                }
            }
        }
        if !crate::diagnostics::is_quiet() {
            eprintln!(
                "LivingWorld DMO models: {} registered from {} bank arenas, {unported} with unported Create routes",
                living_world.dmo_models.dmo_model_count(),
                assets.len(),
            );
        }
        drop(living_world);
        self.phase = Phase::Registered {
            _bank: bank,
            vault: Arc::clone(vault),
        };
        Ok(())
    }

    fn registered_vault(&self) -> Option<&Arc<AttributeRuntimeVault>> {
        match &self.phase {
            Phase::Registered { vault, .. } => Some(vault),
            _ => None,
        }
    }

    /// Runs `step` over the census frame of the World's LivingWorld Manager.
    fn try_with_frame<T>(
        world: &mut PhysicalAssemblyWorld,
        vault: &Arc<AttributeRuntimeVault>,
        actor: &ActorConstruction,
        cvars: &InputCvarReferences,
        recipes: Option<&crate::selected_living_world_recipes::SelectedRecipeFrame<'_>>,
        step: impl FnOnce(
            &mut DmoFrame<
                '_,
                LivingWorldDmoWorld<'_, crate::skateboard_hook::SkateboardHookDefinition>,
            >,
        ) -> Result<T, SelectedLivingWorldDmoError>,
    ) -> Result<T, SelectedLivingWorldDmoError> {
        let actors=world.entity_roster.as_ref().ok_or(SelectedLivingWorldDmoError::MissingManagers)?
            .snapshot().map_err(|e|SelectedLivingWorldDmoError::Pedestrians(format!("DMO actor roster: {e:?}")))?;
        let painter=recipes.map(|r|r.painter.lock().map_err(|_|SelectedLivingWorldDmoError::Pedestrians("DMO painter lock".into()))).transpose()?;
        let avoid=recipes.map(|r|{
            let triggers=r.triggers.lock().map_err(|_|"DMO triggers lock".to_string())?;
            r.runtime.challenge_avoid_volumes.resolve(&triggers)
        }).transpose().map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        let owner: Arc<Mutex<LivingWorldGrindServices>> =
            Arc::clone(world.grind_services.living_world());
        let mut guard = owner
            .lock()
            .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
        let living_world: &mut LivingWorldGrindServices = &mut guard;
        let objects = living_world
            .dmo_objects
            .as_deref_mut()
            .ok_or(SelectedLivingWorldDmoError::MissingManagers)?;
        let locators = living_world
            .dmo_locators
            .as_deref_mut()
            .ok_or(SelectedLivingWorldDmoError::MissingManagers)?;
        let skaters = [actor];
        let mut dmo_world = LivingWorldDmoWorld {
            prop_recipes:recipes.map(|recipes|recipes.prop_recipe_frame()),
            census_targets:&mut living_world.census_targets,
            frame_owners:crate::pedestrian_graph::backend::frame::FrameOwners{
                pedestrians:living_world.pedestrian_entities.as_deref_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?,
                mood:living_world.mood.as_ref().ok_or(SelectedLivingWorldDmoError::MissingManagers)?,
                game_sim_events:living_world.game_sim_events.as_deref().ok_or(SelectedLivingWorldDmoError::MissingManagers)?,
                props:living_world.props.as_deref_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?,
                mesh:living_world.mesh.as_ref().ok_or(SelectedLivingWorldDmoError::MissingManagers)?,
                rain:living_world.rain.as_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?,
                chase:living_world.chase.as_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?,
                vehicles:living_world.grind_vehicle_entities.as_deref().ok_or(SelectedLivingWorldDmoError::MissingManagers)?,
                big_black_entity:&living_world.big_black_entity,
            },
            random:living_world.random.as_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?,
            actors:&actors,painter:painter.as_deref(),avoid:avoid.as_ref(),vault,
            world,
            skaters: &skaters,
            cvars,
        };
        let mut frame = DmoFrame {
            census: &mut living_world.census,
            entity_ids: &mut living_world.entity_ids,
            spline_ids: &mut living_world.spline_ids,
            vault,
            models: &living_world.dmo_models,
            locators,
            objects,
            world: &mut dmo_world,
        };
        step(&mut frame)
    }

    /// `Dmo::SimManager::SpawnStatic` (`0x825eb0a0`), which an online game's
    /// `Simulation::Reset` (`0x8289eec8`) runs with the ghost system on: a static body for every
    /// record of the locator groups `RegDMOArena` (`0x828a0420`) gave `RegDmoLocator`, whose
    /// position (the matrix's translation row) passes `TestForNetworkGameCollisionConstraints`
    /// (`0x8289e510`: the game's `InvisibleBoundaryManager::IsPositionOk` at radius 0) and whose
    /// model is a registered Assembly of 1 to 26 Parts, until 50 are out. `kbEnableOnlineDmos`
    /// keeps its registered true. The objects made, in record order.
    pub(crate) fn try_spawn_online_dmos(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        actor: &ActorConstruction,
        cvars: &InputCvarReferences,
        recipes: &crate::selected_living_world_recipes::SelectedRecipeFrame<'_>,
        records: &[skate2_formats::pegasus::Dmo],
        boundaries: &crate::challenge::invisible_boundary::InvisibleBoundaries,
    ) -> Result<Vec<u32>, SelectedLivingWorldDmoError> {
        let Some(vault) = self.registered_vault().map(Arc::clone) else {
            return Ok(Vec::new());
        };
        Self::try_with_frame(world, &vault, actor, cvars, Some(recipes), |frame| {
            let mut spawned = Vec::new();
            for record in records {
                let position = [record.transform[12], record.transform[13], record.transform[14]];
                if !boundaries.is_position_ok(position, 0.0) {
                    continue;
                }
                let Some(model) = frame.models.model(record.instance_guid).map(Arc::clone) else {
                    continue;
                };
                if !(1..=SPAWN_STATIC_MAX_PARTS).contains(&model.world_collision.parts().len()) {
                    continue;
                }
                match frame
                    .try_spawn_static(&model, &record.transform)
                    .map_err(SelectedLivingWorldDmoError::Census)?
                {
                    Some(id) => spawned.push(id),
                    None => break,
                }
            }
            Ok(spawned)
        })
    }

    /// `Dmo::SimManager::UnspawnAll` (`0x825e9628`): the online game's static bodies go, each
    /// object's explicit cull destroyed at once.
    pub(crate) fn try_unspawn_online_dmos(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        actor: &ActorConstruction,
        cvars: &InputCvarReferences,
        recipes: &crate::selected_living_world_recipes::SelectedRecipeFrame<'_>,
        objects: &[u32],
    ) -> Result<(), SelectedLivingWorldDmoError> {
        let Some(vault) = self.registered_vault().map(Arc::clone) else {
            return Ok(());
        };
        Self::try_with_frame(world, &vault, actor, cvars, Some(recipes), |frame| {
            for &id in objects {
                if frame.objects.object(id).is_some_and(|object| object.online_static) {
                    frame.try_request_explicit_cull(id).map_err(SelectedLivingWorldDmoError::Census)?;
                }
            }
            frame.try_process_cull_map().map_err(SelectedLivingWorldDmoError::Census)
        })
    }

    /// Whether every one of the online game's static objects is still out; a world without the
    /// DMO manager holds none.
    pub(crate) fn online_dmos_present(world: &PhysicalAssemblyWorld, objects: &[u32]) -> Result<bool, SelectedLivingWorldDmoError> {
        let owner = Arc::clone(world.grind_services.living_world());
        let living = owner.lock().map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
        Ok(living.dmo_objects.as_deref().map_or(objects.is_empty(), |dmos| {
            objects.iter().all(|&id| dmos.object(id).is_some_and(|object| object.online_static))
        }))
    }

    /// `CharacterPrepareState` exit 0x82760170..8201c: the census at the
    /// stream focus with zero velocity and `bInitialSpawn`, between the
    /// spatial lookup clear and rebuild. The challenge LivingWorld manager's
    /// skip byte and the challenge character spot test are offline no-ops.
    pub(crate) fn try_initial_census(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        actor: &ActorConstruction,
        local: &SkaterManagerLocalIdentity,
        cvars: &InputCvarReferences,
        stream_focus: [f32; 3],
        recipes: &mut crate::selected_living_world_recipes::SelectedRecipeFrame<'_>,
        ground: &crate::static_ground_pool::StaticGroundPool,
        mut simulation_random: Option<&mut crate::native_skater_random::SimulationCommandRandom>,
    ) -> Result<(), SelectedLivingWorldDmoError> {
        let Some(vault) = self.registered_vault().map(Arc::clone) else {
            return Ok(());
        };
        // Initial loading includes the ChaseMan part of Manager.Reset8271d430
        // before its first census. Subsequent frame censuses do not reset it.
        {
            let owner = Arc::clone(world.grind_services.living_world());
            let mut living = owner.lock().map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
            living.chase.as_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?
                .reset(&vault).map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        }
        let census = census_cvars(cvars).map_err(SelectedLivingWorldDmoError::Cvar)?;
        let center = [stream_focus[0], stream_focus[1], stream_focus[2], 0.0];
        let inputs = census_inputs(local, center, [0.0; 4], true)?;
        Self::try_with_frame(world, &vault, actor, cvars, Some(&*recipes), |frame| {
            // Selected synchronous boot omits the original loading frames.
            // Publish this derived collision cache from completed initial
            // PhysOut before any census entity can request a reset. Reuse
            // the exact UpdateTransforms producer; no extra simulation tick.
            crate::living_world_grind_services::dmo_objects::DmoWorld::try_update_reset_capsules(
                frame.world,frame.objects).map_err(SelectedLivingWorldDmoError::World)?;
            Ok(())
        })?;
        Self::try_spatial_lookup_phase(world, true)?;
        Self::try_evaluate_population(
            world,
            actor,
            cvars,
            &vault,
            &inputs,
            &census,
            Some(&mut *recipes),
            ground,simulation_random.as_deref_mut(),
        )?;
        Self::try_spatial_lookup_phase(world, false)
    }

    /// Manager.ClearSpatialLookups826f5320 / UpdateSpatialLookups826f5398.
    /// Both visit pedestrian, vehicle, prop, DMO, in that order. Removal only
    /// detaches map membership; this scheduled reset recycles the KD nodes.
    /// Boot separates the two phases around its initial census; Update1 runs
    /// them consecutively after census processing (82728354..82728360).
    fn try_spatial_lookup_phase(
        world: &PhysicalAssemblyWorld,
        clear: bool,
    ) -> Result<(), SelectedLivingWorldDmoError> {
        use SelectedLivingWorldDmoError as E;
        let owner = Arc::clone(world.grind_services.living_world());
        let mut living = owner.lock().map_err(|_| E::LivingWorldLock)?;
        let pedestrians = living.pedestrian_entities.as_mut().ok_or(E::MissingManagers)?;
        if clear { pedestrians.clear_spatial_lookups() }
        else { pedestrians.update_spatial_lookups() }
            .map_err(|e| E::Pedestrians(format!("pedestrian spatial lookup: {e:?}")))?;
        let vehicles = living.grind_vehicle_entities.as_mut().ok_or(E::MissingManagers)?;
        if clear { vehicles.clear_spatial_lookups() }
        else { vehicles.update_spatial_lookups() }
            .map_err(|e| E::Vehicles(crate::living_world_grind_services::vehicle_entity::frame::error(e)))?;
        let props = &mut living.props.as_mut().ok_or(E::MissingManagers)?.entities;
        if clear { props.clear_spatial_lookups() }
        else { props.update_spatial_lookups() }
            .map_err(|e| E::Pedestrians(format!("prop spatial lookup: {e:?}")))?;
        let dmos = living.dmo_objects.as_mut().ok_or(E::MissingManagers)?;
        if clear { dmos.clear_spatial_lookups(); }
        else { dmos.update_spatial_lookups(); }
        Ok(())
    }

    /// CensusMan.Update82702780: settings, then pedestrian/vehicle/prop/DMO
    /// destruction. Population evaluation must drain these before replacements.
    fn try_update_census(
        world: &mut PhysicalAssemblyWorld,
        actor: &ActorConstruction,
        cvars: &InputCvarReferences,
        vault: &Arc<AttributeRuntimeVault>,
        recipes: Option<&crate::selected_living_world_recipes::SelectedRecipeFrame<'_>>,
    ) -> Result<(), SelectedLivingWorldDmoError> {
        let local = world.local_skater_identity.as_ref()
            .ok_or(SelectedLivingWorldDmoError::MissingManagers)?;
        let mode = local.backend().try_game_mode()
            .map_err(SelectedLivingWorldDmoError::Identity)?;
        // ProfileFreeSkate (3) takes GameInfo's pedestrian and traffic settings; Party Play (2)
        // and online games (4 and 5) have no pedestrians and no traffic.
        let sliders = if mode == 3 {
            let (traffic, pedestrians, _) = local.backend().try_free_skate()
                .map_err(SelectedLivingWorldDmoError::Identity)?;
            Some([pedestrians, traffic])
        } else {
            None
        };
        let owner = Arc::clone(world.grind_services.living_world());
        {
            let mut living = owner.lock()
                .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
            let banned = matches!(mode, 2 | 4 | 5);
            living.census.update_settings(sliders, banned, banned);
        }
        Self::try_process_entity_culls(world, actor, cvars, vault, recipes)
    }

    /// The four EntityCensusMan.Update calls occur both inside Census.Update
    /// (8270288c..28c8) and after EvaluateCensus (82728304..8340). The second
    /// pass releases newly detached entities without advancing census settings.
    fn try_process_entity_culls(
        world: &mut PhysicalAssemblyWorld,
        actor: &ActorConstruction,
        cvars: &InputCvarReferences,
        vault: &Arc<AttributeRuntimeVault>,
        recipes: Option<&crate::selected_living_world_recipes::SelectedRecipeFrame<'_>>,
    ) -> Result<(), SelectedLivingWorldDmoError> {
        let owner = Arc::clone(world.grind_services.living_world());
        {
            let mut living = owner.lock()
                .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
            living.process_pedestrian_culls(world)
                .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
            living.process_vehicle_culls(world)
                .map_err(SelectedLivingWorldDmoError::Vehicles)?;
            if let Some(recipes) = recipes {
                recipes.prop_recipe_frame().process_culls(&mut living)
                    .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
            }
        }
        Self::try_with_frame(world, vault, actor, cvars, recipes, |frame| {
            frame.try_process_cull_map().map_err(SelectedLivingWorldDmoError::Census)
        })
    }

    fn try_evaluate_population(
        world: &mut PhysicalAssemblyWorld,
        actor: &ActorConstruction,
        cvars: &InputCvarReferences,
        vault: &Arc<AttributeRuntimeVault>,
        inputs: &DmoCensusInputs,
        census: &DmoCensusCvars,
        mut recipes: Option<&mut crate::selected_living_world_recipes::SelectedRecipeFrame<'_>>,
        ground: &crate::static_ground_pool::StaticGroundPool,
        mut simulation_random: Option<&mut crate::native_skater_random::SimulationCommandRandom>,
    ) -> Result<(), SelectedLivingWorldDmoError> {
        let living_owner = Arc::clone(world.grind_services.living_world());
        let needs_update = {
            let living = living_owner.lock()
                .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
            living.census.enable & 0x80 != 0 && !living.census.sampled_fe
        };
        if needs_update {
            Self::try_update_census(world, actor, cvars, vault, recipes.as_deref())?;
        }
        let plan = {
            let living = living_owner
                .lock()
                .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
            crate::living_world_grind_services::dmo_objects::census_plan::<DmoWorldError>(
                &living.census,
                inputs,
                &census,
                &vault,
            )
            .map_err(SelectedLivingWorldDmoError::Census)?
        };
        if let Some(plan) = plan {
            if plan.cull_dmos {
                Self::try_with_frame(world, &vault, actor, cvars, recipes.as_deref(), |frame| {
                    frame
                        .try_evaluate_dynamic_object_census(inputs, &census, true, false)
                        .map_err(SelectedLivingWorldDmoError::Census)
                })?;
            }
            if plan.cull_pedestrians {
                if let Some(recipes) = recipes.as_deref_mut() {
                    recipes.evaluate_pedestrians(world,cvars,inputs,true,false,ground,simulation_random.as_deref_mut())
                        .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
                }
            }
            if plan.cull_vehicles {
                if let Some(recipes) = recipes.as_deref_mut() {
                    recipes
                        .evaluate_vehicles(world, cvars, inputs, true, false)
                        .map_err(SelectedLivingWorldDmoError::Vehicles)?;
                }
            }
            if plan.cull_props {
                if let Some(recipes) = recipes.as_deref_mut() {
                    recipes.evaluate_props(world, cvars, inputs)
                        .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
                }
            }
            // All requested culls complete before any new category spawn.
            Self::try_update_census(world, actor, cvars, vault, recipes.as_deref())?;
            if plan.spawn_dmos {
                Self::try_with_frame(world, &vault, actor, cvars, recipes.as_deref(), |frame| {
                    frame
                        .try_evaluate_dynamic_object_census(inputs, &census, false, true)
                        .map_err(SelectedLivingWorldDmoError::Census)
                })?;
            }
            if plan.spawn_pedestrians {
                if let Some(recipes) = recipes.as_deref_mut() {
                    recipes.evaluate_pedestrians(world,cvars,inputs,false,true,ground,simulation_random.as_deref_mut())
                        .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
                }
            }
            if plan.spawn_vehicles {
                if let Some(recipes) = recipes.as_deref_mut() {
                    recipes
                        .evaluate_vehicles(world, cvars, inputs, false, true)
                        .map_err(SelectedLivingWorldDmoError::Vehicles)?;
                }
            }
            if plan.advance {
                let mut living = living_owner
                    .lock()
                    .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
                living.census.component_index = (living.census.component_index + 1) % 4;
            }
        }

        Ok(())
    }
    /// `Manager::Update1` 0x82727ee0's DMO subsequence, then `World::
    /// BeginDMOUpdate`'s mesh collection rebuild: the pedestrian, vehicle and
    /// `LWDynamicObjectMan::ResetCollisionFlags` (0x827281e8..0x827281f8), `CensusMan::
    /// Update` (0x82728234; it drains every census cull map), the local
    /// skater's `EvaluateCensus(centre, velocity, false)` (0x827282c0..8300),
    /// the DMO census `Update` (0x82728340) and the spatial lookup rebuild
    /// (0x82728354..8360).
    pub(crate) fn try_update1(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        actor: &ActorConstruction,
        local: &SkaterManagerLocalIdentity,
        cvars: &InputCvarReferences,
        mut recipes: Option<&mut crate::selected_living_world_recipes::SelectedRecipeFrame<'_>>,
        ground: &crate::static_ground_pool::StaticGroundPool,
        mut simulation_random: Option<&mut crate::native_skater_random::SimulationCommandRandom>,
    ) -> Result<(), SelectedLivingWorldDmoError> {
        // No pedestrian keeps a nav tile the MeshStore removed since the last frame.
        world.grind_services.living_world().lock().map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?
            .release_removed_nav_tiles().map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        let Some(vault) = self.registered_vault().map(Arc::clone) else {
            return Ok(());
        };
        let census = census_cvars(cvars).map_err(SelectedLivingWorldDmoError::Cvar)?;
        let evaluate =
            local_skater_census_gate(actor).map_err(SelectedLivingWorldDmoError::World)?;
        let inputs = if evaluate {
            let (center, velocity) =
                local_skater_census_centre(actor).map_err(SelectedLivingWorldDmoError::World)?;
            Some(census_inputs(local, center, velocity, false)?)
        } else {
            None
        };
        {
            let mut living = world
                .grind_services
                .living_world()
                .lock()
                .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
            living.pedestrian_reset_collision_flags();
            living
                .vehicle_reset_collision_flags()
                .map_err(SelectedLivingWorldDmoError::Vehicles)?;
            living
                .dmo_objects
                .as_mut()
                .ok_or(SelectedLivingWorldDmoError::MissingManagers)?
                .reset_collision_flags();
            // Manager.Update1: ModelMan.Update82728204 follows collision reset.
            living.dmo_models.update();
        }
        // Manager.Update1: WaypointMan.Update82ce8e00 between ModelMan and RAIN.
        if let Some(recipes) = recipes.as_deref() {
            recipes
                .update_spectators(world, cvars, ground, simulation_random.as_deref_mut())
                .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        }
        {
            let mut living = world
                .grind_services
                .living_world()
                .lock()
                .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
            // Manager.Update1: RAIN after ModelMan, before census update.
            let rain = living
                .rain
                .as_mut()
                .ok_or(SelectedLivingWorldDmoError::MissingManagers)?;
            if let Some(assets) = rain.assets.clone() {
                rain.publish_pending_zone(&assets)
                    .map_err(SelectedLivingWorldDmoError::Rain)?;
            }
            rain.update_traffic(UPDATE_TIME)
                .map_err(SelectedLivingWorldDmoError::Traffic)?;
            // This selected session is local; pass the actual census identity
            // when available, preserving RAIN's network early return.
            living
                .vehicle_skater_road_points(
                    actor,
                    cvars,
                    inputs.as_ref().is_some_and(|i| i.network_game),
                )
                .map_err(SelectedLivingWorldDmoError::Vehicles)?;
            living
                .vehicle_entity_road_points(&vault, inputs.as_ref().is_some_and(|i| i.network_game))
                .map_err(SelectedLivingWorldDmoError::Vehicles)?;
        }
        Self::try_update_census(world, actor, cvars, &vault, recipes.as_deref())?;
        // Manager.Update1 82728234..250: pending hand props after Census.Update,
        // before population evaluation. Freeroam has no main challenge (gate true).
        Self::try_evaluate_hand_props(world,&vault,actor,cvars,&census,recipes.as_deref())?;
        if let Some(inputs) = &inputs {
            Self::try_evaluate_population(world, actor, cvars, &vault, inputs, &census, recipes.as_deref_mut(),ground,simulation_random.as_deref_mut())?;
        }
        Self::try_process_entity_culls(world, actor, cvars, &vault, recipes.as_deref())?;
        Self::try_spatial_lookup_phase(world, true)?;
        Self::try_spatial_lookup_phase(world, false)?;
        trace_population(world, inputs.as_ref())?;
        let ignore_volume_enable = cvars
            .try_ignore_volume_enable_flag()
            .map_err(SelectedLivingWorldDmoError::Cvar)?;
        world
            .try_begin_dmo_update(ignore_volume_enable)
            .map_err(SelectedLivingWorldDmoError::World)?;
        let settings = DmoFrameSettings {
            run_pedestrian_ground: cvars.try_run_pedestrian_ground_collision_pipeline()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            draw_pedestrian_ground_contacts: cvars.try_draw_pedestrian_ground_contacts()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            run_skater_pedestrian: cvars.try_run_skater_pedestrian_collision_pipeline()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            run_dmo_pedestrian: cvars.try_run_dmo_pedestrian_collision_pipeline()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            run_pedestrian_pedestrian: cvars.try_run_pedestrian_pedestrian_collision_pipeline()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            run_pedestrian_vehicle: cvars.try_run_pedestrian_vehicle_collision_pipeline()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            run_ground: cvars
                .try_run_dmo_ground_collision_pipeline()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            draw_ground_contacts: cvars
                .try_draw_dmo_ground_contacts()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            run_skater_dmo: cvars
                .try_run_skater_dmo_collision_pipeline()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            run_dmo_dmo: cvars
                .try_run_dmo_dmo_collision_pipeline()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            run_dmo_vehicle: cvars
                .try_run_dmo_vehicle_collision_pipeline()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
            run_skater_vehicle: cvars
                .try_run_skater_vehicle_collision_pipeline()
                .map_err(SelectedLivingWorldDmoError::Cvar)?,
        };
        world.schedule_dmo_frame(settings);
        Ok(())
    }

    pub(crate) fn try_update2(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        actor: &ActorConstruction,
        cvars: &InputCvarReferences,
        ground: &crate::static_ground_pool::StaticGroundPool,
        recipes: Option<&mut crate::selected_living_world_recipes::SelectedRecipeFrame<'_>>,
        mut random: Option<&mut crate::native_skater_random::SimulationCommandRandom>,
        ai_respawn: &mut crate::actor_construction::breadcrumbs::ActorAiRespawnQuery<'_>,
    ) -> Result<(), SelectedLivingWorldDmoError> {
        if self.registered_vault().is_none() {
            return Ok(());
        }
        let (center, _) =
            local_skater_census_centre(actor).map_err(SelectedLivingWorldDmoError::World)?;
        let recipes=recipes.ok_or_else(|| SelectedLivingWorldDmoError::Pedestrians("pedestrian execution session absent".into()))?;
        recipes.update_pedestrian_los_states(world, cvars, center)
            .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        let owner = Arc::clone(world.grind_services.living_world());
        {
            let mut living=owner.lock().map_err(|_|SelectedLivingWorldDmoError::LivingWorldLock)?;
            living.vehicle_update_los_states(center).map_err(SelectedLivingWorldDmoError::Vehicles)?;
            recipes.prop_recipe_frame().update_los_states(&mut living, center)
                .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
            living.dynamic_object_update_los_states(center,self.registered_vault().expect("checked above"))
                .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        }
        recipes.update_chase_manager(world,cvars,UPDATE_TIME,ground,ai_respawn)
            .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        {
            let mut living=owner.lock().map_err(|_|SelectedLivingWorldDmoError::LivingWorldLock)?;
            living.pedestrian_execution.advance();
            living.vehicle_execution.advance();
            living.props.as_deref_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?.execution.advance();
            living.dmo_objects.as_deref_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?.execution.advance();
            living.mesh.as_ref().ok_or(SelectedLivingWorldDmoError::MissingManagers)?
                .frame_begin().map_err(|e|SelectedLivingWorldDmoError::Pedestrians(format!("MeshMan FrameBegin: {e:?}")))?;
            let actors=world.entity_roster.as_ref().ok_or(SelectedLivingWorldDmoError::MissingManagers)?
                .snapshot().map_err(|e|SelectedLivingWorldDmoError::Pedestrians(format!("audibles Actor roster: {e:?}")))?;
            living.update_pedestrian_audibles(&actors,&mut HostReckoningNumeric)
                .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        }
        recipes.update_pedestrian_moodables(world,cvars,UPDATE_TIME)
            .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        recipes.evaluate_transferable_behaviours(world,cvars,ground,random.as_deref_mut())
            .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        recipes.update_pedestrian_current_frame(world,cvars,center)
            .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        let local_index=world.local_skater_identity.as_ref().ok_or(SelectedLivingWorldDmoError::MissingManagers)?
            .try_local_index().map_err(SelectedLivingWorldDmoError::Identity)?;
        recipes.update_pedestrian_preframe(world,cvars,ground,center,local_index,random.as_deref_mut())
            .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        let mut living = owner
            .lock()
            .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?;
        living
            .vehicle_update_preframe(center, actor, cvars)
            .map_err(SelectedLivingWorldDmoError::Vehicles)?;
        {
            let painter=recipes.painter.lock().map_err(|_|SelectedLivingWorldDmoError::Pedestrians("DMO preframe painter lock".into()))?;
            recipes.update_dmo_preframe(&mut living,world,center,cvars,&painter,actor)
                .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        }
        let local_position={
            let physical=actor.cached_physical_allocation().map_err(|e|SelectedLivingWorldDmoError::Pedestrians(format!("Prop Actor physical: {e:?}")))?;
            let output=physical.owner().phys_out().lock().map_err(|_|SelectedLivingWorldDmoError::Pedestrians("Prop Actor PhysOut lock".into()))?;
            // Actor.GetPosition825f9df0 reads PhysOut+416 (root translation).
            // The host Prop core is a geometric projection: distance uses
            // dot3 and tracker queries use XYZ/XZ. SIMD W is not a position
            // requirement. Retain unknown W in PhysOut; project only this
            // host core input, as in the pedestrian avoidance volume adapter.
            let root = output.try_skeleton_output().map_err(|e|SelectedLivingWorldDmoError::Pedestrians(format!("Prop Actor root: {e:?}")))?
                .skeleton_root_matrix.xyz_scratch();
            [root[12], root[13], root[14], root[15]]
        };
        recipes.prop_recipe_frame().update_preframe(&mut living,center,Some(local_position))
            .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        living
            .vehicle_update_targets(cvars, UPDATE_TIME)
            .map_err(SelectedLivingWorldDmoError::Vehicles)?;
        living
            .vehicle_ground_tests(world, ground)
            .map_err(SelectedLivingWorldDmoError::Vehicles)?;
        living
            .vehicle_reset_skitch_flags()
            .map_err(SelectedLivingWorldDmoError::Vehicles)
    }

    /// Manager.Update3's AnimMan prefix82726e80, after World.Update1.
    pub(crate) fn try_update_animation(
        &mut self, world:&mut PhysicalAssemblyWorld, actor:&ActorConstruction,
        local:&SkaterManagerLocalIdentity, cvars:&InputCvarReferences,
        recipes:Option<&mut crate::selected_living_world_recipes::SelectedRecipeFrame<'_>>,
        tokens:&mut crate::actor_face_playback::blend_pose_stream::BlendPoseTokenPool,
    )->Result<(),SelectedLivingWorldDmoError>{
        if self.registered_vault().is_none(){return Ok(());}
        let (center,_) = local_skater_census_centre(actor).map_err(SelectedLivingWorldDmoError::World)?;
        let (camera,_) = camera_lanes(local)?;
        recipes.ok_or_else(||SelectedLivingWorldDmoError::Pedestrians("pedestrian animation session absent".into()))?
            .update_pedestrian_animation(world,cvars,center,camera,tokens)
            .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        // Manager.Update3 -> UpdateRenderStates826f5488. Pedestrian's actual
        // vtable retains LWBasicEntity82d1ced8: SetIsRenderable(true), with no
        // camera/frustum branch. This follows AnimMan's token evaluation.
        let owner=world.grind_services.living_world();
        let mut living=owner.lock().map_err(|_|SelectedLivingWorldDmoError::LivingWorldLock)?;
        let roster=living.pedestrian_entities.as_deref_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?;
        for id in roster.ids(){roster.get_mut(id).expect("captured render-state roster").renderable=true;}
        Ok(())
    }

    /// `World::Update3` 0x827af8f0 resolves the simulation Island's contact
    /// spies (`ResolveContactSpyList` 0x827b3a90, called at 0x827afa58) after
    /// `ResetCollisionFlags` and before `UpdateSyncAnim`, whose
    /// `UpdateTargetTransform` 0x82e7f660 builds this frame's SimLOD bones from
    /// the adjusted target transforms.
    pub(crate) fn try_resolve_contact_spy_lists(
        &self,
        world: &mut PhysicalAssemblyWorld,
        local: &SkaterManagerLocalIdentity,
        cvars: &InputCvarReferences,
    ) -> Result<(), SelectedLivingWorldDmoError> {
        if self.registered_vault().is_none() {
            return Ok(());
        }
        let contact_owner = Arc::clone(world.grind_services.living_world());
        contact_owner
            .lock()
            .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?
            .vehicle_resolve_contacts(world)
            .map_err(SelectedLivingWorldDmoError::Vehicles)?;
        contact_owner
            .lock()
            .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?
            .pedestrian_resolve_contacts(world, local, cvars)
            .map_err(SelectedLivingWorldDmoError::PedestrianContacts)?;
        contact_owner
            .lock()
            .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?
            .resolve_dmo_scoring_contacts(world)
            .map_err(SelectedLivingWorldDmoError::ScoringContacts)
    }

    /// SimUpdate2 8289fa34: the ACS fence precedes pedestrian target transforms.
    pub(crate) fn try_sync_animation(
        &mut self,world:&mut PhysicalAssemblyWorld,cvars:&InputCvarReferences,
        recipes:Option<&mut crate::selected_living_world_recipes::SelectedRecipeFrame<'_>>,
    )->Result<(),SelectedLivingWorldDmoError>{
        if self.registered_vault().is_none(){return Ok(());}
        recipes.ok_or_else(||SelectedLivingWorldDmoError::Pedestrians("pedestrian sync session absent".into()))?
            .sync_pedestrian_animation(world,cvars)
            .map_err(SelectedLivingWorldDmoError::Pedestrians)
    }

    /// `Manager::UpdatePostTransforms` 0x827031e8, DMO portion, after the
    /// World's `Update3` and before the camera update; the census inputs
    /// read the post-physics skater and the previous frame's camera.
    pub(crate) fn try_update_post_transforms(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        actor: &ActorConstruction,
        local: &SkaterManagerLocalIdentity,
        cvars: &InputCvarReferences,
        recipes: Option<&crate::selected_living_world_recipes::SelectedRecipeFrame<'_>>,
    ) -> Result<(), SelectedLivingWorldDmoError> {
        let Some(vault) = self.registered_vault().map(Arc::clone) else {
            return Ok(());
        };
        let (center, velocity) =
            local_skater_census_centre(actor).map_err(SelectedLivingWorldDmoError::World)?;
        let inputs = census_inputs(local, center, velocity, false)?;
        // Manager.UpdatePostTransforms82703210/21c: DMO target/cull prefix.
        Self::try_with_frame(world, &vault, actor, cvars, recipes, |frame| {
            frame.try_update_post_targets(UPDATE_TIME).map_err(SelectedLivingWorldDmoError::Census)
        })?;
        let contact_owner = Arc::clone(world.grind_services.living_world());
        {
            let mut living=contact_owner.lock().map_err(|_|SelectedLivingWorldDmoError::LivingWorldLock)?;
            let living=&mut *living;
            let roster=living.pedestrian_entities.as_deref_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?;
            for id in roster.ids(){
                roster.get_mut(id).expect("captured collision roster")
                    .update_collision_models(world,UPDATE_TIME,cvars,living.props.as_deref())
                    .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
            }
        }
        // Native collision order is Pedestrian, Vehicle, then DMO, before heads.
        contact_owner.lock().map_err(|_|SelectedLivingWorldDmoError::LivingWorldLock)?
            .vehicle_update_collisions(world,cvars,UPDATE_TIME).map_err(SelectedLivingWorldDmoError::Vehicles)?;
        {
            let living=contact_owner.lock().map_err(|_|SelectedLivingWorldDmoError::LivingWorldLock)?;
            let objects=living.dmo_objects.as_deref().ok_or(SelectedLivingWorldDmoError::MissingManagers)?;
            for id in objects.ids(){
                let object=objects.object(id).expect("captured DMO collision roster");
                // FillCollisionTargetTransforms82d36610 emits exactly one target
                // for HandProp; free physics objects emit none and are not driven.
                if !object.is_set_as_hand_prop(){continue;}
                world.try_drive_living_world_collision_body(
                    crate::simulation_assembly_backend::constraint_session::physical_island::vehicle::LivingWorldBodyKind::Dmo,
                    &object.body,&[object.target_transform],UPDATE_TIME,object.collision_ignore_gravity,cvars,
                    &mut HostReckoningNumeric,&mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric)
                    .map_err(|e|SelectedLivingWorldDmoError::Pedestrians(format!("hand prop collision drive: {e:?}")))?;
            }
        }
        if let Some(recipes)=recipes{
            recipes.update_pedestrian_head_tracking(world,cvars,actor,UPDATE_TIME)
                .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
            recipes.update_pedestrian_post_adjust(world,cvars,UPDATE_TIME)
                .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        }
        let owner = Arc::clone(world.grind_services.living_world());
        owner
            .lock()
            .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?
            .vehicle_post_transforms(world, cvars, UPDATE_TIME)
            .map_err(SelectedLivingWorldDmoError::Vehicles)?;
        // Manager.UpdateTransforms826f52a8: pedestrian, vehicle, then DMO.
        // Pedestrian/Guard vslot8c both bind LWBasicEntity.UpdateTransform.
        {
            let mut living=owner.lock().map_err(|_|SelectedLivingWorldDmoError::LivingWorldLock)?;
            let living=&mut *living;
            let roster=living.pedestrian_entities.as_deref_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?;
            for id in roster.ids(){
                let pedestrian=roster.get_mut(id).expect("captured transform roster");
                let core=pedestrian.core.as_mut()
                    .ok_or_else(||SelectedLivingWorldDmoError::Pedestrians("pedestrian current core absent".into()))?;
                core.update_transform(UPDATE_TIME,&mut HostReckoningNumeric)
                    .map_err(|e|SelectedLivingWorldDmoError::Pedestrians(format!("pedestrian UpdateTransform: {e:?}")))?;
                // ComponentMan82e82be0 calls PostUpdateTransform immediately
                // after each entity. Pedestrian vtable823a4e70+90 binds
                // LWBasicEntity82e4c2b8; Guard823a5410+90 is explicitly empty.
                if pedestrian.security_guard.is_none() && !pedestrian.contained_props.is_empty(){
                    living.props.as_deref_mut().ok_or(SelectedLivingWorldDmoError::MissingManagers)?
                        .update_contained_prop_transforms(&pedestrian.contained_props,core.transform)
                        .map_err(SelectedLivingWorldDmoError::Pedestrians)?;
                }
            }
            living.vehicle_update_transforms(UPDATE_TIME).map_err(SelectedLivingWorldDmoError::Vehicles)?;
        }
        // UpdateWheelAnimInfo follows all three categories (8270328c).
        Self::try_with_frame(world, &vault, actor, cvars, recipes, |frame| {
            frame
                .try_update_post_positions(&inputs, UPDATE_TIME)
                .map_err(SelectedLivingWorldDmoError::Census)
        })?;
        owner
            .lock()
            .map_err(|_| SelectedLivingWorldDmoError::LivingWorldLock)?
            .vehicle_update_wheels(UPDATE_TIME)
            .map_err(SelectedLivingWorldDmoError::Vehicles)
    }
}
