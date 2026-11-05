//! The living world half of `cChallengeCharacterManager` and `cChallengeLivingWorldManager`.
//! The native managers call `LW::Man`, `ILWCensusMan`, `RecipeMan` and `ChaseMan` directly;
//! the challenge system queues those calls as [`LivingWorldOp`]s, applied here in call order
//! after its update, and reads the living world through [`CareerLivingWorld`].

use super::{SelectedLiveError, SelectedLiveSession, error};
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    challenge::{
        characters::{self, CharacterRef, LineHit, LivingWorldFacts, LivingWorldOp},
        living_world,
        system::World,
    },
    collision_line_collection::CollisionLineInput,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{LivingWorldGrindServices, waypoints::WaypointRef},
    living_world_recipe_manager::attributes::RecipeAttributes,
    physical_player_assembly::PhysicalAssemblyWorld,
    selected_living_world_recipes::{
        SelectedLivingWorldRecipes, SelectedRecipeFrame, challenge_dmos,
    },
    selected_terrain_startup::dmo::ChallengeDmoChunks,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

/// `TestBBoxAgainstTree`'s spawn: `ILWCensusMan::SpawnPedestrian(entity, transform, initial false,
/// allowCull false, force true, allowWorldCollisions true, ignoreNavMeshInGroundTest true,
/// allowHandProp true)`.
const SPAWN_FLAGS: (bool, bool, bool, bool, bool, bool) = (false, false, true, true, true, true);
/// `PhysicsLineTestBatch` flags `BF_GroundDmo`.
const BF_GROUND_DMO: u32 = 3;
/// `World::AcquireLineBatch(batchID)` (`0x827b1a58`): `BF_Default`.
const BF_DEFAULT: u32 = 7;
#[path = "selected_session_career_big_black.rs"]
mod big_black;

/// The living world the challenge system reads: `LW::Man::GetWaypointByGUID`,
/// `RecipeMan::IsLivingWorldModelTracked` and `ChaseMan::ChaseIsOn`.
pub(super) struct CareerLivingWorld {
    living: Arc<Mutex<LivingWorldGrindServices>>,
    recipes: Arc<Mutex<SelectedLivingWorldRecipes>>,
    vault: Arc<AttributeRuntimeVault>,
    cvars: InputCvarReferences,
}

impl CareerLivingWorld {
    pub(super) fn new(
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        recipes: &Arc<Mutex<SelectedLivingWorldRecipes>>,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
    ) -> Result<Self, SelectedLiveError> {
        let living = Arc::clone(
            world
                .lock()
                .map_err(|_| error("world lock poisoned"))?
                .grind_services
                .living_world(),
        );
        Ok(Self {
            living,
            recipes: Arc::clone(recipes),
            vault: Arc::clone(vault),
            cvars: cvars.clone(),
        })
    }
}

impl LivingWorldFacts for CareerLivingWorld {
    fn waypoint_location(&self, guid: u64) -> Option<[f32; 4]> {
        let living = self.living.lock().ok()?;
        let props = living.props.as_deref()?;
        let &prop = props.waypoint_guids.get(&guid)?;
        props
            .group(prop)
            .ok()?
            .points
            .first()
            .map(|point| point.location)
    }

    fn is_model_tracked(&self, model: u64) -> bool {
        let Ok(filename) = RecipeAttributes::new(&self.vault, &self.cvars).filename(model) else {
            return false;
        };
        self.recipes
            .lock()
            .is_ok_and(|recipes| recipes.recipes.is_tracked(&filename))
    }

    fn big_black_position(&self) -> Option<[f32;4]> {
        let living=self.living.lock().ok()?;
        living.pedestrian_entities.as_deref()?.get(living.big_black_entity?)?
            .core.as_ref().map(|core|core.transform[3])
    }

    fn chase_is_on(&self) -> bool {
        self.living.lock().is_ok_and(|living| {
            living
                .chase
                .as_ref()
                .is_some_and(crate::living_world_chase::ChaseMan::chase_is_on)
        })
    }
}

fn trace() -> bool {
    std::env::var_os("SKATE2_TRACE_CAREER").is_some()
}

/// Whether the model is a marquee one whose schema a skater's load holds (the player's, a party
/// or AI skater's). The session registry cannot share that recipe with a living world load as
/// the native asset manager does, so such a `LoadLivingWorldModel` is refused.
fn skater_holds_marquee(
    frame: &SelectedRecipeFrame<'_>,
    cvars: &InputCvarReferences,
    model: u64,
) -> Result<bool, String> {
    use crate::living_world_recipe_manager::runtime::ModelCategory;
    let model = RecipeAttributes::new(frame.vault, cvars)
        .model(model)
        .map_err(|e| e.to_string())?;
    if model.category != ModelCategory::Marquee {
        return Ok(false);
    }
    let name =
        crate::selected_living_world_recipes::marquee_schema_name(frame.vault, &model.filename)
            .map_err(|e| format!("{e:?}"))?;
    let id = skate2_formats::hash::rw_hash64_string(&name.to_ascii_lowercase());
    Ok(frame.registry.holds_skater_recipe(id))
}

