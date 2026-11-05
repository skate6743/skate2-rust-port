//! CensusMan vehicle population -> SAME RecipeMan/ModelMan/factory/entity map.
//! SpawnVehicle8271c188 and EvaluativeSpawnVehicles8271ed50. The outer census
//! component schedule and complete traffic frame remain separate callers.
use super::{
    random::{CensusPositionError, random_position},
    targets::{CensusRandomError, CensusTargetError, LiveCensusRandom, PopulationKind},
};
use crate::{
    actor_construction::graph_startup::ActorGraphStartupOwners,
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_grind_services::{
        LivingWorldGrindServices,
        dmo_models::NULL_ASSET_ID,
        dmo_objects::ENTITIES_CLASS,
        vehicle_entity::spawn::{VehicleSpawnError, VehicleSpawnInfo},
    },
    living_world_rain::geometry::Transform,
    living_world_recipe_manager::{
        LIVING_WORLD_MODELS_CLASS_ID, LOAD_MODEL_ATTRIBUTE_ID,
        attributes::{RecipeAttributeError, RecipeAttributes},
        runtime::{RecipeMan, RecipeSelectionError},
    },
    native_skater_random::NativeRandomGate,
    physical_player_assembly::PhysicalAssemblyWorld,
    player::mounted_recipe::MountedRecipeRegistry,
    reckoning_numeric::ReckoningNumeric,
    world_painter_layer::WorldPainterLayerMan,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum VehicleCensusError<N> {
    MissingOwner(&'static str),
    Attribute(AttributeRuntimeError),
    RecipeAttribute(RecipeAttributeError),
    RecipeSelection(RecipeSelectionError),
    Cvar(InputCvarError),
    Position(CensusPositionError<N>),
    Target(CensusTargetError<CensusRandomError>),
    Spawn(VehicleSpawnError<N>),
    JuiceModelOverridePending,
}

pub(crate) struct VehicleCensusFrame<'a> {
    pub(crate) living: &'a mut LivingWorldGrindServices,
    pub(crate) recipe_man: &'a RecipeMan,
    pub(crate) mounted_recipes: &'a MountedRecipeRegistry,
    pub(crate) vault: &'a Arc<AttributeRuntimeVault>,
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) graphs: &'a ActorGraphStartupOwners,
    pub(crate) world: &'a mut PhysicalAssemblyWorld,
    pub(crate) painter: &'a WorldPainterLayerMan,
    /// Juice.GetModelName(reset=true)/SetModelName side effects are not yet
    /// connected. Refuse that mode rather than pretending it is ordinary spawn.
    pub(crate) override_model_name: bool,
}

impl VehicleCensusFrame<'_> {
    /// EvaluateVehicleCensus82720a98. Uses the original named range collection
    /// and the SAME interpolation kernel already used by DMO census.
    pub(crate) fn evaluate<N: ReckoningNumeric>(
        &mut self,
        inputs: &super::super::dmo_objects::DmoCensusInputs,
        do_cull: bool,
        do_spawn: bool,
        numeric: &mut N,
    ) -> Result<(), super::super::vehicle_entity::frame::VehicleFrameError>
    where
        N::Error: std::fmt::Debug,
    {
        use super::super::{
            dmo_objects::{CENSUS_RANGES_CLASS, CensusCircle, evaluate_census_circle},
            vehicle_entity::frame::error,
        };
        let ranges = self.vault.instance(
            CENSUS_RANGES_CLASS,
            skate2_formats::attrib::string_hash64("vehicles"),
        );
        if ranges.is_null() {
            return Err(error("vehicle census range collection"));
        }
        let data = ranges
            .try_cached_layout_field(0, 0x28)
            .map_err(error)?
            .ok_or_else(|| error("vehicle census range layout"))?;
        let scalar = |offset: usize| {
            f32::from_be_bytes(
                data.bytes()[offset..offset + 4]
                    .try_into()
                    .expect("checked layout"),
            )
        };
        let circle = |base| CensusCircle {
            no_spawn: scalar(base),
            spawn: scalar(base + 4),
            cull: scalar(base + 8),
            shift: scalar(base + 12),
            speed: scalar(base + 16),
        };
        let speed = crate::reckoning_numeric::try_length(inputs.velocity, numeric)
            .map_err(error)?
            * f32::from_bits(0x40666666);
        let circle = evaluate_census_circle(speed, circle(0), circle(0x14));
        let direction =
            crate::reckoning_numeric::try_normalize_safe(inputs.velocity, [0.; 4], numeric)
                .map_err(error)?;
        let center: [f32; 4] =
            std::array::from_fn(|i| direction[i].mul_add(circle.shift, inputs.center[i]));
        let banned = self.living.census.vehicles_banned;
        if do_cull {
            let radius = if banned { 0. } else { circle.cull };
            let entities = self
                .living
                .grind_vehicle_entities
                .as_deref()
                .ok_or_else(|| error("vehicles"))?;
            let mut culls = Vec::new();
            for id in entities.ids() {
                let vehicle = entities.get(id).expect("entity order");
                if !vehicle.core.allow_culling {
                    continue;
                }
                let delta: [f32; 4] =
                    std::array::from_fn(|i| center[i] - vehicle.core.transform[3][i]);
                // EvaluativeCullVehicles8271f210 includes Y (DMO omits Y).
                if !(numeric.dot3(delta, delta).map_err(error)? - radius * radius < 0.) {
                    culls.push(id);
                }
            }
            for id in culls {
                self.living.request_vehicle_cull(id, self.cvars)?;
            }
        }
        if do_spawn
            && !banned
            && self.cvars.try_enable_spawn_vehicles().map_err(error)?
            && self.living.census.enable & 8 != 0
        {
            self.evaluative_spawn(
                center,
                circle.no_spawn,
                circle.spawn,
                inputs.initial,
                inputs.ghost,
                inputs.network_game,
                numeric,
            )
            .map_err(error)?;
        }
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        static CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_VEHICLES").is_some())
            && CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 120 == 0
        {
            eprintln!(
                "vehicle census banned={banned} enable={:#x} max={:?} roster={} center={center:?}",
                self.living.census.enable,
                self.living.census.max_vehicles,
                self.living.grind_vehicle_entities.as_deref().map_or(0, |entities| entities.captured_member_count()),
            );
        }
        Ok(())
    }
    /// Null results are normal source rejection, distinct from a missing owner
    /// or failed dependency. Load_Model=false skips only ModelMan admission,
    /// not model selection or GenerateModelAssetID.
    pub(crate) fn spawn<N: ReckoningNumeric>(
        &mut self,
        entity_key: u64,
        transform: Transform,
        initial: bool,
        allow_culling: bool,
        force_spawn: bool,
        numeric: &mut N,
    ) -> Result<Option<u32>, VehicleCensusError<N::Error>> {
        use VehicleCensusError as E;
        if self.living.census.enable & 0x80 == 0 {
            return Ok(None);
        }
        let entity = self.vault.instance(ENTITIES_CLASS, entity_key);
        if entity.is_null() {
            return Ok(None);
        }
        let parent = entity
            .try_cached_layout_field(8, 8)
            .map_err(E::Attribute)?
            .map_or(0, |field| {
                u64::from_be_bytes(field.bytes().try_into().expect("checked field"))
            });
        let attributes = RecipeAttributes::new(self.vault, self.cvars);
        let model_key = self
            .recipe_man
            .select_model(parent, &attributes)
            .map_err(E::RecipeSelection)?;
        if self.override_model_name {
            return Err(E::JuiceModelOverridePending);
        }
        let model = self.vault.instance(LIVING_WORLD_MODELS_CLASS_ID, model_key);
        if model.is_null() {
            return Ok(None);
        }
        let load_model = model
            .try_get_data(LOAD_MODEL_ATTRIBUTE_ID, 0)
            .map_err(E::Attribute)?
            .is_some_and(|data| data.bytes().first().is_some_and(|&byte| byte != 0));
        let recipe_id = self
            .recipe_man
            .model_asset_id(model_key, &attributes)
            .map_err(E::RecipeAttribute)?;
        if load_model
            && (recipe_id == NULL_ASSET_ID || !self.living.dmo_models.contains_model(recipe_id))
        {
            return Ok(None);
        }
        let limit = if initial {
            self.cvars
                .try_initial_spawn_vehicle_count()
                .map_err(E::Cvar)?
        } else {
            self.living
                .census
                .max_vehicles
                .ok_or(E::MissingOwner("CensusMan frame-saver vehicle allowance"))?
        };
        if limit <= 0 {
            return Ok(None);
        }
        let count = self
            .living
            .grind_vehicle_entities
            .as_deref()
            .ok_or(E::MissingOwner("vehicle EntityMan"))?
            .captured_member_count();
        if limit <= count as i32 {
            return Ok(None);
        }
        self.living
            .request_spawn_vehicle(
                Some(&VehicleSpawnInfo {
                    entity_key,
                    model_key,
                    recipe_id,
                    transform,
                    allow_culling,
                    force_spawn,
                }),
                self.vault,
                self.mounted_recipes,
                self.world,
                self.cvars,
                self.graphs,
                false,
                numeric,
            )
            .map_err(E::Spawn)
    }

    /// A regular pass allows two attempts and one successful spawn. Initial
    /// population uses 6000 attempts / 600 successes and radii8..80. The large
    /// initial budget is independent of the live population cap (normally15).
    pub(crate) fn evaluative_spawn<N: ReckoningNumeric>(
        &mut self,
        center: [f32; 4],
        inner_radius: f32,
        outer_radius: f32,
        initial: bool,
        ghost: bool,
        network_game: bool,
        numeric: &mut N,
    ) -> Result<(), VehicleCensusError<N::Error>> {
        use VehicleCensusError as E;
        let debug_enabled = self
            .vault
            .instance(
                crate::living_world_rain::DEBUG_CLASS,
                crate::living_world_rain::DEBUG_COLLECTION,
            )
            .try_cached_layout_field(121, 1)
            .map_err(E::Attribute)?
            .is_some_and(|data| data.bytes()[0] != 0);
        // Captured even before the source gate; never refreshed by successes.
        let captured_count = self
            .living
            .grind_vehicle_entities
            .as_deref()
            .ok_or(E::MissingOwner("vehicle EntityMan"))?
            .captured_member_count();
        if !debug_enabled || ghost || network_game {
            return Ok(());
        }
        let (attempts, mut remaining, inner, outer) = if initial {
            (6000, 600, 8., 80.)
        } else {
            (2, 1, inner_radius, outer_radius)
        };
        for _ in 0..attempts {
            let random = self
                .living
                .random
                .as_mut()
                .ok_or(E::MissingOwner("Manager RNG"))?;
            let position = random_position(
                random,
                &NativeRandomGate::production_shared(),
                center,
                inner,
                outer,
                numeric,
            )
            .map_err(E::Position)?;
            let key = self
                .living
                .census
                .target(
                    PopulationKind::Vehicle,
                    [position[0], position[1], position[2]],
                    captured_count as u32,
                    self.vault,
                    self.painter,
                    self.cvars,
                    &mut LiveCensusRandom(random),
                )
                .map_err(E::Target)?;
            let Some(key) = key else { continue };
            let transform = [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                position,
            ];
            // Source common packet: initial, allowCull=true, force=false,
            // allowWorldCollisions=true. Vehicle's lower Spawn ignores last.
            if self
                .spawn(key, transform, initial, true, false, numeric)?
                .is_some()
            {
                remaining -= 1;
                if remaining == 0 {
                    break;
                }
            }
        }
        Ok(())
    }
}