/// `LWBasicEntity::IsIntersectingTriggerVolume` (0x82e4c138): the entity model's bounding volume
/// at `GetBBoxTransform` of the entity transform, against the trigger's
/// (`TriggerVolumeUtils::IsColldingWithTriggerVolume` 0x82e213b8 with `valueWithoutCModel` false).
fn intersects_trigger(
    model: Option<&crate::living_world_grind_services::character_models::CharacterModel>,
    transform: [[f32; 4]; 4],
    trigger: Option<&(skate2_formats::collision::Volume, [f32; 16])>,
) -> bool {
    use crate::narrow_phase::{GpPrimitive, rw_overlap};
    let (Some(model), Some((volume, trigger_transform))) = (model, trigger) else {
        return false;
    };
    let flat: [f32; 16] = std::array::from_fn(|i| transform[i / 4][i % 4]);
    let entity = GpPrimitive::from_volume(&model.bbox_volume, Some(model.bbox_transform(&flat)));
    let trigger = GpPrimitive::from_volume(volume, Some(*trigger_transform));
    entity
        .zip(trigger)
        .and_then(|(entity, trigger)| rw_overlap(&entity, &trigger, 0.0))
        .unwrap_or(false)
}

/// The spawns and model loads this application made, by token.
#[derive(Default)]
struct Answers {
    spawned: BTreeMap<u32, Option<u32>>,
    loads: BTreeMap<u32, bool>,
}

impl Answers {
    fn pedestrian(&self, character: CharacterRef) -> Option<u32> {
        match character {
            CharacterRef::Spawned(id) => Some(id),
            CharacterRef::Pending(token) => self.spawned.get(&token).copied().flatten(),
        }
    }
}

/// The session owners one application borrows.
struct Owners<'f, 'a> {
    frame: &'f mut SelectedRecipeFrame<'a>,
    world: &'f mut PhysicalAssemblyWorld,
    ground: &'f crate::static_ground_pool::StaticGroundPool,
    cvars: &'f InputCvarReferences,
    actor: Option<&'f crate::actor_construction::ActorConstruction>,
    /// The terrain whose DMO pass records the challenge DMO groups.
    terrain: &'f crate::selected_terrain_startup::SelectedTerrainStartup,
}

impl Owners<'_, '_> {
    fn living(&self) -> Arc<Mutex<LivingWorldGrindServices>> {
        Arc::clone(self.world.grind_services.living_world())
    }

    /// `RecipeMan`'s `cMsgAssetSimDRegLWSchema`s, which `cChallengeCharacterManager::
    /// ReceiveDRegLWSchemaMsg` handles.
    fn deliver_dregs(&mut self, challenge: &mut World) {
        for recipe in std::mem::take(&mut self.frame.runtime.deregistered) {
            characters::receive_dreg_lw_schema(challenge, recipe);
        }
    }

    /// `Pedestrian::RequestExplicitCull` (0x82e90dd0): `SetAllowCull(true)`, then the census's
    /// `RequestCull`.
    fn request_explicit_cull(&mut self, id: u32) -> Result<(), String> {
        let owner = self.living();
        let mut living = owner.lock().map_err(|_| "LivingWorld lock")?;
        let Some(core) = living
            .pedestrian_entities
            .as_deref_mut()
            .and_then(|p| p.get_mut(id))
            .and_then(|p| p.core.as_mut())
        else {
            return Ok(());
        };
        core.allow_culling = true;
        if trace() {
            eprintln!("career characters: pedestrian {id} culled");
        }
        let painter = self.frame.painter.lock().map_err(|_| "WorldPainter lock")?;
        let mut random = living.random.take().ok_or("Manager RNG absent")?;
        let result = self.frame.request_pedestrian_cull(
            &mut living,
            self.world,
            id,
            self.cvars,
            &painter,
            &mut random,
        );
        living.random = Some(random);
        result
    }

    /// `LivingWorld.ActivateSpectatorWaypoint` (0x82893da8): the GUID's waypoint group, when it has
    /// waypoints, gets `WaypointMan::ActivateSpectatorWaypoint` (0x82ce9180): the census-managed
    /// cull first, then its forced spectators queued, or the group activated without any.
    fn activate_spectator_waypoint(
        &mut self,
        guid: u64,
        force_spawn_percentage: f32,
        category_group: u64,
        keep: &[u32],
    ) -> Result<(), String> {
        let owner = self.living();
        let (prop, group_guid, waypoints) = {
            let living = owner.lock().map_err(|_| "LivingWorld lock")?;
            let Some(props) = living.props.as_deref() else { return Ok(()) };
            let Some(prop) = props.waypoint_group_by_guid(guid) else { return Ok(()) };
            let group = props.group(prop)?;
            (prop, group.guid()?, u32::try_from(group.points.len()).map_err(|_| "waypoint count")?)
        };
        if waypoints == 0 {
            return Ok(());
        }
        self.census_managed_cull(None, keep)?;
        let max_force_spectators = self
            .cvars
            .try_max_num_force_spectators()
            .map_err(|e| format!("kMaxNumForceSpectators: {e:?}"))?;
        let mut living = owner.lock().map_err(|_| "LivingWorld lock")?;
        let living = &mut *living;
        if living.waypoint_man.activate(waypoints, group_guid, force_spawn_percentage, category_group, max_force_spectators)? {
            living.props.as_deref_mut().ok_or("spectator PropMan absent")?.activate_prop(prop)?;
        }
        Ok(())
    }

    /// `LivingWorld.DeactivateSpectatorWaypoint` (0x8288e488): the GUID's waypoint group gets
    /// `WaypointMan::DeactivateSpectatorWaypoint(group, true)` (0x82ce8ed8): its queued activation
    /// dropped, the group deactivated and each locked waypoint's pedestrian culled
    /// (`RequestExplicitCull`).
    fn deactivate_spectator_waypoint(&mut self, guid: u64) -> Result<(), String> {
        let owner = self.living();
        let released = {
            let mut living = owner.lock().map_err(|_| "LivingWorld lock")?;
            let living = &mut *living;
            let Some(props) = living.props.as_deref_mut() else { return Ok(()) };
            let Some(prop) = props.waypoint_group_by_guid(guid) else { return Ok(()) };
            living.waypoint_man.deactivate(props.group(prop)?.guid()?);
            props.deactivate_waypoint_group(prop)?
        };
        for pedestrian in released {
            self.request_explicit_cull(pedestrian)?;
        }
        Ok(())
    }

    /// `Pedestrian::SetCurrentWaypoint` (0x82e6e600) to the group's first waypoint.
    fn set_current_waypoint(&mut self, id: u32, guid: u64) -> Result<(), String> {
        let owner = self.living();
        let mut living = owner.lock().map_err(|_| "LivingWorld lock")?;
        let Some(&prop) = living
            .props
            .as_deref()
            .and_then(|props| props.waypoint_guids.get(&guid))
        else {
            return Ok(());
        };
        if let Some(entity) = living
            .pedestrian_entities
            .as_deref_mut()
            .and_then(|p| p.get_mut(id))
        {
            entity.set_current_waypoint(Some(WaypointRef { prop, index: 0 }));
        }
        Ok(())
    }

    /// Bits of the pedestrian's `tPedestrianStateStorage` bytes 0xca8 and 0xca9.
    fn state_flags(&mut self, id: u32, set: [u8; 2], clear: [u8; 2]) -> Result<(), String> {
        let owner = self.living();
        let mut living = owner.lock().map_err(|_| "LivingWorld lock")?;
        if let Some(state) = living
            .pedestrian_entities
            .as_deref_mut()
            .and_then(|p| p.get_mut(id))
            .and_then(|p| p.state.as_mut())
        {
            for byte in 0..2 {
                state.flags[byte] = (state.flags[byte] | set[byte]) & !clear[byte];
            }
        }
        Ok(())
    }

    /// `TestBBoxAgainstTree`'s spawn of an entry: the marquee model's `LoadLivingWorldModel`
    /// first, then the census spawn unless that load was refused.
    #[expect(clippy::too_many_arguments, reason = "one queued spawn's fields")]
    fn spawn(
        &mut self,
        challenge: &mut World,
        answers: &mut Answers,
        token: u32,
        entity: u64,
        transform: [[f32; 4]; 4],
        model: u64,
        load_model: bool,
        spawn: bool,
        waypoint: Option<u64>,
        challenge_flags: bool,
    ) -> Result<(), SelectedLiveError> {
        let taken = if load_model {
            let loaded = if skater_holds_marquee(self.frame, self.cvars, model).map_err(error)? {
                if trace() {
                    eprintln!(
                        "career characters: model {model:#018x} not loaded: a skater holds its recipe"
                    );
                }
                false
            } else {
                self.frame
                    .load_living_world_model(self.world, self.cvars, model)
                    .map_err(error)?
            };
            answers.loads.insert(token, loaded);
            Some(loaded)
        } else {
            None
        };
        let mut spawned = None;
        if spawn && taken != Some(false) {
            let (initial, allow_cull, force, collisions, ignore_nav_mesh, hand_prop) = SPAWN_FLAGS;
            if let Some(id) = self
                .frame
                .spawn_census_pedestrian(
                    self.world,
                    self.cvars,
                    self.ground,
                    entity,
                    transform,
                    initial,
                    allow_cull,
                    force,
                    collisions,
                    ignore_nav_mesh,
                    hand_prop,
                )
                .map_err(error)?
            {
                let owner = self.living();
                let recipe = {
                    let mut living = owner.lock().map_err(|_| error("LivingWorld lock poisoned"))?;
                    let pedestrian = living.pedestrian_entities.as_deref_mut()
                        .and_then(|p| p.get_mut(id))
                        .ok_or_else(|| error("spawned challenge pedestrian absent"))?;
                    pedestrian.challenge_owner = challenge.characters.spawn_owner(token);
                    pedestrian.core.as_ref().ok_or_else(|| error("spawned pedestrian core absent"))?.recipe_id
                };
                if let Some(guid) = waypoint {
                    self.set_current_waypoint(id, guid).map_err(error)?;
                }
                if challenge_flags {
                    self.state_flags(id, [0x40, 0x40], [0x20, 0])
                        .map_err(error)?;
                }
                if trace() {
                    eprintln!(
                        "career characters: entity {entity:#018x} spawned as pedestrian {id}"
                    );
                }
                spawned = Some((id, recipe));
            }
        }
        answers.spawned.insert(token, spawned.map(|(id, _)| id));
        characters::spawn_answered(challenge, token, taken, spawned);
        Ok(())
    }

    /// The submitted `PhysicsLineTestBatch`, run at once: `AcquireLineBatch(BF_GroundDmo)`,
    /// each `AddTest`, then `GetResult`; a dynamic object's hit names its assembly owner
    /// (`LWCollideableObjectMan::GetAssemblyOwnerID`). A hit lies on its line, whose W lane is
    /// the tested position's.
    fn line_tests(
        &mut self,
        tests: &[([f32; 4], [f32; 4], f32)],
    ) -> Result<Vec<Option<LineHit>>, String> {
        let (queries, dmos) = self.world.world_line_owners();
        let queries = queries.ok_or("World line queries absent")?;
        let dmos = dmos.ok_or("World DMO pool absent")?;
        let batch = queries
            .try_acquire_line_batch(dmos, "cChallengeCharacterManager", BF_GROUND_DMO)
            .map_err(|e| format!("challenge line batch: {e:?}"))?;
        let mut hits = Vec::with_capacity(tests.len());
        let result = (|| {
            queries
                .try_begin(&batch)
                .map_err(|e| format!("challenge line batch: {e:?}"))?;
            for &(start, end, fatness) in tests {
                let input = CollisionLineInput {
                    start: SkeletonRootVector::known(start),
                    end: SkeletonRootVector::known(end),
                    fatness,
                };
                queries
                    .try_add_line(&batch, input)
                    .map_err(|e| format!("challenge line test: {e:?}"))?;
            }
            queries
                .try_end(
                    &batch,
                    self.ground,
                    dmos,
                    &mut crate::collision_line_numeric::HostCollisionLineNumeric,
                )
                .map_err(|e| format!("challenge line batch: {e:?}"))?;
            queries
                .try_sync(&batch)
                .map_err(|e| format!("challenge line batch: {e:?}"))?;
            for (index, &(start, ..)) in tests.iter().enumerate() {
                let index = u32::try_from(index).map_err(|_| "challenge line batch index")?;
                let hit = queries
                    .try_get_result(&batch, index)
                    .map_err(|e| format!("challenge line result: {e:?}"))?;
                hits.push(hit.map(|hit| {
                    let xyz = hit.position.xyz;
                    ([xyz[0], xyz[1], xyz[2], start[3]], hit.owner)
                }));
            }
            Ok::<_, String>(())
        })();
        queries
            .try_release(&batch)
            .map_err(|e| format!("challenge line batch: {e:?}"))?;
        result?;
        let owner = self.living();
        let living = owner.lock().map_err(|_| "LivingWorld lock")?;
        let objects = living.dmo_objects.as_deref().ok_or("DMO manager absent")?;
        Ok(hits
            .into_iter()
            .map(|hit| {
                hit.map(|(position, body)| LineHit {
                    position,
                    owner: body.map(|body| objects.owner_of(&body)),
                })
            })
            .collect())
    }

    /// A crowd generator's batch, run at once: `AcquireLineBatch(BF_Default)`, each `AddTest`
    /// until the batch refuses one, then `GetResult` of each it took. A hit lies on its line,
    /// whose W lane is the tested start's.
    fn crowd_line_tests(
        &mut self,
        tests: &[([f32; 4], [f32; 4], f32)],
    ) -> Result<Vec<Option<[f32; 4]>>, String> {
        let (queries, dmos) = self.world.world_line_owners();
        let queries = queries.ok_or("World line queries absent")?;
        let dmos = dmos.ok_or("World DMO pool absent")?;
        let batch = queries
            .try_acquire_line_batch(dmos, "cCrowdPositionAndGenerationState::SubmitLineTests", BF_DEFAULT)
            .map_err(|e| format!("crowd line batch: {e:?}"))?;
        let mut hits = Vec::with_capacity(tests.len());
        let result = (|| {
            queries
                .try_begin(&batch)
                .map_err(|e| format!("crowd line batch: {e:?}"))?;
            let mut taken = 0;
            for &(start, end, fatness) in tests {
                let input = CollisionLineInput {
                    start: SkeletonRootVector::known(start),
                    end: SkeletonRootVector::known(end),
                    fatness,
                };
                if !queries
                    .try_add_line(&batch, input)
                    .map_err(|e| format!("crowd line test: {e:?}"))?
                {
                    break;
                }
                taken += 1;
            }
            queries
                .try_end(
                    &batch,
                    self.ground,
                    dmos,
                    &mut crate::collision_line_numeric::HostCollisionLineNumeric,
                )
                .map_err(|e| format!("crowd line batch: {e:?}"))?;
            queries
                .try_sync(&batch)
                .map_err(|e| format!("crowd line batch: {e:?}"))?;
            for (index, &(start, ..)) in tests.iter().take(taken).enumerate() {
                let index = u32::try_from(index).map_err(|_| "crowd line batch index")?;
                let hit = queries
                    .try_get_result(&batch, index)
                    .map_err(|e| format!("crowd line result: {e:?}"))?;
                hits.push(hit.map(|hit| {
                    let xyz = hit.position.xyz;
                    [xyz[0], xyz[1], xyz[2], start[3]]
                }));
            }
            Ok::<_, String>(())
        })();
        queries
            .try_release(&batch)
            .map_err(|e| format!("crowd line batch: {e:?}"))?;
        result?;
        Ok(hits)
    }

    /// `LWDynamicObjectMan::GetDynamicObject`, then `CanResetDynamicObject` and
    /// `ResetDynamicObject` (0x82d3a908, 0x82d39e10).
    fn reset_dynamic_object(&mut self, entity: u32) -> Result<(), String> {
        let owner = self.living();
        let mut living = owner.lock().map_err(|_| "LivingWorld lock")?;
        // CanResetDynamicObject finds no SpawnStatic body among the living world's objects.
        if living
            .dmo_objects
            .as_deref()
            .and_then(|objects| objects.object(entity))
            .is_none_or(|object| object.online_static)
        {
            return Ok(());
        }
        let actor = self.actor.ok_or("DMO reset local Actor absent")?;
        let painter = self.frame.painter.lock().map_err(|_| "WorldPainter lock")?;
        self.frame.try_reset_unsafe_dmo(
            &mut living,
            self.world,
            entity,
            self.cvars,
            &painter,
            actor,
        )
    }

    /// `ILWManager::RequestCensusManagedCull` (0x826f5158, 0x826f51b0): each census-managed
    /// pedestrian (`IsCensusManaged`: not a challenge character) and vehicle, within the volume
    /// when one is given, gets `RequestExplicitCull`; a vehicle's is its core's allow-cull bit and
    /// the vehicle census's `RequestCull`.
    fn census_managed_cull(
        &mut self,
        volume: Option<u64>,
        challenge_characters: &[u32],
    ) -> Result<(), String> {
        let trigger = match volume {
            None => None,
            Some(id) => {
                let triggers = self.frame.triggers.lock().map_err(|_| "trigger lock")?;
                let Some(record) = triggers
                    .try_find_volume_id(crate::skate3::id_float(id))
                    .map_err(|e| format!("cull trigger lookup: {e:?}"))?
                else {
                    return Ok(());
                };
                Some(record.trigger_bounding_volume()?)
            }
        };
        let within = |model: Option<
            &crate::living_world_grind_services::character_models::CharacterModel,
        >,
                      transform: [[f32; 4]; 4]| {
            trigger
                .as_ref()
                .is_none_or(|bounds| intersects_trigger(model, transform, bounds.as_ref()))
        };
        let owner = self.living();
        let (pedestrians, vehicles) = {
            let living = owner.lock().map_err(|_| "LivingWorld lock")?;
            let pedestrians: Vec<u32> = living
                .pedestrian_entities
                .as_deref()
                .map(|roster| {
                    roster
                        .ids()
                        .into_iter()
                        .filter(|id| !challenge_characters.contains(id))
                        .filter(|&id| {
                            roster.get(id).is_some_and(|p| {
                                p.big_black.is_none() && p.core
                                    .as_ref()
                                    .is_some_and(|core| within(p.model.as_deref(), core.transform))
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let vehicles: Vec<u32> = living
                .grind_vehicle_entities
                .as_deref()
                .map(|roster| {
                    roster
                        .ids()
                        .into_iter()
                        .filter(|&id| {
                            roster
                                .get(id)
                                .is_some_and(|v| within(v.model.as_deref(), v.core.transform))
                        })
                        .collect()
                })
                .unwrap_or_default();
            (pedestrians, vehicles)
        };
        for id in pedestrians {
            self.request_explicit_cull(id)?;
        }
        let mut living = owner.lock().map_err(|_| "LivingWorld lock")?;
        for id in vehicles {
            if let Some(vehicle) = living
                .grind_vehicle_entities
                .as_deref_mut()
                .and_then(|v| v.get_mut(id))
            {
                vehicle.core.allow_culling = true;
                living
                    .request_vehicle_cull(id, self.cvars)
                    .map_err(|e| format!("challenge vehicle cull: {e:?}"))?;
            }
        }
        Ok(())
    }

    /// One queued change.
    fn apply(
        &mut self,
        challenge: &mut World,
        answers: &mut Answers,
        op: LivingWorldOp,
    ) -> Result<(), SelectedLiveError> {
        match op {
            LivingWorldOp::BigBlack(spawn) => {
                self.big_black(spawn).map_err(error)?;
                challenge.services.big_black_active=self.living().lock()
                    .map_err(|_|error("BigBlack service readback lock"))?.big_black_entity.is_some();
            },
            LivingWorldOp::Spawn {
                token,
                entity,
                transform,
                model,
                load_model,
                spawn,
                waypoint,
                challenge_flags,
            } => {
                self.spawn(
                    challenge,
                    answers,
                    token,
                    entity,
                    transform,
                    model,
                    load_model,
                    spawn,
                    waypoint,
                    challenge_flags,
                )?;
            }
            LivingWorldOp::RequestExplicitCull(character) => {
                if let Some(id) = answers.pedestrian(character) {
                    self.request_explicit_cull(id).map_err(error)?;
                }
            }
            LivingWorldOp::AllowCull(character) => {
                if let Some(id) = answers.pedestrian(character) {
                    // INavEntity::SetAllowCull (0x82e946b0): the core's allow-cull bit.
                    let owner = self.living();
                    let mut living = owner
                        .lock()
                        .map_err(|_| error("LivingWorld lock poisoned"))?;
                    if let Some(core) = living
                        .pedestrian_entities
                        .as_deref_mut()
                        .and_then(|p| p.get_mut(id))
                        .and_then(|p| p.core.as_mut())
                    {
                        core.allow_culling = true;
                    }
                }
            }
            LivingWorldOp::StateFlags {
                character,
                set,
                clear,
            } => {
                if let Some(id) = answers.pedestrian(character) {
                    self.state_flags(id, set, clear).map_err(error)?;
                }
            }
            LivingWorldOp::SetCurrentWaypoint {
                character,
                waypoint,
            } => {
                if let Some(id) = answers.pedestrian(character) {
                    self.set_current_waypoint(id, waypoint).map_err(error)?;
                }
            }
            LivingWorldOp::UnloadModel { model, after } => {
                if after.is_none_or(|token| answers.loads.get(&token).copied().unwrap_or(true)) {
                    if trace() {
                        eprintln!("career characters: model {model:#018x} released");
                    }
                    self.frame
                        .unload_living_world_model(self.world, self.cvars, model)
                        .map_err(error)?;
                    self.deliver_dregs(challenge);
                }
            }
            LivingWorldOp::LineTests { serial, tests } => {
                let results = self.line_tests(&tests).map_err(error)?;
                characters::line_tests_answered(challenge, serial, results);
            }
            LivingWorldOp::CrowdLineTests { serial, tests } => {
                let results = self.crowd_line_tests(&tests).map_err(error)?;
                crate::challenge::crowd::line_tests_answered(challenge, serial, results);
            }
            LivingWorldOp::ResetDynamicObject(entity) => {
                self.reset_dynamic_object(entity).map_err(error)?
            }
            LivingWorldOp::CensusEnable { mask, value } => {
                let owner = self.living();
                let mut living = owner
                    .lock()
                    .map_err(|_| error("LivingWorld lock poisoned"))?;
                living.census.enable = (living.census.enable & !mask) | value;
            }
            LivingWorldOp::CensusManagedCullAll { keep } => {
                let keep: Vec<u32> = keep
                    .into_iter()
                    .filter_map(|character| answers.pedestrian(character))
                    .collect();
                self.census_managed_cull(None, &keep).map_err(error)?;
            }
            LivingWorldOp::CensusManagedCullVolume { volume, keep } => {
                let keep: Vec<u32> = keep
                    .into_iter()
                    .filter_map(|character| answers.pedestrian(character))
                    .collect();
                self.census_managed_cull(Some(volume), &keep)
                    .map_err(error)?;
            }
            LivingWorldOp::ActivateSpectatorWaypoint { guid, force_spawn_percentage, category_group, keep } => {
                let keep: Vec<u32> = keep
                    .into_iter()
                    .filter_map(|character| answers.pedestrian(character))
                    .collect();
                self.activate_spectator_waypoint(guid, force_spawn_percentage, category_group, &keep)
                    .map_err(error)?;
            }
            LivingWorldOp::DeactivateSpectatorWaypoint { guid } => {
                self.deactivate_spectator_waypoint(guid).map_err(error)?;
            }
            LivingWorldOp::AvoidVolumeAdded(volume) => {
                self.frame
                    .runtime
                    .challenge_avoid_volumes
                    .record_after_avoid_area_cull(volume);
            }
            LivingWorldOp::AvoidVolumesCleared => {
                // CompleteChallenge (0x825b7640) clears AvoidVolumeList as GameUnload does.
                self.frame.runtime.challenge_avoid_volumes.game_unload();
            }
            LivingWorldOp::DmoReset { kind, volume } => {
                self.dmo_reset(challenge, kind, volume).map_err(error)?;
            }
            LivingWorldOp::AddChallengeDmos { treat_as_static } => {
                let chunks = self.challenge_dmo_chunks().map_err(error)?;
                let owner = self.living();
                let mut living = owner
                    .lock()
                    .map_err(|_| error("LivingWorld lock poisoned"))?;
                let count = |living: &LivingWorldGrindServices| {
                    living.dmo_locators.as_deref().map_or(0, |locators| locators.locator_count())
                };
                let before = count(&living);
                challenge_dmos::try_add_challenge_dmos(&mut living, &chunks, treat_as_static)
                    .map_err(error)?;
                if trace() {
                    eprintln!(
                        "career DMOs: challenge DMO groups {:016x?} placed (static {treat_as_static}): locators {before} -> {}",
                        chunks.groups,
                        count(&living)
                    );
                }
            }
            LivingWorldOp::UnwindDmoReset { kind, volume } => {
                self.unwind_dmo_reset(challenge, kind, volume).map_err(error)?;
            }
            LivingWorldOp::ResetChallengeDmos => self.reset_challenge_dmos().map_err(error)?,
            LivingWorldOp::DynamicObjectsCensusRange(key) => {
                let owner = self.living();
                let mut living = owner
                    .lock()
                    .map_err(|_| error("LivingWorld lock poisoned"))?;
                living.census.dynamic_objects_range_key = key.unwrap_or(
                    crate::living_world_grind_services::dmo_objects::DYNAMIC_OBJECTS_KEY,
                );
            }
        }
        Ok(())
    }

    /// The challenge DMO groups the terrain's DMO pass recorded, with their records.
    fn challenge_dmo_chunks(&self) -> Result<ChallengeDmoChunks, String> {
        self.terrain
            .challenge_dmo_chunks()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "challenge DMO groups without a LivingWorld terrain".to_owned())
    }

    /// `RunDMOOperations`' `tDMOResetInfo` on the volume's trigger instance; a Save's locators
    /// go to the DMO manager under the trigger's GUID.
    fn dmo_reset(&mut self, challenge: &mut World, kind: u32, volume: u64) -> Result<(), String> {
        let Some(trigger) = self.frame.challenge_trigger(volume)? else {
            return Ok(());
        };
        let chunks = self.challenge_dmo_chunks()?;
        let owner = self.living();
        let mut living = owner.lock().map_err(|_| "LivingWorld lock")?;
        let painter = self.frame.painter.lock().map_err(|_| "WorldPainter lock")?;
        let saved = self.frame.try_dmo_reset(
            &mut living,
            self.world,
            kind,
            &trigger,
            &chunks,
            self.cvars,
            &painter,
        )?;
        if trace() {
            eprintln!(
                "career DMOs: reset type {kind} on volume {volume:#x} (trigger {:016x}): {} saved, {} DMOs",
                trigger.guid,
                saved.as_ref().map_or(0, Vec::len),
                living.dmo_objects.as_deref().map_or(0, |objects| objects.count())
            );
        }
        if let Some(saved) = saved {
            challenge.dmos.save_locators(trigger.guid, saved);
        }
        Ok(())
    }

    /// `UnwindDMOOperations`' undo on the volume's trigger instance; a Save's locators come back
    /// from the DMO manager.
    fn unwind_dmo_reset(&mut self, challenge: &mut World, kind: u32, volume: u64) -> Result<(), String> {
        let Some(trigger) = self.frame.challenge_trigger(volume)? else {
            return Ok(());
        };
        let saved = if kind == crate::challenge::dmo::reset_type::SAVE {
            let Some(saved) = challenge.dmos.take_saved_locators(trigger.guid) else {
                return Ok(());
            };
            Some(saved)
        } else {
            None
        };
        if trace() {
            eprintln!("career DMOs: reset type {kind} on volume {volume:#x} unwound");
        }
        let chunks = self.challenge_dmo_chunks()?;
        let owner = self.living();
        let mut living = owner.lock().map_err(|_| "LivingWorld lock")?;
        challenge_dmos::try_unwind_dmo_reset(&mut living, kind, &trigger, saved, &chunks)
    }

    /// `ResetChallengeDMOs` of the recorded challenge DMO groups.
    fn reset_challenge_dmos(&mut self) -> Result<(), String> {
        let chunks = self.challenge_dmo_chunks()?;
        if trace() {
            eprintln!("career DMOs: challenge DMO groups {:016x?} reset", chunks.groups);
        }
        let owner = self.living();
        let mut living = owner.lock().map_err(|_| "LivingWorld lock")?;
        let painter = self.frame.painter.lock().map_err(|_| "WorldPainter lock")?;
        self.frame
            .try_reset_challenge_dmos(&mut living, self.world, &chunks, self.cvars, &painter)
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// Installs the living world the challenge system reads.
    pub(super) fn bind_career_living_world(
        &self,
        challenge: &mut World,
    ) -> Result<(), SelectedLiveError> {
        let world = &self
            .inner
            .world
            .as_ref()
            .ok_or_else(|| error("career world absent"))?
            .world;
        let player = self
            .inner
            .player
            .as_ref()
            .ok_or_else(|| error("career player absent"))?;
        let owners = &player.selected.owners;
        let facts = CareerLivingWorld::new(
            world,
            &owners.living_world_recipes,
            &owners.services.vault,
            &owners.cvars,
        )?;
        characters::install_facts(challenge, Box::new(facts));
        set_waypoint_subscription(world, true)
    }

    /// `cMsgWaypointGroupAdded`s the simulation broadcaster delivered since the last update
    /// (`cChallengeCharacterManager::ReceiveWaypointGroupAddedMessage`), before the challenge
    /// system updates.
    pub(super) fn deliver_career_waypoint_groups(&mut self) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_mut() else {
            return Ok(());
        };
        let world = &self
            .inner
            .world
            .as_ref()
            .ok_or_else(|| error("career world absent"))?
            .world;
        let Some(route) = game_sim_route(world)? else {
            return Ok(());
        };
        let groups = route
            .challenge_waypoint_groups
            .lock()
            .map_err(|_| error("challenge waypoint group lock poisoned"))?
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default();
        for guid in groups {
            characters::receive_waypoint_group_added(career.system.world_mut(), guid);
        }
        Ok(())
    }

    /// The challenge system's living world changes since the last application, in call order,
    /// with the `RecipeMan` deregistrations its character manager receives. `NISManager`'s
    /// preloaded scene is read for the next update.
    pub(in super::super) fn apply_career_living_world(&mut self) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_mut() else {
            return Ok(());
        };
        let challenge = career.system.world_mut();
        challenge.characters.nis_scene_preloaded = career.nis.has_preloaded_scene();
        let world_owner = self
            .inner
            .world
            .as_ref()
            .ok_or_else(|| error("career world absent"))?;
        let terrain = self
            .inner
            .terrain
            .as_ref()
            .ok_or_else(|| error("career terrain absent"))?;
        let startup = terrain
            .terrain_for(&world_owner.world)
            .map_err(|e| error(format!("{e:?}")))?;
        let ground = terrain
            .ground_pool_for(&world_owner.world)
            .map_err(|e| error(format!("{e:?}")))?;
        let player = self
            .inner
            .player
            .as_mut()
            .ok_or_else(|| error("career player absent"))?;
        let actor = player.selected.actor.as_deref();
        let owners = &mut player.selected.owners;
        let cvars = owners.cvars.clone();
        let recipe_owner = Arc::clone(&owners.living_world_recipes);
        let mut recipe_runtime = recipe_owner
            .lock()
            .map_err(|_| error("RecipeMan owner lock poisoned"))?;
        if recipe_runtime.deregistered.is_empty() && challenge.characters.ops().is_empty() {
            return Ok(());
        }
        let graph_owner = Arc::clone(
            owners
                .graphs
                .as_ref()
                .ok_or_else(|| error("graph owners absent"))?,
        );
        let graph_guard = graph_owner
            .lock()
            .map_err(|_| error("graph owner lock poisoned"))?;
        let mut frame = SelectedRecipeFrame {
            challenge: None,
            triggers: &startup.triggers,
            pedestrian_animation: crate::selected_living_world_recipes::pedestrian_animation::PedestrianSessionAnimation {
                startup: &owners.pedestrian_animation,
                shared: &owners.animation,
                world: &world_owner.world,
            },
            runtime: &mut recipe_runtime,
            registry: owners.recipe_registry.as_mut().ok_or_else(|| error("mounted recipe registry absent"))?,
            requests: owners.recipe_load.as_mut().ok_or_else(|| error("mounted recipe requests absent"))?.requests_mut(),
            mounts: &owners.mounts,
            vault: &owners.services.vault,
            graphs: &graph_guard,
            painter: &startup.layers,
        };
        let mut world = world_owner
            .world
            .lock()
            .map_err(|_| error("world lock poisoned"))?;
        let mut owners = Owners {
            frame: &mut frame,
            world: &mut *world,
            ground,
            cvars: &cvars,
            actor,
            terrain: startup,
        };
        owners.deliver_dregs(challenge);
        let mut answers = Answers::default();
        loop {
            let ops = characters::take_ops(challenge);
            if ops.is_empty() {
                return Ok(());
            }
            for op in ops {
                owners.apply(challenge, &mut answers, op)?;
            }
        }
    }

    /// `CrowdPresentation::Update` (0x827e8320) of the challenge's crowd buffer, on the
    /// presentation's recipes and pedestrian animation.
    pub(in super::super) fn update_career_crowd_presentation(&mut self) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_ref() else {
            return Ok(());
        };
        let buffer = career.system.world().crowds.buffer.clone();
        let world_owner = self
            .inner
            .world
            .as_ref()
            .ok_or_else(|| error("career world absent"))?;
        let terrain = self
            .inner
            .terrain
            .as_ref()
            .ok_or_else(|| error("career terrain absent"))?;
        let startup = terrain
            .terrain_for(&world_owner.world)
            .map_err(|e| error(format!("{e:?}")))?;
        let player = self
            .inner
            .player
            .as_mut()
            .ok_or_else(|| error("career player absent"))?;
        let owners = &mut player.selected.owners;
        let cvars = owners.cvars.clone();
        let recipe_owner = Arc::clone(&owners.living_world_recipes);
        let mut recipe_runtime = recipe_owner
            .lock()
            .map_err(|_| error("RecipeMan owner lock poisoned"))?;
        let graph_owner = Arc::clone(
            owners
                .graphs
                .as_ref()
                .ok_or_else(|| error("graph owners absent"))?,
        );
        let graph_guard = graph_owner
            .lock()
            .map_err(|_| error("graph owner lock poisoned"))?;
        let mut frame = SelectedRecipeFrame {
            challenge: None,
            triggers: &startup.triggers,
            pedestrian_animation: crate::selected_living_world_recipes::pedestrian_animation::PedestrianSessionAnimation {
                startup: &owners.pedestrian_animation,
                shared: &owners.animation,
                world: &world_owner.world,
            },
            runtime: &mut recipe_runtime,
            registry: owners.recipe_registry.as_mut().ok_or_else(|| error("mounted recipe registry absent"))?,
            requests: owners.recipe_load.as_mut().ok_or_else(|| error("mounted recipe requests absent"))?.requests_mut(),
            mounts: &owners.mounts,
            vault: &owners.services.vault,
            graphs: &graph_guard,
            painter: &startup.layers,
        };
        frame.update_crowd(&buffer, &cvars).map_err(error)
    }

    /// `Sk8ChallengeSystem::GameUnload`'s `cChallengeCharacterManager::GameUnload` and
    /// `cChallengeLivingWorldManager::GameUnload`, applied before the career goes.
    pub(crate) fn unload_career_living_world(&mut self) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_mut() else {
            return Ok(());
        };
        let challenge = career.system.world_mut();
        // Consume the service's physical unspawn while the current roster and
        // Actor still exist. Activity shutdown follows this host unload step.
        crate::challenge::services::game_unload(challenge);
        characters::game_unload(challenge);
        living_world::game_unload(challenge);
        self.apply_career_living_world()?;
        // Presentation::GameUnload (0x827f1390) flushes the crowd presentation.
        if let Some(player) = self.inner.player.as_ref() {
            player
                .selected
                .owners
                .living_world_recipes
                .lock()
                .map_err(|_| error("RecipeMan owner lock poisoned"))?
                .crowd
                .flush();
        }
        let world = &self
            .inner
            .world
            .as_ref()
            .ok_or_else(|| error("career world absent"))?
            .world;
        set_waypoint_subscription(world, false)
    }
}

/// The simulation's `GameSimulationBroadcaster` route.
fn game_sim_route(
    world: &Arc<Mutex<PhysicalAssemblyWorld>>,
) -> Result<Option<Arc<crate::hermes_game_sim_queue::GameSimEventRoute>>, SelectedLiveError> {
    let rate = Arc::clone(
        &world
            .lock()
            .map_err(|_| error("world lock poisoned"))?
            .sim_rate,
    );
    let route = rate
        .lock()
        .map_err(|_| error("GameSim rate lock poisoned"))?
        .game_sim_events();
    Ok(route)
}

/// The character manager's subscription to `cMsgWaypointGroupAdded`, held while a career runs.
fn set_waypoint_subscription(
    world: &Arc<Mutex<PhysicalAssemblyWorld>>,
    subscribed: bool,
) -> Result<(), SelectedLiveError> {
    if let Some(route) = game_sim_route(world)? {
        *route
            .challenge_waypoint_groups
            .lock()
            .map_err(|_| error("challenge waypoint group lock poisoned"))? =
            subscribed.then(Vec::new);
    }
    Ok(())
}
